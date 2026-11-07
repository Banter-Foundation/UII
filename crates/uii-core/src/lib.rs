//! UII layout model. The editor's source of truth is HTML and CSS.
//! The Design canvas edits that model. Animation, Function, the content
//! dock, and Inspect are owned elsewhere.

mod document;
mod html;

mod editor;

pub use document::{
    resize_from_handle, DockSlot, Document, Handle, Node, NodeId, NodeKind, Rect, Rgba,
};
pub use editor::{Editor, Tool, WorkspaceTab};
pub use html::import_html_css;
