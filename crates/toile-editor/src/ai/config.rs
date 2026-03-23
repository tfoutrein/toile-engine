//! AI configuration — API key, model selection, persistence.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

const CONFIG_FILENAME: &str = "toile-ai-config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub api_key: String,
    pub model: String,
    pub custom_system_prompt: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "claude-sonnet-4-20250514".to_string(),
            custom_system_prompt: String::new(),
        }
    }
}

impl AiConfig {
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    pub fn config_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            let dir = PathBuf::from(home).join(".toile");
            let _ = std::fs::create_dir_all(&dir);
            dir.join(CONFIG_FILENAME)
        } else {
            PathBuf::from(CONFIG_FILENAME)
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&json) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

pub const AVAILABLE_MODELS: &[(&str, &str)] = &[
    ("claude-sonnet-4-20250514", "Claude Sonnet 4 (fast, recommended)"),
    ("claude-opus-4-20250514", "Claude Opus 4 (powerful)"),
    ("claude-haiku-3-5-20241022", "Claude Haiku 3.5 (fastest, cheapest)"),
];
