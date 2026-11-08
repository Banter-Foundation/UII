//! Segment easing. The value on a keyframe describes the curve to the next one.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// Stay on the left value until the next keyframe.
    Hold,
}

impl Easing {
    pub fn label(self) -> &'static str {
        match self {
            Easing::Linear => "Linear",
            Easing::EaseIn => "Ease in",
            Easing::EaseOut => "Ease out",
            Easing::EaseInOut => "Ease in out",
            Easing::Hold => "Hold",
        }
    }

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
        }
    }
}

impl Default for Easing {
    fn default() -> Self {
        Easing::Linear
    }
}
