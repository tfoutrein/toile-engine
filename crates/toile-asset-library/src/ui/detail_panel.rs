//! Detail panel — right-side panel showing full metadata and preview for the selected asset.

use crate::types::*;
use super::AssetBrowserApp;

/// Render the detail panel for the currently selected asset.
pub fn show_detail_panel(
    app: &mut AssetBrowserApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let selected_id = match &app.selected_asset {
        Some(id) => id.clone(),
        None => {
            ui.label("No asset selected.");
            return;
        }
    };

    let asset = app
        .library
        .assets
        .iter()
        .find(|a| a.id == selected_id)
        .cloned();

    let asset = match asset {
        Some(a) => a,
        None => {
            ui.label("Asset not found.");
            app.selected_asset = None;
            return;
        }
    };

    // Close button
    ui.horizontal(|ui| {
        ui.heading(format!("{} {}", asset.asset_type.icon(), asset.name));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("\u{2716}").clicked() {
                app.selected_asset = None;
                app.preview_texture = None;
                app.preview_loaded_path.clear();
            }
        });
    });

    ui.separator();

    // -- Full-size preview image --
    let is_image_type = matches!(
        asset.asset_type,
        AssetType::Sprite
            | AssetType::Tileset
            | AssetType::Background
            | AssetType::Icon
            | AssetType::Gui
            | AssetType::Prop
            | AssetType::Vfx
    );

    if is_image_type {
        app.load_preview(&selected_id, ctx);

        if let Some(ref tex) = app.preview_texture {
            let available_width = ui.available_width();
            let tex_size = tex.size_vec2();
            let scale = (available_width / tex_size.x).min(1.0);
            let display_size = tex_size * scale;

            ui.add(egui::Image::new(tex).fit_to_exact_size(display_size));

            // Sprite sheet grid overlay info
            if let AssetMetadata::Sprite(ref sm) = asset.metadata {
                if sm.columns > 1 || sm.rows > 1 {
                    ui.small(format!(
                        "Grid: {}x{} ({}x{}px frames)",
                        sm.columns, sm.rows, sm.frame_width, sm.frame_height
                    ));
                }
            }
            if let AssetMetadata::Tileset(ref tm) = asset.metadata {
                if tm.columns > 1 || tm.rows > 1 {
                    ui.small(format!(
                        "Grid: {}x{} ({}x{}px tiles)",
                        tm.columns, tm.rows, tm.tile_width, tm.tile_height
                    ));
                }
            }

            ui.separator();
        }
    }

    // -- Metadata section --
    ui.heading("Details");
    ui.add_space(4.0);

    egui::Grid::new("detail_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Type:");
            ui.label(format!("{} {}", asset.asset_type.icon(), asset.asset_type.label()));
            ui.end_row();

            if !asset.subtype.is_empty() {
                ui.label("Subtype:");
                ui.label(&asset.subtype);
                ui.end_row();
            }

            ui.label("Pack:");
            ui.label(&asset.pack_id);
            ui.end_row();

            ui.label("Path:");
            ui.label(&asset.path);
            ui.end_row();

            ui.label("ID:");
            ui.label(egui::RichText::new(&asset.id).monospace().small());
            ui.end_row();
        });

    ui.add_space(8.0);

    // -- Type-specific metadata --
    match &asset.metadata {
        AssetMetadata::Sprite(sm) => {
            ui.heading("Sprite Info");
            egui::Grid::new("sprite_meta")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Frame size:");
                    ui.label(format!("{}x{}", sm.frame_width, sm.frame_height));
                    ui.end_row();

                    ui.label("Frame count:");
                    ui.label(format!("{}", sm.frame_count));
                    ui.end_row();

                    ui.label("Grid:");
                    ui.label(format!("{} cols x {} rows", sm.columns, sm.rows));
                    ui.end_row();

                    if !sm.source_format.is_empty() {
                        ui.label("Format:");
                        ui.label(&sm.source_format);
                        ui.end_row();
                    }
                });

            if !sm.animations.is_empty() {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Animations:").strong());
                for anim in &sm.animations {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "  {} — {} frames @ {}fps{}",
                            anim.name,
                            anim.frames.len(),
                            anim.fps,
                            if anim.looping { " (loop)" } else { "" }
                        ));
                    });
                }
            }
        }

        AssetMetadata::Tileset(tm) => {
            ui.heading("Tileset Info");
            egui::Grid::new("tileset_meta")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Tile size:");
                    ui.label(format!("{}x{}", tm.tile_width, tm.tile_height));
                    ui.end_row();

                    ui.label("Tile count:");
                    ui.label(format!("{}", tm.tile_count));
                    ui.end_row();

                    ui.label("Grid:");
                    ui.label(format!("{} cols x {} rows", tm.columns, tm.rows));
                    ui.end_row();

                    if tm.spacing > 0 {
                        ui.label("Spacing:");
                        ui.label(format!("{}px", tm.spacing));
                        ui.end_row();
                    }

                    if tm.margin > 0 {
                        ui.label("Margin:");
                        ui.label(format!("{}px", tm.margin));
                        ui.end_row();
                    }
                });
        }

        AssetMetadata::Tilemap(tm) => {
            ui.heading("Tilemap Info");
            egui::Grid::new("tilemap_meta")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Map size:");
                    ui.label(format!("{}x{} tiles", tm.width, tm.height));
                    ui.end_row();

                    ui.label("Tile size:");
                    ui.label(format!("{}x{}", tm.tile_width, tm.tile_height));
                    ui.end_row();

                    if !tm.orientation.is_empty() {
                        ui.label("Orientation:");
                        ui.label(&tm.orientation);
                        ui.end_row();
                    }

                    if tm.layer_count > 0 {
                        ui.label("Layers:");
                        ui.label(format!("{}", tm.layer_count));
                        ui.end_row();
                    }

                    if !tm.source_format.is_empty() {
                        ui.label("Format:");
                        ui.label(&tm.source_format);
                        ui.end_row();
                    }
                });
        }

        AssetMetadata::Background(bg) => {
            ui.heading("Background Info");
            egui::Grid::new("bg_meta")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Size:");
                    ui.label(format!("{}x{}", bg.width, bg.height));
                    ui.end_row();

                    ui.label("Parallax:");
                    ui.label(if bg.is_parallax { "Yes" } else { "No" });
                    ui.end_row();

                    if !bg.layers.is_empty() {
                        ui.label("Layers:");
                        ui.label(format!("{}", bg.layers.len()));
                        ui.end_row();
                    }
                });
        }

        AssetMetadata::Audio(am) => {
            ui.heading("Audio Info");
            egui::Grid::new("audio_meta")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Format:");
                    ui.label(&am.format);
                    ui.end_row();

                    if am.duration_secs > 0.0 {
                        ui.label("Duration:");
                        ui.label(format!("{:.1}s", am.duration_secs));
                        ui.end_row();
                    }

                    if am.sample_rate > 0 {
                        ui.label("Sample rate:");
                        ui.label(format!("{}Hz", am.sample_rate));
                        ui.end_row();
                    }

                    if am.channels > 0 {
                        ui.label("Channels:");
                        ui.label(format!("{}", am.channels));
                        ui.end_row();
                    }

                    if !am.category.is_empty() {
                        ui.label("Category:");
                        ui.label(&am.category);
                        ui.end_row();
                    }
                });
        }

        AssetMetadata::Font(fm) => {
            ui.heading("Font Info");
            egui::Grid::new("font_meta")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Format:");
                    ui.label(&fm.format);
                    ui.end_row();

                    if !fm.face.is_empty() {
                        ui.label("Face:");
                        ui.label(&fm.face);
                        ui.end_row();
                    }

                    if fm.size > 0 {
                        ui.label("Size:");
                        ui.label(format!("{}px", fm.size));
                        ui.end_row();
                    }

                    if !fm.pages.is_empty() {
                        ui.label("Pages:");
                        ui.label(format!("{}", fm.pages.len()));
                        ui.end_row();
                    }
                });
        }

        AssetMetadata::None => {}
    }

    // -- Tags --
    if !asset.tags.is_empty() {
        ui.add_space(8.0);
        ui.heading("Tags");
        ui.horizontal_wrapped(|ui| {
            for tag in &asset.tags {
                // Colored chip
                let chip = egui::RichText::new(tag)
                    .small()
                    .background_color(egui::Color32::from_rgb(50, 70, 100));
                ui.label(chip);
            }
        });
    }

    // -- Related assets --
    if !asset.related_assets.is_empty() {
        ui.add_space(8.0);
        ui.heading("Related");
        for related_id in &asset.related_assets {
            if ui.link(related_id).clicked() {
                app.selected_asset = Some(related_id.clone());
                app.preview_loaded_path.clear();
            }
        }
    }

    // -- File path (copyable) --
    ui.add_space(8.0);
    ui.separator();
    if let Some(abs_path) = app.library.absolute_path(&asset) {
        ui.horizontal(|ui| {
            ui.label("File:");
            ui.monospace(abs_path.to_string_lossy().to_string());
        });
    }
}
