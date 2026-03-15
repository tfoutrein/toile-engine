use std::path::Path;

use toile_behaviors::platform::PlatformConfig;
use toile_behaviors::topdown::TopDownConfig;
use toile_behaviors::bullet::BulletConfig;
use toile_behaviors::fade::FadeConfig;
use toile_behaviors::sine::{SineConfig, SineProperty};
use toile_behaviors::BehaviorConfig;
use toile_scene::prefab::{Prefab, save_prefab};
use toile_scene::{EntityData, SceneData, save_scene};

pub const TEMPLATES: &[&str] = &["empty", "platformer", "topdown", "shmup"];

pub fn generate(name: &str, template: &str, dir: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();

    // Common directories
    std::fs::create_dir_all(dir.join("assets")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir.join("scripts")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir.join("scenes")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir.join("prefabs")).map_err(|e| e.to_string())?;

    // Toile.toml
    let toml = format!(
        "[project]\nname = \"{name}\"\nversion = \"0.1.0\"\nengine = \"toile\"\ntemplate = \"{template}\"\n"
    );
    write_file(dir, "Toile.toml", &toml, &mut files)?;

    match template {
        "empty" => gen_empty(name, dir, &mut files)?,
        "platformer" => gen_platformer(name, dir, &mut files)?,
        "topdown" => gen_topdown(name, dir, &mut files)?,
        "shmup" => gen_shmup(name, dir, &mut files)?,
        _ => return Err(format!("Unknown template: {template}")),
    }

    // llms.txt
    let llms = gen_llms_txt(name, template);
    write_file(dir, "llms.txt", &llms, &mut files)?;

    Ok(files)
}

fn write_file(dir: &Path, rel: &str, content: &str, files: &mut Vec<String>) -> Result<(), String> {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    files.push(rel.to_string());
    Ok(())
}

fn save_scene_file(dir: &Path, rel: &str, scene: &SceneData, files: &mut Vec<String>) -> Result<(), String> {
    let path = dir.join(rel);
    save_scene(&path, scene).map_err(|e| e.to_string())?;
    files.push(rel.to_string());
    Ok(())
}

fn save_prefab_file(dir: &Path, prefab: &Prefab, files: &mut Vec<String>) -> Result<(), String> {
    let rel = format!("prefabs/{}.prefab.json", prefab.name);
    let path = dir.join(&rel);
    save_prefab(&path, prefab).map_err(|e| e.to_string())?;
    files.push(rel);
    Ok(())
}

fn make_entity(id: u64, name: &str, x: f32, y: f32, w: f32, h: f32, layer: i32) -> EntityData {
    EntityData {
        id, name: name.to_string(),
        x, y, rotation: 0.0,
        scale_x: 1.0, scale_y: 1.0,
        layer, sprite_path: String::new(),
        width: w, height: h,
    }
}

fn make_prefab(name: &str, w: f32, h: f32, layer: i32, behaviors: Vec<BehaviorConfig>) -> Prefab {
    Prefab {
        name: name.to_string(),
        entity: make_entity(0, name, 0.0, 0.0, w, h, layer),
        behaviors: behaviors.iter()
            .map(|b| serde_json::to_value(b).unwrap())
            .collect(),
        event_sheet: None,
    }
}

// ── Empty ────────────────────────────────────────────────────

fn gen_empty(_name: &str, dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
    let scene = SceneData::new("main");
    save_scene_file(dir, "scenes/main.json", &scene, files)?;

    let event_sheet = r#"{
  "name": "main",
  "events": []
}
"#;
    write_file(dir, "scripts/main.event.json", event_sheet, files)?;
    Ok(())
}

// ── Platformer ───────────────────────────────────────────────

