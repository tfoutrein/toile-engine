//! Toile Engine — Breakout
//!
//! Showcases all engine features: sprites, input, ECS, collision,
//! audio (SFX + music), text rendering, camera, batching.
//!
//! Arrow keys or A/D to move paddle. Space to launch ball.
//! M to toggle music. F3 for debug overlay.
//!
//! Run with: `cargo run --example breakout`

use std::path::Path;

use toile_app::ecs::components::{ColliderComponent, SpriteComponent, Transform};
use toile_app::ecs::{Entity, World};
use toile_app::graphics::sprite_renderer::pack_color;
use toile_app::{
    App, FontHandle, Game, GameContext, Key, PlaybackId, SoundId, Sprite, TextureHandle,
    COLOR_WHITE,
};
use toile_collision::{overlap_test, Collider, Shape};
use toile_core::glam::Vec2;

const PADDLE_W: f32 = 120.0;
const PADDLE_H: f32 = 20.0;
const BALL_SIZE: f32 = 12.0;
const BALL_SPEED: f32 = 350.0;
const BRICK_W: f32 = 80.0;
const BRICK_H: f32 = 28.0;
const BRICK_COLS: i32 = 13;
const BRICK_ROWS: i32 = 5;
const BRICK_PAD: f32 = 4.0;
const HALF_W: f32 = 640.0;
const HALF_H: f32 = 360.0;

struct Brick;
struct Ball;
struct Paddle;

struct Breakout {
    world: World,
    paddle: Option<Entity>,
    ball: Option<Entity>,
    ball_vel: Vec2,
    launched: bool,
    score: u32,
    lives: u32,
    tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    // Audio
    sfx_bounce: Option<SoundId>,
    sfx_brick: Option<SoundId>,
    sfx_lose: Option<SoundId>,
    music_playback: Option<PlaybackId>,
    music_paused: bool,
}

