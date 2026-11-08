//! Animation tab for UII.
//!
//! UII is a native layout tool on Banter's Dear ImGui (`dear-imgui-rs` 0.22,
//! the same docking branch Banter ships). This crate is only the Animation
//! tab: clips, property tracks, and keyframes that move layout nodes.
//!
//! The editor window hosts this tab. Register [`TAB_ID`] / [`TAB_LABEL`] and
//! call [`draw_animation_tab`] inside the Animation tab.
//! [`AnimationTab::posed_layout`] is the layout with motion applied at the
//! playhead. [`NodeId`] is a `u32` newtype matching the design document.
//! [`load_animation`] and [`save_animation`] read and write the clip file.
//!
//! `uii-animation-host` can still open the same tab in its own window. The
//! editor is the host that runs with Design and Function.

mod draw;
mod easing;
mod model;
mod store;
mod tab;
mod timeline;

pub use draw::draw_animation_tab;
pub use easing::{interpolate, Easing};
pub use model::{
    apply_pose, AnimationError, AnimationProject, ChannelSample, Clip, ClipId, Keyframe,
    KeyframeId, LayoutNode, MotionProperty, NodeId, Pose, Track, TrackId,
};
pub use store::{animation_path, load_animation, save_animation, AnimationFile};
pub use tab::{AnimationTab, Playback};
pub use timeline::{
    active_key_refs, fit_preview, hit_keyframe, rotated_corners, timeline_rows, KeyRef,
    KeyframeHit, PreviewFit, TimelineRow, TimelineView,
};

/// Stable id the shell should use when it registers top-level tabs.
pub const TAB_ID: &str = "animation";

/// Label drawn on the top-level tab.
pub const TAB_LABEL: &str = "Animation";
