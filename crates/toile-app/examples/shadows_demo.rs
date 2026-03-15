/// 2D shadow demo — Toile Engine v0.4
///
/// Demonstrates 1D shadow maps with PCF soft shadows.
///
/// Shadow occlusion convention:
///   Background is cleared with alpha = 0 (transparent), so it does NOT
///   block rays. Sprite pixels have alpha = 255, so they cast shadows.
///
/// Keyboard controls:
///   S          — toggle shadows on/off
///   L          — toggle lighting on/off
///   A / Z      — increase / decrease ambient intensity
///   1–3        — toggle individual point lights
///   Mouse      — cursor light (casts shadows)
use std::path::Path;
use glam::Vec2;
use toile_app::*;

struct ShadowsDemo {
    tex_white:  Option<TextureHandle>,
    tex_circle: Option<TextureHandle>,
    font:       Option<FontHandle>,

    lighting_on:    bool,
    shadows_on:     bool,
    ambient_level:  f32,
    lights_enabled: [bool; 3],
    time:           f32,
}

impl ShadowsDemo {
    fn new() -> Self {
        Self {
            tex_white:      None,
            tex_circle:     None,
            font:           None,
            lighting_on:    true,
            shadows_on:     true,
            ambient_level:  0.04,
            lights_enabled: [true; 3],
            time:           0.0,
        }
    }
}

fn make_circle_tex(ctx: &mut GameContext, size: u32) -> TextureHandle {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let cx  = size as f32 / 2.0;
    let cy  = size as f32 / 2.0;
    let rad = size as f32 / 2.0 - 1.0;
    for py in 0..size {
        for px in 0..size {
            let dx   = px as f32 - cx;
            let dy   = py as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let a    = ((rad - dist).clamp(0.0, 1.0) * 255.0) as u8;
            let i    = ((py * size + px) * 4) as usize;
            data[i] = 255; data[i+1] = 255; data[i+2] = 255; data[i+3] = a;
        }
    }
    ctx.create_texture_from_rgba(&data, size, size)
}

