//! Data-driven game runner — loads a SceneData and runs it as a playable game.
//!
//! Implements the `Game` trait from toile-app, bridging:
//! - Behaviors (Platform, TopDown, Bullet, Sine, Fade, Wrap, Solid)
//! - Event sheets (conditions → actions)
//! - Collision detection (AABB/Circle via spatial grid)
//! - Particle emitters attached to entities

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::Vec2;
use toile_app::{App, Game, GameContext, Key, MouseButton, TextureHandle};
use toile_app::core::color::Color;
use toile_app::core::particles::{ParticleEmitter, ParticlePool};
use toile_behaviors::*;
use toile_collision::{Collider, Shape, SpatialGrid, overlap_test};
use toile_events::executor::{EventCommand, EventContext, EventSheetState, evaluate_event_sheet};
use toile_events::model::EventSheet;
use toile_graphics::sprite_renderer::DrawSprite;
use toile_scene::{ColliderData, EntityData, SceneData};

use crate::manifest::ProjectManifest;

// ── Runtime entity ──────────────────────────────────────────────────────

/// Runtime behavior state paired with its config.
enum BehaviorRuntime {
    Platform { config: platform::PlatformConfig, state: platform::PlatformState },
    TopDown { config: topdown::TopDownConfig },
    Bullet { config: bullet::BulletConfig, state: bullet::BulletState },
    Sine { config: sine::SineConfig, state: sine::SineState },
    Fade { config: fade::FadeConfig, state: fade::FadeState },
    Wrap { config: wrap::WrapConfig },
    Solid,
}

impl BehaviorRuntime {
    fn from_config(cfg: &BehaviorConfig) -> Self {
        match cfg {
            BehaviorConfig::Platform(c) => Self::Platform { config: c.clone(), state: Default::default() },
            BehaviorConfig::TopDown(c)  => Self::TopDown { config: c.clone() },
            BehaviorConfig::Bullet(c)   => Self::Bullet { config: c.clone(), state: Default::default() },
            BehaviorConfig::Sine(c)     => Self::Sine { config: c.clone(), state: Default::default() },
            BehaviorConfig::Fade(c)     => Self::Fade { config: c.clone(), state: Default::default() },
            BehaviorConfig::Wrap(c)     => Self::Wrap { config: c.clone() },
            BehaviorConfig::Solid       => Self::Solid,
        }
    }
}

struct RuntimeEntity {
    data: EntityData,
    es: EntityState,
    behaviors: Vec<BehaviorRuntime>,
    event_sheet: Option<EventSheet>,
    event_state: EventSheetState,
    collider: Collider,
    texture: Option<TextureHandle>,
    particle_pool: Option<ParticlePool>,
    alive: bool,
}

fn collider_from_data(data: &EntityData) -> Collider {
    match &data.collider {
        Some(ColliderData::Aabb { half_w, half_h }) => Collider::aabb(*half_w, *half_h),
        Some(ColliderData::Circle { radius }) => Collider::circle(*radius),
        None => Collider::aabb(data.width * data.scale_x * 0.5, data.height * data.scale_y * 0.5),
    }
}

fn entity_state_from_data(data: &EntityData) -> EntityState {
    EntityState {
        position: Vec2::new(data.x, data.y),
        velocity: Vec2::ZERO,
        rotation: data.rotation,
        on_ground: false,
        size: Vec2::new(data.width * data.scale_x, data.height * data.scale_y),
        opacity: 1.0,
        alive: true,
    }
}

fn is_player(data: &EntityData) -> bool {
    data.tags.iter().any(|t| t.eq_ignore_ascii_case("player"))
}

fn has_solid_behavior(ent: &RuntimeEntity) -> bool {
    ent.behaviors.iter().any(|b| matches!(b, BehaviorRuntime::Solid))
}

// ── Game Runner ─────────────────────────────────────────────────────────

