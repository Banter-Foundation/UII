//! Assets and Components drawer. These checks are the behavior contract.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use uii::{
    asset_rows, body_height_from_pointer, component_rows, dock_path, expand_for_new_folder,
    folder_contents, handle_hits, insertion_folder, layout_assets_pane, layout_drawer,
    library_path, load_session, open_session, pointer_in_dock, pointer_on_resize_edge,
    prune_browser, resolve_data_dir, save_session, shortcut_fires, should_dismiss_unpinned,
    AssetId, AssetKind, BrowserState, DataDirOs, DockHost, DrawerCommand, DrawerState, Library,
    LibraryError, SessionError, ShortcutEdge, StoreError, CONTENTS_MIN_W, DEFAULT_BODY_H,
    DOCK_VERSION, HANDLE_H, LIBRARY_VERSION, MAX_ASSETS_FRACTION, MAX_BODY_FRACTION, MAX_BODY_H,
    MIN_ASSETS_FRACTION, MIN_BODY_H, PIN_W, RESIZE_HIT, SPLITTER_W, TREE_W,
};

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.02, "{actual} != {expected}");
}

fn open_pinned() -> DrawerState {
    let mut state = DrawerState::default();
    state.apply(DrawerCommand::SetOpen(true));
    state.apply(DrawerCommand::SetPinned(true));
    state
}

#[test]
fn drawer_metrics_match_the_content_drawer() {
    assert_eq!(LIBRARY_VERSION, 1);
    assert_eq!(DOCK_VERSION, 1);
    assert_eq!(HANDLE_H, 32.0);
    assert_eq!(DEFAULT_BODY_H, 280.0);
    assert_eq!(MIN_BODY_H, 160.0);
    assert_eq!(MAX_BODY_H, 1200.0);
    assert_eq!(MAX_BODY_FRACTION, 0.75);
    assert_eq!(MIN_ASSETS_FRACTION, 0.25);
    assert_eq!(MAX_ASSETS_FRACTION, 0.75);
    assert_eq!(SPLITTER_W, 4.0);
    assert_eq!(PIN_W, 72.0);
    assert_eq!(TREE_W, 220.0);
    assert_eq!(CONTENTS_MIN_W, 120.0);
    assert_eq!(RESIZE_HIT, 6.0);
}

#[test]
fn drawer_starts_collapsed() {
    let state = DrawerState::default();
    assert!(!state.open);
    assert!(!state.pinned);
    assert_eq!(state.body_height, DEFAULT_BODY_H);
    assert_eq!(state.assets_fraction, 0.5);
    let layout = layout_drawer(1280.0, 720.0, &state);
    assert!(layout.body.is_none());
    assert!(layout.assets.is_none());
    assert!(layout.components.is_none());
    near(layout.handle.h, HANDLE_H);
    near(layout.handle.y, 720.0 - HANDLE_H);
    near(layout.handle.w, 1280.0);
    near(layout.stage.h, layout.handle.y);
    near(layout.stage.w, 1280.0);
    assert!(!layout.stage.overlaps(layout.handle));
}

#[test]
fn pinned_drawer_sits_under_the_stage() {
    let state = open_pinned();
    let layout = layout_drawer(1280.0, 720.0, &state);
    let body = layout.body.expect("open drawer");
    let assets = layout.assets.expect("assets pane");
    let components = layout.components.expect("components pane");
    near(body.h, DEFAULT_BODY_H);
    near(body.y, 720.0 - HANDLE_H - DEFAULT_BODY_H);
    near(layout.handle.y, body.bottom());
    near(layout.stage.h, body.y);
    near(layout.stage.bottom(), body.y);
    assert!(!layout.stage.overlaps(body));
    assert!(!body.overlaps(layout.handle));
    near(assets.y, components.y);
    near(assets.h, body.h);
    near(components.h, body.h);
    near(assets.y, body.y);
    assert!(assets.right() < components.x);
    near(components.x - assets.right(), SPLITTER_W);
    near(assets.w + components.w + SPLITTER_W, body.w);
    near(assets.w / (assets.w + components.w), 0.5);
    assert!(!assets.overlaps(components));
}

#[test]
fn unpinned_drawer_overlays_the_stage() {
    let mut state = DrawerState::default();
    state.apply(DrawerCommand::SetOpen(true));
    let layout = layout_drawer(1280.0, 720.0, &state);
    let body = layout.body.expect("overlay");
    near(layout.stage.h, 720.0 - HANDLE_H);
    near(layout.stage.bottom(), layout.handle.y);
    assert!(layout.stage.overlaps(body));
    assert!(body.y > layout.stage.y);
    assert!(body.bottom() <= layout.stage.bottom() + 0.02);
}

