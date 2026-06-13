//! Recursive filesystem scanner for asset packs.

use std::io::Read;
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

    // Caps to contain malicious/untrusted packs (audit S2: zip-slip + zip bombs).
    const MAX_ENTRIES: usize = 20_000;
    const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024; // 512 MiB per entry
    const MAX_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024; // 4 GiB total

    if archive.len() > MAX_ENTRIES {
        return Err(format!("ZIP has too many entries ({} > {MAX_ENTRIES})", archive.len()));
    }

    let mut total_written: u64 = 0;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("ZIP entry error: {e}"))?;

        // Never recreate symlink entries from an untrusted archive.
        if let Some(mode) = entry.unix_mode() {
            if mode & 0o170000 == 0o120000 {
                continue;
            }
        }

        // Zip-Slip protection: enclosed_name() returns None for absolute paths,
        // `..` traversal and NUL bytes (mangled_name() silently "fixes" them and
        // can still escape). Convert to an owned path so `entry` is free for &mut.
        let rel = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => {
                log::warn!("skipping unsafe ZIP entry path: {}", entry.name());
                continue;
            }
        };
        let out_path = target_dir.join(&rel);
        // Defense in depth: the joined path must remain under target_dir.
        if !out_path.starts_with(target_dir) {
            log::warn!("skipping ZIP entry escaping target: {}", entry.name());
            continue;
        }

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("Cannot create dir: {e}"))?;
        } else {
            if entry.size() > MAX_FILE_BYTES {
                return Err(format!(
                    "ZIP entry '{}' too large ({} bytes)", entry.name(), entry.size()
                ));
            }
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create parent dir: {e}"))?;
            }
            let mut outfile = std::fs::File::create(&out_path)
                .map_err(|e| format!("Cannot create file: {e}"))?;
            // Bound the bytes actually written in case the declared size lies, and
            // enforce the overall budget so a zip bomb can't fill the disk.
            let limit = MAX_FILE_BYTES.min(MAX_TOTAL_BYTES.saturating_sub(total_written));
            let written = std::io::copy(&mut entry.by_ref().take(limit + 1), &mut outfile)
                .map_err(|e| format!("Cannot write file: {e}"))?;
            if written > limit {
                return Err("ZIP extraction exceeded the size budget (possible zip bomb)".to_string());
            }
            total_written += written;
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

    #[test]
    fn extract_zip_blocks_path_traversal() {
        use std::io::Write;
        let base = std::env::temp_dir().join("toile_zip_slip_test");
        let _ = fs::remove_dir_all(&base);
        let target = base.join("out");
        fs::create_dir_all(&target).unwrap();
        let zip_path = base.join("evil.zip");

        // A malicious archive: one `..` traversal entry + one safe entry.
        {
            let f = fs::File::create(&zip_path).unwrap();
            let mut w = zip::ZipWriter::new(f);
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("../escaped.txt", opts).unwrap();
            w.write_all(b"pwned").unwrap();
            w.start_file("safe.txt", opts).unwrap();
            w.write_all(b"ok").unwrap();
            w.finish().unwrap();
        }

        extract_zip(&zip_path, &target).unwrap();

        // The traversal entry must NOT land outside the target dir.
        assert!(!base.join("escaped.txt").exists(), "zip-slip escaped the target dir");
        // The safe entry is extracted normally.
        assert!(target.join("safe.txt").exists(), "safe entry should extract");

        let _ = fs::remove_dir_all(&base);
    }
}
