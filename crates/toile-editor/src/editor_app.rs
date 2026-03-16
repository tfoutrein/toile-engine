use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::Vec2;
use toile_app::{App, Game, GameContext, Key, Sprite, TextureHandle, COLOR_WHITE};
use toile_graphics::sprite_renderer::DrawSprite;
use toile_core::color::Color;
use toile_core::particles::{ParticleEmitter, ParticlePool};
use toile_graphics::sprite_renderer::pack_color;
use winit::event::WindowEvent;
use winit::window::Window;

use toile_behaviors::BehaviorConfig;

use crate::overlay::EguiOverlay;
use crate::particle_editor::ParticleEditorPanel;
use crate::scene_data::{EntityData, SceneData};
use crate::tilemap_tool::{self, TilemapEditor, TileTool};

use crate::helpers::*;

pub struct EditorApp {
    pub(crate) overlay: Option<EguiOverlay>,
    pub(crate) surface_format: Option<wgpu::TextureFormat>,
    // Project state
    pub(crate) project_dir: Option<PathBuf>,
    pub(crate) show_project_dialog: bool,
    pub(crate) project_path_input: String,
    pub(crate) new_project_name: String,
    pub(crate) new_project_template: String,
    pub(crate) show_file_picker: Option<FilePickerTarget>,
    // Scene state
    pub(crate) scene: SceneData,
    pub(crate) selected_id: Option<u64>,
    pub(crate) hovered_id: Option<u64>,
    pub(crate) white_tex: Option<TextureHandle>,
    pub(crate) logo_tex: Option<TextureHandle>,
    pub(crate) camera_pos: Vec2,
    pub(crate) camera_zoom: f32,
    pub(crate) dragging: Option<u64>,
    pub(crate) drag_offset: Vec2,
    pub(crate) resizing: Option<ResizeHandle>,
    pub(crate) resize_start_size: Vec2,
    pub(crate) resize_start_pos: Vec2,
    pub(crate) resize_start_mouse: Vec2,
    pub(crate) resize_start_rot: f32,
    pub(crate) rotating: bool,
    pub(crate) rotate_start_angle: f32,
    pub(crate) rotate_start_mouse_angle: f32,
    pub(crate) show_grid: bool,
    pub(crate) status_msg: String,
    pub(crate) current_file: String,
    pub(crate) file_path_input: String,
    pub(crate) show_load_dialog: bool,
    pub(crate) show_save_dialog: bool,
    // Splash screen
    pub(crate) splash_timer: f32,
    pub(crate) show_splash: bool,
    // Tilemap editor
    pub(crate) tilemap_editor: TilemapEditor,
    // Background
    pub(crate) background_tex: Option<TextureHandle>,
    pub(crate) background_path_loaded: String,
    // Entity sprite texture cache: sprite_path → TextureHandle
    pub(crate) sprite_cache: HashMap<String, TextureHandle>,
    // Particle editor
    pub(crate) particle_editor: ParticleEditorPanel,
    // Live particle preview pools (entity_id → pool)
    pub(crate) preview_particles: HashMap<u64, ParticlePool>,
    // Track which emitter path each pool was built from
    pub(crate) preview_particle_paths: HashMap<u64, String>,
    pub(crate) show_scene_settings: bool,
    pub(crate) show_frame_picker: bool,
    pub(crate) frame_picker_anim: String,
    pub(crate) frame_picker_egui_tex: Option<egui::TextureHandle>,
    pub(crate) frame_picker_loaded_path: String,
    pub(crate) show_sprite_editor: bool,
    pub(crate) sprite_editor_preview_anim: Option<(String, f32)>, // (anim_name, elapsed_frame)
    pub(crate) clipboard_entity: Option<EntityData>,
    pub(crate) show_viewport_guide: bool,
    pub(crate) last_mouse_pos: Vec2,
    pub(crate) panning: bool,
    pub(crate) editor_mode: EditorMode,
}

