//! CLI for the Toile visual test harness.
//!
//!   toile-harness smoke                       # GPU smoke test -> PNG
//!   toile-harness scene path/to/scene.json    # render a scene -> PNG
//!
//! Exits non-zero on failure so it can gate CI / scripts.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use glam::Vec2;
use toile_core::color::Color;
use toile_graphics::sprite_renderer::{pack_color, DrawSprite};
use toile_harness::{Harness, SnapshotOptions};

#[derive(Parser)]
#[command(name = "toile-harness", about = "Headless render + visual snapshot harness for Toile", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a known colour pattern off-screen and save a PNG (GPU smoke test).
    Smoke {
        #[arg(long, default_value = "harness-out/smoke.png")]
        out: PathBuf,
        #[arg(long, default_value_t = 256)]
        width: u32,
        #[arg(long, default_value_t = 256)]
        height: u32,
    },
    /// Render a scene JSON file to a PNG.
    Scene {
        /// Path to a scene `.json` file.
        scene: PathBuf,
        /// Output PNG path (defaults to `<scene>.png`).
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = 800)]
        width: u32,
        #[arg(long, default_value_t = 600)]
        height: u32,
        /// Camera zoom; omit to auto-fit all entities.
        #[arg(long)]
        zoom: Option<f32>,
        /// Camera centre as "x,y" in world space.
        #[arg(long)]
        camera: Option<String>,
        /// Assets root for resolving relative `sprite_path`s (default: scene file's directory).
        #[arg(long)]
        assets: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Smoke { out, width, height } => smoke(out, width, height),
        Commands::Scene { scene, out, width, height, zoom, camera, assets } => {
            render_scene(scene, out, width, height, zoom, camera, assets)
        }
    };

    match result {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn smoke(out: PathBuf, width: u32, height: u32) -> Result<String, String> {
    let mut h = Harness::new(width, height)?;
    let w = h.white();

    // A deterministic pattern: R/G/B quads in a row + a rotated white quad below.
    let sprites = vec![
        DrawSprite { texture: w, position: Vec2::new(-80.0, 0.0), size: Vec2::splat(60.0), rotation: 0.0, color: pack_color(230, 60, 60, 255), layer: 0, uv_min: Vec2::ZERO, uv_max: Vec2::ONE },
        DrawSprite { texture: w, position: Vec2::new(0.0, 0.0), size: Vec2::splat(60.0), rotation: 0.0, color: pack_color(60, 200, 90, 255), layer: 0, uv_min: Vec2::ZERO, uv_max: Vec2::ONE },
        DrawSprite { texture: w, position: Vec2::new(80.0, 0.0), size: Vec2::splat(60.0), rotation: 0.0, color: pack_color(70, 120, 240, 255), layer: 0, uv_min: Vec2::ZERO, uv_max: Vec2::ONE },
        DrawSprite { texture: w, position: Vec2::new(0.0, -80.0), size: Vec2::splat(50.0), rotation: 0.6, color: pack_color(245, 245, 245, 255), layer: 0, uv_min: Vec2::ZERO, uv_max: Vec2::ONE },
    ];

    let mut camera = toile_graphics::camera::Camera2D::new(width as f32, height as f32);
    camera.position = Vec2::ZERO;
    camera.zoom = 1.0;

    let stats = h.render(&camera, &sprites, Color::new(0.06, 0.06, 0.1, 1.0));
    h.save_png(&out)?;
    Ok(format!(
        "smoke OK on '{}': {} sprites, {} draw calls -> {}",
        h.gpu.adapter_name,
        stats.sprite_count,
        stats.draw_calls,
        out.display()
    ))
}

fn render_scene(
    scene_path: PathBuf,
    out: Option<PathBuf>,
    width: u32,
    height: u32,
    zoom: Option<f32>,
    camera: Option<String>,
    assets: Option<PathBuf>,
) -> Result<String, String> {
    let scene = toile_scene::load_scene(&scene_path).map_err(|e| format!("load scene: {e}"))?;

    let assets_root = assets.or_else(|| scene_path.parent().map(|p| p.to_path_buf()));
    let camera_pos = match camera {
        Some(s) => Some(parse_vec2(&s)?),
        None => None,
    };
    let opts = SnapshotOptions {
        zoom,
        camera: camera_pos,
        assets_root,
        margin: 0.1,
    };

    let mut h = Harness::new(width, height)?;
    let stats = h.render_scene(&scene, &opts);

    let out = out.unwrap_or_else(|| scene_path.with_extension("png"));
    h.save_png(&out)?;
    Ok(format!(
        "scene '{}' ({} entities, {} drawn) -> {}",
        scene.name,
        scene.entities.len(),
        stats.sprite_count,
        out.display()
    ))
}

fn parse_vec2(s: &str) -> Result<Vec2, String> {
    let (x, y) = s
        .split_once(',')
        .ok_or_else(|| format!("expected 'x,y', got '{s}'"))?;
    Ok(Vec2::new(
        x.trim().parse().map_err(|_| format!("bad x in '{s}'"))?,
        y.trim().parse().map_err(|_| format!("bad y in '{s}'"))?,
    ))
}
