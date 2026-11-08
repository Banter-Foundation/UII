//! UII layout model. The editor's source of truth is HTML and CSS.
//! Other formats import into that markup and export back out of it.

mod commands;
mod document;
mod easing;
mod exchange;
mod html;
mod scan;

mod editor;

pub use commands::{
    command_for, shortcuts, Bind, BindKey, Command, Shortcut, MARKUP_STARTS_CLOSED,
};
pub use document::{
    format_color, parse_hex_color, resize_from_handle, snap_rect, DockSlot, Document, Guide,
    Handle, Node, NodeId, NodeKind, Rect, Rgba, SNAP_PX,
};
pub use easing::Easing;
pub use editor::{
    node_purpose, Component, DrawerPage, Editor, Inspection, Motion, Pose, Properties, Tool,
    WorkspaceTab,
};
pub use exchange::{export_source, formats, html_and_css, import_source, FormatError, FormatInfo};
pub use html::import_html_css;
