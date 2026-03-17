//! Heuristic detection for frame sizes, tileset grids, and parallax layers.

use std::path::Path;

/// Common frame/tile sizes in pixel art.
pub const COMMON_SIZES: &[u32] = &[8, 16, 24, 32, 48, 64, 96, 128, 256];

/// Try to detect frame size from a filename.
/// Matches patterns like "(32x32)", "_32x32", "strip4", etc.
pub fn frame_size_from_filename(filename: &str) -> Option<(u32, u32)> {
    // Pattern: (WxH)
    if let Some(start) = filename.find('(') {
        if let Some(end) = filename[start..].find(')') {
            let inner = &filename[start + 1..start + end];
            if let Some((w, h)) = parse_wxh(inner) {
                return Some((w, h));
            }
        }
    }

    // Pattern: _WxH_ or _WxH.
    for sep in &["_", "-"] {
        for part in filename.split(|c: char| c == '_' || c == '-' || c == '.') {
            if let Some((w, h)) = parse_wxh(part) {
                return Some((w, h));
            }
        }
    }

    // Pattern: strip<N> → we know it's a horizontal strip but not the frame size
    None
}

fn parse_wxh(s: &str) -> Option<(u32, u32)> {
    let lower = s.to_lowercase();
    let parts: Vec<&str> = lower.split('x').collect();
    if parts.len() == 2 {
        if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            if w > 0 && h > 0 && w <= 1024 && h <= 1024 {
                return Some((w, h));
            }
        }
    }
    None
}

/// Auto-detect spritesheet grid from image dimensions.
/// Returns (frame_width, frame_height, columns, rows).
///
/// Scoring prefers:
/// 1. Exact division (must divide evenly)
/// 2. Reasonable frame sizes (16-128px preferred over tiny 8px)
/// 3. Square frames
/// 4. Moderate frame counts (4-64 frames ideal, not 900+)
pub fn detect_sprite_grid(img_width: u32, img_height: u32) -> (u32, u32, u32, u32) {
    let mut best = (img_width, img_height, 1u32, 1u32);
    let mut best_score = 0i32;

    for &fw in COMMON_SIZES {
        for &fh in COMMON_SIZES {
            if fw > img_width || fh > img_height { continue; }
            let cols = img_width / fw;
            let rows = img_height / fh;
            if cols == 0 || rows == 0 { continue; }

            // Must divide evenly
            if img_width % fw != 0 || img_height % fh != 0 { continue; }

            let frames = cols * rows;

            // Prefer reasonable frame sizes (penalize tiny frames like 8×8)
            let size_score = match fw.min(fh) {
                0..=8 => 10,      // very small — unlikely for characters
                16 => 200,        // common pixel art
                24 => 250,
                32 => 300,        // most common
                48 => 280,
                64 => 260,
                96 => 200,
                128 => 150,
                _ => 100,
            };

            // Prefer reasonable frame counts (4-64 is typical)
            let count_score = match frames {
                1 => 50,
                2..=4 => 150,
                5..=16 => 200,
                17..=64 => 180,
                65..=128 => 100,
                _ => 20,          // 200+ frames is suspicious
            };

            let square = if fw == fh { 80 } else { 0 };

            let score = size_score + count_score + square;

            if score > best_score {
                best_score = score;
                best = (fw, fh, cols, rows);
            }
        }
    }

    best
}

/// Detect if an image is likely a horizontal strip (1 row of frames).
pub fn is_horizontal_strip(img_width: u32, img_height: u32) -> bool {
    img_width > img_height * 2
}

/// Order parallax layers from a list of filenames.
/// Returns (filename, depth) pairs sorted from farthest (0.0) to nearest (1.0).
pub fn order_parallax_layers(filenames: &[String]) -> Vec<(String, f32)> {
    let mut layers: Vec<(String, u32)> = filenames.iter().map(|f| {
        // Try to extract a number from the filename
        let num = f.chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0);
        (f.clone(), num)
    }).collect();

    layers.sort_by_key(|(_, n)| *n);

    let count = layers.len().max(1) as f32;
    layers.iter().enumerate().map(|(i, (name, _))| {
        let depth = i as f32 / (count - 1.0).max(1.0);
        (name.clone(), depth)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_size_parentheses() {
        assert_eq!(frame_size_from_filename("Idle (32x32).png"), Some((32, 32)));
        assert_eq!(frame_size_from_filename("Run (48x48).png"), Some((48, 48)));
    }

    #[test]
    fn detect_grid() {
        let (fw, fh, cols, rows) = detect_sprite_grid(512, 512);
        // Should find a valid grid that divides evenly
        assert_eq!(512 % fw, 0);
        assert_eq!(512 % fh, 0);
        assert_eq!(cols, 512 / fw);
        assert_eq!(rows, 512 / fh);
    }

    #[test]
    fn strip_detection() {
        assert!(is_horizontal_strip(256, 32));
        assert!(!is_horizontal_strip(64, 64));
    }

    #[test]
    fn parallax_ordering() {
        let files = vec!["3_trees.png".into(), "1_sky.png".into(), "2_mountains.png".into()];
        let ordered = order_parallax_layers(&files);
        assert_eq!(ordered[0].0, "1_sky.png");
        assert_eq!(ordered[2].0, "3_trees.png");
        assert!(ordered[0].1 < ordered[2].1);
    }
}
