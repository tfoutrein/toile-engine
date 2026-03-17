//! Toile Asset Library — import, classify, index, and browse game asset packs.
//!
//! # Architecture
//!
//! - `types` — Core data structures (AssetType, ToileAsset, AssetManifest)
//! - `scanner` — Recursive directory/ZIP scanning
//! - `classifier` — Three-pass classification (extension → path → heuristics)
//! - `heuristics` — Frame size, grid, and parallax detection
//! - `manifest` — Manifest read/write (toile-asset-manifest.json)
//! - `thumbnail` — Thumbnail generation for the browser
//! - `library` — In-memory index with queries
//! - `importers` — Format-specific parsers (Tiled, audio, etc.)
//! - `ui` — egui widget (AssetBrowserPanel)

pub mod types;
pub mod scanner;
pub mod classifier;
pub mod heuristics;
pub mod manifest;
pub mod thumbnail;
pub mod library;
pub mod importers;
pub mod ui;

// Re-exports for convenience
pub use types::{AssetType, ToileAsset, AssetManifest, AssetMetadata};
pub use library::ToileAssetLibrary;