pub struct GameRunner {
    project_dir: PathBuf,
    manifest: ProjectManifest,
    entities: Vec<RuntimeEntity>,
    spatial_grid: SpatialGrid,
    white_tex: Option<TextureHandle>,
    textures: HashMap<String, TextureHandle>,
    prefabs: HashMap<String, toile_scene::prefab::Prefab>,
    event_sheets: HashMap<String, EventSheet>,
    background_tex: Option<TextureHandle>,
    pending_scene: Option<String>,
    next_id: u64,
    scene_settings: toile_scene::SceneSettings,
    /// Smooth camera position for platformer follow mode.
    camera_pos: Vec2,
}

impl GameRunner {
    pub fn load(project_dir: &Path) -> Result<Self, String> {
        let manifest = ProjectManifest::load(project_dir)?;
        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            manifest,
            entities: Vec::new(),
            spatial_grid: SpatialGrid::new(64.0),
            white_tex: None,
            background_tex: None,
            textures: HashMap::new(),
            prefabs: HashMap::new(),
            event_sheets: HashMap::new(),
            pending_scene: None,
            next_id: 1,
            scene_settings: Default::default(),
            camera_pos: Vec2::ZERO,
        })
    }

    pub fn manifest(&self) -> &ProjectManifest { &self.manifest }

    fn resolve(&self, relative: &str) -> PathBuf {
        self.project_dir.join(relative)
    }

    fn load_scene_data(&mut self, scene: &SceneData, ctx: &mut GameContext) {
        self.entities.clear();
        self.next_id = scene.next_id.max(1);
        self.scene_settings = scene.settings.clone();

        // Load background image
        self.background_tex = scene.settings.background_image.as_ref().and_then(|path| {
            let full = self.resolve(path);
            if full.exists() {
                Some(ctx.load_texture(&full))
            } else {
                log::warn!("Background image not found: {}", full.display());
                None
            }
        });

        for edata in &scene.entities {
            let rt = self.spawn_entity(edata, ctx);
            self.entities.push(rt);
        }
    }

    fn spawn_entity(&mut self, data: &EntityData, ctx: &mut GameContext) -> RuntimeEntity {
        let behaviors: Vec<BehaviorRuntime> = data.behaviors.iter()
            .map(BehaviorRuntime::from_config)
            .collect();

        // Load texture if sprite_path is set
        let texture = if !data.sprite_path.is_empty() {
            Some(self.load_texture_cached(&data.sprite_path, ctx))
        } else {
            None
        };

        // Load event sheet if referenced
        let event_sheet = data.event_sheet.as_ref().and_then(|path| {
            if let Some(cached) = self.event_sheets.get(path) {
                return Some(cached.clone());
            }
            let full_path = self.resolve(path);
            match std::fs::read_to_string(&full_path) {
                Ok(json) => {
                    match serde_json::from_str::<EventSheet>(&json) {
                        Ok(sheet) => {
                            self.event_sheets.insert(path.clone(), sheet.clone());
                            Some(sheet)
                        }
                        Err(e) => { log::error!("Event sheet parse error {path}: {e}"); None }
                    }
                }
                Err(e) => { log::warn!("Cannot load event sheet {path}: {e}"); None }
            }
        });

        // Load particle emitter if referenced
        let particle_pool = data.particle_emitter.as_ref().and_then(|path| {
            let full_path = self.resolve(path);
            match std::fs::read_to_string(&full_path) {
                Ok(json) => {
                    match serde_json::from_str::<ParticleEmitter>(&json) {
                        Ok(emitter) => Some(ParticlePool::new(emitter, Vec2::new(data.x, data.y))),
                        Err(e) => { log::error!("Particle emitter parse error {path}: {e}"); None }
                    }
                }
                Err(e) => { log::warn!("Cannot load particle emitter {path}: {e}"); None }
            }
        });

        // Initialize event sheet state with entity variables
        let mut event_state = EventSheetState::default();
        for (k, v) in &data.variables {
            event_state.variables.insert(k.clone(), *v);
        }

        RuntimeEntity {
            es: entity_state_from_data(data),
            collider: collider_from_data(data),
            data: data.clone(),
            behaviors,
            event_sheet,
            event_state,
            texture,
            particle_pool,
            alive: true,
        }
    }

    fn load_texture_cached(&mut self, path: &str, ctx: &mut GameContext) -> TextureHandle {
        if let Some(&tex) = self.textures.get(path) {
            return tex;
        }
        let full_path = self.resolve(path);
        let tex = ctx.load_texture(&full_path);
        self.textures.insert(path.to_string(), tex);
        tex
    }

    fn load_prefabs(&mut self) {
        let prefab_dir = self.project_dir.join("prefabs");
        if let Ok(entries) = std::fs::read_dir(&prefab_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    if let Ok(prefab) = toile_scene::prefab::load_prefab(&path) {
                        self.prefabs.insert(prefab.name.clone(), prefab);
                    }
                }
            }
        }
    }

    fn build_behavior_input(ctx: &GameContext) -> BehaviorInput {
        BehaviorInput {
            left: ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA),
            right: ctx.input.is_key_down(Key::ArrowRight) || ctx.input.is_key_down(Key::KeyD),
            up: ctx.input.is_key_down(Key::ArrowUp) || ctx.input.is_key_down(Key::KeyW),
            down: ctx.input.is_key_down(Key::ArrowDown) || ctx.input.is_key_down(Key::KeyS),
            jump_pressed: ctx.input.is_key_just_pressed(Key::Space)
                || ctx.input.is_key_just_pressed(Key::ArrowUp)
                || ctx.input.is_key_just_pressed(Key::KeyW),
            jump_down: ctx.input.is_key_down(Key::Space),
        }
    }

    /// Build collision map: entity_id → set of tags it's colliding with.
    fn compute_collisions(&mut self) -> HashMap<u64, Vec<String>> {
        self.spatial_grid.clear();
        for (i, ent) in self.entities.iter().enumerate() {
            if !ent.alive { continue; }
            let half = ent.collider.bounding_half_extents();
            self.spatial_grid.insert(i as u32, ent.es.position, half);
        }

        let mut collision_map: HashMap<u64, Vec<String>> = HashMap::new();
        let pairs = self.spatial_grid.query_pairs();
        for (a_idx, b_idx) in pairs {
            let a = &self.entities[a_idx as usize];
            let b = &self.entities[b_idx as usize];
            if !a.alive || !b.alive { continue; }

            if overlap_test(a.es.position, &a.collider, b.es.position, &b.collider).is_some() {
                // A collides with B's tags
                for tag in &b.data.tags {
                    collision_map.entry(a.data.id).or_default().push(tag.clone());
                }
                // B collides with A's tags
                for tag in &a.data.tags {
                    collision_map.entry(b.data.id).or_default().push(tag.clone());
                }
            }
        }
        collision_map
    }

    /// Check if a position+half_extents overlaps any Solid entity.
    fn is_solid_at(&self, pos: Vec2, half: Vec2) -> bool {
        let test_collider = Collider::aabb(half.x, half.y);
        for ent in &self.entities {
            if !ent.alive || !has_solid_behavior(ent) { continue; }
            if overlap_test(pos, &test_collider, ent.es.position, &ent.collider).is_some() {
                return true;
            }
        }
        false
    }
}

