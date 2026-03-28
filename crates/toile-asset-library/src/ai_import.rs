//! AI-powered asset import analysis.
//!
//! Collects pack context (README, file tree, metadata) and sends it to an AI
//! provider to produce an ImportPlan that overrides heuristic classification.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

// ── Pack Context (collected before AI call) ─────────────────────────────────

/// All the context from a pack that the AI needs to produce an ImportPlan.
#[derive(Debug, Clone)]
pub struct PackContext {
    pub pack_name: String,
    pub readme_contents: Vec<(String, String)>,
    pub file_tree: Vec<FileEntry>,
    pub metadata_files: Vec<(String, String)>,
    pub extension_counts: HashMap<String, usize>,
    pub total_files: usize,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size_bytes: u64,
    pub image_dims: Option<(u32, u32)>,
}

/// Scan a pack directory and collect all context for AI analysis.
pub fn collect_pack_context(pack_dir: &Path) -> PackContext {
    let pack_name = pack_dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());

    let mut readme_contents = Vec::new();
    let mut file_tree = Vec::new();
    let mut metadata_files = Vec::new();
    let mut extension_counts: HashMap<String, usize> = HashMap::new();

    let entries: Vec<walkdir::DirEntry> = walkdir::WalkDir::new(pack_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .take(2000)
        .collect();

    let total_files = entries.len();

    for entry in &entries {
        let rel_path = entry.path().strip_prefix(pack_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // Skip hidden files
        if rel_path.starts_with('.') || rel_path.contains("/.") {
            continue;
        }

        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let ext = entry.path().extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let name_lower = entry.file_name().to_string_lossy().to_lowercase();

        // Count extensions
        if !ext.is_empty() {
            *extension_counts.entry(ext.clone()).or_insert(0) += 1;
        }

        // Collect image dimensions
        let image_dims = if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "webp") {
            image::image_dimensions(entry.path()).ok()
        } else {
            None
        };

        file_tree.push(FileEntry {
            path: rel_path.clone(),
            size_bytes: size,
            image_dims,
        });

        // Collect README-like files
        if name_lower.contains("readme") || name_lower.contains("license")
            || name_lower.contains("credits") || name_lower.contains("changelog")
            || (ext == "md" && size < 50_000)
            || (ext == "txt" && size < 50_000 && (name_lower.contains("read") || name_lower.contains("info")))
        {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let truncated = if content.len() > 4000 {
                    format!("{}...\n[truncated at 4000 chars]", &content[..4000])
                } else {
                    content
                };
                readme_contents.push((rel_path.clone(), truncated));
            }
        }

        // Collect metadata files (JSON, XML, FNT, PLIST)
        if matches!(ext.as_str(), "json" | "xml" | "fnt" | "plist" | "toml" | "yaml" | "yml")
            && size < 100_000
        {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let preview = if content.len() > 800 {
                    format!("{}...", &content[..800])
                } else {
                    content
                };
                metadata_files.push((rel_path.clone(), preview));
            }
        }
    }

    PackContext {
        pack_name,
        readme_contents,
        file_tree,
        metadata_files,
        extension_counts,
        total_files,
    }
}

