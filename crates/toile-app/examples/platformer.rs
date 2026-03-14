//! Toile Engine — Platformer Demo (Week 7-8)
//!
//! Showcases: tilemap (Tiled JSON), sprite animation (Aseprite),
//! Lua scripting (enemy AI), collision, audio, text, camera scrolling.
//!
//! Arrow keys / AD to move, Space to jump. F3 for debug.
//!
//! Run with: `cargo run --example platformer`

use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, Sprite, TextureHandle, COLOR_WHITE};
use toile_assets::animation::{self, AnimationFrame, PlaybackMode, SpriteSheet, SpriteSheetHandle};
use toile_assets::tilemap;
use toile_collision::{overlap_test, Collider};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};
use toile_scripting::vm::ScriptVm;

const GRAVITY: f32 = -800.0;
const PLAYER_SPEED: f32 = 200.0;
const JUMP_VEL: f32 = 400.0;
const PLAYER_HALF_W: f32 = 10.0;
const PLAYER_HALF_H: f32 = 14.0;

struct SolidRect {
    center: Vec2,
    half: Vec2,
}

struct EnemyData {
    pos: Vec2,
    vel_x: f32,
    patrol_left: f32,
    patrol_right: f32,
}

struct Platformer {
    // Assets
    player_tex: Option<TextureHandle>,
    player_sheet: Option<SpriteSheet>,
    enemy_tex: Option<TextureHandle>,
    font: Option<FontHandle>,

    // Tilemap
    tile_sprites: Vec<Vec<DrawSprite>>,
    solids: Vec<SolidRect>,
    map_width_px: f32,
    map_height_px: f32,

    // Player
    player_pos: Vec2,
    player_vel: Vec2,
    on_ground: bool,
    facing_right: bool,

    // Animation
    current_clip: String,
    current_frame: usize,
    anim_elapsed: f32,

    // Enemies
    enemies: Vec<EnemyData>,
    lua_vm: Option<ScriptVm>,

    // Audio
    sfx_jump: Option<toile_app::SoundId>,
}

impl Platformer {
    fn new() -> Self {
        Self {
            player_tex: None,
            player_sheet: None,
            enemy_tex: None,
            font: None,
            tile_sprites: Vec::new(),
            solids: Vec::new(),
            map_width_px: 0.0,
            map_height_px: 0.0,
            player_pos: Vec2::ZERO,
            player_vel: Vec2::ZERO,
            on_ground: false,
            facing_right: true,
            current_clip: "idle".into(),
            current_frame: 0,
            anim_elapsed: 0.0,
            enemies: Vec::new(),
            lua_vm: None,
            sfx_jump: None,
        }
    }

    fn set_animation(&mut self, clip: &str) {
        if self.current_clip != clip {
            self.current_clip = clip.to_string();
            self.current_frame = 0;
            self.anim_elapsed = 0.0;
        }
    }

    fn advance_animation(&mut self, dt: f32) {
        let Some(sheet) = &self.player_sheet else {
            return;
        };
        let Some(clip) = sheet.clips.get(&self.current_clip) else {
            return;
        };
        if clip.frames.is_empty() {
            return;
        }

        self.anim_elapsed += dt;
        let frame_dur = clip.frames[self.current_frame].duration;
        while self.anim_elapsed >= frame_dur {
            self.anim_elapsed -= frame_dur;
            match clip.mode {
                PlaybackMode::Loop => {
                    self.current_frame = (self.current_frame + 1) % clip.frames.len();
                }
                PlaybackMode::Once => {
                    if self.current_frame + 1 < clip.frames.len() {
                        self.current_frame += 1;
                    }
                }
                PlaybackMode::PingPong => {
                    self.current_frame = (self.current_frame + 1) % clip.frames.len();
                }
            }
        }
    }

    fn current_anim_frame(&self) -> Option<&AnimationFrame> {
        let sheet = self.player_sheet.as_ref()?;
        let clip = sheet.clips.get(&self.current_clip)?;
        clip.frames.get(self.current_frame)
    }
}

