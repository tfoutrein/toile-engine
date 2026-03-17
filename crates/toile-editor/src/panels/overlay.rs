use std::path::{Path, PathBuf};

use glam::Vec2;

use crate::editor_app::{EditorApp, EditorMode};
use crate::scene_data::SceneData;
use crate::tilemap_tool::{self, TileTool};
use crate::helpers::*;

impl EditorApp {
    /// Show all overlay panels: welcome dialog, menu bar, hierarchy, inspector
    /// delegations, scene settings, tilemap tools, and status bar.
    ///
    /// Called from `render_overlay()` after the egui frame has been started.
    pub(crate) fn show_overlay_panels(
        &mut self,
        ctx: &egui::Context,
        project_scenes: &[String],
        project_scripts: &[String],
        project_particles: &[String],
        pdir: &Option<PathBuf>,
    ) {
        // Set grab cursor while panning
        if self.panning {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        // ── Welcome / Project dialog ─────────────────────────────────────
        if self.project_dir.is_none() {
            let mut action_create: Option<PathBuf> = None;
            let mut action_open: Option<PathBuf> = None;

            egui::CentralPanel::default().show(ctx, |ui| {
                let panel_width = 420.0_f32;
                let avail = ui.available_width();
                let margin = ((avail - panel_width) * 0.5).max(0.0);

                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(egui::RichText::new("Toile Editor").size(28.0).strong());
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Open or create a project to begin.").size(13.0).color(egui::Color32::from_gray(160)));
                    ui.add_space(24.0);
                });

                // Centered fixed-width container
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.vertical(|ui| {
                        ui.set_max_width(panel_width);

                        // ── New Project ──
                        ui.group(|ui| {
                            ui.set_min_width(panel_width - 20.0);
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("New Project").strong().size(15.0));
                            });
                            ui.add_space(6.0);
                            egui::Grid::new("new_proj_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                                ui.label("Name:");
                                ui.add_sized([280.0, 20.0], egui::TextEdit::singleline(&mut self.new_project_name));
                                ui.end_row();
                                ui.label("Template:");
                                egui::ComboBox::from_id_salt("template_combo")
                                    .width(280.0)
                                    .selected_text(&self.new_project_template)
                                    .show_ui(ui, |ui| {
                                        for t in &["empty", "platformer", "topdown", "shmup"] {
                                            ui.selectable_value(&mut self.new_project_template, t.to_string(), *t);
                                        }
                                    });
                                ui.end_row();
                            });
                            ui.add_space(6.0);
                            ui.vertical_centered(|ui| {
                                if ui.button("  Create Project  ").clicked() {
                                    action_create = Some(PathBuf::from(&self.new_project_name));
                                }
                            });
                        });

                        ui.add_space(12.0);

                        // ── Open Project ──
                        ui.group(|ui| {
                            ui.set_min_width(panel_width - 20.0);
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("Open Project").strong().size(15.0));
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label("Path:");
                                ui.add_sized([260.0, 20.0], egui::TextEdit::singleline(&mut self.project_path_input));
                                if ui.button("Browse...").clicked() {
                                    if let Some(dir) = rfd::FileDialog::new()
                                        .set_title("Open Toile Project")
                                        .pick_folder()
                                    {
                                        self.project_path_input = dir.to_string_lossy().to_string();
                                    }
                                }
                            });

                            // Scan for directories with Toile.toml nearby
                            let mut found_projects: Vec<String> = Vec::new();
                            if let Ok(entries) = std::fs::read_dir(".") {
                                for entry in entries.flatten() {
                                    let p = entry.path();
                                    if p.is_dir() && p.join("Toile.toml").exists() {
                                        if let Some(name) = p.file_name() {
                                            found_projects.push(name.to_string_lossy().to_string());
                                        }
                                    }
                                }
                            }
                            if Path::new("examples/run-demo/Toile.toml").exists() {
                                found_projects.push("examples/run-demo".to_string());
                            }

