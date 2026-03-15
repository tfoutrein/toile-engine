//! Toile Engine — Aseprite Binary Import Demo (v0.3)
//!
//! Demonstrates direct import of Aseprite binary (.ase) files.
//! Generates a sample .ase file with a 4-frame animated sprite
//! (using the binary format), parses it, builds an atlas, and
//! renders the animation with tag-based clips.
//!
//! Controls:
//!   1/2/3: switch animation clip (idle, blink, wave)
//!   Space: pause/resume
//!
//! Run with: `cargo run --example aseprite_demo`

use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, TextureHandle, COLOR_WHITE};
use toile_assets::aseprite::{build_atlas, parse_ase, AseFile};
use toile_assets::animation::{AnimationClip, AnimationFrame, PlaybackMode};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};

// ── Build a synthetic .ase file ──────────────────────────────

const FILE_MAGIC: u16 = 0xA5E0;
const FRAME_MAGIC: u16 = 0xF1FA;
const CHUNK_LAYER: u16 = 0x2004;
const CHUNK_CEL: u16 = 0x2005;
const CHUNK_TAGS: u16 = 0x2018;

fn build_demo_ase() -> Vec<u8> {
    let w: u16 = 16;
    let h: u16 = 16;
    let color_depth: u16 = 32;

    // 6 frames: idle(0-1), blink(2-3), wave(4-5)
    let frame_colors: Vec<(u16, [u8; 4])> = vec![
        (200, [80, 150, 230, 255]),   // idle 1: blue
        (200, [70, 130, 210, 255]),   // idle 2: darker blue
        (100, [80, 150, 230, 255]),   // blink 1: blue (eyes open)
        (100, [80, 150, 230, 200]),   // blink 2: semi-transparent (eyes closed)
        (150, [230, 180, 80, 255]),   // wave 1: gold
        (150, [255, 200, 100, 255]),  // wave 2: bright gold
    ];

    let tags: Vec<(&str, u16, u16, u8)> = vec![
        ("idle", 0, 1, 0),    // forward
        ("blink", 2, 3, 2),   // pingpong
        ("wave", 4, 5, 0),    // forward
    ];

    // Build pixel data for each frame
    let pixels_per_frame: Vec<Vec<u8>> = frame_colors
        .iter()
        .enumerate()
        .map(|(fi, (_, color))| {
            let mut px = vec![0u8; (w as usize * h as usize) * 4];
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let idx = (y * w as usize + x) * 4;
                    // Draw a character shape
                    let cx = x as f32 - 7.5;
                    let cy = y as f32 - 7.5;
                    let in_body = cx * cx + cy * cy < 36.0; // circle radius 6
                    let in_eye_l = (cx + 2.5).abs() < 1.5 && (cy + 2.0).abs() < 1.5;
                    let in_eye_r = (cx - 2.5).abs() < 1.5 && (cy + 2.0).abs() < 1.5;

                    if in_body {
                        px[idx] = color[0];
                        px[idx + 1] = color[1];
                        px[idx + 2] = color[2];
                        px[idx + 3] = color[3];

                        // Eyes (white with black pupils)
                        if in_eye_l || in_eye_r {
                            if fi == 3 {
                                // blink frame: closed eyes (line)
                                if (cy + 2.0).abs() < 0.5 {
                                    px[idx] = 30; px[idx + 1] = 30; px[idx + 2] = 30; px[idx + 3] = 255;
                                }
                            } else {
                                px[idx] = 255; px[idx + 1] = 255; px[idx + 2] = 255; px[idx + 3] = 255;
                            }
                        }

                        // Mouth (smile)
                        if cy > 1.5 && cy < 3.0 && cx.abs() < 3.0 {
                            px[idx] = 40; px[idx + 1] = 40; px[idx + 2] = 40; px[idx + 3] = 255;
                        }
                    }

                    // Wave frames: add arm
                    if fi >= 4 {
                        let arm_angle = if fi == 4 { 0.3 } else { -0.3 };
                        let ax = cx - 6.0;
                        let ay = cy - arm_angle * 5.0;
                        if ax > 0.0 && ax < 5.0 && ay.abs() < 1.5 {
                            px[idx] = color[0];
                            px[idx + 1] = color[1];
                            px[idx + 2] = color[2];
                            px[idx + 3] = 255;
                        }
                    }
                }
            }
            px
        })
        .collect();

    // Assemble .ase binary
    let mut frame_datas = Vec::new();
    for (i, (duration, _)) in frame_colors.iter().enumerate() {
        let mut chunks = Vec::new();

        if i == 0 {
            // Layer chunk
            let mut lc = Vec::new();
            lc.extend_from_slice(&1u16.to_le_bytes()); // visible
            lc.extend_from_slice(&0u16.to_le_bytes()); // image
            lc.extend_from_slice(&0u16.to_le_bytes()); // child level
            lc.extend_from_slice(&0u16.to_le_bytes()); // default w
            lc.extend_from_slice(&0u16.to_le_bytes()); // default h
            lc.extend_from_slice(&0u16.to_le_bytes()); // blend
            lc.push(255); // opacity
            lc.extend_from_slice(&[0u8; 3]);
            let name = b"Main";
            lc.extend_from_slice(&(name.len() as u16).to_le_bytes());
            lc.extend_from_slice(name);
            let sz = (6 + lc.len()) as u32;
            chunks.extend_from_slice(&sz.to_le_bytes());
            chunks.extend_from_slice(&CHUNK_LAYER.to_le_bytes());
            chunks.extend_from_slice(&lc);

            // Tags chunk
            let mut tc = Vec::new();
            tc.extend_from_slice(&(tags.len() as u16).to_le_bytes());
            tc.extend_from_slice(&[0u8; 8]);
            for (name, from, to, dir) in &tags {
                tc.extend_from_slice(&from.to_le_bytes());
                tc.extend_from_slice(&to.to_le_bytes());
                tc.push(*dir);
                tc.extend_from_slice(&0u16.to_le_bytes()); // repeat
                tc.extend_from_slice(&[0u8; 6]);
                tc.extend_from_slice(&[0u8; 3]); // rgb
                tc.push(0);
                let nb = name.as_bytes();
                tc.extend_from_slice(&(nb.len() as u16).to_le_bytes());
                tc.extend_from_slice(nb);
            }
            let sz = (6 + tc.len()) as u32;
            chunks.extend_from_slice(&sz.to_le_bytes());
            chunks.extend_from_slice(&CHUNK_TAGS.to_le_bytes());
            chunks.extend_from_slice(&tc);
        }

        // Cel chunk
        let mut cc = Vec::new();
        cc.extend_from_slice(&0u16.to_le_bytes()); // layer
        cc.extend_from_slice(&0i16.to_le_bytes()); // x
        cc.extend_from_slice(&0i16.to_le_bytes()); // y
        cc.push(255); // opacity
        cc.extend_from_slice(&0u16.to_le_bytes()); // raw
        cc.extend_from_slice(&0i16.to_le_bytes()); // z
        cc.extend_from_slice(&[0u8; 5]);
        cc.extend_from_slice(&w.to_le_bytes());
        cc.extend_from_slice(&h.to_le_bytes());
        cc.extend_from_slice(&pixels_per_frame[i]);
        let sz = (6 + cc.len()) as u32;
        chunks.extend_from_slice(&sz.to_le_bytes());
        chunks.extend_from_slice(&CHUNK_CEL.to_le_bytes());
        chunks.extend_from_slice(&cc);

        let chunk_count = if i == 0 { 3u16 } else { 1u16 };
        let frame_size = (16 + chunks.len()) as u32;
        let mut frame = Vec::new();
        frame.extend_from_slice(&frame_size.to_le_bytes());
        frame.extend_from_slice(&FRAME_MAGIC.to_le_bytes());
        frame.extend_from_slice(&chunk_count.to_le_bytes());
        frame.extend_from_slice(&duration.to_le_bytes());
        frame.extend_from_slice(&[0u8; 2]);
        frame.extend_from_slice(&0u32.to_le_bytes());
        frame.extend_from_slice(&chunks);
        frame_datas.push(frame);
    }

    let total: usize = frame_datas.iter().map(|f| f.len()).sum();
    let file_size = (128 + total) as u32;
    let frame_count = frame_colors.len() as u16;

    let mut file = Vec::new();
    file.extend_from_slice(&file_size.to_le_bytes());
    file.extend_from_slice(&FILE_MAGIC.to_le_bytes());
    file.extend_from_slice(&frame_count.to_le_bytes());
    file.extend_from_slice(&w.to_le_bytes());
    file.extend_from_slice(&h.to_le_bytes());
    file.extend_from_slice(&color_depth.to_le_bytes());
    file.extend_from_slice(&0u32.to_le_bytes()); // flags
    file.extend_from_slice(&100u16.to_le_bytes()); // speed
    file.extend_from_slice(&[0u8; 8]);
    file.push(0); // transparent
    file.extend_from_slice(&[0u8; 3]);
    file.extend_from_slice(&0u16.to_le_bytes()); // colors
    file.push(1); file.push(1); // pixel ratio
    file.extend_from_slice(&[0u8; 8]); // grid
    file.extend_from_slice(&[0u8; 84]); // future
    for f in &frame_datas { file.extend_from_slice(f); }
    file
}

