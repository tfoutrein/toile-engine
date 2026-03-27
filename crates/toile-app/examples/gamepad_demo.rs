//! Gamepad demo — visual controller with buttons that light up and sticks that move.
//!
//! Run with: cargo run --example gamepad_demo

use std::path::Path;

use glam::Vec2;
use toile_app::{App, Game, GameContext, Key, GamepadButton, GamepadAxis, TextureHandle, FontHandle};
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};

struct GamepadDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
}

impl GamepadDemo {
    fn new() -> Self {
        Self { white_tex: None, font: None }
    }
}

// Drawing helpers
fn draw_rect(ctx: &mut GameContext, tex: TextureHandle, pos: Vec2, size: Vec2, color: u32, layer: i32) {
    ctx.draw_sprite(DrawSprite {
        texture: tex, position: pos, size, rotation: 0.0, color, layer,
        uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
    });
}

fn draw_circle(ctx: &mut GameContext, tex: TextureHandle, pos: Vec2, radius: f32, color: u32, layer: i32) {
    // Approximate circle with a square (good enough for small buttons)
    draw_rect(ctx, tex, pos, Vec2::splat(radius * 2.0), color, layer);
}

fn btn_color(pressed: bool, base: u32) -> u32 {
    if pressed { 0xFF00FF80 } else { base }
}

fn axis_bar_color(value: f32) -> u32 {
    if value.abs() > 0.01 {
        pack_color((100.0 + 155.0 * value.abs()) as u8, 200, 100, 255)
    } else {
        pack_color(60, 60, 80, 200)
    }
}

