//! "Replace base sprite" flow with a Keep-animations / Replace-all guard (ADR-039
//! Phase 3). Replacing an entity's base sprite used to wipe its animations + state
//! bindings unconditionally; now the user chooses, and is warned when keeping
//! grid-sourced clips whose frame indices may no longer line up with the new sheet.

use toile_scene::{AnimationData, EntityData, SourcingModel, SpriteSheetData};

use crate::editor_app::EditorApp;

/// A resolved new base sprite for an entity, built from an asset or a picked file.
pub(crate) struct SpriteSource {
    pub sprite_path: String,
    pub sheet: Option<SpriteSheetData>,
    pub width: f32,
    pub height: f32,
    /// Grid animations carried by the source's metadata (empty for a raw file).
    pub anims: Vec<AnimationData>,
}

/// What to do with the entity's existing animations when its base sprite changes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SpriteReplaceMode {
    /// Wipe sheet + clips + bindings and repopulate from the new source (fresh start).
    ReplaceAll,
    /// Swap only the base sprite (and its sheet); keep clips + bindings untouched.
    KeepAnimations,
}

/// Apply a base-sprite change to `entity` according to `mode` (ADR-039 Phase 3).
pub(crate) fn apply_sprite_replacement(entity: &mut EntityData, source: &SpriteSource, mode: SpriteReplaceMode) {
    // The base image always changes; what happens to the sheet/clips depends on the mode.
    entity.sprite_path = source.sprite_path.clone();
    match mode {
        SpriteReplaceMode::ReplaceAll => {
            // Fresh start: adopt the new sheet/dims and repopulate clips from the source.
            entity.sprite_sheet = source.sheet.clone();
            entity.width = source.width;
            entity.height = source.height;
            entity.animation_states = None;
            entity.animations = source.anims.clone();
            entity.default_animation = entity.animations.first().map(|a| a.name.clone());
            crate::helpers::auto_populate_missing_bindings(entity);
        }
        SpriteReplaceMode::KeepAnimations => {
            // Keep clips/bindings AND the sheet + dims they depend on — only the base image
            // pixels change. This keeps grid clips functional (they still index a valid
            // sheet) and leaves strip clips entirely unaffected. Grid clips may simply not
            // line up if the new image's layout differs (surfaced as a dialog warning).
        }
    }
}

/// Pending form for the modal "Replace base sprite" dialog (ADR-039 Phase 3).
#[derive(Default)]
pub(crate) struct ReplaceSpriteForm {
    pub source: Option<SpriteSource>,
    pub source_name: String,
    /// Entity the replacement was started for — captured so changing the selection while
    /// the dialog is open can't redirect the apply to the wrong entity.
    pub target_id: u64,
}

impl EditorApp {
    /// Start a base-sprite replacement on the current selection. If the entity has no
    /// animations there is nothing to lose, so it applies immediately; otherwise it opens
    /// the Keep / Replace-all dialog (ADR-039 Phase 3).
    pub(crate) fn begin_sprite_replace(&mut self, source: SpriteSource, name: String) {
        let Some(sel) = self.selected_id else {
            self.status_msg = "Select an entity first to replace its sprite".to_string();
            return;
        };
        let has_anims = self.scene.entities.iter().find(|e| e.id == sel).is_some_and(|e| !e.animations.is_empty());
        if has_anims {
            self.replace_sprite_form = ReplaceSpriteForm { source: Some(source), source_name: name, target_id: sel };
            self.show_replace_sprite_dialog = true;
        } else {
            self.push_undo();
            if let Some(e) = self.scene.find_entity_mut(sel) {
                apply_sprite_replacement(e, &source, SpriteReplaceMode::ReplaceAll);
            }
            self.sprite_cache.clear();
            self.editor_mode = crate::editor_app::EditorMode::Entity;
            self.status_msg = format!("Set sprite to '{name}'");
        }
    }

    /// Modal "Replace base sprite" dialog — choose to keep or replace existing clips,
    /// with a warning when keeping grid-sourced clips (ADR-039 Phase 3).
    pub(crate) fn show_replace_sprite_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_replace_sprite_dialog {
            return;
        }
        // Operate on the entity the replacement was started for, not the live selection.
        let target = self.replace_sprite_form.target_id;
        let entity_exists = self.scene.entities.iter().any(|e| e.id == target);
        if self.replace_sprite_form.source.is_none() || !entity_exists {
            self.show_replace_sprite_dialog = false;
            self.replace_sprite_form = ReplaceSpriteForm::default();
            return;
        }

        // Pull display facts about the target entity (read-only, disjoint from the form).
        let (anim_count, model) = self
            .scene
            .entities
            .iter()
            .find(|e| e.id == target)
            .map(|e| (e.animations.len(), toile_scene::detect_sourcing_model(&e.animations)))
            .unwrap_or((0, SourcingModel::None));

        let mut open = true;
        let mut chosen: Option<SpriteReplaceMode> = None;
        let mut cancel = false;
        egui::Window::new("Replace base sprite")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.label(format!("New sprite: {}", self.replace_sprite_form.source_name));
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("This entity has {anim_count} animation(s)."))
                        .color(egui::Color32::from_gray(190)),
                );
                // KeepAnimations preserves the sheet, so grid clips keep indexing it — but on a
                // new image whose layout may differ. Strip clips are genuinely unaffected.
                if matches!(model, SourcingModel::Grid | SourcingModel::Mixed) {
                    ui.label(
                        egui::RichText::new(
                            "⚠ Grid clips will keep slicing the current grid over the new image, so frames may not line up if its layout differs. Strip clips are unaffected.",
                        )
                        .size(11.0)
                        .color(egui::Color32::from_rgb(220, 180, 90)),
                    );
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("Keep animations")
                        .on_hover_text("Change only the base image; keep the clips, sheet and state bindings")
                        .clicked()
                    {
                        chosen = Some(SpriteReplaceMode::KeepAnimations);
                    }
                    if ui
                        .button("Replace all")
                        .on_hover_text("Wipe the clips/bindings and repopulate from the new sprite")
                        .clicked()
                    {
                        chosen = Some(SpriteReplaceMode::ReplaceAll);
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });

        if let Some(mode) = chosen {
            if let Some(source) = self.replace_sprite_form.source.take() {
                self.push_undo();
                if let Some(e) = self.scene.find_entity_mut(target) {
                    apply_sprite_replacement(e, &source, mode);
                }
                self.sprite_cache.clear();
                self.editor_mode = crate::editor_app::EditorMode::Entity;
                let what = match mode {
                    SpriteReplaceMode::KeepAnimations => "kept animations",
                    SpriteReplaceMode::ReplaceAll => "replaced all",
                };
                self.status_msg = format!("Sprite replaced ({what})");
            }
            self.show_replace_sprite_dialog = false;
            self.replace_sprite_form = ReplaceSpriteForm::default();
        } else if cancel || !open {
            // Closing without applying must not leave a stale source in the form.
            self.show_replace_sprite_dialog = false;
            self.replace_sprite_form = ReplaceSpriteForm::default();
        }
    }
}
