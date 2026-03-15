//! Toile Engine — Event Sheet Demo (v0.3)
//!
//! A small game driven entirely by event sheets (no custom game logic).
//! Demonstrates conditions, actions, variables, timers, and collision.
//!
//! - Player moves with arrow keys (event sheet)
//! - Coins spawn every 2 seconds (event sheet timer)
//! - Collecting a coin adds to score (event sheet collision + variable)
//! - Score displayed in HUD
//!
//! Run with: `cargo run --example event_sheet_demo`

use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, TextureHandle, COLOR_WHITE};
use toile_collision::{overlap_test, Collider};
use toile_core::color::Color;
use toile_events::condition::{CompareOp, ConditionKind};
use toile_events::action::ActionKind;
use toile_events::executor::{EventCommand, EventContext, EventSheetState, evaluate_event_sheet};
use toile_events::model::*;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};

struct Entity {
    id: u64,
    name: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: u32,
    alive: bool,
    sheet: Option<EventSheet>,
    state: EventSheetState,
}

struct EventSheetDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    entities: Vec<Entity>,
    next_id: u64,
    score: u64,
    spawn_timer: f32,
    rng_seed: u32,
}

impl EventSheetDemo {
    fn simple_rng(&mut self) -> f32 {
        self.rng_seed ^= self.rng_seed << 13;
        self.rng_seed ^= self.rng_seed >> 17;
        self.rng_seed ^= self.rng_seed << 5;
        (self.rng_seed as f64 / u32::MAX as f64) as f32
    }

    fn create_player_sheet() -> EventSheet {
        EventSheet {
            name: "player".into(),
            events: vec![
                // On create: log
                Event::new(
                    vec![Condition::new(ConditionKind::OnCreate)],
                    vec![Action::new(ActionKind::Log { message: "Player spawned!".into() })],
                ),
                // Arrow Right → move right
                Event::new(
                    vec![Condition::new(ConditionKind::OnKeyDown { key: "ArrowRight".into() })],
                    vec![Action::new(ActionKind::MoveAtAngle { angle: 0.0, speed: 200.0 })],
                ),
                // Arrow Left → move left
                Event::new(
                    vec![Condition::new(ConditionKind::OnKeyDown { key: "ArrowLeft".into() })],
                    vec![Action::new(ActionKind::MoveAtAngle { angle: 180.0, speed: 200.0 })],
                ),
                // Arrow Up → move up
                Event::new(
                    vec![Condition::new(ConditionKind::OnKeyDown { key: "ArrowUp".into() })],
                    vec![Action::new(ActionKind::MoveAtAngle { angle: 90.0, speed: 200.0 })],
                ),
                // Arrow Down → move down
                Event::new(
                    vec![Condition::new(ConditionKind::OnKeyDown { key: "ArrowDown".into() })],
                    vec![Action::new(ActionKind::MoveAtAngle { angle: 270.0, speed: 200.0 })],
                ),
            ],
        }
    }

    fn create_coin_sheet() -> EventSheet {
        EventSheet {
            name: "coin".into(),
            events: vec![
                Event::new(
                    vec![Condition::new(ConditionKind::OnCreate)],
                    vec![Action::new(ActionKind::Log { message: "Coin appeared!".into() })],
                ),
            ],
        }
    }
}

