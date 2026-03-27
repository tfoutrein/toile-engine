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

fn draw_rect(ctx: &mut GameContext, tex: TextureHandle, pos: Vec2, size: Vec2, color: u32, layer: i32) {
    ctx.draw_sprite(DrawSprite {
        texture: tex, position: pos, size, rotation: 0.0, color, layer,
        uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
    });
}

fn btn_color(pressed: bool, base: u32) -> u32 {
    if pressed { pack_color(0, 255, 120, 255) } else { base }
}

impl Game for GamepadDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        let font_path = Path::new("assets/fonts/PressStart2P.ttf");
        if font_path.exists() {
            self.font = Some(ctx.load_ttf(font_path, 48.0));
        }
        // No zoom — draw in world units, 1 unit = 1 pixel at native res
        ctx.camera.zoom = 1.0;
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f64) {
        if ctx.input.is_key_just_pressed(Key::Escape) {
            std::process::exit(0);
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex { Some(t) => t, None => return };
        let font = match self.font { Some(f) => f, None => return };

        // ── Collect all gamepad state ──
        let gp_count = ctx.input.gamepad_count();
        let gp_name = ctx.input.gamepad(0).map(|s| s.name.clone());
        let gp_type = ctx.input.gamepad(0).map(|s| s.gamepad_type);

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

        let lstick = ctx.input.gamepad_left_stick(0);
        let rstick = ctx.input.gamepad_right_stick(0);
        let lt_axis = ctx.input.gamepad_axis(0, GamepadAxis::LeftTrigger);
        let rt_axis = ctx.input.gamepad_axis(0, GamepadAxis::RightTrigger);

        let all_btns: Vec<String> = ctx.input.gamepad(0)
            .map(|s| s.buttons_down.iter().map(|b| format!("{:?}", b)).collect())
            .unwrap_or_default();

        // ── Layout ──
        // Controller centered, text info on the right
        let cx = -100.0; // Controller center X
        let cy = 0.0;    // Controller center Y

        // ── Controller body ──
        let body_w = 420.0;
        let body_h = 240.0;
        draw_rect(ctx, tex, Vec2::new(cx, cy), Vec2::new(body_w, body_h),
            pack_color(55, 55, 65, 245), 0);
        // Grips
        draw_rect(ctx, tex, Vec2::new(cx - 180.0, cy - 50.0), Vec2::new(90.0, 150.0),
            pack_color(48, 48, 58, 245), 0);
        draw_rect(ctx, tex, Vec2::new(cx + 180.0, cy - 50.0), Vec2::new(90.0, 150.0),
            pack_color(48, 48, 58, 245), 0);

        // ── Left stick ──
        let ls_center = Vec2::new(cx - 95.0, cy + 25.0);
        let stick_range = 28.0;
        let stick_r = 22.0;
        draw_rect(ctx, tex, ls_center, Vec2::splat((stick_r + 6.0) * 2.0),
            pack_color(35, 35, 45, 255), 1);
        let ls_pos = ls_center + Vec2::new(lstick.x * stick_range, lstick.y * stick_range);
        draw_rect(ctx, tex, ls_pos, Vec2::splat(stick_r * 2.0),
            btn_color(l3, pack_color(95, 95, 115, 255)), 2);
        draw_rect(ctx, tex, ls_pos, Vec2::splat(6.0), pack_color(160, 160, 180, 255), 3);

        // ── Right stick ──
        let rs_center = Vec2::new(cx + 50.0, cy - 40.0);
        draw_rect(ctx, tex, rs_center, Vec2::splat((stick_r + 6.0) * 2.0),
            pack_color(35, 35, 45, 255), 1);
        let rs_pos = rs_center + Vec2::new(rstick.x * stick_range, rstick.y * stick_range);
        draw_rect(ctx, tex, rs_pos, Vec2::splat(stick_r * 2.0),
            btn_color(r3, pack_color(95, 95, 115, 255)), 2);
        draw_rect(ctx, tex, rs_pos, Vec2::splat(6.0), pack_color(160, 160, 180, 255), 3);

        // ── D-pad ──
        let dp = Vec2::new(cx - 50.0, cy - 40.0);
        let dp_s = 18.0;
        let dp_gap = 22.0;
        draw_rect(ctx, tex, dp, Vec2::splat(dp_s), pack_color(40, 40, 52, 255), 1);
        draw_rect(ctx, tex, dp + Vec2::new(0.0, dp_gap), Vec2::splat(dp_s),
            btn_color(dup, pack_color(72, 72, 92, 255)), 2);
        draw_rect(ctx, tex, dp + Vec2::new(0.0, -dp_gap), Vec2::splat(dp_s),
            btn_color(ddown, pack_color(72, 72, 92, 255)), 2);
        draw_rect(ctx, tex, dp + Vec2::new(-dp_gap, 0.0), Vec2::splat(dp_s),
            btn_color(dleft, pack_color(72, 72, 92, 255)), 2);
        draw_rect(ctx, tex, dp + Vec2::new(dp_gap, 0.0), Vec2::splat(dp_s),
            btn_color(dright, pack_color(72, 72, 92, 255)), 2);

        // ── Face buttons ──
        let fb = Vec2::new(cx + 100.0, cy + 25.0);
        let fb_r = 15.0;
        let fb_gap = 24.0;
        // A (South) - green
        draw_rect(ctx, tex, fb + Vec2::new(0.0, -fb_gap), Vec2::splat(fb_r * 2.0),
            btn_color(south, pack_color(50, 130, 50, 255)), 2);
        // B (East) - red
        draw_rect(ctx, tex, fb + Vec2::new(fb_gap, 0.0), Vec2::splat(fb_r * 2.0),
            btn_color(east, pack_color(170, 50, 50, 255)), 2);
        // X (West) - blue
        draw_rect(ctx, tex, fb + Vec2::new(-fb_gap, 0.0), Vec2::splat(fb_r * 2.0),
            btn_color(west, pack_color(50, 50, 170, 255)), 2);
        // Y (North) - yellow
        draw_rect(ctx, tex, fb + Vec2::new(0.0, fb_gap), Vec2::splat(fb_r * 2.0),
            btn_color(north, pack_color(170, 170, 30, 255)), 2);

        // Face labels
        let lbl_fs = 12.0;
        ctx.draw_text("A", fb + Vec2::new(-5.0, -fb_gap - 5.0), font, lbl_fs, 0xFFFFFFFF, 5);
        ctx.draw_text("B", fb + Vec2::new(fb_gap - 5.0, -5.0), font, lbl_fs, 0xFFFFFFFF, 5);
        ctx.draw_text("X", fb + Vec2::new(-fb_gap - 5.0, -5.0), font, lbl_fs, 0xFFFFFFFF, 5);
        ctx.draw_text("Y", fb + Vec2::new(-5.0, fb_gap - 5.0), font, lbl_fs, 0xFFFFFFFF, 5);

        // ── Shoulder buttons ──
        let sh_y = cy + body_h / 2.0 + 14.0;
        let sh_w = 80.0;
        let sh_h = 22.0;
        draw_rect(ctx, tex, Vec2::new(cx - 100.0, sh_y), Vec2::new(sh_w, sh_h),
            btn_color(lb, pack_color(80, 80, 100, 255)), 2);
        draw_rect(ctx, tex, Vec2::new(cx + 100.0, sh_y), Vec2::new(sh_w, sh_h),
            btn_color(rb, pack_color(80, 80, 100, 255)), 2);
        ctx.draw_text("LB", Vec2::new(cx - 116.0, sh_y - 6.0), font, lbl_fs, 0xFFFFFFFF, 5);
        ctx.draw_text("RB", Vec2::new(cx + 84.0, sh_y - 6.0), font, lbl_fs, 0xFFFFFFFF, 5);

        // ── Triggers — fill bars ──
        let trig_y = sh_y + 28.0;
        let trig_w = 80.0;
        let trig_h = 16.0;

        let lt_val = if lt_axis > 0.01 { lt_axis } else if lt_btn { 1.0 } else { 0.0 };
        let rt_val = if rt_axis > 0.01 { rt_axis } else if rt_btn { 1.0 } else { 0.0 };

        // LT
        draw_rect(ctx, tex, Vec2::new(cx - 100.0, trig_y), Vec2::new(trig_w, trig_h),
            pack_color(40, 40, 52, 200), 1);
        if lt_val > 0.01 {
            let fw = trig_w * lt_val;
            draw_rect(ctx, tex, Vec2::new(cx - 100.0 - (trig_w - fw) / 2.0, trig_y),
                Vec2::new(fw, trig_h), pack_color(255, 110, 50, 255), 2);
        }
        // RT
        draw_rect(ctx, tex, Vec2::new(cx + 100.0, trig_y), Vec2::new(trig_w, trig_h),
            pack_color(40, 40, 52, 200), 1);
        if rt_val > 0.01 {
            let fw = trig_w * rt_val;
            draw_rect(ctx, tex, Vec2::new(cx + 100.0 - (trig_w - fw) / 2.0, trig_y),
                Vec2::new(fw, trig_h), pack_color(255, 110, 50, 255), 2);
        }
        ctx.draw_text("LT", Vec2::new(cx - 116.0, trig_y - 5.0), font, 10.0, 0xFFAAAAAA, 5);
        ctx.draw_text("RT", Vec2::new(cx + 84.0, trig_y - 5.0), font, 10.0, 0xFFAAAAAA, 5);

        // ── Select / Start ──
        let mid_w = 24.0;
        let mid_h = 12.0;
        draw_rect(ctx, tex, Vec2::new(cx - 22.0, cy + 25.0), Vec2::new(mid_w, mid_h),
            btn_color(select, pack_color(72, 72, 88, 255)), 2);
        draw_rect(ctx, tex, Vec2::new(cx + 22.0, cy + 25.0), Vec2::new(mid_w, mid_h),
            btn_color(start, pack_color(72, 72, 88, 255)), 2);

        // ── Text info panel (right side) ──
        let tx = 200.0;
        let fs = 16.0;
        let lh = 24.0;
        let shadow = 2.0;
        let mut ty = 200.0;

        let draw_txt = |ctx: &mut GameContext, text: &str, x: f32, y: f32, color: u32| {
            ctx.draw_text(text, Vec2::new(x + shadow, y - shadow), font, fs, 0xFF000000, 99);
            ctx.draw_text(text, Vec2::new(x, y), font, fs, color, 100);
        };

        draw_txt(ctx, "GAMEPAD DEMO", tx, ty, 0xFFFFFFFF);
        ty -= lh * 1.5;

        draw_txt(ctx, &format!("Connected: {}", gp_count), tx, ty, 0xFFCCCCCC);
        ty -= lh;

        if let Some(name) = &gp_name {
            let type_str = match gp_type {
                Some(toile_app::GamepadType::Xbox) => "Xbox",
                Some(toile_app::GamepadType::PlayStation) => "PlayStation",
                Some(toile_app::GamepadType::SwitchPro) => "Switch",
                _ => "Generic",
            };
            draw_txt(ctx, &format!("{}", name), tx, ty, 0xFF00FF80);
            ty -= lh;
            draw_txt(ctx, &format!("Type: {}", type_str), tx, ty, 0xFF00CC66);
            ty -= lh * 1.2;
        } else {
            draw_txt(ctx, "No gamepad detected", tx, ty, 0xFF6666FF);
            ty -= lh;
            draw_txt(ctx, "Connect a controller", tx, ty, 0xFF888888);
            ty -= lh * 1.2;
        }

        // Sticks
        draw_txt(ctx, &format!("L Stick: {:.2}, {:.2}", lstick.x, lstick.y), tx, ty, 0xFFFFFF88);
        ty -= lh;
        draw_txt(ctx, &format!("R Stick: {:.2}, {:.2}", rstick.x, rstick.y), tx, ty, 0xFFFFFF88);
        ty -= lh;

        // Triggers
        draw_txt(ctx, &format!("LT: {:.2}  RT: {:.2}", lt_val, rt_val), tx, ty,
            if lt_val > 0.01 || rt_val > 0.01 { 0xFFFF8844 } else { 0xFF888888 });
        ty -= lh * 1.2;

        // Active buttons
        if !all_btns.is_empty() {
            draw_txt(ctx, "Active:", tx, ty, 0xFFFF88FF);
            ty -= lh;
            let btns_str = all_btns.join(", ");
            // Split long lines
            for chunk in btns_str.as_bytes().chunks(30) {
                let s = String::from_utf8_lossy(chunk);
                draw_txt(ctx, &s, tx, ty, 0xFFDD66DD);
                ty -= lh;
            }
        } else {
            draw_txt(ctx, "Active: ---", tx, ty, 0xFF666666);
            ty -= lh;
        }

        let _ = ty;
    }
}

fn main() {
    App::new()
        .with_title("Gamepad Demo - Connect a controller!")
        .with_size(1280, 720)
        .run(GamepadDemo::new());
}