                            if !found_projects.is_empty() {
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new("Recent projects:").size(11.0).color(egui::Color32::from_gray(140)));
                                for proj in &found_projects {
                                    if ui.selectable_label(self.project_path_input == *proj, proj).clicked() {
                                        self.project_path_input = proj.clone();
                                    }
                                }
                            }

                            ui.add_space(6.0);
                            ui.vertical_centered(|ui| {
                                if ui.button("  Open  ").clicked() && !self.project_path_input.is_empty() {
                                    action_open = Some(PathBuf::from(&self.project_path_input));
                                }
                            });
                        });

                        // Status
                        if !self.status_msg.is_empty() {
                            ui.add_space(16.0);
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new(&self.status_msg).color(egui::Color32::YELLOW).size(12.0));
                            });
                        }
                    });
                });
            });

            // Apply deferred actions (after egui panel)
            if let Some(dir) = action_create {
                if dir.exists() {
                    self.status_msg = format!("Directory '{}' already exists", dir.display());
                } else {
                    match self.create_project(&dir) {
                        Ok(()) => self.open_project(dir),
                        Err(e) => self.status_msg = format!("Error: {e}"),
                    }
                }
            }
            if let Some(dir) = action_open {
                if dir.join("Toile.toml").exists() {
                    self.open_project(dir);
                } else {
                    self.status_msg = format!("No Toile.toml found in '{}'", dir.display());
                }
            }
            return;
        }

        // Menu bar
        let mut new_scene = false;
        let mut save_scene = false;
        let mut _load_scene = false;
        let mut add_entity = false;
        let mut delete_selected = false;
        let mut play_game = false;

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Scene").clicked() { new_scene = true; ui.close_menu(); }
                    // Scene switcher
                    if !project_scenes.is_empty() {
                        ui.menu_button("Open Scene", |ui| {
                            for s in project_scenes {
                                let is_current = self.current_file == *s;
                                if ui.selectable_label(is_current, s).clicked() {
                                    let path = pdir.as_ref().map(|d| d.join(s)).unwrap_or_else(|| PathBuf::from(s));
                                    match toile_scene::load_scene(&path) {
                                        Ok(scene) => {
                                            self.camera_zoom = scene.settings.camera_zoom;
                                            self.camera_pos = Vec2::ZERO;
                                            self.scene = scene;
                                            self.current_file = s.clone();
                                            self.selected_id = None;
                                            self.status_msg = format!("Loaded {s}");
                                        }
                                        Err(e) => self.status_msg = format!("Error: {e}"),
                                    }
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                    ui.separator();
                    if ui.button("Save...").clicked() {
                        self.file_path_input = self.current_file.clone();
                        self.show_save_dialog = true;
                        ui.close_menu();
                    }
                    if !self.current_file.is_empty() {
                        if ui.button(format!("Quick Save ({})", self.current_file)).clicked() {
                            save_scene = true;
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Close Project").clicked() {
                        self.project_dir = None;
                        self.show_project_dialog = true;
                        self.scene = SceneData::new("Untitled");
                        self.selected_id = None;
                        self.current_file.clear();
                        self.status_msg = "Project closed".to_string();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Add Entity").clicked() { add_entity = true; ui.close_menu(); }
                    if ui.button("Delete Selected").clicked() { delete_selected = true; ui.close_menu(); }
                });
                ui.separator();
                // Mode toggle
                let entity_label  = if self.editor_mode == EditorMode::Entity   { "[ Entity ]"   } else { "Entity" };
                let tilemap_label = if self.editor_mode == EditorMode::Tilemap  { "[ Tilemap ]"  } else { "Tilemap" };
                let particle_label = if self.editor_mode == EditorMode::Particle { "[ Particles ]" } else { "Particles" };
                let assets_label = if self.editor_mode == EditorMode::AssetBrowser { "[ Assets ]" } else { "Assets" };
                if ui.button(entity_label).clicked() {
                    self.editor_mode = EditorMode::Entity;
                }
                if ui.button(tilemap_label).clicked() {
                    self.editor_mode = EditorMode::Tilemap;
                    // Create default tilemap if none exists
                    if self.scene.tilemap.is_none() {
                        self.scene.tilemap = Some(tilemap_tool::create_default_tilemap(
                            40, 23, 32, "assets/platformer/tileset.png", 4,
                        ));
                        self.status_msg = "Created 40x23 tilemap (1280x736px)".to_string();
                    }
                }
                if ui.button(particle_label).clicked() {
                    self.editor_mode = EditorMode::Particle;
                }
                if ui.button(assets_label).clicked() {
                    self.editor_mode = EditorMode::AssetBrowser;
                }
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_grid, "Show Grid");
                    ui.checkbox(&mut self.show_viewport_guide, "Show Player Viewport");
                    if ui.button("Scene Settings...").clicked() {
                        self.show_scene_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("Reset Camera").clicked() {
                        self.camera_pos = Vec2::ZERO;
                        self.camera_zoom = 1.0;
                        ui.close_menu();
                    }
                });
                // Play button — pushed to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("▶ Play").color(egui::Color32::from_rgb(80, 220, 80)).strong()).clicked() {
                        play_game = true;
                    }
                });
            });
        });

        // Apply menu actions
        if new_scene {
            self.scene = SceneData::new("Untitled");
            self.selected_id = None;
            self.status_msg = "New scene".to_string();
        }
        if save_scene && !self.current_file.is_empty() {
            let path = pdir.as_ref().map(|d| d.join(&self.current_file)).unwrap_or_else(|| PathBuf::from(&self.current_file));
            let json = serde_json::to_string_pretty(&self.scene).unwrap();
            match std::fs::write(&path, &json) {
                Ok(()) => self.status_msg = format!("Saved to {} ({} entities)", self.current_file, self.scene.entities.len()),
                Err(e) => self.status_msg = format!("Save failed: {e}"),
            }
        }

        // Load dialog
        if self.show_load_dialog {
            let mut open = true;
            // Scan for JSON files in current directory
            let json_files: Vec<String> = std::fs::read_dir(".")
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension().is_some_and(|ext| ext == "json")
                        && e.path().file_name().is_some_and(|n| n != ".mcp.json")
                })
                .filter_map(|e| e.file_name().into_string().ok())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();

            egui::Window::new("Load Scene")
                .open(&mut open)
                .collapsible(false)
                .default_width(350.0)
                .show(ctx, |ui| {
                    ui.label("File path:");
                    ui.text_edit_singleline(&mut self.file_path_input);

                    if !json_files.is_empty() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Available scenes:").strong());
                        ui.separator();
                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                            for file in &json_files {
                                let selected = self.file_path_input == *file;
                                if ui.selectable_label(selected, file).clicked() {
                                    self.file_path_input = file.clone();
                                }
                            }
                        });
                    }

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Load").clicked() {
                            let path = std::path::Path::new(&self.file_path_input);
                            match toile_scene::load_scene(path) {
                                Ok(data) => {
                                    self.scene = data;
                                    self.current_file = self.file_path_input.clone();
                                    self.selected_id = None;
                                    self.status_msg = format!("Loaded {}", self.current_file);
                                    self.show_load_dialog = false;
                                }
                                Err(e) => {
                                    self.status_msg = format!("Load failed: {e}");
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_load_dialog = false;
                        }
                    });
                });
            if !open { self.show_load_dialog = false; }
        }

        // Save dialog
        if self.show_save_dialog {
            let mut open = true;
            egui::Window::new("Save Scene")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Scene path (relative to project):");
                    ui.text_edit_singleline(&mut self.file_path_input);
                    // Quick pick from existing scenes
                    if !project_scenes.is_empty() {
                        ui.label(egui::RichText::new("Existing scenes:").size(11.0));
                        for s in project_scenes {
                            if ui.selectable_label(self.file_path_input == *s, s).clicked() {
                                self.file_path_input = s.clone();
                            }
                        }
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let path = pdir.as_ref().map(|d| d.join(&self.file_path_input)).unwrap_or_else(|| PathBuf::from(&self.file_path_input));
                            if let Some(parent) = path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let json = serde_json::to_string_pretty(&self.scene).unwrap();
                            match std::fs::write(&path, &json) {
                                Ok(()) => {
                                    self.current_file = self.file_path_input.clone();
                                    self.status_msg = format!("Saved to {}", self.current_file);
                                    self.show_save_dialog = false;
                                }
                                Err(e) => {
                                    self.status_msg = format!("Save failed: {e}");
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_save_dialog = false;
                        }
                    });
                });
            if !open { self.show_save_dialog = false; }
        }
        if add_entity {
            let id = self.scene.add_entity(
                &format!("Entity_{}", self.scene.next_id),
                self.camera_pos.x, self.camera_pos.y,
            );
            self.selected_id = Some(id);
            self.status_msg = format!("Created entity {id}");
        }
        if delete_selected {
            if let Some(id) = self.selected_id.take() {
                self.scene.remove_entity(id);
                self.status_msg = format!("Deleted entity {id}");
            }
        }
        if play_game {
            if let Some(dir) = pdir {
                // Auto-save before playing
                if !self.current_file.is_empty() {
                    let save_path = dir.join(&self.current_file);
                    if let Ok(json) = serde_json::to_string_pretty(&self.scene) {
                        let _ = std::fs::write(&save_path, &json);
                    }
                }
                // Spawn toile run as a child process
                match std::process::Command::new("toile")
                    .arg("run")
                    .arg(dir)
                    .spawn()
                {
                    Ok(_) => self.status_msg = "Game launched!".to_string(),
                    Err(e) => self.status_msg = format!("Failed to launch: {e}. Is `toile` in PATH? (cargo install --path crates/toile-cli)"),
                }
            } else {
                self.status_msg = "No project open".to_string();
            }
        }

        // Hierarchy panel — tree view: Game > Scenes > Entities
        // ── Asset Browser (full-screen mode) ─────────────────────────────
        if self.editor_mode == EditorMode::AssetBrowser {
            // Load registered packs on first use
            if !self.asset_browser.initialized {
                self.asset_browser.reload_registered_packs();
                self.asset_browser.initialized = true;
            }
            self.asset_browser.show_ui(ctx);
        }

        if self.editor_mode != EditorMode::Particle && self.editor_mode != EditorMode::SpriteAnim && self.editor_mode != EditorMode::AssetBrowser {
        egui::SidePanel::left("hierarchy").default_width(200.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
            // Project root
            let project_name = pdir.as_ref()
                .and_then(|d| d.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Game".to_string());

            let root_id = ui.make_persistent_id("hierarchy_root");
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), root_id, true)
                .show_header(ui, |ui| {
                    ui.label(egui::RichText::new(format!("\u{1f3ae} {project_name}")).strong());
                })
                .body(|ui| {
                    // ── Scenes ──
                    let scenes_id = ui.make_persistent_id("hierarchy_scenes");
                    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), scenes_id, true)
                        .show_header(ui, |ui| {
                            ui.label(egui::RichText::new("\u{1f4c1} Scenes").color(egui::Color32::from_rgb(180, 200, 255)));
                        })
                        .body(|ui| {
                            let mut switch_scene: Option<String> = None;
                            for scene_file in project_scenes {
                                let is_current = self.current_file == *scene_file;
                                let scene_name = scene_file.strip_prefix("scenes/").unwrap_or(scene_file);
                                let scene_name = scene_name.strip_suffix(".json").unwrap_or(scene_name);

                                let scene_node_id = ui.make_persistent_id(scene_file);
                                egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), scene_node_id, is_current)
                                    .show_header(ui, |ui| {
                                        let icon = if is_current { "\u{1f4c4}" } else { "\u{1f4c4}" };
                                        let color = if is_current { egui::Color32::YELLOW } else { egui::Color32::from_gray(200) };
                                        if ui.selectable_label(is_current, egui::RichText::new(format!("{icon} {scene_name}")).color(color)).clicked() {
                                            if !is_current {
                                                switch_scene = Some(scene_file.clone());
                                            }
                                        }
                                    })
                                    .body(|ui| {
                                        if is_current {
                                            // Show entities of the current scene with sub-components
                                            let mut click_id = None;
                                            for entity in &self.scene.entities {
                                                let selected = self.selected_id == Some(entity.id);
                                                let icon = entity_icon(entity);
                                                let has_children = !entity.behaviors.is_empty()
                                                    || entity.light.is_some()
                                                    || entity.particle_emitter.is_some()
                                                    || entity.event_sheet.is_some()
                                                    || entity.collider.is_some();

                                                if has_children {
                                                    let ent_node_id = ui.make_persistent_id(format!("ent_{}", entity.id));
                                                    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ent_node_id, false)
                                                        .show_header(ui, |ui| {
                                                            let color = if selected { egui::Color32::YELLOW } else { egui::Color32::WHITE };
                                                            if ui.selectable_label(selected, egui::RichText::new(format!("{icon} {}", entity.name)).color(color)).clicked() {
                                                                click_id = Some(entity.id);
                                                            }
                                                        })
                                                        .body(|ui| {
                                                            let dim = egui::Color32::from_gray(140);
                                                            for beh in &entity.behaviors {
                                                                ui.label(egui::RichText::new(format!("    \u{1f3ad} {}", behavior_label(beh))).size(11.0).color(dim));
                                                            }
                                                            if let Some(ref light) = entity.light {
                                                                ui.label(egui::RichText::new(format!("    \u{1f4a1} Light (r={:.0})", light.radius)).size(11.0).color(dim));
                                                            }
                                                            if let Some(ref pe) = entity.particle_emitter {
                                                                let short = pe.rsplit('/').next().unwrap_or(pe);
                                                                ui.label(egui::RichText::new(format!("    \u{2728} {short}")).size(11.0).color(dim));
                                                            }
                                                            if let Some(ref es) = entity.event_sheet {
                                                                let short = es.rsplit('/').next().unwrap_or(es);
                                                                ui.label(egui::RichText::new(format!("    \u{1f4dc} {short}")).size(11.0).color(dim));
                                                            }
                                                            if let Some(ref col) = entity.collider {
                                                                let shape = match col {
                                                                    toile_scene::ColliderData::Aabb { .. } => "AABB",
                                                                    toile_scene::ColliderData::Circle { .. } => "Circle",
                                                                };
                                                                ui.label(egui::RichText::new(format!("    \u{1f532} {shape}")).size(11.0).color(dim));
                                                            }
                                                        });
                                                } else {
                                                    // Simple leaf — no children
                                                    let color = if selected { egui::Color32::YELLOW } else { egui::Color32::WHITE };
                                                    if ui.selectable_label(selected, egui::RichText::new(format!("  {icon} {}", entity.name)).color(color)).clicked() {
                                                        click_id = Some(entity.id);
                                                    }
                                                }
                                            }
                                            if let Some(id) = click_id {
                                                self.selected_id = Some(id);
                                            }
                                        } else {
                                            ui.label(egui::RichText::new("(click to open)").size(10.0).color(egui::Color32::from_gray(120)));
                                        }
                                    });
                            }
                            // Switch scene if clicked
                            if let Some(scene_file) = switch_scene {
                                let path = pdir.as_ref().map(|d| d.join(&scene_file)).unwrap_or_else(|| PathBuf::from(&scene_file));
                                match toile_scene::load_scene(&path) {
                                    Ok(scene) => {
                                        self.camera_zoom = scene.settings.camera_zoom;
                                        self.camera_pos = Vec2::ZERO;
                                        self.scene = scene;
                                        self.current_file = scene_file;
                                        self.selected_id = None;
                                        self.status_msg = "Scene loaded".to_string();
                                    }
                                    Err(e) => self.status_msg = format!("Error: {e}"),
                                }
                            }

                            // New scene button
                            if ui.small_button("+ New Scene").clicked() {
                                let name = format!("scene_{}", project_scenes.len() + 1);
                                let path_str = format!("scenes/{name}.json");
                                let new_scene = SceneData::new(&name);
                                let full_path = pdir.as_ref().map(|d| d.join(&path_str)).unwrap_or_else(|| PathBuf::from(&path_str));
                                if let Ok(json) = serde_json::to_string_pretty(&new_scene) {
                                    let _ = std::fs::write(&full_path, &json);
                                }
                                self.scene = new_scene;
                                self.current_file = path_str;
                                self.selected_id = None;
                                self.status_msg = format!("Created scene '{name}'");
                            }
                        });

                    // ── Current scene entities (flat for quick access) ──
                    ui.separator();
                    ui.label(egui::RichText::new("Entities").size(11.0).color(egui::Color32::from_gray(150)));
                    if ui.button("+ Add Entity").clicked() {
                        let id = self.scene.add_entity(
                            &format!("Entity_{}", self.scene.next_id),
                            self.camera_pos.x, self.camera_pos.y,
                        );
                        self.selected_id = Some(id);
                    }
                });
            }); // end ScrollArea
        });
        } // end hierarchy panel

        // ── Sprite & Animation Editor (full-screen mode) ─────────────────
        self.show_sprite_anim_panels(ctx, pdir);

        // Inspector panel — replaced by particle panel in Particle mode
        if self.editor_mode == EditorMode::Particle {
            egui::SidePanel::right("inspector").min_width(320.0).max_width(320.0).show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.particle_editor.show(ui);
                });
            });
        }

        delete_selected |= self.show_inspector(ctx, pdir, project_scripts, project_particles);
        if delete_selected {
            if let Some(id) = self.selected_id.take() {
                self.scene.remove_entity(id);
                self.status_msg = format!("Deleted entity {id}");
            }
        }

        // ── Sprite & Animation Editor window ─────────────────────────────
        self.show_sprite_editor_window(ctx, pdir);

        // ── Frame Picker window ───────────────────────────────────────────
        self.show_frame_picker_window(ctx, pdir);

        // Scene Settings window
        if self.show_scene_settings {
            let mut open = true;
            egui::Window::new("Scene Settings")
                .open(&mut open)
                .default_width(300.0)
                .show(ctx, |ui| {
                    let s = &mut self.scene.settings;
                    egui::Grid::new("scene_settings_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Gravity");
                        ui.add(egui::DragValue::new(&mut s.gravity).speed(1.0));
                        ui.end_row();

                        ui.label("Viewport W");
                        ui.add(egui::DragValue::new(&mut s.viewport_width).range(320..=3840));
                        ui.end_row();

                        ui.label("Viewport H");
                        ui.add(egui::DragValue::new(&mut s.viewport_height).range(240..=2160));
                        ui.end_row();

                        ui.label("Camera Zoom");
                        ui.add(egui::DragValue::new(&mut s.camera_zoom).speed(0.1).range(0.1..=10.0));
                        ui.end_row();

                        ui.label("Camera Mode");
                        let mode_label = match &s.camera_mode {
                            toile_scene::CameraMode::Fixed => "Fixed",
                            toile_scene::CameraMode::FollowPlayer => "Follow Player",
                            toile_scene::CameraMode::PlatformerFollow { .. } => "Platformer Follow",
                        };
                        let mut new_mode: Option<toile_scene::CameraMode> = None;
                        egui::ComboBox::from_id_salt("camera_mode")
                            .selected_text(mode_label)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(mode_label == "Fixed", "Fixed \u{2014} camera stays at position").clicked() {
                                    new_mode = Some(toile_scene::CameraMode::Fixed);
                                }
                                if ui.selectable_label(mode_label == "Follow Player", "Follow Player \u{2014} always centered").clicked() {
                                    new_mode = Some(toile_scene::CameraMode::FollowPlayer);
                                }
                                if ui.selectable_label(mode_label == "Platformer Follow", "Platformer \u{2014} deadzone + bounds").clicked() {
                                    new_mode = Some(toile_scene::CameraMode::PlatformerFollow {
                                        deadzone_x: 0.3,
                                        deadzone_y: 0.4,
                                        bounds: [0.0; 4],
                                    });
                                }
                            });
                        if let Some(m) = new_mode { s.camera_mode = m; }
                        ui.end_row();

                        ui.label("Clear R");
                        ui.add(egui::Slider::new(&mut s.clear_color[0], 0.0..=1.0));
                        ui.end_row();
                        ui.label("Clear G");
                        ui.add(egui::Slider::new(&mut s.clear_color[1], 0.0..=1.0));
                        ui.end_row();
                        ui.label("Clear B");
                        ui.add(egui::Slider::new(&mut s.clear_color[2], 0.0..=1.0));
                        ui.end_row();
                    });

                    // Platformer camera settings
                    if let toile_scene::CameraMode::PlatformerFollow { deadzone_x, deadzone_y, bounds } = &mut s.camera_mode {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Platformer Camera").strong());
                        ui.separator();
                        egui::Grid::new("platformer_cam_grid").num_columns(2).show(ui, |ui| {
                            ui.label("Deadzone X");
                            ui.add(egui::Slider::new(deadzone_x, 0.0..=0.8).text("of viewport"));
                            ui.end_row();
                            ui.label("Deadzone Y");
                            ui.add(egui::Slider::new(deadzone_y, 0.0..=0.8).text("of viewport"));
                            ui.end_row();
                        });
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Scene Bounds (camera clamp)").size(11.0));
                        ui.horizontal(|ui| {
                            if ui.small_button("Set to viewport").clicked() {
                                let vw = s.viewport_width as f32 / s.camera_zoom;
                                let vh = s.viewport_height as f32 / s.camera_zoom;
                                let cx = s.camera_position[0];
                                let cy = s.camera_position[1];
                                *bounds = [cx - vw * 0.5, cy - vh * 0.5, cx + vw * 0.5, cy + vh * 0.5];
                            }
                            if !s.background_tiles.is_empty() {
                                if ui.small_button("Set to background").clicked() {
                                    let tw = s.viewport_width as f32 / s.camera_zoom;
                                    let th = s.viewport_height as f32 / s.camera_zoom;
                                    let hw = tw * 0.5;
                                    let hh = th * 0.5;
                                    let (mut mn_x, mut mn_y, mut mx_x, mut mx_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                                    for p in &s.background_tiles {
                                        mn_x = mn_x.min(p[0] - hw);
                                        mx_x = mx_x.max(p[0] + hw);
                                        mn_y = mn_y.min(p[1] - hh);
                                        mx_y = mx_y.max(p[1] + hh);
                                    }
                                    *bounds = [mn_x, mn_y, mx_x, mx_y];
                                }
                            }
                            if ui.small_button("Clear").clicked() {
                                *bounds = [0.0; 4];
                            }
                        });
                        egui::Grid::new("bounds_grid").num_columns(4).show(ui, |ui| {
                            ui.label("Min X");
                            ui.add(egui::DragValue::new(&mut bounds[0]).speed(1.0));
                            ui.label("Min Y");
                            ui.add(egui::DragValue::new(&mut bounds[1]).speed(1.0));
                            ui.end_row();
                            ui.label("Max X");
                            ui.add(egui::DragValue::new(&mut bounds[2]).speed(1.0));
                            ui.label("Max Y");
                            ui.add(egui::DragValue::new(&mut bounds[3]).speed(1.0));
                            ui.end_row();
                        });
                    }

                    // ── Background Image ──
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Background").strong());
                    ui.separator();
                    let mut bg_path = s.background_image.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label("Image:");
                        if ui.text_edit_singleline(&mut bg_path).changed() {
                            s.background_image = if bg_path.is_empty() { None } else { Some(bg_path.clone()) };
                        }
                        if ui.small_button("Browse...").clicked() {
                            if let Some(file) = rfd::FileDialog::new()
                                .set_title("Select Background Image")
                                .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                                .pick_file()
                            {
                                // Try to make relative to project dir
                                let path_str = if let Some(pd) = pdir {
                                    file.strip_prefix(pd)
                                        .map(|p| p.to_string_lossy().to_string())
                                        .unwrap_or_else(|_| file.to_string_lossy().to_string())
                                } else {
                                    file.to_string_lossy().to_string()
                                };
                                s.background_image = Some(path_str);
                            }
                        }
                    });
                    if s.background_image.is_some() {
                        ui.horizontal(|ui| {
                            if ui.small_button("Reset tiles").on_hover_text("Re-create the initial background tile at camera position").clicked() {
                                s.background_tiles.clear();
                                s.background_tiles.push(s.camera_position);
                                self.background_path_loaded.clear(); // force texture reload
                            }
                            if ui.small_button("Reload").on_hover_text("Force reload background + restore tiles if missing").clicked() {
                                self.background_tex = None;
                                self.background_path_loaded.clear();
                                self.sprite_cache.clear();
                                // Always ensure at least one tile exists
                                if s.background_tiles.is_empty() {
                                    s.background_tiles.push(s.camera_position);
                                }
                            }
                            ui.label(egui::RichText::new(format!("{} tile(s)", s.background_tiles.len())).size(10.0).color(egui::Color32::from_gray(140)));
                        });
                        if ui.small_button("Clear background").on_hover_text("Remove background image entirely").clicked() {
                            s.background_image = None;
                            s.background_tiles.clear();
                        }
                        ui.label(egui::RichText::new("Shift + Right-click a tile in viewport to remove it").size(9.0).color(egui::Color32::from_gray(120)));
                    }

                    // ── Lighting ──
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Lighting").strong());
                    ui.separator();
                    ui.checkbox(&mut s.lighting.enabled, "Enable lighting");
                    if s.lighting.enabled {
                        egui::Grid::new("lighting_grid").num_columns(2).show(ui, |ui| {
                            ui.label("Ambient R");
                            ui.add(egui::Slider::new(&mut s.lighting.ambient[0], 0.0..=1.0));
                            ui.end_row();
                            ui.label("Ambient G");
                            ui.add(egui::Slider::new(&mut s.lighting.ambient[1], 0.0..=1.0));
                            ui.end_row();
                            ui.label("Ambient B");
                            ui.add(egui::Slider::new(&mut s.lighting.ambient[2], 0.0..=1.0));
                            ui.end_row();
                            ui.label("Ambient Int");
                            ui.add(egui::Slider::new(&mut s.lighting.ambient[3], 0.0..=2.0));
                            ui.end_row();
                        });
                        ui.checkbox(&mut s.lighting.shadows_enabled, "Enable shadows");
                    }

                    // ── Post-Processing ──
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Post-Processing").strong());
                    ui.separator();
                    let mut remove_fx: Option<usize> = None;
                    for (i, fx) in s.post_effects.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(post_effect_label(fx));
                            if ui.small_button("x").clicked() { remove_fx = Some(i); }
                        });
                        post_effect_inspector(ui, fx, i);
                        ui.separator();
                    }
                    if let Some(idx) = remove_fx { s.post_effects.remove(idx); }
                    let mut add_fx = String::new();
                    egui::ComboBox::from_id_salt("add_fx")
                        .selected_text("+ Add Effect")
                        .show_ui(ui, |ui| {
                            for name in &["Vignette", "Bloom", "CRT", "Pixelate", "ColorGrading"] {
                                if ui.selectable_label(false, *name).clicked() {
                                    add_fx = name.to_string();
                                }
                            }
                        });
                    if !add_fx.is_empty() {
                        s.post_effects.push(default_post_effect(&add_fx));
                    }
                });
            if !open { self.show_scene_settings = false; }
        }

        // Tilemap tools panel (when in tilemap mode)
        if self.editor_mode == EditorMode::Tilemap {
            egui::TopBottomPanel::bottom("tilemap_tools").exact_height(80.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Tilemap").strong());
                    ui.separator();

                    // Tool buttons
                    let brush = self.tilemap_editor.tool == TileTool::Brush;
                    let eraser = self.tilemap_editor.tool == TileTool::Eraser;
                    let fill = self.tilemap_editor.tool == TileTool::Fill;

                    if ui.selectable_label(brush, "Brush").clicked() {
                        self.tilemap_editor.tool = TileTool::Brush;
                    }
                    if ui.selectable_label(eraser, "Eraser").clicked() {
                        self.tilemap_editor.tool = TileTool::Eraser;
                    }
                    if ui.selectable_label(fill, "Fill").clicked() {
                        self.tilemap_editor.tool = TileTool::Fill;
                    }

                    ui.separator();
                    ui.label("Tile:");
                    ui.add(egui::DragValue::new(&mut self.tilemap_editor.selected_gid)
                        .range(1..=self.tilemap_editor.tileset_columns * self.tilemap_editor.tileset_rows));

                    ui.separator();
                    if let Some(tilemap) = &self.scene.tilemap {
                        ui.label(format!("Map: {}x{}", tilemap.width, tilemap.height));
                        ui.label(format!("Layers: {}", tilemap.layers.len()));
                    }
                });

                // Tile palette preview (colored squares for each GID)
                ui.horizontal(|ui| {
                    let total = self.tilemap_editor.tileset_columns * self.tilemap_editor.tileset_rows;
                    for gid in 1..=total {
                        let selected = self.tilemap_editor.selected_gid == gid;
                        let size = if selected { 28.0 } else { 24.0 };
                        let color = if selected {
                            egui::Color32::YELLOW
                        } else {
                            // Color-code by GID
                            let hue = (gid as f32 * 0.25) % 1.0;
                            let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.8);
                            egui::Color32::from_rgb(r, g, b)
                        };
                        let response = ui.add(egui::Button::new(format!("{gid}"))
                            .fill(color)
                            .min_size(egui::vec2(size, size)));
                        if response.clicked() {
                            self.tilemap_editor.selected_gid = gid;
                        }
                    }
                });
            });

            // Load tileset texture if needed
            if self.tilemap_editor.tileset_tex.is_none() {
                if let Some(tilemap) = &self.scene.tilemap {
                    let path = std::path::Path::new(&tilemap.tileset_path);
                    if path.exists() {
                        // We can't load here (no GameContext), mark for loading in init
                        self.status_msg = format!("Tileset: {}", tilemap.tileset_path);
                    }
                }
            }
        }

        // Status bar (skip when asset browser provides its own)
        if self.editor_mode == EditorMode::AssetBrowser { return; }
        egui::TopBottomPanel::bottom("status").exact_height(24.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "Toile v{} | {} | Entities: {} | Zoom: {:.1}x",
                        env!("CARGO_PKG_VERSION"),
                        self.current_file,
                        self.scene.entities.len(),
                        self.camera_zoom
                    ));
                });
            });
        });
    }
}