impl Game for GameRunner {
    fn init(&mut self, ctx: &mut GameContext) {
        // Load white texture for rendering untextured entities
        let white_path = self.resolve("assets/white.png");
        if white_path.exists() {
            self.white_tex = Some(ctx.load_texture(&white_path));
        } else {
            // Create a 1x1 white texture programmatically
            self.white_tex = Some(ctx.create_texture_from_rgba(&[255, 255, 255, 255], 1, 1));
        }

        // Camera zoom for Retina
        ctx.camera.zoom = self.scene_settings.camera_zoom;

        // Load prefabs
        self.load_prefabs();

        // Load the entry scene
        let scene_path = self.resolve(&self.manifest.entry_scene);
        match toile_scene::load_scene(&scene_path) {
            Ok(scene) => {
                log::info!("Loaded scene '{}' with {} entities", scene.name, scene.entities.len());
                self.load_scene_data(&scene, ctx);
                // Initialize camera position
                self.camera_pos = Vec2::new(
                    self.scene_settings.camera_position[0],
                    self.scene_settings.camera_position[1],
                );
                log::info!("Scene camera: pos=({},{}), zoom={}",
                    self.scene_settings.camera_position[0],
                    self.scene_settings.camera_position[1],
                    self.scene_settings.camera_zoom);
            }
            Err(e) => log::error!("Failed to load scene {}: {e}", scene_path.display()),
        }
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        // Apply camera settings from scene — adapt zoom to window size to keep
        // the designed viewport visible regardless of actual window dimensions.
        let s = &self.scene_settings;
        let designed_w = s.viewport_width as f32 / s.camera_zoom;
        let designed_h = s.viewport_height as f32 / s.camera_zoom;
        let vp = ctx.camera.viewport_size(); // physical pixels
        if designed_w > 0.0 && designed_h > 0.0 {
            let zoom_w = vp.x / designed_w;
            let zoom_h = vp.y / designed_h;
            ctx.camera.zoom = zoom_w.min(zoom_h);
        }
        // Camera position depends on mode
        let half_vp_w = designed_w * 0.5;
        let half_vp_h = designed_h * 0.5;
        match &s.camera_mode {
            toile_scene::CameraMode::Fixed => {
                self.camera_pos = Vec2::new(s.camera_position[0], s.camera_position[1]);
            }
            toile_scene::CameraMode::FollowPlayer => {
                if let Some(player) = self.entities.iter().find(|e| e.alive && is_player(&e.data)) {
                    self.camera_pos = player.es.position;
                }
            }
            toile_scene::CameraMode::PlatformerFollow { deadzone_x, deadzone_y, bounds } => {
                if let Some(player) = self.entities.iter().find(|e| e.alive && is_player(&e.data)) {
                    let pp = player.es.position;
                    let dz_w = half_vp_w * deadzone_x;
                    let dz_h = half_vp_h * deadzone_y;

                    // Horizontal: only scroll when player leaves deadzone
                    if pp.x > self.camera_pos.x + dz_w {
                        self.camera_pos.x = pp.x - dz_w;
                    } else if pp.x < self.camera_pos.x - dz_w {
                        self.camera_pos.x = pp.x + dz_w;
                    }

                    // Vertical: only scroll when player leaves deadzone
                    if pp.y > self.camera_pos.y + dz_h {
                        self.camera_pos.y = pp.y - dz_h;
                    } else if pp.y < self.camera_pos.y - dz_h {
                        self.camera_pos.y = pp.y + dz_h;
                    }

                    // Clamp to scene bounds if set (non-zero)
                    if *bounds != [0.0, 0.0, 0.0, 0.0] {
                        let cam_min_x = bounds[0] + half_vp_w;
                        let cam_max_x = bounds[2] - half_vp_w;
                        let cam_min_y = bounds[1] + half_vp_h;
                        let cam_max_y = bounds[3] - half_vp_h;
                        if cam_min_x < cam_max_x {
                            self.camera_pos.x = self.camera_pos.x.clamp(cam_min_x, cam_max_x);
                        } else {
                            self.camera_pos.x = (bounds[0] + bounds[2]) * 0.5;
                        }
                        if cam_min_y < cam_max_y {
                            self.camera_pos.y = self.camera_pos.y.clamp(cam_min_y, cam_max_y);
                        } else {
                            self.camera_pos.y = (bounds[1] + bounds[3]) * 0.5;
                        }
                    }
                }
            }
        }
        ctx.camera.position = self.camera_pos;

        let dt_f = dt as f32;
        let input = Self::build_behavior_input(ctx);

        // ── 1. Update behaviors ──────────────────────────────────────────
        // We need &self for is_solid_at, but also &mut self.entities.
        // Pre-collect solid entity positions+colliders for the closure.
        let solids: Vec<(Collider, Vec2)> = self.entities.iter()
            .filter(|e| e.alive && has_solid_behavior(e))
            .map(|e| (e.collider, e.es.position))
            .collect();

        let solid_check = move |pos: Vec2, half: Vec2| -> bool {
            let test = Collider::aabb(half.x, half.y);
            solids.iter().any(|(c, p)| overlap_test(pos, &test, *p, c).is_some())
        };

        let camera_pos = ctx.camera.position;
        let view_half = ctx.camera.viewport_size() * 0.5 / ctx.camera.zoom;

        for ent in &mut self.entities {
            if !ent.alive { continue; }
            let is_player_ent = is_player(&ent.data);

            for beh in &mut ent.behaviors {
                match beh {
                    BehaviorRuntime::Platform { config, state } => {
                        if is_player_ent {
                            platform::update(config, state, &mut ent.es, &input, &solid_check, dt_f);
                        }
                    }
                    BehaviorRuntime::TopDown { config } => {
                        if is_player_ent {
                            topdown::update(config, &mut ent.es, &input, dt_f);
                        }
                    }
                    BehaviorRuntime::Bullet { config, state } => {
                        bullet::update(config, state, &mut ent.es, dt_f);
                    }
                    BehaviorRuntime::Sine { config, state } => {
                        sine::update(config, state, &mut ent.es, dt_f);
                    }
                    BehaviorRuntime::Fade { config, state } => {
                        fade::update(config, state, &mut ent.es, dt_f);
                    }
                    BehaviorRuntime::Wrap { config } => {
                        wrap::update(config, &mut ent.es, view_half, camera_pos);
                    }
                    BehaviorRuntime::Solid => {}
                }
            }

            // Clamp entity position to scene bounds (if set via PlatformerFollow)
            if let toile_scene::CameraMode::PlatformerFollow { bounds, .. } = &self.scene_settings.camera_mode {
                if *bounds != [0.0, 0.0, 0.0, 0.0] {
                    let hw = ent.es.size.x * 0.5;
                    let hh = ent.es.size.y * 0.5;
                    let min_x = bounds[0] + hw;
                    let max_x = bounds[2] - hw;
                    let min_y = bounds[1] + hh;
                    let max_y = bounds[3] - hh;
                    // Only clamp if bounds are valid (bigger than entity)
                    if min_x < max_x {
                        ent.es.position.x = ent.es.position.x.clamp(min_x, max_x);
                    }
                    if min_y < max_y {
                        ent.es.position.y = ent.es.position.y.clamp(min_y, max_y);
                    }
                }
            }

            // Sync entity state back to data
            ent.data.x = ent.es.position.x;
            ent.data.y = ent.es.position.y;
            ent.data.rotation = ent.es.rotation;
            ent.alive = ent.es.alive;
        }

        // ── 2. Collision detection ───────────────────────────────────────
        let collision_map = self.compute_collisions();

        // ── 3. Evaluate event sheets ─────────────────────────────────────
        let mut commands: Vec<EventCommand> = Vec::new();

        // We need key state closures from ctx.input
        let keys_down = |k: &str| -> bool {
            key_from_name(k).is_some_and(|kc| ctx.input.is_key_down(kc))
        };
        let keys_just_pressed = |k: &str| -> bool {
            key_from_name(k).is_some_and(|kc| ctx.input.is_key_just_pressed(kc))
        };
        let keys_just_released = |_k: &str| -> bool { false };
        let mouse_just_pressed = |b: &str| -> bool {
            match b {
                "Left" => ctx.input.is_mouse_just_pressed(MouseButton::Left),
                "Right" => ctx.input.is_mouse_just_pressed(MouseButton::Right),
                _ => false,
            }
        };

        for ent in &mut self.entities {
            if !ent.alive { continue; }
            if let Some(sheet) = &ent.event_sheet {
                let eid = ent.data.id;
                let tags_colliding = collision_map.get(&eid);
                let is_colliding_with = |tag: &str| -> bool {
                    tags_colliding.is_some_and(|tags| tags.iter().any(|t| t == tag))
                };

                let ectx = EventContext {
                    entity_id: eid,
                    entity_x: ent.es.position.x,
                    entity_y: ent.es.position.y,
                    dt,
                    keys_down: &keys_down,
                    keys_just_pressed: &keys_just_pressed,
                    keys_just_released: &keys_just_released,
                    mouse_just_pressed: &mouse_just_pressed,
                    is_colliding_with: &is_colliding_with,
                };

                let cmds = evaluate_event_sheet(sheet, &mut ent.event_state, &ectx);
                commands.extend(cmds);
            }
        }

        // ── 4. Apply event commands ──────────────────────────────────────
        let mut spawns: Vec<EntityData> = Vec::new();

        for cmd in &commands {
            match cmd {
                EventCommand::SetPosition { entity_id, x, y } => {
                    if let Some(ent) = self.entities.iter_mut().find(|e| e.data.id == *entity_id) {
                        ent.es.position = Vec2::new(*x, *y);
                    }
                }
                EventCommand::MoveAtAngle { entity_id, angle_deg, speed } => {
                    if let Some(ent) = self.entities.iter_mut().find(|e| e.data.id == *entity_id) {
                        let rad = angle_deg.to_radians();
                        ent.es.velocity = Vec2::new(rad.cos(), rad.sin()) * *speed;
                    }
                }
                EventCommand::Destroy { entity_id } => {
                    if let Some(ent) = self.entities.iter_mut().find(|e| e.data.id == *entity_id) {
                        ent.alive = false;
                        ent.es.alive = false;
                    }
                }
                EventCommand::SpawnObject { prefab, x, y } => {
                    if let Some(p) = self.prefabs.get(prefab) {
                        let id = self.next_id;
                        self.next_id += 1;
                        let mut overrides = HashMap::new();
                        overrides.insert("x".into(), serde_json::json!(*x));
                        overrides.insert("y".into(), serde_json::json!(*y));
                        let edata = p.instantiate(id, &overrides);
                        spawns.push(edata);
                    }
                }
                EventCommand::GoToScene { scene } => {
                    self.pending_scene = Some(scene.clone());
                }
                EventCommand::PlaySound { sound } => {
                    let path = self.resolve(sound);
                    if let Ok(sid) = ctx.audio.load_sound(&path) {
                        let _ = ctx.audio.play_sound(sid);
                    }
                }
                EventCommand::SetVariable { entity_id, name, value } => {
                    if let Some(ent) = self.entities.iter_mut().find(|e| e.data.id == *entity_id) {
                        ent.event_state.variables.insert(name.clone(), *value);
                    }
                }
                EventCommand::Log { message } => {
                    log::info!("[Game] {message}");
                }
                _ => {} // MoveToward, PlayAnimation — not yet implemented
            }
        }

        // Spawn new entities
        for edata in &spawns {
            let rt = self.spawn_entity(edata, ctx);
            self.entities.push(rt);
        }

        // ── 5. Update particles ──────────────────────────────────────────
        for ent in &mut self.entities {
            if !ent.alive { continue; }
            if let Some(pool) = &mut ent.particle_pool {
                pool.position = ent.es.position;
                pool.update(dt_f);
            }
        }

        // ── 6. Remove dead entities ──────────────────────────────────────
        self.entities.retain(|e| e.alive);

        // ── 7. Scene transition ──────────────────────────────────────────
        if let Some(scene_path) = self.pending_scene.take() {
            let full = self.resolve(&scene_path);
            match toile_scene::load_scene(&full) {
                Ok(scene) => {
                    log::info!("Transition to scene '{}'", scene.name);
                    self.load_scene_data(&scene, ctx);
                    ctx.camera.zoom = self.scene_settings.camera_zoom;
                }
                Err(e) => log::error!("Failed to load scene {scene_path}: {e}"),
            }
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let fallback_tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        // ── Apply lighting from scene settings + entity lights ───────────
        let ls = &self.scene_settings.lighting;
        if ls.enabled {
            ctx.lighting.enabled = true;
            ctx.lighting.ambient = ls.ambient;
            ctx.lighting.shadow.enabled = ls.shadows_enabled;
            ctx.lighting.lights.clear();
            for ent in &self.entities {
                if !ent.alive { continue; }
                if let Some(ref light) = ent.data.light {
                    ctx.lighting.lights.push(toile_app::Light {
                        position: ent.es.position,
                        radius: light.radius,
                        falloff: light.falloff,
                        color: light.color,
                        intensity: light.intensity,
                        cast_shadow: light.cast_shadow,
                    });
                }
            }
        }

        // ── Apply post-processing from scene settings ────────────────────
        if !self.scene_settings.post_effects.is_empty() {
            ctx.post_processing.enabled = true;
            ctx.post_processing.effects.clear();
            for fx in &self.scene_settings.post_effects {
                let effect = match fx {
                    toile_scene::PostEffectData::Vignette { intensity, smoothness } =>
                        toile_app::PostEffect::Vignette { intensity: *intensity, smoothness: *smoothness },
                    toile_scene::PostEffectData::Crt { scanline_intensity, curvature, chromatic_aberration } =>
                        toile_app::PostEffect::Crt { scanline_intensity: *scanline_intensity, curvature: *curvature, chromatic_aberration: *chromatic_aberration },
                    toile_scene::PostEffectData::Pixelate { pixel_size } =>
                        toile_app::PostEffect::Pixelate { pixel_size: *pixel_size },
                    toile_scene::PostEffectData::Bloom { threshold, intensity, radius } =>
                        toile_app::PostEffect::Bloom { threshold: *threshold, intensity: *intensity, radius: *radius },
                    toile_scene::PostEffectData::ColorGrading { saturation, brightness, contrast } =>
                        toile_app::PostEffect::ColorGrading { saturation: *saturation, brightness: *brightness, contrast: *contrast },
                };
                ctx.post_processing.effects.push(effect);
            }
        }

        // Render background tiles
        if let Some(bg_tex) = self.background_tex {
            let s = &self.scene_settings;
            let tile_w = s.viewport_width as f32 / s.camera_zoom;
            let tile_h = s.viewport_height as f32 / s.camera_zoom;
            let positions = if s.background_tiles.is_empty() {
                vec![s.camera_position]
            } else {
                s.background_tiles.clone()
            };
            for pos in &positions {
                ctx.draw_sprite(DrawSprite {
                    texture: bg_tex,
                    position: Vec2::new(pos[0], pos[1]),
                    size: Vec2::new(tile_w, tile_h),
                    rotation: 0.0,
                    color: 0xFFFFFFFF,
                    layer: -100,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }

        for ent in &self.entities {
            if !ent.alive || !ent.data.visible { continue; }

            let tex = ent.texture.unwrap_or(fallback_tex);
            let alpha = (ent.es.opacity.clamp(0.0, 1.0) * 255.0) as u8;

            // Tint untextured entities with a color based on layer
            let color = if ent.texture.is_some() {
                u32::from_be_bytes([255, 255, 255, alpha])
            } else {
                // Distinct colors per layer for visibility
                let hue = ((ent.data.layer.abs() as f32 * 0.3) % 1.0 * 6.0) as u8;
                let (r, g, b) = match hue % 6 {
                    0 => (100u8, 150, 220),
                    1 => (220, 100, 100),
                    2 => (100, 220, 100),
                    3 => (220, 220, 100),
                    4 => (220, 100, 220),
                    _ => (100, 220, 220),
                };
                u32::from_be_bytes([r, g, b, alpha])
            };

            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: ent.es.position,
                size: ent.es.size,
                rotation: ent.es.rotation,
                color,
                layer: ent.data.layer,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });

            // Draw particles
            if let Some(pool) = &ent.particle_pool {
                for (pos, size, rot, pcolor) in pool.render_data() {
                    ctx.draw_sprite(DrawSprite {
                        texture: fallback_tex,
                        position: pos,
                        size: Vec2::splat(size),
                        rotation: rot,
                        color: pcolor,
                        layer: ent.data.layer + 1,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }
            }
        }
    }
}

/// Launch a Toile project as a playable game.
pub fn run_project(project_dir: &Path) -> Result<(), String> {
    let runner = GameRunner::load(project_dir)?;
    let m = runner.manifest().clone();

    App::new()
        .with_title(&m.window_title)
        .with_size(m.window_width, m.window_height)
        .with_clear_color(Color::new(0.08, 0.08, 0.12, 1.0))
        .run(runner);

    Ok(())
}

// ── Key name → KeyCode mapping ──────────────────────────────────────────

fn key_from_name(name: &str) -> Option<Key> {
    match name {
        "ArrowLeft" | "Left" => Some(Key::ArrowLeft),
        "ArrowRight" | "Right" => Some(Key::ArrowRight),
        "ArrowUp" | "Up" => Some(Key::ArrowUp),
        "ArrowDown" | "Down" => Some(Key::ArrowDown),
        "Space" => Some(Key::Space),
        "Enter" | "Return" => Some(Key::Enter),
        "Escape" | "Esc" => Some(Key::Escape),
        "ShiftLeft" | "Shift" => Some(Key::ShiftLeft),
        "ControlLeft" | "Control" | "Ctrl" => Some(Key::ControlLeft),
        "KeyA" | "A" | "a" => Some(Key::KeyA),
        "KeyB" | "B" | "b" => Some(Key::KeyB),
        "KeyC" | "C" | "c" => Some(Key::KeyC),
        "KeyD" | "D" | "d" => Some(Key::KeyD),
        "KeyE" | "E" | "e" => Some(Key::KeyE),
        "KeyF" | "F" | "f" => Some(Key::KeyF),
        "KeyG" | "G" | "g" => Some(Key::KeyG),
        "KeyH" | "H" | "h" => Some(Key::KeyH),
        "KeyI" | "I" | "i" => Some(Key::KeyI),
        "KeyJ" | "J" | "j" => Some(Key::KeyJ),
        "KeyK" | "K" | "k" => Some(Key::KeyK),
        "KeyL" | "L" | "l" => Some(Key::KeyL),
        "KeyM" | "M" | "m" => Some(Key::KeyM),
        "KeyN" | "N" | "n" => Some(Key::KeyN),
        "KeyO" | "O" | "o" => Some(Key::KeyO),
        "KeyP" | "P" | "p" => Some(Key::KeyP),
        "KeyQ" | "Q" | "q" => Some(Key::KeyQ),
        "KeyR" | "R" | "r" => Some(Key::KeyR),
        "KeyS" | "S" | "s" => Some(Key::KeyS),
        "KeyT" | "T" | "t" => Some(Key::KeyT),
        "KeyU" | "U" | "u" => Some(Key::KeyU),
        "KeyV" | "V" | "v" => Some(Key::KeyV),
        "KeyW" | "W" | "w" => Some(Key::KeyW),
        "KeyX" | "X" | "x" => Some(Key::KeyX),
        "KeyY" | "Y" | "y" => Some(Key::KeyY),
        "KeyZ" | "Z" | "z" => Some(Key::KeyZ),
        "Digit1" | "1" => Some(Key::Digit1),
        "Digit2" | "2" => Some(Key::Digit2),
        "Digit3" | "3" => Some(Key::Digit3),
        "Digit4" | "4" => Some(Key::Digit4),
        "Digit5" | "5" => Some(Key::Digit5),
        "Digit6" | "6" => Some(Key::Digit6),
        "Digit7" | "7" => Some(Key::Digit7),
        "Digit8" | "8" => Some(Key::Digit8),
        "Digit9" | "9" => Some(Key::Digit9),
        "Digit0" | "0" => Some(Key::Digit0),
        "Tab" => Some(Key::Tab),
        "Backspace" => Some(Key::Backspace),
        _ => None,
    }
}
