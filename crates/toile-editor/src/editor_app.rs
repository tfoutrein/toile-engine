use std::path::Path;

use glam::Vec2;
use toile_app::{App, Game, GameContext, Key, Sprite, TextureHandle, COLOR_WHITE};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::pack_color;
use winit::event::WindowEvent;
use winit::window::Window;

use crate::overlay::EguiOverlay;
use crate::scene_data::{EntityData, SceneData};

pub struct EditorApp {
    overlay: Option<EguiOverlay>,
    surface_format: Option<wgpu::TextureFormat>,
    scene: SceneData,
    selected_id: Option<u64>,
    white_tex: Option<TextureHandle>,
    camera_pos: Vec2,
    camera_zoom: f32,
    dragging: Option<u64>,
    drag_offset: Vec2,
    show_grid: bool,
    status_msg: String,
}

impl EditorApp {
    pub fn new() -> Self {
        let mut scene = SceneData::new("Untitled");
        // Add some default entities for demo
        scene.add_entity("Player", 0.0, 50.0);
        scene.add_entity("Enemy", 100.0, 50.0);
        scene.add_entity("Platform", 0.0, 0.0);
        if let Some(e) = scene.find_entity_mut(3) {
            e.width = 200.0;
            e.height = 20.0;
        }

        Self {
            overlay: None,
            surface_format: None,
            scene,
            selected_id: None,
            white_tex: None,
            camera_pos: Vec2::ZERO,
            camera_zoom: 1.0,
            dragging: None,
            drag_offset: Vec2::ZERO,
            show_grid: true,
            status_msg: "Ready".to_string(),
        }
    }

    fn ui_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Scene").clicked() {
                    self.scene = SceneData::new("Untitled");
                    self.selected_id = None;
                    self.status_msg = "New scene created".to_string();
                    ui.close_menu();
                }
                if ui.button("Save (scene.json)").clicked() {
                    let json = serde_json::to_string_pretty(&self.scene).unwrap();
                    std::fs::write("scene.json", &json).unwrap();
                    self.status_msg = format!("Saved to scene.json ({} entities)", self.scene.entities.len());
                    ui.close_menu();
                }
                if ui.button("Load (scene.json)").clicked() {
                    if let Ok(json) = std::fs::read_to_string("scene.json") {
                        if let Ok(data) = serde_json::from_str(&json) {
                            self.scene = data;
                            self.selected_id = None;
                            self.status_msg = "Loaded scene.json".to_string();
                        }
                    }
                    ui.close_menu();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Add Entity").clicked() {
                    let id = self.scene.add_entity(
                        &format!("Entity_{}", self.scene.next_id),
                        self.camera_pos.x,
                        self.camera_pos.y,
                    );
                    self.selected_id = Some(id);
                    self.status_msg = format!("Created entity {id}");
                    ui.close_menu();
                }
                if ui.button("Delete Selected").clicked() {
                    if let Some(id) = self.selected_id {
                        self.scene.remove_entity(id);
                        self.selected_id = None;
                        self.status_msg = format!("Deleted entity {id}");
                    }
                    ui.close_menu();
                }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_grid, "Show Grid");
                if ui.button("Reset Camera").clicked() {
                    self.camera_pos = Vec2::ZERO;
                    self.camera_zoom = 1.0;
                    ui.close_menu();
                }
            });
        });
    }

    fn ui_hierarchy(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("hierarchy")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Hierarchy");
                ui.separator();

                let mut click_id = None;
                for entity in &self.scene.entities {
                    let selected = self.selected_id == Some(entity.id);
                    let label = egui::RichText::new(&entity.name)
                        .color(if selected {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::WHITE
                        });
                    if ui.selectable_label(selected, label).clicked() {
                        click_id = Some(entity.id);
                    }
                }
                if let Some(id) = click_id {
                    self.selected_id = Some(id);
                }

                ui.separator();
                if ui.button("+ Add Entity").clicked() {
                    let id = self.scene.add_entity(
                        &format!("Entity_{}", self.scene.next_id),
                        self.camera_pos.x,
                        self.camera_pos.y,
                    );
                    self.selected_id = Some(id);
                }
            });
    }

    fn ui_inspector(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                ui.separator();

                if let Some(id) = self.selected_id {
                    if let Some(entity) = self.scene.find_entity_mut(id) {
                        ui.label(format!("ID: {}", entity.id));
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut entity.name);
                        });
                        ui.separator();
                        ui.label("Transform");
                        ui.horizontal(|ui| {
                            ui.label("X:");
                            ui.add(egui::DragValue::new(&mut entity.x).speed(1.0));
                            ui.label("Y:");
                            ui.add(egui::DragValue::new(&mut entity.y).speed(1.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Rotation:");
                            ui.add(
                                egui::DragValue::new(&mut entity.rotation)
                                    .speed(0.1)
                                    .suffix("°"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Scale:");
                            ui.add(egui::DragValue::new(&mut entity.scale_x).speed(0.05));
                            ui.add(egui::DragValue::new(&mut entity.scale_y).speed(0.05));
                        });
                        ui.separator();
                        ui.label("Sprite");
                        ui.horizontal(|ui| {
                            ui.label("W:");
                            ui.add(egui::DragValue::new(&mut entity.width).speed(1.0));
                            ui.label("H:");
                            ui.add(egui::DragValue::new(&mut entity.height).speed(1.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Layer:");
                            ui.add(egui::DragValue::new(&mut entity.layer));
                        });
                        ui.separator();
                        if ui.button("Delete").clicked() {
                            self.scene.remove_entity(id);
                            self.selected_id = None;
                        }
                    } else {
                        self.selected_id = None;
                    }
                } else {
                    ui.label("No entity selected");
                }
            });
    }

    fn ui_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status_msg);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!(
                            "Entities: {} | Zoom: {:.1}x",
                            self.scene.entities.len(),
                            self.camera_zoom
                        ));
                    });
                });
            });
    }
}

