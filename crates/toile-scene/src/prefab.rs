use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::EntityData;

/// A reusable entity template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefab {
    pub name: String,
    pub entity: EntityData,
    #[serde(default)]
    pub behaviors: Vec<serde_json::Value>,
    #[serde(default)]
    pub event_sheet: Option<String>,
}

impl Prefab {
    /// Create a prefab from an entity.
    pub fn from_entity(name: &str, entity: &EntityData) -> Self {
        Self {
            name: name.to_string(),
            entity: entity.clone(),
            behaviors: Vec::new(),
            event_sheet: None,
        }
    }

    /// Create an instance of this prefab with a new ID and optional overrides.
    pub fn instantiate(&self, id: u64, overrides: &HashMap<String, serde_json::Value>) -> EntityData {
        let mut entity = self.entity.clone();
        entity.id = id;
        entity.name = format!("{}_{}", self.name, id);

        // Apply overrides
        if let Some(v) = overrides.get("name") {
            if let Some(s) = v.as_str() {
                entity.name = s.to_string();
            }
        }
        if let Some(v) = overrides.get("x") {
            if let Some(f) = v.as_f64() {
                entity.x = f as f32;
            }
        }
        if let Some(v) = overrides.get("y") {
            if let Some(f) = v.as_f64() {
                entity.y = f as f32;
            }
        }
        if let Some(v) = overrides.get("width") {
            if let Some(f) = v.as_f64() {
                entity.width = f as f32;
            }
        }
        if let Some(v) = overrides.get("height") {
            if let Some(f) = v.as_f64() {
                entity.height = f as f32;
            }
        }
        if let Some(v) = overrides.get("rotation") {
            if let Some(f) = v.as_f64() {
                entity.rotation = f as f32;
            }
        }
        if let Some(v) = overrides.get("layer") {
            if let Some(i) = v.as_i64() {
                entity.layer = i as i32;
            }
        }

        entity
    }
}

/// Error type for prefab operations.
#[derive(Debug, thiserror::Error)]
pub enum PrefabError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Prefab not found: {0}")]
    NotFound(String),
}

/// Load a prefab from a JSON file.
pub fn load_prefab(path: &Path) -> Result<Prefab, PrefabError> {
    let json = std::fs::read_to_string(path)?;
    let prefab: Prefab = serde_json::from_str(&json)?;
    Ok(prefab)
}

/// Save a prefab to a JSON file.
pub fn save_prefab(path: &Path, prefab: &Prefab) -> Result<(), PrefabError> {
    let json = serde_json::to_string_pretty(prefab)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// List all .prefab.json files in a directory.
pub fn list_prefabs(dir: &Path) -> Result<Vec<PathBuf>, PrefabError> {
    let mut prefabs = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.ends_with(".prefab.json") {
                prefabs.push(path);
            }
        }
    }
    prefabs.sort();
    Ok(prefabs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_instantiate() {
        let entity = EntityData {
            id: 0,
            name: "Enemy".into(),
            x: 100.0,
            y: 200.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            layer: 0,
            sprite_path: "enemy.png".into(),
            width: 32.0,
            height: 32.0,
        };

        let prefab = Prefab::from_entity("Enemy", &entity);
        assert_eq!(prefab.name, "Enemy");

        // Instantiate with position override
        let mut overrides = HashMap::new();
        overrides.insert("x".into(), serde_json::json!(300.0));
        overrides.insert("y".into(), serde_json::json!(50.0));

        let instance = prefab.instantiate(42, &overrides);
        assert_eq!(instance.id, 42);
        assert_eq!(instance.x, 300.0);
        assert_eq!(instance.y, 50.0);
        assert_eq!(instance.width, 32.0); // inherited from prefab
    }

    #[test]
    fn serialization_roundtrip() {
        let entity = EntityData {
            id: 0, name: "Coin".into(),
            x: 0.0, y: 0.0, rotation: 0.0,
            scale_x: 1.0, scale_y: 1.0, layer: 0,
            sprite_path: "coin.png".into(),
            width: 16.0, height: 16.0,
        };
        let prefab = Prefab::from_entity("Coin", &entity);

        let json = serde_json::to_string(&prefab).unwrap();
        let loaded: Prefab = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.name, "Coin");
        assert_eq!(loaded.entity.width, 16.0);
    }
}
