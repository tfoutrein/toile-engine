use std::path::Path;

use glam::Vec2;
use toile_app::{App, Game, GameContext, Key, Sprite, TextureHandle, COLOR_WHITE};
use toile_graphics::sprite_renderer::DrawSprite;
use toile_core::color::Color;
use toile_graphics::sprite_renderer::pack_color;
use winit::event::WindowEvent;
use winit::window::Window;

use crate::overlay::EguiOverlay;
use crate::particle_editor::ParticleEditorPanel;
use crate::scene_data::{EntityData, SceneData};
use crate::tilemap_tool::{self, TilemapEditor, TileTool};

pub struct EditorApp {
    overlay: Option<EguiOverlay>,
    surface_format: Option<wgpu::TextureFormat>,
    scene: SceneData,
    selected_id: Option<u64>,
    white_tex: Option<TextureHandle>,
    logo_tex: Option<TextureHandle>,
    camera_pos: Vec2,
    camera_zoom: f32,
    dragging: Option<u64>,
    drag_offset: Vec2,
    resizing: Option<ResizeHandle>,
    resize_start_size: Vec2,
    resize_start_pos: Vec2,
    resize_start_mouse: Vec2,
    resize_start_rot: f32,
    rotating: bool,
    rotate_start_angle: f32,
    rotate_start_mouse_angle: f32,
    show_grid: bool,
    status_msg: String,
    current_file: String,
    file_path_input: String,
    show_load_dialog: bool,
    show_save_dialog: bool,
    // Splash screen
    splash_timer: f32,
    show_splash: bool,
    // Tilemap editor
    tilemap_editor: TilemapEditor,
    // Particle editor
    particle_editor: ParticleEditorPanel,
    editor_mode: EditorMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorMode {
    Entity,
    Tilemap,
    Particle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ResizeHandle {
    // Corners (resize both axes)
    TopRight,
    BottomRight,
    BottomLeft,
    TopLeft,
    // Edges (resize one axis, move position to keep opposite edge fixed)
    Top,
    Bottom,
    Left,
    Right,
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
            resizing: None,
            resize_start_size: Vec2::ZERO,
            resize_start_pos: Vec2::ZERO,
            resize_start_mouse: Vec2::ZERO,
            resize_start_rot: 0.0,
            rotating: false,
            rotate_start_angle: 0.0,
            rotate_start_mouse_angle: 0.0,
            show_grid: true,
            status_msg: "Ready".to_string(),
            current_file: "scene.json".to_string(),
            file_path_input: String::new(),
            show_load_dialog: false,
            show_save_dialog: false,
            logo_tex: None,
            splash_timer: 2.5,
            show_splash: true,
            tilemap_editor: TilemapEditor::new(),
            particle_editor: ParticleEditorPanel::new(),
            editor_mode: EditorMode::Entity,
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
        self.logo_tex = Some(ctx.load_texture(Path::new("assets/toile-logo-transparent.png")));
        self.surface_format = Some(ctx.surface_format());

        // Pre-load the platformer tileset for tilemap mode
        let tileset_path = Path::new("assets/platformer/tileset.png");
        if tileset_path.exists() {
            self.tilemap_editor.tileset_tex = Some(ctx.load_texture(tileset_path));
        }

        log::info!("Toile Editor ready");
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f64) {
        // Splash screen countdown
        if self.show_splash {
            self.splash_timer -= _dt as f32;
            if self.splash_timer <= 0.0 || ctx.input.is_key_just_pressed(Key::Space) || ctx.input.is_key_just_pressed(Key::Escape) {
                self.show_splash = false;
            }
            return;
        }

        // Camera zoom with scroll (when egui doesn't consume it)
        let scroll = ctx.input.scroll_delta();
        if scroll.y != 0.0 {
            self.camera_zoom *= 1.0 + scroll.y * 0.1;
            self.camera_zoom = self.camera_zoom.clamp(0.2, 5.0);
        }

        ctx.camera.position = self.camera_pos;
        ctx.camera.zoom = self.camera_zoom;

        // Entity selection, drag, and resize in Entity mode
        if self.editor_mode == EditorMode::Entity {
            let world_pos = ctx.camera.screen_to_world(ctx.input.mouse_position());
            let handle_size = 8.0 / self.camera_zoom; // handle size in world units

            // Start interaction: detect transition from mouse-up to mouse-down
            if ctx.input.is_mouse_down(toile_app::MouseButton::Left)
                && self.dragging.is_none()
                && self.resizing.is_none()
                && !self.rotating
            {
                // First check: are we clicking on a resize handle of the selected entity?
                let mut hit_handle = None;
                if let Some(sel_id) = self.selected_id {
                    if let Some(entity) = self.scene.entities.iter().find(|e| e.id == sel_id) {
                        let hw = entity.width * entity.scale_x * 0.5;
                        let hh = entity.height * entity.scale_y * 0.5;
                        let rot = entity.rotation;
                        let center = Vec2::new(entity.x, entity.y);

                        // Rotate local offset around entity center
                        let rotated = |local: Vec2| -> Vec2 {
                            let (sin, cos) = rot.sin_cos();
                            center + Vec2::new(
                                local.x * cos - local.y * sin,
                                local.x * sin + local.y * cos,
                            )
                        };

                        let handles = [
                            (rotated(Vec2::new(hw, hh)), ResizeHandle::TopRight),
                            (rotated(Vec2::new(hw, -hh)), ResizeHandle::BottomRight),
                            (rotated(Vec2::new(-hw, -hh)), ResizeHandle::BottomLeft),
                            (rotated(Vec2::new(-hw, hh)), ResizeHandle::TopLeft),
                            (rotated(Vec2::new(0.0, hh)), ResizeHandle::Top),
                            (rotated(Vec2::new(0.0, -hh)), ResizeHandle::Bottom),
                            (rotated(Vec2::new(-hw, 0.0)), ResizeHandle::Left),
                            (rotated(Vec2::new(hw, 0.0)), ResizeHandle::Right),
                        ];
                        for (pos, handle) in &handles {
                            if (world_pos - *pos).length() < handle_size * 1.5 {
                                hit_handle = Some(*handle);
                                break;
                            }
                        }
                    }
                }

                // Check rotation handle (diamond above top edge, rotated)
                let mut hit_rotate = false;
                if hit_handle.is_none() {
                    if let Some(sel_id) = self.selected_id {
                        if let Some(entity) = self.scene.entities.iter().find(|e| e.id == sel_id) {
                            let hh = entity.height * entity.scale_y * 0.5;
                            let rot = entity.rotation;
                            let center = Vec2::new(entity.x, entity.y);
                            let local = Vec2::new(0.0, hh + handle_size * 4.0);
                            let (sin, cos) = rot.sin_cos();
                            let rotate_handle_pos = center + Vec2::new(
                                local.x * cos - local.y * sin,
                                local.x * sin + local.y * cos,
                            );
                            if (world_pos - rotate_handle_pos).length() < handle_size * 2.0 {
                                hit_rotate = true;
                            }
                        }
                    }
                }

                if hit_rotate {
                    // Start rotation
                    self.rotating = true;
                    if let Some(sel_id) = self.selected_id {
                        if let Some(entity) = self.scene.entities.iter().find(|e| e.id == sel_id) {
                            self.rotate_start_angle = entity.rotation;
                            let to_mouse = world_pos - Vec2::new(entity.x, entity.y);
                            self.rotate_start_mouse_angle = to_mouse.y.atan2(to_mouse.x);
                        }
                    }
                } else if let Some(handle) = hit_handle {
                    // Start resize
                    self.resizing = Some(handle);
                    self.resize_start_mouse = world_pos;
                    if let Some(sel_id) = self.selected_id {
                        if let Some(entity) = self.scene.entities.iter().find(|e| e.id == sel_id) {
                            self.resize_start_size = Vec2::new(entity.width, entity.height);
                            self.resize_start_pos = Vec2::new(entity.x, entity.y);
                            self.resize_start_rot = entity.rotation;
                        }
                    }
                } else {
                    // Try to pick an entity for drag (rotation-aware hit test)
                    let mut clicked_id = None;
                    for entity in self.scene.entities.iter().rev() {
                        let hw = entity.width * entity.scale_x * 0.5;
                        let hh = entity.height * entity.scale_y * 0.5;
                        // Transform mouse into entity's local space (undo rotation)
                        let d = world_pos - Vec2::new(entity.x, entity.y);
                        let (sin, cos) = (-entity.rotation).sin_cos();
                        let local = Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
                        if local.x >= -hw && local.x <= hw && local.y >= -hh && local.y <= hh {
                            clicked_id = Some(entity.id);
                            break;
                        }
                    }

                    if let Some(id) = clicked_id {
                        self.selected_id = Some(id);
                        if let Some(entity) = self.scene.entities.iter().find(|e| e.id == id) {
                            self.drag_offset =
                                Vec2::new(entity.x - world_pos.x, entity.y - world_pos.y);
                        }
                        self.dragging = Some(id);
                        self.status_msg = format!("Selected entity {id}");
                    } else {
                        self.selected_id = None;
                        self.dragging = Some(u64::MAX); // sentinel
                    }
                }
            }

            // Continue drag
            if ctx.input.is_mouse_down(toile_app::MouseButton::Left) {
                if let Some(drag_id) = self.dragging {
                    if drag_id != u64::MAX {
                        if let Some(entity) = self.scene.find_entity_mut(drag_id) {
                            entity.x = world_pos.x + self.drag_offset.x;
                            entity.y = world_pos.y + self.drag_offset.y;
                        }
                    }
                }

                // Continue resize
                // Transform mouse delta into entity's local space (undo rotation)
                // Default: asymmetric (only the dragged face moves)
                // Shift: symmetric (both faces move, center stays)
                if let Some(handle) = self.resizing {
                    if let Some(sel_id) = self.selected_id {
                        let world_delta = world_pos - self.resize_start_mouse;
                        // Project delta into entity's local axes
                        let rot = self.resize_start_rot;
                        let (sin, cos) = (-rot).sin_cos();
                        let ld = Vec2::new(
                            world_delta.x * cos - world_delta.y * sin,
                            world_delta.x * sin + world_delta.y * cos,
                        );

                        let symmetric = ctx.input.is_key_down(Key::ShiftLeft)
                            || ctx.input.is_key_down(Key::ShiftRight);

                        if let Some(entity) = self.scene.find_entity_mut(sel_id) {
                            let sw = self.resize_start_size.x;
                            let sh = self.resize_start_size.y;
                            let sp = self.resize_start_pos;

                            // Compute size deltas in local space based on handle
                            let (dw, dh) = match handle {
                                ResizeHandle::Right => (ld.x, 0.0),
                                ResizeHandle::Left => (-ld.x, 0.0),
                                ResizeHandle::Top => (0.0, ld.y),
                                ResizeHandle::Bottom => (0.0, -ld.y),
                                ResizeHandle::TopRight => (ld.x, ld.y),
                                ResizeHandle::BottomRight => (ld.x, -ld.y),
                                ResizeHandle::BottomLeft => (-ld.x, -ld.y),
                                ResizeHandle::TopLeft => (-ld.x, ld.y),
                            };

                            if symmetric {
                                entity.width = (sw + dw * 2.0).max(4.0);
                                entity.height = (sh + dh * 2.0).max(4.0);
                                entity.x = sp.x;
                                entity.y = sp.y;
                            } else {
                                entity.width = (sw + dw).max(4.0);
                                entity.height = (sh + dh).max(4.0);

                                // Shift position in world space so the opposite edge stays fixed
                                // Local offset = half the size change along each axis
                                let local_shift = Vec2::new(
                                    match handle {
                                        ResizeHandle::Right | ResizeHandle::TopRight | ResizeHandle::BottomRight => dw * 0.5,
                                        ResizeHandle::Left | ResizeHandle::TopLeft | ResizeHandle::BottomLeft => -dw * 0.5,
                                        _ => 0.0,
                                    },
                                    match handle {
                                        ResizeHandle::Top | ResizeHandle::TopRight | ResizeHandle::TopLeft => dh * 0.5,
                                        ResizeHandle::Bottom | ResizeHandle::BottomRight | ResizeHandle::BottomLeft => -dh * 0.5,
                                        _ => 0.0,
                                    },
                                );
                                // Rotate the local shift back to world space
                                let (sin, cos) = rot.sin_cos();
                                let world_shift = Vec2::new(
                                    local_shift.x * cos - local_shift.y * sin,
                                    local_shift.x * sin + local_shift.y * cos,
                                );
                                entity.x = sp.x + world_shift.x;
                                entity.y = sp.y + world_shift.y;
                            }
                        }
                    }
                }

                // Continue rotation
                if self.rotating {
                    if let Some(sel_id) = self.selected_id {
                        if let Some(entity) = self.scene.find_entity_mut(sel_id) {
                            let to_mouse = world_pos - Vec2::new(entity.x, entity.y);
                            let current_angle = to_mouse.y.atan2(to_mouse.x);
                            let delta_angle = current_angle - self.rotate_start_mouse_angle;
                            entity.rotation = self.rotate_start_angle + delta_angle;

                            // Snap to 15° increments when Shift is held
                            if ctx.input.is_key_down(Key::ShiftLeft)
                                || ctx.input.is_key_down(Key::ShiftRight)
                            {
                                let snap = std::f32::consts::PI / 12.0; // 15°
                                entity.rotation = (entity.rotation / snap).round() * snap;
                            }
                        }
                    }
                }
            }

            // End drag/resize/rotate on mouse release
            if !ctx.input.is_mouse_down(toile_app::MouseButton::Left) {
                self.dragging = None;
                self.resizing = None;
                self.rotating = false;
            }
        }

        // Tilemap painting with mouse
        if self.editor_mode == EditorMode::Tilemap {
            if ctx.input.is_mouse_down(toile_app::MouseButton::Left) {
                let world_pos = ctx.camera.screen_to_world(ctx.input.mouse_position());
                if let Some(tilemap) = &mut self.scene.tilemap {
                    let w = tilemap.width;
                    let h = tilemap.height;
                    if let Some((col, row)) = self.tilemap_editor.world_to_tile(world_pos, w, h) {
                        match self.tilemap_editor.tool {
                            TileTool::Brush => self.tilemap_editor.paint(tilemap, col, row),
                            TileTool::Eraser => self.tilemap_editor.erase(tilemap, col, row),
                            TileTool::Fill => {} // fill on click, not drag
                        }
                    }
                }
            }
            // Fill on single click
            if ctx.input.is_mouse_just_pressed(toile_app::MouseButton::Left) {
                if self.tilemap_editor.tool == TileTool::Fill {
                    let world_pos = ctx.camera.screen_to_world(ctx.input.mouse_position());
                    if let Some(tilemap) = &mut self.scene.tilemap {
                        let w = tilemap.width;
                        let h = tilemap.height;
                        if let Some((col, row)) = self.tilemap_editor.world_to_tile(world_pos, w, h) {
                            self.tilemap_editor.flood_fill(tilemap, col, row);
                        }
                    }
                }
            }
        }

        // Particle simulation tick
        if self.editor_mode == EditorMode::Particle {
            self.particle_editor.update(_dt as f32);
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        // Splash screen: centered logo
        if self.show_splash {
            if let Some(logo) = self.logo_tex {
                let fade = ((2.5 - self.splash_timer) * 2.0).clamp(0.0, 1.0); // fade in
                let alpha = (fade * 255.0) as u8;
                let size = 256.0;
                ctx.draw_sprite(Sprite {
                    texture: logo,
                    position: Vec2::ZERO,
                    size: Vec2::new(size, size),
                    rotation: 0.0,
                    color: pack_color(255, 255, 255, alpha),
                    layer: 100,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
            return;
        }

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

        // Draw tilemap layers and entities — skipped in Particle mode
        if self.editor_mode != EditorMode::Particle {

        if let Some(tilemap) = &self.scene.tilemap {
            if let Some(tileset_tex) = self.tilemap_editor.tileset_tex {
                let ts = tilemap.tile_size as f32;
                let map_w = tilemap.width as f32 * ts;
                let map_h = tilemap.height as f32 * ts;
                let offset_x = -map_w * 0.5;
                let offset_y = map_h * 0.5;

                for layer in &tilemap.layers {
                    if !layer.visible {
                        continue;
                    }
                    for row in 0..tilemap.height {
                        for col in 0..tilemap.width {
                            let gid = layer.tiles[(row * tilemap.width + col) as usize];
                            if gid == 0 {
                                continue;
                            }
                            let (uv_min, uv_max) = self.tilemap_editor.tile_uv(gid);
                            let x = offset_x + col as f32 * ts + ts * 0.5;
                            let y = offset_y - (row as f32 * ts + ts * 0.5);
                            ctx.draw_sprite(Sprite {
                                texture: tileset_tex,
                                position: Vec2::new(x, y),
                                size: Vec2::new(ts, ts),
                                rotation: 0.0,
                                color: COLOR_WHITE,
                                layer: -5,
                                uv_min,
                                uv_max,
                            });
                        }
                    }
                }
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

            // Selection outline + resize handles (rotated with entity)
            if selected {
                let hw = entity.width * entity.scale_x * 0.5;
                let hh = entity.height * entity.scale_y * 0.5;
                let ow = hw + 2.0;
                let oh = hh + 2.0;
                let thickness = 2.0 / self.camera_zoom;
                let handle_size = 8.0 / self.camera_zoom;
                let outline_color = pack_color(255, 255, 100, 200);
                let handle_color = pack_color(255, 255, 255, 255);
                let rot = entity.rotation;
                let center = Vec2::new(entity.x, entity.y);

                // Helper: rotate a local offset around entity center
                let rotated = |local: Vec2| -> Vec2 {
                    let (sin, cos) = rot.sin_cos();
                    center + Vec2::new(
                        local.x * cos - local.y * sin,
                        local.x * sin + local.y * cos,
                    )
                };

                // Outline edges (4 lines, each rotated)
                let edges = [
                    (Vec2::new(0.0, oh), Vec2::new(ow * 2.0, thickness)),   // top
                    (Vec2::new(0.0, -oh), Vec2::new(ow * 2.0, thickness)),  // bottom
                    (Vec2::new(-ow, 0.0), Vec2::new(thickness, oh * 2.0)),  // left
                    (Vec2::new(ow, 0.0), Vec2::new(thickness, oh * 2.0)),   // right
                ];
                for (local_pos, size) in edges {
                    ctx.draw_sprite(DrawSprite {
                        texture: tex,
                        position: rotated(local_pos),
                        size,
                        rotation: rot,
                        color: outline_color,
                        layer: 90,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                // Corner handles (4 squares)
                let corners_local = [
                    Vec2::new(hw, hh),
                    Vec2::new(hw, -hh),
                    Vec2::new(-hw, -hh),
                    Vec2::new(-hw, hh),
                ];
                for local in corners_local {
                    ctx.draw_sprite(DrawSprite {
                        texture: tex,
                        position: rotated(local),
                        size: Vec2::splat(handle_size),
                        rotation: rot,
                        color: handle_color,
                        layer: 91,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                // Edge midpoint handles
                let edge_color = pack_color(200, 220, 255, 255);
                let edge_handles = [
                    (Vec2::new(0.0, hh), Vec2::new(handle_size * 2.0, handle_size * 0.6)),   // top
                    (Vec2::new(0.0, -hh), Vec2::new(handle_size * 2.0, handle_size * 0.6)),  // bottom
                    (Vec2::new(-hw, 0.0), Vec2::new(handle_size * 0.6, handle_size * 2.0)),  // left
                    (Vec2::new(hw, 0.0), Vec2::new(handle_size * 0.6, handle_size * 2.0)),   // right
                ];
                for (local, size) in edge_handles {
                    ctx.draw_sprite(DrawSprite {
                        texture: tex,
                        position: rotated(local),
                        size,
                        rotation: rot,
                        color: edge_color,
                        layer: 91,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                // Rotation handle: line + diamond above top edge
                let rot_arm = hh + handle_size * 4.0;
                let rot_color = pack_color(120, 220, 255, 255);

                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: rotated(Vec2::new(0.0, hh + handle_size * 2.0)),
                    size: Vec2::new(thickness, handle_size * 4.0),
                    rotation: rot,
                    color: rot_color,
                    layer: 91,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });

                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: rotated(Vec2::new(0.0, rot_arm)),
                    size: Vec2::splat(handle_size * 1.5),
                    rotation: rot + std::f32::consts::FRAC_PI_4,
                    color: rot_color,
                    layer: 92,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }
        } // end `if self.editor_mode != EditorMode::Particle`

        // Render particles in Particle mode
        if self.editor_mode == EditorMode::Particle {
            for (pos, size, rot, color) in self.particle_editor.render_data() {
                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: pos,
                    size: Vec2::splat(size),
                    rotation: rot,
                    color,
                    layer: 0,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
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
        // Skip egui during splash screen
        if self.show_splash {
            return;
        }

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
                    if ui.button("Save...").clicked() {
                        self.file_path_input = self.current_file.clone();
                        self.show_save_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("Load...").clicked() {
                        self.file_path_input = self.current_file.clone();
                        self.show_load_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if !self.current_file.is_empty() {
                        if ui.button(format!("Quick Save ({})", self.current_file)).clicked() {
                            save_scene = true;
                            ui.close_menu();
                        }
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
        if save_scene && !self.current_file.is_empty() {
            let json = serde_json::to_string_pretty(&self.scene).unwrap();
            match std::fs::write(&self.current_file, &json) {
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
                .show(&ctx, |ui| {
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
                .show(&ctx, |ui| {
                    ui.label("File path:");
                    ui.text_edit_singleline(&mut self.file_path_input);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let json = serde_json::to_string_pretty(&self.scene).unwrap();
                            match std::fs::write(&self.file_path_input, &json) {
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

        // Hierarchy panel — hidden in Particle mode
        if self.editor_mode != EditorMode::Particle {
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
        } // end hierarchy panel

        // Inspector panel — replaced by particle panel in Particle mode
        if self.editor_mode == EditorMode::Particle {
            egui::SidePanel::right("inspector").min_width(320.0).max_width(320.0).show(&ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.particle_editor.show(ui);
                });
            });
        }

        if self.editor_mode != EditorMode::Particle {
        egui::SidePanel::right("inspector").default_width(260.0).show(&ctx, |ui| {
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

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Sprite").strong());
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
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                        });
                } else {
                    self.selected_id = None;
                    ui.label("No entity selected");
                }
            } else {
                ui.label("No entity selected");
            }
        });
        } // end `if self.editor_mode != EditorMode::Particle`

        // Status bar
        // Tilemap tools panel (when in tilemap mode)
        if self.editor_mode == EditorMode::Tilemap {
            egui::TopBottomPanel::bottom("tilemap_tools").exact_height(80.0).show(&ctx, |ui| {
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

        egui::TopBottomPanel::bottom("status").exact_height(24.0).show(&ctx, |ui| {
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

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
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

/// Launch the editor.
pub fn run_editor() {
    App::new()
        .with_title("Toile Editor")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.12, 0.12, 0.16))
        .run(EditorApp::new());
}
