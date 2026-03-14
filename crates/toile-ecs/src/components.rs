use std::path::PathBuf;

use glam::Vec2;
use toile_assets::animation::SpriteSheetHandle;
use toile_collision::Shape;
use toile_graphics::texture::TextureHandle;

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

impl Transform {
    pub fn at(position: Vec2) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpriteComponent {
    pub texture: TextureHandle,
    pub size: Vec2,
    pub color: u32,
    pub layer: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct ColliderComponent {
    pub shape: Shape,
    pub offset: Vec2,
}

impl ColliderComponent {
    pub fn aabb(half_w: f32, half_h: f32) -> Self {
        Self {
            shape: Shape::Aabb {
                half_extents: Vec2::new(half_w, half_h),
            },
            offset: Vec2::ZERO,
        }
    }

    pub fn circle(radius: f32) -> Self {
        Self {
            shape: Shape::Circle { radius },
            offset: Vec2::ZERO,
        }
    }
}

/// Animator: tracks current animation state for an entity.
pub struct AnimatorComponent {
    pub sheet: SpriteSheetHandle,
    pub current_clip: String,
    pub current_frame: usize,
    pub elapsed: f32,
    pub playing: bool,
}

impl AnimatorComponent {
    pub fn new(sheet: SpriteSheetHandle, clip: &str) -> Self {
        Self {
            sheet,
            current_clip: clip.to_string(),
            current_frame: 0,
            elapsed: 0.0,
            playing: true,
        }
    }
}

/// Script: references a Lua script file for entity behavior.
pub struct ScriptComponent {
    pub script_path: PathBuf,
    pub initialized: bool,
}

impl ScriptComponent {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            script_path: path.into(),
            initialized: false,
        }
    }
}
