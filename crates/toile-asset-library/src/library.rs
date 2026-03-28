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
    pub pack_roots: HashMap<String, PathBuf>,
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
    /// Import a pack from a directory.
    pub fn import_pack(&mut self, pack_dir: &Path) -> Result<usize, String> {
        self.import_pack_with_plan(pack_dir, None, None)
    }

    /// Import with an AI-generated ImportPlan for better classification.
    pub fn import_pack_with_ai_plan(&mut self, pack_dir: &Path, plan: &crate::ai_import::ImportPlan) -> Result<usize, String> {
        self.import_pack_with_plan(pack_dir, None, Some(plan))
    }

    /// Import with optional progress callback and optional AI plan.
    pub fn import_pack_with_plan(
        &mut self,
        pack_dir: &Path,
        progress: Option<&dyn Fn(u32, u32)>,
        ai_plan: Option<&crate::ai_import::ImportPlan>,
    ) -> Result<usize, String> {
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

        // Detect spritesheet.txt descriptors and build atlas-based assets
        let descriptor_paths = crate::importers::spritesheet_txt::find_spritesheet_descriptors(&scanned);
        let mut atlas_sprite_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Classify and build assets
        let mut new_assets = Vec::new();
        let thumb_dir = pack_dir.join(".toile").join("thumbs");

        // Process spritesheet.txt + spritesheet.png pairs
        for desc_path in &descriptor_paths {
            let full_desc = pack_dir.join(desc_path);
            if let Ok(content) = std::fs::read_to_string(&full_desc) {
                let frames = crate::importers::spritesheet_txt::parse_spritesheet_txt(&content);
                let anims = crate::importers::spritesheet_txt::group_into_animations(&frames);

                // The atlas PNG is next to the txt
                let desc_dir = Path::new(desc_path).parent().unwrap_or(Path::new(""));
                let atlas_path = format!("{}/spritesheet.png", desc_dir.to_string_lossy());

                // Mark all individual frame PNGs so we skip them in the main loop
                for frame in &frames {
                    atlas_sprite_paths.insert(frame.path.clone());
                }

                // Create one asset per animation group
                for anim in &anims {
                    let anim_id = format!("{}_{}", pack_id, anim.name.replace(' ', "_").to_lowercase());
                    let anim_defs = vec![crate::types::AnimationDef {
                        name: anim.name.clone(),
                        frames: (0..anim.frames.len() as u32).collect(),
                        fps: 10.0,
                        looping: true,
                    }];

                    // Generate thumbnail from first frame in the atlas
                    let thumb_path_opt = if let Some(first) = anim.frames.first() {
                        let atlas_full = pack_dir.join(&atlas_path);
                        let thumb_name = format!("{}.png", anim.name.replace('/', "_").replace(' ', "_"));
                        let thumb_full = thumb_dir.join(&thumb_name);
                        if atlas_full.exists() {
                            if let Ok(img) = image::open(&atlas_full) {
                                let cropped = img.crop_imm(first.x, first.y, first.width, first.height);
                                let thumb = cropped.thumbnail(128, 128);
                                let _ = std::fs::create_dir_all(&thumb_dir);
                                let _ = thumb.save(&thumb_full);
                                Some(format!(".toile/thumbs/{thumb_name}"))
                            } else { None }
                        } else { None }
                    } else { None };

                    new_assets.push(ToileAsset {
                        id: anim_id,
                        pack_id: pack_id.clone(),
                        asset_type: AssetType::Sprite,
                        subtype: "atlas_animation".into(),
                        name: anim.name.clone(),
                        path: atlas_path.clone(),
                        thumbnail_path: thumb_path_opt,
                        metadata: AssetMetadata::Sprite(SpriteMetadata {
                            frame_width: anim.frame_width,
                            frame_height: anim.frame_height,
                            frame_count: anim.frames.len() as u32,
                            columns: anim.frames.len() as u32,
                            rows: 1,
                            animations: anim_defs,
                            source_format: "atlas_txt".into(),
                        }),
                        tags: classifier::tags_from_path(&anim.name),
                        related_assets: vec![],
                    });
                }
            }
        }

        // If we found atlas descriptors, also skip the entire PNG/ folder
        // (it contains individual frames that duplicate the atlas content)
        let has_atlas = !descriptor_paths.is_empty();

        let total_files = scanned.len() as u32;
        for (file_idx, file) in scanned.iter().enumerate() {
            if let Some(cb) = &progress {
                cb(file_idx as u32, total_files);
            }
            // Skip individual frames that are referenced in a spritesheet.txt descriptor
            if atlas_sprite_paths.contains(&file.path) { continue; }
            // Skip spritesheet.txt descriptors themselves
            if file.path.to_lowercase().ends_with("spritesheet.txt") { continue; }
            // Skip individual frame PNGs (frameXXXX.png) in packs that have atlas descriptors
            if has_atlas && file.extension == "png" {
                let filename = std::path::Path::new(&file.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                if filename.starts_with("frame") && filename.len() <= 15 {
                    continue; // e.g. frame0000.png, frame0042.png
                }
            }

            let mut asset_type = classifier::classify(file);
            if asset_type == AssetType::Unknown || asset_type == AssetType::Data {
                continue;
            }

            let name = Path::new(&file.path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| file.path.clone());

            let mut subtype = classifier::detect_subtype(file, asset_type);
            let mut tags = classifier::tags_from_path(&file.path);

            // Build metadata (heuristic)
            let mut metadata = build_metadata(pack_dir, file, asset_type);

            // ── AI Plan overrides ──
            if let Some(plan) = ai_plan {
                // Override classification
                if let Some(classif) = plan.classifications.iter().find(|c| c.file == file.path) {
                    let new_type = match classif.asset_type.as_str() {
                        "sprite" => AssetType::Sprite,
                        "tileset" => AssetType::Tileset,
                        "background" => AssetType::Background,
                        "gui" => AssetType::Gui,
                        "icon" => AssetType::Icon,
                        "vfx" => AssetType::Vfx,
                        "prop" => AssetType::Prop,
                        _ => asset_type,
                    };
                    if new_type != asset_type {
                        log::info!("AI override: {} reclassified {:?} → {:?}", file.path, asset_type, new_type);
                        asset_type = new_type;
                    }
                    // Override tile size for tilesets
                    if asset_type == AssetType::Tileset {
                        if let (Some(tw), Some(th)) = (classif.tile_width, classif.tile_height) {
                            metadata = AssetMetadata::Sprite(SpriteMetadata {
                                frame_width: tw, frame_height: th,
                                columns: 1, rows: 1, frame_count: 1,
                                animations: vec![], source_format: String::new(),
                            });
                        }
                    }
                }
                // Override sprite metadata with animation plan
                if let Some(anim_plan) = plan.animations.iter().find(|a| a.file == file.path) {
                    log::info!("AI override: {} → {}x{} grid {}x{}, {} animations",
                        file.path, anim_plan.frame_width, anim_plan.frame_height,
                        anim_plan.columns, anim_plan.rows, anim_plan.animations.len());
                    asset_type = AssetType::Sprite;
                    subtype = if anim_plan.rows == 1 && anim_plan.columns > 1 {
                        "spritesheet_strip".into()
                    } else if anim_plan.columns > 1 || anim_plan.rows > 1 {
                        "spritesheet_grid".into()
                    } else {
                        String::new()
                    };
                    metadata = AssetMetadata::Sprite(SpriteMetadata {
                        frame_width: anim_plan.frame_width,
                        frame_height: anim_plan.frame_height,
                        columns: anim_plan.columns,
                        rows: anim_plan.rows,
                        frame_count: anim_plan.columns * anim_plan.rows,
                        animations: anim_plan.animations.clone(),
                        source_format: String::new(),
                    });
                }
                // Override tags
                for (path_prefix, extra_tags) in &plan.tags {
                    if file.path.starts_with(path_prefix) {
                        for t in extra_tags {
                            if !tags.contains(t) {
                                tags.push(t.clone());
                            }
                        }
                    }
                }
            }

            // Generate thumbnail for images
            let thumb_path = if matches!(asset_type, AssetType::Sprite | AssetType::Tileset | AssetType::Background | AssetType::Icon | AssetType::Gui | AssetType::Prop | AssetType::Vfx) {
                let thumb_name = format!("{}.png", file.path.replace('/', "_").replace(' ', "_"));
                let thumb_full = thumb_dir.join(&thumb_name);
                let source = pack_dir.join(&file.path);

                // Generate thumbnail: only crop first frame if it's actually a multi-frame sheet
                let result = if let AssetMetadata::Sprite(ref sm) = metadata {
                    if sm.frame_count > 1 && sm.columns > 1 {
                        thumbnail::generate_spritesheet_thumbnail(&source, &thumb_full, sm.frame_width, sm.frame_height)
                    } else {
                        thumbnail::generate_thumbnail(&source, &thumb_full)
                    }
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

    /// Find an asset by its unique ID.
    pub fn by_id(&self, id: &str) -> Option<&ToileAsset> {
        self.assets.iter().find(|a| a.id == id)
    }

    /// Search assets filtered by type.
    pub fn search_typed(&self, query: &str, asset_type: AssetType) -> Vec<&ToileAsset> {
        let lower = query.to_lowercase();
        self.assets.iter().filter(|a| {
            a.asset_type == asset_type && (
                a.name.to_lowercase().contains(&lower)
                || a.path.to_lowercase().contains(&lower)
                || a.tags.iter().any(|t| t.contains(&lower))
            )
        }).collect()
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
                let from_name = heuristics::frame_size_from_filename(&file.path);
                let (fw, fh, cols, rows) = if let Some((fw, fh)) = from_name {
                    let c = if fw > 0 { w / fw } else { 1 };
                    let r = if fh > 0 { h / fh } else { 1 };
                    (fw, fh, c, r)
                } else if heuristics::is_horizontal_strip(w, h) {
                    // Horizontal strip: height = frame size
                    let fw = h;
                    let fh = h;
                    let c = w / fw.max(1);
                    (fw, fh, c, 1)
                } else {
                    let (fw, fh, c, r) = heuristics::detect_sprite_grid(w, h);
                    // If grid detection returns 1×1 or the image is small, treat as single sprite
                    if c <= 1 && r <= 1 {
                        (w, h, 1, 1)
                    } else {
                        (fw, fh, c, r)
                    }
                };

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
