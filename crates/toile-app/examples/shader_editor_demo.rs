/// Shader Graph demo — Toile Engine v0.4  (ADR-027)
///
/// Each effect is defined as a `ShaderGraph` (node graph data model),
/// compiled to WGSL at startup, then pushed as `PostEffect::Custom(...)`.
///
/// Press 1–4 to switch effects, Space to toggle on/off, G to print the
/// active graph's JSON to stdout.
///
/// Effects:
///   1 — Ripple     : UV distorted by time-driven sine waves
///   2 — Rainbow    : Hue rotation via HSV → RGB conversion
///   3 — Glitch     : Row-shifted noise displacement
///   4 — Vignette+  : Distance-based darkening with value noise texture
use std::path::Path;
use std::sync::Arc;

use glam::Vec2;
use toile_app::*;

// ── Shader graph builders ─────────────────────────────────────────────────────

/// 1. Ripple: UV.x += sin(UV.y * 20 + time * 3) * 0.025
fn build_ripple() -> ShaderGraph {
    let mut g = ShaderGraph::new("ripple");

    let uv     = g.add_node(1,  NodeKind::UV);
    let time   = g.add_node(2,  NodeKind::Time);
    let c20    = g.add_node(3,  NodeKind::ConstF32(20.0));
    let c3     = g.add_node(4,  NodeKind::ConstF32(3.0));
    let c025   = g.add_node(5,  NodeKind::ConstF32(0.025));
    let split  = g.add_node(6,  NodeKind::SplitVec2);  // → x, y
    let ymul   = g.add_node(7,  NodeKind::MulF);       // y * 20
    let tscale = g.add_node(8,  NodeKind::MulF);       // time * 3
    let yoff   = g.add_node(9,  NodeKind::AddF);       // y*20 + t*3
    let sinv   = g.add_node(10, NodeKind::Sin);        // sin(...)
    let amp    = g.add_node(11, NodeKind::MulF);       // sin * 0.025
    let newx   = g.add_node(12, NodeKind::AddF);       // x + amp
    let newuv  = g.add_node(13, NodeKind::CombineVec2);
    let scene  = g.add_node(14, NodeKind::SceneColor);
    let out    = g.add_node(15, NodeKind::FragmentColor);

    // y * 20
    g.connect(uv, 0, split, 0);
    g.connect(split, 1, ymul, 0);   // y
    g.connect(c20, 0, ymul, 1);
    // time * 3
    g.connect(time, 0, tscale, 0);
    g.connect(c3, 0, tscale, 1);
    // y*20 + t*3
    g.connect(ymul, 0, yoff, 0);
    g.connect(tscale, 0, yoff, 1);
    // sin → * 0.025
    g.connect(yoff, 0, sinv, 0);
    g.connect(sinv, 0, amp, 0);
    g.connect(c025, 0, amp, 1);
    // new_x = x + amp
    g.connect(split, 0, newx, 0);   // x
    g.connect(amp, 0, newx, 1);
    // new_uv
    g.connect(newx, 0, newuv, 0);
    g.connect(split, 1, newuv, 1);  // y unchanged
    // scene sample
    g.connect(newuv, 0, scene, 0);
    g.connect(scene, 0, out, 0);
    g
}

/// 2. Rainbow: hue = time*60 + UV.x*180 → HSV(hue, 0.9, 0.9) → tint scene
fn build_rainbow() -> ShaderGraph {
    let mut g = ShaderGraph::new("rainbow");

    let uv      = g.add_node(1,  NodeKind::UV);
    let time    = g.add_node(2,  NodeKind::Time);
    let c60     = g.add_node(3,  NodeKind::ConstF32(60.0));
    let c180    = g.add_node(4,  NodeKind::ConstF32(180.0));
    let cs      = g.add_node(5,  NodeKind::ConstF32(0.85));  // saturation
    let cv      = g.add_node(6,  NodeKind::ConstF32(0.95));  // value
    let split   = g.add_node(7,  NodeKind::SplitVec2);
    let tscale  = g.add_node(8,  NodeKind::MulF);   // time * 60
    let xscale  = g.add_node(9,  NodeKind::MulF);   // x * 180
    let hue     = g.add_node(10, NodeKind::AddF);   // hue
    let tint    = g.add_node(11, NodeKind::HSVtoRGB);
    let scene   = g.add_node(12, NodeKind::SceneColor);
    let blend   = g.add_node(13, NodeKind::MulV4);  // scene * tint
    let out     = g.add_node(14, NodeKind::FragmentColor);

    g.connect(uv, 0, split, 0);
    g.connect(time, 0, tscale, 0); g.connect(c60, 0, tscale, 1);
    g.connect(split, 0, xscale, 0); g.connect(c180, 0, xscale, 1);
    g.connect(tscale, 0, hue, 0);  g.connect(xscale, 0, hue, 1);
    g.connect(hue, 0, tint, 0);
    g.connect(cs, 0, tint, 1);
    g.connect(cv, 0, tint, 2);
    g.connect(uv, 0, scene, 0);
    g.connect(scene, 0, blend, 0);
    g.connect(tint, 0, blend, 1);
    g.connect(blend, 0, out, 0);
    g
}

