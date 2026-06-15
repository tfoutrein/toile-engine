use std::path::PathBuf;

use toile_behaviors::BehaviorConfig;

use crate::scene_data::EntityData;

/// Get image dimensions without loading the full image.
pub(crate) fn get_image_dimensions(sprite_path: &str, pdir: &Option<PathBuf>) -> Option<(u32, u32)> {
    if sprite_path.is_empty() { return None; }
    let full = pdir.as_ref().map(|d| d.join(sprite_path)).unwrap_or_else(|| PathBuf::from(sprite_path));
    image::image_dimensions(&full).ok()
}

/// Clean display name for an animation state (avoids `Custom("attack")` debug form).
pub(crate) fn anim_state_label(s: &toile_scene::AnimState) -> String {
    use toile_scene::AnimState::*;
    match s {
        Idle => "Idle".into(),
        Walk => "Walk".into(),
        Run => "Run".into(),
        Jump => "Jump".into(),
        Fall => "Fall".into(),
        Custom(n) => n.clone(),
    }
}

/// "grid" (shares the entity sprite sheet) or "strip" (own sprite_file).
pub(crate) fn anim_source_tag(anim: &toile_scene::AnimationData) -> &'static str {
    if anim.sprite_file.is_some() { "strip" } else { "grid" }
}

/// Human-readable trigger condition for a canonical state (ADR-039).
/// KEEP IN SYNC with `select_states` in toile-runner/src/game_runner.rs — a future
/// refactor (ADR-039 Phase 0.5) will derive both from one shared source.
pub(crate) fn state_condition_label(state: &toile_scene::AnimState, move_threshold: f32, topdown: bool) -> String {
    use toile_scene::AnimState::*;
    if topdown {
        return match state {
            Walk | Run => "en mouvement".into(),
            Idle => "immobile".into(),
            Custom(_) => "scripté (event sheet)".into(),
            _ => "—".into(),
        };
    }
    match state {
        Idle => "au sol, immobile".into(),
        Walk => format!("au sol, |vx| > {:.0}", move_threshold),
        Run => format!("au sol, |vx| > {:.0}", move_threshold * 24.0),
        Jump => "en l'air, monte".into(),
        Fall => "en l'air, descend".into(),
        Custom(_) => "scripté (event sheet)".into(),
    }
}

/// How [`add_animation_to_entity`] resolves a name collision (ADR-039). One rule
/// replaces the three divergent importer behaviors that used to coexist
/// (Aseprite = replace, Strip = silently skip, AI = replace).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AnimConflict {
    /// Keep the existing clip and store the new one under a suffixed name (`walk_2`).
    /// Re-adding an *identical* clip (same name + source + frames) is a no-op.
    KeepBoth,
    /// Overwrite the existing clip of the same name in place (re-importable sources).
    Replace,
}

/// Outcome of an additive add, for status reporting. The carried `String` is the
/// name actually stored (may be suffixed under [`AnimConflict::KeepBoth`]).
pub(crate) enum AnimAddResult {
    Added(String),
    Replaced(String),
}

/// Add ONE animation to an entity **additively** — never clears existing clips
/// (ADR-039). The sourcing model is encoded in `anim` itself: a *strip* carries
/// its own `sprite_file` + `strip_frames` (autonomous, renders from its own file);
/// a *grid* leaves them `None` and indexes the entity's shared `sprite_sheet`. On a
/// name collision the `policy` decides. Sets `default_animation` when the entity had
/// none, then refreshes missing state bindings. Single source of truth shared by the
/// asset browser, the inspector and the AI tool.
pub(crate) fn add_animation_to_entity(
    entity: &mut EntityData,
    mut anim: toile_scene::AnimationData,
    policy: AnimConflict,
) -> AnimAddResult {
    let collides = entity.animations.iter().any(|a| a.name == anim.name);
    if collides {
        match policy {
            AnimConflict::Replace => {
                let name = anim.name.clone();
                if let Some(slot) = entity.animations.iter_mut().find(|a| a.name == name) {
                    *slot = anim;
                }
                auto_populate_missing_bindings(entity);
                return AnimAddResult::Replaced(name);
            }
            AnimConflict::KeepBoth => {
                // Idempotent re-add: re-importing the exact same clip must not pile up
                // dead `walk_2`, `walk_3`… copies — only genuine variants get suffixed.
                if let Some(existing) = entity.animations.iter().find(|a| {
                    a.name == anim.name && a.sprite_file == anim.sprite_file && a.frames == anim.frames
                }) {
                    return AnimAddResult::Added(existing.name.clone());
                }
                anim.name = unique_anim_name(entity, &anim.name);
            }
        }
    }
    let name = anim.name.clone();
    if entity.default_animation.is_none() {
        entity.default_animation = Some(name.clone());
    }
    entity.animations.push(anim);
    auto_populate_missing_bindings(entity);
    AnimAddResult::Added(name)
}