#[test]
fn body_height_and_split_are_clamped() {
    let mut state = DrawerState::default();
    state.apply(DrawerCommand::SetOpen(true));
    state.apply(DrawerCommand::SetPinned(true));
    state.apply(DrawerCommand::SetBodyHeight(10.0));
    assert_eq!(state.body_height, MIN_BODY_H);
    state.apply(DrawerCommand::SetBodyHeight(10_000.0));
    assert_eq!(state.body_height, MAX_BODY_H);
    let layout = layout_drawer(1280.0, 720.0, &state);
    near(layout.body.expect("body").h, 720.0 * MAX_BODY_FRACTION);

    state.apply(DrawerCommand::SetAssetsFraction(0.0));
    assert_eq!(state.assets_fraction, MIN_ASSETS_FRACTION);
    state.apply(DrawerCommand::SetAssetsFraction(0.9));
    assert_eq!(state.assets_fraction, MAX_ASSETS_FRACTION);
    state.apply(DrawerCommand::SetAssetsFraction(0.4));
    assert_eq!(state.assets_fraction, 0.4);
    let layout = layout_drawer(1280.0, 720.0, &state);
    let assets = layout.assets.expect("assets");
    let components = layout.components.expect("components");
    near(assets.w / (assets.w + components.w), 0.4);
}

#[test]
fn short_viewports_keep_a_handle() {
    let state = open_pinned();
    let layout = layout_drawer(800.0, 40.0, &state);
    near(layout.handle.h, HANDLE_H);
    near(layout.handle.y, 8.0);
    near(layout.body.expect("shrunk body").h, 8.0);
    near(layout.stage.h, 0.0);

    let tiny = layout_drawer(100.0, 20.0, &state);
    near(tiny.handle.h, 20.0);
    near(tiny.handle.y, 0.0);
    assert!(tiny.body.is_none());
    near(tiny.stage.h, 0.0);
}

#[test]
fn zero_and_negative_viewports_are_empty() {
    let state = open_pinned();
    for (width, height) in [(0.0, 0.0), (-10.0, 50.0), (50.0, -5.0), (0.0, 720.0)] {
        let layout = layout_drawer(width, height, &state);
        assert_eq!(layout.stage, uii::Rect::ZERO);
        assert_eq!(layout.handle, uii::Rect::ZERO);
        assert!(layout.body.is_none());
        assert!(layout.assets.is_none());
        assert!(layout.components.is_none());
    }
}

#[test]
fn toggle_pin_and_outside_click() {
    let mut state = DrawerState::default();
    state.apply(DrawerCommand::Toggle);
    assert!(state.open);
    state.apply(DrawerCommand::Toggle);
    assert!(!state.open);
    state.apply(DrawerCommand::SetOpen(true));
    state.apply(DrawerCommand::TogglePin);
    assert!(state.pinned);
    assert!(state.open);
    state.apply(DrawerCommand::PointerLeftStage);
    assert!(state.open);
    state.apply(DrawerCommand::SetPinned(false));
    state.apply(DrawerCommand::PointerLeftStage);
    assert!(!state.open);
    state.apply(DrawerCommand::PointerLeftStage);
    assert!(!state.open);

    let mut open = DrawerState::default();
    open.apply(DrawerCommand::SetOpen(true));
    assert!(should_dismiss_unpinned(&open, true, false));
    assert!(!should_dismiss_unpinned(&open, false, false));
    assert!(!should_dismiss_unpinned(&open, true, true));
    open.apply(DrawerCommand::SetPinned(true));
    assert!(!should_dismiss_unpinned(&open, true, false));
    let closed = DrawerState::default();
    assert!(!should_dismiss_unpinned(&closed, true, false));
}

#[test]
fn handle_puts_pin_on_the_right_only_while_open() {
    let handle = uii::Rect {
        x: 0.0,
        y: 400.0,
        w: 400.0,
        h: HANDLE_H,
    };
    let closed = handle_hits(handle, false);
    assert!(closed.pin.is_none());
    assert_eq!(closed.toggle, handle);

    let open = handle_hits(handle, true);
    let pin = open.pin.expect("pin");
    near(pin.w, PIN_W);
    near(pin.right(), handle.right());
    near(pin.y, handle.y);
    assert!(!open.toggle.overlaps(pin));
    near(open.toggle.w + pin.w, handle.w);
    near(open.toggle.x, handle.x);

    let narrow = uii::Rect {
        x: 0.0,
        y: 0.0,
        w: PIN_W,
        h: HANDLE_H,
    };
    assert!(handle_hits(narrow, true).pin.is_none());
}

