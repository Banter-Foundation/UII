//! Animation tab for UII.
//!
//! UII is a native layout tool on Banter's Dear ImGui (`dear-imgui-rs` 0.22,
//! the same docking branch Banter ships). This crate is only the Animation
//! tab: clips, property tracks, and keyframes that move layout nodes.
//!
//! The shell is not on `main` yet, so the tab lives here and can be merged
//! later. Register [`TAB_ID`] / [`TAB_LABEL`] and call [`draw_animation_tab`]
//! inside that tab. [`AnimationTab::posed_layout`] is the layout with motion
//! applied at the playhead. [`NodeId`] is a `u32` newtype so it can match the
//! design document's node id when that crate lands.
//!
//! The standalone host is `uii-animation-host`. It opens one window and draws
//! this tab. It does not implement Design, Function, the assets dock, Inspect,
//! import/export, or the Cursor plugin.

mod draw;
mod easing;
mod model;
mod tab;
mod timeline;

pub use draw::draw_animation_tab;
pub use easing::{interpolate, Easing};
pub use model::{
    apply_pose, AnimationError, AnimationProject, ChannelSample, Clip, ClipId, Keyframe,
    KeyframeId, LayoutNode, MotionProperty, NodeId, Pose, Track, TrackId,
};
pub use tab::{AnimationTab, Playback};
pub use timeline::{
    active_key_refs, fit_preview, hit_keyframe, rotated_corners, timeline_rows, KeyRef,
    KeyframeHit, PreviewFit, TimelineRow, TimelineView,
};

/// Stable id the shell should use when it registers top-level tabs.
pub const TAB_ID: &str = "animation";

/// Label drawn on the top-level tab.
pub const TAB_LABEL: &str = "Animation";