impl Game for GamepadDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        let font_path = Path::new("assets/fonts/PressStart2P.ttf");
        if font_path.exists() {
            self.font = Some(ctx.load_ttf(font_path, 24.0));
        }
        ctx.camera.zoom = 2.0;
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f64) {
        if ctx.input.is_key_just_pressed(Key::Escape) {
            std::process::exit(0);
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex { Some(t) => t, None => return };
        let font = match self.font { Some(f) => f, None => return };
        let zoom = ctx.camera.zoom;

        // Collect all gamepad state upfront
        let gp_count = ctx.input.gamepad_count();
        let gp_name = ctx.input.gamepad(0).map(|s| s.name.clone()).unwrap_or_default();
        let gp_type = ctx.input.gamepad(0).map(|s| s.gamepad_type);

        // Buttons
        let south = ctx.input.is_gamepad_button_down(0, GamepadButton::South);
        let east = ctx.input.is_gamepad_button_down(0, GamepadButton::East);
        let west = ctx.input.is_gamepad_button_down(0, GamepadButton::West);
        let north = ctx.input.is_gamepad_button_down(0, GamepadButton::North);
        let lb = ctx.input.is_gamepad_button_down(0, GamepadButton::LeftShoulder);
        let rb = ctx.input.is_gamepad_button_down(0, GamepadButton::RightShoulder);
        let lt_btn = ctx.input.is_gamepad_button_down(0, GamepadButton::LeftTrigger);
        let rt_btn = ctx.input.is_gamepad_button_down(0, GamepadButton::RightTrigger);
        let select = ctx.input.is_gamepad_button_down(0, GamepadButton::Select);
        let start = ctx.input.is_gamepad_button_down(0, GamepadButton::Start);
        let l3 = ctx.input.is_gamepad_button_down(0, GamepadButton::LeftStick);
        let r3 = ctx.input.is_gamepad_button_down(0, GamepadButton::RightStick);
        let dup = ctx.input.is_gamepad_button_down(0, GamepadButton::DPadUp);
        let ddown = ctx.input.is_gamepad_button_down(0, GamepadButton::DPadDown);
        let dleft = ctx.input.is_gamepad_button_down(0, GamepadButton::DPadLeft);
        let dright = ctx.input.is_gamepad_button_down(0, GamepadButton::DPadRight);

        // Axes
        let lstick = ctx.input.gamepad_left_stick(0);
        let rstick = ctx.input.gamepad_right_stick(0);
        let lt_axis = ctx.input.gamepad_axis(0, GamepadAxis::LeftTrigger);
        let rt_axis = ctx.input.gamepad_axis(0, GamepadAxis::RightTrigger);

        // Layout constants
        let cx = 0.0_f32;  // Center of the controller drawing
        let cy = 0.0_f32;
        let s = 1.0 / zoom; // Scale factor

        // ── Controller body ──
        let body_w = 280.0 * s;
        let body_h = 160.0 * s;
        draw_rect(ctx, tex, Vec2::new(cx, cy), Vec2::new(body_w, body_h),
            pack_color(50, 50, 60, 240), 0);
        // Grips
        draw_rect(ctx, tex, Vec2::new(cx - 120.0 * s, cy - 30.0 * s), Vec2::new(60.0 * s, 100.0 * s),
            pack_color(45, 45, 55, 240), 0);
        draw_rect(ctx, tex, Vec2::new(cx + 120.0 * s, cy - 30.0 * s), Vec2::new(60.0 * s, 100.0 * s),
            pack_color(45, 45, 55, 240), 0);

        // ── Left stick ──
        let ls_center = Vec2::new(cx - 65.0 * s, cy + 15.0 * s);
        let stick_range = 18.0 * s;
        let stick_r = 14.0 * s;
        // Base ring
        draw_circle(ctx, tex, ls_center, stick_r + 4.0 * s, pack_color(35, 35, 45, 255), 1);
        // Stick position
        let ls_pos = ls_center + Vec2::new(lstick.x * stick_range, lstick.y * stick_range);
        draw_circle(ctx, tex, ls_pos, stick_r, btn_color(l3, pack_color(90, 90, 110, 255)), 2);
        // Center dot
        draw_circle(ctx, tex, ls_pos, 3.0 * s, pack_color(150, 150, 170, 255), 3);

        // ── Right stick ──
        let rs_center = Vec2::new(cx + 35.0 * s, cy - 25.0 * s);
        draw_circle(ctx, tex, rs_center, stick_r + 4.0 * s, pack_color(35, 35, 45, 255), 1);
        let rs_pos = rs_center + Vec2::new(rstick.x * stick_range, rstick.y * stick_range);
        draw_circle(ctx, tex, rs_pos, stick_r, btn_color(r3, pack_color(90, 90, 110, 255)), 2);
        draw_circle(ctx, tex, rs_pos, 3.0 * s, pack_color(150, 150, 170, 255), 3);

        // ── D-pad ──
        let dp = Vec2::new(cx - 35.0 * s, cy - 25.0 * s);
        let dp_s = 11.0 * s;
        let dp_gap = 13.0 * s;
        // Center
        draw_rect(ctx, tex, dp, Vec2::splat(dp_s), pack_color(40, 40, 50, 255), 1);
        // Directions
        draw_rect(ctx, tex, dp + Vec2::new(0.0, dp_gap), Vec2::splat(dp_s), btn_color(dup, pack_color(70, 70, 90, 255)), 2);
        draw_rect(ctx, tex, dp + Vec2::new(0.0, -dp_gap), Vec2::splat(dp_s), btn_color(ddown, pack_color(70, 70, 90, 255)), 2);
        draw_rect(ctx, tex, dp + Vec2::new(-dp_gap, 0.0), Vec2::splat(dp_s), btn_color(dleft, pack_color(70, 70, 90, 255)), 2);
        draw_rect(ctx, tex, dp + Vec2::new(dp_gap, 0.0), Vec2::splat(dp_s), btn_color(dright, pack_color(70, 70, 90, 255)), 2);

        // ── Face buttons (A/B/X/Y) ──
        let fb = Vec2::new(cx + 65.0 * s, cy + 15.0 * s);
        let fb_r = 9.0 * s;
        let fb_gap = 15.0 * s;
        // South (A) — green
        draw_circle(ctx, tex, fb + Vec2::new(0.0, -fb_gap), fb_r,
            btn_color(south, pack_color(60, 140, 60, 255)), 2);
        // East (B) — red
        draw_circle(ctx, tex, fb + Vec2::new(fb_gap, 0.0), fb_r,
            btn_color(east, pack_color(180, 60, 60, 255)), 2);
        // West (X) — blue
        draw_circle(ctx, tex, fb + Vec2::new(-fb_gap, 0.0), fb_r,
            btn_color(west, pack_color(60, 60, 180, 255)), 2);
        // North (Y) — yellow
        draw_circle(ctx, tex, fb + Vec2::new(0.0, fb_gap), fb_r,
            btn_color(north, pack_color(180, 180, 40, 255)), 2);

        // Button labels
        let lbl_s = 7.0 * s;
        ctx.draw_text("A", fb + Vec2::new(-3.0 * s, -fb_gap - 4.0 * s), font, lbl_s, 0xFFFFFFFF, 5);
        ctx.draw_text("B", fb + Vec2::new(fb_gap - 3.0 * s, -4.0 * s), font, lbl_s, 0xFFFFFFFF, 5);
        ctx.draw_text("X", fb + Vec2::new(-fb_gap - 3.0 * s, -4.0 * s), font, lbl_s, 0xFFFFFFFF, 5);
        ctx.draw_text("Y", fb + Vec2::new(-3.0 * s, fb_gap - 4.0 * s), font, lbl_s, 0xFFFFFFFF, 5);

        // ── Shoulder buttons (LB/RB) ──
        let shoulder_y = cy + body_h / 2.0 + 8.0 * s;
        let shoulder_w = 50.0 * s;
        let shoulder_h = 14.0 * s;
        draw_rect(ctx, tex, Vec2::new(cx - 70.0 * s, shoulder_y), Vec2::new(shoulder_w, shoulder_h),
            btn_color(lb, pack_color(80, 80, 100, 255)), 2);
        draw_rect(ctx, tex, Vec2::new(cx + 70.0 * s, shoulder_y), Vec2::new(shoulder_w, shoulder_h),
            btn_color(rb, pack_color(80, 80, 100, 255)), 2);
        ctx.draw_text("LB", Vec2::new(cx - 80.0 * s, shoulder_y - 4.0 * s), font, lbl_s, 0xFFFFFFFF, 5);
        ctx.draw_text("RB", Vec2::new(cx + 60.0 * s, shoulder_y - 4.0 * s), font, lbl_s, 0xFFFFFFFF, 5);

        // ── Triggers (LT/RT) — bar that fills with analog value ──
        let trig_y = shoulder_y + 18.0 * s;
        let trig_w = 50.0 * s;
        let trig_h = 10.0 * s;
        // LT background
        draw_rect(ctx, tex, Vec2::new(cx - 70.0 * s, trig_y), Vec2::new(trig_w, trig_h),
            pack_color(40, 40, 50, 200), 1);
        // LT fill (analog axis OR digital button)
        let lt_val = if lt_axis > 0.01 { lt_axis } else if lt_btn { 1.0 } else { 0.0 };
        if lt_val > 0.01 {
            let fill_w = trig_w * lt_val;
            draw_rect(ctx, tex,
                Vec2::new(cx - 70.0 * s - (trig_w - fill_w) / 2.0, trig_y),
                Vec2::new(fill_w, trig_h),
                pack_color(255, 100, 50, 255), 2);
        }
        // RT background
        draw_rect(ctx, tex, Vec2::new(cx + 70.0 * s, trig_y), Vec2::new(trig_w, trig_h),
            pack_color(40, 40, 50, 200), 1);
        let rt_val = if rt_axis > 0.01 { rt_axis } else if rt_btn { 1.0 } else { 0.0 };
        if rt_val > 0.01 {
            let fill_w = trig_w * rt_val;
            draw_rect(ctx, tex,
                Vec2::new(cx + 70.0 * s - (trig_w - fill_w) / 2.0, trig_y),
                Vec2::new(fill_w, trig_h),
                pack_color(255, 100, 50, 255), 2);
        }
        ctx.draw_text("LT", Vec2::new(cx - 80.0 * s, trig_y - 3.0 * s), font, lbl_s * 0.8, 0xFFAAAAAA, 5);
        ctx.draw_text("RT", Vec2::new(cx + 60.0 * s, trig_y - 3.0 * s), font, lbl_s * 0.8, 0xFFAAAAAA, 5);

        // ── Select / Start ──
        let mid_y = cy + 15.0 * s;
        let mid_btn_w = 16.0 * s;
        let mid_btn_h = 8.0 * s;
        draw_rect(ctx, tex, Vec2::new(cx - 16.0 * s, mid_y), Vec2::new(mid_btn_w, mid_btn_h),
            btn_color(select, pack_color(70, 70, 85, 255)), 2);
        draw_rect(ctx, tex, Vec2::new(cx + 16.0 * s, mid_y), Vec2::new(mid_btn_w, mid_btn_h),
            btn_color(start, pack_color(70, 70, 85, 255)), 2);

        // ── Axis values text ──
        let vp = ctx.camera.viewport_size();
        let half_h = vp.y / (2.0 * zoom);
        let fs = 10.0 * s;
        let info_y = cy - half_h + 10.0 * s;

        // Title
        let title_y = cy + half_h - 16.0 * s;
        ctx.draw_text("GAMEPAD DEMO", Vec2::new(cx - 50.0 * s, title_y), font, 12.0 * s, 0xFFFFFFFF, 10);

        if gp_count == 0 {
            ctx.draw_text("No gamepad connected", Vec2::new(cx - 75.0 * s, cy + 110.0 * s), font, fs, 0xFF6666FF, 10);
            ctx.draw_text("Connect a Bluetooth controller", Vec2::new(cx - 110.0 * s, cy + 96.0 * s), font, fs * 0.9, 0xFF888888, 10);
        } else {
            let type_str = match gp_type {
                Some(toile_app::GamepadType::Xbox) => "Xbox",
                Some(toile_app::GamepadType::PlayStation) => "PlayStation",
                Some(toile_app::GamepadType::SwitchPro) => "Switch",
                _ => "Generic",
            };
            ctx.draw_text(&format!("{} ({})", gp_name, type_str),
                Vec2::new(cx - 120.0 * s, cy + 110.0 * s), font, fs * 0.9, 0xFF00FF00, 10);
        }

        // Stick + trigger values
        ctx.draw_text(&format!("L: {:.2},{:.2}  R: {:.2},{:.2}  LT:{:.2} RT:{:.2}",
            lstick.x, lstick.y, rstick.x, rstick.y, lt_val, rt_val),
            Vec2::new(cx - 130.0 * s, info_y), font, fs * 0.8, 0xFF888888, 10);
    }
}

fn main() {
    App::new()
        .with_title("Gamepad Demo - Connect a controller!")
        .with_size(1280, 720)
        .run(GamepadDemo::new());
}
