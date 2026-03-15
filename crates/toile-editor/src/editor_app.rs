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

// ── Behavior helpers for the Inspector ──────────────────────────────────

/// Pick an icon based on entity properties.
fn entity_icon(entity: &EntityData) -> &'static str {
    if entity.light.is_some() { return "💡"; }
    if entity.particle_emitter.is_some() { return "✨"; }
    if entity.tags.iter().any(|t| t.eq_ignore_ascii_case("player")) { return "🧑"; }
    if entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Solid)) { return "🧱"; }
    if entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Platform(_) | BehaviorConfig::TopDown(_))) { return "🏃"; }
    "📦"
}

fn behavior_label(beh: &BehaviorConfig) -> &'static str {
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

fn default_behavior_config(name: &str) -> BehaviorConfig {
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

fn post_effect_label(fx: &toile_scene::PostEffectData) -> &'static str {
    match fx {
        toile_scene::PostEffectData::Vignette { .. } => "Vignette",
        toile_scene::PostEffectData::Crt { .. } => "CRT",
        toile_scene::PostEffectData::Pixelate { .. } => "Pixelate",
        toile_scene::PostEffectData::Bloom { .. } => "Bloom",
        toile_scene::PostEffectData::ColorGrading { .. } => "Color Grading",
    }
}

fn default_post_effect(name: &str) -> toile_scene::PostEffectData {
    match name {
        "Vignette" => toile_scene::PostEffectData::Vignette { intensity: 0.5, smoothness: 0.5 },
        "Bloom" => toile_scene::PostEffectData::Bloom { threshold: 0.8, intensity: 0.5, radius: 2.0 },
        "CRT" => toile_scene::PostEffectData::Crt { scanline_intensity: 0.3, curvature: 0.02, chromatic_aberration: 0.003 },
        "Pixelate" => toile_scene::PostEffectData::Pixelate { pixel_size: 4.0 },
        "ColorGrading" => toile_scene::PostEffectData::ColorGrading { saturation: 1.0, brightness: 1.0, contrast: 1.0 },
        _ => toile_scene::PostEffectData::Vignette { intensity: 0.5, smoothness: 0.5 },
    }
}

fn post_effect_inspector(ui: &mut egui::Ui, fx: &mut toile_scene::PostEffectData, idx: usize) {
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

fn behavior_inspector(ui: &mut egui::Ui, beh: &mut BehaviorConfig, idx: usize) {
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

pub struct EditorApp {
    overlay: Option<EguiOverlay>,
    surface_format: Option<wgpu::TextureFormat>,
    // Project state
    project_dir: Option<PathBuf>,
    show_project_dialog: bool,
    project_path_input: String,
    new_project_name: String,
    new_project_template: String,
    show_file_picker: Option<FilePickerTarget>,
    // Scene state
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
    // Background
    background_tex: Option<TextureHandle>,
    background_path_loaded: String,
    // Particle editor
    particle_editor: ParticleEditorPanel,
    // Live particle preview pools (entity_id → pool)
    preview_particles: HashMap<u64, ParticlePool>,
    // Track which emitter path each pool was built from
    preview_particle_paths: HashMap<u64, String>,
    show_scene_settings: bool,
    show_viewport_guide: bool,
    last_mouse_pos: Vec2,
    panning: bool,
    editor_mode: EditorMode,
}

/// What field the file picker is targeting.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FilePickerTarget {
    SpritePath,
    EventSheet,
    ParticleEmitter,
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
            particle_editor: ParticleEditorPanel::new(),
            preview_particles: HashMap::new(),
            preview_particle_paths: HashMap::new(),
            show_scene_settings: false,
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

                if let Some(pos) = new_tile {
                    self.scene.settings.background_tiles.push(pos);
                    // Auto-update PlatformerFollow bounds to cover all tiles
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

        // Draw entities
        for entity in &self.scene.entities {
            let selected = self.selected_id == Some(entity.id);
            let is_player_ent = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("player"));
            let is_solid = entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Solid));
            let is_coin = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("coin"));
            let is_enemy = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("enemy"));
            let color = if selected {
                pack_color(255, 220, 80, 255)   // yellow — selected
            } else if is_player_ent {
                pack_color(80, 220, 120, 255)   // green — player
            } else if is_solid {
                pack_color(160, 160, 180, 255)  // grey — solid/ground
            } else if is_coin {
                pack_color(255, 220, 50, 200)   // gold — collectible
            } else if is_enemy {
                pack_color(220, 80, 80, 255)    // red — enemy
            } else {
                pack_color(100, 150, 220, 255)  // blue — default
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
        let overlay = self.overlay.get_or_insert_with(|| {
            let o = EguiOverlay::new(device, surface_format, window);
            let mut style = (*o.ctx().style()).clone();
            style.visuals = egui::Visuals::dark();
            o.ctx().set_style(style);
            o
        });

        overlay.begin_frame(window);

        let ctx = overlay.ctx().clone();

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

            overlay.end_frame_and_render(device, queue, encoder, view, window, size);

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
        if self.editor_mode != EditorMode::Particle {
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

        // Inspector panel — replaced by particle panel in Particle mode
        if self.editor_mode == EditorMode::Particle {
            egui::SidePanel::right("inspector").min_width(320.0).max_width(320.0).show(&ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.particle_editor.show(ui);
                });
            });
        }

        if self.editor_mode != EditorMode::Particle {
        egui::SidePanel::right("inspector").min_width(280.0).default_width(300.0).show(&ctx, |ui| {
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
                            ui.label("Vis");
                            ui.checkbox(&mut entity.visible, "");
                            ui.end_row();

                            ui.label("Sprite");
                            ui.text_edit_singleline(&mut entity.sprite_path);
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                        });

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

                    // ── Event Sheet ───────────────────────────────────────
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
                                    for f in &project_scripts {
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
                                    for f in &project_particles {
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
                        if ui.small_button("Clear background").clicked() {
                            s.background_image = None;
                        }
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
