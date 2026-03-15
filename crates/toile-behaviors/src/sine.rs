use serde::{Deserialize, Serialize};
use crate::types::EntityState;

/// Oscillates a property over time (floating platforms, pulsing effects).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SineConfig {
    pub property: SineProperty,
    pub magnitude: f32,
    pub period: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SineProperty {
    X,
    Y,
    Angle,
    Opacity,
    Size,
}

impl Default for SineConfig {
    fn default() -> Self {
        Self {
            property: SineProperty::Y,
            magnitude: 20.0,
            period: 2.0,
        }
    }
}

pub struct SineState {
    pub time: f32,
    pub base_value: f32,
    pub initialized: bool,
}

impl Default for SineState {
    fn default() -> Self {
        Self { time: 0.0, base_value: 0.0, initialized: false }
    }
}

pub fn update(config: &SineConfig, state: &mut SineState, entity: &mut EntityState, dt: f32) {
    if !state.initialized {
        state.base_value = match config.property {
            SineProperty::X => entity.position.x,
            SineProperty::Y => entity.position.y,
            SineProperty::Angle => entity.rotation,
            SineProperty::Opacity => entity.opacity,
            SineProperty::Size => entity.size.x,
        };
        state.initialized = true;
    }

    state.time += dt;
    let wave = (state.time * std::f32::consts::TAU / config.period).sin() * config.magnitude;

    match config.property {
        SineProperty::X => entity.position.x = state.base_value + wave,
        SineProperty::Y => entity.position.y = state.base_value + wave,
        SineProperty::Angle => entity.rotation = state.base_value + wave,
        SineProperty::Opacity => entity.opacity = (state.base_value + wave).clamp(0.0, 1.0),
        SineProperty::Size => {
            let s = (state.base_value + wave).max(1.0);
            entity.size.x = s;
            entity.size.y = s;
        }
    }
}
