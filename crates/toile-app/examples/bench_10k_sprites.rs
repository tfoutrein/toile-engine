//! Toile Engine — 10,000 Sprites Benchmark
//!
//! Spawns 10,000 moving sprites across 4 textures and 4 layers.
//! Press F3 to toggle the debug overlay (FPS, draw calls, batches).
//!
//! Run with: `cargo run --release --example bench_10k_sprites`

use std::path::Path;

use toile_app::{App, Game, GameContext, Sprite, TextureHandle, COLOR_WHITE};
use toile_core::glam::Vec2;

const SPRITE_COUNT: usize = 10_000;
const TEXTURE_COUNT: usize = 4;
const LAYER_COUNT: usize = 4;

/// Simple xorshift32 RNG (no external dependency).
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        let t = (self.next_u32() as f64) / (u32::MAX as f64);
        min + (max - min) * t as f32
    }
}

struct SpriteData {
    pos: Vec2,
    vel: Vec2,
    tex_idx: usize,
    layer: i32,
}

struct BenchGame {
    textures: Vec<TextureHandle>,
    sprites: Vec<SpriteData>,
}

impl Game for BenchGame {
    fn init(&mut self, ctx: &mut GameContext) {
        // Load the same texture 4 times (simulates different textures for batching)
        for _ in 0..TEXTURE_COUNT {
            self.textures
                .push(ctx.load_texture(Path::new("assets/test_sprite.png")));
        }

        let mut rng = Rng::new(42);

        for i in 0..SPRITE_COUNT {
            self.sprites.push(SpriteData {
                pos: Vec2::new(rng.range(-600.0, 600.0), rng.range(-340.0, 340.0)),
                vel: Vec2::new(rng.range(-80.0, 80.0), rng.range(-80.0, 80.0)),
                tex_idx: i % TEXTURE_COUNT,
                layer: (i % LAYER_COUNT) as i32,
            });
        }

        log::info!(
            "Benchmark: {} sprites, {} textures, {} layers. Press F3 for debug overlay.",
            SPRITE_COUNT,
            TEXTURE_COUNT,
            LAYER_COUNT
        );
    }

    fn update(&mut self, _ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;
        for s in &mut self.sprites {
            s.pos += s.vel * dt;
            if s.pos.x.abs() > 650.0 {
                s.vel.x = -s.vel.x;
            }
            if s.pos.y.abs() > 380.0 {
                s.vel.y = -s.vel.y;
            }
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        for s in &self.sprites {
            ctx.draw_sprite(Sprite {
                texture: self.textures[s.tex_idx],
                position: s.pos,
                size: Vec2::new(16.0, 16.0),
                rotation: 0.0,
                color: COLOR_WHITE,
                layer: s.layer,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile Benchmark — 10,000 Sprites")
        .with_size(1280, 720)
        .run(BenchGame {
            textures: Vec::new(),
            sprites: Vec::new(),
        });
}
