use glam::Vec2;
use serde::{Deserialize, Serialize};
use crate::types::EntityState;

/// Wraps the entity around screen edges (Asteroids-style).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrapConfig {
    pub margin: f32,
}

impl Default for WrapConfig {
    fn default() -> Self {
        Self { margin: 0.0 }
    }
}

/// Wrap entity position to stay within bounds.
/// `view_half` is the half-size of the visible area from the camera center.
pub fn update(_config: &WrapConfig, entity: &mut EntityState, view_half: Vec2, camera_pos: Vec2) {
    let margin = entity.size.x.max(entity.size.y) * 0.5;
    let left = camera_pos.x - view_half.x - margin;
    let right = camera_pos.x + view_half.x + margin;
    let bottom = camera_pos.y - view_half.y - margin;
    let top = camera_pos.y + view_half.y + margin;
    let width = right - left;
    let height = top - bottom;

    if entity.position.x > right { entity.position.x -= width; }
    if entity.position.x < left { entity.position.x += width; }
    if entity.position.y > top { entity.position.y -= height; }
    if entity.position.y < bottom { entity.position.y += height; }
}
