use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Input state snapshot passed to behaviors each tick.
pub struct BehaviorInput {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump_pressed: bool,
    pub jump_down: bool,
}

/// Entity state that behaviors read and write.
pub struct EntityState {
    pub position: Vec2,
    pub velocity: Vec2,
    pub rotation: f32,
    pub on_ground: bool,
    pub size: Vec2,
    pub opacity: f32,
    pub alive: bool,
}

/// Result of checking if a position is solid.
pub type SolidCheck = dyn Fn(Vec2, Vec2) -> bool; // (position, half_extents) -> is_blocked

/// Serializable behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BehaviorConfig {
    Platform(platform::PlatformConfig),
    TopDown(topdown::TopDownConfig),
    Bullet(bullet::BulletConfig),
    Sine(sine::SineConfig),
    Fade(fade::FadeConfig),
    Wrap(wrap::WrapConfig),
    Solid,
}

use crate::{platform, topdown, bullet, sine, fade, wrap};
