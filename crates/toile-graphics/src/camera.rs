use glam::{Mat4, Vec2};

/// 2D orthographic camera.
///
/// Position is the center of the view in world space.
/// Zoom 1.0 means 1 world unit = 1 pixel.
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
    viewport_size: Vec2,
}

impl Camera2D {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            viewport_size: Vec2::new(viewport_width, viewport_height),
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.viewport_size = Vec2::new(width, height);
    }

    pub fn view_projection(&self) -> Mat4 {
        let half_w = self.viewport_size.x / (2.0 * self.zoom);
        let half_h = self.viewport_size.y / (2.0 * self.zoom);

        Mat4::orthographic_rh(
            self.position.x - half_w,
            self.position.x + half_w,
            self.position.y - half_h,
            self.position.y + half_h,
            -1.0,
            1.0,
        )
    }

    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        let half_w = self.viewport_size.x / (2.0 * self.zoom);
        let half_h = self.viewport_size.y / (2.0 * self.zoom);

        Vec2::new(
            self.position.x + (screen_pos.x / self.viewport_size.x - 0.5) * 2.0 * half_w,
            self.position.y - (screen_pos.y / self.viewport_size.y - 0.5) * 2.0 * half_h,
        )
    }

    pub fn viewport_size(&self) -> Vec2 {
        self.viewport_size
    }

    /// Half-size of the visible area in world units.
    pub fn half_viewport(&self) -> Vec2 {
        self.viewport_size / (2.0 * self.zoom)
    }

    /// Top-left corner of the visible area in world coordinates.
    pub fn top_left(&self) -> Vec2 {
        let half = self.half_viewport();
        Vec2::new(self.position.x - half.x, self.position.y + half.y)
    }

    /// Bottom-right corner of the visible area in world coordinates.
    pub fn bottom_right(&self) -> Vec2 {
        let half = self.half_viewport();
        Vec2::new(self.position.x + half.x, self.position.y - half.y)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn from_camera(camera: &Camera2D) -> Self {
        Self {
            view_proj: camera.view_projection().to_cols_array_2d(),
        }
    }
}
