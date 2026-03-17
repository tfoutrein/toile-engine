//! File browser view — tree view of pack directories with README display.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::AssetBrowserApp;

/// Render the file browser view.
pub fn show_file_browser(
    app: &mut AssetBrowserApp,
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
) {
    // View mode toggle (same as browser panel top bar)
    ui.horizontal(|ui| {
        if ui.selectable_label(false, "🖼 Assets").clicked() {
            app.view_mode = super::ViewMode::Assets;
        }
        if ui.selectable_label(true, "📂 Files").clicked() {
            // already in files mode
        }
    });
    ui.separator();

    if app.library.assets.is_empty() {
        ui.label(egui::RichText::new("No packs imported yet.").color(egui::Color32::from_gray(130)));
        return;
    }

    // Split view: tree on the left, readme/content on the right
    let readme = app.readme_content.clone();

    egui::SidePanel::left("file_tree_inner")
        .default_width(350.0)
        .min_width(250.0)
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Build file tree — filter by selected pack if any
                let packs = app.registry.packs.clone();
                for pack_reg in &packs {
                    let pack_id_str = pack_reg.name.replace(' ', "_").to_lowercase();

                    // Skip if a pack filter is active and this isn't the selected pack
                    if let Some(ref fp) = app.filter_pack {
                        if *fp != pack_id_str { continue; }
                    }

                    let pack_path = Path::new(&pack_reg.path);
                    if !pack_path.is_dir() { continue; }

                    let pack_ui_id = ui.make_persistent_id(&pack_reg.name);
                    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), pack_ui_id, true)
                        .show_header(ui, |ui| {
                            ui.label(egui::RichText::new(format!("📦 {}", pack_reg.name)).strong());
                        })
                        .body(|ui| {
                            show_directory_tree(ui, pack_path, pack_path, app);
                        });
                }
            });
        });

    // Right side: readme content or file info
    if let Some((filename, content)) = &readme {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(filename);
            ui.separator();
            ui.label(egui::RichText::new(content).monospace().size(12.0));
        });
    } else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(egui::RichText::new("Click a README, LICENSE, or text file to view it here.")
                .color(egui::Color32::from_gray(130)));
        });
    }
}

/// Recursively render a directory tree.
fn show_directory_tree(
    ui: &mut egui::Ui,
    dir: &Path,
    pack_root: &Path,
    app: &mut AssetBrowserApp,
) {
    // Collect entries sorted: directories first, then files
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut dirs = BTreeSet::new();
    let mut files = BTreeMap::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files and .toile cache
        if name.starts_with('.') { continue; }

        if path.is_dir() {
            dirs.insert(name);
        } else {
            files.insert(name, path);
        }
    }

    // Directories
    for dir_name in &dirs {
        let subdir = dir.join(dir_name);
        let dir_id = ui.make_persistent_id(subdir.to_string_lossy().as_ref());
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), dir_id, false)
            .show_header(ui, |ui| {
                ui.label(format!("📁 {dir_name}"));
            })
            .body(|ui| {
                show_directory_tree(ui, &subdir, pack_root, app);
            });
    }

    // Files
    for (name, path) in &files {
        let icon = file_icon(&name);
        let is_readme = is_text_file(&name);
        let label = if is_readme {
            egui::RichText::new(format!("{icon} {name}")).color(egui::Color32::from_rgb(100, 200, 255))
        } else {
            egui::RichText::new(format!("{icon} {name}")).size(11.0)
        };

        if ui.selectable_label(false, label).clicked() {
            if is_text_file(&name) {
                // Load and display text file content
                if let Ok(content) = std::fs::read_to_string(path) {
                    app.readme_content = Some((name.clone(), content));
                }
            } else {
                // For non-text files, show basic info
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                let rel = path.strip_prefix(pack_root)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| name.clone());
                let info = format!(
                    "File: {}\nPath: {}\nSize: {}",
                    name, rel, format_size(size)
                );
                app.readme_content = Some((name.clone(), info));

                // Select as asset if it exists in the library — match by full relative path within the pack
                let pack_id = pack_root.file_name()
                    .map(|n| n.to_string_lossy().replace(' ', "_").to_lowercase())
                    .unwrap_or_default();
                if let Some(asset) = app.library.assets.iter().find(|a| {
                    a.pack_id == pack_id && a.path == rel
                }) {
                    app.selected_asset = Some(asset.id.clone());
                    app.preview_loaded_path.clear(); // force preview reload
                }
            }
        }
    }
}

fn file_icon(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".bmp") {
        "🖼"
    } else if lower.ends_with(".wav") || lower.ends_with(".ogg") || lower.ends_with(".mp3") {
        "🔊"
    } else if lower.ends_with(".ttf") || lower.ends_with(".otf") || lower.ends_with(".fnt") {
        "🔤"
    } else if lower.ends_with(".tmx") || lower.ends_with(".tmj") || lower.ends_with(".tsx") || lower.ends_with(".tsj") {
        "🗺"
    } else if lower.ends_with(".ldtk") {
        "🗺"
    } else if lower.ends_with(".aseprite") || lower.ends_with(".ase") {
        "✨"
    } else if lower.ends_with(".json") || lower.ends_with(".xml") {
        "📋"
    } else if is_text_file(name) {
        "📝"
    } else if lower.ends_with(".zip") {
        "📦"
    } else {
        "📄"
    }
}

fn is_text_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".txt") || lower.ends_with(".md") || lower.ends_with(".readme")
        || lower.ends_with(".json") || lower.ends_with(".xml") || lower.ends_with(".toml")
        || lower.ends_with(".yaml") || lower.ends_with(".yml") || lower.ends_with(".csv")
        || lower.ends_with(".tsx") || lower.ends_with(".tsj") || lower.ends_with(".tmj")
        || lower.ends_with(".fnt") || lower.ends_with(".plist")
        || lower == "readme" || lower == "license" || lower == "credits"
        || lower.contains("readme") || lower.contains("license") || lower.contains("credits")
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{bytes} B") }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
}
