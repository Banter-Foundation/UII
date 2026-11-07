//! Clips, tracks, and the sampled pose that moves a layout.

use serde::{Deserialize, Serialize};

use crate::easing::{interpolate, Easing};

const TIME_EPSILON: f32 = 1.0e-4;

/// Layout node id. Matches the design document's `NodeId(u32)` when that crate is linked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

/// One named motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClipId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyframeId(pub u32);

/// A layout channel the Animation tab can move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MotionProperty {
    X,
    Y,
    Width,
    Height,
    Rotation,
    Opacity,
    FillR,
    FillG,
    FillB,
    FillA,
}

impl MotionProperty {
    pub fn all() -> &'static [MotionProperty] {
        &[
            MotionProperty::X,
            MotionProperty::Y,
            MotionProperty::Width,
            MotionProperty::Height,
            MotionProperty::Rotation,
            MotionProperty::Opacity,
            MotionProperty::FillR,
            MotionProperty::FillG,
            MotionProperty::FillB,
            MotionProperty::FillA,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            MotionProperty::X => "X",
            MotionProperty::Y => "Y",
            MotionProperty::Width => "Width",
            MotionProperty::Height => "Height",
            MotionProperty::Rotation => "Rotation",
            MotionProperty::Opacity => "Opacity",
            MotionProperty::FillR => "Fill R",
            MotionProperty::FillG => "Fill G",
            MotionProperty::FillB => "Fill B",
            MotionProperty::FillA => "Fill A",
        }
    }
}

/// Rest pose of one layout node, plus the name shown in the Animation tab.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutNode {
    pub id: NodeId,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation_deg: f32,
    pub opacity: f32,
    pub fill: [f32; 4],
}

