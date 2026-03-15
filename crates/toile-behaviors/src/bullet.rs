use glam::Vec2;
use serde::{Deserialize, Serialize};
use crate::types::EntityState;

/// Moves in a straight line. For projectiles, enemies, moving platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletConfig {
    pub speed: f32,
    pub acceleration: f32,
    pub gravity: f32,
    pub angle_degrees: f32,
}

impl Default for BulletConfig {
    fn default() -> Self {
        Self {
            speed: 300.0,
            acceleration: 0.0,
            gravity: 0.0,
            angle_degrees: 0.0,
        }
    }
}

pub struct BulletState {
    pub current_speed: f32,
    pub initialized: bool,
}

impl Default for BulletState {
    fn default() -> Self {
        Self {
            current_speed: 0.0,
            initialized: false,
        }
    }
}

pub fn update(config: &BulletConfig, state: &mut BulletState, entity: &mut EntityState, dt: f32) {
    if !state.initialized {
        state.current_speed = config.speed;
        state.initialized = true;
    }

    state.current_speed += config.acceleration * dt;

    let rad = config.angle_degrees.to_radians();
    let dir = Vec2::new(rad.cos(), rad.sin());

    entity.velocity = dir * state.current_speed;
    entity.velocity.y -= config.gravity * dt;
    entity.position += entity.velocity * dt;
}