fn gen_platformer(_name: &str, dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
    let mut scene = SceneData::new("main");

    // Ground
    scene.entities.push(make_entity(1, "Ground", 0.0, -200.0, 1000.0, 30.0, -1));
    // Platforms
    scene.entities.push(make_entity(2, "Platform_1", -200.0, -100.0, 120.0, 16.0, -1));
    scene.entities.push(make_entity(3, "Platform_2", 100.0, -20.0, 100.0, 16.0, -1));
    scene.entities.push(make_entity(4, "Platform_3", 300.0, 60.0, 80.0, 16.0, -1));
    // Player
    scene.entities.push(make_entity(5, "Player", 0.0, -150.0, 24.0, 32.0, 1));
    // Enemies
    scene.entities.push(make_entity(6, "Enemy_1", -250.0, -170.0, 28.0, 28.0, 0));
    scene.entities.push(make_entity(7, "Enemy_2", 200.0, -170.0, 28.0, 28.0, 0));
    // Collectibles
    scene.entities.push(make_entity(8, "Coin_1", -200.0, -70.0, 16.0, 16.0, 0));
    scene.entities.push(make_entity(9, "Coin_2", 100.0, 10.0, 16.0, 16.0, 0));
    scene.entities.push(make_entity(10, "Coin_3", 300.0, 90.0, 16.0, 16.0, 0));
    scene.next_id = 11;

    save_scene_file(dir, "scenes/main.json", &scene, files)?;

    // Prefabs
    save_prefab_file(dir, &make_prefab("Player", 24.0, 32.0, 1, vec![
        BehaviorConfig::Platform(PlatformConfig::default()),
    ]), files)?;

    save_prefab_file(dir, &make_prefab("Enemy", 28.0, 28.0, 0, vec![
        BehaviorConfig::Fade(FadeConfig { fade_in_time: 0.0, fade_out_time: 0.5, destroy_on_fade_out: true }),
    ]), files)?;

    save_prefab_file(dir, &make_prefab("Coin", 16.0, 16.0, 0, vec![
        BehaviorConfig::Sine(SineConfig { property: SineProperty::Y, magnitude: 5.0, period: 2.0 }),
        BehaviorConfig::Fade(FadeConfig { fade_in_time: 0.0, fade_out_time: 0.3, destroy_on_fade_out: true }),
    ]), files)?;

    save_prefab_file(dir, &make_prefab("Platform", 120.0, 16.0, -1, vec![
        BehaviorConfig::Solid,
    ]), files)?;

    // Event sheet
    let event_sheet = r#"{
  "name": "platformer_rules",
  "events": [
    {
      "name": "collect_coin",
      "conditions": [
        { "kind": "Overlap", "params": { "entity_a": "Player", "entity_b": "Coin" } }
      ],
      "actions": [
        { "kind": "AddScore", "params": { "value": 50 } },
        { "kind": "Destroy", "params": { "target": "Coin" } }
      ]
    },
    {
      "name": "enemy_hit",
      "conditions": [
        { "kind": "Overlap", "params": { "entity_a": "Bullet", "entity_b": "Enemy" } }
      ],
      "actions": [
        { "kind": "AddScore", "params": { "value": 100 } },
        { "kind": "Destroy", "params": { "target": "Enemy" } },
        { "kind": "Destroy", "params": { "target": "Bullet" } }
      ]
    }
  ]
}
"#;
    write_file(dir, "scripts/platformer_rules.event.json", event_sheet, files)?;

    Ok(())
}

// ── Top-Down ─────────────────────────────────────────────────

