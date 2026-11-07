//! Function graph for the UII Function tab.
//!
//! A function says how something works. Its place says where it works, using the
//! same name a layout node uses (`data-uii-name`). A connection runs from an
//! output port to an input port. This crate does not draw the design canvas,
//! animation, assets, inspect, import/export, or the Cursor plugin.

mod camera;
mod editor;
mod graph;
mod store;

pub use camera::Camera;
pub use editor::{Button, Editor, LinkPreview};
pub use graph::{
    Connection, ConnectionId, Direction, EditError, Function, FunctionId, FunctionKind, Graph, Hit,
    Port, PortId, Rect, HEADER_HEIGHT, NODE_WIDTH, PORT_PAD, PORT_RADIUS, PORT_ROW, PORT_SLOP,
};
pub use store::{default_graph_path, from_text, to_text, StoreError};

#[cfg(feature = "desktop")]
mod function_tab;
#[cfg(feature = "desktop")]
mod platform;

#[cfg(feature = "desktop")]
pub use function_tab::{draw_function_tab, Session};
#[cfg(feature = "desktop")]
pub use platform::run;
