use glam::Vec2;

#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Aabb { half_extents: Vec2 },
    Circle { radius: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub shape: Shape,
    pub offset: Vec2,
}

impl Collider {
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

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn bounding_half_extents(&self) -> Vec2 {
        match self.shape {
            Shape::Aabb { half_extents } => half_extents,
            Shape::Circle { radius } => Vec2::splat(radius),
        }
    }
}
