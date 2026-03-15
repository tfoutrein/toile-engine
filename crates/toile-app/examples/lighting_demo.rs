/// 2D lighting demo — Toile Engine v0.4
///
/// Keyboard controls:
///   L          — toggle lighting on/off
///   A / Z      — increase / decrease ambient intensity
///   1–4        — toggle individual point lights
///   Mouse      — fifth light follows the cursor
///   F3         — debug title overlay
use std::path::Path;
use glam::Vec2;
use toile_app::*;

// ── Demo state ────────────────────────────────────────────────────────────────

struct LightingDemo {
    tex_white:  Option<TextureHandle>,
    tex_circle: Option<TextureHandle>,
    font:       Option<FontHandle>,

    lighting_on:    bool,
    ambient_level:  f32,
    lights_enabled: [bool; 4],
    time:           f32,
}

impl LightingDemo {
    fn new() -> Self {
        Self {
            tex_white:      None,
            tex_circle:     None,
            font:           None,
            lighting_on:    true,
            ambient_level:  0.06,
            lights_enabled: [true; 4],
            time:           0.0,
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

fn pack(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

// ── Game impl ─────────────────────────────────────────────────────────────────

impl Game for LightingDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        ctx.camera.zoom = 2.0;
        self.tex_white  = Some(ctx.create_texture_from_rgba(&[255, 255, 255, 255], 1, 1));
        self.tex_circle = Some(make_circle_tex(ctx, 64));
        self.font       = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 8.0));
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;
        self.time += dt;

        if !ctx.first_tick { return; }

        if ctx.input.is_key_just_pressed(Key::KeyL) {
            self.lighting_on = !self.lighting_on;
        }
        if ctx.input.is_key_just_pressed(Key::KeyA) {
            self.ambient_level = (self.ambient_level + 0.05).min(1.0);
        }
        if ctx.input.is_key_just_pressed(Key::KeyZ) {
            self.ambient_level = (self.ambient_level - 0.05).max(0.0);
        }
        if ctx.input.is_key_just_pressed(Key::Digit1) { self.lights_enabled[0] = !self.lights_enabled[0]; }
        if ctx.input.is_key_just_pressed(Key::Digit2) { self.lights_enabled[1] = !self.lights_enabled[1]; }
        if ctx.input.is_key_just_pressed(Key::Digit3) { self.lights_enabled[2] = !self.lights_enabled[2]; }
        if ctx.input.is_key_just_pressed(Key::Digit4) { self.lights_enabled[3] = !self.lights_enabled[3]; }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let (Some(tex_white), Some(tex_circle), Some(font)) =
            (self.tex_white, self.tex_circle, self.font) else { return };
        let t = self.time;

        // ── Scene geometry ─────────────────────────────────────────────────

        // Dark floor / background tiles (a 5×4 grid of dark rectangles)
        let tile_colors: &[(u8, u8, u8)] = &[
            (12, 14, 20), (10, 12, 18), (14, 16, 22), (10, 12, 18), (12, 14, 20),
            (10, 12, 18), (12, 14, 20), (10, 12, 18), (14, 16, 22), (10, 12, 18),
            (14, 16, 22), (10, 12, 18), (12, 14, 20), (10, 12, 18), (14, 16, 22),
            (10, 12, 18), (14, 16, 22), (10, 12, 18), (12, 14, 20), (10, 12, 18),
        ];
        let tile_w = 96.0_f32;
        let tile_h = 80.0_f32;
        for (idx, &(r, g, b)) in tile_colors.iter().enumerate() {
            let col = (idx % 5) as f32;
            let row = (idx / 5) as f32;
            let x = -240.0 + col * tile_w + tile_w / 2.0;
            let y =  160.0 - row * tile_h - tile_h / 2.0;
            ctx.draw_sprite(Sprite {
                texture: tex_white,
                position: Vec2::new(x, y),
                size: Vec2::new(tile_w - 2.0, tile_h - 2.0),
                rotation: 0.0,
                color: pack(r, g, b, 255),
                layer: -10,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // Coloured crates / objects scattered in the scene
        let objects: &[(f32, f32, f32, f32, u8, u8, u8)] = &[
            (-160.0,  80.0, 40.0, 40.0, 180,  60,  30),
            (  50.0,  60.0, 30.0, 50.0,  40, 120, 200),
            ( 140.0, -20.0, 50.0, 30.0,  50, 180,  60),
            ( -80.0, -60.0, 35.0, 35.0, 200, 160,  40),
            (  10.0, -90.0, 45.0, 25.0, 160,  40, 180),
            (-200.0, -30.0, 28.0, 55.0,  40, 160, 160),
            ( 100.0,  100.0, 35.0, 35.0, 220, 100,  60),
        ];
        for &(x, y, w, h, r, g, b) in objects {
            ctx.draw_sprite(Sprite {
                texture: tex_white,
                position: Vec2::new(x, y),
                size: Vec2::new(w, h),
                rotation: 0.0,
                color: pack(r, g, b, 255),
                layer: 0,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // Central glowing orb
        let pulse = 0.9 + 0.1 * (t * 2.5).sin();
        ctx.draw_sprite(Sprite {
            texture: tex_circle,
            position: Vec2::ZERO,
            size: Vec2::splat(22.0 * pulse),
            rotation: 0.0,
            color: pack(255, 230, 160, 255),
            layer: 1,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });

        // ── Lighting config ────────────────────────────────────────────────
        ctx.lighting.enabled = self.lighting_on;
        ctx.lighting.ambient = [
            self.ambient_level * 0.4,
            self.ambient_level * 0.5,
            self.ambient_level,
            1.0,
        ];

        // Four coloured orbiting lights
        let light_defs: &[(f32, f32, f32, f32, f32, f32, f32)] = &[
            // radius, speed, phase, r,   g,   b,   intensity
            (110.0, 0.7, 0.0,   1.0, 0.3, 0.1, 2.5),  // warm orange
            ( 90.0, 1.1, 1.6,   0.1, 0.5, 1.0, 2.0),  // cool blue
            ( 70.0, 1.6, 3.2,   0.2, 1.0, 0.3, 1.8),  // green
            (130.0, 0.5, 4.7,   1.0, 0.9, 0.2, 2.2),  // yellow
        ];

        for (i, &(radius, speed, phase, lr, lg, lb, intensity)) in
            light_defs.iter().enumerate()
        {
            if !self.lights_enabled[i] { continue; }
            let angle = t * speed + phase;
            let pos = Vec2::new(angle.cos() * radius, angle.sin() * radius * 0.6);

            // Draw a small circle where the light is
            ctx.draw_sprite(Sprite {
                texture: tex_circle,
                position: pos,
                size: Vec2::splat(10.0),
                rotation: 0.0,
                color: pack(
                    (lr * 255.0) as u8,
                    (lg * 255.0) as u8,
                    (lb * 255.0) as u8,
                    200,
                ),
                layer: 2,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });

            ctx.lighting.lights.push(Light {
                position: pos,
                radius: 160.0,
                falloff: 2.0,
                color: [lr, lg, lb],
                intensity,
            });
        }

        // Fifth light follows the mouse
        let mouse_screen = ctx.input.mouse_position();
        let mouse_world  = ctx.camera.screen_to_world(mouse_screen);
        ctx.draw_sprite(Sprite {
            texture: tex_circle,
            position: mouse_world,
            size: Vec2::splat(8.0),
            rotation: 0.0,
            color: pack(255, 255, 255, 180),
            layer: 2,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });
        ctx.lighting.lights.push(Light {
            position: mouse_world,
            radius: 120.0,
            falloff: 1.5,
            color: [1.0, 1.0, 0.9],
            intensity: 2.0,
        });

        // ── HUD ────────────────────────────────────────────────────────────
        let on  = pack(100, 255, 100, 255);
        let off = pack(120, 120, 120, 255);
        let lbl = pack(200, 200, 200, 255);

        let mut y = -155.0_f32;
        ctx.draw_text("[L] Lighting",   Vec2::new(-230.0, y), font, 8.0, lbl, 10);
        ctx.draw_text(
            if self.lighting_on { "ON" } else { "off" },
            Vec2::new(-60.0, y), font, 8.0,
            if self.lighting_on { on } else { off }, 10,
        );
        y += 18.0;

        let ambient_str = format!("[A/Z] Ambient {:.0}%", self.ambient_level * 100.0);
        ctx.draw_text(&ambient_str, Vec2::new(-230.0, y), font, 8.0, lbl, 10);
        y += 18.0;

        for (i, &enabled) in self.lights_enabled.iter().enumerate() {
            let label_names = ["[1] Orange", "[2] Blue  ", "[3] Green ", "[4] Yellow"];
            ctx.draw_text(label_names[i], Vec2::new(-230.0, y), font, 8.0, lbl, 10);
            ctx.draw_text(
                if enabled { "ON" } else { "off" },
                Vec2::new(-60.0, y), font, 8.0,
                if enabled { on } else { off }, 10,
            );
            y += 18.0;
        }

        ctx.draw_text("[Mouse] cursor light", Vec2::new(-230.0, y), font, 8.0, lbl, 10);
    }
}

fn main() {
    App::new()
        .with_title("Toile v0.4 — 2D Lighting  [L]ight [A/Z]ambient [1-4]toggle [Mouse]cursor")
        .with_size(1280, 720)
        .with_clear_color(toile_app::core::color::Color::new(0.02, 0.02, 0.04, 1.0))
        .run(LightingDemo::new());
}
