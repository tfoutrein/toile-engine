//! Toile.toml manifest parsing.

use std::path::Path;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ToileToml {
    pub project: ProjectSection,
    #[serde(default)]
    pub game: GameSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub template: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameSection {
    #[serde(default = "default_entry_scene")]
    pub entry_scene: String,
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    #[serde(default = "default_window_height")]
    pub window_height: u32,
    #[serde(default)]
    pub window_title: Option<String>,
}

fn default_version() -> String { "0.1.0".into() }
fn default_entry_scene() -> String { "scenes/main.json".into() }
fn default_window_width() -> u32 { 1280 }
fn default_window_height() -> u32 { 720 }

impl Default for GameSection {
    fn default() -> Self {
        Self {
            entry_scene: default_entry_scene(),
            window_width: default_window_width(),
            window_height: default_window_height(),
            window_title: None,
        }
    }
}

/// Parsed project manifest with resolved paths.
#[derive(Debug, Clone)]
pub struct ProjectManifest {
    pub name: String,
    pub version: String,
    pub entry_scene: String,
    pub window_width: u32,
    pub window_height: u32,
    pub window_title: String,
}

impl ProjectManifest {
    pub fn load(project_dir: &Path) -> Result<Self, String> {
        let toml_path = project_dir.join("Toile.toml");
        let content = std::fs::read_to_string(&toml_path)
            .map_err(|e| format!("Cannot read {}: {e}", toml_path.display()))?;
        let parsed: ToileToml = toml_dep::from_str(&content)
            .map_err(|e| format!("Invalid Toile.toml: {e}"))?;

        let title = parsed.game.window_title
            .unwrap_or_else(|| parsed.project.name.clone());

        Ok(Self {
            name: parsed.project.name,
            version: parsed.project.version,
            entry_scene: parsed.game.entry_scene,
            window_width: parsed.game.window_width,
            window_height: parsed.game.window_height,
            window_title: title,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_toml() {
        let toml_str = r#"
[project]
name = "Test Game"
"#;
        let parsed: ToileToml = toml_dep::from_str(toml_str).unwrap();
        assert_eq!(parsed.project.name, "Test Game");
        assert_eq!(parsed.game.entry_scene, "scenes/main.json");
        assert_eq!(parsed.game.window_width, 1280);
    }

    #[test]
    fn parse_full_toml() {
        let toml_str = r#"
[project]
name = "My Platformer"
version = "1.0.0"
template = "platformer"

[game]
entry_scene = "scenes/level1.json"
window_width = 800
window_height = 600
window_title = "My Awesome Game"
"#;
        let parsed: ToileToml = toml_dep::from_str(toml_str).unwrap();
        assert_eq!(parsed.project.name, "My Platformer");
        assert_eq!(parsed.game.entry_scene, "scenes/level1.json");
        assert_eq!(parsed.game.window_width, 800);
        assert_eq!(parsed.game.window_title.as_deref(), Some("My Awesome Game"));
    }
}
