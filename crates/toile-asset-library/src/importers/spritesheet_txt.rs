//! Parser for spritesheet.txt + spritesheet.png atlas format.
//!
//! Format: each line is `path/to/frame.png = x y width height`
//! The atlas image (spritesheet.png) is in the same directory.

use std::path::Path;

/// A single frame definition from spritesheet.txt.
#[derive(Debug, Clone)]
pub struct AtlasFrame {
    pub path: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A parsed atlas with all frame definitions.
#[derive(Debug, Clone)]
pub struct AtlasDescriptor {
    pub frames: Vec<AtlasFrame>,
    pub atlas_image: String, // relative path to the spritesheet.png
}

/// Group of frames belonging to one animation.
#[derive(Debug, Clone)]
pub struct AtlasAnimation {
    pub name: String,
    pub frames: Vec<AtlasFrame>,
    pub frame_width: u32,
    pub frame_height: u32,
}

/// Parse a spritesheet.txt file.
pub fn parse_spritesheet_txt(content: &str) -> Vec<AtlasFrame> {
    let mut frames = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        // Format: path = x y w h
        if let Some((path_part, coords_part)) = line.split_once('=') {
            let path = path_part.trim().to_string();
            let nums: Vec<u32> = coords_part.trim()
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();

            if nums.len() >= 4 {
                frames.push(AtlasFrame {
                    path,
                    x: nums[0],
                    y: nums[1],
                    width: nums[2],
                    height: nums[3],
                });
            }
        }
    }

    frames
}

/// Group frames into animations based on directory structure.
/// e.g. "PNG/Explosions/explosion_001/frame0000.png" → animation "explosion_001"
pub fn group_into_animations(frames: &[AtlasFrame]) -> Vec<AtlasAnimation> {
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<String, Vec<AtlasFrame>> = BTreeMap::new();

    for frame in frames {
        // Extract animation name from path — use the parent folder name
        let parts: Vec<&str> = frame.path.split('/').collect();
        let anim_name = if parts.len() >= 2 {
            // Use the second-to-last folder as animation name
            parts[parts.len() - 2].to_string()
        } else {
            "default".to_string()
        };

        groups.entry(anim_name).or_default().push(frame.clone());
    }

    groups.into_iter().map(|(name, mut frames)| {
        // Sort frames by their path (frame0000, frame0001, etc.)
        frames.sort_by(|a, b| a.path.cmp(&b.path));

        let (w, h) = frames.first()
            .map(|f| (f.width, f.height))
            .unwrap_or((32, 32));

        AtlasAnimation {
            name,
            frames,
            frame_width: w,
            frame_height: h,
        }
    }).collect()
}

/// Detect spritesheet.txt files in a scanned file list and return their paths.
pub fn find_spritesheet_descriptors(files: &[crate::types::ScannedFile]) -> Vec<String> {
    files.iter()
        .filter(|f| {
            let lower = f.path.to_lowercase();
            lower.ends_with("spritesheet.txt") || lower.ends_with("sprite_sheet.txt")
        })
        .map(|f| f.path.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let txt = r#"
PNG/Explosions/exp_001/frame0000.png = 0 0 80 80
PNG/Explosions/exp_001/frame0001.png = 80 0 80 80
PNG/Explosions/exp_001/frame0002.png = 160 0 80 80
PNG/Symbols/sym_001/frame0000.png = 0 80 64 64
PNG/Symbols/sym_001/frame0001.png = 64 80 64 64
"#;
        let frames = parse_spritesheet_txt(txt);
        assert_eq!(frames.len(), 5);
        assert_eq!(frames[0].x, 0);
        assert_eq!(frames[0].width, 80);
        assert_eq!(frames[2].x, 160);
    }

    #[test]
    fn group_animations() {
        let txt = r#"
PNG/exp_001/frame0000.png = 0 0 80 80
PNG/exp_001/frame0001.png = 80 0 80 80
PNG/sym_001/frame0000.png = 0 80 64 64
PNG/sym_001/frame0001.png = 64 80 64 64
"#;
        let frames = parse_spritesheet_txt(txt);
        let anims = group_into_animations(&frames);
        assert_eq!(anims.len(), 2);
        assert!(anims.iter().any(|a| a.name == "exp_001" && a.frames.len() == 2));
        assert!(anims.iter().any(|a| a.name == "sym_001" && a.frames.len() == 2));
    }
}
