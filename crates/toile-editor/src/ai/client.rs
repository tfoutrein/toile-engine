//! Anthropic API client — sends messages to Claude with tool definitions.

use crate::ai::config::AiConfig;
use crate::ai::tools;

/// A message in the conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,       // "user" or "assistant"
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

/// A tool call from Claude's response.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
    pub result: Option<String>,
}

/// Response from the API.
pub struct ApiResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub stop_reason: String,
}

/// Build the system prompt with scene context.
pub fn build_system_prompt(config: &AiConfig, scene_name: &str, entity_count: usize, viewport: (u32, u32)) -> String {
    let mut prompt = format!(
        "Tu es l'assistant IA intégré dans Toile Editor, un éditeur de jeux 2D.\n\n\
        Scène : \"{}\" — {} entités — viewport {}x{}\n\n\
        OUTILS DISPONIBLES :\n\
        - get_scene_info, set_scene_settings : infos et config scène (gravity, camera, viewport)\n\
        - list_entities, create_entity, update_entity, delete_entity : gestion entités\n\
        - add_behavior : ajouter Platform, TopDown, Bullet, Sine, Fade, Wrap, Solid à une entité\n\
        - remove_behavior : retirer un behavior par index\n\
        - set_tags : définir les tags (Player, Solid, Coin, Enemy, Projectile...)\n\
        - set_variables : définir des variables (health, score, ammo...)\n\
        - create_event_sheet : créer des règles de jeu (conditions → actions)\n\
          Conditions: OnKeyPressed, OnCollisionWith, EveryTick, EveryNSeconds, OnCreate, IfVariable\n\
          Actions: Destroy, SpawnObject, SetPosition, MoveAtAngle, SetVariable, AddToVariable, PlaySound, GoToScene, Log\n\
        - save_as_prefab : sauvegarder une entité comme template réutilisable\n\
        - get_game_logs : lire les logs de la dernière session de jeu (Play). Utile pour diagnostiquer les bugs (erreurs, collisions, spawns, etc.)\n\
        - report_bug : signaler un bug dans le MOTEUR ou l'EDITEUR Toile (crée une GitHub Issue automatiquement)\n\n\
        REPORT DE BUGS :\n\
        - report_bug est UNIQUEMENT pour les bugs du moteur/éditeur Toile, PAS pour les erreurs utilisateur\n\
        - Bug moteur = tool call échoue pour raison interne, NaN/crash dans la physique, event sheet valide non exécuté, prefab pas sauvegardé sur disque\n\
        - PAS un bug = tag manquant, prefab pas créé par l'utilisateur, scene vide, mauvaise config\n\
        - En cas de doute, aide l'utilisateur plutôt que de reporter un bug\n\n\
        CONVENTIONS :\n\
        - Coordonnées Y-up (Y positif = haut), (0,0) = centre\n\
        - create_entity avec role= auto-configure behaviors+tags\n\
        - Bullet behavior : se déplace en ligne droite (speed, angle_degrees)\n\
        - Pour faire tirer un joueur : créer un prefab Bullet + event sheet OnKeyPressed Space → SpawnObject\n\
        - Pour collision : les entités doivent avoir des tags, event sheet OnCollisionWith tag → action\n\n\
        DEBUGGING :\n\
        - Quand l'utilisateur dit que ça ne marche pas, utilise get_game_logs pour voir les logs de la dernière session\n\
        - Les logs contiennent les erreurs, avertissements, spawns d'entités, collisions\n\n\
        Sois concis. Exécute immédiatement. Décris brièvement ce que tu as fait.",
        scene_name, entity_count, viewport.0, viewport.1
    );

    if !config.custom_system_prompt.is_empty() {
        prompt.push_str("\n\nInstructions additionnelles :\n");
        prompt.push_str(&config.custom_system_prompt);
    }

    prompt
}

/// Call the Anthropic Messages API (blocking, should be called from a thread).
pub fn call_api(
    config: &AiConfig,
    messages: &[ChatMessage],
    system_prompt: &str,
) -> Result<ApiResponse, String> {
    let client = reqwest::blocking::Client::new();

    // Build messages array
    let mut api_messages = Vec::new();
    for msg in messages {
        if msg.role == "user" {
            api_messages.push(serde_json::json!({
                "role": "user",
                "content": msg.content
            }));
        } else if msg.role == "assistant" {
            if msg.tool_calls.is_empty() {
                api_messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": msg.content
                }));
            } else {
                // Assistant message with tool calls
                let mut content_blocks: Vec<serde_json::Value> = Vec::new();
                if !msg.content.is_empty() {
                    content_blocks.push(serde_json::json!({"type": "text", "text": msg.content}));
                }
                for tc in &msg.tool_calls {
                    content_blocks.push(serde_json::json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.name,
                        "input": tc.input,
                    }));
                }
                api_messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": content_blocks
                }));

                // Tool results
                let mut result_blocks: Vec<serde_json::Value> = Vec::new();
                for tc in &msg.tool_calls {
                    if let Some(ref result) = tc.result {
                        result_blocks.push(serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": tc.id,
                            "content": result,
                        }));
                    }
                }
                if !result_blocks.is_empty() {
                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": result_blocks
                    }));
                }
            }
        }
    }

    let body = serde_json::json!({
        "model": config.model,
        "max_tokens": 4096,
        "system": system_prompt,
        "tools": tools::tool_definitions(),
        "messages": api_messages,
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("HTTP error: {e}"))?;

    let status = response.status();
    let text = response.text().map_err(|e| format!("Read error: {e}"))?;

    if !status.is_success() {
        return Err(format!("API error {}: {}", status, text));
    }

    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("JSON parse error: {e}"))?;

    // Parse response
    let stop_reason = json.get("stop_reason").and_then(|v| v.as_str()).unwrap_or("end_turn").to_string();
    let content = json.get("content").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let mut response_text = String::new();
    let mut tool_calls = Vec::new();

    for block in &content {
        match block.get("type").and_then(|v| v.as_str()) {
            Some("text") => {
                if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                    response_text.push_str(t);
                }
            }
            Some("tool_use") => {
                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let input = block.get("input").cloned().unwrap_or(serde_json::json!({}));
                tool_calls.push(ToolCall { id, name, input, result: None });
            }
            _ => {}
        }
    }

    Ok(ApiResponse { text: response_text, tool_calls, stop_reason })
}
