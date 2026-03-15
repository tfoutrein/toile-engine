//! Toile Engine — Prefab Demo (v0.3)
//!
//! Demonstrates the prefab system combined with live gameplay:
//! place entities from prefab templates, then play as a character
//! that interacts with them.
//!
//! Controls:
//!   Tab: toggle between EDIT mode (place prefabs) and PLAY mode
//!   --- EDIT mode ---
//!   Click: instantiate the selected prefab at cursor position
//!   1/2/3: select prefab type (Enemy, Coin, Platform)
//!   R: reset (clear all instances)
//!   S: save current scene to prefab_demo.json
//!   --- PLAY mode ---
//!   Left/Right arrows: move player
//!   Space: jump
//!   Click: shoot toward cursor
//!
//! Run with: `cargo run --example prefab_demo`

use std::collections::HashMap;
use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, MouseButton as MB, TextureHandle, COLOR_WHITE};
use toile_behaviors::bullet::{self, BulletConfig, BulletState};
use toile_behaviors::fade::{self, FadeConfig, FadeState};
use toile_behaviors::platform::{self, PlatformConfig, PlatformState};
use toile_behaviors::types::{BehaviorInput, EntityState};
use toile_collision::{overlap_test, Collider};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};
use toile_scene::prefab::Prefab;
use toile_scene::{EntityData, SceneData};

// --- Game entities ---

struct Player {
    state: EntityState,
    config: PlatformConfig,
    pstate: PlatformState,
}

struct Projectile {
    state: EntityState,
    bullet_config: BulletConfig,
    bullet_state: BulletState,
    alive: bool,
}

struct FadingEnemy {
    index: usize, // index into scene.entities
    fade_config: FadeConfig,
    fade_state: FadeState,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Mode {
    Edit,
    Play,
}

struct PrefabDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    prefabs: Vec<Prefab>,
    selected_prefab: usize,
    scene: SceneData,
    status: String,
    mode: Mode,
    player: Player,
    projectiles: Vec<Projectile>,
    fading: Vec<FadingEnemy>,
    score: u32,
}

impl PrefabDemo {
    fn create_prefabs() -> Vec<Prefab> {
        vec![
            Prefab::from_entity("Enemy", &EntityData {
                id: 0, name: "Enemy".into(),
                x: 0.0, y: 0.0, rotation: 0.0,
                scale_x: 1.0, scale_y: 1.0, layer: 0,
                sprite_path: String::new(),
                width: 28.0, height: 28.0,
            }),
            Prefab::from_entity("Coin", &EntityData {
                id: 0, name: "Coin".into(),
                x: 0.0, y: 0.0, rotation: 0.0,
                scale_x: 1.0, scale_y: 1.0, layer: 0,
                sprite_path: String::new(),
                width: 16.0, height: 16.0,
            }),
            Prefab::from_entity("Platform", &EntityData {
                id: 0, name: "Platform".into(),
                x: 0.0, y: 0.0, rotation: 0.0,
                scale_x: 1.0, scale_y: 1.0, layer: -1,
                sprite_path: String::new(),
                width: 120.0, height: 16.0,
            }),
        ]
    }

    fn color_for_prefab(name: &str) -> u32 {
        match name {
            "Enemy" => pack_color(220, 60, 60, 255),
            "Coin" => pack_color(255, 220, 50, 255),
            "Platform" => pack_color(100, 120, 160, 255),
            _ => pack_color(150, 150, 150, 255),
        }
    }

    fn prefab_name_for(entity: &EntityData) -> &str {
        if entity.name.starts_with("Enemy") { "Enemy" }
        else if entity.name.starts_with("Coin") { "Coin" }
        else if entity.name.starts_with("Platform") { "Platform" }
        else { "Unknown" }
    }

}

