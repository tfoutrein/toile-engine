use std::path::PathBuf;

use toile_behaviors::BehaviorConfig;

use crate::scene_data::EntityData;

/// Get image dimensions without loading the full image.
pub(crate) fn get_image_dimensions(sprite_path: &str, pdir: &Option<PathBuf>) -> Option<(u32, u32)> {
    if sprite_path.is_empty() { return None; }
    let full = pdir.as_ref().map(|d| d.join(sprite_path)).unwrap_or_else(|| PathBuf::from(sprite_path));
    image::image_dimensions(&full).ok()
}

/// Try to auto-detect sprite sheet layout from image dimensions.
/// Tests common frame sizes and picks the best fit.
pub(crate) fn auto_detect_sprite_sheet(sprite_path: &str, pdir: &Option<PathBuf>) -> toile_scene::SpriteSheetData {
    let common_sizes: &[u32] = &[16, 24, 32, 48, 64, 96, 128, 256];

    if let Some((img_w, img_h)) = get_image_dimensions(sprite_path, pdir) {
        // Try each common size and see which divides evenly
        let mut best = (32u32, 32u32, 1u32, 1u32); // (fw, fh, cols, rows)
        let mut best_score = 0u32;

        for &fw in common_sizes {
            for &fh in common_sizes {
                if fw > img_w || fh > img_h { continue; }
                let cols = img_w / fw;
                let rows = img_h / fh;
                if cols == 0 || rows == 0 { continue; }
                // Score: prefer exact division + more frames + square-ish frames
                let exact = if img_w % fw == 0 && img_h % fh == 0 { 1000 } else { 0 };
                let frame_count = cols * rows;
                let squareness = if fw == fh { 100 } else { 0 };
                let score = exact + frame_count.min(200) + squareness;
                if score > best_score {
                    best_score = score;
                    best = (fw, fh, cols, rows);
                }
            }
        }

        toile_scene::SpriteSheetData {
            frame_width: best.0,
            frame_height: best.1,
            columns: best.2,
            rows: best.3,
        }
    } else {
        toile_scene::SpriteSheetData {
            frame_width: 32,
            frame_height: 32,
            columns: 4,
            rows: 4,
        }
    }
}

/// Pick an icon based on entity properties.
pub(crate) fn entity_icon(entity: &EntityData) -> &'static str {
    if entity.light.is_some() { return "💡"; }
    if entity.particle_emitter.is_some() { return "✨"; }
    if entity.tags.iter().any(|t| t.eq_ignore_ascii_case("player")) { return "🧑"; }
    if entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Solid)) { return "🧱"; }
    if entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Platform(_) | BehaviorConfig::TopDown(_))) { return "🏃"; }
    "📦"
}

pub(crate) fn behavior_label(beh: &BehaviorConfig) -> &'static str {
    match beh {
        BehaviorConfig::Platform(_) => "Platform",
        BehaviorConfig::TopDown(_)  => "TopDown",
        BehaviorConfig::Bullet(_)   => "Bullet",
        BehaviorConfig::Sine(_)     => "Sine",
        BehaviorConfig::Fade(_)     => "Fade",
        BehaviorConfig::Wrap(_)     => "Wrap",
        BehaviorConfig::Solid       => "Solid",
    }
}

pub(crate) fn default_behavior_config(name: &str) -> BehaviorConfig {
    match name {
        "Platform" => BehaviorConfig::Platform(Default::default()),
        "TopDown"  => BehaviorConfig::TopDown(Default::default()),
        "Bullet"   => BehaviorConfig::Bullet(Default::default()),
        "Sine"     => BehaviorConfig::Sine(Default::default()),
        "Fade"     => BehaviorConfig::Fade(Default::default()),
        "Wrap"     => BehaviorConfig::Wrap(Default::default()),
        "Solid"    => BehaviorConfig::Solid,
        _          => BehaviorConfig::Solid,
    }
}

// ── Post-effect helpers for Scene Settings ──────────────────────────

pub(crate) fn post_effect_label(fx: &toile_scene::PostEffectData) -> &'static str {
    match fx {
        toile_scene::PostEffectData::Vignette { .. } => "Vignette",
        toile_scene::PostEffectData::Crt { .. } => "CRT",
        toile_scene::PostEffectData::Pixelate { .. } => "Pixelate",
        toile_scene::PostEffectData::Bloom { .. } => "Bloom",
        toile_scene::PostEffectData::ColorGrading { .. } => "Color Grading",
    }
}

