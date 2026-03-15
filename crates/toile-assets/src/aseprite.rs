//! Aseprite binary (.ase / .aseprite) parser.
//!
//! Parses the Aseprite binary format directly, extracting frames, tags,
//! layers, and palette data. Produces an RGBA atlas image that can be
//! turned into a texture, plus animation clip definitions matching the
//! existing `SpriteSheet`/`AnimationClip` types from `animation.rs`.

use std::collections::HashMap;
use std::io::{Cursor, Read as _};
use std::path::Path;

use flate2::read::ZlibDecoder;
use glam::Vec2;

use crate::animation::{AnimationClip, AnimationFrame, PlaybackMode, SpriteSheet};
use toile_graphics::texture::TextureHandle;

// ── Magic numbers ────────────────────────────────────────────

const FILE_MAGIC: u16 = 0xA5E0;
const FRAME_MAGIC: u16 = 0xF1FA;
const CHUNK_LAYER: u16 = 0x2004;
const CHUNK_CEL: u16 = 0x2005;
const CHUNK_TAGS: u16 = 0x2018;
const CHUNK_PALETTE: u16 = 0x2019;

// ── Reader helpers ───────────────────────────────────────────

struct BinReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn skip(&mut self, n: usize) {
        self.pos += n;
    }

    fn byte(&mut self) -> u8 {
        let v = self.data[self.pos];
        self.pos += 1;
        v
    }

    fn word(&mut self) -> u16 {
        let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        v
    }

    fn short(&mut self) -> i16 {
        let v = i16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        v
    }

    fn dword(&mut self) -> u32 {
        let v = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        v
    }

    fn bytes(&mut self, n: usize) -> &'a [u8] {
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        slice
    }

    fn string(&mut self) -> String {
        let len = self.word() as usize;
        let s = std::str::from_utf8(&self.data[self.pos..self.pos + len])
            .unwrap_or("")
            .to_string();
        self.pos += len;
        s
    }
}

// ── Parsed data ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AseLayer {
    pub name: String,
    pub visible: bool,
    pub opacity: u8,
    pub layer_type: u16, // 0=image, 1=group, 2=tilemap
}

#[derive(Debug, Clone)]
pub struct AseTag {
    pub name: String,
    pub from: u16,
    pub to: u16,
    pub direction: u8, // 0=forward, 1=reverse, 2=pingpong
}

#[derive(Debug, Clone)]
pub struct AseFrameData {
    pub duration_ms: u16,
    pub cels: Vec<AseCel>,
}

#[derive(Debug, Clone)]
pub struct AseCel {
    pub layer_index: u16,
    pub x: i16,
    pub y: i16,
    pub opacity: u8,
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>, // RGBA, row-major
}

/// Complete parsed Aseprite file.
pub struct AseFile {
    pub width: u16,
    pub height: u16,
    pub color_depth: u16,
    pub transparent_index: u8,
    pub layers: Vec<AseLayer>,
    pub frames: Vec<AseFrameData>,
    pub tags: Vec<AseTag>,
    pub palette: Vec<[u8; 4]>, // RGBA entries
}

// ── Parser ───────────────────────────────────────────────────

