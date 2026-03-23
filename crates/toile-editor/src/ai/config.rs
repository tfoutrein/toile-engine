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

/// Fetch available models from the Anthropic API.
pub fn fetch_models(api_key: &str) -> Result<Vec<ModelInfo>, String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .map_err(|e| format!("HTTP error: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let json: serde_json::Value = response.json()
        .map_err(|e| format!("JSON error: {e}"))?;

    let mut models = Vec::new();
    if let Some(data) = json.get("data").and_then(|v| v.as_array()) {
        for item in data {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let name = item.get("display_name").and_then(|v| v.as_str())
                .unwrap_or_else(|| item.get("id").and_then(|v| v.as_str()).unwrap_or(""))
                .to_string();
            if !id.is_empty() {
                models.push(ModelInfo { id, name });
            }
        }
    }

    // Sort: newest/most capable first
    models.sort_by(|a, b| b.id.cmp(&a.id));

    Ok(models)
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
}
