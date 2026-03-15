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
use glam::Vec2;
use toile_app::*;

// ── Demo state ────────────────────────────────────────────────────────────────

struct PostDemo {
    tex_white: Option<TextureHandle>,
    tex_circle: Option<TextureHandle>,

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

fn make_circle_tex(ctx: &mut GameContext, size: u32, r: u8, g: u8, b: u8) -> TextureHandle {
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
            data[i]     = r;
            data[i + 1] = g;
            data[i + 2] = b;
            data[i + 3] = alpha;
        }
    }
    ctx.create_texture_from_rgba(&data, size, size)
}

fn make_white_tex(ctx: &mut GameContext) -> TextureHandle {
    ctx.create_texture_from_rgba(&[255, 255, 255, 255], 1, 1)
}

// ── Pseudo-noise for shake ────────────────────────────────────────────────────

fn noise1(t: f32) -> f32 {
    ((t * 127.1).sin() * 43758.545).fract() * 2.0 - 1.0
}

// ── Game impl ─────────────────────────────────────────────────────────────────

impl Game for PostDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        // macOS Retina: zoom 2.0 so content is not too small
        ctx.camera.zoom = 2.0;

        self.tex_white  = Some(make_white_tex(ctx));
        self.tex_circle = Some(make_circle_tex(ctx, 64, 255, 255, 255));
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
        let (Some(tex_white), Some(tex_circle)) = (self.tex_white, self.tex_circle) else { return };
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
            ctx.post_processing.effects.push(PostEffect::ColorGrading {
                saturation: 0.3,
                brightness: 1.1,
                contrast: 1.4,
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
            (20, 10, 40),   // deep purple
            (10, 20, 50),   // deep blue
            (5, 30, 20),    // deep green
            (40, 10, 10),   // deep red
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
            // radius, speed, phase, R, G, B, size
            (120.0, 0.8, 0.0,   255, 100,  50,  30.0),
            (100.0, 1.2, 1.2,    50, 200, 255,  24.0),
            ( 80.0, 1.5, 2.4,   100, 255, 100,  20.0),
            (140.0, 0.6, 3.7,   255, 255,  50,  28.0),
            ( 60.0, 2.0, 0.8,   200,  50, 255,  18.0),
            (110.0, 0.9, 4.5,   255,  80, 180,  22.0),
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
        // Core
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
        // Glow ring (slightly larger, semi-transparent)
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

        // ── Rotating bright squares (more bloom sources) ───────────────────
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

        let mut y = -155.0_f32;
        let lines = [
            format!("[V] Vignette    {}", if v { "ON " } else { "off" }),
            format!("[C] CRT         {}", if c { "ON " } else { "off" }),
            format!("[P] Pixelate    {}", if p { "ON " } else { "off" }),
            format!("[B] Bloom       {}", if b { "ON " } else { "off" }),
            format!("[G] Color Grade {}", if g { "ON " } else { "off" }),
            format!("[Space] Shake   trauma={:.2}", self.trauma),
        ];
        for line in &lines {
            draw_label(ctx, tex_white, line, Vec2::new(-230.0, y));
            y += 18.0;
        }
    }
}

// ── Draw a text label using small pixel squares ───────────────────────────────
// (We don't load a font to keep the demo self-contained — use sprite squares as chars.)
fn draw_label(ctx: &mut GameContext, tex: TextureHandle, text: &str, pos: Vec2) {
    // Minimal 5×7 digit/letter renderer using tiny squares
    let char_w = 5.0;
    let mut x = pos.x;
    for ch in text.chars() {
        let color = match ch {
            'O' | 'N' => pack(100, 255, 100, 255),
            'o' | 'f' => pack(160, 160, 160, 255),
            '0'..='9' | '.' => pack(255, 220, 100, 255),
            '[' | ']' => pack(180, 180, 255, 255),
            _ => pack(220, 220, 220, 255),
        };
        if ch != ' ' {
            ctx.draw_sprite(Sprite {
                texture: tex,
                position: Vec2::new(x + 2.0, pos.y),
                size: Vec2::new(char_w - 1.0, 7.0),
                rotation: 0.0,
                color,
                layer: 10,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }
        x += char_w;
    }
}

fn pack(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

fn main() {
    App::new()
        .with_title("Toile v0.4 — Post-Processing Demo  [V]ignette [C]RT [P]ixelate [B]loom [G]rading [Space]shake")
        .with_size(1280, 720)
        .with_clear_color(toile_app::core::color::Color::new(0.04, 0.02, 0.08, 1.0))
        .run(PostDemo::new());
}
