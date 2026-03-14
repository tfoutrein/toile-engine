use std::f32::consts::PI;

/// Easing functions for animation interpolation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    SineIn,
    SineOut,
    SineInOut,
    ExpoIn,
    ExpoOut,
    BackIn,
    BackOut,
    BounceOut,
}

impl Easing {
    /// Apply the easing function to a normalized time `t` in 0..1.
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::QuadIn => t * t,
            Easing::QuadOut => t * (2.0 - t),
            Easing::QuadInOut => {
                if t < 0.5 { 2.0 * t * t } else { -1.0 + (4.0 - 2.0 * t) * t }
            }
            Easing::CubicIn => t * t * t,
            Easing::CubicOut => {
                let t = t - 1.0;
                t * t * t + 1.0
            }
            Easing::CubicInOut => {
                if t < 0.5 { 4.0 * t * t * t } else {
                    let t = 2.0 * t - 2.0;
                    0.5 * t * t * t + 1.0
                }
            }
            Easing::SineIn => 1.0 - (t * PI * 0.5).cos(),
            Easing::SineOut => (t * PI * 0.5).sin(),
            Easing::SineInOut => 0.5 * (1.0 - (PI * t).cos()),
            Easing::ExpoIn => {
                if t == 0.0 { 0.0 } else { (10.0 * (t - 1.0)).exp2() }
            }
            Easing::ExpoOut => {
                if t == 1.0 { 1.0 } else { 1.0 - (-10.0 * t).exp2() }
            }
            Easing::BackIn => {
                let s = 1.70158;
                t * t * ((s + 1.0) * t - s)
            }
            Easing::BackOut => {
                let s = 1.70158;
                let t = t - 1.0;
                t * t * ((s + 1.0) * t + s) + 1.0
            }
            Easing::BounceOut => bounce_out(t),
        }
    }
}

fn bounce_out(t: f32) -> f32 {
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t = t - 1.5 / 2.75;
        7.5625 * t * t + 0.75
    } else if t < 2.5 / 2.75 {
        let t = t - 2.25 / 2.75;
        7.5625 * t * t + 0.9375
    } else {
        let t = t - 2.625 / 2.75;
        7.5625 * t * t + 0.984375
    }
}

/// Playback mode for tweens and animations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepeatMode {
    Once,
    Loop,
    PingPong,
}

/// Interpolates a value from `from` to `to` over `duration` seconds.
#[derive(Debug, Clone)]
pub struct Tween {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: Easing,
    pub repeat: RepeatMode,
    done: bool,
}

impl Tween {
    pub fn new(from: f32, to: f32, duration: f32) -> Self {
        Self {
            from,
            to,
            duration: duration.max(0.001),
            elapsed: 0.0,
            easing: Easing::Linear,
            repeat: RepeatMode::Once,
            done: false,
        }
    }

    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn with_repeat(mut self, repeat: RepeatMode) -> Self {
        self.repeat = repeat;
        self
    }

    /// Advance the tween by `dt` seconds. Returns the current value.
    pub fn advance(&mut self, dt: f32) -> f32 {
        if self.done {
            return self.to;
        }

        self.elapsed += dt;

        match self.repeat {
            RepeatMode::Once => {
                if self.elapsed >= self.duration {
                    self.elapsed = self.duration;
                    self.done = true;
                }
            }
            RepeatMode::Loop => {
                while self.elapsed >= self.duration {
                    self.elapsed -= self.duration;
                }
            }
            RepeatMode::PingPong => {
                let cycle = self.duration * 2.0;
                while self.elapsed >= cycle {
                    self.elapsed -= cycle;
                }
            }
        }

        self.value()
    }

    /// Get the current value without advancing.
    pub fn value(&self) -> f32 {
        let t = match self.repeat {
            RepeatMode::PingPong => {
                let t = self.elapsed / self.duration;
                if t <= 1.0 { t } else { 2.0 - t }
            }
            _ => (self.elapsed / self.duration).min(1.0),
        };

        let eased = self.easing.apply(t);
        self.from + (self.to - self.from) * eased
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.done = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_tween() {
        let mut tw = Tween::new(0.0, 100.0, 1.0);
        assert!((tw.advance(0.5) - 50.0).abs() < 0.01);
        assert!(!tw.is_done());
        assert!((tw.advance(0.5) - 100.0).abs() < 0.01);
        assert!(tw.is_done());
    }

    #[test]
    fn loop_tween() {
        let mut tw = Tween::new(0.0, 1.0, 1.0).with_repeat(RepeatMode::Loop);
        tw.advance(1.5);
        assert!(!tw.is_done());
        assert!((tw.value() - 0.5).abs() < 0.01);
    }

    #[test]
    fn pingpong_tween() {
        let mut tw = Tween::new(0.0, 100.0, 1.0).with_repeat(RepeatMode::PingPong);
        assert!((tw.advance(0.5) - 50.0).abs() < 0.1);
        assert!((tw.advance(0.5) - 100.0).abs() < 0.1);
        assert!((tw.advance(0.5) - 50.0).abs() < 0.1);
    }

    #[test]
    fn easing_bounds() {
        for easing in [
            Easing::Linear, Easing::QuadIn, Easing::QuadOut, Easing::CubicIn,
            Easing::SineIn, Easing::SineOut, Easing::ExpoIn, Easing::BounceOut,
        ] {
            assert!((easing.apply(0.0)).abs() < 0.01, "{easing:?} at 0");
            assert!((easing.apply(1.0) - 1.0).abs() < 0.01, "{easing:?} at 1");
        }
    }
}