impl LayoutNode {
    /// Initial rest pose for a target the user just added.
    pub fn rest(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            x: 32.0,
            y: 32.0,
            width: 160.0,
            height: 64.0,
            rotation_deg: 0.0,
            opacity: 1.0,
            fill: [0.35, 0.55, 0.95, 1.0],
        }
    }

    pub fn channel(&self, property: MotionProperty) -> f32 {
        match property {
            MotionProperty::X => self.x,
            MotionProperty::Y => self.y,
            MotionProperty::Width => self.width,
            MotionProperty::Height => self.height,
            MotionProperty::Rotation => self.rotation_deg,
            MotionProperty::Opacity => self.opacity,
            MotionProperty::FillR => self.fill[0],
            MotionProperty::FillG => self.fill[1],
            MotionProperty::FillB => self.fill[2],
            MotionProperty::FillA => self.fill[3],
        }
    }

    pub fn set_channel(&mut self, property: MotionProperty, value: f32) {
        if !value.is_finite() {
            return;
        }
        match property {
            MotionProperty::X => self.x = value,
            MotionProperty::Y => self.y = value,
            MotionProperty::Width => self.width = value.max(0.0),
            MotionProperty::Height => self.height = value.max(0.0),
            MotionProperty::Rotation => self.rotation_deg = value,
            MotionProperty::Opacity => self.opacity = value.clamp(0.0, 1.0),
            MotionProperty::FillR => self.fill[0] = value.clamp(0.0, 1.0),
            MotionProperty::FillG => self.fill[1] = value.clamp(0.0, 1.0),
            MotionProperty::FillB => self.fill[2] = value.clamp(0.0, 1.0),
            MotionProperty::FillA => self.fill[3] = value.clamp(0.0, 1.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    id: KeyframeId,
    time: f32,
    value: f32,
    easing: Easing,
}

impl Keyframe {
    pub fn id(&self) -> KeyframeId {
        self.id
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn easing(&self) -> Easing {
        self.easing
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    id: TrackId,
    node: NodeId,
    property: MotionProperty,
    keyframes: Vec<Keyframe>,
}

impl Track {
    pub fn id(&self) -> TrackId {
        self.id
    }

    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn property(&self) -> MotionProperty {
        self.property
    }

    pub fn keyframes(&self) -> &[Keyframe] {
        &self.keyframes
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    id: ClipId,
    name: String,
    duration: f32,
    looping: bool,
    tracks: Vec<Track>,
}

impl Clip {
    pub fn id(&self) -> ClipId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn looping(&self) -> bool {
        self.looping
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelSample {
    pub node: NodeId,
    pub property: MotionProperty,
    pub value: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    channels: Vec<ChannelSample>,
}

impl Pose {
    pub fn get(&self, node: NodeId, property: MotionProperty) -> Option<f32> {
        self.channels
            .iter()
            .find(|channel| channel.node == node && channel.property == property)
            .map(|channel| channel.value)
    }

    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationError {
    UnknownClip,
    UnknownTarget,
    UnknownKeyframe,
    InvalidDuration,
    Json(String),
}

impl std::fmt::Display for AnimationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnimationError::UnknownClip => write!(f, "unknown clip"),
            AnimationError::UnknownTarget => write!(f, "unknown target"),
            AnimationError::UnknownKeyframe => write!(f, "unknown keyframe"),
            AnimationError::InvalidDuration => {
                write!(f, "duration must be zero or a positive finite number")
            }
            AnimationError::Json(message) => write!(f, "animation json: {message}"),
        }
    }
}

impl std::error::Error for AnimationError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimationProject {
    #[serde(default)]
    active: Option<ClipId>,
    #[serde(default)]
    clips: Vec<Clip>,
    #[serde(default)]
    targets: Vec<LayoutNode>,
    #[serde(default)]
    next_clip: u32,
    #[serde(default)]
    next_track: u32,
    #[serde(default)]
    next_key: u32,
    #[serde(default)]
    next_node: u32,
}

impl Default for AnimationProject {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationProject {
    pub fn new() -> Self {
        Self {
            active: None,
            clips: Vec::new(),
            targets: Vec::new(),
            next_clip: 1,
            next_track: 1,
            next_key: 1,
            next_node: 1,
        }
    }

    pub fn clips(&self) -> &[Clip] {
        &self.clips
    }

    pub fn clip(&self, id: ClipId) -> Option<&Clip> {
        self.clips.iter().find(|clip| clip.id == id)
    }

    pub fn active_clip(&self) -> Option<ClipId> {
        self.active
    }

    pub fn targets(&self) -> &[LayoutNode] {
        &self.targets
    }

    pub fn target(&self, id: NodeId) -> Option<&LayoutNode> {
        self.targets.iter().find(|target| target.id == id)
    }

    pub fn add_clip(&mut self, name: &str) -> ClipId {
        let name = unique_name(
            self.clips.iter().map(|clip| clip.name.as_str()),
            name,
            "Clip",
        );
        let id = ClipId(self.alloc_clip());
        self.clips.push(Clip {
            id,
            name,
            duration: 1.0,
            looping: false,
            tracks: Vec::new(),
        });
        if self.active.is_none() {
            self.active = Some(id);
        }
        id
    }

    pub fn rename_clip(&mut self, id: ClipId, name: &str) -> Result<(), AnimationError> {
        let clip = self.clip_mut(id)?;
        if let Some(name) = trimmed_or_none(name) {
            clip.name = name;
        }
        Ok(())
    }

    pub fn set_clip_duration(&mut self, id: ClipId, duration: f32) -> Result<(), AnimationError> {
        if !duration.is_finite() || duration < 0.0 {
            return Err(AnimationError::InvalidDuration);
        }
        let clip = self.clip_mut(id)?;
        clip.duration = duration;
        for track in &mut clip.tracks {
            clamp_keyframes(&mut track.keyframes, duration);
        }
        clip.tracks.retain(|track| !track.keyframes.is_empty());
        Ok(())
    }

    pub fn set_clip_loop(&mut self, id: ClipId, looping: bool) -> Result<(), AnimationError> {
        self.clip_mut(id)?.looping = looping;
        Ok(())
    }

    pub fn delete_clip(&mut self, id: ClipId) -> Result<(), AnimationError> {
        let index = self
            .clips
            .iter()
            .position(|clip| clip.id == id)
            .ok_or(AnimationError::UnknownClip)?;
        self.clips.remove(index);
        if self.active == Some(id) {
            self.active = None;
        }
        Ok(())
    }

    pub fn duplicate_clip(&mut self, id: ClipId) -> Result<ClipId, AnimationError> {
        let source = self.clip(id).ok_or(AnimationError::UnknownClip)?.clone();
        let name = unique_name(
            self.clips.iter().map(|clip| clip.name.as_str()),
            &format!("{} copy", source.name),
            "Clip copy",
        );
        let id = ClipId(self.alloc_clip());
        let mut tracks = Vec::new();
        for track in source.tracks {
            let mut keyframes = Vec::new();
            for key in track.keyframes {
                keyframes.push(Keyframe {
                    id: KeyframeId(self.alloc_key()),
                    time: key.time,
                    value: key.value,
                    easing: key.easing,
                });
            }
            tracks.push(Track {
                id: TrackId(self.alloc_track()),
                node: track.node,
                property: track.property,
                keyframes,
            });
        }
        self.clips.push(Clip {
            id,
            name,
            duration: source.duration,
            looping: source.looping,
            tracks,
        });
        Ok(id)
    }

    pub fn set_active_clip(&mut self, id: Option<ClipId>) -> Result<(), AnimationError> {
        if let Some(id) = id {
            if self.clip(id).is_none() {
                return Err(AnimationError::UnknownClip);
            }
        }
        self.active = id;
        Ok(())
    }

    pub fn add_target(&mut self, name: &str) -> NodeId {
        let name = unique_name(
            self.targets.iter().map(|target| target.name.as_str()),
            name,
            "Target",
        );
        let id = NodeId(self.alloc_node());
        self.targets.push(LayoutNode::rest(id, name));
        id
    }

    pub fn rename_target(&mut self, id: NodeId, name: &str) -> Result<(), AnimationError> {
        let target = self
            .targets
            .iter_mut()
            .find(|target| target.id == id)
            .ok_or(AnimationError::UnknownTarget)?;
        if let Some(name) = trimmed_or_none(name) {
            target.name = name;
        }
        Ok(())
    }

    pub fn set_rest(&mut self, id: NodeId, mut rest: LayoutNode) -> Result<(), AnimationError> {
        let target = self
            .targets
            .iter_mut()
            .find(|target| target.id == id)
            .ok_or(AnimationError::UnknownTarget)?;
        rest.id = id;
        *target = clamp_rest(rest);
        target.id = id;
        Ok(())
    }

    pub fn remove_target(&mut self, id: NodeId) -> Result<(), AnimationError> {
        let index = self
            .targets
            .iter()
            .position(|target| target.id == id)
            .ok_or(AnimationError::UnknownTarget)?;
        self.targets.remove(index);
        for clip in &mut self.clips {
            clip.tracks.retain(|track| track.node != id);
        }
        Ok(())
    }

    /// Update rest poses from the layout. Missing animation targets stay, so a
    /// clip is not dropped when the layout list is partial.
    pub fn sync_targets(&mut self, nodes: &[LayoutNode]) {
        for node in nodes {
            if let Some(existing) = self.targets.iter_mut().find(|target| target.id == node.id) {
                let id = existing.id;
                *existing = clamp_rest(node.clone());
                existing.id = id;
            } else {
                let mut created = clamp_rest(node.clone());
                created.id = node.id;
                self.targets.push(created);
                self.note_node(node.id);
            }
        }
    }

    pub fn upsert_keyframe(
        &mut self,
        clip: ClipId,
        node: NodeId,
        property: MotionProperty,
        time: f32,
        value: f32,
        easing: Easing,
    ) -> Result<KeyframeId, AnimationError> {
        self.require_target(node)?;
        if !value.is_finite() {
            return Err(AnimationError::Json("keyframe value is not finite".into()));
        }
        let duration = self.clip(clip).ok_or(AnimationError::UnknownClip)?.duration;
        let time = clamp_time(time, duration);
        let easing = easing.sanitized();
        let existing = self.clip(clip).and_then(|clip| {
            clip.tracks
                .iter()
                .find(|track| track.node == node && track.property == property)
                .and_then(|track| {
                    track
                        .keyframes
                        .iter()
                        .find(|key| (key.time - time).abs() <= TIME_EPSILON)
                        .map(|key| key.id)
                })
        });
        if let Some(id) = existing {
            let key = self.key_mut(clip, id)?;
            key.value = value;
            key.easing = easing;
            key.time = time;
            return Ok(id);
        }
        let has_track = self.clip(clip).is_some_and(|clip| {
            clip.tracks
                .iter()
                .any(|track| track.node == node && track.property == property)
        });
        let key_id = KeyframeId(self.alloc_key());
        let track_id = if has_track {
            None
        } else {
            Some(TrackId(self.alloc_track()))
        };
        let clip = self.clip_mut(clip)?;
        if let Some(track) = clip
            .tracks
            .iter_mut()
            .find(|track| track.node == node && track.property == property)
        {
            track.keyframes.push(Keyframe {
                id: key_id,
                time,
                value,
                easing,
            });
            track.keyframes.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            clip.tracks.push(Track {
                id: track_id.unwrap_or(TrackId(0)),
                node,
                property,
                keyframes: vec![Keyframe {
                    id: key_id,
                    time,
                    value,
                    easing,
                }],
            });
        }
        Ok(key_id)
    }

    pub fn move_keyframe(
        &mut self,
        clip: ClipId,
        id: KeyframeId,
        time: f32,
    ) -> Result<(), AnimationError> {
        let clip = self.clip_mut(clip)?;
        let time = clamp_time(time, clip.duration);
        let Some(track) = clip
            .tracks
            .iter_mut()
            .find(|track| track.keyframes.iter().any(|key| key.id == id))
        else {
            return Err(AnimationError::UnknownKeyframe);
        };
        let Some(key) = track.keyframes.iter_mut().find(|key| key.id == id) else {
            return Err(AnimationError::UnknownKeyframe);
        };
        key.time = time;
        track
            .keyframes
            .retain(|key| key.id == id || (key.time - time).abs() > TIME_EPSILON);
        track.keyframes.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(())
    }

    pub fn set_keyframe_value(
        &mut self,
        clip: ClipId,
        id: KeyframeId,
        value: f32,
    ) -> Result<(), AnimationError> {
        if !value.is_finite() {
            return Ok(());
        }
        let key = self.key_mut(clip, id)?;
        key.value = value;
        Ok(())
    }

    pub fn set_keyframe_easing(
        &mut self,
        clip: ClipId,
        id: KeyframeId,
        easing: Easing,
    ) -> Result<(), AnimationError> {
        let key = self.key_mut(clip, id)?;
        key.easing = easing.sanitized();
        Ok(())
    }

    pub fn delete_keyframe(&mut self, clip: ClipId, id: KeyframeId) -> Result<(), AnimationError> {
        let clip = self.clip_mut(clip)?;
        let Some(track_index) = clip
            .tracks
            .iter()
            .position(|track| track.keyframes.iter().any(|key| key.id == id))
        else {
            return Err(AnimationError::UnknownKeyframe);
        };
        clip.tracks[track_index]
            .keyframes
            .retain(|key| key.id != id);
        if clip.tracks[track_index].keyframes.is_empty() {
            clip.tracks.remove(track_index);
        }
        Ok(())
    }

    /// Write a keyframe at `time` using the sampled channel, or the rest pose
    /// when that channel has no keys yet.
    pub fn capture_keyframe(
        &mut self,
        clip: ClipId,
        node: NodeId,
        property: MotionProperty,
        time: f32,
    ) -> Result<KeyframeId, AnimationError> {
        let sampled = self.sample(clip, time)?.get(node, property);
        let value = match sampled {
            Some(value) => value,
            None => self
                .target(node)
                .ok_or(AnimationError::UnknownTarget)?
                .channel(property),
        };
        self.upsert_keyframe(clip, node, property, time, value, Easing::Linear)
    }

    pub fn sample(&self, clip: ClipId, time: f32) -> Result<Pose, AnimationError> {
        let clip = self.clip(clip).ok_or(AnimationError::UnknownClip)?;
        let time = clamp_time(time, clip.duration);
        let mut channels = Vec::new();
        for track in &clip.tracks {
            if let Some(value) = sample_track(&track.keyframes, time) {
                channels.push(ChannelSample {
                    node: track.node,
                    property: track.property,
                    value,
                });
            }
        }
        Ok(Pose { channels })
    }

    pub fn sample_active(&self, time: f32) -> Pose {
        match self.active.and_then(|id| self.sample(id, time).ok()) {
            Some(pose) => pose,
            None => Pose {
                channels: Vec::new(),
            },
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn from_json(text: &str) -> Result<Self, AnimationError> {
        let mut project: AnimationProject =
            serde_json::from_str(text).map_err(|err| AnimationError::Json(err.to_string()))?;
        project.sanitize()?;
        Ok(project)
    }

    fn sanitize(&mut self) -> Result<(), AnimationError> {
        for clip in &mut self.clips {
            if !clip.duration.is_finite() || clip.duration < 0.0 {
                return Err(AnimationError::InvalidDuration);
            }
            for track in &mut clip.tracks {
                for key in &mut track.keyframes {
                    if !key.value.is_finite() {
                        key.value = 0.0;
                    }
                    key.easing = key.easing.sanitized();
                }
                clamp_keyframes(&mut track.keyframes, clip.duration);
            }
            clip.tracks.retain(|track| !track.keyframes.is_empty());
        }
        if let Some(active) = self.active {
            if !self.clips.iter().any(|clip| clip.id == active) {
                self.active = None;
            }
        }
        for target in &mut self.targets {
            let id = target.id;
            *target = clamp_rest(target.clone());
            target.id = id;
        }
        self.recompute_ids();
        Ok(())
    }

    fn recompute_ids(&mut self) {
        let mut next_clip = self.next_clip.max(1);
        let mut next_track = self.next_track.max(1);
        let mut next_key = self.next_key.max(1);
        let mut next_node = self.next_node.max(1);
        for clip in &self.clips {
            next_clip = next_clip.max(clip.id.0.saturating_add(1));
            for track in &clip.tracks {
                next_track = next_track.max(track.id.0.saturating_add(1));
                for key in &track.keyframes {
                    next_key = next_key.max(key.id.0.saturating_add(1));
                }
            }
        }
        for target in &self.targets {
            next_node = next_node.max(target.id.0.saturating_add(1));
        }
        self.next_clip = next_clip;
        self.next_track = next_track;
        self.next_key = next_key;
        self.next_node = next_node;
    }

    fn clip_mut(&mut self, id: ClipId) -> Result<&mut Clip, AnimationError> {
        self.clips
            .iter_mut()
            .find(|clip| clip.id == id)
            .ok_or(AnimationError::UnknownClip)
    }

    fn require_target(&self, id: NodeId) -> Result<(), AnimationError> {
        if self.target(id).is_some() {
            Ok(())
        } else {
            Err(AnimationError::UnknownTarget)
        }
    }

    fn key_mut(&mut self, clip: ClipId, id: KeyframeId) -> Result<&mut Keyframe, AnimationError> {
        let clip = self.clip_mut(clip)?;
        clip.tracks
            .iter_mut()
            .find_map(|track| track.keyframes.iter_mut().find(|key| key.id == id))
            .ok_or(AnimationError::UnknownKeyframe)
    }

    fn alloc_clip(&mut self) -> u32 {
        let id = self.next_clip.max(1);
        self.next_clip = id.saturating_add(1);
        id
    }

    fn alloc_track(&mut self) -> u32 {
        let id = self.next_track.max(1);
        self.next_track = id.saturating_add(1);
        id
    }

    fn alloc_key(&mut self) -> u32 {
        let id = self.next_key.max(1);
        self.next_key = id.saturating_add(1);
        id
    }

    fn alloc_node(&mut self) -> u32 {
        let id = self.next_node.max(1);
        self.next_node = id.saturating_add(1);
        id
    }

    fn note_node(&mut self, id: NodeId) {
        if id.0.saturating_add(1) > self.next_node {
            self.next_node = id.0.saturating_add(1);
        }
    }
}

/// Apply sampled channels onto the rest pose. Size, opacity, and fill are clamped.
pub fn apply_pose(rest: &[LayoutNode], pose: &Pose) -> Vec<LayoutNode> {
    rest.iter()
        .map(|node| {
            let mut posed = node.clone();
            for property in MotionProperty::all() {
                if let Some(value) = pose.get(node.id, *property) {
                    posed.set_channel(*property, value);
                }
            }
            posed
        })
        .collect()
}

fn clamp_rest(mut node: LayoutNode) -> LayoutNode {
    if !node.x.is_finite() {
        node.x = 0.0;
    }
    if !node.y.is_finite() {
        node.y = 0.0;
    }
    if !node.rotation_deg.is_finite() {
        node.rotation_deg = 0.0;
    }
    node.width = if node.width.is_finite() {
        node.width.max(0.0)
    } else {
        0.0
    };
    node.height = if node.height.is_finite() {
        node.height.max(0.0)
    } else {
        0.0
    };
    node.opacity = if node.opacity.is_finite() {
        node.opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    for channel in &mut node.fill {
        *channel = if channel.is_finite() {
            channel.clamp(0.0, 1.0)
        } else {
            1.0
        };
    }
    node
}

fn clamp_time(time: f32, duration: f32) -> f32 {
    if !time.is_finite() {
        return 0.0;
    }
    if !duration.is_finite() || duration <= 0.0 {
        return 0.0;
    }
    time.clamp(0.0, duration)
}

fn clamp_keyframes(keys: &mut Vec<Keyframe>, duration: f32) {
    let mut tagged: Vec<(f32, Keyframe)> = keys
        .drain(..)
        .map(|mut key| {
            let original = key.time;
            key.time = clamp_time(original, duration);
            (original, key)
        })
        .collect();
    tagged.sort_by(|a, b| {
        a.1.time
            .partial_cmp(&b.1.time)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut kept: Vec<(f32, Keyframe)> = Vec::new();
    for (original, key) in tagged {
        if let Some((previous_original, previous)) = kept.last_mut() {
            if (previous.time - key.time).abs() <= TIME_EPSILON {
                if original >= *previous_original {
                    *previous_original = original;
                    *previous = key;
                }
            } else {
                kept.push((original, key));
            }
        } else {
            kept.push((original, key));
        }
    }
    *keys = kept.into_iter().map(|(_, key)| key).collect();
}

fn sample_track(keys: &[Keyframe], time: f32) -> Option<f32> {
    if keys.is_empty() {
        return None;
    }
    if time <= keys[0].time {
        return Some(keys[0].value);
    }
    if time >= keys[keys.len() - 1].time {
        return Some(keys[keys.len() - 1].value);
    }
    let right = keys.partition_point(|key| key.time <= time);
    let left = &keys[right - 1];
    if (left.time - time).abs() <= TIME_EPSILON || right >= keys.len() {
        return Some(left.value);
    }
    let next = &keys[right];
    let span = (next.time - left.time).max(TIME_EPSILON);
    let t = (time - left.time) / span;
    Some(interpolate(left.value, next.value, t, left.easing))
}

fn trimmed_or_none(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn unique_name<'a>(
    existing: impl Iterator<Item = &'a str>,
    requested: &str,
    fallback: &str,
) -> String {
    let base = trimmed_or_none(requested).unwrap_or_else(|| fallback.to_string());
    let existing: Vec<&str> = existing.collect();
    if !existing.iter().any(|name| *name == base) {
        return base;
    }
    let mut index = 2;
    loop {
        let candidate = format!("{base} {index}");
        if !existing.iter().any(|name| *name == candidate) {
            return candidate;
        }
        index += 1;
    }
}
