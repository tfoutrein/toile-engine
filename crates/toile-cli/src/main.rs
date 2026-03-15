mod templates;

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
        /// Template to use: empty, platformer, topdown, shmup
        #[arg(short, long, default_value = "empty")]
        template: String,
    },
    /// List available project templates
    Templates,
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
        Commands::New { name, template } => {
            if !templates::TEMPLATES.contains(&template.as_str()) {
                eprintln!(
                    "Error: unknown template '{}'. Available: {}",
                    template,
                    templates::TEMPLATES.join(", ")
                );
                std::process::exit(1);
            }

            let dir = PathBuf::from(&name);
            if dir.exists() {
                eprintln!("Error: directory '{}' already exists", name);
                std::process::exit(1);
            }

            match templates::generate(&name, &template, &dir) {
                Ok(files) => {
                    println!("Created Toile project: {name}/ (template: {template})");
                    println!();
                    for f in &files {
                        println!("  {f}");
                    }
                    println!();
                    println!("  assets/");
                    println!();
                    println!("{} files generated.", files.len());
                }
                Err(e) => {
                    eprintln!("Error creating project: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Templates => {
            println!("Available project templates:");
            println!();
            println!("  empty       — Blank scene with camera. The minimum starting point.");
            println!("  platformer  — Side-scrolling platformer with player, enemies, coins, platforms.");
            println!("  topdown     — Top-down game with player, walls, enemies, collectibles.");
            println!("  shmup       — Vertical shoot-em-up with player ship, enemy waves, projectiles.");
            println!();
            println!("Usage: toile new my-game --template platformer");
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