pub(crate) fn default_post_effect(name: &str) -> toile_scene::PostEffectData {
    match name {
        "Vignette" => toile_scene::PostEffectData::Vignette { intensity: 0.5, smoothness: 0.5 },
        "Bloom" => toile_scene::PostEffectData::Bloom { threshold: 0.8, intensity: 0.5, radius: 2.0 },
        "CRT" => toile_scene::PostEffectData::Crt { scanline_intensity: 0.3, curvature: 0.02, chromatic_aberration: 0.003 },
        "Pixelate" => toile_scene::PostEffectData::Pixelate { pixel_size: 4.0 },
        "ColorGrading" => toile_scene::PostEffectData::ColorGrading { saturation: 1.0, brightness: 1.0, contrast: 1.0 },
        _ => toile_scene::PostEffectData::Vignette { intensity: 0.5, smoothness: 0.5 },
    }
}

pub(crate) fn post_effect_inspector(ui: &mut egui::Ui, fx: &mut toile_scene::PostEffectData, idx: usize) {
    let grid_id = format!("fx_grid_{idx}");
    match fx {
        toile_scene::PostEffectData::Vignette { intensity, smoothness } => {
            egui::Grid::new(grid_id).num_columns(2).show(ui, |ui| {
                ui.label("Intensity"); ui.add(egui::Slider::new(intensity, 0.0..=2.0)); ui.end_row();
                ui.label("Smoothness"); ui.add(egui::Slider::new(smoothness, 0.0..=2.0)); ui.end_row();
            });
        }
        toile_scene::PostEffectData::Bloom { threshold, intensity, radius } => {
            egui::Grid::new(grid_id).num_columns(2).show(ui, |ui| {
                ui.label("Threshold"); ui.add(egui::Slider::new(threshold, 0.0..=1.0)); ui.end_row();
                ui.label("Intensity"); ui.add(egui::Slider::new(intensity, 0.0..=2.0)); ui.end_row();
                ui.label("Radius"); ui.add(egui::Slider::new(radius, 0.5..=10.0)); ui.end_row();
            });
        }
        toile_scene::PostEffectData::Crt { scanline_intensity, curvature, chromatic_aberration } => {
            egui::Grid::new(grid_id).num_columns(2).show(ui, |ui| {
                ui.label("Scanlines"); ui.add(egui::Slider::new(scanline_intensity, 0.0..=1.0)); ui.end_row();
                ui.label("Curvature"); ui.add(egui::Slider::new(curvature, 0.0..=0.1)); ui.end_row();
                ui.label("Chrom. Ab."); ui.add(egui::Slider::new(chromatic_aberration, 0.0..=0.02)); ui.end_row();
            });
        }
        toile_scene::PostEffectData::Pixelate { pixel_size } => {
            ui.horizontal(|ui| {
                ui.label("Pixel size");
                ui.add(egui::Slider::new(pixel_size, 1.0..=32.0));
            });
        }
        toile_scene::PostEffectData::ColorGrading { saturation, brightness, contrast } => {
            egui::Grid::new(grid_id).num_columns(2).show(ui, |ui| {
                ui.label("Saturation"); ui.add(egui::Slider::new(saturation, 0.0..=3.0)); ui.end_row();
                ui.label("Brightness"); ui.add(egui::Slider::new(brightness, 0.0..=3.0)); ui.end_row();
                ui.label("Contrast"); ui.add(egui::Slider::new(contrast, 0.0..=3.0)); ui.end_row();
            });
        }
    }
}