impl Game for Platformer {
    fn init(&mut self, ctx: &mut GameContext) {
        let base = Path::new("assets/platformer");

        // Load player sprite sheet
        let player_tex = ctx.load_texture(&base.join("player.png"));
        let player_json = std::fs::read_to_string(base.join("player.json"))
            .expect("Failed to read player.json");
        let player_sheet = animation::load_aseprite_json(&player_json, player_tex);
        self.player_tex = Some(player_tex);
        self.player_sheet = Some(player_sheet);

        // Load enemy texture
        self.enemy_tex = Some(ctx.load_texture(&base.join("enemy.png")));

        // Load font
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));

        // Load tilemap
        let tmap = tilemap::load_tiled_json(&base.join("level.json"), &mut |img_path| {
            ctx.load_texture(img_path)
        });

        self.map_width_px = (tmap.width * tmap.tile_width) as f32;
        self.map_height_px = (tmap.height * tmap.tile_height) as f32;

        // Build tile collision rects from solid tiles (GID 1 = earth, GID 2 = grass, GID 3 = platform)
        for tile_layer in &tmap.tile_layers {
            for row in 0..tile_layer.height {
                for col in 0..tile_layer.width {
                    let gid = tile_layer.gids[(row * tile_layer.width + col) as usize] & 0x1FFFFFFF;
                    if gid == 0 {
                        continue;
                    }
                    let tw = tmap.tile_width as f32;
                    let th = tmap.tile_height as f32;
                    let x = col as f32 * tw + tw * 0.5;
                    let y = self.map_height_px - (row as f32 * th + th * 0.5);
                    self.solids.push(SolidRect {
                        center: Vec2::new(x, y),
                        half: Vec2::new(tw * 0.5, th * 0.5),
                    });
                }
            }
        }

        // Build tile sprites for rendering
        self.tile_sprites = tilemap::build_tile_sprites(&tmap, 0);

        // Parse object layer for spawn points and enemies
        for obj_layer in &tmap.object_layers {
            for obj in &obj_layer.objects {
                let pos = tmap.tiled_to_engine(obj.x, obj.y, obj.width, obj.height);
                match obj.obj_type.as_str() {
                    "spawn" => {
                        self.player_pos = pos;
                    }
                    "enemy" => {
                        let pl = obj
                            .properties
                            .get("patrol_left")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(obj.x as f64) as f32;
                        let pr = obj
                            .properties
                            .get("patrol_right")
                            .and_then(|v| v.as_f64())
                            .unwrap_or((obj.x + 200.0) as f64) as f32;
                        self.enemies.push(EnemyData {
                            pos,
                            vel_x: 60.0,
                            patrol_left: pl,
                            patrol_right: pr,
                        });
                    }
                    _ => {}
                }
            }
        }

        // Lua VM for enemy scripts
        let mut vm = ScriptVm::new().expect("Failed to create Lua VM");
        let script_path = base.join("scripts/enemy_patrol.lua");
        if let Err(e) = vm.load_script(&script_path) {
            log::error!("Failed to load enemy script: {e}");
        }
        self.lua_vm = Some(vm);

        // Audio
        self.sfx_jump = Some(
            ctx.audio
                .load_sound(Path::new("assets/bounce.wav"))
                .expect("Failed to load jump SFX"),
        );

        // Music
        let music = ctx
            .audio
            .load_sound(Path::new("assets/music_test.wav"))
            .expect("Failed to load music");
        let pb = ctx
            .audio
            .play_sound_looped(music)
            .expect("Failed to play music");
        ctx.audio.set_volume(pb, 0.1);

        log::info!("Platformer loaded! Arrows/AD=move, Space=jump, F3=debug");
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;

        // --- Player input ---
        let mut move_x = 0.0f32;
        if ctx.input.is_key_down(Key::ArrowRight) || ctx.input.is_key_down(Key::KeyD) {
            move_x += 1.0;
            self.facing_right = true;
        }
        if ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA) {
            move_x -= 1.0;
            self.facing_right = false;
        }

        self.player_vel.x = move_x * PLAYER_SPEED;

        if ctx.first_tick && ctx.input.is_key_just_pressed(Key::Space) && self.on_ground {
            self.player_vel.y = JUMP_VEL;
            self.on_ground = false;
            if let Some(sfx) = self.sfx_jump {
                let _ = ctx.audio.play_sound(sfx);
            }
        }

        // --- Gravity ---
        self.player_vel.y += GRAVITY * dt;

        // --- Move and collide ---
        // Move X
        self.player_pos.x += self.player_vel.x * dt;
        let player_col = Collider::aabb(PLAYER_HALF_W, PLAYER_HALF_H);
        for solid in &self.solids {
            let solid_col = Collider::aabb(solid.half.x, solid.half.y);
            if let Some(mtv) = overlap_test(self.player_pos, &player_col, solid.center, &solid_col)
            {
                self.player_pos.x += mtv.x;
                if mtv.x != 0.0 {
                    self.player_vel.x = 0.0;
                }
            }
        }

        // Move Y
        self.player_pos.y += self.player_vel.y * dt;
        self.on_ground = false;
        for solid in &self.solids {
            let solid_col = Collider::aabb(solid.half.x, solid.half.y);
            if let Some(mtv) = overlap_test(self.player_pos, &player_col, solid.center, &solid_col)
            {
                self.player_pos.y += mtv.y;
                if mtv.y > 0.0 {
                    self.on_ground = true;
                    self.player_vel.y = 0.0;
                } else if mtv.y < 0.0 {
                    self.player_vel.y = 0.0;
                }
            }
        }

        // Fall off map
        if self.player_pos.y < -100.0 {
            self.player_pos = Vec2::new(80.0, self.map_height_px - 100.0);
            self.player_vel = Vec2::ZERO;
        }

        // --- Animation ---
        if !self.on_ground {
            self.set_animation("jump");
        } else if move_x.abs() > 0.0 {
            self.set_animation("run");
        } else {
            self.set_animation("idle");
        }
        self.advance_animation(dt);

        // --- Enemy Lua update ---
        if let Some(vm) = &self.lua_vm {
            let script_path = Path::new("assets/platformer/scripts/enemy_patrol.lua");
            for (i, enemy) in self.enemies.iter_mut().enumerate() {
                // Set globals for this enemy
                let lua = vm.lua();
                let _ = lua.globals().set("pos_x", enemy.pos.x as f64);
                let _ = lua.globals().set("pos_y", enemy.pos.y as f64);
                let _ = lua.globals().set("patrol_left", enemy.patrol_left as f64);
                let _ = lua.globals().set("patrol_right", enemy.patrol_right as f64);
                let _ = lua.globals().set("vel_x", enemy.vel_x as f64);

                let _ = vm.call_on_update(script_path, i as u64, dt as f64);

                // Read back velocity
                if let Ok(vx) = lua.globals().get::<f64>("vel_x") {
                    enemy.vel_x = vx as f32;
                }
                enemy.pos.x += enemy.vel_x * dt;
            }
        }

        // --- Camera follow ---
        let half_view_w = 400.0; // half of 800 window
        let half_view_h = 240.0;
        let target_x = self
            .player_pos
            .x
            .clamp(half_view_w, self.map_width_px - half_view_w);
        let target_y = self
            .player_pos
            .y
            .clamp(half_view_h, self.map_height_px - half_view_h);
        let target = Vec2::new(target_x, target_y);
        ctx.camera.position = ctx.camera.position + (target - ctx.camera.position) * 8.0 * dt;
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        // Draw tilemap layers
        for layer in &self.tile_sprites {
            for sprite in layer {
                ctx.draw_sprite(sprite.clone());
            }
        }

        // Draw enemies
        if let Some(enemy_tex) = self.enemy_tex {
            for enemy in &self.enemies {
                ctx.draw_sprite(DrawSprite {
                    texture: enemy_tex,
                    position: enemy.pos,
                    size: Vec2::new(32.0, 32.0),
                    rotation: 0.0,
                    color: COLOR_WHITE,
                    layer: 5,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }

        // Draw player (animated)
        if let (Some(tex), Some(frame)) = (self.player_tex, self.current_anim_frame()) {
            let mut uv_min = frame.uv_min;
            let mut uv_max = frame.uv_max;
            if !self.facing_right {
                std::mem::swap(&mut uv_min.x, &mut uv_max.x);
            }
            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: self.player_pos,
                size: Vec2::new(32.0, 32.0),
                rotation: 0.0,
                color: COLOR_WHITE,
                layer: 10,
                uv_min,
                uv_max,
            });
        }

        // HUD
        if let Some(font) = self.font {
            ctx.draw_text(
                "PLATFORMER DEMO",
                ctx.camera.position + Vec2::new(-380.0, 210.0),
                font,
                16.0,
                COLOR_WHITE,
                20,
            );
            ctx.draw_text(
                "Arrows/AD: Move  Space: Jump",
                ctx.camera.position + Vec2::new(-380.0, 190.0),
                font,
                10.0,
                pack_color(180, 180, 180, 255),
                20,
            );
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Platformer Demo")
        .with_size(800, 480)
        .with_clear_color(Color::rgb(0.2, 0.3, 0.5))
        .run(Platformer::new());
}
