//! Pack registry — remembers which packs have been imported across sessions.
//!
//! Stores a simple JSON file listing pack directories.
//! On startup, the library reloads all registered packs from their manifests.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

const REGISTRY_FILENAME: &str = "asset-library-registry.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRegistry {
    pub packs: Vec<RegisteredPack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredPack {
    pub name: String,
    pub path: String,
}

impl Default for PackRegistry {
    fn default() -> Self {
        Self { packs: Vec::new() }
    }
}

/// Get the registry file path (next to the executable or in a config dir).
pub fn registry_path() -> PathBuf {
    // Use current working directory for simplicity
    PathBuf::from(REGISTRY_FILENAME)
}

/// Load the registry from disk. Returns empty if not found.
pub fn load_registry() -> PackRegistry {
    let path = registry_path();
    if !path.exists() {
        return PackRegistry::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => PackRegistry::default(),
    }
}

/// Save the registry to disk.
pub fn save_registry(registry: &PackRegistry) {
    let path = registry_path();
    if let Ok(json) = serde_json::to_string_pretty(registry) {
        let _ = std::fs::write(path, json);
    }
}

/// Add a pack to the registry (if not already present).
pub fn register_pack(registry: &mut PackRegistry, name: &str, path: &Path) {
    let path_str = path.to_string_lossy().to_string();
    if !registry.packs.iter().any(|p| p.path == path_str) {
        registry.packs.push(RegisteredPack {
            name: name.into(),
            path: path_str,
        });
        save_registry(registry);
    }
}

/// Remove a pack from the registry by path.
pub fn unregister_pack(registry: &mut PackRegistry, path: &str) {
    registry.packs.retain(|p| p.path != path);
    save_registry(registry);
}
