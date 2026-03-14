use crate::tween::{Easing, Tween};

/// Transition effect between scenes.
#[derive(Debug, Clone)]
pub enum TransitionKind {
    Fade,
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
}

/// Describes a transition between two scenes.
#[derive(Debug, Clone)]
pub struct Transition {
    pub kind: TransitionKind,
    pub duration: f32,
    pub easing: Easing,
}

impl Transition {
    pub fn fade(duration: f32) -> Self {
        Self {
            kind: TransitionKind::Fade,
            duration,
            easing: Easing::SineInOut,
        }
    }

    pub fn slide_left(duration: f32) -> Self {
        Self {
            kind: TransitionKind::SlideLeft,
            duration,
            easing: Easing::CubicOut,
        }
    }

    pub fn slide_right(duration: f32) -> Self {
        Self {
            kind: TransitionKind::SlideRight,
            duration,
            easing: Easing::CubicOut,
        }
    }

    /// Get the interpolation value at the given elapsed time (0..1).
    pub fn progress(&self, elapsed: f32) -> f32 {
        let t = (elapsed / self.duration).clamp(0.0, 1.0);
        self.easing.apply(t)
    }

    pub fn is_done(&self, elapsed: f32) -> bool {
        elapsed >= self.duration
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::fade(0.3)
    }
}
