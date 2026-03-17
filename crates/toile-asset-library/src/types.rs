//! Core data types for the Asset Library.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The top-level asset type categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Sprite,
    Tileset,
    Tilemap,
    Background,
    Gui,
    Icon,
    Audio,
    Font,
    Vfx,
    Prop,
    Skeleton,
    Data,
    Unknown,
}

impl AssetType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sprite => "Sprite",
            Self::Tileset => "Tileset",
            Self::Tilemap => "Tilemap",
            Self::Background => "Background",
            Self::Gui => "GUI",
            Self::Icon => "Icon",
            Self::Audio => "Audio",
            Self::Font => "Font",
            Self::Vfx => "VFX",
            Self::Prop => "Prop",
            Self::Skeleton => "Skeleton",
            Self::Data => "Data",
            Self::Unknown => "Unknown",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Sprite => "🧑",
            Self::Tileset => "🧱",
            Self::Tilemap => "🗺",
            Self::Background => "🌄",
            Self::Gui => "🖥",
            Self::Icon => "🔷",
            Self::Audio => "🔊",
            Self::Font => "🔤",
            Self::Vfx => "✨",
            Self::Prop => "📦",
            Self::Skeleton => "🦴",
            Self::Data => "📄",
            Self::Unknown => "❓",
        }
    }
}

/// Metadata specific to sprite assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteMetadata {
    pub frame_width: u32,
    pub frame_height: u32,
    pub frame_count: u32,
    pub columns: u32,
    pub rows: u32,
    #[serde(default)]
    pub animations: Vec<AnimationDef>,
    #[serde(default)]
    pub source_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationDef {
    pub name: String,
    pub frames: Vec<u32>,
    pub fps: f32,
    #[serde(default = "default_true")]
    pub looping: bool,
}

fn default_true() -> bool { true }

/// Metadata specific to tileset assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilesetMetadata {
    pub tile_width: u32,
    pub tile_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub tile_count: u32,
    #[serde(default)]
    pub spacing: u32,
    #[serde(default)]
    pub margin: u32,
}

/// Metadata specific to tilemap assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilemapMetadata {
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    #[serde(default)]
    pub orientation: String,
    #[serde(default)]
    pub layer_count: u32,
    #[serde(default)]
    pub source_format: String,
}

/// Metadata specific to background assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundMetadata {
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub is_parallax: bool,
    #[serde(default)]
    pub layers: Vec<ParallaxLayerDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallaxLayerDef {
    pub path: String,
    pub depth: f32,
    pub scroll_factor: f32,
}

/// Metadata specific to audio assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub format: String,
    #[serde(default)]
    pub duration_secs: f32,
    #[serde(default)]
    pub sample_rate: u32,
    #[serde(default)]
    pub channels: u32,
    #[serde(default)]
    pub category: String,
}

/// Metadata specific to font assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontMetadata {
    pub format: String,
    #[serde(default)]
    pub face: String,
    #[serde(default)]
    pub size: u32,
    #[serde(default)]
    pub pages: Vec<String>,
}

/// Union metadata — tagged by asset type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AssetMetadata {
    Sprite(SpriteMetadata),
    Tileset(TilesetMetadata),
    Tilemap(TilemapMetadata),
    Background(BackgroundMetadata),
    Audio(AudioMetadata),
    Font(FontMetadata),
    None,
}

/// A single indexed asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToileAsset {
    pub id: String,
    pub pack_id: String,
    pub asset_type: AssetType,
    #[serde(default)]
    pub subtype: String,
    pub name: String,
    /// Path relative to pack root.
    pub path: String,
    #[serde(default)]
    pub thumbnail_path: Option<String>,
    #[serde(default)]
    pub metadata: AssetMetadata,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub related_assets: Vec<String>,
}

/// Pack-level information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInfo {
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub import_date: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// The full manifest for a pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub manifest_version: String,
    pub pack: PackInfo,
    pub assets: Vec<ToileAsset>,
}

impl Default for AssetMetadata {
    fn default() -> Self { Self::None }
}

/// A scanned file (before classification).
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: String,
    pub extension: String,
    pub size_bytes: u64,
}
