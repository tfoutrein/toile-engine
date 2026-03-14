use std::collections::HashMap;
use std::path::Path;

use glam::Vec2;
use serde::Deserialize;
use toile_graphics::texture::TextureHandle;

// --- Aseprite JSON serde structs ---

#[derive(Deserialize)]
struct AseExport {
    frames: Vec<AseFrame>,
    meta: AseMeta,
}

#[derive(Deserialize)]
struct AseFrame {
    frame: AseRect,
    duration: u32,
}

#[derive(Deserialize)]
struct AseRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Deserialize)]
struct AseMeta {
    size: AseSize,
    #[serde(rename = "frameTags", default)]
    frame_tags: Vec<AseTag>,
}

#[derive(Deserialize)]
struct AseSize {
    w: u32,
    h: u32,
}

#[derive(Deserialize)]
struct AseTag {
    name: String,
    from: usize,
    to: usize,
    #[serde(default = "default_direction")]
    direction: String,
}

fn default_direction() -> String {
    "forward".to_string()
}

// --- Engine animation types ---

#[derive(Debug, Clone, Copy)]
pub struct AnimationFrame {
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub size: Vec2,
    pub duration: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackMode {
    Loop,
    Once,
    PingPong,
}

#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub frames: Vec<AnimationFrame>,
    pub mode: PlaybackMode,
}

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub texture: TextureHandle,
    pub clips: HashMap<String, AnimationClip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpriteSheetHandle(pub u32);

/// Parse an Aseprite JSON export (array format) and build a SpriteSheet.
pub fn load_aseprite_json(json_text: &str, texture: TextureHandle) -> SpriteSheet {
    let export: AseExport =
        serde_json::from_str(json_text).expect("Failed to parse Aseprite JSON");

    let atlas_w = export.meta.size.w as f32;
    let atlas_h = export.meta.size.h as f32;

    // Convert all frames to engine format
    let all_frames: Vec<AnimationFrame> = export
        .frames
        .iter()
        .map(|f| {
            let r = &f.frame;
            AnimationFrame {
                uv_min: Vec2::new(r.x as f32 / atlas_w, r.y as f32 / atlas_h),
                uv_max: Vec2::new((r.x + r.w) as f32 / atlas_w, (r.y + r.h) as f32 / atlas_h),
                size: Vec2::new(r.w as f32, r.h as f32),
                duration: f.duration as f32 / 1000.0,
            }
        })
        .collect();

    let mut clips = HashMap::new();

    if export.meta.frame_tags.is_empty() {
        // No tags — create a single "default" clip with all frames
        clips.insert(
            "default".to_string(),
            AnimationClip {
                name: "default".to_string(),
                frames: all_frames.clone(),
                mode: PlaybackMode::Loop,
            },
        );
    } else {
        for tag in &export.meta.frame_tags {
            let mode = match tag.direction.as_str() {
                "pingpong" => PlaybackMode::PingPong,
                "reverse" => PlaybackMode::Loop, // TODO: reverse frame order
                _ => PlaybackMode::Loop,
            };
            let frames = all_frames[tag.from..=tag.to].to_vec();
            clips.insert(
                tag.name.clone(),
                AnimationClip {
                    name: tag.name.clone(),
                    frames,
                    mode,
                },
            );
        }
    }

    SpriteSheet { texture, clips }
}

/// Load an Aseprite export from file paths.
pub fn load_aseprite(json_path: &Path, texture: TextureHandle) -> SpriteSheet {
    let json_text = std::fs::read_to_string(json_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", json_path.display()));
    load_aseprite_json(&json_text, texture)
}
