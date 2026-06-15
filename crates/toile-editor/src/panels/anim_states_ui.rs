//! Shared editor widget for the entity state→animation map (ADR-038 / ADR-039 Phase 2).
//!
//! Renders the editable Idle/Walk/Run/Jump/Fall bindings — with each state's readable
//! trigger condition — plus any read-only scripted (Custom) bindings. Used by BOTH the
//! Sprite & Animation editor and the entity Inspector so the two never drift.

use std::path::PathBuf;

use toile_behaviors::BehaviorConfig;
use toile_scene::{AnimState, AnimationData, AnimationStateMap, EntityData};

use crate::editor_app::EditorApp;

/// The canonical motion states an entity exposes, given its movement behavior.
/// (Custom/scripted states live in the bindings list and are shown read-only.)
fn canonical_states(entity: &EntityData) -> Vec<(AnimState, &'static str)> {
    let has_platform = entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Platform(_)));
    let has_topdown = entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::TopDown(_)));
    if has_platform {
        vec![
            (AnimState::Idle, "Idle"),
            (AnimState::Walk, "Walk"),
            (AnimState::Run, "Run"),
            (AnimState::Jump, "Jump"),
            (AnimState::Fall, "Fall"),
        ]
    } else if has_topdown {
        vec![(AnimState::Idle, "Idle"), (AnimState::Walk, "Walk")]
    } else {
        Vec::new()
    }
}

/// Render the state→clip binding editor for `entity`. Returns the clip name to preview
/// when the user changed a binding (the Sprite editor uses it to drive its live preview;
/// the Inspector ignores it). Self-contained: derives states from the entity's behaviors,
/// shows the trigger condition per state, and lists scripted (Custom) bindings read-only.
pub(crate) fn animation_states_editor(ui: &mut egui::Ui, entity: &mut EntityData) -> Option<String> {
    let states = canonical_states(entity);
    let custom_bindings: Vec<(AnimState, String)> = entity
        .animation_states
        .as_ref()
        .map(|m| {
            m.bindings
                .iter()
                .filter(|b| matches!(b.state, AnimState::Custom(_)))
                .map(|b| (b.state.clone(), b.anim.clone()))
                .collect()
        })
        .unwrap_or_default();

    if states.is_empty() && custom_bindings.is_empty() {
        ui.label(
            egui::RichText::new(
                "Add a movement behavior (e.g. ‘Make Player’ in the Inspector) so idle/walk/jump play automatically.",
            )
            .size(11.0)
            .color(egui::Color32::from_gray(150)),
        );
        return None;
    }

    let topdown = entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::TopDown(_)));
    let move_threshold = entity.animation_states.as_ref().map_or(default_threshold(), |m| m.move_threshold);
    let anim_names: Vec<String> = entity.animations.iter().map(|a| a.name.clone()).collect();
    let eid = entity.id;
    let mut preview: Option<String> = None;

    // Auto-play toggle (only meaningful when there are canonical states to drive).
    if !states.is_empty() {
        let mut auto = entity.animation_states.as_ref().is_none_or(|m| m.auto);
        if ui
            .checkbox(&mut auto, "Auto-play these states in game")
            .on_hover_text("When off, animations only change via event sheets (PlayAnimation).")
            .changed()
        {
            entity.animation_states.get_or_insert_with(AnimationStateMap::default).auto = auto;
        }
    }

    // Namespace egui ids by entity so two instances could coexist in one frame.
    egui::Grid::new(format!("anim_states_editor_grid_{}", entity.id))
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            for (state, label) in &states {
                // Column 1: state name + its readable trigger condition.
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(*label).strong());
                    ui.label(
                        egui::RichText::new(crate::helpers::state_condition_label(state, move_threshold, topdown))
                            .size(9.0)
                            .color(egui::Color32::from_gray(120)),
                    );
                });

                // Column 2: clip picker (⚠ when the bound clip no longer exists).
                let current: String = entity
                    .animation_states
                    .as_ref()
                    .and_then(|m| m.anim_for(state))
                    .unwrap_or("")
                    .to_string();
                let broken = !current.is_empty() && !anim_names.iter().any(|n| n == &current);
                let mut selected = current.clone();
                let shown = if selected.is_empty() {
                    "(none)".to_string()
                } else if broken {
                    format!("⚠ {selected}")
                } else {
                    selected.clone()
                };
                egui::ComboBox::from_id_salt(format!("anim_state_edit_{eid}_{label}"))
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(selected.is_empty(), "(none)").clicked() {
                            selected.clear();
                        }
                        for n in &anim_names {
                            if ui.selectable_label(&selected == n, n.as_str()).clicked() {
                                selected = n.clone();
                            }
                        }
                    });
                if selected != current {
                    entity
                        .animation_states
                        .get_or_insert_with(AnimationStateMap::default)
                        .set_binding(state.clone(), selected.clone());
                    if !selected.is_empty() {
                        preview = Some(selected.clone());
                    }
                }
                ui.end_row();
            }
        });

    // Scripted (Custom) states are read-only here — they fire via event sheets (ADR-039 §5).
    if !custom_bindings.is_empty() {
        ui.add_space(2.0);
        ui.label(egui::RichText::new("Scripted states (event sheets)").size(10.0).color(egui::Color32::from_gray(110)));
        for (state, anim) in &custom_bindings {
            let name = crate::helpers::anim_state_label(state);
            let warn = if anim_names.iter().any(|n| n == anim) { "" } else { " ⚠" };
            ui.label(
                egui::RichText::new(format!("  {name} → {anim}{warn}   [script]"))
                    .size(10.0)
                    .color(egui::Color32::from_gray(150)),
            );
        }
    }

    // Fallback note only makes sense when there are canonical states to fall back.
    if !states.is_empty() {
        ui.label(
            egui::RichText::new("Empty states fall back to anims named idle/walk/jump… (case-insensitive).")
                .size(10.0)
                .color(egui::Color32::from_gray(120)),
        );
    }

    preview
}