#[test]
fn resize_uses_the_top_edge_of_the_body() {
    let body = uii::Rect {
        x: 0.0,
        y: 100.0,
        w: 200.0,
        h: 80.0,
    };
    assert!(pointer_on_resize_edge(body, 100.0));
    assert!(pointer_on_resize_edge(body, 100.0 + RESIZE_HIT));
    assert!(!pointer_on_resize_edge(body, 100.0 + RESIZE_HIT + 0.2));
    near(
        body_height_from_pointer(720.0, 400.0),
        720.0 - HANDLE_H - 400.0,
    );

    let mut state = open_pinned();
    let layout = layout_drawer(1280.0, 720.0, &state);
    assert!(pointer_in_dock(&layout, 10.0, layout.handle.y + 2.0));
    assert!(pointer_in_dock(
        &layout,
        10.0,
        layout.body.expect("body").y + 20.0
    ));
    assert!(!pointer_in_dock(&layout, 10.0, 10.0));
    state.apply(DrawerCommand::SetPinned(false));
    let overlay = layout_drawer(1280.0, 720.0, &state);
    assert!(!pointer_in_dock(&overlay, 10.0, 10.0));
    assert!(pointer_in_dock(
        &overlay,
        10.0,
        overlay.body.expect("body").y + 8.0
    ));
}

#[test]
fn ctrl_space_toggles_once_unless_text_is_active() {
    let mut edge = ShortcutEdge::default();
    assert!(!edge.ctrl_space(true, false));
    assert!(edge.ctrl_space(true, true));
    assert!(!edge.ctrl_space(true, true));
    assert!(!edge.ctrl_space(false, true));
    assert!(edge.ctrl_space(true, true));
    assert!(!edge.ctrl_space(true, false));
    assert!(edge.ctrl_space(true, true));
    assert!(shortcut_fires(true, false));
    assert!(!shortcut_fires(true, true));
    assert!(!shortcut_fires(false, false));
}

#[test]
fn assets_pane_is_a_tree_beside_contents() {
    let pane = uii::Rect {
        x: 10.0,
        y: 20.0,
        w: 600.0,
        h: 300.0,
    };
    let split = layout_assets_pane(pane);
    near(split.tree.x, 10.0);
    near(split.tree.y, 20.0);
    near(split.tree.w, TREE_W);
    near(split.tree.h, 300.0);
    near(split.contents.x, 10.0 + TREE_W + SPLITTER_W);
    near(split.contents.w, 600.0 - TREE_W - SPLITTER_W);
    near(split.contents.h, 300.0);
    assert!(!split.tree.overlaps(split.contents));

    let narrow = uii::Rect {
        x: 0.0,
        y: 0.0,
        w: 200.0,
        h: 40.0,
    };
    let split = layout_assets_pane(narrow);
    near(split.tree.w, 76.0);
    near(split.contents.w, CONTENTS_MIN_W);
    near(split.contents.x, split.tree.right() + SPLITTER_W);

    let tight = layout_assets_pane(uii::Rect {
        x: 0.0,
        y: 0.0,
        w: 50.0,
        h: 10.0,
    });
    near(tight.tree.w, 18.4);
    near(tight.contents.w, 27.6);
    assert!(!tight.tree.overlaps(tight.contents));
}

#[test]
fn new_library_has_no_seed_data() {
    let library = Library::new();
    assert_eq!(library.asset_count(), 0);
    assert_eq!(library.component_count(), 0);
    assert!(library.assets().is_empty());
    assert!(library.components().is_empty());
    assert!(asset_rows(&library, "", &BTreeSet::new()).is_empty());
    assert!(component_rows(&library, "").is_empty());
    let host = open_session(None, None).expect("empty session");
    assert_eq!(host.library.asset_count(), 0);
    assert!(!host.drawer.open);
}

