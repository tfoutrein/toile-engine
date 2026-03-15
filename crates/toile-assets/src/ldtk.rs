//! LDtk (.ldtk) level importer.
//!
//! Parses the LDtk JSON format and converts it into Toile's scene data model.
//! Each LDtk Level becomes a `SceneData` with entities from Entity layers and
//! tilemaps from Tile/AutoLayer/IntGrid layers.

use std::collections::HashMap;
use std::path::Path;

use glam::Vec2;
use serde::Deserialize;
use toile_graphics::sprite_renderer::{DrawSprite, COLOR_WHITE};
use toile_graphics::texture::TextureHandle;
use toile_scene::{EntityData, SceneData, TilemapData, TilemapLayerData};


// ── LDtk JSON serde structs ─────────────────────────────────
// Fields are deserialized from JSON but not all are read directly in code.

#[allow(dead_code)]
#[derive(Deserialize)]
struct LdtkProject {
    #[serde(default)]
    levels: Vec<LdtkLevel>,
    #[serde(default)]
    defs: LdtkDefs,
}

#[allow(dead_code)]
#[derive(Deserialize, Default)]
struct LdtkDefs {
    #[serde(default)]
    tilesets: Vec<LdtkTilesetDef>,
    #[serde(default)]
    entities: Vec<LdtkEntityDef>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct LdtkTilesetDef {
    uid: i64,
    identifier: String,
    #[serde(default)]
    #[serde(rename = "relPath")]
    rel_path: Option<String>,
    #[serde(rename = "pxWid")]
    px_wid: u32,
    #[serde(rename = "pxHei")]
    px_hei: u32,
    #[serde(rename = "tileGridSize")]
    tile_grid_size: u32,
    #[serde(rename = "__cWid")]
    c_wid: u32,
    #[serde(rename = "__cHei")]
    c_hei: u32,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct LdtkEntityDef {
    uid: i64,
    identifier: String,
    width: f32,
    height: f32,
}

#[derive(Deserialize)]
struct LdtkLevel {
    identifier: String,
    #[serde(rename = "worldX")]
    world_x: i32,
    #[serde(rename = "worldY")]
    world_y: i32,
    #[serde(rename = "pxWid")]
    px_wid: u32,
    #[serde(rename = "pxHei")]
    px_hei: u32,
    #[serde(rename = "layerInstances")]
    layer_instances: Option<Vec<LdtkLayerInstance>>,
}

#[derive(Deserialize)]
struct LdtkLayerInstance {
    #[serde(rename = "__type")]
    layer_type: String,
    #[serde(rename = "__identifier")]
    identifier: String,
    #[serde(rename = "__gridSize")]
    grid_size: u32,
    #[serde(rename = "__cWid")]
    c_wid: u32,
    #[serde(rename = "__cHei")]
    c_hei: u32,
    #[serde(rename = "__tilesetRelPath")]
    tileset_rel_path: Option<String>,
    #[serde(rename = "__tilesetDefUid")]
    tileset_def_uid: Option<i64>,
    #[serde(rename = "gridTiles", default)]
    grid_tiles: Vec<LdtkTile>,
    #[serde(rename = "autoLayerTiles", default)]
    auto_layer_tiles: Vec<LdtkTile>,
    #[serde(rename = "entityInstances", default)]
    entity_instances: Vec<LdtkEntityInstance>,
    #[serde(rename = "intGridCsv", default)]
    int_grid_csv: Vec<i32>,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
struct LdtkTile {
    /// Pixel position [x, y] in the layer
    px: [i32; 2],
    /// Source pixel position [x, y] in the tileset
    src: [i32; 2],
    /// Tile ID in the tileset
    #[serde(default)]
    t: u32,
    /// Flip flags: 0=none, 1=X, 2=Y, 3=both
    #[serde(default)]
    f: u8,
    /// Alpha (0.0–1.0)
    #[serde(default = "default_alpha")]
    a: f32,
}

fn default_alpha() -> f32 {
    1.0
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct LdtkEntityInstance {
    #[serde(rename = "__identifier")]
    identifier: String,
    /// Pixel position [x, y] relative to the level
    px: [i32; 2],
    width: f32,
    height: f32,
    #[serde(rename = "fieldInstances", default)]
    field_instances: Vec<LdtkFieldInstance>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct LdtkFieldInstance {
    #[serde(rename = "__identifier")]
    identifier: String,
    #[serde(rename = "__value")]
    value: serde_json::Value,
}

// ── Public result types ──────────────────────────────────────

/// A single imported LDtk level.
pub struct LdtkLevelResult {
    pub name: String,
    pub world_x: i32,
    pub world_y: i32,
    pub width: u32,
    pub height: u32,
    pub scene: SceneData,
}

/// Result of loading an LDtk project.
pub struct LdtkImportResult {
    pub levels: Vec<LdtkLevelResult>,
}

// ── Import function ──────────────────────────────────────────

/// Load an LDtk project file and convert each level to a `SceneData`.
///
/// Tile/AutoLayer layers are converted to `TilemapData`.
/// Entity layers are converted to `EntityData`.
/// IntGrid layers are converted to `TilemapData` (value as tile ID).
///
/// `load_texture_fn` resolves relative image paths to TextureHandles (for tilemap rendering).
pub fn load_ldtk(
    ldtk_path: &Path,
    _load_texture_fn: &mut dyn FnMut(&Path) -> TextureHandle,
) -> LdtkImportResult {
    let json_text = std::fs::read_to_string(ldtk_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", ldtk_path.display()));
    let project: LdtkProject =
        serde_json::from_str(&json_text).expect("Failed to parse LDtk JSON");
    let _ldtk_dir = ldtk_path.parent().unwrap_or(Path::new("."));

    // Build tileset lookup
    let tileset_defs: HashMap<i64, &LdtkTilesetDef> =
        project.defs.tilesets.iter().map(|ts| (ts.uid, ts)).collect();

    let mut levels = Vec::new();

    for level in &project.levels {
        let layers = match &level.layer_instances {
            Some(l) => l,
            None => continue, // External levels not loaded
        };

        let mut scene = SceneData::new(&level.identifier);
        let level_h = level.px_hei as f32;

        // Process layers (LDtk layers are ordered back-to-front in the array,
        // but the first element is the topmost layer in the editor)
        let mut tilemap_layers: Vec<TilemapLayerData> = Vec::new();
        let mut tileset_path = String::new();
        let mut tile_size = 0u32;
        let mut tileset_columns = 0u32;
        let mut map_width = 0u32;
        let mut map_height = 0u32;

        for layer in layers.iter().rev() {
            match layer.layer_type.as_str() {
                "Tiles" | "AutoLayer" => {
                    let tiles = if !layer.grid_tiles.is_empty() {
                        &layer.grid_tiles
                    } else {
                        &layer.auto_layer_tiles
                    };

                    if tiles.is_empty() {
                        continue;
                    }

                    // Resolve tileset info
                    if let Some(uid) = layer.tileset_def_uid {
                        if let Some(ts_def) = tileset_defs.get(&uid) {
                            if tileset_path.is_empty() {
                                if let Some(ref rel) = ts_def.rel_path {
                                    tileset_path = rel.clone();
                                }
                                tile_size = ts_def.tile_grid_size;
                                tileset_columns = ts_def.c_wid;
                            }
                        }
                    }

                    map_width = layer.c_wid;
                    map_height = layer.c_hei;

                    // Convert tiles to row-major grid
                    let grid_size = layer.grid_size;
                    let mut tile_data = vec![0u32; (layer.c_wid * layer.c_hei) as usize];

                    for tile in tiles {
                        let col = tile.px[0] as u32 / grid_size;
                        let row = tile.px[1] as u32 / grid_size;
                        if col < layer.c_wid && row < layer.c_hei {
                            // Store tile ID + 1 (0 = empty)
                            tile_data[(row * layer.c_wid + col) as usize] = tile.t + 1;
                        }
                    }

                    tilemap_layers.push(TilemapLayerData {
                        name: layer.identifier.clone(),
                        tiles: tile_data,
                        visible: true,
                    });
                }

                "IntGrid" => {
                    // IntGrid stores integer values per cell — treat as collision tiles
                    if layer.int_grid_csv.is_empty() {
                        continue;
                    }

                    map_width = layer.c_wid;
                    map_height = layer.c_hei;

                    // Also check for auto-layer tiles on this IntGrid layer
                    if !layer.auto_layer_tiles.is_empty() {
                        if let Some(uid) = layer.tileset_def_uid {
                            if let Some(ts_def) = tileset_defs.get(&uid) {
                                if tileset_path.is_empty() {
                                    if let Some(ref rel) = ts_def.rel_path {
                                        tileset_path = rel.clone();
                                    }
                                    tile_size = ts_def.tile_grid_size;
                                    tileset_columns = ts_def.c_wid;
                                }
                            }
                        }

                        let grid_size = layer.grid_size;
                        let mut tile_data = vec![0u32; (layer.c_wid * layer.c_hei) as usize];
                        for tile in &layer.auto_layer_tiles {
                            let col = tile.px[0] as u32 / grid_size;
                            let row = tile.px[1] as u32 / grid_size;
                            if col < layer.c_wid && row < layer.c_hei {
                                tile_data[(row * layer.c_wid + col) as usize] = tile.t + 1;
                            }
                        }
                        tilemap_layers.push(TilemapLayerData {
                            name: format!("{}_auto", layer.identifier),
                            tiles: tile_data,
                            visible: true,
                        });
                    }

                    // Store IntGrid values as a separate layer
                    let tile_data: Vec<u32> = layer
                        .int_grid_csv
                        .iter()
                        .map(|&v| if v > 0 { v as u32 } else { 0 })
                        .collect();

                    tilemap_layers.push(TilemapLayerData {
                        name: format!("{}_intgrid", layer.identifier),
                        tiles: tile_data,
                        visible: true,
                    });

                    if tile_size == 0 {
                        tile_size = layer.grid_size;
                    }
                }

                "Entities" => {
                    for ent in &layer.entity_instances {
                        let id = scene.next_id;
                        scene.next_id += 1;

                        // LDtk Y is top-down, convert to Y-up
                        let x = ent.px[0] as f32 + ent.width * 0.5;
                        let y = level_h - (ent.px[1] as f32 + ent.height * 0.5);

                        scene.entities.push(EntityData {
                            id,
                            name: ent.identifier.clone(),
                            x,
                            y,
                            rotation: 0.0,
                            scale_x: 1.0,
                            scale_y: 1.0,
                            layer: 0,
                            sprite_path: String::new(),
                            width: ent.width,
                            height: ent.height,
                        });
                    }
                }

                _ => {}
            }
        }

        // Build tilemap if we have layers
        if !tilemap_layers.is_empty() && tile_size > 0 {
            scene.tilemap = Some(TilemapData {
                tileset_path,
                tile_size,
                columns: tileset_columns,
                width: map_width,
                height: map_height,
                layers: tilemap_layers,
            });
        }

        levels.push(LdtkLevelResult {
            name: level.identifier.clone(),
            world_x: level.world_x,
            world_y: level.world_y,
            width: level.px_wid,
            height: level.px_hei,
            scene,
        });
    }

    LdtkImportResult { levels }
}

/// Convert an LDtk level's tile layers into renderable DrawSprite lists.
///
/// This is analogous to `build_tile_sprites` for Tiled maps.
/// Requires that the tileset texture has been loaded.
pub fn build_ldtk_tile_sprites(
    scene: &SceneData,
    tileset_texture: TextureHandle,
    tileset_px_wid: u32,
    tileset_px_hei: u32,
    base_layer: i32,
) -> Vec<Vec<DrawSprite>> {
    let tilemap = match &scene.tilemap {
        Some(tm) => tm,
        None => return Vec::new(),
    };

    let tw = tilemap.tile_size as f32;
    let th = tilemap.tile_size as f32;
    let map_h = (tilemap.height * tilemap.tile_size) as f32;
    let cols = if tileset_px_wid > 0 {
        tileset_px_wid / tilemap.tile_size
    } else {
        tilemap.columns
    };

    let mut result = Vec::new();

    for (layer_idx, layer) in tilemap.layers.iter().enumerate() {
        if !layer.visible {
            result.push(Vec::new());
            continue;
        }

        let mut sprites = Vec::new();

        for row in 0..tilemap.height {
            for col in 0..tilemap.width {
                let tile_id = layer.tiles[(row * tilemap.width + col) as usize];
                if tile_id == 0 {
                    continue;
                }

                // tile_id is stored as (original + 1), so subtract 1
                let local_id = tile_id - 1;
                let src_col = local_id % cols;
                let src_row = local_id / cols;

                let uv_min = Vec2::new(
                    (src_col * tilemap.tile_size) as f32 / tileset_px_wid as f32,
                    (src_row * tilemap.tile_size) as f32 / tileset_px_hei as f32,
                );
                let uv_max = Vec2::new(
                    ((src_col + 1) * tilemap.tile_size) as f32 / tileset_px_wid as f32,
                    ((src_row + 1) * tilemap.tile_size) as f32 / tileset_px_hei as f32,
                );

                // Center of tile, Y-up
                let x = col as f32 * tw + tw * 0.5;
                let y = map_h - (row as f32 * th + th * 0.5);

                sprites.push(DrawSprite {
                    texture: tileset_texture,
                    position: Vec2::new(x, y),
                    size: Vec2::new(tw, th),
                    rotation: 0.0,
                    color: COLOR_WHITE,
                    layer: base_layer + layer_idx as i32,
                    uv_min,
                    uv_max,
                });
            }
        }

        result.push(sprites);
    }

    result
}

// ── Convenience: load LDtk to SceneData only (no textures) ──

/// Load an LDtk file and return SceneData for each level, without loading textures.
/// Useful for CLI tools and MCP where rendering is not needed.
pub fn load_ldtk_scenes(ldtk_path: &Path) -> Result<Vec<LdtkLevelResult>, String> {
    let json_text =
        std::fs::read_to_string(ldtk_path).map_err(|e| format!("IO error: {e}"))?;
    let project: LdtkProject =
        serde_json::from_str(&json_text).map_err(|e| format!("JSON error: {e}"))?;

    let mut levels = Vec::new();

    for level in &project.levels {
        let layers = match &level.layer_instances {
            Some(l) => l,
            None => continue,
        };

        let mut scene = SceneData::new(&level.identifier);
        let level_h = level.px_hei as f32;

        let mut tilemap_layers: Vec<TilemapLayerData> = Vec::new();
        let mut tileset_path = String::new();
        let mut tile_size = 0u32;
        let tileset_columns = 0u32;
        let mut map_width = 0u32;
        let mut map_height = 0u32;

        for layer in layers.iter().rev() {
            match layer.layer_type.as_str() {
                "Tiles" | "AutoLayer" => {
                    let tiles = if !layer.grid_tiles.is_empty() {
                        &layer.grid_tiles
                    } else {
                        &layer.auto_layer_tiles
                    };
                    if tiles.is_empty() {
                        continue;
                    }

                    if let Some(ref rel) = layer.tileset_rel_path {
                        if tileset_path.is_empty() {
                            tileset_path = rel.clone();
                        }
                    }

                    map_width = layer.c_wid;
                    map_height = layer.c_hei;
                    if tile_size == 0 {
                        tile_size = layer.grid_size;
                    }

                    let grid_size = layer.grid_size;
                    let mut tile_data = vec![0u32; (layer.c_wid * layer.c_hei) as usize];
                    for tile in tiles {
                        let col = tile.px[0] as u32 / grid_size;
                        let row = tile.px[1] as u32 / grid_size;
                        if col < layer.c_wid && row < layer.c_hei {
                            tile_data[(row * layer.c_wid + col) as usize] = tile.t + 1;
                        }
                    }

                    tilemap_layers.push(TilemapLayerData {
                        name: layer.identifier.clone(),
                        tiles: tile_data,
                        visible: true,
                    });
                }

                "IntGrid" => {
                    if layer.int_grid_csv.is_empty() {
                        continue;
                    }
                    map_width = layer.c_wid;
                    map_height = layer.c_hei;
                    if tile_size == 0 {
                        tile_size = layer.grid_size;
                    }

                    if !layer.auto_layer_tiles.is_empty() {
                        if let Some(ref rel) = layer.tileset_rel_path {
                            if tileset_path.is_empty() {
                                tileset_path = rel.clone();
                            }
                        }

                        let grid_size = layer.grid_size;
                        let mut tile_data = vec![0u32; (layer.c_wid * layer.c_hei) as usize];
                        for tile in &layer.auto_layer_tiles {
                            let col = tile.px[0] as u32 / grid_size;
                            let row = tile.px[1] as u32 / grid_size;
                            if col < layer.c_wid && row < layer.c_hei {
                                tile_data[(row * layer.c_wid + col) as usize] = tile.t + 1;
                            }
                        }
                        tilemap_layers.push(TilemapLayerData {
                            name: format!("{}_auto", layer.identifier),
                            tiles: tile_data,
                            visible: true,
                        });
                    }

                    let tile_data: Vec<u32> = layer
                        .int_grid_csv
                        .iter()
                        .map(|&v| if v > 0 { v as u32 } else { 0 })
                        .collect();
                    tilemap_layers.push(TilemapLayerData {
                        name: format!("{}_intgrid", layer.identifier),
                        tiles: tile_data,
                        visible: true,
                    });
                }

                "Entities" => {
                    for ent in &layer.entity_instances {
                        let id = scene.next_id;
                        scene.next_id += 1;
                        let x = ent.px[0] as f32 + ent.width * 0.5;
                        let y = level_h - (ent.px[1] as f32 + ent.height * 0.5);

                        scene.entities.push(EntityData {
                            id,
                            name: ent.identifier.clone(),
                            x,
                            y,
                            rotation: 0.0,
                            scale_x: 1.0,
                            scale_y: 1.0,
                            layer: 0,
                            sprite_path: String::new(),
                            width: ent.width,
                            height: ent.height,
                        });
                    }
                }

                _ => {}
            }
        }

        if !tilemap_layers.is_empty() && tile_size > 0 {
            scene.tilemap = Some(TilemapData {
                tileset_path,
                tile_size,
                columns: tileset_columns,
                width: map_width,
                height: map_height,
                layers: tilemap_layers,
            });
        }

        levels.push(LdtkLevelResult {
            name: level.identifier.clone(),
            world_x: level.world_x,
            world_y: level.world_y,
            width: level.px_wid,
            height: level.px_hei,
            scene,
        });
    }

    Ok(levels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ldtk_json() -> &'static str {
        r#"{
            "levels": [
                {
                    "identifier": "Level_0",
                    "worldX": 0,
                    "worldY": 0,
                    "pxWid": 256,
                    "pxHei": 256,
                    "layerInstances": [
                        {
                            "__type": "Entities",
                            "__identifier": "Entities",
                            "__gridSize": 16,
                            "__cWid": 16,
                            "__cHei": 16,
                            "__tilesetRelPath": null,
                            "__tilesetDefUid": null,
                            "gridTiles": [],
                            "autoLayerTiles": [],
                            "intGridCsv": [],
                            "entityInstances": [
                                {
                                    "__identifier": "Player",
                                    "px": [128, 200],
                                    "width": 16,
                                    "height": 16,
                                    "fieldInstances": []
                                },
                                {
                                    "__identifier": "Enemy",
                                    "px": [64, 200],
                                    "width": 16,
                                    "height": 16,
                                    "fieldInstances": [
                                        {
                                            "__identifier": "health",
                                            "__value": 3
                                        }
                                    ]
                                }
                            ]
                        },
                        {
                            "__type": "IntGrid",
                            "__identifier": "Collision",
                            "__gridSize": 16,
                            "__cWid": 4,
                            "__cHei": 4,
                            "__tilesetRelPath": null,
                            "__tilesetDefUid": null,
                            "gridTiles": [],
                            "autoLayerTiles": [],
                            "intGridCsv": [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1],
                            "entityInstances": []
                        },
                        {
                            "__type": "Tiles",
                            "__identifier": "Background",
                            "__gridSize": 16,
                            "__cWid": 4,
                            "__cHei": 4,
                            "__tilesetRelPath": "tileset.png",
                            "__tilesetDefUid": 1,
                            "gridTiles": [
                                { "px": [0, 48], "src": [0, 0], "t": 0, "f": 0, "a": 1.0 },
                                { "px": [16, 48], "src": [16, 0], "t": 1, "f": 0, "a": 1.0 },
                                { "px": [32, 48], "src": [0, 0], "t": 0, "f": 0, "a": 1.0 },
                                { "px": [48, 48], "src": [16, 0], "t": 1, "f": 0, "a": 1.0 }
                            ],
                            "autoLayerTiles": [],
                            "intGridCsv": [],
                            "entityInstances": []
                        }
                    ]
                }
            ],
            "defs": {
                "tilesets": [
                    {
                        "uid": 1,
                        "identifier": "Tileset",
                        "relPath": "tileset.png",
                        "pxWid": 64,
                        "pxHei": 64,
                        "tileGridSize": 16,
                        "__cWid": 4,
                        "__cHei": 4
                    }
                ],
                "entities": [
                    { "uid": 10, "identifier": "Player", "width": 16, "height": 16 },
                    { "uid": 11, "identifier": "Enemy", "width": 16, "height": 16 }
                ]
            }
        }"#
    }

    #[test]
    fn parse_entities() {
        let tmp = std::env::temp_dir().join("test_ldtk_entities.ldtk");
        std::fs::write(&tmp, sample_ldtk_json()).unwrap();

        let result = load_ldtk_scenes(&tmp).unwrap();
        assert_eq!(result.len(), 1);

        let level = &result[0];
        assert_eq!(level.name, "Level_0");
        assert_eq!(level.world_x, 0);
        assert_eq!(level.width, 256);

        // Check entities (Y is flipped: level height is 256)
        let entities = &level.scene.entities;
        assert_eq!(entities.len(), 2);

        let player = entities.iter().find(|e| e.name == "Player").unwrap();
        assert_eq!(player.width, 16.0);
        // px=[128, 200] → x=128+8=136, y=256-(200+8)=48
        assert!((player.x - 136.0).abs() < 0.01);
        assert!((player.y - 48.0).abs() < 0.01);

        let enemy = entities.iter().find(|e| e.name == "Enemy").unwrap();
        assert!((enemy.x - 72.0).abs() < 0.01);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn parse_intgrid() {
        let tmp = std::env::temp_dir().join("test_ldtk_intgrid.ldtk");
        std::fs::write(&tmp, sample_ldtk_json()).unwrap();

        let result = load_ldtk_scenes(&tmp).unwrap();
        let level = &result[0];
        let tilemap = level.scene.tilemap.as_ref().unwrap();

        // Should have IntGrid layer + Tiles layer
        let intgrid = tilemap.layers.iter().find(|l| l.name.contains("intgrid")).unwrap();
        assert_eq!(intgrid.tiles.len(), 16); // 4x4
        // Bottom row should be solid (1)
        assert_eq!(intgrid.tiles[12], 1);
        assert_eq!(intgrid.tiles[13], 1);
        assert_eq!(intgrid.tiles[14], 1);
        assert_eq!(intgrid.tiles[15], 1);
        // Top row should be empty
        assert_eq!(intgrid.tiles[0], 0);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn parse_tile_layer() {
        let tmp = std::env::temp_dir().join("test_ldtk_tiles.ldtk");
        std::fs::write(&tmp, sample_ldtk_json()).unwrap();

        let result = load_ldtk_scenes(&tmp).unwrap();
        let level = &result[0];
        let tilemap = level.scene.tilemap.as_ref().unwrap();

        let bg = tilemap.layers.iter().find(|l| l.name == "Background").unwrap();
        assert_eq!(bg.tiles.len(), 16); // 4x4

        // Row 3 (bottom): tiles at all 4 columns
        assert_eq!(bg.tiles[12], 1); // t=0 → stored as 1
        assert_eq!(bg.tiles[13], 2); // t=1 → stored as 2
        assert_eq!(bg.tiles[14], 1);
        assert_eq!(bg.tiles[15], 2);

        // Other rows: empty
        assert_eq!(bg.tiles[0], 0);

        assert_eq!(tilemap.tileset_path, "tileset.png");
        assert_eq!(tilemap.tile_size, 16);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn multi_level() {
        let json = r#"{
            "levels": [
                {
                    "identifier": "Level_A",
                    "worldX": 0, "worldY": 0,
                    "pxWid": 160, "pxHei": 160,
                    "layerInstances": [
                        {
                            "__type": "Entities", "__identifier": "E",
                            "__gridSize": 16, "__cWid": 10, "__cHei": 10,
                            "__tilesetRelPath": null, "__tilesetDefUid": null,
                            "gridTiles": [], "autoLayerTiles": [],
                            "intGridCsv": [],
                            "entityInstances": [
                                { "__identifier": "Spawn", "px": [0, 0], "width": 16, "height": 16, "fieldInstances": [] }
                            ]
                        }
                    ]
                },
                {
                    "identifier": "Level_B",
                    "worldX": 160, "worldY": 0,
                    "pxWid": 160, "pxHei": 160,
                    "layerInstances": [
                        {
                            "__type": "Entities", "__identifier": "E",
                            "__gridSize": 16, "__cWid": 10, "__cHei": 10,
                            "__tilesetRelPath": null, "__tilesetDefUid": null,
                            "gridTiles": [], "autoLayerTiles": [],
                            "intGridCsv": [],
                            "entityInstances": [
                                { "__identifier": "Exit", "px": [144, 144], "width": 16, "height": 16, "fieldInstances": [] }
                            ]
                        }
                    ]
                }
            ],
            "defs": { "tilesets": [], "entities": [] }
        }"#;

        let tmp = std::env::temp_dir().join("test_ldtk_multi.ldtk");
        std::fs::write(&tmp, json).unwrap();

        let result = load_ldtk_scenes(&tmp).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Level_A");
        assert_eq!(result[0].world_x, 0);
        assert_eq!(result[1].name, "Level_B");
        assert_eq!(result[1].world_x, 160);

        assert_eq!(result[0].scene.entities[0].name, "Spawn");
        assert_eq!(result[1].scene.entities[0].name, "Exit");

        std::fs::remove_file(&tmp).ok();
    }
}
