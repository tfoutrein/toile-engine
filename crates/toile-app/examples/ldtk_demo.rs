//! Toile Engine — LDtk Import Demo (v0.3)
//!
//! Demonstrates importing an LDtk project file. The demo generates a sample
//! .ldtk file with multiple levels, entities, IntGrid collision, and tile layers,
//! then loads and renders it.
//!
//! Controls:
//!   Left/Right arrows: move player
//!   Space: jump
//!   1/2: switch between Level_0 and Level_1
//!
//! Run with: `cargo run --example ldtk_demo`

use std::path::Path;

use glam::Vec2;
use toile_app::{App, FontHandle, Game, GameContext, Key, TextureHandle, COLOR_WHITE};
use toile_assets::ldtk::{load_ldtk_scenes, LdtkLevelResult};
use toile_behaviors::platform::{self, PlatformConfig, PlatformState};
use toile_behaviors::types::{BehaviorInput, EntityState};
use toile_collision::{overlap_test, Collider};
use toile_core::color::Color;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};
use toile_scene::SceneData;

fn sample_ldtk() -> String {
    // A two-level LDtk project with entities, IntGrid collision, and a tile layer
    r#"{
        "jsonVersion": "1.5.3",
        "worldLayout": "LinearHorizontal",
        "levels": [
            {
                "identifier": "Level_0",
                "worldX": 0, "worldY": 0,
                "pxWid": 640, "pxHei": 360,
                "layerInstances": [
                    {
                        "__type": "Entities",
                        "__identifier": "Entities",
                        "__gridSize": 16, "__cWid": 40, "__cHei": 22,
                        "__tilesetRelPath": null, "__tilesetDefUid": null,
                        "gridTiles": [], "autoLayerTiles": [], "intGridCsv": [],
                        "entityInstances": [
                            { "__identifier": "Player", "px": [80, 280], "width": 24, "height": 32, "fieldInstances": [] },
                            { "__identifier": "Enemy", "px": [300, 280], "width": 28, "height": 28, "fieldInstances": [] },
                            { "__identifier": "Enemy", "px": [500, 280], "width": 28, "height": 28, "fieldInstances": [] },
                            { "__identifier": "Coin", "px": [200, 240], "width": 16, "height": 16, "fieldInstances": [] },
                            { "__identifier": "Coin", "px": [400, 200], "width": 16, "height": 16, "fieldInstances": [] },
                            { "__identifier": "Exit", "px": [600, 280], "width": 20, "height": 32, "fieldInstances": [] }
                        ]
                    },
                    {
                        "__type": "IntGrid",
                        "__identifier": "Collision",
                        "__gridSize": 16, "__cWid": 40, "__cHei": 22,
                        "__tilesetRelPath": null, "__tilesetDefUid": null,
                        "gridTiles": [], "autoLayerTiles": [],
                        "entityInstances": [],
                        "intGridCsv": [
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1
                        ]
                    }
                ]
            },
            {
                "identifier": "Level_1",
                "worldX": 640, "worldY": 0,
                "pxWid": 640, "pxHei": 360,
                "layerInstances": [
                    {
                        "__type": "Entities",
                        "__identifier": "Entities",
                        "__gridSize": 16, "__cWid": 40, "__cHei": 22,
                        "__tilesetRelPath": null, "__tilesetDefUid": null,
                        "gridTiles": [], "autoLayerTiles": [], "intGridCsv": [],
                        "entityInstances": [
                            { "__identifier": "Enemy", "px": [150, 280], "width": 28, "height": 28, "fieldInstances": [] },
                            { "__identifier": "Enemy", "px": [350, 240], "width": 28, "height": 28, "fieldInstances": [] },
                            { "__identifier": "Enemy", "px": [500, 280], "width": 28, "height": 28, "fieldInstances": [] },
                            { "__identifier": "Coin", "px": [250, 180], "width": 16, "height": 16, "fieldInstances": [] },
                            { "__identifier": "Coin", "px": [450, 220], "width": 16, "height": 16, "fieldInstances": [] },
                            { "__identifier": "Coin", "px": [100, 240], "width": 16, "height": 16, "fieldInstances": [] }
                        ]
                    },
                    {
                        "__type": "IntGrid",
                        "__identifier": "Collision",
                        "__gridSize": 16, "__cWid": 40, "__cHei": 22,
                        "__tilesetRelPath": null, "__tilesetDefUid": null,
                        "gridTiles": [], "autoLayerTiles": [],
                        "entityInstances": [],
                        "intGridCsv": [
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                            1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1
                        ]
                    }
                ]
            }
        ],
        "defs": {
            "tilesets": [],
            "entities": [
                { "uid": 1, "identifier": "Player", "width": 24, "height": 32 },
                { "uid": 2, "identifier": "Enemy", "width": 28, "height": 28 },
                { "uid": 3, "identifier": "Coin", "width": 16, "height": 16 },
                { "uid": 4, "identifier": "Exit", "width": 20, "height": 32 }
            ]
        }
    }"#.to_string()
}

