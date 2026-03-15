//! Toile Engine — Tweening & Easing Demo (v0.2)
//!
//! Visual showcase of all easing functions.
//! Each row animates a sprite with a different easing curve.
//!
//! Keys:
//!   Space: restart all animations
//!   1: Once mode  2: Loop mode  3: PingPong mode
//!   Up/Down: change animation duration
//!   F3: debug overlay
//!
//! Run with: `cargo run --example tween_demo`

use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, TextureHandle, COLOR_WHITE};
use toile_core::color::Color;
use toile_core::tween::{Easing, RepeatMode, Tween};
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};

const EASINGS: &[(Easing, &str)] = &[
    (Easing::Linear, "Linear"),
    (Easing::QuadIn, "QuadIn"),
    (Easing::QuadOut, "QuadOut"),
    (Easing::QuadInOut, "QuadInOut"),
    (Easing::CubicIn, "CubicIn"),
    (Easing::CubicOut, "CubicOut"),
    (Easing::CubicInOut, "CubicInOut"),
    (Easing::SineIn, "SineIn"),
    (Easing::SineOut, "SineOut"),
    (Easing::SineInOut, "SineInOut"),
    (Easing::ExpoIn, "ExpoIn"),
    (Easing::ExpoOut, "ExpoOut"),
    (Easing::BackIn, "BackIn"),
    (Easing::BackOut, "BackOut"),
    (Easing::BounceOut, "BounceOut"),
];

struct TweenDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    tweens: Vec<Tween>,
    duration: f32,
    repeat: RepeatMode,
}

impl TweenDemo {
    fn rebuild_tweens(&mut self) {
        self.tweens.clear();
        for (easing, _) in EASINGS {
            self.tweens.push(
                Tween::new(0.0, 1.0, self.duration)
                    .with_easing(*easing)
                    .with_repeat(self.repeat),
            );
        }
    }
}

impl Game for TweenDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));
        self.rebuild_tweens();
        log::info!("Tween Demo! Space=restart, 1/2/3=Once/Loop/PingPong, Up/Down=speed");
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;

        // Advance all tweens
        for tw in &mut self.tweens {
            tw.advance(dt);
        }

        // Restart
        if ctx.input.is_key_just_pressed(Key::Space) {
            self.rebuild_tweens();
        }

        // Mode switching
        if ctx.input.is_key_just_pressed(Key::Digit1) {
            self.repeat = RepeatMode::Once;
            self.rebuild_tweens();
        }
        if ctx.input.is_key_just_pressed(Key::Digit2) {
            self.repeat = RepeatMode::Loop;
            self.rebuild_tweens();
        }
        if ctx.input.is_key_just_pressed(Key::Digit3) {
            self.repeat = RepeatMode::PingPong;
            self.rebuild_tweens();
        }

        // Duration
        if ctx.input.is_key_just_pressed(Key::ArrowUp) {
            self.duration = (self.duration + 0.5).min(5.0);
            self.rebuild_tweens();
        }
        if ctx.input.is_key_just_pressed(Key::ArrowDown) {
            self.duration = (self.duration - 0.5).max(0.5);
            self.rebuild_tweens();
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        let tl = ctx.camera.top_left();
        let br = ctx.camera.bottom_right();
        let view_w = br.x - tl.x;
        let view_h = tl.y - br.y;

        let row_count = EASINGS.len() as f32;
        let row_h = view_h / (row_count + 2.0); // +2 for header + footer
        let track_left = tl.x + 130.0;
        let track_right = br.x - 20.0;
        let track_w = track_right - track_left;

        // Header
        if let Some(font) = self.font {
            let mode_name = match self.repeat {
                RepeatMode::Once => "Once",
                RepeatMode::Loop => "Loop",
                RepeatMode::PingPong => "PingPong",
            };
            ctx.draw_text(
                &format!("Easing Demo | {} | {:.1}s | Space=restart", mode_name, self.duration),
                Vec2::new(tl.x + 10.0, tl.y - 10.0),
                font,
                10.0,
                COLOR_WHITE,
                10,
            );
            ctx.draw_text(
                "1:Once 2:Loop 3:PingPong Up/Down:speed",
                Vec2::new(tl.x + 10.0, tl.y - 28.0),
                font,
                6.0,
                pack_color(150, 150, 170, 255),
                10,
            );
        }

        // Draw each easing row
        for (i, ((_, name), tw)) in EASINGS.iter().zip(self.tweens.iter()).enumerate() {
            let y = tl.y - (i as f32 + 2.0) * row_h;
            let t = tw.value();

            // Label
            if let Some(font) = self.font {
                ctx.draw_text(
                    name,
                    Vec2::new(tl.x + 8.0, y + 4.0),
                    font,
                    6.0,
                    pack_color(200, 200, 220, 255),
                    10,
                );
            }

            // Track background
            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: Vec2::new(track_left + track_w * 0.5, y),
                size: Vec2::new(track_w, 2.0),
                rotation: 0.0,
                color: pack_color(50, 50, 60, 255),
                layer: 0,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });

            // Animated sprite
            let x = track_left + t * track_w;
            let ball_size = row_h * 0.6;

            // Color based on row (rainbow)
            let hue = i as f32 / row_count;
            let r = ((hue * 6.0 - 0.0).sin().max(0.0) * 255.0) as u8;
            let g = ((hue * 6.0 - 2.0).sin().max(0.0) * 255.0) as u8;
            let b = ((hue * 6.0 - 4.0).sin().max(0.0) * 255.0) as u8;

            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: Vec2::new(x, y),
                size: Vec2::splat(ball_size),
                rotation: t * std::f32::consts::TAU,
                color: pack_color(r.max(80), g.max(80), b.max(80), 255),
                layer: 1,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Tweening & Easing Demo (v0.2)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.08, 0.08, 0.12))
        .run(TweenDemo {
            white_tex: None,
            font: None,
            tweens: Vec::new(),
            duration: 2.0,
            repeat: RepeatMode::PingPong,
        });
}
