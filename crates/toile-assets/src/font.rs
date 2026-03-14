use std::collections::HashMap;

use glam::Vec2;
use toile_graphics::texture::TextureHandle;

/// Metrics for a single glyph in the font atlas.
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub size: Vec2,
    pub offset: Vec2, // offset from pen position to glyph bottom-left
    pub advance: f32,
}

/// A loaded font (TTF or BMFont) ready for rendering.
pub struct Font {
    pub texture: TextureHandle,
    pub line_height: f32,
    pub glyphs: HashMap<u32, GlyphMetrics>,
}

/// Simple handle to a loaded font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontHandle(pub u32);
