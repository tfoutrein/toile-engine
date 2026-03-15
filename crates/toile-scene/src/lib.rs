pub mod prefab;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toile_behaviors::BehaviorConfig;

#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct SceneData {
    pub name: String,
    pub entities: Vec<EntityData>,
    #[serde(default)]
    pub tilemap: Option<TilemapData>,
    #[serde(default)]
    pub settings: SceneSettings,
    #[serde(skip)]
    pub next_id: u64,
}

/// Scene-level settings (gravity, viewport, camera, background).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SceneSettings {
    #[serde(default = "default_gravity")]
    pub gravity: f32,
    #[serde(default = "default_viewport_w")]
    pub viewport_width: u32,
    #[serde(default = "default_viewport_h")]
    pub viewport_height: u32,
    #[serde(default = "default_clear_color")]
    pub clear_color: [f32; 4],
    #[serde(default = "default_camera_zoom")]
    pub camera_zoom: f32,
    #[serde(default)]
    pub camera_position: [f32; 2],
    #[serde(default)]
    pub camera_mode: CameraMode,
    #[serde(default)]
    pub background_image: Option<String>,
    /// Positions of background tile instances (world-space centers).
    /// First entry is the main tile. Click "+" in editor to add adjacent tiles.
    #[serde(default)]
    pub background_tiles: Vec<[f32; 2]>,
    #[serde(default)]
    pub lighting: LightingSettings,
    #[serde(default)]
    pub post_effects: Vec<PostEffectData>,
}

fn default_gravity() -> f32 { 800.0 }
fn default_viewport_w() -> u32 { 1280 }
fn default_viewport_h() -> u32 { 720 }
fn default_clear_color() -> [f32; 4] { [0.1, 0.1, 0.15, 1.0] }
fn default_camera_zoom() -> f32 { 1.0 }

impl Default for SceneSettings {
    fn default() -> Self {
        Self {
            gravity: default_gravity(),
            viewport_width: default_viewport_w(),
            viewport_height: default_viewport_h(),
            clear_color: default_clear_color(),
            camera_zoom: default_camera_zoom(),
            camera_position: [0.0, 0.0],
            camera_mode: CameraMode::default(),
            background_image: None,
            background_tiles: Vec::new(),
            lighting: LightingSettings::default(),
            post_effects: Vec::new(),
        }
    }
}

/// Camera behavior mode.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub enum CameraMode {
    /// Camera stays at camera_position, shows the designed viewport.
    #[default]
    Fixed,
    /// Camera centers on the Player entity.
    FollowPlayer,
    /// Platformer camera: follows player horizontally with deadzone,
    /// only scrolls vertically when player reaches screen edges.
    /// Camera is clamped to scene bounds so the background never shows gaps.
    PlatformerFollow {
        /// Horizontal deadzone: fraction of viewport width (0.0–1.0).
        /// Player can move this much before camera scrolls. Default 0.3.
        #[serde(default = "default_deadzone_x")]
        deadzone_x: f32,
        /// Vertical deadzone: fraction of viewport height. Default 0.4.
        #[serde(default = "default_deadzone_y")]
        deadzone_y: f32,
        /// Scene bounds: camera won't show beyond these world-space limits.
        /// [min_x, min_y, max_x, max_y]. If all zero, no clamping.
        #[serde(default)]
        bounds: [f32; 4],
    },
}

fn default_deadzone_x() -> f32 { 0.3 }
fn default_deadzone_y() -> f32 { 0.4 }

/// Sprite sheet configuration for frame-based animation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpriteSheetData {
    /// Width of a single frame in pixels.
    pub frame_width: u32,
    /// Height of a single frame in pixels.
    pub frame_height: u32,
    /// Number of columns in the sheet.
    pub columns: u32,
    /// Number of rows in the sheet.
    pub rows: u32,
}

/// A named animation: sequence of frame indices + playback speed.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnimationData {
    pub name: String,
    /// Frame indices in the sprite sheet (0-based, left-to-right top-to-bottom).
    pub frames: Vec<u32>,
    /// Playback speed in frames per second.
    pub fps: f32,
    /// Whether the animation loops.
    #[serde(default = "default_true_val")]
    pub looping: bool,
}

fn default_true_val() -> bool { true }

/// Collision shape data for scene serialization.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "shape")]
pub enum ColliderData {
    Aabb { half_w: f32, half_h: f32 },
    Circle { radius: f32 },
}

/// Point light data attached to an entity.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LightData {
    #[serde(default = "default_light_radius")]
    pub radius: f32,
    #[serde(default = "default_light_falloff")]
    pub falloff: f32,
    #[serde(default = "default_light_color")]
    pub color: [f32; 3],
    #[serde(default = "default_light_intensity")]
    pub intensity: f32,
    #[serde(default)]
    pub cast_shadow: bool,
}