/// Parse an Aseprite binary file from raw bytes.
pub fn parse_ase(data: &[u8]) -> Result<AseFile, String> {
    let mut r = BinReader::new(data);

    // File header (128 bytes)
    let _file_size = r.dword();
    let magic = r.word();
    if magic != FILE_MAGIC {
        return Err(format!("Not an Aseprite file (magic: {:#06x})", magic));
    }

    let frame_count = r.word();
    let width = r.word();
    let height = r.word();
    let color_depth = r.word();
    let _flags = r.dword();
    let _speed = r.word(); // deprecated
    r.skip(8); // reserved
    let transparent_index = r.byte();
    r.skip(3); // ignore
    let color_count = r.word();
    let _pixel_w = r.byte();
    let _pixel_h = r.byte();
    r.skip(2 + 2 + 2 + 2); // grid x, y, w, h
    r.skip(84); // future

    let mut layers = Vec::new();
    let mut tags = Vec::new();
    let mut palette = vec![[0u8; 4]; if color_count == 0 { 256 } else { color_count as usize }];
    let mut frames = Vec::new();

    // Parse frames
    for _frame_idx in 0..frame_count {
        let frame_start = r.pos;
        let frame_bytes = r.dword() as usize;
        let frame_magic = r.word();
        if frame_magic != FRAME_MAGIC {
            return Err(format!("Bad frame magic: {:#06x}", frame_magic));
        }

        let old_chunks = r.word();
        let duration = r.word();
        r.skip(2); // future
        let new_chunks = r.dword();
        let chunk_count = if new_chunks == 0 { old_chunks as u32 } else { new_chunks };

        let mut frame_cels = Vec::new();

        for _chunk_idx in 0..chunk_count {
            let chunk_start = r.pos;
            let chunk_size = r.dword() as usize;
            let chunk_type = r.word();
            let chunk_data_end = chunk_start + chunk_size;

            match chunk_type {
                CHUNK_LAYER => {
                    let flags = r.word();
                    let layer_type = r.word();
                    let _child_level = r.word();
                    let _default_w = r.word();
                    let _default_h = r.word();
                    let _blend_mode = r.word();
                    let opacity = r.byte();
                    r.skip(3); // future
                    let name = r.string();

                    layers.push(AseLayer {
                        name,
                        visible: flags & 1 != 0,
                        opacity,
                        layer_type,
                    });
                }

                CHUNK_CEL => {
                    let layer_index = r.word();
                    let x = r.short();
                    let y = r.short();
                    let opacity = r.byte();
                    let cel_type = r.word();
                    let _z_index = r.short();
                    r.skip(5); // future

                    match cel_type {
                        0 => {
                            // Raw image
                            let w = r.word();
                            let h = r.word();
                            let pixel_bytes = decode_pixels(
                                r.bytes((chunk_data_end - r.pos).min(r.remaining())),
                                w, h, color_depth, transparent_index, &palette,
                            );
                            frame_cels.push(AseCel {
                                layer_index, x, y, opacity,
                                width: w, height: h,
                                pixels: pixel_bytes,
                            });
                        }
                        2 => {
                            // Compressed image
                            let w = r.word();
                            let h = r.word();
                            let compressed = r.bytes((chunk_data_end - r.pos).min(r.remaining()));
                            let mut decoder = ZlibDecoder::new(Cursor::new(compressed));
                            let mut raw = Vec::new();
                            decoder.read_to_end(&mut raw).map_err(|e| format!("Zlib: {e}"))?;
                            let pixel_bytes = decode_pixels(
                                &raw, w, h, color_depth, transparent_index, &palette,
                            );
                            frame_cels.push(AseCel {
                                layer_index, x, y, opacity,
                                width: w, height: h,
                                pixels: pixel_bytes,
                            });
                        }
                        1 => {
                            // Linked cel — reference a previous frame
                            let _linked_frame = r.word();
                            // TODO: resolve linked cels
                        }
                        _ => {} // tilemap cels etc.
                    }
                }

                CHUNK_TAGS => {
                    let tag_count = r.word();
                    r.skip(8); // future

                    for _ in 0..tag_count {
                        let from = r.word();
                        let to = r.word();
                        let direction = r.byte();
                        let _repeat = r.word();
                        r.skip(6); // future
                        r.skip(3); // deprecated RGB
                        r.skip(1); // extra
                        let name = r.string();

                        tags.push(AseTag { name, from, to, direction });
                    }
                }

                CHUNK_PALETTE => {
                    let _total = r.dword();
                    let first = r.dword();
                    let last = r.dword();
                    r.skip(8); // future

                    for i in first..=last {
                        let flags = r.word();
                        let red = r.byte();
                        let green = r.byte();
                        let blue = r.byte();
                        let alpha = r.byte();
                        if (i as usize) < palette.len() {
                            palette[i as usize] = [red, green, blue, alpha];
                        }
                        if flags & 1 != 0 {
                            r.string(); // color name
                        }
                    }
                }

                _ => {}
            }

            r.pos = chunk_data_end;
        }

        frames.push(AseFrameData {
            duration_ms: duration,
            cels: frame_cels,
        });

        r.pos = frame_start + frame_bytes;
    }

    Ok(AseFile {
        width,
        height,
        color_depth,
        transparent_index,
        layers,
        frames,
        tags,
        palette,
    })
}