// ── Import Plan (AI response) ───────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportPlan {
    #[serde(default)]
    pub pack_description: String,
    #[serde(default)]
    pub animations: Vec<AnimationPlan>,
    #[serde(default)]
    pub classifications: Vec<ClassificationOverride>,
    #[serde(default)]
    pub tags: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationPlan {
    pub file: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
    #[serde(default)]
    pub animations: Vec<crate::types::AnimationDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationOverride {
    pub file: String,
    #[serde(rename = "type")]
    pub asset_type: String,
    #[serde(default)]
    pub tile_width: Option<u32>,
    #[serde(default)]
    pub tile_height: Option<u32>,
}

// ── Prompt building ─────────────────────────────────────────────────────────

const CHUNK_SIZE: usize = 80; // files per AI call

const INSTRUCTIONS: &str = "\
Analyse ces fichiers et produis un Import Plan PARTIEL en JSON (UNIQUEMENT du JSON, pas de texte).\n\
Format exact :\n\
{\n\
  \"pack_description\": \"Description courte\",\n\
  \"animations\": [\n\
    {\n\
      \"file\": \"chemin/relatif/image.png\",\n\
      \"frame_width\": 32,\n\
      \"frame_height\": 32,\n\
      \"columns\": 4,\n\
      \"rows\": 1,\n\
      \"animations\": [\n\
        {\"name\": \"idle\", \"frames\": [0,1,2,3], \"fps\": 8.0, \"looping\": true}\n\
      ]\n\
    }\n\
  ],\n\
  \"classifications\": [\n\
    {\"file\": \"chemin/image.png\", \"type\": \"tileset\", \"tile_width\": 16, \"tile_height\": 16}\n\
  ],\n\
  \"tags\": {\n\
    \"Characters/Knight/\": [\"knight\", \"player\"]\n\
  }\n\
}\n\n\
Regles :\n\
- frame_width et frame_height DOIVENT diviser exactement les dimensions de l'image\n\
- Noms d'animation standards : idle, walk, run, jump, attack, die, hurt, dash, fall, climb, cast, shoot\n\
- Types valides : sprite, tileset, background, gui, icon, vfx, prop\n\
- FPS typiques : idle=6-8, walk=8-10, run=10-12, attack=12-15\n\
- Si le README indique des tailles, utilise-les en priorite\n\
- Si des fichiers se terminent par _01, _02 etc, ce sont des frames separees d'une meme animation\n\
- N'inclus que les fichiers que tu peux analyser avec certitude\n\
- Reponds UNIQUEMENT avec le JSON, pas d'explication";

/// Build the shared context (README + stats) included in every chunk.
fn build_shared_context(ctx: &PackContext) -> String {
    let mut s = String::with_capacity(4000);
    s.push_str(&format!("Pack: \"{}\"\nFichiers: {} total\n", ctx.pack_name, ctx.total_files));

    s.push_str("Extensions: ");
    let mut exts: Vec<_> = ctx.extension_counts.iter().collect();
    exts.sort_by(|a, b| b.1.cmp(a.1));
    for (ext, count) in &exts {
        s.push_str(&format!("{} .{}, ", count, ext));
    }
    s.push_str("\n\n");

    if !ctx.readme_contents.is_empty() {
        s.push_str("=== DOCUMENTATION ===\n");
        for (name, content) in &ctx.readme_contents {
            s.push_str(&format!("--- {} ---\n{}\n\n", name, content));
        }
    }

    if !ctx.metadata_files.is_empty() {
        s.push_str("=== METADATA ===\n");
        for (name, preview) in ctx.metadata_files.iter().take(5) {
            s.push_str(&format!("--- {} ---\n{}\n\n", name, preview));
        }
    }

    s
}

/// Build a prompt for one chunk of files.
fn build_chunk_prompt(shared_context: &str, files: &[&FileEntry], chunk_idx: usize, total_chunks: usize) -> String {
    let mut prompt = String::with_capacity(4000);
    prompt.push_str("Tu es un expert en analyse de packs d'assets pour jeux 2D.\n\n");
    prompt.push_str(shared_context);
    prompt.push_str(&format!("=== FICHIERS (lot {}/{}) ===\n", chunk_idx + 1, total_chunks));
    for entry in files {
        if let Some((w, h)) = entry.image_dims {
            prompt.push_str(&format!("{} ({}x{})\n", entry.path, w, h));
        } else {
            prompt.push_str(&format!("{} ({}B)\n", entry.path, entry.size_bytes));
        }
    }
    prompt.push('\n');
    prompt.push_str("=== INSTRUCTIONS ===\n");
    prompt.push_str(INSTRUCTIONS);
    prompt
}

/// Legacy single-prompt builder for small packs.
pub fn build_analysis_prompt(ctx: &PackContext) -> String {
    let shared = build_shared_context(ctx);
    let files: Vec<&FileEntry> = ctx.file_tree.iter().collect();
    build_chunk_prompt(&shared, &files, 0, 1)
}

// ── Call AI ──────────────────────────────────────────────────────────────────

/// Call the AI to analyze a pack, using chunked strategy for large packs.
/// Returns a merged ImportPlan. Call from a background thread.
pub fn analyze_pack(
    context: &PackContext,
    api_key: &str,
    base_url: &str,
    model: &str,
    use_anthropic: bool,
) -> Result<ImportPlan, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());

    let shared_context = build_shared_context(context);
    let file_refs: Vec<&FileEntry> = context.file_tree.iter().collect();
    let chunks: Vec<&[&FileEntry]> = file_refs.chunks(CHUNK_SIZE).collect();
    let total_chunks = chunks.len();

    log::info!("AI analysis: {} files in {} chunk(s), sending to {}", context.total_files, total_chunks, base_url);

    let mut merged = ImportPlan::default();

    for (i, chunk) in chunks.iter().enumerate() {
        let prompt = build_chunk_prompt(&shared_context, chunk, i, total_chunks);
        log::info!("AI chunk {}/{}: {} files, {} chars", i + 1, total_chunks, chunk.len(), prompt.len());

        let partial = call_ai_single(&client, &prompt, api_key, base_url, model, use_anthropic)
            .map_err(|e| format!("Chunk {}/{} failed: {}", i + 1, total_chunks, e))?;

        // Merge partial into main plan
        if merged.pack_description.is_empty() && !partial.pack_description.is_empty() {
            merged.pack_description = partial.pack_description;
        }
        merged.animations.extend(partial.animations);
        merged.classifications.extend(partial.classifications);
        for (k, v) in partial.tags {
            merged.tags.entry(k).or_default().extend(v);
        }
    }

    log::info!("AI analysis complete: {} animations, {} classifications",
        merged.animations.len(), merged.classifications.len());
    Ok(merged)
}

