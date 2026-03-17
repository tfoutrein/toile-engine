//! Browser panel — top bar with import/search/filters + scrollable thumbnail grid.

use crate::types::AssetType;
use super::AssetBrowserApp;

/// All filterable asset types.
const FILTER_TYPES: &[(AssetType, &str)] = &[
    (AssetType::Sprite, "Sprite"),
    (AssetType::Tileset, "Tileset"),
    (AssetType::Background, "BG"),
    (AssetType::Audio, "Audio"),
    (AssetType::Font, "Font"),
    (AssetType::Gui, "GUI"),
    (AssetType::Vfx, "VFX"),
    (AssetType::Prop, "Prop"),
    (AssetType::Icon, "Icon"),
    (AssetType::Skeleton, "Skel"),
];

/// Render the main browser panel: toolbar + thumbnail grid.
pub fn show_browser_panel(
    app: &mut AssetBrowserApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    filtered_ids: &[String],
) {
    // -- Top bar --
    ui.horizontal_wrapped(|ui| {
        // View mode toggle
        let is_assets = app.view_mode == super::ViewMode::Assets;
        let is_files = app.view_mode == super::ViewMode::Files;
        if ui.selectable_label(is_assets, "🖼 Assets").clicked() {
            app.view_mode = super::ViewMode::Assets;
        }
        if ui.selectable_label(is_files, "📂 Files").clicked() {
            app.view_mode = super::ViewMode::Files;
        }
        ui.separator();

        // Search field
        ui.label("\u{1f50d}");
        ui.add(
            egui::TextEdit::singleline(&mut app.search_text)
                .hint_text("Search assets...")
                .desired_width(200.0),
        );

        ui.separator();

        // Type filter buttons
        let all_selected = app.filter_type.is_none();
        if ui.selectable_label(all_selected, "All").clicked() {
            app.filter_type = None;
        }

        for (asset_type, label) in FILTER_TYPES {
            let is_selected = app.filter_type == Some(*asset_type);
            let btn_text = format!("{} {}", asset_type.icon(), label);
            if ui.selectable_label(is_selected, btn_text).clicked() {
                if is_selected {
                    app.filter_type = None;
                } else {
                    app.filter_type = Some(*asset_type);
                }
            }
        }
    });

    ui.separator();

    // -- Empty state --
    if filtered_ids.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            if app.library.count() == 0 {
                ui.heading("No assets imported yet");
                ui.label("Click \"Import Pack...\" to add an asset pack folder.");
            } else {
                ui.heading("No assets match your filter");
                ui.label("Try changing the type filter or search text.");
            }
        });
        return;
    }

    // -- Thumbnail grid --
    let thumb_size = 128.0_f32;
    let cell_width = thumb_size + 8.0;
    let _cell_height = thumb_size + 24.0; // extra space for name label

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let columns = ((available_width / cell_width) as usize).max(1);

            // Collect asset info for all filtered IDs
            let asset_infos: Vec<(String, String, AssetType)> = filtered_ids
                .iter()
                .filter_map(|id| {
                    app.library
                        .assets
                        .iter()
                        .find(|a| a.id == *id)
                        .map(|a| (a.id.clone(), a.name.clone(), a.asset_type))
                })
                .collect();

            // Ensure thumbnails are loaded for visible assets
            for (id, _, _) in &asset_infos {
                app.ensure_thumbnail(id, ctx);
            }

            // Render grid rows
            for row_start in (0..asset_infos.len()).step_by(columns) {
                let row_end = (row_start + columns).min(asset_infos.len());
                let row = &asset_infos[row_start..row_end];

                ui.horizontal(|ui| {
                    for (asset_id, name, asset_type) in row {
                        let is_selected =
                            app.selected_asset.as_deref() == Some(asset_id.as_str());

                        let response = ui
                            .vertical(|ui| {
                                // Thumbnail area
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(thumb_size, thumb_size),
                                    egui::Sense::click(),
                                );

                                // Background fill
                                let bg_color = if is_selected {
                                    egui::Color32::from_rgba_unmultiplied(80, 80, 20, 80)
                                } else if response.hovered() {
                                    egui::Color32::from_rgba_unmultiplied(60, 60, 80, 60)
                                } else {
                                    egui::Color32::from_rgba_unmultiplied(40, 40, 50, 40)
                                };
                                ui.painter().rect_filled(rect, 4.0, bg_color);

                                // Selection border
                                if is_selected {
                                    ui.painter().rect_stroke(
                                        rect,
                                        4.0,
                                        egui::Stroke::new(2.0, egui::Color32::YELLOW),
                                        egui::StrokeKind::Outside,
                                    );
                                }

                                // Draw thumbnail image or placeholder
                                if let Some(tex) = app.thumbnail_cache.get(asset_id) {
                                    let tex_size = tex.size_vec2();
                                    let scale =
                                        (thumb_size / tex_size.x.max(tex_size.y)).min(1.0);
                                    let img_size = tex_size * scale;
                                    let img_rect = egui::Rect::from_center_size(
                                        rect.center(),
                                        img_size,
                                    );
                                    ui.painter().image(
                                        tex.id(),
                                        img_rect,
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        egui::Color32::WHITE,
                                    );
                                } else {
                                    // Placeholder icon
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        asset_type.icon(),
                                        egui::FontId::proportional(40.0),
                                        egui::Color32::from_gray(120),
                                    );
                                }

                                // Type badge in top-right
                                let badge_pos =
                                    egui::pos2(rect.max.x - 4.0, rect.min.y + 2.0);
                                ui.painter().text(
                                    badge_pos,
                                    egui::Align2::RIGHT_TOP,
                                    asset_type.icon(),
                                    egui::FontId::proportional(14.0),
                                    egui::Color32::from_gray(180),
                                );

                                // Name label (truncated to cell width)
                                let display_name = if name.len() > 16 {
                                    format!("{}...", &name[..14])
                                } else {
                                    name.clone()
                                };
                                ui.add_sized(
                                    [thumb_size, 18.0],
                                    egui::Label::new(
                                        egui::RichText::new(display_name)
                                            .small()
                                            .color(egui::Color32::from_gray(200)),
                                    )
                                    .truncate(),
                                );

                                response
                            })
                            .inner;

                        // Handle selection
                        if response.clicked() {
                            app.selected_asset = Some(asset_id.clone());
                            app.preview_loaded_path.clear();
                        }
                    }
                });
            }
        });
}