impl Game for EditorApp {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        self.surface_format = Some(ctx.surface_format());
        log::info!("Toile Editor ready (surface format: {:?})", self.surface_format);
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f64) {
        // Camera zoom with scroll (when egui doesn't consume it)
        let scroll = ctx.input.scroll_delta();
        if scroll.y != 0.0 {
            self.camera_zoom *= 1.0 + scroll.y * 0.1;
            self.camera_zoom = self.camera_zoom.clamp(0.2, 5.0);
        }

        ctx.camera.position = self.camera_pos;
        ctx.camera.zoom = self.camera_zoom;
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        // Draw grid — use actual viewport size from camera
        if self.show_grid {
            let grid_size = 32.0;
            let vp = ctx.camera.viewport_size();
            let half_view = Vec2::new(
                vp.x / (2.0 * self.camera_zoom),
                vp.y / (2.0 * self.camera_zoom),
            );
            let min_x = ((self.camera_pos.x - half_view.x) / grid_size).floor() as i32;
            let max_x = ((self.camera_pos.x + half_view.x) / grid_size).ceil() as i32;
            let min_y = ((self.camera_pos.y - half_view.y) / grid_size).floor() as i32;
            let max_y = ((self.camera_pos.y + half_view.y) / grid_size).ceil() as i32;

            let grid_color = pack_color(60, 60, 80, 80);
            for x in min_x..=max_x {
                let wx = x as f32 * grid_size;
                ctx.draw_sprite(Sprite {
                    texture: tex,
                    position: Vec2::new(wx, self.camera_pos.y),
                    size: Vec2::new(1.0 / self.camera_zoom, half_view.y * 2.0),
                    rotation: 0.0,
                    color: grid_color,
                    layer: -10,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
            for y in min_y..=max_y {
                let wy = y as f32 * grid_size;
                ctx.draw_sprite(Sprite {
                    texture: tex,
                    position: Vec2::new(self.camera_pos.x, wy),
                    size: Vec2::new(half_view.x * 2.0, 1.0 / self.camera_zoom),
                    rotation: 0.0,
                    color: grid_color,
                    layer: -10,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }

        // Draw entities
        for entity in &self.scene.entities {
            let selected = self.selected_id == Some(entity.id);
            let color = if selected {
                pack_color(255, 220, 80, 255)
            } else {
                pack_color(100, 150, 220, 255)
            };

            ctx.draw_sprite(Sprite {
                texture: tex,
                position: Vec2::new(entity.x, entity.y),
                size: Vec2::new(
                    entity.width * entity.scale_x,
                    entity.height * entity.scale_y,
                ),
                rotation: entity.rotation,
                color,
                layer: entity.layer,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });

            // Selection outline
            if selected {
                let outline = pack_color(255, 255, 100, 255);
                let w = entity.width * entity.scale_x + 4.0;
                let h = entity.height * entity.scale_y + 4.0;
                let thickness = 2.0 / self.camera_zoom;
                // Top
                ctx.draw_sprite(Sprite::new(tex, Vec2::new(entity.x, entity.y + h / 2.0), Vec2::new(w, thickness)));
                // Bottom
                ctx.draw_sprite(Sprite::new(tex, Vec2::new(entity.x, entity.y - h / 2.0), Vec2::new(w, thickness)));
                // Left
                ctx.draw_sprite(Sprite::new(tex, Vec2::new(entity.x - w / 2.0, entity.y), Vec2::new(thickness, h)));
                // Right
                ctx.draw_sprite(Sprite::new(tex, Vec2::new(entity.x + w / 2.0, entity.y), Vec2::new(thickness, h)));
                // Set color on last 4 sprites
                let len = ctx.stats.sprite_count; // approximate
                // (outline sprites use default white color from Sprite::new — that's fine for now)
            }
        }
    }

    fn render_overlay(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
        size: (u32, u32),
    ) {
        let surface_format = self.surface_format.unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);
        let overlay = self.overlay.get_or_insert_with(|| {
            let o = EguiOverlay::new(device, surface_format, window);
            let mut style = (*o.ctx().style()).clone();
            style.visuals = egui::Visuals::dark();
            o.ctx().set_style(style);
            o
        });

        overlay.begin_frame(window);

        let ctx = overlay.ctx().clone();

        // Menu bar
        let mut new_scene = false;
        let mut save_scene = false;
        let mut load_scene = false;
        let mut add_entity = false;
        let mut delete_selected = false;

        egui::TopBottomPanel::top("menu").show(&ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Scene").clicked() { new_scene = true; ui.close_menu(); }
                    if ui.button("Save (scene.json)").clicked() { save_scene = true; ui.close_menu(); }
                    if ui.button("Load (scene.json)").clicked() { load_scene = true; ui.close_menu(); }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Add Entity").clicked() { add_entity = true; ui.close_menu(); }
                    if ui.button("Delete Selected").clicked() { delete_selected = true; ui.close_menu(); }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_grid, "Show Grid");
                    if ui.button("Reset Camera").clicked() {
                        self.camera_pos = Vec2::ZERO;
                        self.camera_zoom = 1.0;
                        ui.close_menu();
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
        if save_scene {
            let json = serde_json::to_string_pretty(&self.scene).unwrap();
            let _ = std::fs::write("scene.json", &json);
            self.status_msg = format!("Saved ({} entities)", self.scene.entities.len());
        }
        if load_scene {
            if let Ok(json) = std::fs::read_to_string("scene.json") {
                if let Ok(data) = serde_json::from_str(&json) {
                    self.scene = data;
                    self.selected_id = None;
                    self.status_msg = "Loaded scene.json".to_string();
                }
            }
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

        // Hierarchy panel
        egui::SidePanel::left("hierarchy").default_width(200.0).show(&ctx, |ui| {
            ui.heading("Hierarchy");
            ui.separator();
            let mut click_id = None;
            for entity in &self.scene.entities {
                let selected = self.selected_id == Some(entity.id);
                let label = egui::RichText::new(&entity.name)
                    .color(if selected { egui::Color32::YELLOW } else { egui::Color32::WHITE });
                if ui.selectable_label(selected, label).clicked() {
                    click_id = Some(entity.id);
                }
            }
            if let Some(id) = click_id {
                self.selected_id = Some(id);
            }
            ui.separator();
            if ui.button("+ Add Entity").clicked() {
                let id = self.scene.add_entity(
                    &format!("Entity_{}", self.scene.next_id),
                    self.camera_pos.x, self.camera_pos.y,
                );
                self.selected_id = Some(id);
            }
        });

        // Inspector panel
        egui::SidePanel::right("inspector").default_width(250.0).show(&ctx, |ui| {
            ui.heading("Inspector");
            ui.separator();
            if let Some(id) = self.selected_id {
                if let Some(entity) = self.scene.find_entity_mut(id) {
                    ui.label(format!("ID: {}", entity.id));
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut entity.name);
                    });
                    ui.separator();
                    ui.label("Transform");
                    ui.horizontal(|ui| {
                        ui.label("X:"); ui.add(egui::DragValue::new(&mut entity.x).speed(1.0));
                        ui.label("Y:"); ui.add(egui::DragValue::new(&mut entity.y).speed(1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Rot:");
                        ui.add(egui::DragValue::new(&mut entity.rotation).speed(0.1).suffix("°"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Scale:");
                        ui.add(egui::DragValue::new(&mut entity.scale_x).speed(0.05));
                        ui.add(egui::DragValue::new(&mut entity.scale_y).speed(0.05));
                    });
                    ui.separator();
                    ui.label("Size");
                    ui.horizontal(|ui| {
                        ui.label("W:"); ui.add(egui::DragValue::new(&mut entity.width).speed(1.0));
                        ui.label("H:"); ui.add(egui::DragValue::new(&mut entity.height).speed(1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Layer:"); ui.add(egui::DragValue::new(&mut entity.layer));
                    });
                } else {
                    self.selected_id = None;
                    ui.label("No entity selected");
                }
            } else {
                ui.label("No entity selected");
            }
        });

        // Status bar
        egui::TopBottomPanel::bottom("status").exact_height(24.0).show(&ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Entities: {} | Zoom: {:.1}x", self.scene.entities.len(), self.camera_zoom));
                });
            });
        });

        overlay.end_frame_and_render(device, queue, encoder, view, window, size);
    }

    fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        if let Some(overlay) = &mut self.overlay {
            overlay.handle_event(window, event)
        } else {
            false
        }
    }
}

/// Launch the editor.
pub fn run_editor() {
    App::new()
        .with_title("Toile Editor")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.12, 0.12, 0.16))
        .run(EditorApp::new());
}