fn pack(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

impl Game for ShadowsDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        ctx.camera.zoom  = 2.0;
        self.tex_white   = Some(ctx.create_texture_from_rgba(&[255, 255, 255, 255], 1, 1));
        self.tex_circle  = Some(make_circle_tex(ctx, 64));
        self.font        = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 8.0));
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;
        self.time += dt;

        if !ctx.first_tick { return; }

        if ctx.input.is_key_just_pressed(Key::KeyS) { self.shadows_on  = !self.shadows_on; }
        if ctx.input.is_key_just_pressed(Key::KeyL) { self.lighting_on = !self.lighting_on; }
        if ctx.input.is_key_just_pressed(Key::KeyA) { self.ambient_level = (self.ambient_level + 0.02).min(1.0); }
        if ctx.input.is_key_just_pressed(Key::KeyZ) { self.ambient_level = (self.ambient_level - 0.02).max(0.0); }
        if ctx.input.is_key_just_pressed(Key::Digit1) { self.lights_enabled[0] = !self.lights_enabled[0]; }
        if ctx.input.is_key_just_pressed(Key::Digit2) { self.lights_enabled[1] = !self.lights_enabled[1]; }
        if ctx.input.is_key_just_pressed(Key::Digit3) { self.lights_enabled[2] = !self.lights_enabled[2]; }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let (Some(tex_white), Some(tex_circle), Some(font)) =
            (self.tex_white, self.tex_circle, self.font) else { return };
        let t = self.time;

        // ── Scene: occluder objects (opaque walls / pillars) ──────────────

        // A thick surrounding wall border
        let wall_color = pack(80, 75, 70, 255);
        // Top wall
        ctx.draw_sprite(Sprite { texture: tex_white, position: Vec2::new(0.0, 150.0),
            size: Vec2::new(480.0, 20.0), rotation: 0.0, color: wall_color, layer: 0,
            uv_min: Vec2::ZERO, uv_max: Vec2::ONE });
        // Bottom wall
        ctx.draw_sprite(Sprite { texture: tex_white, position: Vec2::new(0.0, -150.0),
            size: Vec2::new(480.0, 20.0), rotation: 0.0, color: wall_color, layer: 0,
            uv_min: Vec2::ZERO, uv_max: Vec2::ONE });
        // Left wall
        ctx.draw_sprite(Sprite { texture: tex_white, position: Vec2::new(-230.0, 0.0),
            size: Vec2::new(20.0, 280.0), rotation: 0.0, color: wall_color, layer: 0,
            uv_min: Vec2::ZERO, uv_max: Vec2::ONE });
        // Right wall
        ctx.draw_sprite(Sprite { texture: tex_white, position: Vec2::new(230.0, 0.0),
            size: Vec2::new(20.0, 280.0), rotation: 0.0, color: wall_color, layer: 0,
            uv_min: Vec2::ZERO, uv_max: Vec2::ONE });

        // Interior pillars / crates (these are the occluders that cast visible shadows)
        let occluders: &[(f32, f32, f32, f32, u8, u8, u8)] = &[
            // x,     y,    w,    h,    r,   g,   b
            (-120.0,  80.0, 35.0, 35.0, 160,  80,  40),   // warm brown crate
            (  80.0,  60.0, 28.0, 55.0,  50, 100, 160),   // blue pillar
            ( 140.0, -40.0, 45.0, 28.0,  50, 150,  60),   // green crate
            ( -60.0, -80.0, 32.0, 32.0, 180, 140,  40),   // yellow crate
            (  20.0,  20.0, 55.0, 22.0, 140,  50, 140),   // purple block
            (-160.0, -30.0, 22.0, 60.0,  40, 140, 140),   // teal pillar
            ( 100.0, -90.0, 38.0, 38.0, 200,  80,  60),   // orange crate
            (  -5.0, 100.0, 50.0, 20.0,  90,  90, 200),   // pale blue shelf
        ];
        for &(x, y, w, h, r, g, b) in occluders {
            ctx.draw_sprite(Sprite {
                texture: tex_white,
                position: Vec2::new(x, y),
                size: Vec2::new(w, h),
                rotation: 0.0,
                color: pack(r, g, b, 255),
                layer: 1,
                uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // ── Lighting configuration ────────────────────────────────────────
        ctx.lighting.enabled     = self.lighting_on;
        ctx.lighting.shadow.enabled = self.shadows_on;
        ctx.lighting.ambient = [
            self.ambient_level * 0.3,
            self.ambient_level * 0.4,
            self.ambient_level,
            1.0,
        ];

        // Three coloured orbiting lights (all cast shadows)
        let light_defs: &[(f32, f32, f32, f32, f32, f32, f32)] = &[
            // orbit_r, speed, phase, r,   g,   b,   intensity
            (100.0, 0.6,  0.0,  1.0, 0.4, 0.1, 2.8),  // orange
            ( 80.0, 1.0,  2.1,  0.2, 0.5, 1.0, 2.4),  // blue
            (120.0, 0.8,  4.2,  0.3, 1.0, 0.3, 2.2),  // green
        ];

        for (i, &(orbit_r, speed, phase, lr, lg, lb, intensity)) in light_defs.iter().enumerate() {
            if !self.lights_enabled[i] { continue; }
            let angle = t * speed + phase;
            let pos   = Vec2::new(angle.cos() * orbit_r, angle.sin() * orbit_r * 0.6);

            // Draw a glow dot at the light position
            ctx.draw_sprite(Sprite {
                texture: tex_circle,
                position: pos,
                size: Vec2::splat(12.0),
                rotation: 0.0,
                color: pack(
                    (lr * 255.0) as u8,
                    (lg * 255.0) as u8,
                    (lb * 255.0) as u8,
                    210,
                ),
                layer: 5,
                uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });

            ctx.lighting.lights.push(Light {
                position:    pos,
                radius:      180.0,
                falloff:     2.0,
                color:       [lr, lg, lb],
                intensity,
                cast_shadow: true,
            });
        }

        // Mouse cursor light (casts shadows too)
        let mouse_world = ctx.camera.screen_to_world(ctx.input.mouse_position());
        ctx.draw_sprite(Sprite {
            texture: tex_circle,
            position: mouse_world,
            size: Vec2::splat(8.0),
            rotation: 0.0,
            color: pack(255, 255, 200, 180),
            layer: 5,
            uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
        });
        ctx.lighting.lights.push(Light {
            position:    mouse_world,
            radius:      140.0,
            falloff:     1.8,
            color:       [1.0, 1.0, 0.85],
            intensity:   2.5,
            cast_shadow: true,
        });

        // ── HUD ──────────────────────────────────────────────────────────
        let on  = pack(100, 255, 100, 255);
        let off = pack(120, 120, 120, 255);
        let lbl = pack(200, 200, 200, 255);

        let mut y = -145.0_f32;
        ctx.draw_text("[L] Lighting", Vec2::new(-220.0, y), font, 8.0, lbl, 10);
        ctx.draw_text(if self.lighting_on { "ON" } else { "off" }, Vec2::new(-60.0, y), font, 8.0,
            if self.lighting_on { on } else { off }, 10);
        y += 18.0;

        ctx.draw_text("[S] Shadows", Vec2::new(-220.0, y), font, 8.0, lbl, 10);
        ctx.draw_text(if self.shadows_on { "ON" } else { "off" }, Vec2::new(-60.0, y), font, 8.0,
            if self.shadows_on { on } else { off }, 10);
        y += 18.0;

        let ambient_str = format!("[A/Z] Ambient {:.0}%", self.ambient_level * 100.0);
        ctx.draw_text(&ambient_str, Vec2::new(-220.0, y), font, 8.0, lbl, 10);
        y += 18.0;

        let light_names = ["[1] Orange", "[2] Blue  ", "[3] Green "];
        for (i, &enabled) in self.lights_enabled.iter().enumerate() {
            ctx.draw_text(light_names[i], Vec2::new(-220.0, y), font, 8.0, lbl, 10);
            ctx.draw_text(if enabled { "ON" } else { "off" }, Vec2::new(-60.0, y), font, 8.0,
                if enabled { on } else { off }, 10);
            y += 18.0;
        }
        ctx.draw_text("[Mouse] cursor light", Vec2::new(-220.0, y), font, 8.0, lbl, 10);
    }
}

fn main() {
    App::new()
        .with_title("Toile v0.4 — 2D Shadows  [L]ight [S]hadow [A/Z]ambient [1-3]toggle [Mouse]cursor")
        .with_size(1280, 720)
        // IMPORTANT: alpha = 0.0 so background pixels don't block shadow rays
        .with_clear_color(toile_app::core::color::Color::new(0.04, 0.04, 0.06, 0.0))
        .run(ShadowsDemo::new());
}
