use uii_core::{DrawerPage, Editor, Motion, NodeKind, Rect, Rgba, Tool, WorkspaceTab};

#[test]
fn top_tabs_are_design_animation_and_function() {
    let labels: Vec<_> = Editor::tabs().iter().map(|tab| tab.label()).collect();
    assert_eq!(labels, vec!["Design", "Animation", "Function"]);
}

#[test]
fn assets_and_components_live_in_a_collapsible_drawer() {
    let mut editor = Editor::new();
    assert!(editor.drawer_collapsed());
    let pages: Vec<_> = Editor::drawer_pages().iter().map(|page| page.label()).collect();
    assert_eq!(pages, vec!["Assets", "Components"]);
    editor.open_drawer();
    assert!(!editor.drawer_collapsed());
    editor.show_drawer(DrawerPage::Components);
    assert_eq!(editor.drawer_page(), DrawerPage::Components);
    editor.collapse_drawer();
    assert!(editor.drawer_collapsed());
    assert_eq!(editor.tab(), WorkspaceTab::Design);
}

#[test]
fn inspect_is_a_tool_and_shows_size_color_and_type() {
    let mut editor = Editor::new();
    let id = editor.add_shape(NodeKind::Rectangle, "Shade", Rect::new(10.0, 20.0, 40.0, 50.0));
    editor.set_fill(id, Rgba::rgb(0x11, 0x22, 0x33));
    editor.set_tool(Tool::Inspect);
    let tool_labels: Vec<_> = Editor::tools().iter().map(|tool| tool.label()).collect();
    assert!(tool_labels.contains(&"Inspect"));
    assert!(!Editor::tabs().iter().any(|tab| tab.label() == "Inspect"));
    assert_eq!(editor.tab(), WorkspaceTab::Design);
    editor.select(id);
    let info = editor.inspection().expect("selected shape");
    assert_eq!(info.name, "Shade");
    assert_eq!(info.kind, "rectangle");
    assert_eq!(info.width, 40.0);
    assert_eq!(info.height, 50.0);
    assert_eq!(info.color, "#112233");
}

#[test]
fn inspect_without_a_selection_shows_nothing() {
    let mut editor = Editor::new();
    editor.set_tool(Tool::Inspect);
    assert!(editor.inspection().is_none());
}

#[test]
fn moving_on_the_design_canvas_rewrites_html() {
    let mut editor = Editor::new();
    editor.set_tab(WorkspaceTab::Design);
    let id = editor.add_shape(NodeKind::Frame, "Box", Rect::new(1.0, 2.0, 30.0, 40.0));
    editor.move_node(id, 5.0, 6.0);
    let (html, css) = editor.markup();
    assert!(html.contains("data-uii-name=\"Box\""), "{html}");
    assert!(html.contains("left:5px"), "{html}");
    assert!(html.contains("top:6px"), "{html}");
    assert!(css.contains("left: 5px"), "{css}");
}

#[test]
fn animation_samples_motion_without_changing_the_design() {
    let mut editor = Editor::new();
    let id = editor.add_shape(NodeKind::Frame, "Box", Rect::new(0.0, 0.0, 10.0, 10.0));
    editor.set_tab(WorkspaceTab::Animation);
    editor.add_key(id, Motion::X, 0.0, 0.0);
    editor.add_key(id, Motion::X, 1.0, 100.0);
    editor.set_time(0.5);
    let pose = editor.pose(id);
    assert!((pose.x - 50.0).abs() < 0.01, "{}", pose.x);
    assert_eq!(editor.document().node(id).rect.x, 0.0);
    assert_eq!(editor.tab(), WorkspaceTab::Animation);
}

#[test]
fn animation_playback_advances_time() {
    let mut editor = Editor::new();
    editor.set_tab(WorkspaceTab::Animation);
    editor.set_playing(true);
    editor.tick(0.25);
    assert!((editor.time() - 0.25).abs() < 0.001);
}

#[test]
fn assets_can_be_added_and_placed_from_the_drawer() {
    let mut editor = Editor::new();
    editor.open_drawer();
    editor.show_drawer(DrawerPage::Assets);
    editor.add_asset("logo.png");
    assert!(editor.assets().iter().any(|path| path == "logo.png"));
    let id = editor.place_asset("logo.png");
    let node = editor.document().node(id);
    assert_eq!(node.kind, NodeKind::Image);
    assert_eq!(node.image_src, "logo.png");
}

#[test]
fn components_can_be_saved_and_inserted_from_the_drawer() {
    let mut editor = Editor::new();
    let id = editor.add_shape(NodeKind::Frame, "Card", Rect::new(0.0, 0.0, 80.0, 40.0));
    editor.select(id);
    editor.save_component("Card");
    editor.open_drawer();
    editor.show_drawer(DrawerPage::Components);
    assert!(editor.components().iter().any(|component| component.name == "Card"));
    let before = editor.document().roots().len();
    editor.insert_component("Card");
    assert!(editor.document().roots().len() > before);
}

#[test]
fn cursor_can_read_region_purpose_and_apply_a_layout() {
    let mut editor = Editor::new();
    let applied = editor.cursor(
        r#"{"op":"apply","html":"<header class=\"bar\">Title</header>","css":"header.bar { width: 200px; height: 30px; }"}"#,
    );
    assert!(applied.contains("\"ok\":true"), "{applied}");
    let header = editor.document().find_named("bar").expect("header");
    assert_eq!(header.kind, NodeKind::Dock);
    let read = editor.cursor(r#"{"op":"get"}"#);
    assert!(read.contains("\"ok\":true"), "{read}");
    assert!(read.contains("\"html\""), "{read}");
    assert!(read.contains("\"css\""), "{read}");
    assert!(read.contains("Top dock for bar"), "{read}");
    assert!(read.contains("Text Title"), "{read}");
}

#[test]
fn cursor_get_on_an_empty_design_has_no_regions() {
    let mut editor = Editor::new();
    let read = editor.cursor(r#"{"op":"get"}"#);
    assert!(read.contains("\"regions\":[]"), "{read}");
}
