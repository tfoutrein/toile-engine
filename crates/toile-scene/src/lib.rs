use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct SceneData {
    pub name: String,
    pub entities: Vec<EntityData>,
    #[serde(default)]
    pub tilemap: Option<TilemapData>,
    #[serde(skip)]
    pub next_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TilemapData {
    pub tileset_path: String,
    pub tile_size: u32,
    pub columns: u32,
    pub width: u32,
    pub height: u32,
    pub layers: Vec<TilemapLayerData>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TilemapLayerData {
    pub name: String,
    pub tiles: Vec<u32>, // row-major, 0 = empty
    pub visible: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntityData {
    pub id: u64,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub layer: i32,
    #[serde(default)]
    pub sprite_path: String,
    pub width: f32,
    pub height: f32,
}

impl SceneData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            entities: Vec::new(),
            tilemap: None,
            next_id: 1,
        }
    }

    pub fn add_entity(&mut self, name: &str, x: f32, y: f32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(EntityData {
            id,
            name: name.to_string(),
            x,
            y,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            layer: 0,
            sprite_path: String::new(),
            width: 32.0,
            height: 32.0,
        });
        id
    }

    pub fn remove_entity(&mut self, id: u64) {
        self.entities.retain(|e| e.id != id);
    }

    pub fn find_entity_mut(&mut self, id: u64) -> Option<&mut EntityData> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Recompute next_id from existing entities (needed after deserialization).
    pub fn fix_next_id(&mut self) {
        self.next_id = self.entities.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    }
}

/// Load a scene from a JSON file.
pub fn load_scene(path: &Path) -> Result<SceneData, SceneError> {
    let json = std::fs::read_to_string(path)?;
    let mut scene: SceneData = serde_json::from_str(&json)?;
    scene.fix_next_id();
    Ok(scene)
}

/// Save a scene to a JSON file.
pub fn save_scene(path: &Path, scene: &SceneData) -> Result<(), SceneError> {
    let json = serde_json::to_string_pretty(scene)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// List all .json scene files in a directory.
pub fn list_scene_files(dir: &Path) -> Result<Vec<PathBuf>, SceneError> {
    let mut scenes = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                scenes.push(path);
            }
        }
    }
    scenes.sort();
    Ok(scenes)
}