struct LdtkDemo {
    white_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    levels: Vec<LdtkLevelResult>,
    current_level: usize,
    player: EntityState,
    player_config: PlatformConfig,
    player_state: PlatformState,
    score: u32,
    status: String,
}

fn entity_color(name: &str) -> u32 {
    match name {
        "Player" => pack_color(80, 150, 230, 255),
        "Enemy" => pack_color(220, 60, 60, 255),
        "Coin" => pack_color(255, 220, 50, 255),
        "Exit" => pack_color(100, 255, 150, 255),
        _ => pack_color(150, 150, 150, 255),
    }
}

impl LdtkDemo {
    fn current_scene(&self) -> &SceneData {
        &self.levels[self.current_level].scene
    }

    fn collect_solids(&self) -> Vec<(Vec2, Vec2)> {
        let scene = self.current_scene();
        let mut solids = Vec::new();

        // IntGrid collision tiles
        if let Some(ref tilemap) = scene.tilemap {
            let tw = tilemap.tile_size as f32;
            let map_h = (tilemap.height * tilemap.tile_size) as f32;

            for layer in &tilemap.layers {
                if !layer.name.contains("intgrid") { continue; }
                for row in 0..tilemap.height {
                    for col in 0..tilemap.width {
                        let val = layer.tiles[(row * tilemap.width + col) as usize];
                        if val > 0 {
                            let x = col as f32 * tw + tw * 0.5;
                            let y = map_h - (row as f32 * tw + tw * 0.5);
                            solids.push((Vec2::new(x, y), Vec2::new(tw * 0.5, tw * 0.5)));
                        }
                    }
                }
            }
        }

        solids
    }

    fn load_level(&mut self, idx: usize) {
        self.current_level = idx;
        self.player_state = PlatformState::default();

        // Find player entity in this level
        let scene = &self.levels[idx].scene;
        if let Some(p) = scene.entities.iter().find(|e| e.name == "Player") {
            self.player.position = Vec2::new(p.x, p.y);
            self.player.size = Vec2::new(p.width, p.height);
        } else {
            // Default spawn
            self.player.position = Vec2::new(40.0, 100.0);
        }
        self.player.velocity = Vec2::ZERO;
        self.player.on_ground = false;

        let level = &self.levels[idx];
        self.status = format!(
            "Level: {} ({}x{}) — {} entities",
            level.name, level.width, level.height,
            level.scene.entities.len()
        );
    }
}

