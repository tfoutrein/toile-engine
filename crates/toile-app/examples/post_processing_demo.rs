/// Post-processing demo — Toile Engine v0.4
///
/// Keyboard controls:
///   V  — toggle Vignette
///   C  — toggle CRT (scanlines + barrel + chromatic aberration)
///   P  — toggle Pixelate
///   B  — toggle Bloom
///   G  — toggle Color Grading (desaturate + high contrast)
///   Space — trigger screen shake (trauma)
///   F3 — debug title overlay
use std::path::Path;
use glam::Vec2;
use toile_app::*;

// ── Demo state ────────────────────────────────────────────────────────────────

struct PostDemo {
    tex_white: Option<TextureHandle>,
    tex_circle: Option<TextureHandle>,
    font: Option<FontHandle>,

    // Effect toggles
    vignette: bool,
    crt: bool,
    pixelate: bool,
    bloom: bool,
    grading: bool,

    // Screen shake (trauma model)
    trauma: f32,
    shake_time: f32,

    // Scene animation
    time: f32,
}

impl PostDemo {
    fn new() -> Self {
        Self {
            tex_white: None,
            tex_circle: None,
            font: None,
            vignette: false,
            crt: false,
            pixelate: false,
            bloom: false,
            grading: false,
            trauma: 0.0,
            shake_time: 0.0,
            time: 0.0,
        }
    }
}

// ── Texture helpers ───────────────────────────────────────────────────────────

fn make_circle_tex(ctx: &mut GameContext, size: u32) -> TextureHandle {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;
    let rad = size as f32 / 2.0 - 1.0;
    for py in 0..size {
        for px in 0..size {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = ((rad - dist).clamp(0.0, 1.0) * 255.0) as u8;
            let i = ((py * size + px) * 4) as usize;
            data[i]     = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = alpha;
        }
    }
    ctx.create_texture_from_rgba(&data, size, size)
}

// ── Pseudo-noise for shake ────────────────────────────────────────────────────

fn noise1(t: f32) -> f32 {
    ((t * 127.1).sin() * 43758.545).fract() * 2.0 - 1.0
}

// ── Pack RGBA ─────────────────────────────────────────────────────────────────

