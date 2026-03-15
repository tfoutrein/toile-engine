//! Toile Engine — Async Loading Demo (v0.2 Week 5)
//!
//! Demonstrates background asset loading with a progress bar.
//! Loads multiple textures asynchronously, shows a loading bar,
//! then displays "LOADED!" when done.
//!
//! Run with: `cargo run --example loading_demo`

use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Sprite, TextureHandle, COLOR_WHITE};
use toile_assets::async_loader::{AssetKind, AsyncLoader, RawAsset};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};

struct LoadingDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    loader: AsyncLoader,
    phase: Phase,
    loaded_textures: Vec<TextureHandle>,
    logo_tex: Option<TextureHandle>,
}

enum Phase {
    Init,
    Loading,
    Done,
}

impl Game for LoadingDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        self.logo_tex = Some(ctx.load_texture(Path::new("assets/toile-logo-transparent.png")));
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));

        // Queue textures with simulated delay so the progress bar is visible
        for i in 0..20 {
            let path = match i % 3 {
                0 => "assets/white.png",
                1 => "assets/toile-logo-transparent.png",
                _ => "assets/test_sprite.png",
            };
            self.loader
                .request_with_delay(Path::new(path), AssetKind::Texture, 100);
        }

        self.phase = Phase::Loading;
        log::info!("Loading 30 assets asynchronously...");
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f64) {
        if let Phase::Loading = self.phase {
            // Poll completed assets and upload textures to GPU
            let completed = self.loader.poll();
            for asset in completed {
                if let Ok(RawAsset::Texture { rgba, width, height }) = asset.result {
                    let tex = ctx.create_texture_from_rgba(&rgba, width, height);
                    self.loaded_textures.push(tex);
                }
            }

            if self.loader.all_done() {
                self.phase = Phase::Done;
                log::info!(
                    "All assets loaded! {} textures ready.",
                    self.loaded_textures.len()
                );
            }
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        match self.phase {
            Phase::Init => {}
            Phase::Loading => {
                let progress = self.loader.progress();

                // Logo
                if let Some(logo) = self.logo_tex {
                    ctx.draw_sprite(DrawSprite {
                        texture: logo,
                        position: Vec2::new(0.0, 60.0),
                        size: Vec2::new(150.0, 150.0),
                        rotation: 0.0,
                        color: COLOR_WHITE,
                        layer: 0,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                // "Loading..." text (centered)
                if let Some(font) = self.font {
                    let text = format!("Loading... {:.0}%", progress * 100.0);
                    let text_w = text.len() as f32 * 8.0; // approximate
                    ctx.draw_text(
                        &text,
                        Vec2::new(-text_w * 0.5, -60.0),
                        font,
                        12.0,
                        COLOR_WHITE,
                        10,
                    );
                }

                // Progress bar background
                let bar_w = 400.0;
                let bar_h = 20.0;
                let bar_y = -100.0;
                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: Vec2::new(0.0, bar_y),
                    size: Vec2::new(bar_w, bar_h),
                    rotation: 0.0,
                    color: pack_color(40, 40, 50, 255),
                    layer: 0,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });

                // Progress bar fill
                let fill_w = bar_w * progress;
                if fill_w > 1.0 {
                    ctx.draw_sprite(DrawSprite {
                        texture: tex,
                        position: Vec2::new(-(bar_w - fill_w) * 0.5, bar_y),
                        size: Vec2::new(fill_w, bar_h - 4.0),
                        rotation: 0.0,
                        color: pack_color(80, 200, 120, 255),
                        layer: 1,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }
            }
            Phase::Done => {
                // Show logo
                if let Some(logo) = self.logo_tex {
                    ctx.draw_sprite(DrawSprite {
                        texture: logo,
                        position: Vec2::new(0.0, 60.0),
                        size: Vec2::new(150.0, 150.0),
                        rotation: 0.0,
                        color: COLOR_WHITE,
                        layer: 0,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                if let Some(font) = self.font {
                    let t1 = "ALL LOADED!";
                    let t1_w = t1.len() as f32 * 9.0;
                    ctx.draw_text(
                        t1,
                        Vec2::new(-t1_w * 0.5, -60.0),
                        font,
                        14.0,
                        pack_color(80, 255, 120, 255),
                        10,
                    );
                    let t2 = format!("{} textures ready", self.loaded_textures.len());
                    let t2_w = t2.len() as f32 * 5.5;
                    ctx.draw_text(
                        &t2,
                        Vec2::new(-t2_w * 0.5, -90.0),
                        font,
                        8.0,
                        pack_color(150, 150, 180, 255),
                        10,
                    );
                }
            }
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Async Loading Demo (v0.2)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.08, 0.08, 0.12))
        .run(LoadingDemo {
            white_tex: None,
            font: None,
            loader: AsyncLoader::new(),
            phase: Phase::Init,
            loaded_textures: Vec::new(),
            logo_tex: None,
        });
}