fn gen_topdown(_name: &str, dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
    let mut scene = SceneData::new("main");

    // Walls (arena border)
    scene.entities.push(make_entity(1, "Wall_Top", 0.0, 250.0, 600.0, 20.0, -1));
    scene.entities.push(make_entity(2, "Wall_Bottom", 0.0, -250.0, 600.0, 20.0, -1));
    scene.entities.push(make_entity(3, "Wall_Left", -300.0, 0.0, 20.0, 520.0, -1));
    scene.entities.push(make_entity(4, "Wall_Right", 300.0, 0.0, 20.0, 520.0, -1));
    // Interior walls
    scene.entities.push(make_entity(5, "Wall_5", -100.0, 100.0, 120.0, 20.0, -1));
    scene.entities.push(make_entity(6, "Wall_6", 100.0, -60.0, 20.0, 120.0, -1));
    // Player
    scene.entities.push(make_entity(7, "Player", 0.0, 0.0, 24.0, 24.0, 1));
    // Enemies (patrol)
    scene.entities.push(make_entity(8, "Enemy_1", -200.0, -150.0, 24.0, 24.0, 0));
    scene.entities.push(make_entity(9, "Enemy_2", 200.0, 150.0, 24.0, 24.0, 0));
    scene.entities.push(make_entity(10, "Enemy_3", -150.0, 180.0, 24.0, 24.0, 0));
    // Items
    scene.entities.push(make_entity(11, "Key_1", -200.0, 100.0, 14.0, 14.0, 0));
    scene.entities.push(make_entity(12, "Gem_1", 200.0, -150.0, 12.0, 12.0, 0));
    scene.entities.push(make_entity(13, "Gem_2", -50.0, -200.0, 12.0, 12.0, 0));
    scene.next_id = 14;

    save_scene_file(dir, "scenes/main.json", &scene, files)?;

    // Prefabs
    save_prefab_file(dir, &make_prefab("Player", 24.0, 24.0, 1, vec![
        BehaviorConfig::TopDown(TopDownConfig::default()),
    ]), files)?;

    save_prefab_file(dir, &make_prefab("Enemy", 24.0, 24.0, 0, vec![
        BehaviorConfig::TopDown(TopDownConfig { max_speed: 80.0, ..TopDownConfig::default() }),
        BehaviorConfig::Fade(FadeConfig { fade_in_time: 0.0, fade_out_time: 0.5, destroy_on_fade_out: true }),
    ]), files)?;

    save_prefab_file(dir, &make_prefab("Wall", 120.0, 20.0, -1, vec![
        BehaviorConfig::Solid,
    ]), files)?;

    save_prefab_file(dir, &make_prefab("Gem", 12.0, 12.0, 0, vec![
        BehaviorConfig::Sine(SineConfig { property: SineProperty::Opacity, magnitude: 0.3, period: 1.5 }),
    ]), files)?;

    // Event sheet
    let event_sheet = r#"{
  "name": "topdown_rules",
  "events": [
    {
      "name": "collect_gem",
      "conditions": [
        { "kind": "Overlap", "params": { "entity_a": "Player", "entity_b": "Gem" } }
      ],
      "actions": [
        { "kind": "AddScore", "params": { "value": 25 } },
        { "kind": "Destroy", "params": { "target": "Gem" } }
      ]
    },
    {
      "name": "enemy_kills_player",
      "conditions": [
        { "kind": "Overlap", "params": { "entity_a": "Player", "entity_b": "Enemy" } }
      ],
      "actions": [
        { "kind": "LoseLife", "params": {} },
        { "kind": "SetPosition", "params": { "target": "Player", "x": 0, "y": 0 } }
      ]
    }
  ]
}
"#;
    write_file(dir, "scripts/topdown_rules.event.json", event_sheet, files)?;

    Ok(())
}

// ── Shoot-em-up ──────────────────────────────────────────────

