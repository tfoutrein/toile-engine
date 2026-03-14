//! Toile Engine — Scene Stack Demo (v0.2)
//!
//! Demonstrates scene push/pop/replace with transitions.
//!
//! Menu Screen → Press Enter → Gameplay → Press Escape → Pause Overlay → Enter to resume
//!
//! Run with: `cargo run --example scene_demo`

use std::path::Path;

use glam::Vec2;
use toile_app::scene::{Scene, SceneStack};
use toile_app::{App, Game, GameContext, Key, Sprite, TextureHandle, COLOR_WHITE};
use toile_core::color::Color;
use toile_core::particles::{presets, ParticlePool};
use toile_core::scene_stack::Transition;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};

// ---- Shared state ----

struct SharedAssets {
    white_tex: TextureHandle,
    font: toile_app::FontHandle,
}

// ---- Menu Scene ----

struct MenuScene {
    assets: Option<SharedAssets>,
    blink_timer: f32,
}

impl Scene for MenuScene {
    fn on_enter(&mut self, ctx: &mut GameContext) {
        if self.assets.is_none() {
            let white = ctx.load_texture(Path::new("assets/white.png"));
            let font = ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0);
            self.assets = Some(SharedAssets {
                white_tex: white,
                font,
            });
        }
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        self.blink_timer += dt as f32;
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let Some(assets) = &self.assets else { return };

        // Title
        ctx.draw_text("TOILE ENGINE", Vec2::new(-120.0, 60.0), assets.font, 14.0, COLOR_WHITE, 10);

        // Blinking "Press Enter"
        if (self.blink_timer * 2.0) as i32 % 2 == 0 {
            ctx.draw_text(
                "Press Enter to Play",
                Vec2::new(-110.0, -20.0),
                assets.font,
                7.0,
                pack_color(200, 200, 100, 255),
                10,
            );
        }

        ctx.draw_text(
            "Scene Stack Demo",
            Vec2::new(-80.0, -80.0),
            assets.font,
            5.0,
            pack_color(120, 120, 150, 255),
            10,
        );
    }
}

// ---- Gameplay Scene ----

struct GameplayScene {
    assets: SharedAssets,
    player_pos: Vec2,
    particles: ParticlePool,
    score: u32,
}

impl GameplayScene {
    fn new(assets: SharedAssets) -> Self {
        let particles = ParticlePool::new(presets::dust(), Vec2::ZERO);
        Self {
            assets,
            player_pos: Vec2::ZERO,
            particles,
            score: 0,
        }
    }
}

impl Scene for GameplayScene {
    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let speed = 150.0 * dt as f32;
        if ctx.input.is_key_down(Key::ArrowRight) || ctx.input.is_key_down(Key::KeyD) {
            self.player_pos.x += speed;
        }
        if ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA) {
            self.player_pos.x -= speed;
        }
        if ctx.input.is_key_down(Key::ArrowUp) || ctx.input.is_key_down(Key::KeyW) {
            self.player_pos.y += speed;
        }
        if ctx.input.is_key_down(Key::ArrowDown) || ctx.input.is_key_down(Key::KeyS) {
            self.player_pos.y -= speed;
        }

        self.score += 1;
        self.particles.position = self.player_pos + Vec2::new(0.0, -15.0);
        self.particles.update(dt as f32);
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        // Ground line
        ctx.draw_sprite(DrawSprite {
            texture: self.assets.white_tex,
            position: Vec2::new(0.0, -100.0),
            size: Vec2::new(600.0, 4.0),
            rotation: 0.0,
            color: pack_color(80, 120, 80, 255),
            layer: 0,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });

        // Player
        ctx.draw_sprite(DrawSprite {
            texture: self.assets.white_tex,
            position: self.player_pos,
            size: Vec2::new(24.0, 32.0),
            rotation: 0.0,
            color: pack_color(80, 140, 220, 255),
            layer: 5,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });

        // Particles
        for (pos, size, rot, color) in self.particles.render_data() {
            ctx.draw_sprite(DrawSprite {
                texture: self.assets.white_tex,
                position: pos,
                size: Vec2::splat(size),
                rotation: rot,
                color,
                layer: 3,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // HUD
        ctx.draw_text(
            &format!("Score: {} | Escape = Pause", self.score / 60),
            Vec2::new(-280.0, 160.0),
            self.assets.font,
            6.0,
            COLOR_WHITE,
            10,
        );
    }
}

// ---- Pause Overlay Scene ----

struct PauseScene {
    font: toile_app::FontHandle,
    white_tex: TextureHandle,
}

impl Scene for PauseScene {
    fn update(&mut self, _ctx: &mut GameContext, _dt: f64) {}

    fn draw(&mut self, ctx: &mut GameContext) {
        // Semi-transparent dark overlay
        ctx.draw_sprite(DrawSprite {
            texture: self.white_tex,
            position: Vec2::ZERO,
            size: Vec2::new(800.0, 600.0),
            rotation: 0.0,
            color: pack_color(0, 0, 0, 150),
            layer: 50,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });

        ctx.draw_text("PAUSED", Vec2::new(-55.0, 20.0), self.font, 14.0, COLOR_WHITE, 60);
        ctx.draw_text(
            "Enter = Resume  |  Q = Quit to Menu",
            Vec2::new(-170.0, -30.0),
            self.font,
            5.0,
            pack_color(180, 180, 180, 255),
            60,
        );
    }

    fn is_transparent(&self) -> bool {
        true // gameplay scene is visible underneath
    }
}

// ---- Main Game (owns the SceneStack) ----

struct SceneDemo {
    stack: Option<SceneStack>,
}

impl Game for SceneDemo {
    fn init(&mut self, _ctx: &mut GameContext) {
        self.stack = Some(SceneStack::new(MenuScene {
            assets: None,
            blink_timer: 0.0,
        }));
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let stack = self.stack.as_mut().unwrap();

        // Handle scene transitions based on input
        let depth = stack.depth();

        if depth == 1 && ctx.first_tick && ctx.input.is_key_just_pressed(Key::Enter) {
            // Menu → Gameplay: need assets
            let white = ctx.load_texture(Path::new("assets/white.png"));
            let font = ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0);
            let assets = SharedAssets {
                white_tex: white,
                font,
            };
            let font_copy = font;
            let white_copy = white;
            stack.replace(
                GameplayScene::new(assets),
                Some(Transition::fade(0.5)),
            );
        } else if depth == 1 && ctx.first_tick && ctx.input.is_key_just_pressed(Key::Escape) {
            // Gameplay → Pause overlay
            // We need font/tex from somewhere — load fresh (cheap, cached)
            let white = ctx.load_texture(Path::new("assets/white.png"));
            let font = ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0);
            stack.push(
                PauseScene {
                    font,
                    white_tex: white,
                },
                None,
            );
        } else if depth == 2 {
            // We're in pause overlay
            if ctx.first_tick && ctx.input.is_key_just_pressed(Key::Enter) {
                stack.pop(None);
            }
            if ctx.first_tick && ctx.input.is_key_just_pressed(Key::KeyQ) {
                // Back to menu
                stack.pop(None);
                stack.replace(
                    MenuScene {
                        assets: None,
                        blink_timer: 0.0,
                    },
                    Some(Transition::fade(0.3)),
                );
            }
        }

        stack.update(ctx, dt);
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        if let Some(stack) = &mut self.stack {
            stack.draw(ctx);
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Scene Stack Demo (v0.2)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.1, 0.1, 0.15))
        .run(SceneDemo { stack: None });
}
