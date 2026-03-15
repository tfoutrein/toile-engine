//! Toile Engine — Behaviors Demo (v0.3)
//!
//! Showcases all 7 pre-built behaviors in one scene:
//! - Player with Platform behavior (arrows + space to jump)
//! - Floating platform with Sine behavior (bobs up and down)
//! - Projectile with Bullet behavior (click to shoot)
//! - Enemies that fade out when hit
//! - Wrapping stars in the background
//!
//! Run with: `cargo run --example behaviors_demo`

use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, MouseButton as MB, TextureHandle, COLOR_WHITE};
use toile_behaviors::bullet::{self, BulletConfig, BulletState};
use toile_behaviors::fade::{self, FadeConfig, FadeState};
use toile_behaviors::platform::{self, PlatformConfig, PlatformState};
use toile_behaviors::sine::{self, SineConfig, SineProperty, SineState};
use toile_behaviors::topdown::{self, TopDownConfig};
use toile_behaviors::wrap::{self, WrapConfig};
use toile_behaviors::types::{BehaviorInput, EntityState};
use toile_collision::{overlap_test, Collider};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};

// --- Entities ---

struct Player {
    state: EntityState,
    platform_config: PlatformConfig,
    platform_state: PlatformState,
}

struct FloatingPlatform {
    state: EntityState,
    sine_config: SineConfig,
    sine_state: SineState,
}

struct Projectile {
    state: EntityState,
    bullet_config: BulletConfig,
    bullet_state: BulletState,
    alive: bool,
}

struct Enemy {
    state: EntityState,
    fade_config: FadeConfig,
    fade_state: FadeState,
    hit: bool,
}

struct Star {
    state: EntityState,
    wrap_config: WrapConfig,
    vel: Vec2,
}

struct BehaviorsDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    player: Player,
    platforms: Vec<FloatingPlatform>,
    ground: Vec<(Vec2, Vec2)>, // (center, half_extents) for solid rects
    projectiles: Vec<Projectile>,
    enemies: Vec<Enemy>,
    stars: Vec<Star>,
    rng: u32,
}

impl BehaviorsDemo {
    fn rng_f32(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng as f64 / u32::MAX as f64) as f32
    }

    fn is_solid(&self, pos: Vec2, half: Vec2) -> bool {
        let col = Collider::aabb(half.x, half.y);
        for (center, gh) in &self.ground {
            let gc = Collider::aabb(gh.x, gh.y);
            if overlap_test(pos, &col, *center, &gc).is_some() {
                return true;
            }
        }
        // Check floating platforms as solids too
        for plat in &self.platforms {
            let pc = Collider::aabb(plat.state.size.x * 0.5, plat.state.size.y * 0.5);
            if overlap_test(pos, &col, plat.state.position, &pc).is_some() {
                return true;
            }
        }
        false
    }
}