fn default_threshold() -> f32 {
    AnimationStateMap::default().move_threshold
}

/// Form state for the modal "Add Animation" dialog (ADR-039 Phase 2). Lives on
/// `EditorApp` so it persists across frames while the dialog is open.
pub(crate) struct AddAnimForm {
    /// The strip/PNG to import (chosen via Browse). The add is gated on this being set.
    pub file: Option<PathBuf>,
    pub name: String,
    pub fps: f32,
    pub looping: bool,
    /// false = KeepBoth (suffix on collision), true = Replace same-name in place.
    pub replace: bool,
    /// Optionally bind the new clip to a motion state explicitly (else auto by name).
    pub bind_state: Option<AnimState>,
}

impl Default for AddAnimForm {
    fn default() -> Self {
        Self { file: None, name: String::new(), fps: 10.0, looping: true, replace: false, bind_state: None }
    }
}

/// Derive a clip name from a file stem, biased to a motion-state keyword (shared by the
/// modal and the quick add paths).
fn name_from_stem(stem: &str) -> String {
    let s = stem.to_lowercase();
    ["idle", "run", "walk", "jump", "fall", "die", "dash", "slide", "attack", "hurt", "climb"]
        .iter()
        .find(|k| s.contains(**k))
        .map(|k| k.to_string())
        .unwrap_or_else(|| s.split(|c: char| !c.is_alphanumeric()).find(|t| !t.is_empty()).unwrap_or("anim").to_string())
}

impl EditorApp {
    /// Modal "Add Animation" dialog (ADR-039 Phase 2). Lets the user set name / fps /
    /// loop / collision policy / state binding before adding. Imports as an autonomous
    /// horizontal strip and routes through the unified additive helper with push_undo.
    pub(crate) fn show_add_animation_dialog(&mut self, ctx: &egui::Context, pdir: &Option<PathBuf>) {
        if !self.show_add_anim_dialog {
            return;
        }
        let Some(sel) = self.selected_id else {
            self.show_add_anim_dialog = false;
            return;
        };

        let mut open = true;
        let mut do_add = false;
        egui::Window::new("Add Animation")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                // File picker row.
                ui.horizontal(|ui| {
                    ui.label("File:");
                    let fname = self.add_anim_form.file.as_ref()
                        .and_then(|p| p.file_name())
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "(none — Browse…)".to_string());
                    ui.label(egui::RichText::new(fname).color(egui::Color32::from_gray(180)));
                    if ui.button("Browse…").clicked()
                        && let Some(f) = rfd::FileDialog::new()
                            .set_title("Add Animation")
                            .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                            .pick_file()
                    {
                        if self.add_anim_form.name.trim().is_empty() {
                            let stem = f.file_stem().unwrap_or_default().to_string_lossy().to_string();
                            self.add_anim_form.name = name_from_stem(&stem);
                        }
                        self.add_anim_form.file = Some(f);
                    }
                });
                ui.label(egui::RichText::new("Imported as an autonomous horizontal strip (1 row).").size(9.0).color(egui::Color32::from_gray(120)));
                ui.add_space(6.0);