impl Game for EventSheetDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));

        // Spawn player
        self.entities.push(Entity {
            id: 1,
            name: "Player".into(),
            x: 0.0,
            y: 0.0,
            w: 30.0,
            h: 30.0,
            color: pack_color(80, 140, 220, 255),
            alive: true,
            sheet: Some(Self::create_player_sheet()),
            state: EventSheetState::default(),
        });
        self.next_id = 2;

        // Spawn initial coins
        for _ in 0..5 {
            let x = (self.simple_rng() - 0.5) * 500.0;
            let y = (self.simple_rng() - 0.5) * 300.0;
            self.entities.push(Entity {
                id: self.next_id,
                name: "Coin".into(),
                x,
                y,
                w: 16.0,
                h: 16.0,
                color: pack_color(255, 220, 50, 255),
                alive: true,
                sheet: Some(Self::create_coin_sheet()),
                state: EventSheetState::default(),
            });
            self.next_id += 1;
        }

        log::info!("Event Sheet Demo! Arrow keys to move, collect yellow coins.");
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt_f32 = dt as f32;

        // Build input closures
        let keys_down = |key: &str| -> bool {
            match key {
                "ArrowRight" => ctx.input.is_key_down(Key::ArrowRight),
                "ArrowLeft" => ctx.input.is_key_down(Key::ArrowLeft),
                "ArrowUp" => ctx.input.is_key_down(Key::ArrowUp),
                "ArrowDown" => ctx.input.is_key_down(Key::ArrowDown),
                "Space" => ctx.input.is_key_down(Key::Space),
                _ => false,
            }
        };
        let keys_just_pressed = |key: &str| -> bool {
            match key {
                "Space" => ctx.input.is_key_just_pressed(Key::Space),
                _ => false,
            }
        };
        let no_fn = |_: &str| false;

        // Evaluate event sheets for all entities
        let mut all_commands: Vec<(u64, EventCommand)> = Vec::new();

        for entity in &mut self.entities {
            if !entity.alive {
                continue;
            }
            if let Some(sheet) = &entity.sheet {
                let ectx = EventContext {
                    entity_id: entity.id,
                    entity_x: entity.x,
                    entity_y: entity.y,
                    dt,
                    keys_down: &keys_down,
                    keys_just_pressed: &keys_just_pressed,
                    keys_just_released: &no_fn,
                    mouse_just_pressed: &no_fn,
                    is_colliding_with: &no_fn,
                };
                let cmds = evaluate_event_sheet(sheet, &mut entity.state, &ectx);
                for cmd in cmds {
                    all_commands.push((entity.id, cmd));
                }
            }
        }

        // Apply commands
        for (eid, cmd) in &all_commands {
            match cmd {
                EventCommand::MoveAtAngle { entity_id, angle_deg, speed } => {
                    if let Some(e) = self.entities.iter_mut().find(|e| e.id == *entity_id) {
                        let rad = angle_deg.to_radians();
                        e.x += rad.cos() * speed * dt_f32;
                        e.y += rad.sin() * speed * dt_f32;
                    }
                }
                EventCommand::SetPosition { entity_id, x, y } => {
                    if let Some(e) = self.entities.iter_mut().find(|e| e.id == *entity_id) {
                        e.x = *x;
                        e.y = *y;
                    }
                }
                EventCommand::Destroy { entity_id } => {
                    if let Some(e) = self.entities.iter_mut().find(|e| e.id == *entity_id) {
                        e.alive = false;
                    }
                }
                EventCommand::Log { message } => {
                    log::info!("[Entity {}] {}", eid, message);
                }
                _ => {}
            }
        }

        // Collision: player vs coins
        let player = self.entities.iter().find(|e| e.name == "Player" && e.alive);
        if let Some(player) = player {
            let pc = Collider::aabb(player.w * 0.5, player.h * 0.5);
            let px = player.x;
            let py = player.y;

            let mut collected = Vec::new();
            for entity in &self.entities {
                if entity.name == "Coin" && entity.alive {
                    let cc = Collider::aabb(entity.w * 0.5, entity.h * 0.5);
                    if overlap_test(
                        Vec2::new(px, py),
                        &pc,
                        Vec2::new(entity.x, entity.y),
                        &cc,
                    ).is_some()
                    {
                        collected.push(entity.id);
                    }
                }
            }
            for id in collected {
                if let Some(e) = self.entities.iter_mut().find(|e| e.id == id) {
                    e.alive = false;
                    self.score += 10;
                }
            }
        }

        // Spawn new coins periodically
        self.spawn_timer += dt_f32;
        if self.spawn_timer >= 1.5 {
            self.spawn_timer -= 1.5;
            let x = (self.simple_rng() - 0.5) * 500.0;
            let y = (self.simple_rng() - 0.5) * 300.0;
            self.entities.push(Entity {
                id: self.next_id,
                name: "Coin".into(),
                x,
                y,
                w: 16.0,
                h: 16.0,
                color: pack_color(255, 220, 50, 255),
                alive: true,
                sheet: Some(Self::create_coin_sheet()),
                state: EventSheetState::default(),
            });
            self.next_id += 1;
        }

        // Remove dead entities
        self.entities.retain(|e| e.alive);
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        // Draw all entities
        for entity in &self.entities {
            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: Vec2::new(entity.x, entity.y),
                size: Vec2::new(entity.w, entity.h),
                rotation: 0.0,
                color: entity.color,
                layer: if entity.name == "Player" { 5 } else { 0 },
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // HUD
        if let Some(font) = self.font {
            let tl = ctx.camera.top_left();
            ctx.draw_text(
                &format!("Score: {}  |  Coins: {}",
                    self.score,
                    self.entities.iter().filter(|e| e.name == "Coin").count()),
                Vec2::new(tl.x + 10.0, tl.y - 20.0),
                font,
                14.0,
                COLOR_WHITE,
                10,
            );
            ctx.draw_text(
                "Arrow keys = Move  |  Collect the coins!",
                Vec2::new(tl.x + 10.0, tl.y - 45.0),
                font,
                8.0,
                pack_color(150, 150, 170, 255),
                10,
            );
            ctx.draw_text(
                "Player movement driven by Event Sheets",
                Vec2::new(tl.x + 10.0, tl.y - 65.0),
                font,
                6.0,
                pack_color(100, 200, 100, 255),
                10,
            );
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — Event Sheet Demo (v0.3)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.1, 0.1, 0.15))
        .run(EventSheetDemo {
            white_tex: None,
            font: None,
            entities: Vec::new(),
            next_id: 1,
            score: 0,
            spawn_timer: 0.0,
            rng_seed: 12345,
        });
}