#[test]
fn user_creates_folders_assets_and_components() {
    let mut library = Library::new();
    let folder = library.create_folder(None, " Widgets ").unwrap();
    let button = library.create_asset(Some(folder), "按钮").unwrap();
    let label = library.create_asset(Some(folder), "Label").unwrap();
    let readme = library.create_asset(None, "Readme").unwrap();
    assert_eq!(folder.0, 1);
    assert_eq!(button.0, 2);
    assert_eq!(library.asset(button).unwrap().name, "按钮");
    assert_eq!(library.asset(folder).unwrap().kind, AssetKind::Folder);
    assert_eq!(library.children(None).len(), 2);
    assert_eq!(library.children(None)[0], folder);
    // "Label" sorts before "按钮".
    assert_eq!(library.children(Some(folder)), vec![label, button]);

    let pad = library.create_component("Pad").unwrap();
    let pad_lower = library.create_component("pad").unwrap();
    assert_eq!(pad.0, 1);
    assert_eq!(library.component_count(), 2);
    assert_eq!(library.asset_count(), 4);
    assert!(library.create_component("Pad").is_err());
    assert_eq!(
        library.create_asset(Some(button), "Child"),
        Err(LibraryError::ParentNotFolder)
    );
    assert_eq!(
        library.create_folder(Some(AssetId(99)), "Missing"),
        Err(LibraryError::Missing)
    );
    assert_eq!(
        library.create_asset(None, "   "),
        Err(LibraryError::EmptyName)
    );
    assert_eq!(
        library.create_asset(None, "a/b"),
        Err(LibraryError::InvalidName)
    );
    assert_eq!(
        library.create_folder(None, "a\\b"),
        Err(LibraryError::InvalidName)
    );
    assert_eq!(
        library.create_component("line\nbreak"),
        Err(LibraryError::InvalidName)
    );
    assert_eq!(
        library.create_asset(None, "Readme"),
        Err(LibraryError::DuplicateName)
    );
    assert_eq!(
        library.create_folder(None, " Readme "),
        Err(LibraryError::DuplicateName)
    );
    library.rename_asset(readme, "Readme").unwrap();
    assert_eq!(
        library.rename_asset(readme, "Label"),
        Err(LibraryError::DuplicateName)
    );
    assert_eq!(
        library.rename_asset(AssetId(50), "Nope"),
        Err(LibraryError::Missing)
    );
    assert_eq!(
        library.delete_asset(AssetId(50)),
        Err(LibraryError::Missing)
    );

    let nested = library.create_folder(Some(folder), "Nested").unwrap();
    let deep = library.create_asset(Some(nested), "Icon").unwrap();
    library.delete_asset(folder).unwrap();
    assert!(library.asset(folder).is_none());
    assert!(library.asset(button).is_none());
    assert!(library.asset(label).is_none());
    assert!(library.asset(nested).is_none());
    assert!(library.asset(deep).is_none());
    assert!(library.asset(readme).is_some());
    assert!(library.component(pad).is_some());
    library.delete_component(pad).unwrap();
    assert!(library.component(pad).is_none());
    assert!(library.component(pad_lower).is_some());
    assert!(library.asset(readme).is_some());
    assert_eq!(library.delete_component(pad), Err(LibraryError::Missing));

    let again = library.create_asset(None, "Second").unwrap();
    assert_eq!(again.0, deep.0 + 1);
}