/// Single AI call with a prompt. Returns a partial ImportPlan.
fn call_ai_single(
    client: &reqwest::blocking::Client,
    prompt: &str,
    api_key: &str,
    base_url: &str,
    model: &str,
    use_anthropic: bool,
) -> Result<ImportPlan, String> {
    let (url, body, auth_header, auth_value) = if use_anthropic {
        let url = format!("{}/v1/messages", base_url);
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 16384,
            "messages": [{"role": "user", "content": prompt}],
        });
        (url, body, "x-api-key".to_string(), api_key.to_string())
    } else {
        let url = format!("{}/chat/completions", base_url);
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 16384,
            "messages": [
                {"role": "system", "content": "Tu es un expert en analyse de packs d'assets pour jeux 2D. Reponds uniquement en JSON valide."},
                {"role": "user", "content": prompt}
            ],
        });
        (url, body, "Authorization".to_string(), format!("Bearer {}", api_key))
    };

    let mut req = client.post(&url)
        .header("content-type", "application/json")
        .header(&auth_header, &auth_value);

    if use_anthropic {
        req = req.header("anthropic-version", "2023-06-01");
    }

    let response = req.json(&body).send()
        .map_err(|e| format!("HTTP error: {} (is_timeout: {})", e, e.is_timeout()))?;

    let status = response.status();
    let text = response.text().map_err(|e| format!("Read error: {e}"))?;

    if !status.is_success() {
        return Err(format!("API error {}: {}", status, &text[..500.min(text.len())]));
    }

    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("JSON parse error: {e}"))?;

    // Extract the response text
    let response_text = if use_anthropic {
        json.get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|b| b.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        let message = json.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"));

        let content = message
            .and_then(|m| m.get("content"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        if content.is_empty() {
            message
                .and_then(|m| m.get("reasoning"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            content.to_string()
        }
    };

    let json_str = extract_json(&response_text);

    serde_json::from_str::<ImportPlan>(&json_str)
        .map_err(|e| {
            let preview_len = 2000.min(response_text.len());
            format!("Failed to parse ImportPlan: {e}\nExtracted JSON:\n{}\nRaw response (first 2000 chars):\n{}",
                &json_str[..500.min(json_str.len())],
                &response_text[..preview_len])
        })
}

/// Extract JSON from a response that may contain markdown code fences,
/// reasoning text ("Thinking Process: ..."), or other wrapper text.
fn extract_json(text: &str) -> String {
    let trimmed = text.trim();

    // Try to find ```json ... ``` block
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim().to_string();
        }
    }

    // Try to find ``` ... ``` block containing JSON
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            let candidate = after[..end].trim();
            if candidate.starts_with('{') {
                return candidate.to_string();
            }
        }
    }

    // Try as-is (maybe it's pure JSON)
    if trimmed.starts_with('{') {
        return trimmed.to_string();
    }

    // Reasoning models put "Thinking Process: ..." before the JSON.
    // Search for `{"pack_description"` which is the required first field.
    if let Some(start) = trimmed.rfind("{\"pack_description\"") {
        // Find the matching closing brace
        let mut depth = 0;
        let bytes = trimmed[start..].as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return trimmed[start..start + i + 1].to_string();
                    }
                }
                _ => {}
            }
        }
    }

    // Also try `{"animations"` as some models skip pack_description
    for marker in &["{\"animations\"", "{\"classifications\"", "{\"tags\""] {
        if let Some(start) = trimmed.rfind(marker) {
            let mut depth = 0;
            let bytes = trimmed[start..].as_bytes();
            for (i, &b) in bytes.iter().enumerate() {
                match b {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            // Validate it parses as ImportPlan before returning
                            let candidate = &trimmed[start..start + i + 1];
                            if serde_json::from_str::<ImportPlan>(candidate).is_ok() {
                                return candidate.to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Last resort: find the largest balanced {...} block from the end
    if let Some(end) = trimmed.rfind('}') {
        let mut depth = 0;
        let bytes = trimmed.as_bytes();
        for i in (0..=end).rev() {
            match bytes[i] {
                b'}' => depth += 1,
                b'{' => {
                    depth -= 1;
                    if depth == 0 {
                        let candidate = &trimmed[i..=end];
                        if candidate.len() > 20 { // skip tiny fragments
                            return candidate.to_string();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    trimmed.to_string()
}

// ── Save / Load cached plan ─────────────────────────────────────────────────

const PLAN_FILENAME: &str = "toile-import-plan.json";

pub fn save_plan(pack_dir: &Path, plan: &ImportPlan) -> Result<(), String> {
    let path = pack_dir.join(PLAN_FILENAME);
    let json = serde_json::to_string_pretty(plan)
        .map_err(|e| format!("Serialize error: {e}"))?;
    std::fs::write(path, json)
        .map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

pub fn load_plan(pack_dir: &Path) -> Option<ImportPlan> {
    let path = pack_dir.join(PLAN_FILENAME);
    if !path.exists() { return None; }
    let json = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}