/// What field the file picker is targeting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FilePickerTarget {
    SpritePath,
    EventSheet,
    ParticleEmitter,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorMode {
    Entity,
    Tilemap,
    Particle,
    SpriteAnim,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ResizeHandle {
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
        let scene = SceneData::new("Untitled");

        Self {
            overlay: None,
            surface_format: None,
            project_dir: None,
            show_project_dialog: true, // show welcome on startup
            project_path_input: String::new(),
            new_project_name: "my-game".to_string(),
            new_project_template: "empty".to_string(),
            show_file_picker: None,
            scene,
            selected_id: None,
            hovered_id: None,
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
            status_msg: "Welcome — open or create a project to begin".to_string(),
            current_file: String::new(),
            file_path_input: String::new(),
            show_load_dialog: false,
            show_save_dialog: false,
            logo_tex: None,
            splash_timer: 2.5,
            show_splash: true,
            tilemap_editor: TilemapEditor::new(),
            background_tex: None,
            background_path_loaded: String::new(),
            sprite_cache: HashMap::new(),
            particle_editor: ParticleEditorPanel::new(),
            preview_particles: HashMap::new(),
            preview_particle_paths: HashMap::new(),
            show_scene_settings: false,
            show_frame_picker: false,
            frame_picker_anim: String::new(),
            frame_picker_egui_tex: None,
            frame_picker_loaded_path: String::new(),
            show_sprite_editor: false,
            sprite_editor_preview_anim: None,
            clipboard_entity: None,
            show_viewport_guide: true,
            last_mouse_pos: Vec2::ZERO,
            panning: false,
            editor_mode: EditorMode::Entity,
        }
    }

    /// Resolve a path relative to the project directory.
    fn project_path(&self, relative: &str) -> PathBuf {
        match &self.project_dir {
            Some(dir) => dir.join(relative),
            None => PathBuf::from(relative),
        }
    }

    /// Create a minimal project structure.
    fn create_project(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir.join("scenes")).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir.join("scripts")).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir.join("prefabs")).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir.join("assets")).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir.join("particles")).map_err(|e| e.to_string())?;

        let name = dir.file_name().unwrap_or_default().to_string_lossy();
        let toml = format!(
            "[project]\nname = \"{name}\"\nversion = \"0.1.0\"\ntemplate = \"{}\"\n\n[game]\nentry_scene = \"scenes/main.json\"\nwindow_width = 1280\nwindow_height = 720\n",
            self.new_project_template
        );
        std::fs::write(dir.join("Toile.toml"), toml).map_err(|e| e.to_string())?;

        // Create a default main scene
        let scene = SceneData::new(&name);
        toile_scene::save_scene(&dir.join("scenes/main.json"), &scene)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Open an existing project and load the entry scene.
    fn open_project(&mut self, dir: PathBuf) {
        let name = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        self.project_dir = Some(dir.clone());
        self.show_project_dialog = false;

        // Try to load Toile.toml to find entry scene
        let entry = if let Ok(content) = std::fs::read_to_string(dir.join("Toile.toml")) {
            // Simple parse — look for entry_scene
            content.lines()
                .find(|l| l.starts_with("entry_scene"))
                .and_then(|l| l.split('=').nth(1))
                .map(|v| v.trim().trim_matches('"').to_string())
                .unwrap_or_else(|| "scenes/main.json".to_string())
        } else {
            "scenes/main.json".to_string()
        };

        let scene_path = dir.join(&entry);
        if scene_path.exists() {
            match toile_scene::load_scene(&scene_path) {
                Ok(scene) => {
                    self.camera_zoom = scene.settings.camera_zoom;
                    self.scene = scene;
                    self.current_file = entry;
                    self.status_msg = format!("Opened project '{name}'");
                }
                Err(e) => {
                    self.status_msg = format!("Error loading scene: {e}");
                    self.scene = SceneData::new(&name);
                    self.current_file = entry;
                }
            }
        } else {
            self.scene = SceneData::new(&name);
            self.current_file = entry;
            self.status_msg = format!("Opened project '{name}' (new scene)");
        }
        self.selected_id = None;
        self.camera_pos = Vec2::ZERO;
    }

    /// List scene files in the project's scenes/ directory.
    fn list_project_scenes(&self) -> Vec<String> {
        let dir = self.project_path("scenes");
        let mut scenes = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "json") {
                    if let Some(name) = p.file_name() {
                        scenes.push(format!("scenes/{}", name.to_string_lossy()));
                    }
                }
            }
        }
        scenes.sort();
        scenes
    }

    /// List files in a project subdirectory matching an extension.
    fn list_project_files(&self, subdir: &str, ext: &str) -> Vec<String> {
        let dir = self.project_path(subdir);
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == ext) {
                    if let Some(name) = p.file_name() {
                        files.push(format!("{subdir}/{}", name.to_string_lossy()));
                    }
                }
            }
        }
        files.sort();
        files
    }

    /// Recalculate PlatformerFollow bounds to cover all background tiles.
    fn auto_update_bounds_from_tiles(&mut self) {
        let s = &self.scene.settings;
        if s.background_tiles.is_empty() { return; }
        let tile_w = s.viewport_width as f32 / s.camera_zoom;
        let tile_h = s.viewport_height as f32 / s.camera_zoom;
        let half_w = tile_w * 0.5;
        let half_h = tile_h * 0.5;

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for pos in &s.background_tiles {
            min_x = min_x.min(pos[0] - half_w);
            max_x = max_x.max(pos[0] + half_w);
            min_y = min_y.min(pos[1] - half_h);
            max_y = max_y.max(pos[1] + half_h);
        }

        if let toile_scene::CameraMode::PlatformerFollow { bounds, .. } = &mut self.scene.settings.camera_mode {
            *bounds = [min_x, min_y, max_x, max_y];
        }
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

        // Camera zoom with scroll
        let scroll = ctx.input.scroll_delta();
        if scroll.y != 0.0 {
            self.camera_zoom *= 1.0 + scroll.y * 0.1;
            self.camera_zoom = self.camera_zoom.clamp(0.2, 5.0);
        }

        // Camera pan with middle mouse button drag
        let mouse_pos = ctx.input.mouse_position();
        if ctx.input.is_mouse_down(toile_app::MouseButton::Middle) {
            let delta = mouse_pos - self.last_mouse_pos;
            self.camera_pos.x -= delta.x / self.camera_zoom;
            self.camera_pos.y += delta.y / self.camera_zoom; // y-up
            self.panning = true;
        } else {
            self.panning = false;
        }
        self.last_mouse_pos = mouse_pos;

        ctx.camera.position = self.camera_pos;
        ctx.camera.zoom = self.camera_zoom;

        // In SpriteAnim mode, center camera on selected entity and zoom in
        if self.editor_mode == EditorMode::SpriteAnim {
            if let Some(id) = self.selected_id {
                if let Some(entity) = self.scene.entities.iter().find(|e| e.id == id) {
                    ctx.camera.position = Vec2::new(entity.x, entity.y);
                    // Zoom to show entity nicely (fit ~3x the entity size)
                    let ent_size = entity.width.max(entity.height) * entity.scale_x.max(entity.scale_y);
                    if let Some(ref sheet) = entity.sprite_sheet {
                        let frame_size = sheet.frame_width.max(sheet.frame_height) as f32;
                        let vp = ctx.camera.viewport_size();
                        ctx.camera.zoom = (vp.x.min(vp.y) / (frame_size * 4.0)).max(1.0);
                    } else if ent_size > 0.0 {
                        let vp = ctx.camera.viewport_size();
                        ctx.camera.zoom = (vp.x.min(vp.y) / (ent_size * 4.0)).max(1.0);
                    }
                }
            }
        }

        // Keyboard shortcuts (Cmd on Mac = SuperLeft, Ctrl on PC = ControlLeft)
        let modifier = ctx.input.is_key_down(Key::SuperLeft)
            || ctx.input.is_key_down(Key::SuperRight)
            || ctx.input.is_key_down(Key::ControlLeft)
            || ctx.input.is_key_down(Key::ControlRight);

        if modifier && self.editor_mode == EditorMode::Entity {
            // Cmd+C / Ctrl+C — Copy selected entity
            if ctx.input.is_key_just_pressed(Key::KeyC) {
                if let Some(id) = self.selected_id {
                    if let Some(entity) = self.scene.entities.iter().find(|e| e.id == id) {
                        self.clipboard_entity = Some(entity.clone());
                        self.status_msg = format!("Copied '{}'", entity.name);
                    }
                }
            }

            // Cmd+V / Ctrl+V — Paste entity (offset by 20px)
            if ctx.input.is_key_just_pressed(Key::KeyV) {
                if let Some(ref source) = self.clipboard_entity.clone() {
                    let id = self.scene.next_id;
                    self.scene.next_id += 1;
                    let mut new_entity = source.clone();
                    new_entity.id = id;
                    new_entity.name = format!("{}_copy", source.name);
                    new_entity.x += 20.0;
                    new_entity.y -= 20.0;
                    self.scene.entities.push(new_entity);
                    self.selected_id = Some(id);
                    self.status_msg = format!("Pasted '{}_copy'", source.name);
                }
            }

            // Cmd+D / Ctrl+D — Duplicate selected entity in place
            if ctx.input.is_key_just_pressed(Key::KeyD) {
                if let Some(sel_id) = self.selected_id {
                    if let Some(source) = self.scene.entities.iter().find(|e| e.id == sel_id).cloned() {
                        let id = self.scene.next_id;
                        self.scene.next_id += 1;
                        let mut new_entity = source.clone();
                        new_entity.id = id;
                        new_entity.name = format!("{}_dup", source.name);
                        new_entity.x += 20.0;
                        new_entity.y -= 20.0;
                        self.scene.entities.push(new_entity);
                        self.selected_id = Some(id);
                        self.status_msg = format!("Duplicated '{}'", source.name);
                    }
                }
            }

            // Cmd+S / Ctrl+S — Quick Save
            if ctx.input.is_key_just_pressed(Key::KeyS) {
                if !self.current_file.is_empty() {
                    if let Some(ref dir) = self.project_dir {
                        let path = dir.join(&self.current_file);
                        if let Ok(json) = serde_json::to_string_pretty(&self.scene) {
                            match std::fs::write(&path, &json) {
                                Ok(()) => self.status_msg = format!("Saved {}", self.current_file),
                                Err(e) => self.status_msg = format!("Save failed: {e}"),
                            }
                        }
                    }
                }
            }
        }

        // Delete key — delete selected entity
        if (ctx.input.is_key_just_pressed(Key::Delete) || ctx.input.is_key_just_pressed(Key::Backspace))
            && self.editor_mode == EditorMode::Entity
        {
            if let Some(id) = self.selected_id.take() {
                self.scene.remove_entity(id);
                self.status_msg = format!("Deleted entity {id}");
            }
        }

        // Hover detection — find entity under mouse cursor
        self.hovered_id = None;
        if self.editor_mode == EditorMode::Entity && !self.panning {
            let world_mouse = ctx.camera.screen_to_world(ctx.input.mouse_position());
            // Check entities in reverse order (top-most first)
            for entity in self.scene.entities.iter().rev() {
                let hw = entity.width * entity.scale_x * 0.5;
                let hh = entity.height * entity.scale_y * 0.5;
                let dx = (world_mouse.x - entity.x).abs();
                let dy = (world_mouse.y - entity.y).abs();
                if dx <= hw && dy <= hh {
                    self.hovered_id = Some(entity.id);
                    break;
                }
            }
        }

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

        // Update preview particles for entities with emitters
        if self.editor_mode == EditorMode::Entity {
            let dt_f = _dt as f32;
            // Collect entity ids and their emitter paths + positions
            let mut active: Vec<(u64, String, Vec2)> = Vec::new();
            for e in &self.scene.entities {
                if let Some(ref path) = e.particle_emitter {
                    active.push((e.id, path.clone(), Vec2::new(e.x, e.y)));
                }
            }
            // Remove pools for entities that no longer have emitters
            self.preview_particles.retain(|id, _| active.iter().any(|(eid, _, _)| eid == id));
            self.preview_particle_paths.retain(|id, _| active.iter().any(|(eid, _, _)| eid == id));

            for (eid, path, pos) in &active {
                // Check if pool exists and matches the path
                let needs_reload = match self.preview_particle_paths.get(eid) {
                    Some(existing) => existing != path,
                    None => true,
                };
                if needs_reload {
                    let full = self.project_path(path);
                    if let Ok(json) = std::fs::read_to_string(&full) {
                        if let Ok(emitter) = serde_json::from_str::<ParticleEmitter>(&json) {
                            self.preview_particles.insert(*eid, ParticlePool::new(emitter, *pos));
                            self.preview_particle_paths.insert(*eid, path.clone());
                        }
                    }
                }
                // Update position and tick
                if let Some(pool) = self.preview_particles.get_mut(eid) {
                    pool.position = *pos;
                    pool.update(dt_f);
                }
            }
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

        // ── Background tiles ─────────────────────────────────────────────
        if let Some(ref bg_path) = self.scene.settings.background_image {
            // Load texture if needed
            if self.background_path_loaded != *bg_path {
                let full = self.project_path(bg_path);
                if full.exists() {
                    self.background_tex = Some(ctx.load_texture(&full));
                } else {
                    self.background_tex = None;
                }
                self.background_path_loaded = bg_path.clone();
            }
            // Ensure at least one tile exists
            if self.scene.settings.background_tiles.is_empty() {
                let cp = self.scene.settings.camera_position;
                self.scene.settings.background_tiles.push(cp);
            }
            let s = &self.scene.settings;
            let tile_w = s.viewport_width as f32 / s.camera_zoom;
            let tile_h = s.viewport_height as f32 / s.camera_zoom;

            if let Some(bg_tex) = self.background_tex {
                // Draw all tiles
                for pos in &s.background_tiles {
                    ctx.draw_sprite(Sprite {
                        texture: bg_tex,
                        position: Vec2::new(pos[0], pos[1]),
                        size: Vec2::new(tile_w, tile_h),
                        rotation: 0.0,
                        color: COLOR_WHITE,
                        layer: -100,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                // Draw "+" buttons on edges of outer tiles
                let btn_size = 16.0 / self.camera_zoom;
                let btn_color = pack_color(80, 200, 80, 200);
                let tiles = s.background_tiles.clone();
                let mut new_tile: Option<[f32; 2]> = None;

                for pos in &tiles {
                    let cx = pos[0];
                    let cy = pos[1];
                    // Check if adjacent positions are already occupied
                    let has_right = tiles.iter().any(|t| (t[0] - (cx + tile_w)).abs() < 1.0 && (t[1] - cy).abs() < 1.0);
                    let has_left  = tiles.iter().any(|t| (t[0] - (cx - tile_w)).abs() < 1.0 && (t[1] - cy).abs() < 1.0);
                    let has_up    = tiles.iter().any(|t| (t[0] - cx).abs() < 1.0 && (t[1] - (cy + tile_h)).abs() < 1.0);
                    let has_down  = tiles.iter().any(|t| (t[0] - cx).abs() < 1.0 && (t[1] - (cy - tile_h)).abs() < 1.0);

                    // Draw "+" sprites on empty edges
                    let candidates = [
                        (!has_right, Vec2::new(cx + tile_w * 0.5, cy), [cx + tile_w, cy]),
                        (!has_left,  Vec2::new(cx - tile_w * 0.5, cy), [cx - tile_w, cy]),
                        (!has_up,    Vec2::new(cx, cy + tile_h * 0.5), [cx, cy + tile_h]),
                        (!has_down,  Vec2::new(cx, cy - tile_h * 0.5), [cx, cy - tile_h]),
                    ];

                    for (show, btn_pos, new_pos) in &candidates {
                        if !show { continue; }
                        // Draw the "+" marker
                        ctx.draw_sprite(Sprite {
                            texture: tex,
                            position: *btn_pos,
                            size: Vec2::splat(btn_size),
                            rotation: 0.0,
                            color: btn_color,
                            layer: 98,
                            uv_min: Vec2::ZERO,
                            uv_max: Vec2::ONE,
                        });
                        // Check click
                        let world_mouse = ctx.camera.screen_to_world(ctx.input.mouse_position());
                        let d = (world_mouse - *btn_pos).abs();
                        if d.x < btn_size && d.y < btn_size
                            && ctx.input.is_mouse_just_pressed(toile_app::MouseButton::Left)
                            && new_tile.is_none()
                        {
                            new_tile = Some(*new_pos);
                        }
                    }
                }

                // Shift + Right-click on a tile to remove it (keep at least one)
                let world_mouse = ctx.camera.screen_to_world(ctx.input.mouse_position());
                let mut remove_tile: Option<usize> = None;
                let shift_held = ctx.input.is_key_down(Key::ShiftLeft) || ctx.input.is_key_down(Key::ShiftRight);
                if ctx.input.is_mouse_just_pressed(toile_app::MouseButton::Right) && shift_held && tiles.len() > 1 {
                    for (i, pos) in tiles.iter().enumerate() {
                        let dx = (world_mouse.x - pos[0]).abs();
                        let dy = (world_mouse.y - pos[1]).abs();
                        if dx < tile_w * 0.5 && dy < tile_h * 0.5 {
                            remove_tile = Some(i);
                            break;
                        }
                    }
                }

                if let Some(idx) = remove_tile {
                    if self.scene.settings.background_tiles.len() > 1 {
                        self.scene.settings.background_tiles.remove(idx);
                        self.auto_update_bounds_from_tiles();
                        self.status_msg = format!("Removed background tile. {} remaining.", self.scene.settings.background_tiles.len());
                    } else {
                        self.status_msg = "Cannot remove last background tile. Use Clear in Scene Settings.".to_string();
                    }
                }
                if let Some(pos) = new_tile {
                    self.scene.settings.background_tiles.push(pos);
                    self.auto_update_bounds_from_tiles();
                }
            }
        } else {
            if !self.background_path_loaded.is_empty() {
                self.background_tex = None;
                self.background_path_loaded.clear();
            }
        }

        // ── Player viewport guide ─────────────────────────────────────────
        // Fixed rectangle representing the game camera view from scene settings.
        if self.show_viewport_guide {
            let s = &self.scene.settings;
            let vp_w = s.viewport_width as f32 / s.camera_zoom;
            let vp_h = s.viewport_height as f32 / s.camera_zoom;
            let vp_cx = s.camera_position[0];
            let vp_cy = s.camera_position[1];

            let thickness = 1.5 / self.camera_zoom;
            let guide_color = pack_color(255, 200, 50, 180);

            // Top
            ctx.draw_sprite(Sprite {
                texture: tex, position: Vec2::new(vp_cx, vp_cy + vp_h * 0.5),
                size: Vec2::new(vp_w + thickness, thickness), rotation: 0.0,
                color: guide_color, layer: 99, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
            // Bottom
            ctx.draw_sprite(Sprite {
                texture: tex, position: Vec2::new(vp_cx, vp_cy - vp_h * 0.5),
                size: Vec2::new(vp_w + thickness, thickness), rotation: 0.0,
                color: guide_color, layer: 99, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
            // Left
            ctx.draw_sprite(Sprite {
                texture: tex, position: Vec2::new(vp_cx - vp_w * 0.5, vp_cy),
                size: Vec2::new(thickness, vp_h), rotation: 0.0,
                color: guide_color, layer: 99, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
            // Right
            ctx.draw_sprite(Sprite {
                texture: tex, position: Vec2::new(vp_cx + vp_w * 0.5, vp_cy),
                size: Vec2::new(thickness, vp_h), rotation: 0.0,
                color: guide_color, layer: 99, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
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

        // Load sprite textures for entities
        let sprite_paths: Vec<(usize, String)> = self.scene.entities.iter().enumerate()
            .filter(|(_, e)| !e.sprite_path.is_empty() && !self.sprite_cache.contains_key(&e.sprite_path))
            .map(|(i, e)| (i, e.sprite_path.clone()))
            .collect();
        for (_i, path) in sprite_paths {
            let full = self.project_path(&path);
            if full.exists() {
                let handle = ctx.load_texture(&full);
                self.sprite_cache.insert(path, handle);
            }
        }

        // Draw entities
        for entity in &self.scene.entities {
            let selected = self.selected_id == Some(entity.id);
            let hovered = self.hovered_id == Some(entity.id) && !selected;
            let is_player_ent = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("player"));
            let is_solid = entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Solid));
            let is_coin = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("coin"));
            let is_enemy = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("enemy"));

            let has_sprite = !entity.sprite_path.is_empty() && self.sprite_cache.contains_key(&entity.sprite_path);
            let entity_tex = if has_sprite {
                self.sprite_cache[&entity.sprite_path]
            } else {
                tex
            };

            // Alpha: invisible entities shown as semi-transparent in editor
            let alpha: u8 = if !entity.visible { 60 } else { 255 };

            // Lighten colors when hovered (add ~40 to each channel)
            let brighten = |r: u8, g: u8, b: u8, a: u8| -> u32 {
                if hovered {
                    pack_color(r.saturating_add(50), g.saturating_add(50), b.saturating_add(50), a)
                } else {
                    pack_color(r, g, b, a)
                }
            };

            let color = if has_sprite {
                if selected { pack_color(255, 255, 200, alpha) }
                else { brighten(255, 255, 255, alpha) }
            } else if selected {
                pack_color(255, 220, 80, alpha)
            } else if is_player_ent {
                brighten(80, 220, 120, alpha)
            } else if is_solid {
                brighten(160, 160, 180, alpha)
            } else if is_coin {
                brighten(255, 220, 50, alpha.min(200))
            } else if is_enemy {
                brighten(220, 80, 80, alpha)
            } else {
                brighten(100, 150, 220, alpha)
            };

            // Compute UV from sprite sheet (show first frame or idle frame 0)
            let (uv_min, uv_max) = if let Some(ref sheet) = entity.sprite_sheet {
                let frame_idx = entity.default_animation.as_ref()
                    .and_then(|anim_name| entity.animations.iter().find(|a| a.name == *anim_name))
                    .and_then(|a| a.frames.first().copied())
                    .unwrap_or(0);
                let col = frame_idx % sheet.columns;
                let row = frame_idx / sheet.columns;
                let u_step = 1.0 / sheet.columns as f32;
                let v_step = 1.0 / sheet.rows as f32;
                (
                    Vec2::new(col as f32 * u_step, row as f32 * v_step),
                    Vec2::new((col + 1) as f32 * u_step, (row + 1) as f32 * v_step),
                )
            } else {
                (Vec2::ZERO, Vec2::ONE)
            };

            // Render size: use frame size if sprite sheet, else entity size
            let render_size = if has_sprite {
                if let Some(ref sheet) = entity.sprite_sheet {
                    Vec2::new(sheet.frame_width as f32 * entity.scale_x,
                              sheet.frame_height as f32 * entity.scale_y)
                } else {
                    Vec2::new(entity.width * entity.scale_x, entity.height * entity.scale_y)
                }
            } else {
                Vec2::new(entity.width * entity.scale_x, entity.height * entity.scale_y)
            };

            ctx.draw_sprite(Sprite {
                texture: entity_tex,
                position: Vec2::new(entity.x, entity.y),
                size: render_size,
                rotation: entity.rotation,
                color,
                layer: entity.layer,
                uv_min,
                uv_max,
            });

            // Hover outline (thin, white, semi-transparent)
            if hovered {
                let hw = entity.width * entity.scale_x * 0.5 + 1.0;
                let hh = entity.height * entity.scale_y * 0.5 + 1.0;
                let thickness = 1.0 / self.camera_zoom;
                let rot = entity.rotation;
                let center = Vec2::new(entity.x, entity.y);
                let hover_color = pack_color(255, 255, 255, 120);
                let rotated = |local: Vec2| -> Vec2 {
                    let (sin, cos) = rot.sin_cos();
                    center + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos)
                };
                // Top/Bottom/Left/Right edges
                for (pos, size) in [
                    (rotated(Vec2::new(0.0, hh)), Vec2::new(hw * 2.0, thickness)),
                    (rotated(Vec2::new(0.0, -hh)), Vec2::new(hw * 2.0, thickness)),
                    (rotated(Vec2::new(-hw, 0.0)), Vec2::new(thickness, hh * 2.0)),
                    (rotated(Vec2::new(hw, 0.0)), Vec2::new(thickness, hh * 2.0)),
                ] {
                    ctx.draw_sprite(Sprite {
                        texture: tex, position: pos, size, rotation: rot,
                        color: hover_color, layer: 89,
                        uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
                    });
                }
            }

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
        // Render preview particles on entities
        for pool in self.preview_particles.values() {
            for (pos, size, rot, color) in pool.render_data() {
                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: pos,
                    size: Vec2::splat(size),
                    rotation: rot,
                    color,
                    layer: 50,
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

        // Pre-collect data before borrowing overlay (avoids self borrow conflicts)
        let project_scenes = self.list_project_scenes();
        let project_scripts = self.list_project_files("scripts", "json");
        let project_particles = self.list_project_files("particles", "json");
        let pdir = self.project_dir.clone();

        let surface_format = self.surface_format.unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);
        // Initialize overlay and begin frame, then extract ctx so we can
        // call &mut self methods without holding a borrow on self.overlay.
        {
            let overlay = self.overlay.get_or_insert_with(|| {
                let o = EguiOverlay::new(device, surface_format, window);
                let mut style = (*o.ctx().style()).clone();
                style.visuals = egui::Visuals::dark();
                o.ctx().set_style(style);
                o
            });
            overlay.begin_frame(window);
        }
        let ctx = self.overlay.as_ref().unwrap().ctx().clone();

        // Set grab cursor while panning
        if self.panning {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        // ── Welcome / Project dialog ─────────────────────────────────────
        if self.project_dir.is_none() {
            let mut action_create: Option<PathBuf> = None;
            let mut action_open: Option<PathBuf> = None;

            egui::CentralPanel::default().show(&ctx, |ui| {
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

            self.overlay.as_mut().unwrap().end_frame_and_render(device, queue, encoder, view, window, size);

            // Apply deferred actions (after overlay borrow ends)
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
        let mut load_scene = false;
        let mut add_entity = false;
        let mut delete_selected = false;
        let mut play_game = false;

        egui::TopBottomPanel::top("menu").show(&ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Scene").clicked() { new_scene = true; ui.close_menu(); }
                    // Scene switcher
                    if !project_scenes.is_empty() {
                        ui.menu_button("Open Scene", |ui| {
                            for s in &project_scenes {
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
                    ui.label("Scene path (relative to project):");
                    ui.text_edit_singleline(&mut self.file_path_input);
                    // Quick pick from existing scenes
                    if !project_scenes.is_empty() {
                        ui.label(egui::RichText::new("Existing scenes:").size(11.0));
                        for s in &project_scenes {
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
            if let Some(ref dir) = pdir {
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
        if self.editor_mode != EditorMode::Particle && self.editor_mode != EditorMode::SpriteAnim {
        egui::SidePanel::left("hierarchy").default_width(200.0).show(&ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
            // Project root
            let project_name = pdir.as_ref()
                .and_then(|d| d.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Game".to_string());

            let root_id = ui.make_persistent_id("hierarchy_root");
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), root_id, true)
                .show_header(ui, |ui| {
                    ui.label(egui::RichText::new(format!("🎮 {project_name}")).strong());
                })
                .body(|ui| {
                    // ── Scenes ──
                    let scenes_id = ui.make_persistent_id("hierarchy_scenes");
                    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), scenes_id, true)
                        .show_header(ui, |ui| {
                            ui.label(egui::RichText::new("📁 Scenes").color(egui::Color32::from_rgb(180, 200, 255)));
                        })
                        .body(|ui| {
                            let mut switch_scene: Option<String> = None;
                            for scene_file in &project_scenes {
                                let is_current = self.current_file == *scene_file;
                                let scene_name = scene_file.strip_prefix("scenes/").unwrap_or(scene_file);
                                let scene_name = scene_name.strip_suffix(".json").unwrap_or(scene_name);

                                let scene_node_id = ui.make_persistent_id(scene_file);
                                egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), scene_node_id, is_current)
                                    .show_header(ui, |ui| {
                                        let icon = if is_current { "📄" } else { "📄" };
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
                                                                ui.label(egui::RichText::new(format!("    🎭 {}", behavior_label(beh))).size(11.0).color(dim));
                                                            }
                                                            if let Some(ref light) = entity.light {
                                                                ui.label(egui::RichText::new(format!("    💡 Light (r={:.0})", light.radius)).size(11.0).color(dim));
                                                            }
                                                            if let Some(ref pe) = entity.particle_emitter {
                                                                let short = pe.rsplit('/').next().unwrap_or(pe);
                                                                ui.label(egui::RichText::new(format!("    ✨ {short}")).size(11.0).color(dim));
                                                            }
                                                            if let Some(ref es) = entity.event_sheet {
                                                                let short = es.rsplit('/').next().unwrap_or(es);
                                                                ui.label(egui::RichText::new(format!("    📜 {short}")).size(11.0).color(dim));
                                                            }
                                                            if let Some(ref col) = entity.collider {
                                                                let shape = match col {
                                                                    toile_scene::ColliderData::Aabb { .. } => "AABB",
                                                                    toile_scene::ColliderData::Circle { .. } => "Circle",
                                                                };
                                                                ui.label(egui::RichText::new(format!("    🔲 {shape}")).size(11.0).color(dim));
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
        self.show_sprite_anim_panels(&ctx, &pdir);

        // Inspector panel — replaced by particle panel in Particle mode
        if self.editor_mode == EditorMode::Particle {
            egui::SidePanel::right("inspector").min_width(320.0).max_width(320.0).show(&ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.particle_editor.show(ui);
                });
            });
        }

        delete_selected |= self.show_inspector(&ctx, &pdir, &project_scripts, &project_particles);
        if delete_selected {
            if let Some(id) = self.selected_id.take() {
                self.scene.remove_entity(id);
                self.status_msg = format!("Deleted entity {id}");
            }
        }

        // ── Sprite & Animation Editor window ─────────────────────────────
        self.show_sprite_editor_window(&ctx, &pdir);

        // ── Frame Picker window ───────────────────────────────────────────
        self.show_frame_picker_window(&ctx, &pdir);

        // Scene Settings window
        if self.show_scene_settings {
            let mut open = true;
            egui::Window::new("Scene Settings")
                .open(&mut open)
                .default_width(300.0)
                .show(&ctx, |ui| {
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
                                if ui.selectable_label(mode_label == "Fixed", "Fixed — camera stays at position").clicked() {
                                    new_mode = Some(toile_scene::CameraMode::Fixed);
                                }
                                if ui.selectable_label(mode_label == "Follow Player", "Follow Player — always centered").clicked() {
                                    new_mode = Some(toile_scene::CameraMode::FollowPlayer);
                                }
                                if ui.selectable_label(mode_label == "Platformer Follow", "Platformer — deadzone + bounds").clicked() {
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
                                let path_str = if let Some(ref pd) = pdir {
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

        self.overlay.as_mut().unwrap().end_frame_and_render(device, queue, encoder, view, window, size);
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
