//! In-memory asset library — the central index for all imported packs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::classifier;
use crate::heuristics;
use crate::manifest;
use crate::scanner;
use crate::thumbnail;
use crate::types::*;

/// The main asset library. Holds all imported packs and provides queries.
pub struct ToileAssetLibrary {
    pub packs: HashMap<String, PackInfo>,
    pub assets: Vec<ToileAsset>,
    pack_roots: HashMap<String, PathBuf>,
}

impl ToileAssetLibrary {
    pub fn new() -> Self {
        Self {
            packs: HashMap::new(),
            assets: Vec::new(),
            pack_roots: HashMap::new(),
        }
    }

    /// Import a pack from a directory. Scans, classifies, generates manifest.
    pub fn import_pack(&mut self, pack_dir: &Path) -> Result<usize, String> {
        let pack_name = pack_dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".into());

        let pack_id = pack_name.replace(' ', "_").to_lowercase();

        // Check for existing manifest
        if manifest::has_manifest(pack_dir) {
            let m = manifest::load_manifest(&manifest::manifest_path(pack_dir))?;
            let count = m.assets.len();
            self.packs.insert(pack_id.clone(), m.pack);
            self.pack_roots.insert(pack_id.clone(), pack_dir.to_path_buf());
            // Remove old assets from this pack
            self.assets.retain(|a| a.pack_id != pack_id);
            self.assets.extend(m.assets);
            return Ok(count);
        }

        // Scan
        let scanned = scanner::scan_directory(pack_dir);
        log::info!("Scanned {} files in '{}'", scanned.len(), pack_name);

        // Classify and build assets
        let mut new_assets = Vec::new();
        let thumb_dir = pack_dir.join(".toile").join("thumbs");

        for file in &scanned {
            let asset_type = classifier::classify(file);
            if asset_type == AssetType::Unknown || asset_type == AssetType::Data {
                continue; // Skip unknown and data files
            }

            let name = Path::new(&file.path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| file.path.clone());

            let subtype = classifier::detect_subtype(file, asset_type);
            let tags = classifier::tags_from_path(&file.path);

            // Build metadata
            let metadata = build_metadata(pack_dir, file, asset_type);

            // Generate thumbnail for images
            let thumb_path = if matches!(asset_type, AssetType::Sprite | AssetType::Tileset | AssetType::Background | AssetType::Icon | AssetType::Gui | AssetType::Prop | AssetType::Vfx) {
                let thumb_name = format!("{}.png", file.path.replace('/', "_").replace(' ', "_"));
                let thumb_full = thumb_dir.join(&thumb_name);
                let source = pack_dir.join(&file.path);

                // Generate thumbnail based on metadata
                let result = if let AssetMetadata::Sprite(ref sm) = metadata {
                    thumbnail::generate_spritesheet_thumbnail(&source, &thumb_full, sm.frame_width, sm.frame_height)
                } else {
                    thumbnail::generate_thumbnail(&source, &thumb_full)
                };

                match result {
                    Ok(()) => Some(format!(".toile/thumbs/{thumb_name}")),
                    Err(e) => { log::warn!("Thumbnail failed for {}: {e}", file.path); None }
                }
            } else {
                None
            };

            let id = format!("{}_{}", pack_id, name.replace(' ', "_").to_lowercase());

            new_assets.push(ToileAsset {
                id,
                pack_id: pack_id.clone(),
                asset_type,
                subtype,
                name,
                path: file.path.clone(),
                thumbnail_path: thumb_path,
                metadata,
                tags,
                related_assets: vec![],
            });
        }

        let count = new_assets.len();
        log::info!("Classified {} assets from '{}'", count, pack_name);

        // Build and save manifest
        let pack_info = PackInfo {
            name: pack_name.clone(),
            author: String::new(),
            license: String::new(),
            source: String::new(),
            import_date: String::new(),
            tags: vec![],
        };

        let manifest_data = AssetManifest {
            manifest_version: manifest::current_version().into(),
            pack: pack_info.clone(),
            assets: new_assets.clone(),
        };

        if let Err(e) = manifest::save_manifest(&manifest::manifest_path(pack_dir), &manifest_data) {
            log::warn!("Could not save manifest: {e}");
        }

        // Store
        self.packs.insert(pack_id.clone(), pack_info);
        self.pack_roots.insert(pack_id.clone(), pack_dir.to_path_buf());
        self.assets.retain(|a| a.pack_id != pack_id);
        self.assets.extend(new_assets);

        Ok(count)
    }

