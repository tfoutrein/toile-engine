use std::path::PathBuf;

use clap::{Parser, Subcommand};
use toile_scene::{SceneData, load_scene, save_scene};

#[derive(Parser)]
#[command(name = "toile", about = "Toile Engine CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Toile project
    New {
        /// Project name
        name: String,
    },
    /// List entities in a scene file
    ListEntities {
        /// Path to scene JSON file
        scene: PathBuf,
    },
    /// Add an entity to a scene file
    AddEntity {
        /// Path to scene JSON file
        scene: PathBuf,
        /// Entity name
        name: String,
        /// X position
        x: f32,
        /// Y position
        y: f32,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            let dir = PathBuf::from(&name);
            if dir.exists() {
                eprintln!("Error: directory '{}' already exists", name);
                std::process::exit(1);
            }

            std::fs::create_dir_all(dir.join("assets")).unwrap();
            std::fs::create_dir_all(dir.join("scripts")).unwrap();
            std::fs::create_dir_all(dir.join("scenes")).unwrap();

            // Create default scene
            let scene = SceneData::new("main");
            save_scene(&dir.join("scenes/main.json"), &scene).unwrap();

            // Create project manifest
            let toml = format!(
                "[project]\nname = \"{name}\"\nversion = \"0.1.0\"\nengine = \"toile\"\n"
            );
            std::fs::write(dir.join("Toile.toml"), toml).unwrap();

            // Create llms.txt stub
            let llms = format!(
                "# {name}\n\n> A 2D game built with Toile Engine.\n\n\
                ## Scenes\n- scenes/main.json\n\n\
                ## Assets\nPlace sprites in assets/\n\n\
                ## Scripts\nPlace Lua scripts in scripts/\n"
            );
            std::fs::write(dir.join("llms.txt"), llms).unwrap();

            println!("Created Toile project: {name}/");
            println!("  Toile.toml");
            println!("  scenes/main.json");
            println!("  assets/");
            println!("  scripts/");
            println!("  llms.txt");
        }

        Commands::ListEntities { scene } => {
            let data = load_scene(&scene).unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "{:<6} {:<20} {:<10} {:<10} {:<10}",
                "ID", "Name", "X", "Y", "Size"
            );
            println!("{}", "-".repeat(56));
            for e in &data.entities {
                println!(
                    "{:<6} {:<20} {:<10.1} {:<10.1} {}x{}",
                    e.id, e.name, e.x, e.y, e.width, e.height
                );
            }
            println!("\n{} entities in '{}'", data.entities.len(), data.name);
        }

        Commands::AddEntity {
            scene,
            name,
            x,
            y,
        } => {
            let mut data = if scene.exists() {
                load_scene(&scene).unwrap_or_else(|e| {
                    eprintln!("Error loading: {e}");
                    std::process::exit(1);
                })
            } else {
                SceneData::new("untitled")
            };

            let id = data.add_entity(&name, x, y);
            save_scene(&scene, &data).unwrap_or_else(|e| {
                eprintln!("Error saving: {e}");
                std::process::exit(1);
            });

            println!("Added entity '{name}' (id={id}) at ({x}, {y}) to {}", scene.display());
        }
    }
}
