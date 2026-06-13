//! High-level helpers built on [`Headless`]: a [`Harness`] that owns a
//! [`SpriteRenderer`] and can render arbitrary sprites or a whole
//! [`SceneData`] to an off-screen PNG — the engine equivalent of taking a
//! Playwright screenshot of a page.

use std::path::{Path, PathBuf};

use glam::Vec2;
use toile_core::color::Color;
use toile_graphics::camera::Camera2D;
use toile_graphics::sprite_renderer::{DrawSprite, RenderStats, SpriteRenderer, COLOR_WHITE, pack_color};
use toile_graphics::texture::TextureHandle;
use toile_scene::SceneData;

/// A reusable off-screen renderer: GPU target + sprite pipeline + a 1×1 white
/// texture for solid-colour quads.
pub struct Harness {
    pub gpu: crate::headless::Headless,
    pub sprite: SpriteRenderer,
    white: TextureHandle,
}

/// How to frame a scene snapshot.
#[derive(Clone, Debug)]
pub struct SnapshotOptions {
    /// Camera zoom. `None` auto-fits all entities into view.
    pub zoom: Option<f32>,
    /// Camera centre in world space. `None` uses the scene setting, or the
    /// centre of all entities when auto-fitting.
    pub camera: Option<Vec2>,
    /// Directory that relative `sprite_path`s are resolved against. When `None`,
    /// sprites are drawn as solid colour quads tinted by role.
    pub assets_root: Option<PathBuf>,
    /// Extra empty space around the content when auto-fitting (fraction, e.g. 0.1 = 10%).
    pub margin: f32,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self { zoom: None, camera: None, assets_root: None, margin: 0.1 }
    }
}

impl Harness {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let gpu = crate::headless::Headless::new(width, height)?;
        let mut sprite = SpriteRenderer::new(&gpu.device, gpu.format);
        let white = sprite.create_texture_from_rgba(&gpu.device, &gpu.queue, &[255, 255, 255, 255], 1, 1);
        Ok(Self { gpu, sprite, white })
    }

    /// A 1×1 opaque-white texture handle (tint it via `DrawSprite::color`).
    pub fn white(&self) -> TextureHandle {
        self.white
    }

    /// Load a PNG/JPEG/BMP from disk into a texture handle.
    pub fn load_texture(&mut self, path: &Path) -> TextureHandle {
        self.sprite.load_texture(&self.gpu.device, &self.gpu.queue, path)
    }

    /// Draw a sprite list into the off-screen target.
    pub fn render(&mut self, camera: &Camera2D, sprites: &[DrawSprite], clear: Color) -> RenderStats {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("toile-harness-frame"),
            });
        let stats = self.sprite.draw(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &self.gpu.view,
            camera,
            sprites,
            &clear,
        );
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        stats
    }

    /// Render a whole scene's entities to the off-screen target.
    pub fn render_scene(&mut self, scene: &SceneData, opts: &SnapshotOptions) -> RenderStats {
        let mut sprites: Vec<DrawSprite> = Vec::with_capacity(scene.entities.len());

        for e in &scene.entities {
            if !e.visible {
                continue;
            }
            let size = Vec2::new(e.width * e.scale_x, e.height * e.scale_y);
            let position = Vec2::new(e.x, e.y);

            let mut texture = self.white;
            let mut color = role_color(&e.name, &e.tags);
            let mut uv_min = Vec2::ZERO;
            let mut uv_max = Vec2::ONE;

            if !e.sprite_path.is_empty() {
                if let Some(path) = resolve_asset(opts.assets_root.as_deref(), &e.sprite_path) {
                    if path.exists() {
                        texture = self.load_texture(&path);
                        color = COLOR_WHITE;
                        if let Some((mn, mx)) = sheet_uv(e) {
                            uv_min = mn;
                            uv_max = mx;
                        }
                    }
                }
            }

            sprites.push(DrawSprite {
                texture,
                position,
                size,
                rotation: e.rotation,
                color,
                layer: e.layer,
                uv_min,
                uv_max,
            });
        }

        let camera = build_camera(self.gpu.width as f32, self.gpu.height as f32, scene, opts);
        let c = scene.settings.clear_color;
        let clear = Color::new(c[0] as f64, c[1] as f64, c[2] as f64, c[3] as f64);
        self.render(&camera, &sprites, clear)
    }

    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), String> {
        self.gpu.save_png(path)
    }

    pub fn pixels(&self) -> Result<Vec<u8>, String> {
        self.gpu.pixels()
    }
}