    /// Get all assets of a specific type.
    pub fn by_type(&self, asset_type: AssetType) -> Vec<&ToileAsset> {
        self.assets.iter().filter(|a| a.asset_type == asset_type).collect()
    }

    /// Search assets by text (matches name, path, and tags).
    pub fn search(&self, query: &str) -> Vec<&ToileAsset> {
        let lower = query.to_lowercase();
        self.assets.iter().filter(|a| {
            a.name.to_lowercase().contains(&lower)
                || a.path.to_lowercase().contains(&lower)
                || a.tags.iter().any(|t| t.contains(&lower))
        }).collect()
    }

    /// Get the absolute path of an asset.
    pub fn absolute_path(&self, asset: &ToileAsset) -> Option<PathBuf> {
        self.pack_roots.get(&asset.pack_id).map(|root| root.join(&asset.path))
    }

    /// Get the absolute thumbnail path of an asset.
    pub fn thumbnail_absolute_path(&self, asset: &ToileAsset) -> Option<PathBuf> {
        asset.thumbnail_path.as_ref().and_then(|tp| {
            self.pack_roots.get(&asset.pack_id).map(|root| root.join(tp))
        })
    }

    /// Total asset count.
    pub fn count(&self) -> usize {
        self.assets.len()
    }

    /// List all pack names.
    pub fn pack_names(&self) -> Vec<&str> {
        self.packs.values().map(|p| p.name.as_str()).collect()
    }
}

/// Build metadata from file context.
fn build_metadata(pack_dir: &Path, file: &ScannedFile, asset_type: AssetType) -> AssetMetadata {
    match asset_type {
        AssetType::Sprite | AssetType::Vfx => {
            let source = pack_dir.join(&file.path);
            if let Some((w, h)) = thumbnail::image_dimensions(&source) {
                // Try filename heuristic first
                let (fw, fh) = heuristics::frame_size_from_filename(&file.path)
                    .unwrap_or_else(|| {
                        if heuristics::is_horizontal_strip(w, h) {
                            (h, h) // Square frames, height = frame size
                        } else {
                            let (fw, fh, _, _) = heuristics::detect_sprite_grid(w, h);
                            (fw, fh)
                        }
                    });

                let cols = if fw > 0 { w / fw } else { 1 };
                let rows = if fh > 0 { h / fh } else { 1 };

                AssetMetadata::Sprite(SpriteMetadata {
                    frame_width: fw,
                    frame_height: fh,
                    frame_count: cols * rows,
                    columns: cols,
                    rows,
                    animations: vec![],
                    source_format: file.extension.clone(),
                })
            } else {
                AssetMetadata::None
            }
        }
        AssetType::Tileset => {
            let source = pack_dir.join(&file.path);
            if let Some((w, h)) = thumbnail::image_dimensions(&source) {
                let (tw, th) = heuristics::frame_size_from_filename(&file.path)
                    .unwrap_or_else(|| {
                        let (fw, fh, _, _) = heuristics::detect_sprite_grid(w, h);
                        (fw, fh)
                    });

                let cols = if tw > 0 { w / tw } else { 1 };
                let rows = if th > 0 { h / th } else { 1 };

                AssetMetadata::Tileset(TilesetMetadata {
                    tile_width: tw,
                    tile_height: th,
                    columns: cols,
                    rows,
                    tile_count: cols * rows,
                    spacing: 0,
                    margin: 0,
                })
            } else {
                AssetMetadata::None
            }
        }
        AssetType::Audio => {
            AssetMetadata::Audio(AudioMetadata {
                format: file.extension.clone(),
                duration_secs: 0.0,
                sample_rate: 0,
                channels: 0,
                category: if file.path.to_lowercase().contains("music") { "music".into() } else { "sfx".into() },
            })
        }
        AssetType::Font => {
            AssetMetadata::Font(FontMetadata {
                format: file.extension.clone(),
                face: String::new(),
                size: 0,
                pages: vec![],
            })
        }
        AssetType::Background => {
            let source = pack_dir.join(&file.path);
            let (w, h) = thumbnail::image_dimensions(&source).unwrap_or((0, 0));
            AssetMetadata::Background(BackgroundMetadata {
                width: w,
                height: h,
                is_parallax: false,
                layers: vec![],
            })
        }
        _ => AssetMetadata::None,
    }
}