/// 3. Glitch: per-row horizontal offset driven by hash(row, time)
fn build_glitch() -> ShaderGraph {
    let mut g = ShaderGraph::new("glitch");

    let uv      = g.add_node(1,  NodeKind::UV);
    let time    = g.add_node(2,  NodeKind::Time);
    let c20     = g.add_node(3,  NodeKind::ConstF32(20.0));   // row count
    let c5      = g.add_node(4,  NodeKind::ConstF32(5.0));    // time speed
    let c005    = g.add_node(5,  NodeKind::ConstF32(0.04));   // displacement
    let split   = g.add_node(6,  NodeKind::SplitVec2);
    let ymul    = g.add_node(7,  NodeKind::MulF);             // y * 20
    let rowf    = g.add_node(8,  NodeKind::Floor);            // floor(y*20)
    let tscale  = g.add_node(9,  NodeKind::MulF);             // time * 5
    let noiseuv = g.add_node(10, NodeKind::CombineVec2);      // (row, t*5)
    let rnd     = g.add_node(11, NodeKind::Hash);             // random per row
    let disp    = g.add_node(12, NodeKind::MulF);             // rnd * 0.04
    let newx    = g.add_node(13, NodeKind::AddF);             // x + disp
    let newuv   = g.add_node(14, NodeKind::CombineVec2);
    let scene   = g.add_node(15, NodeKind::SceneColor);
    let out     = g.add_node(16, NodeKind::FragmentColor);

    g.connect(uv, 0, split, 0);
    g.connect(split, 1, ymul, 0); g.connect(c20, 0, ymul, 1);
    g.connect(ymul, 0, rowf, 0);
    g.connect(time, 0, tscale, 0); g.connect(c5, 0, tscale, 1);
    g.connect(rowf, 0, noiseuv, 0); g.connect(tscale, 0, noiseuv, 1);
    g.connect(noiseuv, 0, rnd, 0);
    g.connect(rnd, 0, disp, 0); g.connect(c005, 0, disp, 1);
    g.connect(split, 0, newx, 0); g.connect(disp, 0, newx, 1);
    g.connect(newx, 0, newuv, 0); g.connect(split, 1, newuv, 1);
    g.connect(newuv, 0, scene, 0);
    g.connect(scene, 0, out, 0);
    g
}

/// 4. Vignette+Noise: dark radial vignette blended with value noise
fn build_vignette_noise() -> ShaderGraph {
    let mut g = ShaderGraph::new("vignette_noise");

    let uv      = g.add_node(1,  NodeKind::UV);
    let time    = g.add_node(2,  NodeKind::Time);
    let c05     = g.add_node(3,  NodeKind::ConstF32(0.5));
    let c05b    = g.add_node(4,  NodeKind::ConstF32(0.5));
    let clo     = g.add_node(5,  NodeKind::ConstF32(0.2));    // smoothstep lo
    let chi     = g.add_node(6,  NodeKind::ConstF32(0.7));    // smoothstep hi
    let cns     = g.add_node(7,  NodeKind::ConstF32(5.0));    // noise scale
    let cnoise  = g.add_node(8,  NodeKind::ConstF32(0.15));   // noise blend
    let ct      = g.add_node(9,  NodeKind::ConstF32(0.05));   // time drift
    let split   = g.add_node(10, NodeKind::SplitVec2);
    let cx      = g.add_node(11, NodeKind::SubF);   // x - 0.5
    let cy      = g.add_node(12, NodeKind::SubF);   // y - 0.5
    let cv      = g.add_node(13, NodeKind::CombineVec2);
    let dist    = g.add_node(14, NodeKind::Length);
    let vign    = g.add_node(15, NodeKind::Smoothstep);  // vignette mask
    let inv     = g.add_node(16, NodeKind::SubF);        // 1.0 - vignette
    let tdrift  = g.add_node(17, NodeKind::MulF);        // time * 0.05
    let nuvx    = g.add_node(18, NodeKind::AddF);        // uv.x + tdrift
    let nuv     = g.add_node(19, NodeKind::CombineVec2);
    let noise   = g.add_node(20, NodeKind::ValueNoise);
    let nscale  = g.add_node(21, NodeKind::MulF);        // noise * 0.15
    let combined= g.add_node(22, NodeKind::AddF);        // inv + nscale
    let scene   = g.add_node(23, NodeKind::SceneColor);
    let lit     = g.add_node(24, NodeKind::MulFV4);      // scene * combined
    let out     = g.add_node(25, NodeKind::FragmentColor);

    g.connect(uv, 0, split, 0);
    // center offset
    g.connect(split, 0, cx, 0); g.connect(c05, 0, cx, 1);
    g.connect(split, 1, cy, 0); g.connect(c05b, 0, cy, 1);
    g.connect(cx, 0, cv, 0); g.connect(cy, 0, cv, 1);
    // distance → vignette
    g.connect(cv, 0, dist, 0);
    g.connect(clo, 0, vign, 0); g.connect(chi, 0, vign, 1); g.connect(dist, 0, vign, 2);
    g.connect(c05, 0, inv, 0); // reusing 0.5 as approx 1.0 — let's add a 1.0 const
    // redo: use 1.0 const for inv
    let c1 = g.add_node(26, NodeKind::ConstF32(1.0));
    g.connect(c1, 0, inv, 0); g.connect(vign, 0, inv, 1);  // 1 - vignette
    // noise drift
    g.connect(time, 0, tdrift, 0); g.connect(ct, 0, tdrift, 1);
    g.connect(split, 0, nuvx, 0); g.connect(tdrift, 0, nuvx, 1);
    g.connect(nuvx, 0, nuv, 0); g.connect(split, 1, nuv, 1);
    g.connect(nuv, 0, noise, 0); g.connect(cns, 0, noise, 1);
    g.connect(noise, 0, nscale, 0); g.connect(cnoise, 0, nscale, 1);
    // combine vignette + noise
    g.connect(inv, 0, combined, 0); g.connect(nscale, 0, combined, 1);
    // apply
    g.connect(uv, 0, scene, 0);
    g.connect(combined, 0, lit, 0); g.connect(scene, 0, lit, 1);
    g.connect(lit, 0, out, 0);
    g
}

