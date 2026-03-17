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

    // Right side: readme content or file info + image preview
    if let Some((filename, content)) = &readme {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(filename);
            ui.separator();
            // Show image preview if available
            if let Some(ref tex) = app.preview_texture {
                let lower = filename.to_lowercase();
                if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".bmp") {
                    let max_w = ui.available_width().min(400.0);
                    let aspect = tex.size()[1] as f32 / tex.size()[0] as f32;
                    let size = egui::vec2(max_w, max_w * aspect);
                    ui.image(egui::load::SizedTexture::new(tex.id(), size));
                    ui.add_space(8.0);
                }
            }
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

/// Check if a directory contains the highlighted file (for auto-expand).
fn dir_contains_highlight(dir: &Path, pack_root: &Path, highlight: &Option<String>) -> bool {
    if let Some(hl) = highlight {
        let full_hl = pack_root.join(hl);
        full_hl.starts_with(dir)
    } else {
        false
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
        let should_open = dir_contains_highlight(&subdir, pack_root, &app.highlight_file);
        let dir_id = ui.make_persistent_id(subdir.to_string_lossy().as_ref());
        let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), dir_id, false);
        if should_open {
            state.set_open(true);
        }
        state.show_header(ui, |ui| {
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
        let rel = path.strip_prefix(pack_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| name.clone());
        let is_highlighted = app.highlight_file.as_deref() == Some(&rel);

        let label = if is_highlighted {
            egui::RichText::new(format!("{icon} {name}")).strong().color(egui::Color32::YELLOW)
        } else if is_readme {
            egui::RichText::new(format!("{icon} {name}")).color(egui::Color32::from_rgb(100, 200, 255))
        } else {
            egui::RichText::new(format!("{icon} {name}")).size(11.0)
        };

        let response = ui.selectable_label(is_highlighted, label);
        // Scroll to highlighted file
        if is_highlighted {
            response.scroll_to_me(Some(egui::Align::Center));
            // Clear highlight after first display to avoid sticky state
        }
        if response.clicked() {
            app.highlight_file = Some(rel.clone());
            if is_aseprite_file(&name) {
                // Parse .aseprite and show structured info
                match toile_assets::aseprite::load_ase_file(path) {
                    Ok(ase) => {
                        let mut info = format!(
                            "Aseprite File: {}\n\nSize: {}×{} px\nFrames: {}\nColor depth: {} bits\nLayers: {}\n",
                            name, ase.width, ase.height, ase.frames.len(), ase.color_depth, ase.layers.len()
                        );
                        info.push_str("\n── Layers ──\n");
                        for (i, layer) in ase.layers.iter().enumerate() {
                            let vis = if layer.visible { "👁" } else { "  " };
                            info.push_str(&format!("  {} {} {} (opacity: {})\n", vis, i, layer.name, layer.opacity));
                        }
                        if !ase.tags.is_empty() {
                            info.push_str("\n── Tags (Animations) ──\n");
                            for tag in &ase.tags {
                                let dir = match tag.direction {
                                    0 => "→",
                                    1 => "←",
                                    2 => "↔",
                                    _ => "?",
                                };
                                info.push_str(&format!("  {} \"{}\" frames {}-{}\n", dir, tag.name, tag.from, tag.to));
                            }
                        }
                        info.push_str("\n── Frame Durations ──\n");
                        for (i, frame) in ase.frames.iter().enumerate() {
                            info.push_str(&format!("  Frame {}: {}ms\n", i, frame.duration_ms));
                        }
                        if !ase.palette.is_empty() {
                            info.push_str(&format!("\nPalette: {} colors\n", ase.palette.len()));
                        }
                        app.readme_content = Some((name.clone(), info));
                    }
                    Err(e) => {
                        app.readme_content = Some((name.clone(), format!("Cannot parse .aseprite: {e}")));
                    }
                }
            } else if is_text_file(&name) {
                if let Ok(content) = std::fs::read_to_string(path) {
                    app.readme_content = Some((name.clone(), content));
                }
            } else {
                // For non-text files, show basic info
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                let info = format!(
                    "File: {}\nPath: {}\nSize: {}",
                    name, rel, format_size(size)
                );
                app.readme_content = Some((name.clone(), info));

                // If it's an image, load a preview directly
                let lower = name.to_lowercase();
                if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".bmp") {
                    let path_str = path.to_string_lossy().to_string();
                    if app.preview_loaded_path != path_str {
                        if let Ok(img) = image::open(path) {
                            let preview = img.thumbnail(512, 512);
                            let rgba = preview.to_rgba8();
                            let sz = [rgba.width() as usize, rgba.height() as usize];
                            let pixels = rgba.into_raw();
                            let color_image = egui::ColorImage::from_rgba_unmultiplied(sz, &pixels);
                            app.tex_counter += 1;
                            let tex = ui.ctx().load_texture(
                                format!("file_preview_{}", app.tex_counter),
                                color_image,
                                egui::TextureOptions::NEAREST,
                            );
                            app.preview_texture = Some(tex);
                            app.preview_loaded_path = path_str;
                        }
                    }
                    if let Some((w, h)) = crate::thumbnail::image_dimensions(path) {
                        let info = format!(
                            "File: {}\nPath: {}\nSize: {}\nDimensions: {}×{} px",
                            name, rel, format_size(size), w, h
                        );
                        app.readme_content = Some((name.clone(), info));
                    }
                }

                // Select as asset if it exists in the library
                let pack_id = pack_root.file_name()
                    .map(|n| n.to_string_lossy().replace(' ', "_").to_lowercase())
                    .unwrap_or_default();
                if let Some(asset) = app.library.assets.iter().find(|a| {
                    a.pack_id == pack_id && a.path == rel
                }) {
                    app.selected_asset = Some(asset.id.clone());
                    app.preview_loaded_path.clear();
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

fn is_aseprite_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".aseprite") || lower.ends_with(".ase")
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{bytes} B") }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
}
