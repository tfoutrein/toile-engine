use glam::Vec2;
use serde::{Deserialize, Serialize};
use crate::types::{BehaviorInput, EntityState};

/// Top-down 4/8-direction movement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopDownConfig {
    pub max_speed: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub diagonal_correction: bool,
}

impl Default for TopDownConfig {
    fn default() -> Self {
        Self {
            max_speed: 200.0,
            acceleration: 1200.0,
            deceleration: 1000.0,
            diagonal_correction: true,
        }
    }
}

pub fn update(config: &TopDownConfig, entity: &mut EntityState, input: &BehaviorInput, dt: f32) {
    let mut dir = Vec2::ZERO;
    if input.right { dir.x += 1.0; }
    if input.left { dir.x -= 1.0; }
    if input.up { dir.y += 1.0; }
    if input.down { dir.y -= 1.0; }

    if config.diagonal_correction && dir.length_squared() > 1.0 {
        dir = dir.normalize();
    }

    let target = dir * config.max_speed;

    if dir.length_squared() > 0.01 {
        let diff = target - entity.velocity;
        let accel = diff.normalize_or_zero() * config.acceleration * dt;
        if accel.length() > diff.length() {
            entity.velocity = target;
        } else {
            entity.velocity += accel;
        }
    } else {
        let decel = config.deceleration * dt;
        if entity.velocity.length() < decel {
            entity.velocity = Vec2::ZERO;
        } else {
            entity.velocity -= entity.velocity.normalize() * decel;
        }
    }

    entity.position += entity.velocity * dt;
}
