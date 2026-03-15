//! Toile Engine — Prefab Demo (v0.3)
//!
//! Demonstrates the prefab system: save an entity as a template,
//! then instantiate multiple copies with different positions.
//!
//! Controls:
//!   Click: instantiate the selected prefab at cursor position
//!   1/2/3: select prefab type (Enemy, Coin, Platform)
//!   R: reset (clear all instances)
//!   S: save current scene to prefab_demo.json
//!
//! Run with: `cargo run --example prefab_demo`

use std::collections::HashMap;
use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, MouseButton as MB, TextureHandle, COLOR_WHITE};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};
use toile_scene::prefab::Prefab;
use toile_scene::{EntityData, SceneData};

struct PrefabDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    prefabs: Vec<Prefab>,
    selected_prefab: usize,
    scene: SceneData,
    status: String,
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
        // The entity name is "PrefabName_ID", extract the prefix
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

        self.status = "Click to place. 1=Enemy 2=Coin 3=Platform".to_string();
        log::info!("Prefab Demo! Click=place, 1/2/3=select, R=reset, S=save");
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f64) {
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

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        // Draw all instances
        for entity in &self.scene.entities {
            let color = Self::color_for_prefab(Self::prefab_name_for(entity));
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

        // Ghost preview at cursor
        let mouse_world = ctx.camera.screen_to_world(ctx.input.mouse_position());
        let prefab = &self.prefabs[self.selected_prefab];
        let ghost_color = Self::color_for_prefab(&prefab.name) & 0x80FFFFFF; // 50% alpha
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

        // HUD
        if let Some(font) = self.font {
            let tl = ctx.camera.top_left();
            ctx.draw_text(
                &format!("Prefab: {} | Instances: {}",
                    self.prefabs[self.selected_prefab].name,
                    self.scene.entities.len()),
                Vec2::new(tl.x + 10.0, tl.y - 20.0),
                font, 12.0, COLOR_WHITE, 50,
            );
            ctx.draw_text(
                "1:Enemy 2:Coin 3:Platform | Click=Place R=Reset S=Save",
                Vec2::new(tl.x + 10.0, tl.y - 42.0),
                font, 7.0, pack_color(150, 150, 170, 255), 50,
            );
            ctx.draw_text(
                &self.status,
                Vec2::new(tl.x + 10.0, tl.y - 60.0),
                font, 6.0, pack_color(100, 200, 100, 255), 50,
            );
        }
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
        });
}
