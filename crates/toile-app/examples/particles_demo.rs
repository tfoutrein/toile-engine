//! Toile Engine — Particles Demo (v0.2)
//!
//! Showcases the CPU particle system with presets.
//!
//! Keys:
//!   1-6: Switch preset (Fire, Smoke, Sparks, Rain, Snow, Dust)
//!   Space: Trigger explosion burst
//!   Mouse: Move emitter to cursor position
//!   F3: Debug overlay
//!
//! Run with: `cargo run --example particles_demo`

use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, Sprite, TextureHandle, COLOR_WHITE};
use toile_core::color::Color;
use toile_core::particles::{self, presets, ParticleEmitter, ParticlePool};
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};

struct ParticlesDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    pool: Option<ParticlePool>,
    explosion_pool: Option<ParticlePool>,
    preset_name: String,
    emitter_pos: Vec2,
}

impl ParticlesDemo {
    fn switch_preset(&mut self, name: &str, emitter: ParticleEmitter) {
        self.preset_name = name.to_string();
        self.pool = Some(ParticlePool::new(emitter, self.emitter_pos));
    }
}

impl Game for ParticlesDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));

        // Start with fire
        self.switch_preset("Fire", presets::fire());

        // Explosion pool (burst-only, inactive)
        let mut exp = ParticlePool::new(presets::explosion(), Vec2::ZERO);
        exp.active = false;
        self.explosion_pool = Some(exp);

        log::info!("Particles Demo! Keys 1-6 = presets, Space = explosion, Mouse = move");
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;

        // Move emitter with mouse
        let mouse_screen = ctx.input.mouse_position();
        self.emitter_pos = ctx.camera.screen_to_world(mouse_screen);

        if let Some(pool) = &mut self.pool {
            pool.position = self.emitter_pos;
            pool.update(dt);
        }

        // Explosion pool update
        if let Some(exp) = &mut self.explosion_pool {
            exp.update(dt);
        }

        // Switch presets
        if ctx.input.is_key_just_pressed(Key::Digit1) {
            self.switch_preset("Fire", presets::fire());
        }
        if ctx.input.is_key_just_pressed(Key::Digit2) {
            self.switch_preset("Smoke", presets::smoke());
        }
        if ctx.input.is_key_just_pressed(Key::Digit3) {
            self.switch_preset("Sparks", presets::sparks());
        }
        if ctx.input.is_key_just_pressed(Key::Digit4) {
            self.switch_preset("Rain", presets::rain());
        }
        if ctx.input.is_key_just_pressed(Key::Digit5) {
            self.switch_preset("Snow", presets::snow());
        }
        if ctx.input.is_key_just_pressed(Key::Digit6) {
            self.switch_preset("Dust", presets::dust());
        }

        // Explosion burst
        if ctx.input.is_key_just_pressed(Key::Space) {
            if let Some(exp) = &mut self.explosion_pool {
                exp.position = self.emitter_pos;
                exp.burst(80);
            }
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        // Draw continuous particles
        if let Some(pool) = &self.pool {
            for (pos, size, rot, color) in pool.render_data() {
                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: pos,
                    size: Vec2::splat(size),
                    rotation: rot,
                    color,
                    layer: 0,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }

        // Draw explosion particles
        if let Some(exp) = &self.explosion_pool {
            for (pos, size, rot, color) in exp.render_data() {
                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: pos,
                    size: Vec2::splat(size),
                    rotation: rot,
                    color,
                    layer: 1,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }

        // HUD
        if let Some(font) = self.font {
            let count = self.pool.as_ref().map(|p| p.particle_count()).unwrap_or(0)
                + self.explosion_pool.as_ref().map(|p| p.particle_count()).unwrap_or(0);

            ctx.draw_text(
                &format!("Preset: {}  |  Particles: {}", self.preset_name, count),
                Vec2::new(-310.0, 170.0),
                font,
                7.0,
                COLOR_WHITE,
                10,
            );
            ctx.draw_text(
                "1:Fire 2:Smoke 3:Sparks 4:Rain 5:Snow 6:Dust Space:Boom",
                Vec2::new(-310.0, 155.0),
                font,
                5.0,
                pack_color(180, 180, 180, 255),
                10,
            );
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Particles Demo (v0.2)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.08, 0.08, 0.12))
        .run(ParticlesDemo {
            white_tex: None,
            font: None,
            pool: None,
            explosion_pool: None,
            preset_name: String::new(),
            emitter_pos: Vec2::ZERO,
        });
}
