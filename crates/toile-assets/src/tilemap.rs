use std::collections::HashMap;
use std::path::Path;

use glam::Vec2;
use serde::Deserialize;
use toile_graphics::sprite_renderer::{DrawSprite, COLOR_WHITE};
use toile_graphics::texture::TextureHandle;

// --- GID flip flags ---

const FLIPPED_H: u32 = 0x80000000;
const FLIPPED_V: u32 = 0x40000000;
const FLIPPED_D: u32 = 0x20000000;
const GID_MASK: u32 = !(FLIPPED_H | FLIPPED_V | FLIPPED_D);

// --- Tiled JSON serde structs ---

#[derive(Deserialize)]
struct TiledMap {
    width: u32,
    height: u32,
    tilewidth: u32,
    tileheight: u32,
    layers: Vec<serde_json::Value>,
    tilesets: Vec<TiledTileset>,
}

#[derive(Deserialize)]
struct TiledTileset {
    firstgid: u32,
    #[serde(default)]
    tilecount: u32,
    #[serde(default)]
    columns: u32,
    #[serde(default)]
    imagewidth: u32,
    #[serde(default)]
    imageheight: u32,
    #[serde(default)]
    image: String,
}

#[derive(Deserialize)]
struct TiledTileLayer {
    name: String,
    data: Vec<u32>,
    width: u32,
    height: u32,
    #[serde(default = "default_true")]
    visible: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct TiledObjectLayer {
    name: String,
    objects: Vec<TiledObject>,
}

#[derive(Deserialize)]
struct TiledObject {
    #[serde(default)]
    id: u32,
    #[serde(default)]
    name: String,
    #[serde(rename = "type", default)]
    obj_type: String,
    x: f32,
    y: f32,
    #[serde(default)]
    width: f32,
    #[serde(default)]
    height: f32,
    #[serde(default)]
    properties: Vec<TiledProperty>,
}

#[derive(Deserialize)]
struct TiledProperty {
    name: String,
    value: serde_json::Value,
}

// --- Engine tilemap types ---

pub struct Tileset {
    pub texture: TextureHandle,
    pub firstgid: u32,
    pub columns: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub image_width: u32,
    pub image_height: u32,
}

impl Tileset {
    pub fn tile_uv(&self, local_id: u32) -> (Vec2, Vec2) {
        let col = local_id % self.columns;
        let row = local_id / self.columns;
        let x = col * self.tile_width;
        let y = row * self.tile_height;
        (
            Vec2::new(x as f32 / self.image_width as f32, y as f32 / self.image_height as f32),
            Vec2::new(
                (x + self.tile_width) as f32 / self.image_width as f32,
                (y + self.tile_height) as f32 / self.image_height as f32,
            ),
        )
    }
}

pub struct TileLayer {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub gids: Vec<u32>,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct MapObject {
    pub id: u32,
    pub name: String,
    pub obj_type: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub properties: HashMap<String, serde_json::Value>,
}

pub struct ObjectLayer {
    pub name: String,
    pub objects: Vec<MapObject>,
}

pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub tilesets: Vec<Tileset>,
    pub tile_layers: Vec<TileLayer>,
    pub object_layers: Vec<ObjectLayer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TilemapHandle(pub u32);

impl Tilemap {
    pub fn find_tileset(&self, gid: u32) -> &Tileset {
        self.tilesets
            .iter()
            .rev()
            .find(|ts| gid >= ts.firstgid)
            .expect("No tileset found for GID")
    }

    /// Map height in pixels.
    pub fn pixel_height(&self) -> f32 {
        (self.height * self.tile_height) as f32
    }