/// `base`, then `base_2`, `base_3`, … — first name not already used by a clip.
fn unique_anim_name(entity: &EntityData, base: &str) -> String {
    if !entity.animations.iter().any(|a| a.name == base) {
        return base.to_string();
    }
    (2..)
        .map(|i| format!("{base}_{i}"))
        .find(|cand| !entity.animations.iter().any(|a| a.name == *cand))
        .expect("infinite range yields a free name")
}

/// Auto-populate `animation_states` bindings from the entity's animation names, so
/// the editor state slots reflect what will actually play (ADR-038 Phase 4). Only
/// fills states that aren't already bound (never overwrites an explicit binding —
/// hardened contract, ADR-039); matches names case-insensitively via a synonym
/// table (mirrors the runtime fallback).
pub(crate) fn auto_populate_missing_bindings(entity: &mut EntityData) {
    if entity.animations.is_empty() {
        return;
    }
    let states: [(toile_scene::AnimState, &[&str]); 5] = [
        (toile_scene::AnimState::Idle, &["idle", "repos", "stand", "default", "wait"]),
        (toile_scene::AnimState::Walk, &["walk", "marche", "move", "moving"]),
        (toile_scene::AnimState::Run, &["run", "course", "sprint"]),
        (toile_scene::AnimState::Jump, &["jump", "saut", "sauter", "rise"]),
        (toile_scene::AnimState::Fall, &["fall", "chute", "tomber"]),
    ];
    let mut map = entity.animation_states.take().unwrap_or_default();
    for (state, syns) in states {
        if map.anim_for(&state).is_some() {
            continue; // keep an existing explicit binding
        }
        if let Some(a) = entity
            .animations
            .iter()
            .find(|a| syns.iter().any(|s| a.name.eq_ignore_ascii_case(s)))
        {
            map.set_binding(state, a.name.clone());
        }
    }
    // Only attach a map if it actually carries something (keeps legacy scenes clean).
    if !map.bindings.is_empty() {
        entity.animation_states = Some(map);
    } else {
        entity.animation_states = None;
    }
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
    if entity.tags.iter().any(|t| t.eq_ignore_ascii_case("player")) { return "👤"; }
    if entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Solid)) { return "⬛"; }
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

#[cfg(test)]
mod auto_bind_tests {
    use super::*;
    use toile_scene::{AnimState, AnimationData};

    fn anim(name: &str) -> AnimationData {
        AnimationData { name: name.into(), frames: vec![0], fps: 8.0, looping: true, sprite_file: None, strip_frames: None }
    }

    fn strip(name: &str, file: &str, frames: u32) -> AnimationData {
        AnimationData {
            name: name.into(),
            frames: (0..frames).collect(),
            fps: 10.0,
            looping: true,
            sprite_file: Some(file.into()),
            strip_frames: Some(frames),
        }
    }

    #[test]
    fn maps_animation_names_to_states_case_insensitively() {
        let mut e = EntityData::default();
        e.animations = vec![anim("Idle"), anim("course"), anim("Walk")];
        auto_populate_missing_bindings(&mut e);
        let m = e.animation_states.expect("bindings created");
        assert_eq!(m.anim_for(&AnimState::Idle), Some("Idle"));
        assert_eq!(m.anim_for(&AnimState::Run), Some("course")); // synonym of run
        assert_eq!(m.anim_for(&AnimState::Walk), Some("Walk"));
        assert_eq!(m.anim_for(&AnimState::Jump), None); // no jump-like anim present
    }

    #[test]
    fn keeps_existing_binding_and_skips_when_no_match() {
        let mut e = EntityData::default();
        e.animations = vec![anim("custom_walk_anim")]; // matches no synonym
        let mut map = toile_scene::AnimationStateMap::default();
        map.set_binding(AnimState::Walk, "custom_walk_anim".into());
        e.animation_states = Some(map);
        auto_populate_missing_bindings(&mut e);
        let m = e.animation_states.expect("kept");
        assert_eq!(m.anim_for(&AnimState::Walk), Some("custom_walk_anim")); // preserved
    }