fn default_light_radius() -> f32 { 150.0 }
fn default_light_falloff() -> f32 { 2.0 }
fn default_light_color() -> [f32; 3] { [1.0, 1.0, 1.0] }
fn default_light_intensity() -> f32 { 1.0 }

impl Default for LightData {
    fn default() -> Self {
        Self {
            radius: default_light_radius(),
            falloff: default_light_falloff(),
            color: default_light_color(),
            intensity: default_light_intensity(),
            cast_shadow: false,
        }
    }
}

/// Lighting settings for the scene.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LightingSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ambient")]
    pub ambient: [f32; 4],
    #[serde(default)]
    pub shadows_enabled: bool,
}

fn default_ambient() -> [f32; 4] { [0.1, 0.1, 0.15, 1.0] }

/// Post-processing effect entry (serializable subset).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum PostEffectData {
    Vignette { intensity: f32, smoothness: f32 },
    Crt { scanline_intensity: f32, curvature: f32, chromatic_aberration: f32 },
    Pixelate { pixel_size: f32 },
    Bloom { threshold: f32, intensity: f32, radius: f32 },
    ColorGrading { saturation: f32, brightness: f32, contrast: f32 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TilemapData {
    pub tileset_path: String,
    pub tile_size: u32,
    pub columns: u32,
    pub width: u32,
    pub height: u32,
    pub layers: Vec<TilemapLayerData>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TilemapLayerData {
    pub name: String,
    pub tiles: Vec<u32>, // row-major, 0 = empty
    pub visible: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntityData {
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default = "default_scale")]
    pub scale_x: f32,
    #[serde(default = "default_scale")]
    pub scale_y: f32,
    #[serde(default)]
    pub layer: i32,
    #[serde(default)]
    pub sprite_path: String,
    #[serde(default = "default_size")]
    pub width: f32,
    #[serde(default = "default_size")]
    pub height: f32,
    // ── v0.5 fields ──────────────────────────────────────────────────────
    #[serde(default)]
    pub behaviors: Vec<BehaviorConfig>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub variables: HashMap<String, f64>,
    #[serde(default)]
    pub collider: Option<ColliderData>,
    #[serde(default)]
    pub event_sheet: Option<String>,
    #[serde(default)]
    pub particle_emitter: Option<String>,
    #[serde(default)]
    pub sprite_sheet: Option<SpriteSheetData>,
    #[serde(default)]
    pub animations: Vec<AnimationData>,
    #[serde(default)]
    pub default_animation: Option<String>,
    #[serde(default)]
    pub light: Option<LightData>,
    #[serde(default = "default_visible")]
    pub visible: bool,
}

fn default_scale() -> f32 { 1.0 }
fn default_size() -> f32 { 32.0 }
fn default_visible() -> bool { true }

impl Default for EntityData {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            x: 0.0, y: 0.0,
            rotation: 0.0,
            scale_x: 1.0, scale_y: 1.0,
            layer: 0,
            sprite_path: String::new(),
            width: 32.0, height: 32.0,
            behaviors: Vec::new(),
            tags: Vec::new(),
            variables: HashMap::new(),
            collider: None,
            event_sheet: None,
            particle_emitter: None,
            sprite_sheet: None,
            animations: Vec::new(),
            default_animation: None,
            light: None,
            visible: true,
        }
    }
}

impl SceneData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            entities: Vec::new(),
            tilemap: None,
            settings: SceneSettings::default(),
            next_id: 1,
        }
    }

    pub fn add_entity(&mut self, name: &str, x: f32, y: f32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(EntityData {
            id,
            name: name.to_string(),
            x, y,
            ..Default::default()
        });
        id
    }

    pub fn remove_entity(&mut self, id: u64) {
        self.entities.retain(|e| e.id != id);
    }

    pub fn find_entity_mut(&mut self, id: u64) -> Option<&mut EntityData> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Recompute next_id from existing entities (needed after deserialization).
    pub fn fix_next_id(&mut self) {
        self.next_id = self.entities.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    }
}

/// Load a scene from a JSON file.
pub fn load_scene(path: &Path) -> Result<SceneData, SceneError> {
    let json = std::fs::read_to_string(path)?;
    let mut scene: SceneData = serde_json::from_str(&json)?;
    scene.fix_next_id();
    Ok(scene)
}

/// Save a scene to a JSON file.
pub fn save_scene(path: &Path, scene: &SceneData) -> Result<(), SceneError> {
    let json = serde_json::to_string_pretty(scene)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// List all .json scene files in a directory.
pub fn list_scene_files(dir: &Path) -> Result<Vec<PathBuf>, SceneError> {
    let mut scenes = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                scenes.push(path);
            }
        }
    }
    scenes.sort();
    Ok(scenes)
}
