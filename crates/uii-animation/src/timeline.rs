//! Timeline geometry. The draw code uses these so hit testing and zoom stay
//! independent of Dear ImGui.

use crate::model::{AnimationProject, KeyframeId, MotionProperty, NodeId};

/// Seconds visible in the timeline, and how wide one second is.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineView {
    /// Seconds at the left edge of the track area. Never negative.
    pub origin: f32,
    pub pixels_per_second: f32,
}

impl TimelineView {
    pub fn new() -> Self {
        Self {
            origin: 0.0,
            pixels_per_second: 120.0,
        }
    }

    pub fn time_to_x(&self, time: f32, left: f32) -> f32 {
        left + (time - self.origin) * self.pixels_per_second
    }

    pub fn x_to_time(&self, x: f32, left: f32) -> f32 {
        if self.pixels_per_second <= 0.0 {
            return self.origin;
        }
        self.origin + (x - left) / self.pixels_per_second
    }

    /// Zoom by `factor`, keeping `anchor_time` under the same pixel.
    pub fn zoom_at(&mut self, factor: f32, anchor_time: f32, left: f32, _width: f32) {
        let factor = if factor.is_finite() && factor > 0.0 {
            factor
        } else {
            1.0
        };
        let anchor_x = self.time_to_x(anchor_time, left);
        self.pixels_per_second = (self.pixels_per_second * factor).clamp(20.0, 2000.0);
        if self.pixels_per_second > 0.0 {
            self.origin = anchor_time - (anchor_x - left) / self.pixels_per_second;
        }
        if !self.origin.is_finite() || self.origin < 0.0 {
            self.origin = 0.0;
        }
    }

    /// Move the visible range. Positive `dx_pixels` reveals earlier time.
    pub fn pan(&mut self, dx_pixels: f32) {
        if !dx_pixels.is_finite() || self.pixels_per_second <= 0.0 {
            return;
        }
        self.origin -= dx_pixels / self.pixels_per_second;
        if !self.origin.is_finite() || self.origin < 0.0 {
            self.origin = 0.0;
        }
    }
}

impl Default for TimelineView {
    fn default() -> Self {
        Self::new()
    }
}

/// One property row in the timeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineRow {
    pub node: NodeId,
    pub property: MotionProperty,
    pub y: f32,
    pub height: f32,
}

/// A keyframe the hit test can see.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyRef {
    pub id: KeyframeId,
    pub node: NodeId,
    pub property: MotionProperty,
    pub time: f32,
}

/// The keyframe under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyframeHit {
    pub keyframe: KeyframeId,
    pub node: NodeId,
    pub property: MotionProperty,
}

/// Selected target lists every layout channel. Other targets list only the
/// channels that have keyframes on the active clip.
pub fn timeline_rows(
    project: &AnimationProject,
    selected: Option<NodeId>,
    row_height: f32,
) -> Vec<TimelineRow> {
    let mut rows = Vec::new();
    let mut y = 0.0;
    let active = project.active_clip().and_then(|id| project.clip(id));
    for target in project.targets() {
        let properties: Vec<MotionProperty> = if Some(target.id) == selected {
            MotionProperty::all().to_vec()
        } else {
            MotionProperty::all()
                .iter()
                .copied()
                .filter(|property| {
                    active.is_some_and(|clip| {
                        clip.tracks()
                            .iter()
                            .any(|track| track.node() == target.id && track.property() == *property)
                    })
                })
                .collect()
        };
        for property in properties {
            rows.push(TimelineRow {
                node: target.id,
                property,
                y,
                height: row_height,
            });
            y += row_height;
        }
    }
    rows
}

pub fn active_key_refs(project: &AnimationProject) -> Vec<KeyRef> {
    let Some(clip) = project.active_clip().and_then(|id| project.clip(id)) else {
        return Vec::new();
    };
    let mut refs = Vec::new();
    for track in clip.tracks() {
        for key in track.keyframes() {
            refs.push(KeyRef {
                id: key.id(),
                node: track.node(),
                property: track.property(),
                time: key.time(),
            });
        }
    }
    refs
}

pub fn hit_keyframe(
    x: f32,
    y: f32,
    rows: &[TimelineRow],
    keys: &[KeyRef],
    view: &TimelineView,
    left: f32,
    radius: f32,
) -> Option<KeyframeHit> {
    let mut best: Option<(f32, KeyframeHit)> = None;
    for key in keys {
        let Some(row) = rows
            .iter()
            .find(|row| row.node == key.node && row.property == key.property)
        else {
            continue;
        };
        let kx = view.time_to_x(key.time, left);
        let ky = row.y + row.height * 0.5;
        let dx = x - kx;
        let dy = y - ky;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= radius && best.as_ref().is_none_or(|(best_dist, _)| dist < *best_dist) {
            best = Some((
                dist,
                KeyframeHit {
                    keyframe: key.id,
                    node: key.node,
                    property: key.property,
                },
            ));
        }
    }
    best.map(|(_, hit)| hit)
}

/// Fit of layout space into the motion preview.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewFit {
    pub scale: f32,
    pub origin_x: f32,
    pub origin_y: f32,
}

/// Axis-aligned fit of node rectangles. Rotation is drawn after the fit, so a
/// turned box can extend past the padding.
pub fn fit_preview(
    nodes: &[crate::model::LayoutNode],
    width: f32,
    height: f32,
    padding: f32,
) -> Option<PreviewFit> {
    if nodes.is_empty() || !(width > padding * 2.0) || !(height > padding * 2.0) {
        return None;
    }
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for node in nodes {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
        max_x = max_x.max(node.x + node.width.max(0.0));
        max_y = max_y.max(node.y + node.height.max(0.0));
    }
    let bounds_w = (max_x - min_x).max(1.0);
    let bounds_h = (max_y - min_y).max(1.0);
    let inner_w = width - padding * 2.0;
    let inner_h = height - padding * 2.0;
    let scale = (inner_w / bounds_w).min(inner_h / bounds_h);
    let content_w = bounds_w * scale;
    let content_h = bounds_h * scale;
    Some(PreviewFit {
        scale,
        origin_x: padding + (inner_w - content_w) * 0.5 - min_x * scale,
        origin_y: padding + (inner_h - content_h) * 0.5 - min_y * scale,
    })
}

/// Corners of a rectangle rotated clockwise on a y-down screen.
/// Order is top-left, top-right, bottom-right, bottom-left before rotation.
pub fn rotated_corners(x: f32, y: f32, w: f32, h: f32, degrees: f32) -> [[f32; 2]; 4] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let local = [
        [-w * 0.5, -h * 0.5],
        [w * 0.5, -h * 0.5],
        [w * 0.5, h * 0.5],
        [-w * 0.5, h * 0.5],
    ];
    let mut corners = [[0.0; 2]; 4];
    for (index, [dx, dy]) in local.into_iter().enumerate() {
        let rx = dx * cos + dy * sin;
        let ry = -dx * sin + dy * cos;
        corners[index] = [cx + rx, cy + ry];
    }
    corners
}
