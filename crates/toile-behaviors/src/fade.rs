use serde::{Deserialize, Serialize};
use crate::types::EntityState;

/// Fades opacity in/out, optionally destroying the entity after fade-out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FadeConfig {
    pub fade_in_time: f32,
    pub fade_out_time: f32,
    pub destroy_on_fade_out: bool,
}

impl Default for FadeConfig {
    fn default() -> Self {
        Self {
            fade_in_time: 0.0,
            fade_out_time: 1.0,
            destroy_on_fade_out: true,
        }
    }
}

pub struct FadeState {
    pub elapsed: f32,
    pub phase: FadePhase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FadePhase {
    FadingIn,
    Visible,
    FadingOut,
    Done,
}

impl Default for FadeState {
    fn default() -> Self {
        Self { elapsed: 0.0, phase: FadePhase::FadingIn }
    }
}

pub fn update(config: &FadeConfig, state: &mut FadeState, entity: &mut EntityState, dt: f32) {
    state.elapsed += dt;

    match state.phase {
        FadePhase::FadingIn => {
            if config.fade_in_time <= 0.0 {
                entity.opacity = 1.0;
                state.phase = FadePhase::Visible;
                state.elapsed = 0.0;
            } else {
                let t = (state.elapsed / config.fade_in_time).min(1.0);
                entity.opacity = t;
                if t >= 1.0 {
                    state.phase = FadePhase::Visible;
                    state.elapsed = 0.0;
                }
            }
        }
        FadePhase::Visible => {
            // Transition to fade out (caller can trigger this)
        }
        FadePhase::FadingOut => {
            if config.fade_out_time <= 0.0 {
                entity.opacity = 0.0;
                state.phase = FadePhase::Done;
            } else {
                let t = (state.elapsed / config.fade_out_time).min(1.0);
                entity.opacity = 1.0 - t;
                if t >= 1.0 {
                    state.phase = FadePhase::Done;
                    if config.destroy_on_fade_out {
                        entity.alive = false;
                    }
                }
            }
        }
        FadePhase::Done => {}
    }
}

/// Start the fade-out phase.
pub fn start_fade_out(state: &mut FadeState) {
    state.phase = FadePhase::FadingOut;
    state.elapsed = 0.0;
}