/// Build a camera that frames the scene per `opts`. Auto-fits to entity bounds
/// when zoom is unspecified.
fn build_camera(view_w: f32, view_h: f32, scene: &SceneData, opts: &SnapshotOptions) -> Camera2D {
    let mut camera = Camera2D::new(view_w, view_h);

    // Bounds of all entities (world space).
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for e in &scene.entities {
        if !e.visible {
            continue;
        }
        let half = Vec2::new(e.width * e.scale_x, e.height * e.scale_y) * 0.5;
        min = min.min(Vec2::new(e.x, e.y) - half);
        max = max.max(Vec2::new(e.x, e.y) + half);
    }
    let has_bounds = min.x.is_finite() && max.x.is_finite() && (max - min).length() > 0.0;

    let default_center = if has_bounds {
        (min + max) * 0.5
    } else {
        Vec2::new(scene.settings.camera_position[0], scene.settings.camera_position[1])
    };
    camera.position = opts.camera.unwrap_or(default_center);

    camera.zoom = match opts.zoom {
        Some(z) => z.max(0.001),
        None if has_bounds => {
            let extent = (max - min).max(Vec2::splat(1.0));
            let m = 1.0 + opts.margin.max(0.0);
            let zx = view_w / (extent.x * m);
            let zy = view_h / (extent.y * m);
            zx.min(zy).max(0.001)
        }
        None => scene.settings.camera_zoom.max(0.001),
    };
    camera
}

/// UV sub-rect for an entity's preview frame, when it has a sprite sheet.
fn sheet_uv(e: &toile_scene::EntityData) -> Option<(Vec2, Vec2)> {
    let sheet = e.sprite_sheet.as_ref()?;
    if sheet.columns == 0 || sheet.rows == 0 {
        return None;
    }
    let frame = e
        .preview_frame
        .or_else(|| e.animations.first().and_then(|a| a.frames.first().copied()))
        .unwrap_or(0);
    let col = frame % sheet.columns;
    let row = frame / sheet.columns;
    let uw = 1.0 / sheet.columns as f32;
    let uh = 1.0 / sheet.rows as f32;
    let min = Vec2::new(col as f32 * uw, row as f32 * uh);
    Some((min, min + Vec2::new(uw, uh)))
}

/// Resolve a (possibly relative) sprite path against an assets root.
fn resolve_asset(root: Option<&Path>, sprite_path: &str) -> Option<PathBuf> {
    let p = Path::new(sprite_path);
    if p.is_absolute() {
        return Some(p.to_path_buf());
    }
    match root {
        Some(root) => Some(root.join(p)),
        None => Some(p.to_path_buf()),
    }
}

/// A stable, readable tint for entities that have no usable sprite, based on
/// their role tag (or name as a fallback).
fn role_color(name: &str, tags: &[String]) -> u32 {
    let has = |k: &str| tags.iter().any(|t| t.eq_ignore_ascii_case(k));
    let name_l = name.to_ascii_lowercase();
    if has("player") || name_l.contains("player") || name_l.contains("hero") {
        pack_color(80, 160, 255, 255) // blue
    } else if has("coin") || name_l.contains("coin") {
        pack_color(255, 210, 60, 255) // gold
    } else if has("enemy") || name_l.contains("enemy") {
        pack_color(230, 70, 70, 255) // red
    } else if has("solid") || name_l.contains("platform") || name_l.contains("ground") || name_l.contains("wall") {
        pack_color(120, 120, 130, 255) // grey
    } else {
        pack_color(200, 200, 210, 255) // light grey
    }
}