/// Decode raw or decompressed pixel data to RGBA based on color depth.
fn decode_pixels(
    raw: &[u8],
    w: u16,
    h: u16,
    color_depth: u16,
    transparent_index: u8,
    palette: &[[u8; 4]],
) -> Vec<u8> {
    let total = (w as usize) * (h as usize);
    let mut rgba = vec![0u8; total * 4];

    match color_depth {
        32 => {
            // RGBA — direct copy
            let n = (total * 4).min(raw.len());
            rgba[..n].copy_from_slice(&raw[..n]);
        }
        16 => {
            // Grayscale+Alpha (2 bytes per pixel)
            for i in 0..total {
                let offset = i * 2;
                if offset + 1 < raw.len() {
                    let v = raw[offset];
                    let a = raw[offset + 1];
                    rgba[i * 4] = v;
                    rgba[i * 4 + 1] = v;
                    rgba[i * 4 + 2] = v;
                    rgba[i * 4 + 3] = a;
                }
            }
        }
        8 => {
            // Indexed (1 byte per pixel)
            for i in 0..total {
                if i < raw.len() {
                    let idx = raw[i];
                    if idx == transparent_index {
                        // transparent
                    } else if (idx as usize) < palette.len() {
                        let c = palette[idx as usize];
                        rgba[i * 4] = c[0];
                        rgba[i * 4 + 1] = c[1];
                        rgba[i * 4 + 2] = c[2];
                        rgba[i * 4 + 3] = c[3];
                    }
                }
            }
        }
        _ => {}
    }

    rgba
}

// ── Atlas builder ────────────────────────────────────────────

/// Compose all frames into a horizontal sprite-strip atlas (RGBA).
/// Returns (atlas_rgba, atlas_width, atlas_height, frame_durations_ms).
pub fn build_atlas(ase: &AseFile) -> (Vec<u8>, u32, u32, Vec<u16>) {
    let fw = ase.width as u32;
    let fh = ase.height as u32;
    let frame_count = ase.frames.len() as u32;
    let atlas_w = fw * frame_count;
    let atlas_h = fh;

    let mut atlas = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    let mut durations = Vec::new();

    for (frame_idx, frame) in ase.frames.iter().enumerate() {
        durations.push(frame.duration_ms);

        // Compose cels in layer order (bottom to top)
        let mut sorted_cels: Vec<&AseCel> = frame.cels.iter().collect();
        sorted_cels.sort_by_key(|c| c.layer_index);

        for cel in sorted_cels {
            // Check layer visibility
            if let Some(layer) = ase.layers.get(cel.layer_index as usize) {
                if !layer.visible || layer.layer_type != 0 {
                    continue;
                }
            }

            let cel_opacity = cel.opacity as u32;
            let layer_opacity = ase
                .layers
                .get(cel.layer_index as usize)
                .map(|l| l.opacity as u32)
                .unwrap_or(255);

            for cy in 0..cel.height as i32 {
                for cx in 0..cel.width as i32 {
                    let dst_x = cel.x as i32 + cx;
                    let dst_y = cel.y as i32 + cy;

                    if dst_x < 0 || dst_y < 0 || dst_x >= fw as i32 || dst_y >= fh as i32 {
                        continue;
                    }

                    let src_idx = (cy as usize * cel.width as usize + cx as usize) * 4;
                    if src_idx + 3 >= cel.pixels.len() {
                        continue;
                    }

                    let sr = cel.pixels[src_idx] as u32;
                    let sg = cel.pixels[src_idx + 1] as u32;
                    let sb = cel.pixels[src_idx + 2] as u32;
                    let sa = cel.pixels[src_idx + 3] as u32;

                    // Apply cel + layer opacity
                    let a = (sa * cel_opacity / 255 * layer_opacity / 255) as u8;
                    if a == 0 {
                        continue;
                    }

                    let atlas_x = frame_idx as i32 * fw as i32 + dst_x;
                    let atlas_idx = (dst_y as usize * atlas_w as usize + atlas_x as usize) * 4;

                    if a == 255 {
                        atlas[atlas_idx] = sr as u8;
                        atlas[atlas_idx + 1] = sg as u8;
                        atlas[atlas_idx + 2] = sb as u8;
                        atlas[atlas_idx + 3] = 255;
                    } else {
                        // Alpha blend over existing
                        let da = atlas[atlas_idx + 3] as u32;
                        let a32 = a as u32;
                        let inv_a = 255 - a32;
                        atlas[atlas_idx] = ((sr * a32 + atlas[atlas_idx] as u32 * inv_a) / 255) as u8;
                        atlas[atlas_idx + 1] = ((sg * a32 + atlas[atlas_idx + 1] as u32 * inv_a) / 255) as u8;
                        atlas[atlas_idx + 2] = ((sb * a32 + atlas[atlas_idx + 2] as u32 * inv_a) / 255) as u8;
                        atlas[atlas_idx + 3] = (a32 + da * inv_a / 255).min(255) as u8;
                    }
                }
            }
        }
    }

    (atlas, atlas_w, atlas_h, durations)
}