impl Game for LdtkDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.white_tex = Some(ctx.load_texture(Path::new("assets/white.png")));
        self.font = Some(ctx.load_ttf(Path::new("assets/fonts/PressStart2P.ttf"), 32.0));

        // Write sample LDtk file and load it
        let ldtk_path = Path::new("ldtk_demo_sample.ldtk");
        std::fs::write(ldtk_path, sample_ldtk()).unwrap();
        self.levels = load_ldtk_scenes(ldtk_path).unwrap();
        let _ = std::fs::remove_file(ldtk_path);

        self.load_level(0);
        log::info!("LDtk Demo! {} levels loaded. Arrows=Move Space=Jump 1/2=SwitchLevel", self.levels.len());
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        let dt = dt as f32;

        // Switch level
        if ctx.input.is_key_just_pressed(Key::Digit1) && self.levels.len() > 0 {
            self.load_level(0);
            return;
        }
        if ctx.input.is_key_just_pressed(Key::Digit2) && self.levels.len() > 1 {
            self.load_level(1);
            return;
        }

        let input = BehaviorInput {
            left: ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA),
            right: ctx.input.is_key_down(Key::ArrowRight) || ctx.input.is_key_down(Key::KeyD),
            up: false,
            down: false,
            jump_pressed: ctx.input.is_key_just_pressed(Key::Space),
            jump_down: ctx.input.is_key_down(Key::Space),
        };

        let solids = self.collect_solids();
        let solid_check = move |pos: Vec2, half: Vec2| -> bool {
            let col = Collider::aabb(half.x, half.y);
            for (center, sh) in &solids {
                let sc = Collider::aabb(sh.x, sh.y);
                if overlap_test(pos, &col, *center, &sc).is_some() {
                    return true;
                }
            }
            false
        };

        platform::update(
            &self.player_config,
            &mut self.player_state,
            &mut self.player,
            &input,
            &solid_check,
            dt,
        );

        // Coin collection
        let player_col = Collider::aabb(self.player.size.x * 0.5, self.player.size.y * 0.5);
        let scene = &mut self.levels[self.current_level].scene;
        let player_pos = self.player.position;
        let before = scene.entities.len();
        scene.entities.retain(|e| {
            if e.name != "Coin" { return true; }
            let cc = Collider::aabb(e.width * 0.5, e.height * 0.5);
            overlap_test(player_pos, &player_col, Vec2::new(e.x, e.y), &cc).is_none()
        });
        let collected = before - scene.entities.len();
        if collected > 0 {
            self.score += collected as u32 * 50;
            self.status = format!("Coin! Score: {}", self.score);
        }

        // Exit detection
        if let Some(exit) = scene.entities.iter().find(|e| e.name == "Exit") {
            let ec = Collider::aabb(exit.width * 0.5, exit.height * 0.5);
            if overlap_test(player_pos, &player_col, Vec2::new(exit.x, exit.y), &ec).is_some() {
                if self.current_level + 1 < self.levels.len() {
                    let next = self.current_level + 1;
                    self.load_level(next);
                    self.status = format!("Next level! Score: {}", self.score);
                } else {
                    self.status = format!("You win! Final score: {}", self.score);
                }
            }
        }

        // Camera follows player
        ctx.camera.position = self.player.position;
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        let scene = self.current_scene();

        // Draw IntGrid collision tiles
        if let Some(ref tilemap) = scene.tilemap {
            let tw = tilemap.tile_size as f32;
            let map_h = (tilemap.height * tilemap.tile_size) as f32;

            for layer in &tilemap.layers {
                if !layer.name.contains("intgrid") { continue; }
                for row in 0..tilemap.height {
                    for col in 0..tilemap.width {
                        let val = layer.tiles[(row * tilemap.width + col) as usize];
                        if val > 0 {
                            let x = col as f32 * tw + tw * 0.5;
                            let y = map_h - (row as f32 * tw + tw * 0.5);
                            ctx.draw_sprite(DrawSprite {
                                texture: tex,
                                position: Vec2::new(x, y),
                                size: Vec2::splat(tw),
                                rotation: 0.0,
                                color: pack_color(80, 100, 80, 255),
                                layer: -1,
                                uv_min: Vec2::ZERO,
                                uv_max: Vec2::ONE,
                            });
                        }
                    }
                }
            }
        }

        // Draw entities (except Player, drawn separately)
        for entity in &scene.entities {
            if entity.name == "Player" { continue; }
            ctx.draw_sprite(DrawSprite {
                texture: tex,
                position: Vec2::new(entity.x, entity.y),
                size: Vec2::new(entity.width, entity.height),
                rotation: 0.0,
                color: entity_color(&entity.name),
                layer: 0,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
            });
        }

        // Draw player
        ctx.draw_sprite(DrawSprite {
            texture: tex,
            position: self.player.position,
            size: self.player.size,
            rotation: 0.0,
            color: entity_color("Player"),
            layer: 5,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        });

        // HUD
        if let Some(font) = self.font {
            let tl = ctx.camera.top_left();
            let level_name = &self.levels[self.current_level].name;
            ctx.draw_text(
                &format!("LDtk Import: {} | Score: {}", level_name, self.score),
                Vec2::new(tl.x + 10.0, tl.y - 20.0),
                font, 10.0, COLOR_WHITE, 50,
            );
            ctx.draw_text(
                "Arrows=Move Space=Jump | 1/2=Switch Level | Green=Exit",
                Vec2::new(tl.x + 10.0, tl.y - 38.0),
                font, 5.5, pack_color(150, 150, 170, 255), 50,
            );
            ctx.draw_text(
                &self.status,
                Vec2::new(tl.x + 10.0, tl.y - 52.0),
                font, 6.0, pack_color(100, 200, 100, 255), 50,
            );
        }
    }
}

fn main() {
    App::new()
        .with_title("Toile — LDtk Import Demo (v0.3)")
        .with_size(1280, 720)
        .with_clear_color(Color::rgb(0.08, 0.08, 0.12))
        .run(LdtkDemo {
            white_tex: None,
            font: None,
            levels: Vec::new(),
            current_level: 0,
            player: EntityState {
                position: Vec2::ZERO,
                velocity: Vec2::ZERO,
                rotation: 0.0,
                on_ground: false,
                size: Vec2::new(24.0, 32.0),
                opacity: 1.0,
                alive: true,
            },
            player_config: PlatformConfig::default(),
            player_state: PlatformState::default(),
            score: 0,
            status: String::new(),
        });
}