                egui::Grid::new("add_anim_form_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.add_anim_form.name);
                    ui.end_row();
                    ui.label("FPS");
                    ui.add(egui::DragValue::new(&mut self.add_anim_form.fps).range(1.0..=60.0).speed(0.5));
                    ui.end_row();
                    ui.label("Loop");
                    ui.checkbox(&mut self.add_anim_form.looping, "");
                    ui.end_row();
                    ui.label("Bind to state");
                    let bind_label = match &self.add_anim_form.bind_state {
                        None => "(auto by name)".to_string(),
                        Some(s) => crate::helpers::anim_state_label(s),
                    };
                    egui::ComboBox::from_id_salt("add_anim_bind_state").selected_text(bind_label).show_ui(ui, |ui| {
                        if ui.selectable_label(self.add_anim_form.bind_state.is_none(), "(auto by name)").clicked() {
                            self.add_anim_form.bind_state = None;
                        }
                        for s in [AnimState::Idle, AnimState::Walk, AnimState::Run, AnimState::Jump, AnimState::Fall] {
                            let lbl = crate::helpers::anim_state_label(&s);
                            if ui.selectable_label(self.add_anim_form.bind_state.as_ref() == Some(&s), lbl).clicked() {
                                self.add_anim_form.bind_state = Some(s.clone());
                            }
                        }
                    });
                    ui.end_row();
                });

                // Collision policy — only surfaced when the chosen name already exists.
                let name = self.add_anim_form.name.trim().to_string();
                let collides = !name.is_empty()
                    && self.scene.entities.iter().find(|e| e.id == sel)
                        .is_some_and(|e| e.animations.iter().any(|a| a.name == name));
                if collides {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("'{name}' already exists on this entity:")).size(11.0).color(egui::Color32::from_rgb(220, 180, 90)));
                    ui.horizontal(|ui| {
                        let mut replace = self.add_anim_form.replace;
                        ui.radio_value(&mut replace, false, "Keep both (add as variant)");
                        ui.radio_value(&mut replace, true, "Replace in place");
                        self.add_anim_form.replace = replace;
                    });
                } else {
                    // Keep the policy field consistent with the (hidden) UI: no collision → no replace.
                    self.add_anim_form.replace = false;
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let ready = self.add_anim_form.file.is_some() && !name.is_empty();
                    if ui
                        .add_enabled(ready, egui::Button::new("Add"))
                        .on_disabled_hover_text("Pick a file and enter a name first.")
                        .clicked()
                    {
                        do_add = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_add_anim_dialog = false;
                    }
                });
            });

        if !open {
            self.show_add_anim_dialog = false;
        }

        if do_add {
            // Build outside the window closure so we can take push_undo (&mut self).
            if let Some(file) = self.add_anim_form.file.clone() {
                let rel = pdir.as_ref()
                    .and_then(|pd| file.strip_prefix(pd).ok().map(|p| p.to_string_lossy().to_string()))
                    .unwrap_or_else(|| file.to_string_lossy().to_string());
                let frame_count = image::image_dimensions(&file)
                    .map(|(w, h)| if h > 0 && w > h { (w / h).max(1) } else { 1 })
                    .unwrap_or(1);
                let name = self.add_anim_form.name.trim().to_string();
                let policy = if self.add_anim_form.replace {
                    crate::helpers::AnimConflict::Replace
                } else {
                    crate::helpers::AnimConflict::KeepBoth
                };
                let bind_state = self.add_anim_form.bind_state.clone();
                let anim = AnimationData {
                    name,
                    frames: (0..frame_count).collect(),
                    fps: self.add_anim_form.fps,
                    looping: self.add_anim_form.looping,
                    sprite_file: Some(rel.clone()),
                    strip_frames: Some(frame_count),
                };
                self.push_undo();
                if let Some(e) = self.scene.find_entity_mut(sel) {
                    if e.sprite_path.is_empty() {
                        e.sprite_path = rel;
                    }
                    let stored = match crate::helpers::add_animation_to_entity(e, anim, policy) {
                        crate::helpers::AnimAddResult::Added(n) => n,
                        crate::helpers::AnimAddResult::Replaced(n) => n,
                    };
                    // Explicit binding overrides the name-based auto-binding.
                    if let Some(state) = bind_state {
                        e.animation_states.get_or_insert_with(AnimationStateMap::default).set_binding(state, stored.clone());
                    }
                    self.status_msg = format!("Added animation '{stored}'");
                }
                self.sprite_cache.clear();
            }
            self.show_add_anim_dialog = false;
            self.add_anim_form = AddAnimForm::default();
        }
    }
}
