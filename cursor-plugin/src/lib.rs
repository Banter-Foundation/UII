//! Cursor's plug into UII.
//!
//! The native desktop app publishes a [`LayoutSnapshot`] through [`LayoutPlug`].
//! Cursor starts the `uii-cursor` binary and reads that snapshot. The reply is the
//! region tree (structure), each region's style declarations, what each region is
//! for, and Owl's UI intent.
//!
//! HTML and CSS on [`UiIntent`] are how Owl describes the layout. This crate does
//! not serve them, render them, or turn them into a web app. The running UII app
//! stays on Banter's imgui stack (`dear-imgui-rs` 0.22, `dear-imgui-glow` 0.22,
//! GLFW, OpenGL 3.3). See `https://github.com/Banter-Foundation/Banter`.
//!
//! This crate does not draw editor tabs, the dock, or exporters. The desktop app
//! calls [`LayoutPlug::publish`] or [`LayoutPlug::publish_to`] when the open
//! document changes.
//!
//! Build a snapshot the desktop app can publish.
//!
//! ```
//! use uii_cursor::{LayoutSnapshot, Region, UiIntent};
//! let root = Region::new("stage", "Stage", "main work area").unwrap();
//! let intent = UiIntent::new("where the work happens", "<main id=\"stage\"></main>", "");
//! let snap = LayoutSnapshot::new("Example", intent, root).unwrap();
//! assert!(snap.reading().contains("main work area"));
//! ```

mod mcp;
mod model;
mod paths;
mod plug;

pub use mcp::{decode_frames, encode_frame, respond_frames, run_stdio, DecodedFrames};
pub use model::{
    Bounds, Declaration, LayoutError, LayoutSnapshot, Region, RegionDetail, RegionPurpose, Style,
    UiIntent, FORMAT_VERSION,
};
pub use paths::{default_snapshot_path, snapshot_path_from};
pub use plug::{
    discover, fetch_live, options_from_env, read_current, read_snapshot, write_snapshot,
    LayoutPlug, ReadOptions,
};

pub const SERVER_NAME: &str = "uii-cursor";
pub const SERVER_VERSION: &str = "0.1.0";