    /// Convert Tiled coordinates (top-left, Y-down) to engine (center, Y-up).
    pub fn tiled_to_engine(&self, x: f32, y: f32, w: f32, h: f32) -> Vec2 {
        Vec2::new(x + w * 0.5, self.pixel_height() - (y + h * 0.5))
    }
}

/// Load a Tiled JSON export. `load_texture_fn` resolves image paths to TextureHandles.
pub fn load_tiled_json(
    json_path: &Path,
    load_texture_fn: &mut dyn FnMut(&Path) -> TextureHandle,
) -> Tilemap {
    let json_text = std::fs::read_to_string(json_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", json_path.display()));
    let map: TiledMap = serde_json::from_str(&json_text).expect("Failed to parse Tiled JSON");
    let json_dir = json_path.parent().unwrap_or(Path::new("."));

    // Load tilesets
    let tilesets: Vec<Tileset> = map
        .tilesets
        .iter()
        .map(|ts| {
            let img_path = json_dir.join(&ts.image);
            let texture = load_texture_fn(&img_path);
            Tileset {
                texture,
                firstgid: ts.firstgid,
                columns: ts.columns,
                tile_width: map.tilewidth,
                tile_height: map.tileheight,
                image_width: ts.imagewidth,
                image_height: ts.imageheight,
            }
        })
        .collect();

    // Parse layers
    let mut tile_layers = Vec::new();
    let mut object_layers = Vec::new();

    for layer_val in &map.layers {
        let layer_type = layer_val
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match layer_type {
            "tilelayer" => {
                if let Ok(tl) = serde_json::from_value::<TiledTileLayer>(layer_val.clone()) {
                    tile_layers.push(TileLayer {
                        name: tl.name,
                        width: tl.width,
                        height: tl.height,
                        gids: tl.data,
                        visible: tl.visible,
                    });
                }
            }
            "objectgroup" => {
                if let Ok(ol) = serde_json::from_value::<TiledObjectLayer>(layer_val.clone()) {
                    let objects = ol
                        .objects
                        .into_iter()
                        .map(|o| {
                            let props = o
                                .properties
                                .into_iter()
                                .map(|p| (p.name, p.value))
                                .collect();
                            MapObject {
                                id: o.id,
                                name: o.name,
                                obj_type: o.obj_type,
                                x: o.x,
                                y: o.y,
                                width: o.width,
                                height: o.height,
                                properties: props,
                            }
                        })
                        .collect();
                    object_layers.push(ObjectLayer {
                        name: ol.name,
                        objects,
                    });
                }
            }
            _ => {}
        }
    }

    Tilemap {
        width: map.width,
        height: map.height,
        tile_width: map.tilewidth,
        tile_height: map.tileheight,
        tilesets,
        tile_layers,
        object_layers,
    }
}

/// Pre-build DrawSprite lists for efficient tilemap rendering.
pub fn build_tile_sprites(tilemap: &Tilemap, base_layer: i32) -> Vec<Vec<DrawSprite>> {
    let mut result = Vec::new();

    for (layer_idx, tile_layer) in tilemap.tile_layers.iter().enumerate() {
        if !tile_layer.visible {
            result.push(Vec::new());
            continue;
        }

        let tw = tilemap.tile_width as f32;
        let th = tilemap.tile_height as f32;
        let map_h = tilemap.pixel_height();

        let mut sprites = Vec::new();
        for row in 0..tile_layer.height {
            for col in 0..tile_layer.width {
                let raw_gid = tile_layer.gids[(row * tile_layer.width + col) as usize];
                let gid = raw_gid & GID_MASK;
                if gid == 0 {
                    continue;
                }

                let flip_h = raw_gid & FLIPPED_H != 0;
                let flip_v = raw_gid & FLIPPED_V != 0;

                let tileset = tilemap.find_tileset(gid);
                let local_id = gid - tileset.firstgid;
                let (mut uv_min, mut uv_max) = tileset.tile_uv(local_id);

                if flip_h {
                    std::mem::swap(&mut uv_min.x, &mut uv_max.x);
                }
                if flip_v {
                    std::mem::swap(&mut uv_min.y, &mut uv_max.y);
                }

                let x = col as f32 * tw + tw * 0.5;
                let y = map_h - (row as f32 * th + th * 0.5);

                sprites.push(DrawSprite {
                    texture: tileset.texture,
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
