//! AI configuration — API key, model selection, persistence.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

const CONFIG_FILENAME: &str = "toile-ai-config.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    Anthropic,
    OpenaiCompat,
}

impl Default for AiProvider {
    fn default() -> Self { Self::Anthropic }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    // ── Provider selection ──
    #[serde(default)]
    pub provider: AiProvider,

    // ── Anthropic ──
    pub api_key: String,
    pub model: String,

    // ── OpenAI-compatible (Scaleway, etc.) ──
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default = "default_openai_base_url")]
    pub openai_base_url: String,
    #[serde(default = "default_openai_model")]
    pub openai_model: String,

    // ── Shared ──
    pub custom_system_prompt: String,
    #[serde(default = "default_github_repo")]
    pub github_repo: String,
    #[serde(default)]
    pub auto_report_bugs: bool,
}

fn default_openai_base_url() -> String {
    "https://api.scaleway.ai/v1".to_string()
}

fn default_openai_model() -> String {
    "qwen3-32b".to_string()
}

fn default_github_repo() -> String {
    "tfoutrein/toile-engine".to_string()
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::Anthropic,
            api_key: String::new(),
            model: "claude-sonnet-4-20250514".to_string(),
            openai_api_key: String::new(),
            openai_base_url: default_openai_base_url(),
            openai_model: default_openai_model(),
            custom_system_prompt: String::new(),
            github_repo: default_github_repo(),
            auto_report_bugs: false,
        }
    }
}

impl AiConfig {
    pub fn is_configured(&self) -> bool {
        match self.provider {
            AiProvider::Anthropic => !self.api_key.is_empty(),
            AiProvider::OpenaiCompat => !self.openai_api_key.is_empty(),
        }
    }

    /// The active model name for display.
    pub fn active_model(&self) -> &str {
        match self.provider {
            AiProvider::Anthropic => &self.model,
            AiProvider::OpenaiCompat => &self.openai_model,
        }
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

    models.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(models)
}

/// Fetch models from an OpenAI-compatible API (GET /models).
pub fn fetch_openai_models(base_url: &str, api_key: &str) -> Result<Vec<ModelInfo>, String> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/models", base_url.trim_end_matches('/'));

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .map_err(|e| format!("HTTP error: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let json: serde_json::Value = response.json()
        .map_err(|e| format!("JSON error: {e}"))?;

    let mut models = Vec::new();

    // OpenAI format: { "data": [ { "id": "model-name", ... } ] }
    // Some providers return a flat array instead.
    let items = json.get("data").and_then(|v| v.as_array())
        .or_else(|| json.as_array());

    if let Some(data) = items {
        for item in data {
            let id = item.get("id").and_then(|v| v.as_str())
                .or_else(|| item.get("name").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            if id.is_empty() { continue; }
            let owned_by = item.get("owned_by").and_then(|v| v.as_str()).unwrap_or("");
            let name = if owned_by.is_empty() { id.clone() } else { format!("{id} ({owned_by})") };
            models.push(ModelInfo { id, name });
        }
    }

    models.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(models)
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
}
