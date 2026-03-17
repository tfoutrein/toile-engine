//! Three-pass asset classification: extension → path → heuristics.

use crate::types::{AssetType, ScannedFile};

/// Classify a scanned file into an asset type.
pub fn classify(file: &ScannedFile) -> AssetType {
    // Pass 1: Extension-based
    let by_ext = classify_by_extension(&file.extension);
    if by_ext != AssetType::Unknown {
        // Pass 2: Path-based refinement for images (ambiguous between sprite/tileset/bg/ui/icon)
        if matches!(by_ext, AssetType::Sprite) {
            return classify_image_by_path(&file.path);
        }
        return by_ext;
    }

    AssetType::Unknown
}

/// Classify based on file extension.
fn classify_by_extension(ext: &str) -> AssetType {
    match ext {
        // Images → default to Sprite (refined by path)
        "png" | "jpg" | "jpeg" | "bmp" | "webp" => AssetType::Sprite,

        // Aseprite
        "aseprite" | "ase" => AssetType::Sprite,

        // Tiled maps
        "tmx" | "tmj" => AssetType::Tilemap,

        // Tiled tilesets
        "tsx" | "tsj" => AssetType::Tileset,

        // LDtk
        "ldtk" | "ldtkl" => AssetType::Tilemap,

        // Audio
        "wav" | "ogg" | "mp3" | "flac" | "opus" => AssetType::Audio,
        "xm" | "mod" | "it" => AssetType::Audio,

        // Fonts
        "ttf" | "otf" | "woff2" => AssetType::Font,
        "fnt" => AssetType::Font,

        // Skeleton animation
        "skel" | "spine" => AssetType::Skeleton,
        "scml" | "scon" => AssetType::Skeleton,

        // Data
        "json" | "xml" | "plist" => classify_structured_data(ext),

        // Text
        "txt" | "md" | "toml" | "yaml" | "yml" => AssetType::Data,
        "license" | "readme" | "credits" => AssetType::Data,

        _ => AssetType::Unknown,
    }
}

/// For JSON/XML/plist, we can't determine the type without reading content.
/// Default to Data; the library can refine later if needed.
fn classify_structured_data(_ext: &str) -> AssetType {
    AssetType::Data
}

/// Classify an image file based on its path (parent directories, filename).
fn classify_image_by_path(path: &str) -> AssetType {
    let lower = path.to_lowercase();

    // Check directory names
    let path_patterns = [
        // Tileset patterns
        (&["tile/", "tileset/", "terrain/", "tiles/"][..], AssetType::Tileset),
        (&["map/", "level/", "world/"], AssetType::Tilemap),
        (&["bg/", "background/", "parallax/", "backdrop/"], AssetType::Background),
        (&["ui/", "gui/", "hud/", "menu/", "interface/"], AssetType::Gui),
        (&["icon/", "icons/", "item/", "items/", "inventory/"], AssetType::Icon),
        (&["fx/", "vfx/", "effect/", "effects/", "particle/", "particles/"], AssetType::Vfx),
        (&["font/", "fonts/"], AssetType::Font),
        (&["prop/", "props/", "object/", "objects/", "decoration/"], AssetType::Prop),
        (&["character/", "characters/", "player/", "enemy/", "enemies/", "npc/"], AssetType::Sprite),
    ];

    for (patterns, asset_type) in &path_patterns {
        for pattern in *patterns {
            if lower.contains(pattern) {
                return *asset_type;
            }
        }
    }

    // Check filename patterns
    if lower.contains("tileset") || lower.contains("tilesheet") {
        return AssetType::Tileset;
    }
    if lower.contains("background") || lower.contains("_bg") || lower.ends_with("bg.png") {
        return AssetType::Background;
    }
    if lower.contains("preview") || lower.contains("thumbnail") || lower.contains("screenshot") {
        return AssetType::Data;
    }

    // Default: Sprite (most common for images)
    AssetType::Sprite
}

/// Generate tags from the file path.
pub fn tags_from_path(path: &str) -> Vec<String> {
    let lower = path.to_lowercase();
    let mut tags = Vec::new();

    let tag_keywords = [
        "player", "enemy", "npc", "boss",
        "idle", "walk", "run", "jump", "attack", "die", "hurt",
        "terrain", "ground", "wall", "platform",
        "coin", "gem", "key", "chest", "door",
        "fire", "water", "ice", "forest", "dungeon", "cave",
        "medieval", "sci-fi", "pixel", "retro",
    ];

    for kw in &tag_keywords {
        if lower.contains(kw) {
            tags.push(kw.to_string());
        }
    }

    tags
}

/// Detect the subtype string from file context.
pub fn detect_subtype(file: &ScannedFile, asset_type: AssetType) -> String {
    match asset_type {
        AssetType::Sprite => {
            match file.extension.as_str() {
                "aseprite" | "ase" => "aseprite_native".into(),
                "png" | "jpg" => {
                    if file.path.to_lowercase().contains("strip") || file.path.to_lowercase().contains("sheet") {
                        "spritesheet_strip".into()
                    } else {
                        "spritesheet_grid".into()
                    }
                }
                _ => "unknown".into(),
            }
        }
        AssetType::Tilemap => {
            match file.extension.as_str() {
                "tmx" => "tiled_tmx".into(),
                "tmj" => "tiled_json".into(),
                "ldtk" | "ldtkl" => "ldtk".into(),
                _ => "unknown".into(),
            }
        }
        AssetType::Tileset => {
            match file.extension.as_str() {
                "tsx" => "tiled_tsx".into(),
                "tsj" => "tiled_tsj".into(),
                _ => "grid".into(),
            }
        }
        AssetType::Audio => {
            let category = if file.path.to_lowercase().contains("music") { "music" }
                else if file.path.to_lowercase().contains("ambient") { "ambient" }
                else { "sfx" };
            category.into()
        }
        AssetType::Font => {
            match file.extension.as_str() {
                "ttf" | "otf" => "vector".into(),
                "fnt" => "bmfont".into(),
                _ => "unknown".into(),
            }
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file(path: &str, ext: &str) -> ScannedFile {
        ScannedFile { path: path.into(), extension: ext.into(), size_bytes: 1000 }
    }

    #[test]
    fn classify_by_ext() {
        assert_eq!(classify(&make_file("test.wav", "wav")), AssetType::Audio);
        assert_eq!(classify(&make_file("test.ttf", "ttf")), AssetType::Font);
        assert_eq!(classify(&make_file("test.tmj", "tmj")), AssetType::Tilemap);
        assert_eq!(classify(&make_file("test.ldtk", "ldtk")), AssetType::Tilemap);
    }

    #[test]
    fn classify_image_by_path_patterns() {
        assert_eq!(classify(&make_file("tiles/ground.png", "png")), AssetType::Tileset);
        assert_eq!(classify(&make_file("background/sky.png", "png")), AssetType::Background);
        assert_eq!(classify(&make_file("ui/button.png", "png")), AssetType::Gui);
        assert_eq!(classify(&make_file("characters/hero/idle.png", "png")), AssetType::Sprite);
    }

    #[test]
    fn tags_extraction() {
        let tags = tags_from_path("characters/player/idle_walk.png");
        assert!(tags.contains(&"player".to_string()));
        assert!(tags.contains(&"idle".to_string()));
        assert!(tags.contains(&"walk".to_string()));
    }
}
