//! UII layout model. The editor's source of truth is HTML and CSS.
//! Other formats import into that markup and export back out of it.

mod document;
mod exchange;
mod html;
mod scan;

mod editor;

pub use document::{
    resize_from_handle, DockSlot, Document, Handle, Node, NodeId, NodeKind, Rect, Rgba,
};
pub use editor::{
    Component, DrawerPage, Editor, Inspection, Motion, Pose, Tool, WorkspaceTab,
};
pub use exchange::{export_source, formats, html_and_css, import_source, FormatError, FormatInfo};
pub use html::import_html_css;