// ── Demo game ────────────────────────────────────────────────

struct AsepriteDemo {
    atlas_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    ase: Option<AseFile>,
    atlas_w: u32,
    atlas_h: u32,
    durations: Vec<u16>,
    clips: Vec<(String, usize, usize, PlaybackMode)>, // (name, from, to, mode)
    current_clip: usize,
    frame_time: f32,
    current_frame: usize,
    paused: bool,
    direction: i32, // 1 or -1 for pingpong
}

impl AsepriteDemo {
    fn clip_range(&self) -> (usize, usize) {
        if self.clips.is_empty() {
            (0, self.durations.len().saturating_sub(1))
        } else {
            let c = &self.clips[self.current_clip];
            (c.1, c.2)
        }
    }
}

impl Game for AsepriteDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));

        // Build and parse the .ase
        let ase_data = build_demo_ase();
        let ase = parse_ase(&ase_data).expect("Failed to parse demo .ase");

        let (atlas, atlas_w, atlas_h, durations) = build_atlas(&ase);

        // Create texture from atlas RGBA data
        self.atlas_tex = Some(ctx.create_texture_from_rgba(&atlas, atlas_w, atlas_h));
        self.atlas_w = atlas_w;
        self.atlas_h = atlas_h;
        self.durations = durations;

        // Build clip info from tags
        self.clips = ase.tags.iter().map(|t| {
            let mode = match t.direction {
                2 | 3 => PlaybackMode::PingPong,
                _ => PlaybackMode::Loop,
            };
            (t.name.clone(), t.from as usize, t.to as usize, mode)
        }).collect();

        self.ase = Some(ase);
        self.current_clip = 0;
        self.current_frame = 0;
        self.frame_time = 0.0;
        self.direction = 1;

        log::info!(
            "Aseprite Demo! {} frames, {} tags. 1/2/3=clip, Space=pause",
            self.durations.len(),
            self.clips.len()
        );
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;

        // Switch clip
        if ctx.input.is_key_just_pressed(Key::Digit1) && self.clips.len() > 0 {
            self.current_clip = 0;
            let (from, _) = self.clip_range();
            self.current_frame = from;
            self.frame_time = 0.0;
            self.direction = 1;
        }
        if ctx.input.is_key_just_pressed(Key::Digit2) && self.clips.len() > 1 {
            self.current_clip = 1;
            let (from, _) = self.clip_range();
            self.current_frame = from;
            self.frame_time = 0.0;
            self.direction = 1;
        }
        if ctx.input.is_key_just_pressed(Key::Digit3) && self.clips.len() > 2 {
            self.current_clip = 2;
            let (from, _) = self.clip_range();
            self.current_frame = from;
            self.frame_time = 0.0;
            self.direction = 1;
        }

        if ctx.input.is_key_just_pressed(Key::Space) {
            self.paused = !self.paused;
        }

        if self.paused || self.durations.is_empty() { return; }

        // Advance animation
        let dur = self.durations[self.current_frame] as f32 / 1000.0;
        self.frame_time += dt;

        if self.frame_time >= dur {
            self.frame_time -= dur;
            let (from, to) = self.clip_range();
            let mode = if !self.clips.is_empty() { self.clips[self.current_clip].3 } else { PlaybackMode::Loop };

            match mode {
                PlaybackMode::Loop => {
                    self.current_frame += 1;
                    if self.current_frame > to {
                        self.current_frame = from;
                    }
                }
                PlaybackMode::PingPong => {
                    let next = self.current_frame as i32 + self.direction;
                    if next > to as i32 {
                        self.direction = -1;
                        self.current_frame = to.saturating_sub(1).max(from);
                    } else if next < from as i32 {
                        self.direction = 1;
                        self.current_frame = from + 1;
                        if self.current_frame > to { self.current_frame = from; }
                    } else {
                        self.current_frame = next as usize;
                    }
                }
                PlaybackMode::Once => {
                    if self.current_frame < to {
                        self.current_frame += 1;
                    }
                }
            }
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.atlas_tex {
            Some(t) => t,
            None => return,
        };

        let ase = match &self.ase {
            Some(a) => a,
            None => return,
        };

        let fw = ase.width as f32;
        let fh = ase.height as f32;
        let aw = self.atlas_w as f32;
        let ah = self.atlas_h as f32;

        // Draw current frame (scaled up 8x)
        let scale = 8.0;
        let uv_x = self.current_frame as f32 * fw / aw;
        ctx.draw_sprite(DrawSprite {
            texture: tex,
            position: Vec2::new(0.0, 40.0),
            size: Vec2::new(fw * scale, fh * scale),
            rotation: 0.0,
            color: COLOR_WHITE,
            layer: 0,
            uv_min: Vec2::new(uv_x, 0.0),
            uv_max: Vec2::new(uv_x + fw / aw, fh / ah),
        });

        // Draw all frames as filmstrip at the bottom
        let strip_scale = 4.0;
        let strip_y = -120.0;
        let start_x = -(self.durations.len() as f32 * fw * strip_scale) / 2.0 + fw * strip_scale / 2.0;

        for i in 0..self.durations.len() {
            let x = start_x + i as f32 * (fw * strip_scale + 4.0);
            let uv_x = i as f32 * fw / aw;

            let color = if i == self.current_frame {
                COLOR_WHITE
            } else {
                pack_color(120, 120, 120, 255)
            };

            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: Vec2::new(x, strip_y),
                size: Vec2::new(fw * strip_scale, fh * strip_scale),
                rotation: 0.0,
                color,
                layer: 0,
                uv_min: Vec2::new(uv_x, 0.0),
                uv_max: Vec2::new(uv_x + fw / aw, fh / ah),
            });

            // Highlight current frame
            if i == self.current_frame {
                if let Some(white_tex) = self.atlas_tex {
                    // Draw selection border
                    let bs = fw * strip_scale + 6.0;
                    ctx.draw_sprite(DrawSprite {
                        texture: tex,
                        position: Vec2::new(x, strip_y),
                        size: Vec2::new(bs, bs),
                        rotation: 0.0,
                        color: pack_color(255, 220, 50, 100),
                        layer: -1,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::new(1.0 / aw, 1.0 / ah), // tiny white pixel
                    });
                }
            }
        }

        // HUD
        if let Some(font) = self.font {
            let tl = ctx.camera.top_left();
            let clip_name = if !self.clips.is_empty() {
                &self.clips[self.current_clip].0
            } else {
                "default"
            };
            let paused = if self.paused { " [PAUSED]" } else { "" };

            ctx.draw_text(
                &format!("Aseprite Binary Import | Clip: {} | Frame: {}/{}{}",
                    clip_name, self.current_frame + 1, self.durations.len(), paused),
                Vec2::new(tl.x + 10.0, tl.y - 20.0),
                font, 9.0, COLOR_WHITE, 50,
            );
            ctx.draw_text(
                "1:idle 2:blink 3:wave | Space=Pause | 16x16 sprite, 6 frames",
                Vec2::new(tl.x + 10.0, tl.y - 38.0),
                font, 5.5, pack_color(150, 150, 170, 255), 50,
            );

            let info = format!(
                "Format: .ase binary | {}x{} | {}bpp | {} layers | {} tags",
                ase.width, ase.height, ase.color_depth,
                ase.layers.len(), ase.tags.len()
            );
            ctx.draw_text(
                &info,
                Vec2::new(tl.x + 10.0, tl.y - 52.0),
                font, 5.0, pack_color(100, 200, 100, 255), 50,
            );
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Aseprite Binary Import Demo (v0.3)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.08, 0.08, 0.12))
        .run(AsepriteDemo {
            atlas_tex: None,
            font: None,
            ase: None,
            atlas_w: 0,
            atlas_h: 0,
            durations: Vec::new(),
            clips: Vec::new(),
            current_clip: 0,
            frame_time: 0.0,
            current_frame: 0,
            paused: false,
            direction: 1,
        });
}
