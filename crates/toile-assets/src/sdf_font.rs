use std::collections::HashMap;

use glam::Vec2;

/// SDF search spread in pixels (max outline/glow radius).
const SPREAD: u32 = 4;

/// Per-glyph metrics for an SDF atlas entry.
/// The UV rect covers the padded glyph (glyph + SPREAD on every side).
#[derive(Debug, Clone, Copy)]
pub struct SdfGlyphMetrics {
    pub uv_min:  Vec2,
    pub uv_max:  Vec2,
    /// Padded size in pixels at the reference `ref_px` scale.
    pub size:    Vec2,
    /// Offset from pen to top-left of the padded quad.
    pub offset:  Vec2,
    pub advance: f32,
}

/// A loaded SDF font atlas.
pub struct SdfFont {
    /// Index into `SdfTextRenderer::textures`.
    pub texture_idx: usize,
    pub line_height: f32,
    pub glyphs:      HashMap<u32, SdfGlyphMetrics>,
    /// SPREAD value used when generating the atlas.
    pub spread_px:   f32,
    /// Pixel size used to bake the atlas (= the reference size for scaling).
    pub ref_px:      f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MsdfFontHandle(pub u32);

pub struct SdfRasterResult {
    /// Single-channel R8 distance values (0=outside, 128=edge, 255=inside).
    pub atlas_r8:     Vec<u8>,
    pub atlas_width:  u32,
    pub atlas_height: u32,
    pub line_height:  f32,
    pub glyphs:       HashMap<u32, SdfGlyphMetrics>,
    pub spread_px:    f32,
    pub ref_px:       f32,
}

/// Bounded brute-force signed Euclidean distance transform.
/// For every pixel, searches within SPREAD pixels for the nearest pixel
/// of the opposite class (inside vs. outside).
/// Returns R8 values: 128 = edge, 255 = deep inside, 0 = deep outside.
fn compute_sdf(mask: &[bool], w: u32, h: u32) -> Vec<u8> {
    let spread_f = SPREAD as f32;
    let mut out = vec![0u8; (w * h) as usize];

    for y in 0..h {
        for x in 0..w {
            let inside = mask[(y * w + x) as usize];
            let mut min_dist_sq = (SPREAD * SPREAD + 1) as f32;

            let x0 = x.saturating_sub(SPREAD);
            let x1 = (x + SPREAD + 1).min(w);
            let y0 = y.saturating_sub(SPREAD);
            let y1 = (y + SPREAD + 1).min(h);

            for sy in y0..y1 {
                for sx in x0..x1 {
                    if mask[(sy * w + sx) as usize] != inside {
                        let dx = sx as f32 - x as f32;
                        let dy = sy as f32 - y as f32;
                        let d2 = dx * dx + dy * dy;
                        if d2 < min_dist_sq {
                            min_dist_sq = d2;
                        }
                    }
                }
            }

            let dist  = min_dist_sq.sqrt().min(spread_f);
            let signed = if inside { dist } else { -dist };
            out[(y * w + x) as usize] =
                ((signed / spread_f) * 127.0 + 128.0).clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// Rasterize ASCII printable glyphs (32..=126) into a single-channel SDF atlas.
pub fn rasterize_sdf(ttf_bytes: &[u8], px_size: f32) -> SdfRasterResult {
    let font = fontdue::Font::from_bytes(ttf_bytes, fontdue::FontSettings::default())
        .expect("Failed to parse TTF");

    let pad = SPREAD;

    // Rasterize every printable ASCII glyph
    let mut rasterized: Vec<(u32, fontdue::Metrics, Vec<u8>)> = Vec::new();
    for c in 32u32..=126 {
        let ch = char::from_u32(c).unwrap();
        let (metrics, bitmap) = font.rasterize(ch, px_size);
        rasterized.push((c, metrics, bitmap));
    }

    // Row-pack padded glyphs into atlas
    let atlas_w: u32 = 512;
    let padding = 1u32;
    let mut cursor_x = 0u32;
    let mut cursor_y = 0u32;
    let mut row_height = 0u32;

    // (atlas_x, atlas_y, padded_w, padded_h)
    let mut positions: Vec<(u32, u32, u32, u32)> = Vec::new();
    for (_, metrics, _) in &rasterized {
        let pw = metrics.width  as u32 + 2 * pad;
        let ph = metrics.height as u32 + 2 * pad;
        if cursor_x + pw + padding > atlas_w {
            cursor_y += row_height + padding;
            cursor_x = 0;
            row_height = 0;
        }
        positions.push((cursor_x, cursor_y, pw, ph));
        cursor_x  += pw + padding;
        row_height = row_height.max(ph);
    }
    let atlas_h = (cursor_y + row_height + 1).next_power_of_two().max(64);

    // Build atlas — fill with 128 (= "exactly on the edge", acts as transparent border)
    let mut atlas = vec![128u8; (atlas_w * atlas_h) as usize];
    let mut glyphs = HashMap::new();

    let line_metrics = font.horizontal_line_metrics(px_size);
    let line_height  = line_metrics.map(|m| m.new_line_size).unwrap_or(px_size * 1.2);

    for (i, (codepoint, metrics, bitmap)) in rasterized.iter().enumerate() {
        let (ax, ay, pw, ph) = positions[i];
        let gw = metrics.width  as u32;
        let gh = metrics.height as u32;

        // Binary coverage mask with SPREAD padding on all sides
        let mut mask = vec![false; (pw * ph) as usize];
        for row in 0..gh {
            for col in 0..gw {
                let src = (row * gw + col) as usize;
                if src < bitmap.len() && bitmap[src] > 127 {
                    mask[((row + pad) * pw + (col + pad)) as usize] = true;
                }
            }
        }

        let sdf = compute_sdf(&mask, pw, ph);

        // Blit into R8 atlas
        for row in 0..ph {
            for col in 0..pw {
                let dst = ((ay + row) * atlas_w + (ax + col)) as usize;
                if dst < atlas.len() {
                    atlas[dst] = sdf[(row * pw + col) as usize];
                }
            }
        }

        glyphs.insert(
            *codepoint,
            SdfGlyphMetrics {
                uv_min: Vec2::new(
                    ax as f32 / atlas_w as f32,
                    ay as f32 / atlas_h as f32,
                ),
                uv_max: Vec2::new(
                    (ax + pw) as f32 / atlas_w as f32,
                    (ay + ph) as f32 / atlas_h as f32,
                ),
                // Padded size — quads include the spread border so outlines render fully
                size:   Vec2::new(pw as f32, ph as f32),
                // Offset shifted left/up by SPREAD to align padded quad with pen position
                offset: Vec2::new(
                    metrics.xmin as f32 - pad as f32,
                    metrics.ymin as f32 - pad as f32,
                ),
                advance: metrics.advance_width,
            },
        );
    }

    SdfRasterResult {
        atlas_r8: atlas,
        atlas_width: atlas_w,
        atlas_height: atlas_h,
        line_height,
        glyphs,
        spread_px: SPREAD as f32,
        ref_px: px_size,
    }
}
