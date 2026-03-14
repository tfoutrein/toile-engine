use std::collections::HashMap;

use glam::Vec2;

use crate::font::GlyphMetrics;

pub struct TtfRasterResult {
    pub atlas_rgba: Vec<u8>,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub line_height: f32,
    pub glyphs: HashMap<u32, GlyphMetrics>,
}

/// Rasterize ASCII printable characters (32..=126) into a texture atlas.
pub fn rasterize_ascii(ttf_bytes: &[u8], px_size: f32) -> TtfRasterResult {
    let font = fontdue::Font::from_bytes(
        ttf_bytes as &[u8],
        fontdue::FontSettings::default(),
    )
    .expect("Failed to parse TTF");

    // Rasterize each glyph
    let mut rasterized: Vec<(u32, fontdue::Metrics, Vec<u8>)> = Vec::new();
    for c in 32u32..=126 {
        let ch = char::from_u32(c).unwrap();
        let (metrics, bitmap) = font.rasterize(ch, px_size);
        rasterized.push((c, metrics, bitmap));
    }

    // Row packing into atlas
    let atlas_w: u32 = 512;
    let padding = 1u32;
    let mut cursor_x: u32 = 0;
    let mut cursor_y: u32 = 0;
    let mut row_height: u32 = 0;

    let mut positions: Vec<(u32, u32)> = Vec::new();
    for (_, metrics, _) in &rasterized {
        let gw = metrics.width as u32;
        let gh = metrics.height as u32;
        if cursor_x + gw + padding > atlas_w {
            cursor_y += row_height + padding;
            cursor_x = 0;
            row_height = 0;
        }
        positions.push((cursor_x, cursor_y));
        cursor_x += gw + padding;
        row_height = row_height.max(gh);
    }
    let atlas_h = (cursor_y + row_height + 1).next_power_of_two().max(64);

    // Blit glyphs into RGBA atlas (white text, alpha = coverage)
    let mut atlas = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    let mut glyphs = HashMap::new();

    let line_metrics = font.horizontal_line_metrics(px_size);
    let line_height = line_metrics.map(|m| m.new_line_size).unwrap_or(px_size * 1.2);

    for (i, (codepoint, metrics, bitmap)) in rasterized.iter().enumerate() {
        let (px, py) = positions[i];
        let gw = metrics.width as u32;
        let gh = metrics.height as u32;

        for row in 0..gh {
            for col in 0..gw {
                let src = (row * gw + col) as usize;
                let dst = (((py + row) * atlas_w + (px + col)) * 4) as usize;
                if dst + 3 < atlas.len() && src < bitmap.len() {
                    atlas[dst] = 255;
                    atlas[dst + 1] = 255;
                    atlas[dst + 2] = 255;
                    atlas[dst + 3] = bitmap[src];
                }
            }
        }

        glyphs.insert(
            *codepoint,
            GlyphMetrics {
                uv_min: Vec2::new(px as f32 / atlas_w as f32, py as f32 / atlas_h as f32),
                uv_max: Vec2::new(
                    (px + gw) as f32 / atlas_w as f32,
                    (py + gh) as f32 / atlas_h as f32,
                ),
                size: Vec2::new(gw as f32, gh as f32),
                offset: Vec2::new(metrics.xmin as f32, metrics.ymin as f32),
                advance: metrics.advance_width,
            },
        );
    }

    TtfRasterResult {
        atlas_rgba: atlas,
        atlas_width: atlas_w,
        atlas_height: atlas_h,
        line_height,
        glyphs,
    }
}
