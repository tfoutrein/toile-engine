/// ADR-028 — SDF Fonts demo
///
/// A single atlas baked at 32px renders crisp at any display size (8 → 48px).
/// Also demonstrates outline, drop-shadow, and animated glow.
///
/// Controls: none (purely visual).
use std::path::Path;

use glam::Vec2;
use toile_app::{App, Game, GameContext, MsdfFontHandle, TextStyle};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::pack_color;

// ─── State ───────────────────────────────────────────────────────────────────

struct MsdfDemo {
    font:        Option<MsdfFontHandle>,
    bitmap_font: Option<toile_app::FontHandle>,
    elapsed:     f32,
}

impl MsdfDemo {
    fn new() -> Self {
        Self { font: None, bitmap_font: None, elapsed: 0.0 }
    }
}

// ─── Game ────────────────────────────────────────────────────────────────────

impl Game for MsdfDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        ctx.camera.zoom = 2.0;   // Retina: 2 screen px per world unit
        let ttf = Path::new("assets/fonts/PressStart2P.ttf");
        // SDF atlas baked at 32px — will render sharply at any size
        self.font        = Some(ctx.load_msdf_font(ttf, 32.0));
        // Bitmap font baked at 32px — used only for the comparison row
        self.bitmap_font = Some(ctx.load_ttf(ttf, 32.0));
    }

    fn update(&mut self, _ctx: &mut GameContext, dt: f64) {
        self.elapsed += dt as f32;
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let Some(font)        = self.font        else { return };
        let Some(bitmap_font) = self.bitmap_font else { return };

        let t = self.elapsed;
        // Layout convention: `y` = TOP of the next item (world-space, y-up).
        // Each draw call uses pen_y = y - size (baseline), so text extends from
        // pen_y up to pen_y + size ≈ y.  After drawing: y -= size + gap.
        // With zoom=2 and 900px window: half-height = 225 wu.
        let mut y = 215.0_f32;

        macro_rules! top {
            // Return pen_y for an item whose top should be at `y`
            ($sz:expr) => { y - $sz }
        }
        macro_rules! step {
            ($sz:expr, $gap:expr) => { y -= $sz + $gap; }
        }

        // ── Title ─────────────────────────────────────────────────────────────
        ctx.draw_text_msdf(
            "SDF FONTS",
            Vec2::new(-76.0, top!(16.0)),
            font,
            &TextStyle {
                size:          16.0,
                color:         pack_color(255, 220, 60, 255),
                outline_width: 0.10,
                outline_color: pack_color(100, 60, 0, 255),
                ..Default::default()
            },
            0,
        );
        step!(16.0, 10.0);

        // ── Multi-size rows — all from the same 32px atlas ────────────────────
        for &(size, label) in &[
            (8.0_f32, "8px tiny"),
            (12.0,    "12px readable"),
            (16.0,    "16px comfortable"),
            (24.0,    "24px heading"),
            (32.0,    "32px reference"),
            (48.0,    "48px large"),
        ] {
            ctx.draw_text_msdf(
                label,
                Vec2::new(-155.0, top!(size)),
                font,
                &TextStyle {
                    size,
                    color: pack_color(200, 220, 255, 255),
                    ..Default::default()
                },
                0,
            );
            step!(size, 5.0);
        }
        step!(0.0, 8.0);

        // ── Outline ───────────────────────────────────────────────────────────
        ctx.draw_text_msdf(
            "OUTLINE TEXT",
            Vec2::new(-110.0, top!(14.0)),
            font,
            &TextStyle {
                size:          14.0,
                color:         pack_color(80, 200, 255, 255),
                outline_width: 0.20,
                outline_color: pack_color(0, 40, 100, 255),
                ..Default::default()
            },
            0,
        );
        step!(14.0, 8.0);

        // ── Drop shadow ───────────────────────────────────────────────────────
        ctx.draw_text_msdf(
            "DROP SHADOW",
            Vec2::new(-95.0, top!(14.0)),
            font,
            &TextStyle {
                size:          14.0,
                color:         pack_color(255, 255, 255, 255),
                shadow_offset: Vec2::new(2.0, -2.5),
                shadow_color:  pack_color(0, 0, 0, 200),
                ..Default::default()
            },
            0,
        );
        step!(14.0, 8.0);

        // ── Animated glow / pulse ─────────────────────────────────────────────
        // Three independent sine waves drive fill brightness, outline width, and hue.
        let pulse    = (t * 2.5).sin() * 0.5 + 0.5;          // 0..1, period ~2.5s
        let hue_t    = (t * 1.2).sin() * 0.5 + 0.5;           // slow color shift
        let ol_width = 0.05 + pulse * 0.30;                    // 0.05 .. 0.35
        let fill_r   = (180.0 + hue_t * 75.0)  as u8;         // pink → orange
        let fill_b   = (255.0 - hue_t * 120.0) as u8;         // purple → pink
        let glow_r   = (80.0  + pulse * 120.0) as u8;
        let glow_a   = (100.0 + pulse * 155.0) as u8;         // fade in/out
        ctx.draw_text_msdf(
            "ANIMATED GLOW",
            Vec2::new(-130.0, top!(24.0)),
            font,
            &TextStyle {
                size:          24.0,
                color:         pack_color(fill_r, 120, fill_b, 255),
                outline_width: ol_width,
                outline_color: pack_color(glow_r, 0, 80, glow_a),
                ..Default::default()
            },
            0,
        );
        step!(24.0, 16.0);

        // ── SDF vs bitmap comparison ─────────────────────────────────────────
        // Both at 48px: bitmap baked at 32px stretched 1.5× → visibly blurry;
        // SDF stays crisp because it re-evaluates the distance field at any size.
        ctx.draw_text_msdf(
            "SDF (crisp at 48px):",
            Vec2::new(-155.0, top!(8.0)),
            font,
            &TextStyle { size: 8.0, color: pack_color(100, 255, 100, 255), ..Default::default() },
            0,
        );
        step!(8.0, 4.0);
        ctx.draw_text_msdf(
            "Abc 123",
            Vec2::new(-155.0, top!(48.0)),
            font,
            &TextStyle { size: 48.0, color: pack_color(255, 255, 255, 255), ..Default::default() },
            0,
        );
        step!(48.0, 12.0);

        ctx.draw_text_msdf(
            "Bitmap (blurry at 48px, baked at 32px):",
            Vec2::new(-155.0, top!(8.0)),
            font,
            &TextStyle { size: 8.0, color: pack_color(255, 100, 100, 255), ..Default::default() },
            0,
        );
        step!(8.0, 4.0);
        // Bitmap atlas baked at 32px, displayed at 48px → 1.5× upscale → pixelated
        ctx.draw_text(
            "Abc 123",
            Vec2::new(-155.0, top!(48.0)),
            bitmap_font,
            48.0,
            pack_color(200, 200, 200, 255),
            0,
        );
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .with_title("Toile — MSDF Font Demo (ADR-028)")
        .with_size(800, 900)
        .with_clear_color(Color::new(0.07, 0.07, 0.12, 1.0))
        .run(MsdfDemo::new());
}