impl Game for PrefabDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));
        self.prefabs = Self::create_prefabs();

        // Save prefabs to disk for MCP access
        let prefab_dir = Path::new("prefabs");
        let _ = std::fs::create_dir_all(prefab_dir);
        for prefab in &self.prefabs {
            let path = prefab_dir.join(format!("{}.prefab.json", prefab.name));
            let _ = toile_scene::prefab::save_prefab(&path, prefab);
        }

        self.status = "EDIT MODE — Click to place. 1=Enemy 2=Coin 3=Platform".to_string();
        log::info!("Prefab Demo! Tab=toggle mode, Click=place/shoot, Arrows+Space=move/jump");
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;

        // Toggle mode
        if ctx.input.is_key_just_pressed(Key::Tab) {
            self.mode = match self.mode {
                Mode::Edit => {
                    // Reset player position above ground
                    self.player.state.position = Vec2::new(0.0, -150.0);
                    self.player.state.velocity = Vec2::ZERO;
                    self.player.pstate = PlatformState::default();
                    self.projectiles.clear();
                    self.fading.clear();
                    self.score = 0;
                    self.status = "PLAY MODE — Arrows=Move Space=Jump Click=Shoot".to_string();
                    Mode::Play
                }
                Mode::Play => {
                    self.projectiles.clear();
                    self.fading.clear();
                    self.status = "EDIT MODE — Click to place. 1=Enemy 2=Coin 3=Platform".to_string();
                    Mode::Edit
                }
            };
        }

        match self.mode {
            Mode::Edit => self.update_edit(ctx),
            Mode::Play => self.update_play(ctx, dt),
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        // Ground
        ctx.draw_sprite(DrawSprite {
            texture: tex,
            position: Vec2::new(0.0, -200.0),
            size: Vec2::new(1000.0, 30.0),
            rotation: 0.0,
            color: pack_color(80, 120, 80, 255),
            layer: -2,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });

        // Draw all scene entities
        for (i, entity) in self.scene.entities.iter().enumerate() {
            let base_name = Self::prefab_name_for(entity);
            let base_color = Self::color_for_prefab(base_name);

            let color = if let Some(fe) = self.fading.iter().find(|f| f.index == i) {
                let alpha = ((1.0 - fe.fade_state.elapsed / fe.fade_config.fade_out_time.max(0.01)).max(0.0) * 255.0) as u8;
                base_color & 0x00FFFFFF | ((alpha as u32) << 24)
            } else {
                base_color
            };

            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: Vec2::new(entity.x, entity.y),
                size: Vec2::new(entity.width * entity.scale_x, entity.height * entity.scale_y),
                rotation: entity.rotation,
                color,
                layer: entity.layer,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // Ghost preview in edit mode
        if self.mode == Mode::Edit {
            let mouse_world = ctx.camera.screen_to_world(ctx.input.mouse_position());
            let prefab = &self.prefabs[self.selected_prefab];
            let ghost_color = Self::color_for_prefab(&prefab.name) & 0x80FFFFFF;
            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: mouse_world,
                size: Vec2::new(prefab.entity.width, prefab.entity.height),
                rotation: 0.0,
                color: ghost_color,
                layer: 100,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // Player (play mode only)
        if self.mode == Mode::Play {
            let p = &self.player.state;
            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: p.position,
                size: Vec2::new(24.0, 32.0),
                rotation: 0.0,
                color: pack_color(80, 150, 230, 255),
                layer: 5,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });

            // Projectiles
            for proj in &self.projectiles {
                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: proj.state.position,
                    size: proj.state.size,
                    rotation: proj.bullet_config.angle_degrees.to_radians(),
                    color: pack_color(255, 255, 100, 255),
                    layer: 4,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }

        // HUD
        if let Some(font) = self.font {
            let tl = ctx.camera.top_left();
            let mode_str = match self.mode {
                Mode::Edit => "EDIT",
                Mode::Play => "PLAY",
            };
            ctx.draw_text(
                &format!("Prefab Demo [{}] | Entities: {} | Score: {}",
                    mode_str, self.scene.entities.len(), self.score),
                Vec2::new(tl.x + 10.0, tl.y - 20.0),
                font, 10.0, COLOR_WHITE, 50,
            );
            let help = match self.mode {
                Mode::Edit => "1:Enemy 2:Coin 3:Platform | Click=Place R=Reset S=Save | Tab=Play",
                Mode::Play => "Arrows=Move Space=Jump Click=Shoot | Tab=Edit",
            };
            ctx.draw_text(
                help,
                Vec2::new(tl.x + 10.0, tl.y - 38.0),
                font, 6.0, pack_color(150, 150, 170, 255), 50,
            );
            ctx.draw_text(
                &self.status,
                Vec2::new(tl.x + 10.0, tl.y - 54.0),
                font, 6.0, pack_color(100, 200, 100, 255), 50,
            );
        }
    }
}

