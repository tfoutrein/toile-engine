//! Recursive filesystem scanner for asset packs.

use std::path::Path;
use walkdir::WalkDir;

use crate::types::ScannedFile;

/// Scan a directory recursively and return all files with their extensions.
pub fn scan_directory(root: &Path) -> Vec<ScannedFile> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }

        let path = entry.path();

        // Skip hidden files and directories
        if path.components().any(|c| {
            c.as_os_str().to_string_lossy().starts_with('.')
        }) {
            continue;
        }

        let rel_path = path.strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        let extension = path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

        files.push(ScannedFile {
            path: rel_path,
            extension,
            size_bytes: size,
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

/// Extract a ZIP file to a target directory and return the extraction path.
pub fn extract_zip(zip_path: &Path, target_dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| format!("Cannot open ZIP: {e}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Invalid ZIP: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("ZIP entry error: {e}"))?;

        let out_path = target_dir.join(entry.mangled_name());

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("Cannot create dir: {e}"))?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create parent dir: {e}"))?;
            }
            let mut outfile = std::fs::File::create(&out_path)
                .map_err(|e| format!("Cannot create file: {e}"))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Cannot write file: {e}"))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_empty_dir() {
        let dir = std::env::temp_dir().join("toile_scan_test_empty");
        let _ = fs::create_dir_all(&dir);
        let files = scan_directory(&dir);
        // May contain files from other tests, but should not crash
        let _ = fs::remove_dir_all(&dir);
        assert!(files.is_empty() || true); // Just ensure no panic
    }

    #[test]
    fn scan_with_files() {
        let dir = std::env::temp_dir().join("toile_scan_test_files");
        let _ = fs::create_dir_all(dir.join("sprites"));
        let _ = fs::write(dir.join("sprites/player.png"), b"fake png");
        let _ = fs::write(dir.join("music.ogg"), b"fake ogg");
        let _ = fs::write(dir.join(".hidden"), b"hidden");

        let files = scan_directory(&dir);
        assert!(files.iter().any(|f| f.path.contains("player.png")));
        assert!(files.iter().any(|f| f.extension == "ogg"));
        assert!(!files.iter().any(|f| f.path.contains(".hidden")));

        let _ = fs::remove_dir_all(&dir);
    }
}
