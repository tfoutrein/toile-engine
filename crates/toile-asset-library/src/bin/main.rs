//! Standalone Asset Browser application.
//! Phase 2: will open an egui window for browsing imported asset packs.

fn main() {
    env_logger::init();
    println!("Toile Asset Browser — standalone mode");
    println!("Usage: toile-asset-browser <pack_directory>");

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Please provide a pack directory path.");
        std::process::exit(1);
    }

    let path = std::path::Path::new(&args[1]);
    if !path.is_dir() {
        eprintln!("'{}' is not a directory.", path.display());
        std::process::exit(1);
    }

    let mut library = toile_asset_library::ToileAssetLibrary::new();
    match library.import_pack(path) {
        Ok(count) => {
            println!("Imported {} assets from '{}'", count, path.display());
            println!("\nBy type:");
            for asset_type in &[
                toile_asset_library::AssetType::Sprite,
                toile_asset_library::AssetType::Tileset,
                toile_asset_library::AssetType::Background,
                toile_asset_library::AssetType::Audio,
                toile_asset_library::AssetType::Font,
                toile_asset_library::AssetType::Gui,
                toile_asset_library::AssetType::Icon,
                toile_asset_library::AssetType::Vfx,
                toile_asset_library::AssetType::Prop,
            ] {
                let assets = library.by_type(*asset_type);
                if !assets.is_empty() {
                    println!("  {} {} — {} assets", asset_type.icon(), asset_type.label(), assets.len());
                    for a in assets.iter().take(5) {
                        println!("    {} ({})", a.name, a.path);
                    }
                    if assets.len() > 5 {
                        println!("    ... and {} more", assets.len() - 5);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Import failed: {e}");
            std::process::exit(1);
        }
    }
}