impl PrefabDemo {
    fn update_edit(&mut self, ctx: &mut GameContext) {
        // Select prefab
        if ctx.input.is_key_just_pressed(Key::Digit1) {
            self.selected_prefab = 0;
            self.status = "Selected: Enemy".to_string();
        }
        if ctx.input.is_key_just_pressed(Key::Digit2) {
            self.selected_prefab = 1;
            self.status = "Selected: Coin".to_string();
        }
        if ctx.input.is_key_just_pressed(Key::Digit3) {
            self.selected_prefab = 2;
            self.status = "Selected: Platform".to_string();
        }

        // Place instance on click
        if ctx.input.is_mouse_just_pressed(MB::Left) {
            let world_pos = ctx.camera.screen_to_world(ctx.input.mouse_position());
            let prefab = &self.prefabs[self.selected_prefab];
            let mut overrides = HashMap::new();
            overrides.insert("x".into(), serde_json::json!(world_pos.x));
            overrides.insert("y".into(), serde_json::json!(world_pos.y));

            let id = self.scene.next_id;
            self.scene.next_id += 1;
            let instance = prefab.instantiate(id, &overrides);
            self.status = format!("Placed {} at ({:.0}, {:.0})", prefab.name, world_pos.x, world_pos.y);
            self.scene.entities.push(instance);
        }

        // Reset
        if ctx.input.is_key_just_pressed(Key::KeyR) {
            self.scene.entities.clear();
            self.scene.next_id = 1;
            self.status = "Reset! All instances cleared.".to_string();
        }

        // Save scene
        if ctx.input.is_key_just_pressed(Key::KeyS) {
            let json = serde_json::to_string_pretty(&self.scene).unwrap();
            let _ = std::fs::write("prefab_demo.json", &json);
            self.status = format!("Saved prefab_demo.json ({} entities)", self.scene.entities.len());
        }
    }

    fn update_play(&mut self, ctx: &mut GameContext, dt: f32) {
        // Build input
        let input = BehaviorInput {
            left: ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA),
            right: ctx.input.is_key_down(Key::ArrowRight) || ctx.input.is_key_down(Key::KeyD),
            up: false,
            down: false,
            jump_pressed: ctx.input.is_key_just_pressed(Key::Space),
            jump_down: ctx.input.is_key_down(Key::Space),
        };

        // Collect solid geometry into local vecs to avoid borrowing self in closure
        // Ground: center at (0, -200), half-extents (500, 15)
        let ground_center = Vec2::new(0.0, -200.0);
        let ground_half = Vec2::new(500.0, 15.0);

        let platform_solids: Vec<(Vec2, Vec2)> = self.scene.entities.iter()
            .filter(|e| Self::prefab_name_for(e) == "Platform")
            .map(|e| (
                Vec2::new(e.x, e.y),
                Vec2::new(e.width * e.scale_x * 0.5, e.height * e.scale_y * 0.5),
            ))
            .collect();

        let solid_check = move |pos: Vec2, half: Vec2| -> bool {
            let col = Collider::aabb(half.x, half.y);
            // Ground
            let gc = Collider::aabb(ground_half.x, ground_half.y);
            if overlap_test(pos, &col, ground_center, &gc).is_some() {
                return true;
            }
            // Platform prefab instances
            for (center, ph) in &platform_solids {
                let pc = Collider::aabb(ph.x, ph.y);
                if overlap_test(pos, &col, *center, &pc).is_some() {
                    return true;
                }
            }
            false
        };

        platform::update(
            &self.player.config,
            &mut self.player.pstate,
            &mut self.player.state,
            &input,
            &solid_check,
            dt,
        );

        // Shoot on click
        if ctx.input.is_mouse_just_pressed(MB::Left) {
            let mouse_world = ctx.camera.screen_to_world(ctx.input.mouse_position());
            let dir = (mouse_world - self.player.state.position).normalize_or_zero();
            let angle = dir.y.atan2(dir.x).to_degrees();

            self.projectiles.push(Projectile {
                state: EntityState {
                    position: self.player.state.position,
                    velocity: Vec2::ZERO, rotation: 0.0, on_ground: false,
                    size: Vec2::new(8.0, 4.0), opacity: 1.0, alive: true,
                },
                bullet_config: BulletConfig { speed: 400.0, angle_degrees: angle, ..Default::default() },
                bullet_state: BulletState::default(),
                alive: true,
            });
        }

        // Update projectiles
        for proj in &mut self.projectiles {
            if proj.alive {
                bullet::update(&proj.bullet_config, &mut proj.bullet_state, &mut proj.state, dt);
                if proj.state.position.length() > 800.0 {
                    proj.alive = false;
                }
            }
        }

