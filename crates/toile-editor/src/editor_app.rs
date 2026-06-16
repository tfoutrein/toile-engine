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

/// Duration of the animated splash screen, in seconds.
pub(crate) const SPLASH_DURATION: f32 = 2.8;
/// Cross-fade duration after the splash, so the app reveals smoothly instead of popping in.
pub(crate) const SPLASH_FADE: f32 = 0.5;

/// One sampled pixel of the logo spiral, animated as a fluid particle on the splash.
pub(crate) struct SplashParticle {
    pub target: Vec2,   // logo-local position (centered, +y up) at base size 256
    pub color: [u8; 3],
    pub seed: f32,      // 0..1 per-particle, for stagger + shimmer jitter
    pub is_text: bool,  // true for the "TOILE" pixels (vs the spiral)
}

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
    /// Entity being renamed inline in the hierarchy (double-click), with its edit buffer.
    pub(crate) hierarchy_rename: Option<(u64, String)>,
    /// Request keyboard focus for the rename field on the next frame only (so Enter commits).
    pub(crate) hierarchy_rename_focus: bool,
    pub(crate) hovered_id: Option<u64>,
    pub(crate) white_tex: Option<TextureHandle>,
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
    /// Counts down from SPLASH_FADE after the splash ends; fades the app in over the bg color.
    pub(crate) splash_fade_in: f32,
    /// Modal "Add Animation" dialog state (ADR-039 Phase 2). The dialog runs its add
    /// with push_undo outside the inspector's `&mut entity` borrow.
    pub(crate) show_add_anim_dialog: bool,
    pub(crate) add_anim_form: crate::panels::anim_states_ui::AddAnimForm,
    /// Modal "Replace base sprite" dialog state (ADR-039 Phase 3).
    pub(crate) show_replace_sprite_dialog: bool,
    pub(crate) replace_sprite_form: crate::panels::sprite_replace::ReplaceSpriteForm,
    /// Set by the inspector "Replace base sprite…" picker — drained next frame.
    pub(crate) pending_replace_sprite_file: Option<PathBuf>,
    /// Open viewport context menu (right-click), rendered as an egui Area (ADR-037).
    pub(crate) pending_context_menu: Option<crate::context_menu::ContextMenuKind>,
    /// Anchor (egui points) captured the first frame the viewport menu opens.
    pub(crate) context_menu_anchor: Option<egui::Pos2>,
    /// True when egui owns the pointer this frame (over a panel/menu); read next frame
    /// in handle_update so a viewport right-click never opens under an egui panel.
    pub(crate) egui_consumed_pointer: bool,
    /// Project pending a delete confirmation (welcome screen right-click → Delete).
    pub(crate) confirm_delete_project: Option<PathBuf>,
    /// Logo spiral sampled into fluid pixels (built once in init).
    pub(crate) splash_particles: Vec<SplashParticle>,
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
    /// Last frame picked in the Frame Picker, for Shift+click range selection (ADR-038).
    pub(crate) frame_picker_last: Option<u32>,
    pub(crate) show_sprite_editor: bool,
    pub(crate) sprite_editor_preview_anim: Option<(String, f32)>, // (anim_name, elapsed_frame)
    pub(crate) clipboard_entity: Option<EntityData>,
    pub(crate) show_viewport_guide: bool,
    pub(crate) last_mouse_pos: Vec2,
    pub(crate) panning: bool,
    /// Hand/pan tool toggle: when on, left-drag pans the viewport (like holding Space).
    pub(crate) pan_tool_active: bool,
    /// Dragging the starting-screen guide frame (grab its border to move the start camera).
    pub(crate) dragging_guide: bool,
    /// Pointer is over the guide-frame border (shows a move cursor).
    pub(crate) hovering_guide: bool,
    /// Offset between the guide center (camera_position) and the cursor at drag start.
    pub(crate) guide_drag_offset: Vec2,
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
    pub(crate) ai_md_cache: egui_commonmark::CommonMarkCache,
    /// Captured game logs from the last Play session.
    pub(crate) game_logs: Vec<String>,
    pub(crate) game_log_receiver: Option<std::sync::Arc<std::sync::Mutex<Vec<String>>>>,
    /// Handle to the running `toile run` child (Some while a game launched via Play is alive),
    /// so the Play button can toggle to Stop and actually terminate it.
    pub(crate) running_game: Option<std::process::Child>,
    pub(crate) ai_available_models: Vec<crate::ai::config::ModelInfo>,
    pub(crate) ai_models_loaded: bool,
    pub(crate) bug_reporter: crate::ai::bug_reporter::BugReporter,
    // Game Output console (captured stdout/stderr from the last Play session)
    pub(crate) show_game_output: bool,
    // Input Map panel
    pub(crate) show_input_map: bool,
    pub(crate) input_map_listening: Option<String>, // action name we're capturing a binding for
    // Snapshot of gamepad/actions state for UI display (updated each frame)
    pub(crate) gamepad_snapshot: Vec<(usize, toile_app::platform::GamepadState)>,
    pub(crate) actions_snapshot: Vec<(String, String, bool, f32, [f32; 2])>, // (name, type, pressed, value, vec2)
    pub(crate) actions_bindings_snapshot: Vec<(String, String, Vec<String>)>, // (name, type, binding_strs)
    // Pending mutations from UI → applied in update()
    pub(crate) input_map_pending_add_binding: Option<(String, toile_app::platform::input_actions::InputBinding)>,
    pub(crate) input_map_pending_remove_binding: Option<(String, usize)>,
    pub(crate) input_map_pending_add_action: Option<toile_app::platform::input_actions::InputAction>,
    pub(crate) input_map_pending_remove_action: Option<String>,
    pub(crate) input_map_save_requested: bool,
    // Undo/redo history (scene snapshots).
    pub(crate) undo_stack: Vec<SceneData>,
    pub(crate) redo_stack: Vec<SceneData>,
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
            hierarchy_rename: None,
            hierarchy_rename_focus: false,
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
            splash_timer: SPLASH_DURATION,
            show_splash: true,
            splash_fade_in: 0.0,
            show_add_anim_dialog: false,
            add_anim_form: crate::panels::anim_states_ui::AddAnimForm::default(),
            show_replace_sprite_dialog: false,
            replace_sprite_form: crate::panels::sprite_replace::ReplaceSpriteForm::default(),
            pending_replace_sprite_file: None,
            pending_context_menu: None,
            context_menu_anchor: None,
            egui_consumed_pointer: false,
            confirm_delete_project: None,
            splash_particles: Vec::new(),
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
            frame_picker_last: None,
            show_sprite_editor: false,
            sprite_editor_preview_anim: None,
            clipboard_entity: None,
            show_viewport_guide: true,
            last_mouse_pos: Vec2::ZERO,
            panning: false,
            pan_tool_active: false,
            dragging_guide: false,
            hovering_guide: false,
            guide_drag_offset: Vec2::ZERO,
            editor_mode: EditorMode::Entity,
            asset_browser: AssetBrowserApp::new(),
            ai_config: AiConfig::load(),
            ai_messages: Vec::new(),
            ai_input: String::new(),
            ai_loading: false,
            ai_show_settings: false,
            ai_response_rx: None,
            ai_md_cache: egui_commonmark::CommonMarkCache::default(),
            game_logs: Vec::new(),
            game_log_receiver: None,
            running_game: None,
            ai_available_models: Vec::new(),
            ai_models_loaded: false,
            bug_reporter: Default::default(),
            show_game_output: false,
            show_input_map: false,
            input_map_listening: None,
            gamepad_snapshot: Vec::new(),
            actions_snapshot: Vec::new(),
            actions_bindings_snapshot: Vec::new(),
            input_map_pending_add_binding: None,
            input_map_pending_remove_binding: None,
            input_map_pending_add_action: None,
            input_map_pending_remove_action: None,
            input_map_save_requested: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Snapshot the current scene onto the undo stack BEFORE a mutating action.
    /// Skips no-op snapshots (identical to the last one) and clears the redo stack.
    pub(crate) fn push_undo(&mut self) {
        const MAX_HISTORY: usize = 64;
        if let Some(top) = self.undo_stack.last() {
            if scenes_equal(top, &self.scene) {
                self.redo_stack.clear();
                return;
            }
        }
        self.undo_stack.push(self.scene.clone());
        if self.undo_stack.len() > MAX_HISTORY {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// If the most recent snapshot equals the current scene, drop it (used after a
    /// gesture that ended up changing nothing, e.g. a click that only selected).
    pub(crate) fn discard_undo_if_unchanged(&mut self) {
        if let Some(top) = self.undo_stack.last() {
            if scenes_equal(top, &self.scene) {
                self.undo_stack.pop();
            }
        }
    }

    pub(crate) fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(std::mem::replace(&mut self.scene, prev));
            self.after_history_restore();
            self.status_msg = format!("Undo ({} left)", self.undo_stack.len());
        } else {
            self.status_msg = "Nothing to undo".to_string();
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(std::mem::replace(&mut self.scene, next));
            self.after_history_restore();
            self.status_msg = "Redo".to_string();
        } else {
            self.status_msg = "Nothing to redo".to_string();
        }
    }

    /// Fix up editor state after restoring a scene snapshot.
    fn after_history_restore(&mut self) {
        if let Some(id) = self.selected_id {
            if !self.scene.entities.iter().any(|e| e.id == id) {
                self.selected_id = None;
            }
        }
        // Sprite textures may now refer to different paths; rebuild lazily.
        self.sprite_cache.clear();
    }

    /// Persist the current scene to its file if one is set. Called before switching
    /// scenes / creating a new one so in-progress edits are never silently lost.
    pub(crate) fn autosave_current_scene(&mut self) {
        if self.current_file.is_empty() {
            return;
        }
        if let Some(dir) = &self.project_dir {
            let path = dir.join(&self.current_file);
            if let Ok(json) = serde_json::to_string_pretty(&self.scene) {
                let _ = std::fs::write(&path, json);
            }
        }
    }

    /// Forget undo/redo history (e.g. after switching scenes — history belongs to
    /// the scene it was recorded against).
    pub(crate) fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
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
    pub(crate) fn list_project_scenes(&self) -> Vec<String> {
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
    pub(crate) fn list_project_files(&self, subdir: &str, ext: &str) -> Vec<String> {
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
        self.surface_format = Some(ctx.surface_format());

        // Sample the WHOLE logo (spiral + "TOILE") into fluid pixels for the animated splash.
        if self.splash_particles.is_empty() {
            if let Ok(img) = image::open("assets/toile-logo-transparent.png") {
                let img = img.to_rgba8();
                let (iw, ih) = img.dimensions();
                let base = 256.0_f32;
                let split_py = (ih as f32 * 0.685) as u32; // spiral above, "TOILE" below
                let step = 6u32;
                let mut idx = 0u32;
                let mut py = 0u32;
                while py < ih {
                    let mut px = 0u32;
                    while px < iw {
                        let p = img.get_pixel(px, py).0;
                        if p[3] >= 60 {
                            let lx = (px as f32 / iw as f32 - 0.5) * base;
                            let ly = (0.5 - py as f32 / ih as f32) * base;
                            self.splash_particles.push(SplashParticle {
                                target: Vec2::new(lx, ly),
                                color: [p[0], p[1], p[2]],
                                seed: (idx as f32 * 0.6180339).fract(),
                                is_text: py >= split_py,
                            });
                            idx += 1;
                        }
                        px += step;
                    }
                    py += step;
                }
                log::info!("Splash: sampled {} logo pixels", self.splash_particles.len());
            }
        }

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

        // Poll the AI worker every frame — NOT only while the Copilot panel is
        // open — so closing the panel mid-request cannot leave `ai_loading` stuck
        // true and permanently lock the Copilot for the session (audit C1).
        self.check_ai_response();

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

        // Cross-fade the app in over the next screen's bg color, so it reveals smoothly after the
        // splash instead of popping in. A full-screen rect on the topmost layer fades from the
        // panel fill color to transparent.
        if self.splash_fade_in > 0.0 {
            let t = (self.splash_fade_in / SPLASH_FADE).clamp(0.0, 1.0);
            let a = t * t * (3.0 - 2.0 * t); // smoothstep
            let bg = ctx.style().visuals.panel_fill;
            let color = egui::Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), (a * 255.0) as u8);
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("splash_fade_in"),
            ));
            painter.rect_filled(ctx.screen_rect(), 0.0, color);
            ctx.request_repaint();
        }

        // Viewport right-click context menu (ADR-037): drawn as an egui Area over the wgpu
        // viewport. Then record whether egui owns the pointer this frame, so next frame's
        // handle_update won't open a viewport menu under a panel.
        self.show_viewport_context_menu(&ctx);
        self.egui_consumed_pointer = ctx.is_pointer_over_area() || ctx.wants_pointer_input();

        self.overlay.as_mut().unwrap().end_frame_and_render(device, queue, encoder, view, window, size);
    }

    fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        if let Some(overlay) = &mut self.overlay {
            overlay.handle_event(window, event)
        } else {
            false
        }
    }

    /// Throttle the redraw loop so a static editor screen stops pegging the CPU/GPU
    /// (perf fix — was ~27% idle from rendering every frame). Keep rendering every frame
    /// only while something is actually animating; otherwise follow egui's own repaint
    /// timing, capped at 100 ms so async results (game logs, asset loads, AI replies)
    /// still surface promptly. Input always forces an immediate redraw (see AppHandler).
    fn redraw_after(&self) -> std::time::Duration {
        use std::time::Duration;
        let animating = self.splash_fade_in > 0.0
            || self.sprite_editor_preview_anim.is_some()
            || self.asset_browser.ai_analyzing
            || self.ai_loading; // keep the Copilot stream + spinner smooth, not capped to 10fps
        if animating {
            return Duration::ZERO;
        }
        self.overlay
            .as_ref()
            .map(|o| o.repaint_after())
            .unwrap_or(Duration::ZERO)
            .min(Duration::from_millis(100))
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

/// Cheap structural equality for undo/redo dedup (compares serialized form).
fn scenes_equal(a: &SceneData, b: &SceneData) -> bool {
    match (serde_json::to_string(a), serde_json::to_string(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
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
