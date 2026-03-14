use glam::Vec2;
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