        // Projectile-enemy collision: collect hits first, then apply
        let mut enemy_hits: Vec<(usize, usize)> = Vec::new(); // (entity_idx, proj_idx)
        for (i, entity) in self.scene.entities.iter().enumerate() {
            if Self::prefab_name_for(entity) != "Enemy" { continue; }
            if self.fading.iter().any(|f| f.index == i) { continue; }

            let ec = Collider::aabb(
                entity.width * entity.scale_x * 0.5,
                entity.height * entity.scale_y * 0.5,
            );
            let epos = Vec2::new(entity.x, entity.y);

            for (pi, proj) in self.projectiles.iter().enumerate() {
                if !proj.alive { continue; }
                let pc = Collider::aabb(proj.state.size.x * 0.5, proj.state.size.y * 0.5);
                if overlap_test(epos, &ec, proj.state.position, &pc).is_some() {
                    enemy_hits.push((i, pi));
                    break;
                }
            }
        }
        for (ei, pi) in &enemy_hits {
            self.fading.push(FadingEnemy {
                index: *ei,
                fade_config: FadeConfig { fade_in_time: 0.0, fade_out_time: 0.4, destroy_on_fade_out: true },
                fade_state: {
                    let mut fs = FadeState::default();
                    fade::start_fade_out(&mut fs);
                    fs
                },
            });
            self.projectiles[*pi].alive = false;
            self.score += 100;
        }
        if !enemy_hits.is_empty() {
            self.status = format!("Enemy hit! +100 (Score: {})", self.score);
        }

        // Player-coin collision
        let player_col = Collider::aabb(12.0, 16.0);
        let player_pos = self.player.state.position;
        let collected: Vec<usize> = self.scene.entities.iter().enumerate()
            .filter(|(i, e)| {
                Self::prefab_name_for(e) == "Coin"
                    && !self.fading.iter().any(|f| f.index == *i)
            })
            .filter(|(_, e)| {
                let cc = Collider::aabb(
                    e.width * e.scale_x * 0.5,
                    e.height * e.scale_y * 0.5,
                );
                overlap_test(player_pos, &player_col, Vec2::new(e.x, e.y), &cc).is_some()
            })
            .map(|(i, _)| i)
            .collect();

        for i in &collected {
            self.fading.push(FadingEnemy {
                index: *i,
                fade_config: FadeConfig { fade_in_time: 0.0, fade_out_time: 0.3, destroy_on_fade_out: true },
                fade_state: {
                    let mut fs = FadeState::default();
                    fade::start_fade_out(&mut fs);
                    fs
                },
            });
            self.score += 50;
        }
        if !collected.is_empty() {
            self.status = format!("Coin collected! +50 (Score: {})", self.score);
        }

        // Update fading entities
        for fe in &mut self.fading {
            let mut dummy = EntityState {
                position: Vec2::ZERO, velocity: Vec2::ZERO, rotation: 0.0,
                on_ground: false, size: Vec2::ZERO, opacity: 1.0, alive: true,
            };
            fade::update(&fe.fade_config, &mut fe.fade_state, &mut dummy, dt);
        }

        // Remove fully faded entities (iterate in reverse to keep indices valid)
        let done_indices: Vec<usize> = self.fading.iter()
            .filter(|f| f.fade_state.phase == fade::FadePhase::Done)
            .map(|f| f.index)
            .collect();
        self.fading.retain(|f| f.fade_state.phase != fade::FadePhase::Done);

        // Remove from scene (reverse order to preserve indices)
        let mut sorted = done_indices;
        sorted.sort_unstable();
        sorted.dedup();
        for &idx in sorted.iter().rev() {
            if idx < self.scene.entities.len() {
                self.scene.entities.remove(idx);
                // Adjust fading indices
                for fe in &mut self.fading {
                    if fe.index > idx {
                        fe.index -= 1;
                    }
                }
            }
        }

        // Cleanup projectiles
        self.projectiles.retain(|p| p.alive);
    }
}

fn main() {
    App::new()
        .with_title("Toile — Prefab Demo (v0.3)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.1, 0.1, 0.15))
        .run(PrefabDemo {
            white_tex: None,
            font: None,
            prefabs: Vec::new(),
            selected_prefab: 0,
            scene: SceneData::new("prefab_demo"),
            status: String::new(),
            mode: Mode::Edit,
            player: Player {
                state: EntityState {
                    position: Vec2::new(0.0, -150.0),
                    velocity: Vec2::ZERO, rotation: 0.0, on_ground: false,
                    size: Vec2::new(24.0, 32.0), opacity: 1.0, alive: true,
                },
                config: PlatformConfig::default(),
                pstate: PlatformState::default(),
            },
            projectiles: Vec::new(),
            fading: Vec::new(),
            score: 0,
        });
}
