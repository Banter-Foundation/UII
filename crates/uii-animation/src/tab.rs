//! Editor state around the animation project: playback, undo, and the pose
//! the shell paints on the layout.

use crate::model::{
    apply_pose, AnimationProject, ClipId, KeyframeId, LayoutNode, MotionProperty, NodeId,
};
use crate::timeline::TimelineView;

const HISTORY_LIMIT: usize = 100;

/// Playback clock. Time is in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct Playback {
    pub time: f32,
    pub playing: bool,
}

impl Playback {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            playing: false,
        }
    }

    pub fn play(&mut self, duration: f32, looping: bool) {
        if !looping && duration > 0.0 && self.time >= duration {
            self.time = 0.0;
        }
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn stop(&mut self) {
        self.time = 0.0;
        self.playing = false;
    }

    pub fn seek(&mut self, time: f32, duration: f32) {
        let duration = if duration.is_finite() && duration > 0.0 {
            duration
        } else {
            0.0
        };
        let time = if time.is_finite() { time } else { 0.0 };
        self.time = time.clamp(0.0, duration);
    }

    pub fn tick(&mut self, dt: f32, duration: f32, looping: bool) {
        if !self.playing || !(dt > 0.0) || !dt.is_finite() {
            return;
        }
        if !(duration > 0.0) || !duration.is_finite() {
            self.time = 0.0;
            return;
        }
        let next = self.time + dt;
        if looping {
            self.time = next.rem_euclid(duration);
        } else if next >= duration {
            self.time = duration;
            self.playing = false;
        } else {
            self.time = next.max(0.0);
        }
    }
}

impl Default for Playback {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Drag {
    None,
    Keyframe { id: KeyframeId, origin_time: f32 },
    Playhead,
}

/// The Animation tab. `project` is the document. `posed_layout` is what the
/// playhead does to the rest pose.
pub struct AnimationTab {
    pub project: AnimationProject,
    pub playback: Playback,
    pub view: TimelineView,
    pub(crate) selected_target: Option<NodeId>,
    pub(crate) selected_property: Option<MotionProperty>,
    pub(crate) selected_keyframe: Option<KeyframeId>,
    undo: Vec<AnimationProject>,
    redo: Vec<AnimationProject>,
    dirty: bool,
    pub(crate) drag: Drag,
    pub(crate) drag_before: Option<AnimationProject>,
    pub(crate) name_buf: String,
    pub(crate) name_for: Option<NodeId>,
    pub(crate) clip_name_buf: String,
    pub(crate) clip_name_for: Option<ClipId>,
    status: String,
}

impl Default for AnimationTab {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationTab {
    pub fn new() -> Self {
        Self {
            project: AnimationProject::new(),
            playback: Playback::new(),
            view: TimelineView::new(),
            selected_target: None,
            selected_property: None,
            selected_keyframe: None,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            drag: Drag::None,
            drag_before: None,
            name_buf: String::new(),
            name_for: None,
            clip_name_buf: String::new(),
            clip_name_for: None,
            status: String::new(),
        }
    }

    pub fn edit(&mut self, f: impl FnOnce(&mut AnimationProject)) {
        self.commit_drag();
        let before = self.project.clone();
        f(&mut self.project);
        self.note_change(before);
    }

    pub fn edit_with<T>(&mut self, f: impl FnOnce(&mut AnimationProject) -> T) -> T {
        self.commit_drag();
        let before = self.project.clone();
        let value = f(&mut self.project);
        self.note_change(before);
        value
    }

    pub fn undo(&mut self) -> bool {
        self.commit_drag();
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.project, previous);
        self.redo.push(current);
        self.dirty = true;
        true
    }

    pub fn redo(&mut self) -> bool {
        self.commit_drag();
        let Some(next) = self.redo.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.project, next);
        self.undo.push(current);
        self.dirty = true;
        true
    }

    pub fn tick(&mut self, dt: f32) {
        let (duration, looping) = self.active_timing();
        self.playback.tick(dt, duration, looping);
    }

    /// Rest targets with the active clip sampled at the playhead.
    pub fn posed_layout(&self) -> Vec<LayoutNode> {
        let Some(id) = self.project.active_clip() else {
            return self.project.targets().to_vec();
        };
        match self.project.sample(id, self.playback.time) {
            Ok(pose) => apply_pose(self.project.targets(), &pose),
            Err(_) => self.project.targets().to_vec(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn take_dirty(&mut self) -> bool {
        let dirty = self.dirty;
        self.dirty = false;
        dirty
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = text.into();
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub(crate) fn active_timing(&self) -> (f32, bool) {
        self.project
            .active_clip()
            .and_then(|id| self.project.clip(id))
            .map(|clip| (clip.duration(), clip.looping()))
            .unwrap_or((0.0, false))
    }

    pub(crate) fn begin_drag(&mut self) {
        if self.drag_before.is_none() {
            self.drag_before = Some(self.project.clone());
        }
    }

    pub(crate) fn commit_drag(&mut self) {
        self.drag = Drag::None;
        let Some(before) = self.drag_before.take() else {
            return;
        };
        self.note_change(before);
    }

    fn note_change(&mut self, before: AnimationProject) {
        if self.project != before {
            self.redo.clear();
            self.undo.push(before);
            if self.undo.len() > HISTORY_LIMIT {
                self.undo.remove(0);
            }
            self.dirty = true;
        }
    }
}