#[test]
fn rows_filter_without_hiding_the_path() {
    let mut library = Library::new();
    let widgets = library.create_folder(None, "Widgets").unwrap();
    let button = library.create_asset(Some(widgets), "Button").unwrap();
    let label = library.create_asset(Some(widgets), "Label").unwrap();
    let readme = library.create_asset(None, "Readme").unwrap();
    let names = |rows: Vec<uii::AssetRow>| {
        rows.into_iter()
            .map(|row| {
                (
                    library.asset(row.id).unwrap().name.clone(),
                    row.depth,
                    row.kind,
                )
            })
            .collect::<Vec<_>>()
    };

    let collapsed = names(asset_rows(&library, "", &BTreeSet::new()));
    assert_eq!(
        collapsed,
        vec![
            ("Widgets".into(), 0, AssetKind::Folder),
            ("Readme".into(), 0, AssetKind::Asset),
        ]
    );
    let mut expanded = BTreeSet::new();
    expanded.insert(widgets);
    let open = names(asset_rows(&library, "  ", &expanded));
    assert_eq!(
        open,
        vec![
            ("Widgets".into(), 0, AssetKind::Folder),
            ("Button".into(), 1, AssetKind::Asset),
            ("Label".into(), 1, AssetKind::Asset),
            ("Readme".into(), 0, AssetKind::Asset),
        ]
    );
    let found = names(asset_rows(&library, "button", &BTreeSet::new()));
    assert_eq!(
        found,
        vec![
            ("Widgets".into(), 0, AssetKind::Folder),
            ("Button".into(), 1, AssetKind::Asset),
        ]
    );
    let folder_only = names(asset_rows(&library, "widgets", &BTreeSet::new()));
    assert_eq!(folder_only, vec![("Widgets".into(), 0, AssetKind::Folder)]);
    assert_eq!(
        folder_contents(&library, Some(widgets), ""),
        vec![button, label]
    );
    assert_eq!(folder_contents(&library, None, "read"), vec![readme]);

    let alpha = library.create_component("A").unwrap();
    let lower = library.create_component("a").unwrap();
    let bee = library.create_component("b").unwrap();
    let listed = component_rows(&library, "")
        .into_iter()
        .map(|id| library.component(id).unwrap().name.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        listed,
        vec!["A".to_string(), "a".to_string(), "b".to_string()]
    );
    let filtered = component_rows(&library, "a");
    assert_eq!(filtered, vec![alpha, lower]);
    assert!(!filtered.contains(&bee));
}

#[test]
fn browser_selection_follows_creates_and_deletes() {
    let mut library = Library::new();
    let widgets = library.create_folder(None, "Widgets").unwrap();
    let button = library.create_asset(Some(widgets), "Button").unwrap();
    let mut browser = BrowserState {
        selected_asset: Some(button),
        selected_folder: Some(AssetId(99)),
        ..BrowserState::default()
    };
    assert_eq!(insertion_folder(&library, &browser), Some(widgets));
    browser.selected_asset = None;
    browser.selected_folder = Some(widgets);
    assert_eq!(insertion_folder(&library, &browser), Some(widgets));
    browser.selected_asset = Some(button);
    assert_eq!(insertion_folder(&library, &browser), Some(widgets));

    let mut expanded = BTreeSet::new();
    expand_for_new_folder(&mut expanded, Some(widgets), button);
    assert!(expanded.contains(&widgets));
    assert!(expanded.contains(&button));

    browser.selected_folder = Some(button);
    browser.selected_asset = Some(widgets);
    browser.expanded.insert(widgets);
    browser.expanded.insert(button);
    prune_browser(&mut browser, &library);
    assert!(browser.selected_folder.is_none());
    assert!(browser.selected_asset.is_none());
    assert!(browser.expanded.contains(&widgets));
    assert!(!browser.expanded.contains(&button));

    browser.selected_folder = Some(widgets);
    browser.selected_asset = Some(button);
    library.delete_asset(widgets).unwrap();
    prune_browser(&mut browser, &library);
    assert!(browser.selected_folder.is_none());
    assert!(browser.selected_asset.is_none());
    assert!(browser.expanded.is_empty());
}