/// Convert a parsed AseFile + texture handle into a SpriteSheet (same format as JSON import).
pub fn ase_to_sprite_sheet(ase: &AseFile, texture: TextureHandle) -> SpriteSheet {
    let (_, atlas_w, atlas_h, durations) = build_atlas(ase);
    let fw = ase.width as f32;
    let fh = ase.height as f32;
    let aw = atlas_w as f32;
    let ah = atlas_h as f32;

    let all_frames: Vec<AnimationFrame> = durations
        .iter()
        .enumerate()
        .map(|(i, &dur)| {
            let x = i as f32 * fw;
            AnimationFrame {
                uv_min: Vec2::new(x / aw, 0.0),
                uv_max: Vec2::new((x + fw) / aw, fh / ah),
                size: Vec2::new(fw, fh),
                duration: dur as f32 / 1000.0,
            }
        })
        .collect();

    let mut clips = HashMap::new();

    if ase.tags.is_empty() {
        clips.insert(
            "default".to_string(),
            AnimationClip {
                name: "default".to_string(),
                frames: all_frames.clone(),
                mode: PlaybackMode::Loop,
            },
        );
    } else {
        for tag in &ase.tags {
            let from = tag.from as usize;
            let to = (tag.to as usize).min(all_frames.len().saturating_sub(1));
            let mode = match tag.direction {
                1 => PlaybackMode::Loop, // reverse — frames would need reversing
                2 | 3 => PlaybackMode::PingPong,
                _ => PlaybackMode::Loop,
            };
            let mut tag_frames = all_frames[from..=to].to_vec();
            if tag.direction == 1 {
                tag_frames.reverse();
            }
            clips.insert(
                tag.name.clone(),
                AnimationClip {
                    name: tag.name.clone(),
                    frames: tag_frames,
                    mode,
                },
            );
        }
    }

    SpriteSheet { texture, clips }
}

