//! UII desktop shell.
//!
//! The window is GLFW and OpenGL. Dear ImGui is Banter's docking build
//! (`dear-imgui-rs` 0.22). Assets and Components live in one collapsible
//! bottom drawer, the same idea as Unreal's content drawer. The stage above
//! that drawer is a dockspace for other editor surfaces.

mod draw;
mod drawer;
mod library;
mod platform;
mod session;
mod stage;
mod theme;

pub use drawer::{
    body_height_from_pointer, handle_hits, layout_assets_pane, layout_drawer, pointer_in_dock,
    pointer_on_resize_edge, shortcut_fires, should_dismiss_unpinned, DrawerCommand, DrawerLayout,
    DrawerState, HandleHits, PaneSplit, Rect, ShortcutEdge, CONTENTS_MIN_W, DEFAULT_BODY_H,
    HANDLE_H, MAX_ASSETS_FRACTION, MAX_BODY_FRACTION, MAX_BODY_H, MIN_ASSETS_FRACTION, MIN_BODY_H,
    PIN_W, RESIZE_HIT, SPLITTER_W, TREE_W,
};
pub use library::{
    AssetId, AssetKind, AssetRecord, ComponentId, ComponentRecord, Library, LibraryError,
    StoreError, LIBRARY_VERSION,
};
pub use session::{
    asset_rows, component_rows, data_dir, dock_path, expand_for_new_folder, folder_contents,
    insertion_folder, library_path, load_session, open_session, prune_browser, resolve_data_dir,
    save_session, AssetRow, BrowserState, DataDirOs, DockHost, SessionError, DOCK_VERSION,
};

pub fn run() -> Result<(), String> {
    platform::run()
}