fn gen_shmup(_name: &str, dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
    let mut scene = SceneData::new("main");

    // Player ship at bottom center
    scene.entities.push(make_entity(1, "Player", 0.0, -250.0, 32.0, 24.0, 1));
    // Enemy wave
    for i in 0..5 {
        let x = -160.0 + (i as f32) * 80.0;
        scene.entities.push(make_entity(
            2 + i, &format!("Enemy_{}", i + 1),
            x, 200.0, 28.0, 28.0, 0,
        ));
    }
    // Second wave
    for i in 0..3 {
        let x = -80.0 + (i as f32) * 80.0;
        scene.entities.push(make_entity(
            7 + i, &format!("Enemy_{}", 6 + i),
            x, 280.0, 28.0, 28.0, 0,
        ));
    }
    scene.next_id = 10;

    save_scene_file(dir, "scenes/main.json", &scene, files)?;

    // Prefabs
    save_prefab_file(dir, &make_prefab("Player", 32.0, 24.0, 1, vec![
        BehaviorConfig::TopDown(TopDownConfig { max_speed: 250.0, acceleration: 2000.0, deceleration: 1500.0, diagonal_correction: true }),
    ]), files)?;

    save_prefab_file(dir, &make_prefab("Enemy", 28.0, 28.0, 0, vec![
        BehaviorConfig::Bullet(BulletConfig { speed: 60.0, angle_degrees: -90.0, ..BulletConfig::default() }),
        BehaviorConfig::Fade(FadeConfig { fade_in_time: 0.0, fade_out_time: 0.3, destroy_on_fade_out: true }),
    ]), files)?;

    save_prefab_file(dir, &make_prefab("PlayerBullet", 6.0, 12.0, 2, vec![
        BehaviorConfig::Bullet(BulletConfig { speed: 500.0, angle_degrees: 90.0, ..BulletConfig::default() }),
    ]), files)?;

    save_prefab_file(dir, &make_prefab("EnemyBullet", 6.0, 6.0, 2, vec![
        BehaviorConfig::Bullet(BulletConfig { speed: 200.0, angle_degrees: -90.0, ..BulletConfig::default() }),
    ]), files)?;

    // Event sheet
    let event_sheet = r#"{
  "name": "shmup_rules",
  "events": [
    {
      "name": "player_shoots",
      "conditions": [
        { "kind": "KeyPressed", "params": { "key": "Space" } }
      ],
      "actions": [
        { "kind": "SpawnPrefab", "params": { "prefab": "PlayerBullet", "at": "Player", "offset_y": 20 } }
      ]
    },
    {
      "name": "bullet_hits_enemy",
      "conditions": [
        { "kind": "Overlap", "params": { "entity_a": "PlayerBullet", "entity_b": "Enemy" } }
      ],
      "actions": [
        { "kind": "AddScore", "params": { "value": 100 } },
        { "kind": "Destroy", "params": { "target": "Enemy" } },
        { "kind": "Destroy", "params": { "target": "PlayerBullet" } }
      ]
    },
    {
      "name": "enemy_bullet_hits_player",
      "conditions": [
        { "kind": "Overlap", "params": { "entity_a": "EnemyBullet", "entity_b": "Player" } }
      ],
      "actions": [
        { "kind": "LoseLife", "params": {} },
        { "kind": "Destroy", "params": { "target": "EnemyBullet" } }
      ]
    }
  ]
}
"#;
    write_file(dir, "scripts/shmup_rules.event.json", event_sheet, files)?;

    Ok(())
}

// ── llms.txt ─────────────────────────────────────────────────

fn gen_llms_txt(name: &str, template: &str) -> String {
    let template_desc = match template {
        "empty" => "An empty project with a blank scene. Add entities, behaviors, and scripts to build your game.",
        "platformer" => "A side-scrolling platformer with a player (Platform behavior), enemies, collectible coins, and platforms. Arrow keys to move, Space to jump, Click to shoot.",
        "topdown" => "A top-down game with a player (TopDown behavior), walls, patrol enemies, and collectible gems. Arrow keys to move in 4/8 directions.",
        "shmup" => "A vertical shoot-em-up with a player ship, enemy waves, and projectiles. Arrow keys to move, Space to shoot.",
        _ => "A Toile Engine project.",
    };

    format!(
r#"# {name}

> A 2D game built with Toile Engine.
> Template: {template}

## Description

{template_desc}

## Project Structure

- `Toile.toml` — Project manifest
- `scenes/main.json` — Main scene with entities (position, size, layer)
- `prefabs/` — Reusable entity templates with behaviors
- `assets/` — Sprites, sounds, and other assets
- `scripts/` — Event sheets (.event.json) and Lua scripts

## Entities

Entities are defined in scene JSON files with properties:
- id, name, x, y, width, height, rotation, scale_x, scale_y, layer, sprite_path

## Behaviors

Behaviors are attached to prefabs and define entity logic:
- **Platform** — Side-scrolling character controller (gravity, jump, coyote time)
- **TopDown** — 4/8-direction movement
- **Bullet** — Move in a straight line at a given angle and speed
- **Sine** — Oscillate a property (Y, X, Opacity, Angle, Size) over time
- **Fade** — Fade in/out with optional destroy on fade-out
- **Wrap** — Wrap around screen edges
- **Solid** — Static collision

## Event Sheets

Event sheets define game rules as condition-action pairs:
- Conditions: Overlap, KeyPressed, Timer, CompareVariable, etc.
- Actions: AddScore, Destroy, SetPosition, SpawnPrefab, LoseLife, etc.

## AI Integration

This project can be controlled via Toile's MCP server:
- Create/edit scenes and entities
- Place prefab instances
- Modify entity properties
"#)
}
