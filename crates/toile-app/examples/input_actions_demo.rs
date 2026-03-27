//! Input Actions demo — shows named actions working with keyboard AND gamepad.
//!
//! Run with: cargo run --example input_actions_demo
//!
//! Move with WASD / arrows / left stick / D-pad.
//! Jump with Space / gamepad A button.
//! Fire with X key / mouse left / gamepad RB or X button.
//! The same "move" / "jump" / "fire" actions work on any device.

use std::path::Path;

use glam::Vec2;
use toile_app::{App, Game, GameContext, Key, TextureHandle, FontHandle};
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};

struct InputActionsDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    player_pos: Vec2,
}

impl Game for InputActionsDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        let font_path = Path::new("assets/fonts/PressStart2P.ttf");
        if font_path.exists() {
            self.font = Some(ctx.load_ttf(font_path, 48.0));
        }
        ctx.camera.zoom = 1.0;
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;

        // Move via the "move" action — works with WASD, arrows, stick, D-pad
        let move_dir = ctx.actions.get_vec2("move");
        self.player_pos += move_dir * 250.0 * dt;

        if ctx.input.is_key_just_pressed(Key::Escape) {
            std::process::exit(0);
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex { Some(t) => t, None => return };
        let font = match self.font { Some(f) => f, None => return };

        // Collect action states
        let move_vec = ctx.actions.get_vec2("move");
        let jump_pressed = ctx.actions.is_pressed("jump");
        let jump_just = ctx.actions.is_just_pressed("jump");
        let fire_pressed = ctx.actions.is_pressed("fire");
        let fire_just = ctx.actions.is_just_pressed("fire");
        let gp_count = ctx.input.gamepad_count();
        let gp_name = ctx.input.gamepad(0).map(|s| s.name.clone());

        // ── Player ──
        let player_color = if fire_pressed {
            pack_color(255, 80, 50, 255)   // Red when firing
        } else if jump_pressed {
            pack_color(80, 255, 120, 255)   // Green when jumping
        } else {
            pack_color(80, 140, 255, 255)   // Blue default
        };

        let player_size = 40.0;
        ctx.draw_sprite(DrawSprite {
            texture: tex,
            position: self.player_pos,
            size: Vec2::splat(player_size),
            rotation: 0.0,
            color: player_color,
            layer: 1,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });

        // Direction indicator on player
        if move_vec.length() > 0.1 {
            let arrow_pos = self.player_pos + move_vec * 30.0;
            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: arrow_pos,
                size: Vec2::splat(10.0),
                rotation: 0.0,
                color: pack_color(255, 255, 100, 255),
                layer: 2,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // ── HUD ──
        let fs = 16.0;
        let lh = 24.0;
        let shadow = 2.0;
        let left = -300.0;
        let mut y = 300.0;

        let draw_txt = |ctx: &mut GameContext, text: &str, x: f32, y: f32, color: u32| {
            ctx.draw_text(text, Vec2::new(x + shadow, y - shadow), font, fs, 0xFF000000, 99);
            ctx.draw_text(text, Vec2::new(x, y), font, fs, color, 100);
        };

        draw_txt(ctx, "INPUT ACTIONS DEMO", left, y, 0xFFFFFFFF);
        y -= lh * 1.5;

        // Gamepad status
        if let Some(name) = &gp_name {
            draw_txt(ctx, &format!("Gamepad: {}", name), left, y, 0xFF00FF80);
        } else {
            draw_txt(ctx, "No gamepad (keyboard only)", left, y, 0xFF888888);
        }
        y -= lh * 1.3;

        // Action states
        draw_txt(ctx, "--- ACTIONS ---", left, y, 0xFFCCCCCC);
        y -= lh;

        let move_color = if move_vec.length() > 0.1 { 0xFF00FF80 } else { 0xFF666666 };
        draw_txt(ctx, &format!("move: ({:.2}, {:.2})", move_vec.x, move_vec.y), left, y, move_color);
        y -= lh;

        let jump_color = if jump_pressed { 0xFF00FF80 } else { 0xFF666666 };
        let jump_label = if jump_just { "jump: JUST PRESSED!" } else if jump_pressed { "jump: HELD" } else { "jump: ---" };
        draw_txt(ctx, jump_label, left, y, jump_color);
        y -= lh;

        let fire_color = if fire_pressed { 0xFFFF5030 } else { 0xFF666666 };
        let fire_label = if fire_just { "fire: JUST PRESSED!" } else if fire_pressed { "fire: HELD" } else { "fire: ---" };
        draw_txt(ctx, fire_label, left, y, fire_color);
        y -= lh * 1.5;

        // Controls
        draw_txt(ctx, "--- CONTROLS ---", left, y, 0xFFCCCCCC);
        y -= lh;
        draw_txt(ctx, "move: WASD/Arrows/Stick/DPad", left, y, 0xFF888888);
        y -= lh;
        draw_txt(ctx, "jump: Space / Gamepad A", left, y, 0xFF888888);
        y -= lh;
        draw_txt(ctx, "fire: X key / Mouse / RB", left, y, 0xFF888888);
        y -= lh;

        let _ = y;
    }
}

fn main() {
    App::new()
        .with_title("Input Actions Demo")
        .with_size(1280, 720)
        .run(InputActionsDemo {
            white_tex: None,
            font: None,
            player_pos: Vec2::ZERO,
        });
}