impl Game for BehaviorsDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));

        // Ground
        self.ground = vec![
            (Vec2::new(0.0, -200.0), Vec2::new(500.0, 15.0)),
            (Vec2::new(-350.0, -100.0), Vec2::new(100.0, 10.0)),
            (Vec2::new(350.0, -50.0), Vec2::new(80.0, 10.0)),
        ];

        // Floating platforms with Sine
        self.platforms = vec![
            FloatingPlatform {
                state: EntityState {
                    position: Vec2::new(-100.0, -50.0),
                    velocity: Vec2::ZERO, rotation: 0.0, on_ground: false,
                    size: Vec2::new(100.0, 12.0), opacity: 1.0, alive: true,
                },
                sine_config: SineConfig { property: SineProperty::Y, magnitude: 30.0, period: 3.0 },
                sine_state: SineState::default(),
            },
            FloatingPlatform {
                state: EntityState {
                    position: Vec2::new(150.0, 50.0),
                    velocity: Vec2::ZERO, rotation: 0.0, on_ground: false,
                    size: Vec2::new(80.0, 12.0), opacity: 1.0, alive: true,
                },
                sine_config: SineConfig { property: SineProperty::X, magnitude: 60.0, period: 4.0 },
                sine_state: SineState::default(),
            },
        ];

        // Enemies
        self.enemies = vec![
            Enemy {
                state: EntityState {
                    position: Vec2::new(-200.0, -170.0), velocity: Vec2::ZERO,
                    rotation: 0.0, on_ground: false, size: Vec2::new(28.0, 28.0),
                    opacity: 1.0, alive: true,
                },
                fade_config: FadeConfig { fade_in_time: 0.0, fade_out_time: 0.5, destroy_on_fade_out: true },
                fade_state: FadeState::default(),
                hit: false,
            },
            Enemy {
                state: EntityState {
                    position: Vec2::new(200.0, -170.0), velocity: Vec2::ZERO,
                    rotation: 0.0, on_ground: false, size: Vec2::new(28.0, 28.0),
                    opacity: 1.0, alive: true,
                },
                fade_config: FadeConfig { fade_in_time: 0.0, fade_out_time: 0.5, destroy_on_fade_out: true },
                fade_state: FadeState::default(),
                hit: false,
            },
        ];

        // Background stars with Wrap
        for _ in 0..30 {
            let x = (self.rng_f32() - 0.5) * 1200.0;
            let y = (self.rng_f32() - 0.5) * 700.0;
            let speed = self.rng_f32() * 30.0 + 10.0;
            let sz = self.rng_f32() * 3.0 + 1.0;
            let op = self.rng_f32() * 0.5 + 0.3;
            self.stars.push(Star {
                state: EntityState {
                    position: Vec2::new(x, y), velocity: Vec2::ZERO,
                    rotation: 0.0, on_ground: false,
                    size: Vec2::splat(sz), opacity: op, alive: true,
                },
                wrap_config: WrapConfig::default(),
                vel: Vec2::new(-speed, 0.0),
            });
        }

        log::info!("Behaviors Demo! Arrows=move, Space=jump, Click=shoot");
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;

        // Build input
        let input = BehaviorInput {
            left: ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA),
            right: ctx.input.is_key_down(Key::ArrowRight) || ctx.input.is_key_down(Key::KeyD),
            up: ctx.input.is_key_down(Key::ArrowUp) || ctx.input.is_key_down(Key::KeyW),
            down: ctx.input.is_key_down(Key::ArrowDown) || ctx.input.is_key_down(Key::KeyS),
            jump_pressed: ctx.input.is_key_just_pressed(Key::Space),
            jump_down: ctx.input.is_key_down(Key::Space),
        };

        // Platform behavior on player — need to capture ground/platforms for solid check
        let ground = self.ground.clone();
        let plat_positions: Vec<(Vec2, Vec2)> = self.platforms.iter()
            .map(|p| (p.state.position, p.state.size * 0.5))
            .collect();

        let solid_check = move |pos: Vec2, half: Vec2| -> bool {
            let col = Collider::aabb(half.x, half.y);
            for (center, gh) in &ground {
                let gc = Collider::aabb(gh.x, gh.y);
                if overlap_test(pos, &col, *center, &gc).is_some() {
                    return true;
                }
            }
            for (center, ph) in &plat_positions {
                let pc = Collider::aabb(ph.x, ph.y);
                if overlap_test(pos, &col, *center, &pc).is_some() {
                    return true;
                }
            }
            false
        };

        platform::update(
            &self.player.platform_config,
            &mut self.player.platform_state,
            &mut self.player.state,
            &input,
            &solid_check,
            dt,
        );

        // Sine behavior on floating platforms
        for plat in &mut self.platforms {
            sine::update(&plat.sine_config, &mut plat.sine_state, &mut plat.state, dt);
        }

        // Shoot projectile on click
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

        // Bullet behavior on projectiles
        for proj in &mut self.projectiles {
            if proj.alive {
                bullet::update(&proj.bullet_config, &mut proj.bullet_state, &mut proj.state, dt);
                // Remove if too far
                if proj.state.position.length() > 800.0 {
                    proj.alive = false;
                }
            }
        }

        // Projectile-enemy collision → fade out enemy
        for enemy in &mut self.enemies {
            if !enemy.state.alive || enemy.hit { continue; }
            let ec = Collider::aabb(enemy.state.size.x * 0.5, enemy.state.size.y * 0.5);
            for proj in &mut self.projectiles {
                if !proj.alive { continue; }
                let pc = Collider::aabb(proj.state.size.x * 0.5, proj.state.size.y * 0.5);
                if overlap_test(enemy.state.position, &ec, proj.state.position, &pc).is_some() {
                    enemy.hit = true;
                    fade::start_fade_out(&mut enemy.fade_state);
                    proj.alive = false;
                }
            }
        }

        // Fade behavior on hit enemies
        for enemy in &mut self.enemies {
            if enemy.hit {
                fade::update(&enemy.fade_config, &mut enemy.fade_state, &mut enemy.state, dt);
            }
        }

        // Wrap behavior on stars
        let view_half = ctx.camera.half_viewport();
        let cam_pos = ctx.camera.position;
        for star in &mut self.stars {
            star.state.position += star.vel * dt;
            wrap::update(&star.wrap_config, &mut star.state, view_half, cam_pos);
        }

        // Cleanup
        self.projectiles.retain(|p| p.alive);
        self.enemies.retain(|e| e.state.alive);
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        // Stars (background)
        for star in &self.stars {
            let alpha = (star.state.opacity * 255.0) as u8;
            ctx.draw_sprite(DrawSprite {
                texture: tex, position: star.state.position,
                size: star.state.size, rotation: 0.0,
                color: pack_color(200, 200, 255, alpha), layer: -5,
                uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // Ground
        for (center, half) in &self.ground {
            ctx.draw_sprite(DrawSprite {
                texture: tex, position: *center,
                size: *half * 2.0, rotation: 0.0,
                color: pack_color(80, 120, 80, 255), layer: 0,
                uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // Floating platforms
        for plat in &self.platforms {
            ctx.draw_sprite(DrawSprite {
                texture: tex, position: plat.state.position,
                size: plat.state.size, rotation: 0.0,
                color: pack_color(100, 100, 160, 255), layer: 1,
                uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // Enemies
        for enemy in &self.enemies {
            let alpha = (enemy.state.opacity * 255.0) as u8;
            ctx.draw_sprite(DrawSprite {
                texture: tex, position: enemy.state.position,
                size: enemy.state.size, rotation: 0.0,
                color: pack_color(220, 60, 60, alpha), layer: 3,
                uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // Projectiles
        for proj in &self.projectiles {
            ctx.draw_sprite(DrawSprite {
                texture: tex, position: proj.state.position,
                size: proj.state.size, rotation: proj.bullet_config.angle_degrees.to_radians(),
                color: pack_color(255, 255, 100, 255), layer: 4,
                uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // Player
        let p = &self.player.state;
        ctx.draw_sprite(DrawSprite {
            texture: tex, position: p.position,
            size: Vec2::new(24.0, 32.0), rotation: 0.0,
            color: pack_color(80, 150, 230, 255), layer: 5,
            uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
        });

        // HUD
        if let Some(font) = self.font {
            let tl = ctx.camera.top_left();
            ctx.draw_text(
                "Behaviors Demo",
                Vec2::new(tl.x + 10.0, tl.y - 20.0),
                font, 14.0, COLOR_WHITE, 10,
            );
            ctx.draw_text(
                "Arrows=Move  Space=Jump  Click=Shoot",
                Vec2::new(tl.x + 10.0, tl.y - 42.0),
                font, 8.0, pack_color(150, 150, 170, 255), 10,
            );
            ctx.draw_text(
                "Platform | Sine | Bullet | Fade | Wrap",
                Vec2::new(tl.x + 10.0, tl.y - 60.0),
                font, 6.0, pack_color(100, 200, 100, 255), 10,
            );
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Behaviors Demo (v0.3)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.06, 0.06, 0.1))
        .run(BehaviorsDemo {
            white_tex: None,
            font: None,
            player: Player {
                state: EntityState {
                    position: Vec2::new(0.0, -150.0),
                    velocity: Vec2::ZERO, rotation: 0.0, on_ground: false,
                    size: Vec2::new(24.0, 32.0), opacity: 1.0, alive: true,
                },
                platform_config: PlatformConfig::default(),
                platform_state: PlatformState::default(),
            },
            platforms: Vec::new(),
            ground: Vec::new(),
            projectiles: Vec::new(),
            enemies: Vec::new(),
            stars: Vec::new(),
            rng: 54321,
        });
}
