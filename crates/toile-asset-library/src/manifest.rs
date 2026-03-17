//! Asset manifest read/write — toile-asset-manifest.json.

use std::path::Path;

use crate::types::AssetManifest;

const MANIFEST_VERSION: &str = "1.0";
const MANIFEST_FILENAME: &str = "toile-asset-manifest.json";

/// Generate the manifest filename path for a pack directory.
pub fn manifest_path(pack_dir: &Path) -> std::path::PathBuf {
    pack_dir.join(MANIFEST_FILENAME)
}

/// Save a manifest to disk.
pub fn save_manifest(path: &Path, manifest: &AssetManifest) -> Result<(), String> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("JSON serialize error: {e}"))?;
    std::fs::write(path, json)
        .map_err(|e| format!("Cannot write manifest: {e}"))?;
    Ok(())
}

/// Load a manifest from disk.
pub fn load_manifest(path: &Path) -> Result<AssetManifest, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read manifest: {e}"))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Invalid manifest JSON: {e}"))
}

/// Check if a manifest exists for a pack directory.
pub fn has_manifest(pack_dir: &Path) -> bool {
    manifest_path(pack_dir).exists()
}

/// Current manifest version.
pub fn current_version() -> &'static str {
    MANIFEST_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PackInfo, AssetManifest};

    #[test]
    fn roundtrip() {
        let manifest = AssetManifest {
            manifest_version: MANIFEST_VERSION.into(),
            pack: PackInfo {
                name: "Test Pack".into(),
                author: "Test".into(),
                license: "CC0".into(),
                source: "test".into(),
                import_date: "2026-03-17".into(),
                tags: vec!["test".into()],
            },
            assets: vec![],
        };

        let dir = std::env::temp_dir().join("toile_manifest_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = manifest_path(&dir);

        save_manifest(&path, &manifest).unwrap();
        let loaded = load_manifest(&path).unwrap();

        assert_eq!(loaded.pack.name, "Test Pack");
        assert_eq!(loaded.manifest_version, MANIFEST_VERSION);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
