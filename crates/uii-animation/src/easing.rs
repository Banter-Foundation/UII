//! Segment easing. The value on a keyframe describes the curve from that
//! keyframe to the next one.

use serde::{Deserialize, Serialize};

/// How a track moves from one keyframe to the next.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// Stay on the left value until the next keyframe.
    Hold,
    /// CSS-style cubic bezier. Control `x` is clamped to `0..=1`.
    CubicBezier {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
}

impl Easing {
    pub fn label(self) -> &'static str {
        match self {
            Easing::Linear => "Linear",
            Easing::EaseIn => "Ease In",
            Easing::EaseOut => "Ease Out",
            Easing::EaseInOut => "Ease In Out",
            Easing::Hold => "Hold",
            Easing::CubicBezier { .. } => "Cubic Bezier",
        }
    }

    pub fn presets() -> [Easing; 6] {
        [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::Hold,
            Easing::CubicBezier {
                x1: 0.42,
                y1: 0.0,
                x2: 0.58,
                y2: 1.0,
            },
        ]
    }

    /// Map segment progress `t` in `0..=1` through the curve.
    pub fn apply(self, t: f32) -> f32 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t * t,
            Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::Hold => {
                if t < 1.0 {
                    0.0
                } else {
                    1.0
                }
            }
            Easing::CubicBezier { x1, y1, x2, y2 } => {
                if ![x1, y1, x2, y2].into_iter().all(|value| value.is_finite()) {
                    return t;
                }
                let x1 = x1.clamp(0.0, 1.0);
                let x2 = x2.clamp(0.0, 1.0);
                let solved = solve_bezier_x(x1, x2, t);
                bezier_component(y1, y2, solved)
            }
        }
    }

    pub(crate) fn sanitized(self) -> Self {
        match self {
            Easing::CubicBezier { x1, y1, x2, y2 } => {
                if ![x1, y1, x2, y2].into_iter().all(|value| value.is_finite()) {
                    Easing::Linear
                } else {
                    Easing::CubicBezier {
                        x1: x1.clamp(0.0, 1.0),
                        y1,
                        x2: x2.clamp(0.0, 1.0),
                        y2,
                    }
                }
            }
            other => other,
        }
    }
}

/// Blend `left` toward `right` by eased progress `t`.
pub fn interpolate(left: f32, right: f32, t: f32, easing: Easing) -> f32 {
    let u = easing.apply(t);
    left + (right - left) * u
}

fn bezier_component(c1: f32, c2: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    3.0 * u * u * t * c1 + 3.0 * u * t * t * c2 + t * t * t
}

fn bezier_slope(c1: f32, c2: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    3.0 * u * u * c1 + 6.0 * u * t * (c2 - c1) + 3.0 * t * t * (1.0 - c2)
}

fn solve_bezier_x(x1: f32, x2: f32, x: f32) -> f32 {
    let mut t = x.clamp(0.0, 1.0);
    for _ in 0..8 {
        let error = bezier_component(x1, x2, t) - x;
        if error.abs() < 1.0e-6 {
            return t.clamp(0.0, 1.0);
        }
        let slope = bezier_slope(x1, x2, t);
        if slope.abs() < 1.0e-6 {
            break;
        }
        t = (t - error / slope).clamp(0.0, 1.0);
    }
    let mut lo = 0.0;
    let mut hi = 1.0;
    for _ in 0..24 {
        let mid = (lo + hi) * 0.5;
        if bezier_component(x1, x2, mid) < x {
            lo = mid;
        } else {
            hi = mid;
        }
        t = mid;
    }
    t
}
