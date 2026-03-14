//! Toile Engine — Week 2 Milestone
//!
//! A sprite on screen moved by arrow keys, camera zoom with scroll wheel.
//!
//! Run with: `cargo run --example sprite_input`

use std::path::Path;

use toile_app::{App, Camera, Game, GameContext, Key, Sprite, TextureHandle, COLOR_WHITE};
use toile_core::glam::Vec2;

struct MyGame {
    texture: Option<TextureHandle>,
    pos: Vec2,
}

impl Game for MyGame {
    fn init(&mut self, ctx: &mut GameContext) {
        self.texture = Some(ctx.load_texture(Path::new("assets/test_sprite.png")));
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let speed = 200.0 * dt as f32;

        if ctx.input.is_key_down(Key::ArrowRight) || ctx.input.is_key_down(Key::KeyD) {
            self.pos.x += speed;
        }
        if ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA) {
            self.pos.x -= speed;
        }
        if ctx.input.is_key_down(Key::ArrowUp) || ctx.input.is_key_down(Key::KeyW) {
            self.pos.y += speed;
        }
        if ctx.input.is_key_down(Key::ArrowDown) || ctx.input.is_key_down(Key::KeyS) {
            self.pos.y -= speed;
        }

        // Camera zoom with scroll wheel
        let scroll = ctx.input.scroll_delta();
        if scroll.y != 0.0 {
            ctx.camera.zoom *= 1.0 + scroll.y * 0.1;
            ctx.camera.zoom = ctx.camera.zoom.clamp(0.1, 10.0);
        }

        // Reset camera with Space
        if ctx.input.is_key_just_pressed(Key::Space) {
            ctx.camera.position = Vec2::ZERO;
            ctx.camera.zoom = 1.0;
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        if let Some(tex) = self.texture {
            ctx.draw_sprite(Sprite {
                texture: tex,
                position: self.pos,
                size: Vec2::new(64.0, 64.0),
                rotation: 0.0,
                color: COLOR_WHITE,
                layer: 0,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Sprite + Input (Week 2)")
        .with_size(1280, 720)
        .run(MyGame {
            texture: None,
            pos: Vec2::ZERO,
        });
}
