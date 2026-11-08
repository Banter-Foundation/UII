//! UII desktop app.
//!
//! The window is GLFW and OpenGL 3.3. Dear ImGui is the docking branch
//! (`dear-imgui-rs` 0.22), the same stack Banter uses.
//!
//! This crate is the Inspect tool. Inspect is a tool, not a tab. It reads the
//! current selection and shows that item's size, color, and type. It does not
//! create items. Other tabs, the assets dock, import/export, and the Cursor
//! plugin are not part of this crate.

mod host;
pub mod inspect;

pub use inspect::{
    Color, InspectReadout, InspectState, InspectTool, ItemSize, ItemSizeError, ItemType,
    ItemTypeError, SelectedItem,
};

pub fn run() -> Result<(), String> {
    host::run()
}