/// Load and parse an .ase/.aseprite file from disk.
pub fn load_ase_file(path: &Path) -> Result<AseFile, String> {
    let data = std::fs::read(path).map_err(|e| format!("IO: {e}"))?;
    parse_ase(&data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid Aseprite binary file for testing.
    fn build_test_ase(
        width: u16,
        height: u16,
        frames: &[(u16, Vec<u8>)], // (duration_ms, RGBA pixels)
        tags: &[(&str, u16, u16, u8)], // (name, from, to, direction)
    ) -> Vec<u8> {
        let color_depth: u16 = 32;
        let frame_count = frames.len() as u16;

        // Build chunks for each frame
        let mut frame_datas = Vec::new();
        for (i, (duration, pixels)) in frames.iter().enumerate() {
            let mut chunks = Vec::new();

            // On first frame, emit a layer chunk + tags chunk
            if i == 0 {
                // Layer chunk
                let mut layer_chunk = Vec::new();
                layer_chunk.extend_from_slice(&1u16.to_le_bytes()); // flags: visible
                layer_chunk.extend_from_slice(&0u16.to_le_bytes()); // type: image
                layer_chunk.extend_from_slice(&0u16.to_le_bytes()); // child level
                layer_chunk.extend_from_slice(&0u16.to_le_bytes()); // default w
                layer_chunk.extend_from_slice(&0u16.to_le_bytes()); // default h
                layer_chunk.extend_from_slice(&0u16.to_le_bytes()); // blend mode
                layer_chunk.push(255); // opacity
                layer_chunk.extend_from_slice(&[0u8; 3]); // future
                let name = b"Layer 1";
                layer_chunk.extend_from_slice(&(name.len() as u16).to_le_bytes());
                layer_chunk.extend_from_slice(name);

                let chunk_size = (6 + layer_chunk.len()) as u32;
                chunks.extend_from_slice(&chunk_size.to_le_bytes());
                chunks.extend_from_slice(&CHUNK_LAYER.to_le_bytes());
                chunks.extend_from_slice(&layer_chunk);

                // Tags chunk
                if !tags.is_empty() {
                    let mut tag_data = Vec::new();
                    tag_data.extend_from_slice(&(tags.len() as u16).to_le_bytes());
                    tag_data.extend_from_slice(&[0u8; 8]); // future

                    for (name, from, to, dir) in tags {
                        tag_data.extend_from_slice(&from.to_le_bytes());
                        tag_data.extend_from_slice(&to.to_le_bytes());
                        tag_data.push(*dir);
                        tag_data.extend_from_slice(&0u16.to_le_bytes()); // repeat
                        tag_data.extend_from_slice(&[0u8; 6]); // future
                        tag_data.extend_from_slice(&[0u8; 3]); // deprecated RGB
                        tag_data.push(0); // extra
                        let name_bytes = name.as_bytes();
                        tag_data.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
                        tag_data.extend_from_slice(name_bytes);
                    }

                    let chunk_size = (6 + tag_data.len()) as u32;
                    chunks.extend_from_slice(&chunk_size.to_le_bytes());
                    chunks.extend_from_slice(&CHUNK_TAGS.to_le_bytes());
                    chunks.extend_from_slice(&tag_data);
                }
            }

            // Cel chunk (raw, type 0)
            let mut cel_data = Vec::new();
            cel_data.extend_from_slice(&0u16.to_le_bytes()); // layer index
            cel_data.extend_from_slice(&0i16.to_le_bytes()); // x
            cel_data.extend_from_slice(&0i16.to_le_bytes()); // y
            cel_data.push(255); // opacity
            cel_data.extend_from_slice(&0u16.to_le_bytes()); // cel type: raw
            cel_data.extend_from_slice(&0i16.to_le_bytes()); // z-index
            cel_data.extend_from_slice(&[0u8; 5]); // future
            cel_data.extend_from_slice(&width.to_le_bytes());
            cel_data.extend_from_slice(&height.to_le_bytes());
            cel_data.extend_from_slice(pixels);

            let chunk_size = (6 + cel_data.len()) as u32;
            chunks.extend_from_slice(&chunk_size.to_le_bytes());
            chunks.extend_from_slice(&CHUNK_CEL.to_le_bytes());
            chunks.extend_from_slice(&cel_data);

            // Frame header
            let chunk_count = if i == 0 { 1 + if tags.is_empty() { 0 } else { 1 } + 1 } else { 1u32 };
            let frame_size = (16 + chunks.len()) as u32;
            let mut frame = Vec::new();
            frame.extend_from_slice(&frame_size.to_le_bytes());
            frame.extend_from_slice(&FRAME_MAGIC.to_le_bytes());
            frame.extend_from_slice(&(chunk_count as u16).to_le_bytes()); // old chunks
            frame.extend_from_slice(&duration.to_le_bytes());
            frame.extend_from_slice(&[0u8; 2]); // future
            frame.extend_from_slice(&0u32.to_le_bytes()); // new chunks (0 = use old)
            frame.extend_from_slice(&chunks);

            frame_datas.push(frame);
        }

        // File header
        let total_frames_size: usize = frame_datas.iter().map(|f| f.len()).sum();
        let file_size = (128 + total_frames_size) as u32;

        let mut file = Vec::new();
        file.extend_from_slice(&file_size.to_le_bytes());
        file.extend_from_slice(&FILE_MAGIC.to_le_bytes());
        file.extend_from_slice(&frame_count.to_le_bytes());
        file.extend_from_slice(&width.to_le_bytes());
        file.extend_from_slice(&height.to_le_bytes());
        file.extend_from_slice(&color_depth.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes()); // flags
        file.extend_from_slice(&100u16.to_le_bytes()); // speed
        file.extend_from_slice(&[0u8; 8]); // reserved
        file.push(0); // transparent index
        file.extend_from_slice(&[0u8; 3]); // ignore
        file.extend_from_slice(&0u16.to_le_bytes()); // color count
        file.push(1); // pixel w
        file.push(1); // pixel h
        file.extend_from_slice(&[0u8; 8]); // grid
        file.extend_from_slice(&[0u8; 84]); // future

        assert_eq!(file.len(), 128);

        for frame in &frame_datas {
            file.extend_from_slice(frame);
        }

        file
    }

    #[test]
    fn parse_single_frame() {
        // 2x2 RGBA pixels: red, green, blue, white
        let pixels = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            255, 255, 255, 255, // white
        ];
        let data = build_test_ase(2, 2, &[(100, pixels.clone())], &[]);
        let ase = parse_ase(&data).unwrap();

        assert_eq!(ase.width, 2);
        assert_eq!(ase.height, 2);
        assert_eq!(ase.color_depth, 32);
        assert_eq!(ase.frames.len(), 1);
        assert_eq!(ase.frames[0].duration_ms, 100);
        assert_eq!(ase.frames[0].cels.len(), 1);

        let cel = &ase.frames[0].cels[0];
        assert_eq!(cel.width, 2);
        assert_eq!(cel.height, 2);
        assert_eq!(cel.pixels, pixels);

        assert_eq!(ase.layers.len(), 1);
        assert_eq!(ase.layers[0].name, "Layer 1");
        assert!(ase.layers[0].visible);
    }

    #[test]
    fn parse_multiple_frames_with_tags() {
        let red = vec![255, 0, 0, 255];
        let green = vec![0, 255, 0, 255];
        let blue = vec![0, 0, 255, 255];

        let data = build_test_ase(
            1, 1,
            &[(100, red), (150, green), (200, blue)],
            &[("idle", 0, 1, 0), ("jump", 2, 2, 2)],
        );
        let ase = parse_ase(&data).unwrap();

        assert_eq!(ase.frames.len(), 3);
        assert_eq!(ase.frames[0].duration_ms, 100);
        assert_eq!(ase.frames[1].duration_ms, 150);
        assert_eq!(ase.frames[2].duration_ms, 200);

        assert_eq!(ase.tags.len(), 2);
        assert_eq!(ase.tags[0].name, "idle");
        assert_eq!(ase.tags[0].from, 0);
        assert_eq!(ase.tags[0].to, 1);
        assert_eq!(ase.tags[0].direction, 0);
        assert_eq!(ase.tags[1].name, "jump");
        assert_eq!(ase.tags[1].from, 2);
        assert_eq!(ase.tags[1].to, 2);
        assert_eq!(ase.tags[1].direction, 2); // pingpong
    }

    #[test]
    fn build_atlas_horizontal_strip() {
        let frame1 = vec![255, 0, 0, 255]; // red
        let frame2 = vec![0, 0, 255, 255]; // blue

        let data = build_test_ase(1, 1, &[(100, frame1), (200, frame2)], &[]);
        let ase = parse_ase(&data).unwrap();
        let (atlas, w, h, durs) = build_atlas(&ase);

        assert_eq!(w, 2); // 2 frames * 1px
        assert_eq!(h, 1);
        assert_eq!(durs, vec![100, 200]);
        assert_eq!(atlas.len(), 8); // 2 pixels * 4 bytes

        // First pixel: red
        assert_eq!(&atlas[0..4], &[255, 0, 0, 255]);
        // Second pixel: blue
        assert_eq!(&atlas[4..8], &[0, 0, 255, 255]);
    }

    #[test]
    fn tags_to_clips() {
        let frame1 = vec![255, 0, 0, 255];
        let frame2 = vec![0, 255, 0, 255];
        let frame3 = vec![0, 0, 255, 255];

        let data = build_test_ase(
            1, 1,
            &[(100, frame1), (150, frame2), (200, frame3)],
            &[("walk", 0, 1, 0), ("attack", 2, 2, 0)],
        );
        let ase = parse_ase(&data).unwrap();

        // Verify tag parsing (sprite sheet creation requires a TextureHandle
        // which is crate-private to toile-graphics, so we test the data layer)
        assert_eq!(ase.tags.len(), 2);
        assert_eq!(ase.tags[0].name, "walk");
        assert_eq!(ase.tags[0].from, 0);
        assert_eq!(ase.tags[0].to, 1);
        assert_eq!(ase.tags[1].name, "attack");
        assert_eq!(ase.tags[1].from, 2);
        assert_eq!(ase.tags[1].to, 2);

        // Verify atlas dimensions
        let (atlas, w, h, durs) = build_atlas(&ase);
        assert_eq!(w, 3); // 3 frames
        assert_eq!(h, 1);
        assert_eq!(durs, vec![100, 150, 200]);
        assert_eq!(atlas.len(), 12); // 3 pixels * 4 bytes
    }
}
