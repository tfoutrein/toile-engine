//! Headless GPU smoke tests. These need a usable GPU; if none is available
//! (e.g. some CI runners) the test logs and returns instead of failing.

use glam::Vec2;
use toile_core::color::Color;
use toile_graphics::camera::Camera2D;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};
use toile_harness::Harness;

const W: u32 = 64;
const H: u32 = 64;

fn px(buf: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
    let i = ((y * W + x) * 4) as usize;
    (buf[i], buf[i + 1], buf[i + 2], buf[i + 3])
}

#[test]
fn headless_renders_a_green_quad_and_clear_color() {
    let mut h = match Harness::new(W, H) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipping headless GPU test (no usable GPU): {e}");
            return;
        }
    };

    let mut cam = Camera2D::new(W as f32, H as f32);
    cam.zoom = 1.0;
    cam.position = Vec2::ZERO;

    // 1) A green quad larger than the viewport, on a black clear.
    let green = vec![DrawSprite {
        texture: h.white(),
        position: Vec2::ZERO,
        size: Vec2::splat(W as f32 * 2.0),
        rotation: 0.0,
        color: pack_color(0, 255, 0, 255),
        layer: 0,
        uv_min: Vec2::ZERO,
        uv_max: Vec2::ONE,
    }];
    h.render(&cam, &green, Color::BLACK);
    let buf = h.pixels().expect("readback");
    assert_eq!(buf.len(), (W * H * 4) as usize);
    let (r, g, b, a) = px(&buf, W / 2, H / 2);
    assert!(g > 200 && r < 60 && b < 60 && a > 200, "center should be green, got {r},{g},{b},{a}");

    // 2) Empty draw list with a pure-blue clear color fills the frame blue.
    h.render(&cam, &[], Color::new(0.0, 0.0, 1.0, 1.0));
    let buf = h.pixels().expect("readback");
    let (r, g, b, _) = px(&buf, 2, 2);
    assert!(b > 200 && r < 60 && g < 60, "corner should be blue, got {r},{g},{b}");
}