// ── Demo state ────────────────────────────────────────────────────────────────

struct ShaderEditorDemo {
    tex_white:  Option<TextureHandle>,
    tex_circle: Option<TextureHandle>,
    font:       Option<FontHandle>,

    graphs:   Vec<ShaderGraph>,
    pipelines: Vec<Option<Arc<CustomShaderPipeline>>>,
    active:   usize,
    enabled:  bool,
    time:     f32,
}

impl ShaderEditorDemo {
    fn new() -> Self {
        let graphs = vec![
            build_ripple(),
            build_rainbow(),
            build_glitch(),
            build_vignette_noise(),
        ];
        let len = graphs.len();
        Self {
            tex_white: None, tex_circle: None, font: None,
            graphs,
            pipelines: vec![None; len],
            active: 0,
            enabled: true,
            time: 0.0,
        }
    }
}

fn pack(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

fn make_circle_tex(ctx: &mut GameContext, size: u32) -> TextureHandle {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let (cx, cy, rad) = (size as f32 / 2.0, size as f32 / 2.0, size as f32 / 2.0 - 1.0);
    for py in 0..size { for px in 0..size {
        let d = (((px as f32 - cx).powi(2) + (py as f32 - cy).powi(2)).sqrt());
        let a = ((rad - d).clamp(0.0, 1.0) * 255.0) as u8;
        let i = ((py * size + px) * 4) as usize;
        data[i] = 255; data[i+1] = 255; data[i+2] = 255; data[i+3] = a;
    }}
    ctx.create_texture_from_rgba(&data, size, size)
}

impl Game for ShaderEditorDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        ctx.camera.zoom  = 2.0;
        self.tex_white   = Some(ctx.create_texture_from_rgba(&[255, 255, 255, 255], 1, 1));
        self.tex_circle  = Some(make_circle_tex(ctx, 64));
        self.font        = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 8.0));

        // Compile all shader graphs at init
        for (i, graph) in self.graphs.iter().enumerate() {
            self.pipelines[i] = ctx.compile_shader_graph(graph);
            match &self.pipelines[i] {
                Some(_) => log::info!("Shader '{}' compiled OK", graph.name),
                None    => log::error!("Shader '{}' failed to compile", graph.name),
            }
        }
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;
        self.time += dt;

        if !ctx.first_tick { return; }
        if ctx.input.is_key_just_pressed(Key::Space) { self.enabled = !self.enabled; }
        if ctx.input.is_key_just_pressed(Key::Digit1) { self.active = 0; }
        if ctx.input.is_key_just_pressed(Key::Digit2) { self.active = 1; }
        if ctx.input.is_key_just_pressed(Key::Digit3) { self.active = 2; }
        if ctx.input.is_key_just_pressed(Key::Digit4) { self.active = 3; }
        if ctx.input.is_key_just_pressed(Key::KeyG) {
            println!("=== Graph: {} ===\n{}", self.graphs[self.active].name,
                     self.graphs[self.active].to_json());
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let (Some(tw), Some(tc), Some(font)) =
            (self.tex_white, self.tex_circle, self.font) else { return };
        let t = self.time;

        // ── Scene (rich background so effects are clearly visible) ────────────

        // Coloured background tiles
        let colors: &[(u8, u8, u8)] = &[
            (30, 20, 60), (50, 30, 80), (20, 40, 70), (40, 25, 65), (25, 45, 75),
            (60, 25, 45), (45, 40, 30), (25, 55, 40), (55, 35, 25), (35, 50, 35),
            (20, 55, 55), (40, 40, 60), (55, 20, 50), (30, 50, 25), (50, 45, 20),
            (40, 30, 55), (25, 45, 50), (50, 25, 35), (30, 55, 30), (45, 35, 50),
        ];
        let (tw_px, th_px) = (96.0_f32, 80.0_f32);
        for (i, &(r, g, b)) in colors.iter().enumerate() {
            let col = (i % 5) as f32;
            let row = (i / 5) as f32;
            ctx.draw_sprite(Sprite {
                texture: tw,
                position: Vec2::new(-240.0 + col * tw_px + tw_px / 2.0,
                                     160.0 - row * th_px - th_px / 2.0),
                size: Vec2::new(tw_px - 2.0, th_px - 2.0),
                rotation: 0.0, color: pack(r, g, b, 255),
                layer: -5, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // Coloured moving objects (give the shaders something interesting to distort)
        let objects: &[(f32, f32, f32, f32, u8, u8, u8)] = &[
            (-150.0,  70.0, 45.0, 45.0, 220,  80,  40),
            (  60.0,  70.0, 35.0, 60.0,  50, 140, 220),
            ( 150.0, -30.0, 55.0, 35.0,  60, 200,  80),
            ( -70.0, -70.0, 40.0, 40.0, 220, 180,  50),
            (  10.0, -100.0, 50.0, 28.0, 180, 60, 200),
            (-200.0, -20.0, 28.0, 65.0, 50, 180, 180),
            ( 110.0, 110.0, 40.0, 40.0, 240, 120, 70),
        ];
        for &(x, y, w, h, r, g, b) in objects {
            ctx.draw_sprite(Sprite {
                texture: tw, position: Vec2::new(x, y), size: Vec2::new(w, h),
                rotation: 0.0, color: pack(r, g, b, 255),
                layer: 0, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // Orbiting glow dots
        for i in 0..3_u32 {
            let angle = t * (0.7 + i as f32 * 0.4) + i as f32 * 2.1;
            let r = 80.0 + i as f32 * 20.0;
            let pos = Vec2::new(angle.cos() * r, angle.sin() * r * 0.6);
            let (lr, lg, lb): (u8, u8, u8) = [(255, 180, 80), (80, 180, 255), (180, 255, 100)][i as usize];
            ctx.draw_sprite(Sprite {
                texture: tc, position: pos, size: Vec2::splat(14.0),
                rotation: 0.0, color: pack(lr, lg, lb, 220),
                layer: 2, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // ── Post-processing: active shader ────────────────────────────────────
        ctx.post_processing.enabled = self.enabled;
        ctx.post_processing.effects.clear();
        if self.enabled {
            if let Some(pipeline) = &self.pipelines[self.active] {
                ctx.post_processing.effects.push(
                    PostEffect::Custom(Arc::clone(pipeline))
                );
            }
        }

        // ── HUD ──────────────────────────────────────────────────────────────
        let on  = pack(100, 255, 100, 255);
        let off = pack(120, 120, 120, 255);
        let lbl = pack(200, 200, 200, 255);
        let sel = pack(255, 220, 80, 255);

        let names = ["Ripple", "Rainbow", "Glitch ", "Vignette+Noise"];

        let mut y = -150.0_f32;
        ctx.draw_text("[Spc] Shader", Vec2::new(-230.0, y), font, 8.0, lbl, 10);
        ctx.draw_text(if self.enabled {"ON"} else {"off"}, Vec2::new(-50.0, y), font, 8.0,
            if self.enabled { on } else { off }, 10);
        y += 18.0;

        for (i, name) in names.iter().enumerate() {
            let label = format!("[{}] {}", i + 1, name);
            let color = if i == self.active { sel } else { lbl };
            ctx.draw_text(&label, Vec2::new(-230.0, y), font, 8.0, color, 10);
            y += 14.0;
        }
        y += 8.0;
        ctx.draw_text("[G] print graph JSON", Vec2::new(-230.0, y), font, 8.0, lbl, 10);
    }
}

fn main() {
    App::new()
        .with_title("Toile v0.4 — Shader Graph  [1-4]effect [Space]toggle [G]print-JSON")
        .with_size(1280, 720)
        .with_clear_color(toile_app::core::color::Color::new(0.05, 0.03, 0.08, 1.0))
        .run(ShaderEditorDemo::new());
}
