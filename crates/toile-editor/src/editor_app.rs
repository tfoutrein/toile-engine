use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::Vec2;
use toile_app::{App, Game, GameContext, TextureHandle};
use toile_core::color::Color;
use toile_core::particles::ParticlePool;
use winit::event::WindowEvent;
use winit::window::Window;

use toile_asset_library::ui::AssetBrowserApp;
use crate::ai::client::ChatMessage;
use crate::ai::config::AiConfig;

use crate::overlay::EguiOverlay;
use crate::particle_editor::ParticleEditorPanel;
use crate::scene_data::{EntityData, SceneData};
use crate::tilemap_tool::TilemapEditor;

pub struct EditorApp {
    pub(crate) overlay: Option<EguiOverlay>,
    pub(crate) surface_format: Option<wgpu::TextureFormat>,
    // Workspace — root directory for projects, asset packs, working files
    pub(crate) workspace_dir: PathBuf,
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
    // Asset browser (embedded from toile-asset-library)
    pub(crate) asset_browser: AssetBrowserApp,
    // AI Copilot
    pub(crate) ai_config: AiConfig,
    pub(crate) ai_messages: Vec<ChatMessage>,
    pub(crate) ai_input: String,
    pub(crate) ai_loading: bool,
    pub(crate) ai_show_settings: bool,
    pub(crate) ai_response_rx: Option<std::sync::mpsc::Receiver<Result<crate::ai::client::ApiResponse, String>>>,
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
    AssetBrowser,
    AICopilot,
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

        // Load workspace dir from config, or default to ./workspace/
        let workspace_dir = load_workspace_config();

        Self {
            overlay: None,
            surface_format: None,
            workspace_dir,
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
            asset_browser: AssetBrowserApp::new(),
            ai_config: AiConfig::load(),
            ai_messages: Vec::new(),
            ai_input: String::new(),
            ai_loading: false,
            ai_show_settings: false,
            ai_response_rx: None,
        }
    }

    /// Resolve a path relative to the project directory.
    pub(crate) fn project_path(&self, relative: &str) -> PathBuf {
        match &self.project_dir {
            Some(dir) => dir.join(relative),
            None => PathBuf::from(relative),
        }
    }

    /// Create a minimal project structure.
    pub(crate) fn create_project(&self, dir: &Path) -> Result<(), String> {
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
    pub(crate) fn open_project(&mut self, dir: PathBuf) {
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
    pub(crate) fn auto_update_bounds_from_tiles(&mut self) {
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
        self.handle_update(ctx, _dt);
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        self.draw_viewport(ctx);
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

        // Delegate to extracted overlay panels
        self.show_overlay_panels(&ctx, &project_scenes, &project_scripts, &project_particles, &pdir);

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

// ── Workspace config persistence ────────────────────────────────────

const WORKSPACE_CONFIG_FILE: &str = "toile-editor-config.json";

fn config_path() -> PathBuf {
    // Store config in user's home directory
    if let Some(home) = dirs_fallback() {
        let dir = home.join(".toile");
        let _ = std::fs::create_dir_all(&dir);
        dir.join(WORKSPACE_CONFIG_FILE)
    } else {
        PathBuf::from(WORKSPACE_CONFIG_FILE)
    }
}

fn dirs_fallback() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

fn load_workspace_config() -> PathBuf {
    let path = config_path();
    if path.exists() {
        if let Ok(json) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(ws) = config.get("workspace_dir").and_then(|v| v.as_str()) {
                    let dir = PathBuf::from(ws);
                    if dir.exists() {
                        return dir;
                    }
                }
            }
        }
    }
    // Default: workspace/ next to current dir
    let default = PathBuf::from("workspace");
    let _ = std::fs::create_dir_all(&default);
    default
}

pub(crate) fn save_workspace_config(workspace_dir: &std::path::Path) {
    let path = config_path();
    let config = serde_json::json!({
        "workspace_dir": workspace_dir.to_string_lossy(),
    });
    if let Ok(json) = serde_json::to_string_pretty(&config) {
        let _ = std::fs::write(&path, json);
    }
}
