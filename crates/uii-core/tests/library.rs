use std::path::Path;

use uii_core::{
    read_library, resolve_data_dir, write_library, AssetKind, DataDirOs, Editor, Library,
    LibraryError,
};

#[test]
fn new_library_has_no_seed_data() {
    let editor = Editor::new();
    assert_eq!(editor.library().asset_count(), 0);
    assert_eq!(editor.library().component_count(), 0);
    assert!(!editor.library_changed());
}

#[test]
fn folders_nest_assets_and_round_trip_through_json() {
    let mut editor = Editor::new();
    let icons = editor.create_named_folder("Icons").unwrap();
    editor.activate_library_row(icons);
    let logo = editor.create_named_asset("logo.png").unwrap();
    let record = editor.library().asset(logo).unwrap();
    assert_eq!(record.parent, Some(icons));
    assert_eq!(record.kind, AssetKind::Asset);
    assert!(editor.library().is_folder(icons));
    assert!(editor.library_changed());

    let path = std::env::temp_dir().join(format!("uii-library-{}.json", std::process::id()));
    write_library(&path, editor.library()).unwrap();
    let loaded = read_library(&path).unwrap().unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(loaded.asset_count(), 2);
    assert_eq!(loaded.asset(logo).unwrap().name, "logo.png");
    assert_eq!(loaded.children(Some(icons)), vec![logo]);
}

#[test]
fn duplicate_names_and_bad_parents_are_rejected() {
    let mut editor = Editor::new();
    editor.create_named_folder("Icons").unwrap();
    let err = editor.create_named_folder("Icons").unwrap_err();
    assert_eq!(err, LibraryError::DuplicateName);
    let mut library = Library::new();
    let asset = library.create_asset(None, "logo.png").unwrap();
    let err = library.create_folder(Some(asset), "Nested").unwrap_err();
    assert_eq!(err, LibraryError::ParentNotFolder);
}

#[test]
fn search_keeps_a_folder_open_when_a_child_matches() {
    let mut editor = Editor::new();
    let folder = editor.create_named_folder("Icons").unwrap();
    editor.activate_library_row(folder);
    editor.create_named_asset("logo.png").unwrap();
    editor.set_library_query("logo");
    let rows = editor.library_rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].name, "Icons");
    assert_eq!(rows[1].name, "logo.png");
    assert_eq!(rows[1].depth, 1);
}

#[test]
fn adding_an_asset_path_joins_the_folder_library() {
    let mut editor = Editor::new();
    editor.add_asset("images/logo.png");
    assert!(editor
        .library()
        .assets()
        .iter()
        .any(|asset| asset.name == "logo.png" && asset.kind == AssetKind::Asset));
    editor.add_asset("images/logo.png");
    assert_eq!(
        editor
            .library()
            .assets()
            .iter()
            .filter(|asset| asset.name == "logo.png")
            .count(),
        1
    );
}

#[test]
fn data_dir_respects_override_then_os_defaults() {
    let base = Path::new("/home/owl");
    assert_eq!(
        resolve_data_dir(DataDirOs::Other, Some("/tmp/uii"), None, base),
        Path::new("/tmp/uii")
    );
    assert_eq!(
        resolve_data_dir(DataDirOs::Other, Some("  "), Some("/xdg"), base),
        Path::new("/xdg/UII")
    );
    assert_eq!(
        resolve_data_dir(DataDirOs::Other, None, None, base),
        Path::new("/home/owl/.local/share/UII")
    );
    assert_eq!(
        resolve_data_dir(DataDirOs::Macos, None, None, base),
        Path::new("/home/owl/Library/Application Support/UII")
    );
    assert_eq!(
        resolve_data_dir(
            DataDirOs::Windows,
            None,
            None,
            Path::new("C:/Users/owl/AppData/Local")
        ),
        Path::new("C:/Users/owl/AppData/Local/UII")
    );
}

#[test]
fn a_corrupt_library_file_is_not_a_library() {
    let err = read_library(Path::new("/no/such/library.json"));
    assert!(err.unwrap().is_none());
    let path = std::env::temp_dir().join(format!("uii-library-bad-{}.json", std::process::id()));
    std::fs::write(&path, "{not json").unwrap();
    let err = read_library(&path).unwrap_err();
    let _ = std::fs::remove_file(&path);
    assert!(err.contains("library.json"), "{err}");
}