#[test]
fn library_and_dock_json_round_trip_and_reject_corruption() {
    let mut library = Library::new();
    let folder = library.create_folder(None, "UI").unwrap();
    library.create_asset(Some(folder), "Mark").unwrap();
    let json = library.to_json().unwrap();
    assert!(json.contains("\"folder\""));
    assert!(json.contains("\"version\": 1") || json.contains("\"version\":1"));
    let restored = Library::from_json(&json).unwrap();
    assert_eq!(restored.asset_count(), 2);
    assert_eq!(restored.asset(folder).unwrap().name, "UI");

    let dock = format!(
        r#"{{"version":1,"open":true,"pinned":true,"body_height":10,"assets_fraction":0,"asset_query":"ui","component_query":"","selected_folder":999,"selected_asset":null,"selected_component":5,"expanded":[999,{}]}}"#,
        folder.0
    );
    let host = open_session(Some(&json), Some(&dock)).unwrap();
    assert!(host.drawer.open);
    assert!(host.drawer.pinned);
    assert_eq!(host.drawer.body_height, MIN_BODY_H);
    assert_eq!(host.drawer.assets_fraction, MIN_ASSETS_FRACTION);
    assert_eq!(host.browser.asset_query, "ui");
    assert!(host.browser.selected_folder.is_none());
    assert!(host.browser.selected_component.is_none());
    assert!(host.browser.expanded.contains(&folder));
    assert!(!host.browser.expanded.contains(&AssetId(999)));

    assert!(matches!(
        Library::from_json(
            r#"{"version":2,"next_asset":1,"next_component":1,"assets":[],"components":[]}"#
        ),
        Err(StoreError::Version)
    ));
    assert!(matches!(
        open_session(Some("{"), None),
        Err(SessionError::Library(StoreError::Json))
    ));
    assert!(matches!(
        Library::from_json(
            r#"{"version":1,"next_asset":3,"next_component":1,"assets":[{"id":1,"parent":2,"name":"a","kind":"folder"},{"id":2,"parent":1,"name":"b","kind":"folder"}],"components":[]}"#
        ),
        Err(StoreError::Cycle)
    ));
    assert!(matches!(
        Library::from_json(
            r#"{"version":1,"next_asset":3,"next_component":1,"assets":[{"id":1,"parent":null,"name":"A","kind":"asset"},{"id":2,"parent":1,"name":"B","kind":"asset"}],"components":[]}"#
        ),
        Err(StoreError::Parent)
    ));
    assert!(matches!(
        Library::from_json(
            r#"{"version":1,"next_asset":3,"next_component":1,"assets":[{"id":1,"parent":null,"name":"A","kind":"asset"},{"id":2,"parent":null,"name":"A","kind":"folder"}],"components":[]}"#
        ),
        Err(StoreError::DuplicateName)
    ));
    assert!(matches!(
        Library::from_json(
            r#"{"version":1,"next_asset":1,"next_component":1,"assets":[{"id":0,"parent":null,"name":"A","kind":"asset"}],"components":[]}"#
        ),
        Err(StoreError::Id)
    ));
    assert!(matches!(
        open_session(
            Some(
                r#"{"version":1,"next_asset":1,"next_component":1,"assets":[],"components":[],"extra":1}"#
            ),
            None
        ),
        Err(SessionError::Library(StoreError::Json))
    ));
}

#[test]
fn save_and_load_user_files() {
    let dir = temp_dir("save-load");
    let mut host = DockHost::new(Library::new());
    let logo = host.library.create_asset(None, "Logo").unwrap();
    host.drawer.apply(DrawerCommand::SetOpen(true));
    host.drawer.apply(DrawerCommand::SetPinned(true));
    host.browser.asset_query = "Lo".into();
    host.browser.selected_asset = Some(logo);
    save_session(&dir, &host).unwrap();
    assert!(library_path(&dir).is_file());
    assert!(dock_path(&dir).is_file());

    let loaded = load_session(&dir).unwrap();
    assert_eq!(loaded.library.asset_count(), 1);
    assert_eq!(loaded.library.asset(logo).unwrap().name, "Logo");
    assert!(loaded.drawer.open && loaded.drawer.pinned);
    assert_eq!(loaded.browser.asset_query, "Lo");
    assert_eq!(loaded.browser.selected_asset, Some(logo));

    let empty = temp_dir("empty");
    let fresh = load_session(&empty).unwrap();
    assert_eq!(fresh.library.asset_count(), 0);
    assert!(!fresh.drawer.open);

    let broken = temp_dir("broken");
    std::fs::write(library_path(&broken), "{").unwrap();
    assert!(matches!(
        load_session(&broken),
        Err(SessionError::Library(StoreError::Json))
    ));
    assert_eq!(std::fs::read_to_string(library_path(&broken)).unwrap(), "{");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&empty);
    let _ = std::fs::remove_dir_all(&broken);
}

#[test]
fn data_dir_respects_override_then_os_defaults() {
    let home = Path::new("/home/me");
    assert_eq!(
        resolve_data_dir(DataDirOs::Other, Some("/tmp/uii"), Some("/xdg"), home),
        PathBuf::from("/tmp/uii")
    );
    assert_eq!(
        resolve_data_dir(
            DataDirOs::Windows,
            Some("  "),
            Some("/xdg"),
            Path::new("/local")
        ),
        PathBuf::from("/local/UII")
    );
    assert_eq!(
        resolve_data_dir(DataDirOs::Macos, None, Some("/xdg"), home),
        PathBuf::from("/home/me/Library/Application Support/UII")
    );
    assert_eq!(
        resolve_data_dir(DataDirOs::Other, None, Some("/xdg"), home),
        PathBuf::from("/xdg/UII")
    );
    assert_eq!(
        resolve_data_dir(DataDirOs::Other, None, Some("  "), home),
        PathBuf::from("/home/me/.local/share/UII")
    );
}

fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "uii-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}