    #[test]
    fn no_animations_clears_to_none() {
        let mut e = EntityData::default();
        auto_populate_missing_bindings(&mut e);
        assert_eq!(e.animation_states, None);
    }

    #[test]
    fn add_is_additive_and_sets_first_default() {
        let mut e = EntityData::default();
        let r1 = add_animation_to_entity(&mut e, strip("idle", "idle.png", 4), AnimConflict::KeepBoth);
        let r2 = add_animation_to_entity(&mut e, strip("walk", "walk.png", 6), AnimConflict::KeepBoth);
        assert!(matches!(r1, AnimAddResult::Added(ref n) if n == "idle"));
        assert!(matches!(r2, AnimAddResult::Added(ref n) if n == "walk"));
        assert_eq!(e.animations.len(), 2); // additive — idle not clobbered
        assert_eq!(e.default_animation.as_deref(), Some("idle")); // first becomes default
        // Both bound to their states by name.
        let m = e.animation_states.as_ref().expect("states");
        assert_eq!(m.anim_for(&AnimState::Idle), Some("idle"));
        assert_eq!(m.anim_for(&AnimState::Walk), Some("walk"));
    }

    #[test]
    fn keep_both_suffixes_on_collision() {
        let mut e = EntityData::default();
        add_animation_to_entity(&mut e, strip("walk", "a.png", 6), AnimConflict::KeepBoth);
        let r = add_animation_to_entity(&mut e, strip("walk", "b.png", 8), AnimConflict::KeepBoth);
        assert!(matches!(r, AnimAddResult::Added(ref n) if n == "walk_2"));
        assert_eq!(e.animations.len(), 2);
        assert_eq!(e.animations[1].name, "walk_2");
        assert_eq!(e.animations[1].sprite_file.as_deref(), Some("b.png"));
    }

    #[test]
    fn replace_overwrites_in_place() {
        let mut e = EntityData::default();
        add_animation_to_entity(&mut e, strip("walk", "a.png", 6), AnimConflict::KeepBoth);
        let r = add_animation_to_entity(&mut e, strip("walk", "b.png", 8), AnimConflict::Replace);
        assert!(matches!(r, AnimAddResult::Replaced(ref n) if n == "walk"));
        assert_eq!(e.animations.len(), 1);
        assert_eq!(e.animations[0].sprite_file.as_deref(), Some("b.png"));
        assert_eq!(e.animations[0].strip_frames, Some(8));
    }

    #[test]
    fn keep_both_is_idempotent_on_identical_reimport() {
        let mut e = EntityData::default();
        add_animation_to_entity(&mut e, strip("walk", "a.png", 6), AnimConflict::KeepBoth);
        // Same name + same source + same frames → no duplicate, no suffix.
        let r = add_animation_to_entity(&mut e, strip("walk", "a.png", 6), AnimConflict::KeepBoth);
        assert!(matches!(r, AnimAddResult::Added(ref n) if n == "walk"));
        assert_eq!(e.animations.len(), 1);
    }

    #[test]
    fn source_tag_distinguishes_grid_from_strip() {
        let grid = anim("g"); // sprite_file None -> grid
        let mut strip = anim("s");
        strip.sprite_file = Some("walk_strip.png".into());
        assert_eq!(anim_source_tag(&grid), "grid");
        assert_eq!(anim_source_tag(&strip), "strip");
    }

    #[test]
    fn state_label_uses_custom_name() {
        assert_eq!(anim_state_label(&AnimState::Idle), "Idle");
        assert_eq!(anim_state_label(&AnimState::Custom("dash".into())), "dash");
    }

    #[test]
    fn condition_labels_match_runtime_intent() {
        // Platformer reads velocity Y for air states; topdown collapses to moving/idle.
        assert_eq!(state_condition_label(&AnimState::Idle, 5.0, false), "au sol, immobile");
        assert_eq!(state_condition_label(&AnimState::Walk, 5.0, false), "au sol, |vx| > 5");
        assert_eq!(state_condition_label(&AnimState::Jump, 5.0, false), "en l'air, monte");
        assert_eq!(state_condition_label(&AnimState::Fall, 5.0, false), "en l'air, descend");
        assert_eq!(state_condition_label(&AnimState::Walk, 5.0, true), "en mouvement");
        assert_eq!(state_condition_label(&AnimState::Idle, 5.0, true), "immobile");
        assert_eq!(
            state_condition_label(&AnimState::Custom("x".into()), 5.0, false),
            "scripté (event sheet)"
        );
    }
}