fn pack(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

// ── Game impl ─────────────────────────────────────────────────────────────────

impl Game for PostDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        // macOS Retina: zoom 2.0 so content is not too small
        ctx.camera.zoom = 2.0;

        self.tex_white  = Some(ctx.create_texture_from_rgba(&[255, 255, 255, 255], 1, 1));
        self.tex_circle = Some(make_circle_tex(ctx, 64));
        self.font       = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 8.0));
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;
        self.time       += dt;
        self.shake_time += dt;

        if !ctx.first_tick { return; }

        // Toggle effects
        if ctx.input.is_key_just_pressed(Key::KeyV) { self.vignette = !self.vignette; }
        if ctx.input.is_key_just_pressed(Key::KeyC) { self.crt      = !self.crt;      }
        if ctx.input.is_key_just_pressed(Key::KeyP) { self.pixelate = !self.pixelate; }
        if ctx.input.is_key_just_pressed(Key::KeyB) { self.bloom    = !self.bloom;    }
        if ctx.input.is_key_just_pressed(Key::KeyG) { self.grading  = !self.grading;  }

        // Trigger shake
        if ctx.input.is_key_just_pressed(Key::Space) {
            self.trauma = 1.0;
        }

        // Decay trauma
        self.trauma = (self.trauma - 3.0 * dt).max(0.0);
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let (Some(tex_white), Some(tex_circle), Some(font)) =
            (self.tex_white, self.tex_circle, self.font) else { return };
        let t = self.time;

        // ── Rebuild post-processing stack ─────────────────────────────────
        ctx.post_processing.enabled = true;
        ctx.post_processing.effects.clear();

        if self.bloom {
            ctx.post_processing.effects.push(PostEffect::Bloom {
                threshold: 0.65,
                intensity: 3.5,
                radius: 0.006,
            });
        }
        if self.vignette {
            ctx.post_processing.effects.push(PostEffect::Vignette {
                intensity: 1.4,
                smoothness: 0.5,
            });
        }
        if self.crt {
            ctx.post_processing.effects.push(PostEffect::Crt {
                scanline_intensity: 0.25,
                curvature: 0.1,
                chromatic_aberration: 0.6,
            });
        }
        if self.pixelate {
            ctx.post_processing.effects.push(PostEffect::Pixelate { pixel_size: 6.0 });
        }
        if self.grading {
            // B&W + slight contrast boost — clearly visible, safe in linear space
            ctx.post_processing.effects.push(PostEffect::ColorGrading {
                saturation: 0.0,   // full black & white
                brightness: 1.3,   // compensate for B&W looking darker
                contrast: 1.2,     // mild contrast boost around 0.18 pivot
            });
        }
        if self.trauma > 0.001 {
            let t2 = self.trauma * self.trauma;
            let sx = t2 * 0.018 * noise1(self.shake_time * 7.3);
            let sy = t2 * 0.018 * noise1(self.shake_time * 13.7);
            ctx.post_processing.effects.push(PostEffect::ScreenShake {
                offset_x: sx,
                offset_y: sy,
            });
        }

        // ── Background strips ─────────────────────────────────────────────
        let colors: &[(u8, u8, u8)] = &[
            (20, 10, 40),
            (10, 20, 50),
            (5,  30, 20),
            (40, 10, 10),
        ];
        for (i, &(r, g, b)) in colors.iter().enumerate() {
            let x = -240.0 + i as f32 * 120.0;
            ctx.draw_sprite(Sprite {
                texture: tex_white,
                position: Vec2::new(x, 0.0),
                size: Vec2::new(120.0, 360.0),
                rotation: 0.0,
                color: pack(r, g, b, 255),
                layer: -10,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // ── Orbiting coloured circles ──────────────────────────────────────
        let orbit_data: &[(f32, f32, f32, u8, u8, u8, f32)] = &[
            (120.0, 0.8, 0.0,  255, 100,  50, 30.0),
            (100.0, 1.2, 1.2,   50, 200, 255, 24.0),
            ( 80.0, 1.5, 2.4,  100, 255, 100, 20.0),
            (140.0, 0.6, 3.7,  255, 255,  50, 28.0),
            ( 60.0, 2.0, 0.8,  200,  50, 255, 18.0),
            (110.0, 0.9, 4.5,  255,  80, 180, 22.0),
        ];
        for &(radius, speed, phase, r, g, b, size) in orbit_data {
            let angle = t * speed + phase;
            let pos = Vec2::new(angle.cos() * radius, angle.sin() * radius * 0.5);
            ctx.draw_sprite(Sprite {
                texture: tex_circle,
                position: pos,
                size: Vec2::splat(size),
                rotation: 0.0,
                color: pack(r, g, b, 220),
                layer: 0,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // ── Bright centre star (drives bloom) ─────────────────────────────
        let pulse = 0.85 + 0.15 * (t * 3.0).sin();
        ctx.draw_sprite(Sprite {
            texture: tex_circle,
            position: Vec2::ZERO,
            size: Vec2::splat(40.0 * pulse),
            rotation: 0.0,
            color: pack(255, 255, 255, 255),
            layer: 1,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });
        ctx.draw_sprite(Sprite {
            texture: tex_circle,
            position: Vec2::ZERO,
            size: Vec2::splat(70.0 * pulse),
            rotation: 0.0,
            color: pack(255, 240, 180, 80),
            layer: 0,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });

        // ── Rotating bright squares ────────────────────────────────────────
        for i in 0..4 {
            let a = t * 0.5 + i as f32 * std::f32::consts::TAU / 4.0;
            let pos = Vec2::new(a.cos() * 55.0, a.sin() * 55.0);
            ctx.draw_sprite(Sprite {
                texture: tex_white,
                position: pos,
                size: Vec2::splat(10.0),
                rotation: t + i as f32,
                color: pack(255, 255, 200, 255),
                layer: 1,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // ── HUD / Controls ─────────────────────────────────────────────────
        let v = self.vignette;
        let c = self.crt;
        let p = self.pixelate;
        let b = self.bloom;
        let g = self.grading;

        let on  = pack(100, 255, 100, 255);
        let off = pack(120, 120, 120, 255);
        let label_color = pack(200, 200, 200, 255);

        let mut y = -155.0_f32;
        let hud: &[(&str, bool)] = &[
            ("[V] Vignette",    v),
            ("[C] CRT",         c),
            ("[P] Pixelate",    p),
            ("[B] Bloom",       b),
            ("[G] B&W Grade",   g),
        ];
        for &(name, active) in hud {
            ctx.draw_text(name, Vec2::new(-230.0, y), font, 8.0, label_color, 10);
            ctx.draw_text(
                if active { "ON" } else { "off" },
                Vec2::new(-60.0, y), font, 8.0,
                if active { on } else { off },
                10,
            );
            y += 18.0;
        }

        // Shake line
        let shake_label = format!("[Space] Shake  {:.0}%", self.trauma * 100.0);
        let shake_color = if self.trauma > 0.01 { pack(255, 200, 80, 255) } else { label_color };
        ctx.draw_text(&shake_label, Vec2::new(-230.0, y), font, 8.0, shake_color, 10);
    }
}

fn main() {
    App::new()
        .with_title("Toile v0.4 — Post-Processing  [V]ignette [C]RT [P]ixelate [B]loom [G]rading [Space]shake")
        .with_size(1280, 720)
        .with_clear_color(toile_app::core::color::Color::new(0.04, 0.02, 0.08, 1.0))
        .run(PostDemo::new());
}
