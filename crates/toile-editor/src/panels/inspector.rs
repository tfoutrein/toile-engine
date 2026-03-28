use std::path::PathBuf;

use toile_behaviors::BehaviorConfig;

use crate::editor_app::{EditorApp, EditorMode};
use crate::helpers::*;

impl EditorApp {
    /// Show the entity inspector side panel.
    /// Returns `true` if the user clicked "Delete Entity".
    pub(crate) fn show_inspector(
        &mut self,
        ctx: &egui::Context,
        pdir: &Option<PathBuf>,
        project_scripts: &[String],
        project_particles: &[String],
    ) -> bool {
        let mut delete_selected = false;

        if self.editor_mode != EditorMode::Particle && self.editor_mode != EditorMode::SpriteAnim && self.editor_mode != EditorMode::AssetBrowser && self.editor_mode != EditorMode::AICopilot {
        egui::SidePanel::right("inspector").min_width(280.0).default_width(300.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Inspector");
            ui.separator();
            if let Some(id) = self.selected_id {
                if let Some(entity) = self.scene.find_entity_mut(id) {
                    egui::Grid::new("inspector_grid")
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("ID");
                            ui.label(format!("{}", entity.id));
                            ui.end_row();

                            ui.label("Name");
                            ui.text_edit_singleline(&mut entity.name);
                            ui.end_row();
                        });

                    // ── Role ─────────────────────────────────────────────
                    ui.add_space(4.0);
                    let is_player = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("player"));
                    let is_solid = entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Solid));
                    let is_coin = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("coin"));
                    let is_enemy = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("enemy"));

                    let current_role = if is_player {
                        if entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Platform(_))) {
                            "player_platformer"
                        } else if entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::TopDown(_))) {
                            "player_topdown"
                        } else { "player_custom" }
                    } else if is_solid { "solid"
                    } else if is_coin { "collectible"
                    } else if is_enemy { "enemy"
                    } else { "object" };

                    let role_display = match current_role {
                        "player_platformer" => "🧑 Player (Platformer)",
                        "player_topdown"    => "🧑 Player (Top-Down)",
                        "player_custom"     => "🧑 Player (Custom)",
                        "solid"             => "🧱 Ground / Wall",
                        "collectible"       => "⭐ Collectible",
                        "enemy"             => "👾 Enemy",
                        _                   => "📦 Object",
                    };

                    ui.horizontal(|ui| {
                        ui.label("Role:");
                        let mut new_role = String::new();
                        egui::ComboBox::from_id_salt("role_picker")
                            .width(180.0)
                            .selected_text(role_display)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(current_role == "object", "📦 Object — no special role").clicked() {
                                    new_role = "object".to_string();
                                }
                                ui.separator();
                                ui.label(egui::RichText::new("Player").size(11.0).color(egui::Color32::from_gray(150)));
                                if ui.selectable_label(current_role == "player_platformer", "🧑 Platformer — move + jump").clicked() {
                                    new_role = "player_platformer".to_string();
                                }
                                if ui.selectable_label(current_role == "player_topdown", "🧑 Top-Down — 4/8 directions").clicked() {
                                    new_role = "player_topdown".to_string();
                                }
                                ui.separator();
                                ui.label(egui::RichText::new("Environment").size(11.0).color(egui::Color32::from_gray(150)));
                                if ui.selectable_label(current_role == "solid", "🧱 Ground / Wall — blocks player").clicked() {
                                    new_role = "solid".to_string();
                                }
                                ui.separator();
                                ui.label(egui::RichText::new("Game objects").size(11.0).color(egui::Color32::from_gray(150)));
                                if ui.selectable_label(current_role == "collectible", "⭐ Collectible — coin, gem, power-up").clicked() {
                                    new_role = "collectible".to_string();
                                }
                                if ui.selectable_label(current_role == "enemy", "👾 Enemy").clicked() {
                                    new_role = "enemy".to_string();
                                }
                            });

                        if !new_role.is_empty() {
                            // Clear previous role-related tags and behaviors
                            entity.tags.retain(|t| {
                                let low = t.to_lowercase();
                                low != "player" && low != "solid" && low != "coin" && low != "enemy"
                            });
                            entity.behaviors.retain(|b| !matches!(b,
                                BehaviorConfig::Platform(_) | BehaviorConfig::TopDown(_) | BehaviorConfig::Solid
                            ));

                            match new_role.as_str() {
                                "player_platformer" => {
                                    entity.tags.push("Player".to_string());
                                    entity.behaviors.insert(0, BehaviorConfig::Platform(Default::default()));
                                }
                                "player_topdown" => {
                                    entity.tags.push("Player".to_string());
                                    entity.behaviors.insert(0, BehaviorConfig::TopDown(Default::default()));
                                }
                                "solid" => {
                                    entity.tags.push("Solid".to_string());
                                    entity.behaviors.insert(0, BehaviorConfig::Solid);
                                }
                                "collectible" => {
                                    entity.tags.push("Coin".to_string());
                                    // Add a gentle bob animation by default
                                    if !entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Sine(_))) {
                                        entity.behaviors.push(BehaviorConfig::Sine(toile_behaviors::sine::SineConfig {
                                            property: toile_behaviors::sine::SineProperty::Y,
                                            magnitude: 5.0,
                                            period: 1.5,
                                        }));
                                    }
                                }
                                "enemy" => {
                                    entity.tags.push("Enemy".to_string());
                                }
                                _ => {} // "object" — already cleared
                            }
                        }
                    });

                    // Role description
                    let role_hint = match current_role {
                        "player_platformer" => "← → move, Space jump (double-jump). Blocked by Ground/Wall entities.",
                        "player_topdown" => "← → ↑ ↓ or WASD, 8 directions with diagonal correction.",
                        "solid" => "Blocks Player movement. Use as floors, walls, platforms.",
                        "collectible" => "Tag: Coin. Use event sheets to handle collection (OnCollisionWith Player → Destroy).",
                        "enemy" => "Tag: Enemy. Add behaviors (Bullet, Sine…) and event sheets for interactions.",
                        _ => "",
                    };
                    if !role_hint.is_empty() {
                        ui.label(egui::RichText::new(role_hint).size(10.0).color(egui::Color32::from_gray(140)));
                    }

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Transform").strong());
                    ui.separator();

                    egui::Grid::new("transform_grid")
                        .num_columns(4)
                        .spacing([4.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("X");
                            ui.add(egui::DragValue::new(&mut entity.x).speed(1.0).min_decimals(0));
                            ui.label("Y");
                            ui.add(egui::DragValue::new(&mut entity.y).speed(1.0).min_decimals(0));
                            ui.end_row();

                            ui.label("Rot");
                            ui.add(egui::DragValue::new(&mut entity.rotation).speed(0.1).suffix("°"));
                            ui.label("");
                            ui.label("");
                            ui.end_row();

                            ui.label("Sx");
                            ui.add(egui::DragValue::new(&mut entity.scale_x).speed(0.05).min_decimals(1));
                            ui.label("Sy");
                            ui.add(egui::DragValue::new(&mut entity.scale_y).speed(0.05).min_decimals(1));
                            ui.end_row();
                        });

                    // ── Sprite & Display ──────────────────────────────────
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Display").strong());
                    ui.separator();

                    egui::Grid::new("sprite_grid")
                        .num_columns(4)
                        .spacing([4.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("W");
                            ui.add(egui::DragValue::new(&mut entity.width).speed(1.0).min_decimals(0));
                            ui.label("H");
                            ui.add(egui::DragValue::new(&mut entity.height).speed(1.0).min_decimals(0));
                            ui.end_row();

                            ui.label("Layer");
                            ui.add(egui::DragValue::new(&mut entity.layer));
                            ui.label("Vis");
                            ui.checkbox(&mut entity.visible, "");
                            ui.end_row();

                            ui.label("Sprite");
                            ui.add_sized([120.0, 18.0], egui::TextEdit::singleline(&mut entity.sprite_path));
                            if ui.small_button("Browse").clicked() {
                                if let Some(file) = rfd::FileDialog::new()
                                    .set_title("Select Sprite Image")
                                    .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                                    .pick_file()
                                {
                                    entity.sprite_path = if let Some(pd) = pdir {
                                        file.strip_prefix(pd)
                                            .map(|p| p.to_string_lossy().to_string())
                                            .unwrap_or_else(|_| file.to_string_lossy().to_string())
                                    } else {
                                        file.to_string_lossy().to_string()
                                    };
                                }
                            }
                            ui.label("");
                            ui.end_row();
                        });

                    // Frame picker for spritesheet preview
                    if let Some(ref sheet) = entity.sprite_sheet {
                        let total_frames = sheet.columns * sheet.rows;
                        if total_frames > 1 {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Preview frame:").size(11.0));
                                let mut frame = entity.preview_frame.unwrap_or(0) as i32;
                                if ui.add(egui::DragValue::new(&mut frame)
                                    .range(0..=(total_frames as i32 - 1))
                                    .speed(0.2)
                                ).changed() {
                                    entity.preview_frame = Some(frame.max(0) as u32);
                                }
                                ui.label(egui::RichText::new(format!("/ {}", total_frames - 1)).size(10.0).color(egui::Color32::from_gray(130)));
                                if ui.small_button("Edit Sprite").on_hover_text("Open Sprite & Animation Editor").clicked() {
                                    self.show_sprite_editor = true;
                                }
                            });
                        }
                    } else if !entity.sprite_path.is_empty() {
                        ui.horizontal(|ui| {
                            if ui.small_button("Edit Sprite").on_hover_text("Open Sprite & Animation Editor").clicked() {
                                self.show_sprite_editor = true;
                            }
                        });
                    }

                    // ── Behaviors ─────────────────────────────────────────
                    ui.add_space(8.0);
                    egui::CollapsingHeader::new(egui::RichText::new("Behaviors").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            let mut remove_idx: Option<usize> = None;
                            for (i, beh) in entity.behaviors.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(behavior_label(beh));
                                    if ui.small_button("x").clicked() {
                                        remove_idx = Some(i);
                                    }
                                });
                                behavior_inspector(ui, beh, i);
                                ui.separator();
                            }
                            if let Some(idx) = remove_idx {
                                entity.behaviors.remove(idx);
                            }
                            // Add behavior combo
                            let mut add_choice = String::new();
                            egui::ComboBox::from_id_salt("add_behavior")
                                .selected_text("+ Add Behavior")
                                .show_ui(ui, |ui| {
                                    for name in &["Platform", "TopDown", "Bullet", "Sine", "Fade", "Wrap", "Solid"] {
                                        if ui.selectable_label(false, *name).clicked() {
                                            add_choice = name.to_string();
                                        }
                                    }
                                });
                            if !add_choice.is_empty() {
                                entity.behaviors.push(default_behavior_config(&add_choice));
                            }
                        });

                    // ── Tags ─────────────────────────────────────────────
                    ui.add_space(4.0);
                    egui::CollapsingHeader::new(egui::RichText::new("Tags").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            let mut remove_tag: Option<usize> = None;
                            for (i, tag) in entity.tags.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(tag).monospace()
                                        .background_color(egui::Color32::from_gray(50)));
                                    if ui.small_button("x").clicked() {
                                        remove_tag = Some(i);
                                    }
                                });
                            }
                            if let Some(idx) = remove_tag {
                                entity.tags.remove(idx);
                            }
                            ui.horizontal(|ui| {
                                // Inline quick-add for common tags
                                for tag in &["Player", "Solid", "Coin", "Enemy"] {
                                    if !entity.tags.iter().any(|t| t == tag) {
                                        if ui.small_button(format!("+{tag}")).clicked() {
                                            entity.tags.push(tag.to_string());
                                        }
                                    }
                                }
                            });
                        });

                    // ── Variables ─────────────────────────────────────────
                    egui::CollapsingHeader::new(egui::RichText::new("Variables").strong())
                        .default_open(false)
                        .show(ui, |ui| {
                            let keys: Vec<String> = entity.variables.keys().cloned().collect();
                            let mut remove_key: Option<String> = None;
                            for key in &keys {
                                ui.horizontal(|ui| {
                                    ui.label(key);
                                    if let Some(v) = entity.variables.get_mut(key) {
                                        ui.add(egui::DragValue::new(v).speed(0.1));
                                    }
                                    if ui.small_button("x").clicked() {
                                        remove_key = Some(key.clone());
                                    }
                                });
                            }
                            if let Some(k) = remove_key {
                                entity.variables.remove(&k);
                            }
                            if ui.button("+ Add Variable").clicked() {
                                let name = format!("var{}", entity.variables.len());
                                entity.variables.insert(name, 0.0);
                            }
                        });

                    // ── Collision ─────────────────────────────────────────
                    egui::CollapsingHeader::new(egui::RichText::new("Collision").strong())
                        .default_open(false)
                        .show(ui, |ui| {
                            let has_collider = entity.collider.is_some();
                            let mut enabled = has_collider;
                            if ui.checkbox(&mut enabled, "Enable collider").changed() {
                                if enabled && entity.collider.is_none() {
                                    entity.collider = Some(toile_scene::ColliderData::Aabb {
                                        half_w: entity.width * 0.5,
                                        half_h: entity.height * 0.5,
                                    });
                                } else if !enabled {
                                    entity.collider = None;
                                }
                            }
                            if let Some(ref mut col) = entity.collider {
                                match col {
                                    toile_scene::ColliderData::Aabb { half_w, half_h } => {
                                        ui.label("AABB");
                                        ui.horizontal(|ui| {
                                            ui.label("Half W:");
                                            ui.add(egui::DragValue::new(half_w).speed(0.5).range(0.5..=1000.0));
                                            ui.label("Half H:");
                                            ui.add(egui::DragValue::new(half_h).speed(0.5).range(0.5..=1000.0));
                                        });
                                        if ui.button("Switch to Circle").clicked() {
                                            *col = toile_scene::ColliderData::Circle { radius: (*half_w).max(*half_h) };
                                        }
                                    }
                                    toile_scene::ColliderData::Circle { radius } => {
                                        ui.label("Circle");
                                        ui.horizontal(|ui| {
                                            ui.label("Radius:");
                                            ui.add(egui::DragValue::new(radius).speed(0.5).range(0.5..=1000.0));
                                        });
                                        if ui.button("Switch to AABB").clicked() {
                                            *col = toile_scene::ColliderData::Aabb { half_w: *radius, half_h: *radius };
                                        }
                                    }
                                }
                            }
                        });

                    // ── Sprite & Animation summary + edit button ─────────
                    if !entity.sprite_path.is_empty() || !entity.animations.is_empty() {
                        ui.add_space(4.0);
                        egui::CollapsingHeader::new(egui::RichText::new("Sprite & Animations").strong())
                            .default_open(true)
                            .show(ui, |ui| {
                                // Summary
                                if !entity.sprite_path.is_empty() {
                                    let short = entity.sprite_path.rsplit('/').next().unwrap_or(&entity.sprite_path);
                                    ui.label(egui::RichText::new(format!("🖼 {short}")).size(11.0));
                                }
                                if let Some(ref sheet) = entity.sprite_sheet {
                                    ui.label(egui::RichText::new(format!("Grid: {}×{} ({}×{}px frames)", sheet.columns, sheet.rows, sheet.frame_width, sheet.frame_height)).size(10.0).color(egui::Color32::from_gray(150)));
                                }
                                for anim in &entity.animations {
                                    let file_hint = anim.sprite_file.as_ref().map(|f| {
                                        let short = f.rsplit('/').next().unwrap_or(f);
                                        format!(" [{short}]")
                                    }).unwrap_or_default();
                                    let default_marker = if entity.default_animation.as_deref() == Some(&anim.name) { " ★" } else { "" };
                                    ui.label(egui::RichText::new(format!("  ▶ {} — {} frames, {}fps{}{}", anim.name, anim.frames.len(), anim.fps, file_hint, default_marker)).size(10.0).color(egui::Color32::from_gray(170)));
                                }
                                ui.add_space(4.0);
                                if ui.button("Edit Sprite & Animations...").clicked() {
                                    self.editor_mode = EditorMode::SpriteAnim;
                                }
                            });
                    } else {
                        ui.add_space(4.0);
                        if ui.button("Setup Sprite & Animations...").clicked() {
                            self.editor_mode = EditorMode::SpriteAnim;
                        }
                    }

                    egui::CollapsingHeader::new(egui::RichText::new("Event Sheet").strong())
                        .default_open(false)
                        .show(ui, |ui| {
                            let current = entity.event_sheet.clone().unwrap_or_default();
                            ui.label(if current.is_empty() { "None" } else { &current });
                            // Picker
                            egui::ComboBox::from_id_salt("event_sheet_picker")
                                .selected_text(if current.is_empty() { "Select..." } else { &current })
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(current.is_empty(), "(None)").clicked() {
                                        entity.event_sheet = None;
                                    }
                                    for f in project_scripts {
                                        if ui.selectable_label(*f == current, f).clicked() {
                                            entity.event_sheet = Some(f.clone());
                                        }
                                    }
                                });
                            ui.horizontal(|ui| {
                                if entity.event_sheet.is_some() {
                                    if ui.small_button("Clear").clicked() {
                                        entity.event_sheet = None;
                                    }
                                }
                                // Create new event sheet
                                if ui.small_button("+ New").clicked() {
                                    let name = format!("{}.event.json", entity.name.to_lowercase().replace(' ', "_"));
                                    let rel_path = format!("scripts/{name}");
                                    let full_path = pdir.as_ref().map(|d| d.join(&rel_path)).unwrap_or_else(|| PathBuf::from(&rel_path));
                                    if let Some(parent) = full_path.parent() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }
                                    if !full_path.exists() {
                                        let empty_sheet = serde_json::json!({
                                            "name": entity.name,
                                            "events": []
                                        });
                                        let _ = std::fs::write(&full_path, serde_json::to_string_pretty(&empty_sheet).unwrap());
                                    }
                                    entity.event_sheet = Some(rel_path);
                                }
                            });
                        });

                    // ── Particle Emitter ──────────────────────────────────
                    egui::CollapsingHeader::new(egui::RichText::new("Particle Emitter").strong())
                        .default_open(false)
                        .show(ui, |ui| {
                            let current = entity.particle_emitter.clone().unwrap_or_default();
                            ui.label(if current.is_empty() { "None" } else { &current });
                            // Picker
                            egui::ComboBox::from_id_salt("particle_picker")
                                .selected_text(if current.is_empty() { "Select..." } else { &current })
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(current.is_empty(), "(None)").clicked() {
                                        entity.particle_emitter = None;
                                    }
                                    for f in project_particles {
                                        if ui.selectable_label(*f == current, f).clicked() {
                                            entity.particle_emitter = Some(f.clone());
                                        }
                                    }
                                });
                            ui.horizontal(|ui| {
                                if entity.particle_emitter.is_some() {
                                    if ui.small_button("Clear").clicked() {
                                        entity.particle_emitter = None;
                                    }
                                }
                                // Create new particle emitter from preset
                                let mut create_preset = String::new();
                                egui::ComboBox::from_id_salt("new_particle_preset")
                                    .selected_text("+ New from preset")
                                    .width(130.0)
                                    .show_ui(ui, |ui| {
                                        for name in &["Fire", "Smoke", "Sparks", "Rain", "Snow", "Dust", "Explosion", "Confetti"] {
                                            if ui.selectable_label(false, *name).clicked() {
                                                create_preset = name.to_string();
                                            }
                                        }
                                    });
                                if !create_preset.is_empty() {
                                    let fname = format!("{}.particles.json", create_preset.to_lowercase());
                                    let rel_path = format!("particles/{fname}");
                                    let full_path = pdir.as_ref().map(|d| d.join(&rel_path)).unwrap_or_else(|| PathBuf::from(&rel_path));
                                    if let Some(parent) = full_path.parent() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }
                                    if !full_path.exists() {
                                        let emitter = match create_preset.as_str() {
                                            "Fire"      => toile_core::particles::presets::fire(),
                                            "Smoke"     => toile_core::particles::presets::smoke(),
                                            "Sparks"    => toile_core::particles::presets::sparks(),
                                            "Rain"      => toile_core::particles::presets::rain(),
                                            "Snow"      => toile_core::particles::presets::snow(),
                                            "Dust"      => toile_core::particles::presets::dust(),
                                            "Explosion" => toile_core::particles::presets::explosion(),
                                            "Confetti"  => toile_core::particles::presets::confetti(),
                                            _           => toile_core::particles::ParticleEmitter::default(),
                                        };
                                        if let Ok(json) = serde_json::to_string_pretty(&emitter) {
                                            let _ = std::fs::write(&full_path, &json);
                                        }
                                    }
                                    entity.particle_emitter = Some(rel_path);
                                }
                            });
                        });

                    // ── Light ─────────────────────────────────────────────
                    egui::CollapsingHeader::new(egui::RichText::new("Light").strong())
                        .default_open(false)
                        .show(ui, |ui| {
                            let has_light = entity.light.is_some();
                            let mut enabled = has_light;
                            if ui.checkbox(&mut enabled, "Point light").changed() {
                                if enabled && entity.light.is_none() {
                                    entity.light = Some(toile_scene::LightData::default());
                                } else if !enabled {
                                    entity.light = None;
                                }
                            }
                            if let Some(ref mut light) = entity.light {
                                egui::Grid::new("light_grid").num_columns(2).show(ui, |ui| {
                                    ui.label("Radius");
                                    ui.add(egui::DragValue::new(&mut light.radius).speed(1.0).range(1.0..=2000.0));
                                    ui.end_row();
                                    ui.label("Falloff");
                                    ui.add(egui::DragValue::new(&mut light.falloff).speed(0.1).range(0.1..=10.0));
                                    ui.end_row();
                                    ui.label("Intensity");
                                    ui.add(egui::DragValue::new(&mut light.intensity).speed(0.05).range(0.0..=10.0));
                                    ui.end_row();
                                    ui.label("Color R");
                                    ui.add(egui::Slider::new(&mut light.color[0], 0.0..=1.0));
                                    ui.end_row();
                                    ui.label("Color G");
                                    ui.add(egui::Slider::new(&mut light.color[1], 0.0..=1.0));
                                    ui.end_row();
                                    ui.label("Color B");
                                    ui.add(egui::Slider::new(&mut light.color[2], 0.0..=1.0));
                                    ui.end_row();
                                    ui.label("Shadows");
                                    ui.checkbox(&mut light.cast_shadow, "Cast shadow");
                                    ui.end_row();
                                });
                            }
                        });

                    // ── Delete button ─────────────────────────────────────
                    ui.add_space(12.0);
                    if ui.button(egui::RichText::new("Delete Entity").color(egui::Color32::from_rgb(255, 80, 80))).clicked() {
                        delete_selected = true;
                    }
                } else {
                    self.selected_id = None;
                    ui.label("No entity selected");
                }
            } else {
                ui.label("No entity selected");
            }
            }); // end ScrollArea
        });
        } // end `if self.editor_mode != EditorMode::Particle`

        delete_selected
    }
}