impl Game for Breakout {
    fn init(&mut self, ctx: &mut GameContext) {
        self.tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        let tex = self.tex.unwrap();

        // Paddle
        self.paddle = Some(self.world.spawn((
            Paddle,
            Transform::at(Vec2::new(0.0, -HALF_H + 40.0)),
            SpriteComponent {
                texture: tex,
                size: Vec2::new(PADDLE_W, PADDLE_H),
                color: pack_color(200, 200, 220, 255),
                layer: 1,
            },
            ColliderComponent::aabb(PADDLE_W / 2.0, PADDLE_H / 2.0),
        )));

        // Ball
        self.ball = Some(self.world.spawn((
            Ball,
            Transform::at(Vec2::new(0.0, -HALF_H + 70.0)),
            SpriteComponent {
                texture: tex,
                size: Vec2::new(BALL_SIZE, BALL_SIZE),
                color: COLOR_WHITE,
                layer: 2,
            },
            ColliderComponent::circle(BALL_SIZE / 2.0),
        )));

        // Bricks
        let colors = [
            pack_color(230, 60, 60, 255),
            pack_color(230, 140, 40, 255),
            pack_color(230, 220, 40, 255),
            pack_color(60, 200, 60, 255),
            pack_color(60, 120, 230, 255),
        ];

        let grid_w = BRICK_COLS as f32 * (BRICK_W + BRICK_PAD) - BRICK_PAD;
        let start_x = -grid_w / 2.0 + BRICK_W / 2.0;
        let start_y = HALF_H - 80.0;

        for row in 0..BRICK_ROWS {
            for col in 0..BRICK_COLS {
                let x = start_x + col as f32 * (BRICK_W + BRICK_PAD);
                let y = start_y - row as f32 * (BRICK_H + BRICK_PAD);
                self.world.spawn((
                    Brick,
                    Transform::at(Vec2::new(x, y)),
                    SpriteComponent {
                        texture: tex,
                        size: Vec2::new(BRICK_W, BRICK_H),
                        color: colors[row as usize],
                        layer: 0,
                    },
                    ColliderComponent::aabb(BRICK_W / 2.0, BRICK_H / 2.0),
                ));
            }
        }

        // Walls (invisible, thick colliders)
        let wall_color = pack_color(80, 80, 100, 255);
        let wall_thickness = 20.0;

        // Left wall
        self.world.spawn((
            Transform::at(Vec2::new(-HALF_W - wall_thickness / 2.0, 0.0)),
            SpriteComponent { texture: tex, size: Vec2::new(wall_thickness, HALF_H * 2.0), color: wall_color, layer: 0 },
            ColliderComponent::aabb(wall_thickness / 2.0, HALF_H),
        ));
        // Right wall
        self.world.spawn((
            Transform::at(Vec2::new(HALF_W + wall_thickness / 2.0, 0.0)),
            SpriteComponent { texture: tex, size: Vec2::new(wall_thickness, HALF_H * 2.0), color: wall_color, layer: 0 },
            ColliderComponent::aabb(wall_thickness / 2.0, HALF_H),
        ));
        // Top wall
        self.world.spawn((
            Transform::at(Vec2::new(0.0, HALF_H + wall_thickness / 2.0)),
            SpriteComponent { texture: tex, size: Vec2::new(HALF_W * 2.0 + wall_thickness * 2.0, wall_thickness), color: wall_color, layer: 0 },
            ColliderComponent::aabb(HALF_W + wall_thickness, wall_thickness / 2.0),
        ));

        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));

        // Audio
        self.sfx_bounce = Some(
            ctx.audio
                .load_sound(Path::new("assets/bounce.wav"))
                .expect("Failed to load bounce.wav"),
        );
        self.sfx_brick = Some(
            ctx.audio
                .load_sound(Path::new("assets/brick_hit.wav"))
                .expect("Failed to load brick_hit.wav"),
        );
        self.sfx_lose = Some(
            ctx.audio
                .load_sound(Path::new("assets/lose_life.wav"))
                .expect("Failed to load lose_life.wav"),
        );

        // Background music (looped, quiet)
        let music = ctx
            .audio
            .load_sound(Path::new("assets/music_test.wav"))
            .expect("Failed to load music");
        let pb = ctx
            .audio
            .play_sound_looped(music)
            .expect("Failed to play music");
        ctx.audio.set_volume(pb, 0.15);
        self.music_playback = Some(pb);

        self.lives = 3;
        self.score = 0;
        log::info!("Breakout! Arrows/AD=move, Space=launch, M=music, F3=debug");
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;
        let paddle = self.paddle.unwrap();
        let ball = self.ball.unwrap();

        // Paddle movement
        {
            let mut paddle_t = self.world.get::<&mut Transform>(paddle).unwrap();
            let speed = 500.0 * dt;
            if ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA) {
                paddle_t.position.x -= speed;
            }
            if ctx.input.is_key_down(Key::ArrowRight) || ctx.input.is_key_down(Key::KeyD) {
                paddle_t.position.x += speed;
            }
            paddle_t.position.x = paddle_t.position.x.clamp(-HALF_W + PADDLE_W / 2.0, HALF_W - PADDLE_W / 2.0);
        }

        // Launch ball
        if !self.launched {
            let paddle_x = self.world.get::<&Transform>(paddle).unwrap().position.x;
            let mut ball_t = self.world.get::<&mut Transform>(ball).unwrap();
            ball_t.position.x = paddle_x;
            ball_t.position.y = -HALF_H + 70.0;

            if ctx.first_tick && ctx.input.is_key_just_pressed(Key::Space) {
                self.launched = true;
                self.ball_vel = Vec2::new(BALL_SPEED * 0.7, BALL_SPEED);
            }
            return;
        }

        // Move ball
        {
            let mut ball_t = self.world.get::<&mut Transform>(ball).unwrap();
            ball_t.position += self.ball_vel * dt;
        }

        // Ball collision with paddle
        {
            let ball_t = *self.world.get::<&Transform>(ball).unwrap();
            let ball_col = *self.world.get::<&ColliderComponent>(ball).unwrap();
            let paddle_t = *self.world.get::<&Transform>(paddle).unwrap();
            let paddle_col = *self.world.get::<&ColliderComponent>(paddle).unwrap();

            let ball_c = Collider { shape: ball_col.shape, offset: ball_col.offset };
            let paddle_c = Collider { shape: paddle_col.shape, offset: paddle_col.offset };

            if let Some(mtv) = overlap_test(ball_t.position, &ball_c, paddle_t.position, &paddle_c) {
                let mut ball_t = self.world.get::<&mut Transform>(ball).unwrap();
                ball_t.position += mtv;

                let hit_x = (ball_t.position.x - paddle_t.position.x) / (PADDLE_W / 2.0);
                self.ball_vel.y = self.ball_vel.y.abs();
                self.ball_vel.x = hit_x * BALL_SPEED;

                let speed = self.ball_vel.length();
                if speed > 0.0 {
                    self.ball_vel = self.ball_vel / speed * BALL_SPEED;
                }

                // Paddle bounce SFX
                if let Some(sfx) = self.sfx_bounce {
                    let _ = ctx.audio.play_sound(sfx);
                }
            }
        }

        // Ball collision with bricks
        let ball_t = *self.world.get::<&Transform>(ball).unwrap();
        let ball_col = *self.world.get::<&ColliderComponent>(ball).unwrap();
        let ball_c = Collider { shape: ball_col.shape, offset: ball_col.offset };

        let mut to_despawn = Vec::new();
        let mut brick_matches: Vec<(Entity, Transform, ColliderComponent)> = Vec::new();
        for entity in self.world.iter() {
            let entity_id = entity.entity();
            if let Ok(t) = self.world.get::<&Transform>(entity_id) {
                if let Ok(c) = self.world.get::<&ColliderComponent>(entity_id) {
                    if self.world.get::<&Brick>(entity_id).is_ok() {
                        brick_matches.push((entity_id, *t, *c));
                    }
                }
            }
        }

        for (entity, brick_t, brick_col) in &brick_matches {
            let brick_c = Collider { shape: brick_col.shape, offset: brick_col.offset };
            if let Some(mtv) = overlap_test(ball_t.position, &ball_c, brick_t.position, &brick_c) {
                to_despawn.push((*entity, mtv));
            }
        }

        if let Some((_, mtv)) = to_despawn.first() {
            // Bounce ball based on MTV direction
            if mtv.x.abs() > mtv.y.abs() {
                self.ball_vel.x = -self.ball_vel.x;
            } else {
                self.ball_vel.y = -self.ball_vel.y;
            }
            let mut ball_t = self.world.get::<&mut Transform>(ball).unwrap();
            ball_t.position += *mtv;
        }

        if !to_despawn.is_empty() {
            // Brick hit SFX
            if let Some(sfx) = self.sfx_brick {
                let _ = ctx.audio.play_sound(sfx);
            }
        }
        for (entity, _) in &to_despawn {
            let _ = self.world.despawn(*entity);
            self.score += 10;
        }

        // Ball collision with walls (non-brick entities with colliders but no Brick)
        {
            let ball_t = *self.world.get::<&Transform>(ball).unwrap();
            // Simple wall bounce by bounds check
            if ball_t.position.x - BALL_SIZE / 2.0 < -HALF_W {
                self.ball_vel.x = self.ball_vel.x.abs();
                let mut bt = self.world.get::<&mut Transform>(ball).unwrap();
                bt.position.x = -HALF_W + BALL_SIZE / 2.0;
            }
            if ball_t.position.x + BALL_SIZE / 2.0 > HALF_W {
                self.ball_vel.x = -self.ball_vel.x.abs();
                let mut bt = self.world.get::<&mut Transform>(ball).unwrap();
                bt.position.x = HALF_W - BALL_SIZE / 2.0;
            }
            if ball_t.position.y + BALL_SIZE / 2.0 > HALF_H {
                self.ball_vel.y = -self.ball_vel.y.abs();
                let mut bt = self.world.get::<&mut Transform>(ball).unwrap();
                bt.position.y = HALF_H - BALL_SIZE / 2.0;
            }
        }

        // Ball falls below screen
        {
            let ball_t = *self.world.get::<&Transform>(ball).unwrap();
            if ball_t.position.y < -HALF_H - 30.0 {
                self.lives = self.lives.saturating_sub(1);
                self.launched = false;
                let mut bt = self.world.get::<&mut Transform>(ball).unwrap();
                bt.position = Vec2::new(0.0, -HALF_H + 70.0);
                self.ball_vel = Vec2::ZERO;

                // Lose life SFX
                if let Some(sfx) = self.sfx_lose {
                    let _ = ctx.audio.play_sound(sfx);
                }
            }
        }

        // Toggle music with M (only on first tick to avoid double-toggle)
        if ctx.first_tick && ctx.input.is_key_just_pressed(Key::KeyM) {
            if let Some(pb) = self.music_playback {
                if self.music_paused {
                    ctx.audio.resume(pb);
                } else {
                    ctx.audio.pause(pb);
                }
                self.music_paused = !self.music_paused;
            }
        }

        // Check win
        let brick_count = self
            .world
            .iter()
            .filter(|e| self.world.get::<&Brick>(e.entity()).is_ok())
            .count();
        let state = if brick_count == 0 {
            "YOU WIN!"
        } else if self.lives == 0 {
            "GAME OVER"
        } else {
            ""
        };

        // Update window title with score
        // (can't set title from here directly — will show via F3 overlay or log)
        if !state.is_empty() && self.launched {
            log::info!("{} Score: {}", state, self.score);
            self.launched = false;
        }
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        // Draw all ECS entities with Transform + SpriteComponent
        // Draw all entities with Transform + SpriteComponent
        for entity in self.world.iter() {
            let eid = entity.entity();
            let Ok(transform) = self.world.get::<&Transform>(eid) else { continue };
            let Ok(sprite) = self.world.get::<&SpriteComponent>(eid) else { continue };
            let (transform, sprite) = (*transform, *sprite);
            ctx.draw_sprite(Sprite {
                texture: sprite.texture,
                position: transform.position,
                size: sprite.size * transform.scale,
                rotation: transform.rotation,
                color: sprite.color,
                layer: sprite.layer,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // HUD text
        if let Some(font) = self.font {
            ctx.draw_text(
                &format!("SCORE: {}", self.score),
                Vec2::new(-HALF_W + 20.0, HALF_H - 30.0),
                font,
                20.0,
                COLOR_WHITE,
                10,
            );
            ctx.draw_text(
                &format!("LIVES: {}", self.lives),
                Vec2::new(HALF_W - 220.0, HALF_H - 30.0),
                font,
                20.0,
                COLOR_WHITE,
                10,
            );
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Breakout")
        .with_size(1280, 720)
        .run(Breakout {
            world: World::new(),
            paddle: None,
            ball: None,
            ball_vel: Vec2::ZERO,
            launched: false,
            score: 0,
            lives: 3,
            tex: None,
            font: None,
            sfx_bounce: None,
            sfx_brick: None,
            sfx_lose: None,
            music_playback: None,
            music_paused: false,
        });
}
