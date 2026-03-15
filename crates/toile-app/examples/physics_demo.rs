//! Toile Engine — Physics Demo (v0.2 Week 4)
//!
//! Rapier2D integration: dynamic boxes fall onto a static floor.
//! Click to spawn a new box. R to reset.
//!
//! Run with: `cargo run --example physics_demo`

use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, MouseButton, Sprite, TextureHandle, COLOR_WHITE};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};
use toile_physics::{BodyDef, BodyType, PhysicsBodyHandle, PhysicsShape, PhysicsWorld};

struct PhysicsDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    world: PhysicsWorld,
    bodies: Vec<(PhysicsBodyHandle, PhysicsShape, u32)>, // handle, shape (for rendering size), color
}

impl Game for PhysicsDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));

        self.spawn_floor();
        self.spawn_initial_boxes();

        log::info!("Physics Demo! Click=spawn box, R=reset, F3=debug");
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        // Spawn box on click
        if ctx.input.is_mouse_just_pressed(MouseButton::Left) {
            let world_pos = ctx.camera.screen_to_world(ctx.input.mouse_position());
            self.spawn_box(world_pos);
        }

        // Reset
        if ctx.input.is_key_just_pressed(Key::KeyR) {
            self.world = PhysicsWorld::new(Vec2::new(0.0, -500.0));
            self.bodies.clear();
            self.spawn_floor();
            self.spawn_initial_boxes();
        }

        // Step physics
        self.world.step(dt as f32);
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        // Draw all bodies
        for (handle, shape, color) in &self.bodies {
            if let Some((pos, rot)) = self.world.get_transform(*handle) {
                let size = match shape {
                    PhysicsShape::Box { half_w, half_h } => Vec2::new(half_w * 2.0, half_h * 2.0),
                    PhysicsShape::Circle { radius } => Vec2::splat(radius * 2.0),
                    _ => Vec2::splat(20.0),
                };
                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: pos,
                    size,
                    rotation: rot,
                    color: *color,
                    layer: 0,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }

        // HUD
        if let Some(font) = self.font {
            let tl = ctx.camera.top_left();
            ctx.draw_text(
                &format!("Bodies: {} | Click=Spawn R=Reset", self.bodies.len()),
                Vec2::new(tl.x + 10.0, tl.y - 20.0),
                font,
                12.0,
                COLOR_WHITE,
                10,
            );
        }
    }
}

impl PhysicsDemo {
    fn spawn_floor(&mut self) {
        let h = self.world.add_body(
            &BodyDef {
                body_type: BodyType::Static,
                position: Vec2::new(0.0, -200.0),
                ..Default::default()
            },
            &PhysicsShape::Box {
                half_w: 500.0,
                half_h: 15.0,
            },
        );
        self.bodies.push((
            h,
            PhysicsShape::Box {
                half_w: 500.0,
                half_h: 15.0,
            },
            pack_color(80, 120, 80, 255),
        ));

        // Angled platform
        let h2 = self.world.add_body(
            &BodyDef {
                body_type: BodyType::Static,
                position: Vec2::new(-150.0, -50.0),
                rotation: -0.3,
                ..Default::default()
            },
            &PhysicsShape::Box {
                half_w: 120.0,
                half_h: 8.0,
            },
        );
        self.bodies.push((
            h2,
            PhysicsShape::Box {
                half_w: 120.0,
                half_h: 8.0,
            },
            pack_color(100, 100, 140, 255),
        ));

        let h3 = self.world.add_body(
            &BodyDef {
                body_type: BodyType::Static,
                position: Vec2::new(150.0, 0.0),
                rotation: 0.2,
                ..Default::default()
            },
            &PhysicsShape::Box {
                half_w: 100.0,
                half_h: 8.0,
            },
        );
        self.bodies.push((
            h3,
            PhysicsShape::Box {
                half_w: 100.0,
                half_h: 8.0,
            },
            pack_color(100, 100, 140, 255),
        ));
    }

    fn spawn_initial_boxes(&mut self) {
        for i in 0..8 {
            let x = -100.0 + (i % 4) as f32 * 50.0;
            let y = 100.0 + (i / 4) as f32 * 60.0;
            self.spawn_box(Vec2::new(x, y));
        }
    }

    fn spawn_box(&mut self, pos: Vec2) {
        let half = 12.0 + (self.bodies.len() as f32 * 3.7).sin().abs() * 10.0;
        let h = self.world.add_body(
            &BodyDef {
                body_type: BodyType::Dynamic,
                position: pos,
                restitution: 0.4,
                friction: 0.6,
                ..Default::default()
            },
            &PhysicsShape::Box {
                half_w: half,
                half_h: half,
            },
        );
        let r = ((self.bodies.len() * 73) % 200 + 55) as u8;
        let g = ((self.bodies.len() * 137) % 200 + 55) as u8;
        let b = ((self.bodies.len() * 191) % 200 + 55) as u8;
        self.bodies.push((
            h,
            PhysicsShape::Box {
                half_w: half,
                half_h: half,
            },
            pack_color(r, g, b, 255),
        ));
    }
}

fn main() {
    App::new()
        .with_title("Toile — Physics Demo (Rapier2D)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.1, 0.1, 0.15))
        .run(PhysicsDemo {
            white_tex: None,
            font: None,
            world: PhysicsWorld::new(Vec2::new(0.0, -500.0)),
            bodies: Vec::new(),
        });
}