pub(crate) fn behavior_inspector(ui: &mut egui::Ui, beh: &mut BehaviorConfig, idx: usize) {
    let grid_id = format!("beh_grid_{idx}");
    match beh {
        BehaviorConfig::Platform(c) => {
            egui::Grid::new(grid_id).num_columns(2).show(ui, |ui| {
                ui.label("Gravity"); ui.add(egui::DragValue::new(&mut c.gravity).speed(1.0)); ui.end_row();
                ui.label("Jump Force"); ui.add(egui::DragValue::new(&mut c.jump_force).speed(1.0)); ui.end_row();
                ui.label("Max Speed"); ui.add(egui::DragValue::new(&mut c.max_speed).speed(1.0)); ui.end_row();
                ui.label("Accel"); ui.add(egui::DragValue::new(&mut c.acceleration).speed(1.0)); ui.end_row();
                ui.label("Decel"); ui.add(egui::DragValue::new(&mut c.deceleration).speed(1.0)); ui.end_row();
                ui.label("Coyote"); ui.add(egui::DragValue::new(&mut c.coyote_time).speed(0.01)); ui.end_row();
                ui.label("Jump Buf"); ui.add(egui::DragValue::new(&mut c.jump_buffer).speed(0.01)); ui.end_row();
                ui.label("Max Jumps"); ui.add(egui::DragValue::new(&mut c.max_jumps).range(1..=5)); ui.end_row();
            });
        }
        BehaviorConfig::TopDown(c) => {
            egui::Grid::new(grid_id).num_columns(2).show(ui, |ui| {
                ui.label("Max Speed"); ui.add(egui::DragValue::new(&mut c.max_speed).speed(1.0)); ui.end_row();
                ui.label("Accel"); ui.add(egui::DragValue::new(&mut c.acceleration).speed(1.0)); ui.end_row();
                ui.label("Decel"); ui.add(egui::DragValue::new(&mut c.deceleration).speed(1.0)); ui.end_row();
                ui.label("Diag Fix"); ui.checkbox(&mut c.diagonal_correction, ""); ui.end_row();
            });
        }
        BehaviorConfig::Bullet(c) => {
            egui::Grid::new(grid_id).num_columns(2).show(ui, |ui| {
                ui.label("Speed"); ui.add(egui::DragValue::new(&mut c.speed).speed(1.0)); ui.end_row();
                ui.label("Accel"); ui.add(egui::DragValue::new(&mut c.acceleration).speed(0.1)); ui.end_row();
                ui.label("Gravity"); ui.add(egui::DragValue::new(&mut c.gravity).speed(1.0)); ui.end_row();
                ui.label("Angle°"); ui.add(egui::DragValue::new(&mut c.angle_degrees).speed(1.0)); ui.end_row();
            });
        }
        BehaviorConfig::Sine(c) => {
            egui::Grid::new(grid_id).num_columns(2).show(ui, |ui| {
                ui.label("Property");
                egui::ComboBox::from_id_salt(format!("sine_prop_{idx}"))
                    .selected_text(format!("{:?}", c.property))
                    .show_ui(ui, |ui| {
                        use toile_behaviors::sine::SineProperty;
                        ui.selectable_value(&mut c.property, SineProperty::X, "X");
                        ui.selectable_value(&mut c.property, SineProperty::Y, "Y");
                        ui.selectable_value(&mut c.property, SineProperty::Angle, "Angle");
                        ui.selectable_value(&mut c.property, SineProperty::Opacity, "Opacity");
                        ui.selectable_value(&mut c.property, SineProperty::Size, "Size");
                    });
                ui.end_row();
                ui.label("Magnitude"); ui.add(egui::DragValue::new(&mut c.magnitude).speed(0.5)); ui.end_row();
                ui.label("Period"); ui.add(egui::DragValue::new(&mut c.period).speed(0.1).range(0.1..=60.0)); ui.end_row();
            });
        }
        BehaviorConfig::Fade(c) => {
            egui::Grid::new(grid_id).num_columns(2).show(ui, |ui| {
                ui.label("Fade In"); ui.add(egui::DragValue::new(&mut c.fade_in_time).speed(0.1)); ui.end_row();
                ui.label("Fade Out"); ui.add(egui::DragValue::new(&mut c.fade_out_time).speed(0.1)); ui.end_row();
                ui.label("Destroy"); ui.checkbox(&mut c.destroy_on_fade_out, "on fade out"); ui.end_row();
            });
        }
        BehaviorConfig::Wrap(c) => {
            ui.horizontal(|ui| {
                ui.label("Margin");
                ui.add(egui::DragValue::new(&mut c.margin).speed(1.0));
            });
        }
        BehaviorConfig::Solid => {
            ui.label(egui::RichText::new("Static solid — blocks Platform movement").size(10.0).color(egui::Color32::from_gray(140)));
        }
    }
}

pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let i = (h * 6.0).floor() as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}
