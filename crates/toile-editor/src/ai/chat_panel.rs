//! Chat panel UI — renders the AI copilot as a right-side panel alongside the viewport.

use crate::editor_app::{EditorApp, EditorMode};
use crate::ai::client::{ChatMessage, ToolCall};

impl EditorApp {
    /// Render the AI copilot as a right-side panel (viewport stays visible).
    pub(crate) fn show_ai_copilot(&mut self, ctx: &egui::Context) {
        // Settings window (modal, above everything)
        if self.ai_show_settings {
            let mut open = true;
            egui::Window::new("AI Settings")
                .open(&mut open)
                .default_width(420.0)
                .show(ctx, |ui| {
                    use crate::ai::config::AiProvider;

                    // ── Provider selection ──
                    ui.label(egui::RichText::new("Provider").strong());
                    ui.separator();

                    let prev_provider = self.ai_config.provider.clone();
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.ai_config.provider, AiProvider::Anthropic, "Anthropic (Claude)");
                        ui.radio_value(&mut self.ai_config.provider, AiProvider::OpenaiCompat, "OpenAI-compatible");
                    });
                    if self.ai_config.provider != prev_provider {
                        self.ai_available_models.clear();
                        self.ai_models_loaded = false;
                    }

                    ui.add_space(4.0);

                    // Helper: render model ComboBox + refresh button
                    let render_model_combo = |ui: &mut egui::Ui, model: &mut String, models: &[crate::ai::config::ModelInfo], salt: &str| -> bool {
                        let mut refresh_clicked = false;
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt(salt)
                                .selected_text(model.as_str())
                                .width(240.0)
                                .show_ui(ui, |ui| {
                                    for m in models {
                                        let label = if m.name != m.id {
                                            format!("{} ({})", m.name, m.id)
                                        } else {
                                            m.id.clone()
                                        };
                                        ui.selectable_value(model, m.id.clone(), label);
                                    }
                                    if models.is_empty() {
                                        ui.label(egui::RichText::new("Set API key, then click Refresh")
                                            .color(egui::Color32::from_gray(130)).size(11.0));
                                    }
                                });
                            if ui.small_button("Refresh").on_hover_text("Fetch available models").clicked() {
                                refresh_clicked = true;
                            }
                        });
                        refresh_clicked
                    };

                    match self.ai_config.provider {
                        AiProvider::Anthropic => {
                            egui::Grid::new("anthropic_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                                ui.label("API Key:");
                                let key_resp = ui.add(egui::TextEdit::singleline(&mut self.ai_config.api_key)
                                    .password(true)
                                    .hint_text("sk-ant-...")
                                    .desired_width(280.0));
                                // Auto-fetch models when key is pasted (lost focus after edit)
                                if key_resp.lost_focus() && !self.ai_config.api_key.is_empty() && !self.ai_models_loaded {
                                    if let Ok(models) = crate::ai::config::fetch_models(&self.ai_config.api_key) {
                                        self.ai_available_models = models;
                                        self.ai_models_loaded = true;
                                    }
                                }
                                ui.end_row();

                                ui.label("Model:");
                                let refresh = render_model_combo(ui, &mut self.ai_config.model, &self.ai_available_models, "anthropic_model");
                                if refresh && !self.ai_config.api_key.is_empty() {
                                    match crate::ai::config::fetch_models(&self.ai_config.api_key) {
                                        Ok(models) => {
                                            self.ai_available_models = models;
                                            self.ai_models_loaded = true;
                                        }
                                        Err(e) => { self.status_msg = format!("Failed: {e}"); }
                                    }
                                }
                                ui.end_row();
                            });
                        }
                        AiProvider::OpenaiCompat => {
                            // Presets
                            ui.horizontal(|ui| {
                                ui.label("Preset:");
                                if ui.small_button("Scaleway").clicked() {
                                    self.ai_config.openai_base_url = "https://api.scaleway.ai/v1".into();
                                    self.ai_config.openai_model.clear();
                                    self.ai_models_loaded = false;
                                    self.ai_available_models.clear();
                                }
                                if ui.small_button("OpenAI").clicked() {
                                    self.ai_config.openai_base_url = "https://api.openai.com/v1".into();
                                    self.ai_config.openai_model.clear();
                                    self.ai_models_loaded = false;
                                    self.ai_available_models.clear();
                                }
                                if ui.small_button("Groq").clicked() {
                                    self.ai_config.openai_base_url = "https://api.groq.com/openai/v1".into();
                                    self.ai_config.openai_model.clear();
                                    self.ai_models_loaded = false;
                                    self.ai_available_models.clear();
                                }
                                if ui.small_button("Ollama").clicked() {
                                    self.ai_config.openai_base_url = "http://localhost:11434/v1".into();
                                    self.ai_config.openai_model.clear();
                                    self.ai_models_loaded = false;
                                    self.ai_available_models.clear();
                                }
                            });

                            ui.add_space(4.0);

                            egui::Grid::new("openai_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                                ui.label("Base URL:");
                                ui.add(egui::TextEdit::singleline(&mut self.ai_config.openai_base_url)
                                    .hint_text("https://api.scaleway.ai/v1")
                                    .desired_width(280.0));
                                ui.end_row();

                                ui.label("API Key:");
                                let key_resp = ui.add(egui::TextEdit::singleline(&mut self.ai_config.openai_api_key)
                                    .password(true)
                                    .hint_text("API key or token")
                                    .desired_width(280.0));
                                // Auto-fetch models when key is pasted
                                if key_resp.lost_focus() && !self.ai_config.openai_api_key.is_empty() && !self.ai_models_loaded {
                                    if let Ok(models) = crate::ai::config::fetch_openai_models(
                                        &self.ai_config.openai_base_url,
                                        &self.ai_config.openai_api_key,
                                    ) {
                                        self.ai_available_models = models;
                                        self.ai_models_loaded = true;
                                    }
                                }
                                ui.end_row();

                                ui.label("Model:");
                                let refresh = render_model_combo(ui, &mut self.ai_config.openai_model, &self.ai_available_models, "openai_model");
                                if refresh && !self.ai_config.openai_api_key.is_empty() {
                                    match crate::ai::config::fetch_openai_models(
                                        &self.ai_config.openai_base_url,
                                        &self.ai_config.openai_api_key,
                                    ) {
                                        Ok(models) => {
                                            self.ai_available_models = models;
                                            self.ai_models_loaded = true;
                                        }
                                        Err(e) => { self.status_msg = format!("Failed: {e}"); }
                                    }
                                }
                                ui.end_row();
                            });
                        }
                    }

                    // ── System prompt ──
                    ui.add_space(8.0);
                    ui.label("Custom system prompt:");
                    ui.add(egui::TextEdit::multiline(&mut self.ai_config.custom_system_prompt)
                        .hint_text("Additional instructions...")
                        .desired_rows(3)
                        .desired_width(380.0));

                    // ── Bug Reporting ──
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Bug Reporting").strong());
                    ui.separator();

                    ui.checkbox(&mut self.ai_config.auto_report_bugs, "Auto-report engine bugs to GitHub");
                    if self.ai_config.auto_report_bugs {
                        ui.horizontal(|ui| {
                            ui.label("GitHub repo:");
                            ui.add(egui::TextEdit::singleline(&mut self.ai_config.github_repo)
                                .hint_text("owner/repo")
                                .desired_width(200.0));
                        });
                        ui.label(egui::RichText::new("Requires gh CLI installed and authenticated").size(10.0).color(egui::Color32::from_gray(130)));
                    }

                    ui.add_space(8.0);
                    if ui.button("Save").clicked() {
                        self.ai_config.save();
                        self.status_msg = "AI settings saved".into();
                    }
                });
            if !open { self.ai_show_settings = false; }
        }

        // Right panel — chat
        egui::SidePanel::right("ai_chat_panel")
            .min_width(350.0)
            .default_width(400.0)
            .show(ctx, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🤖 AI Copilot").strong().size(14.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✖").on_hover_text("Close AI panel").clicked() {
                            self.editor_mode = EditorMode::Entity;
                        }
                        if ui.small_button("⚙").on_hover_text("Settings").clicked() {
                            self.ai_show_settings = !self.ai_show_settings;
                            if self.ai_show_settings && !self.ai_models_loaded && self.ai_config.is_configured()
                                && self.ai_config.provider == crate::ai::config::AiProvider::Anthropic {
                                if let Ok(models) = crate::ai::config::fetch_models(&self.ai_config.api_key) {
                                    self.ai_available_models = models;
                                    self.ai_models_loaded = true;
                                }
                            }
                        }
                        if ui.small_button("🗑").on_hover_text("Clear chat").clicked() {
                            self.ai_messages.clear();
                        }
                    });
                });

                // Status
                if self.ai_config.is_configured() {
                    let provider_label = match self.ai_config.provider {
                        crate::ai::config::AiProvider::Anthropic => "Anthropic",
                        crate::ai::config::AiProvider::OpenaiCompat => "OpenAI",
                    };
                    let model_short = self.ai_config.active_model().split('-').take(2).collect::<Vec<_>>().join("-");
                    ui.label(egui::RichText::new(format!("{provider_label}: {model_short}")).size(10.0).color(egui::Color32::from_rgb(80, 200, 80)));
                } else {
                    ui.label(egui::RichText::new("⚠ Click ⚙ to configure API key").size(10.0).color(egui::Color32::YELLOW));
                }
                ui.separator();

                // Not configured
                if !self.ai_config.is_configured() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label("Set your API key in ⚙ Settings.");
                    });
                    return;
                }

                // Chat area — takes all space except input at bottom
                let input_height = 40.0;
                let avail_height = ui.available_height() - input_height - 8.0;

                // Messages
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .max_height(avail_height)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());

                        if self.ai_messages.is_empty() {
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new("Type below to start.").color(egui::Color32::from_gray(130)));
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("Try:").size(11.0).color(egui::Color32::from_gray(120)));
                            for suggestion in &[
                                "\"Crée un joueur platformer et un sol\"",
                                "\"Ajoute 5 pièces en arc de cercle\"",
                                "\"Liste les entités de la scène\"",
                            ] {
                                ui.label(egui::RichText::new(*suggestion).size(11.0).color(egui::Color32::from_gray(140)).italics());
                            }
                        }

                        for msg in &self.ai_messages {
                            let is_user = msg.role == "user";
                            let bg = if is_user {
                                egui::Color32::from_rgba_unmultiplied(40, 60, 90, 80)
                            } else {
                                egui::Color32::from_rgba_unmultiplied(50, 50, 60, 60)
                            };

                            egui::Frame::NONE
                                .fill(bg)
                                .inner_margin(egui::Margin::same(6))
                                .corner_radius(4.0)
                                .show(ui, |ui| {
                                    let label = if is_user { "You" } else { "Claude" };
                                    let color = if is_user { egui::Color32::from_rgb(100, 180, 255) } else { egui::Color32::from_rgb(180, 140, 255) };
                                    ui.label(egui::RichText::new(label).strong().color(color).size(11.0));

                                    if !msg.content.is_empty() {
                                        if is_user {
                                            ui.label(egui::RichText::new(&msg.content).size(12.0));
                                        } else {
                                            // Render assistant messages as markdown
                                            let id = format!("ai_msg_{}", ui.id().value());
                                            egui_commonmark::CommonMarkViewer::new()
                                                .show(ui, &mut self.ai_md_cache, &msg.content);
                                        }
                                    }

                                    for tc in &msg.tool_calls {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(egui::RichText::new(format!("🔧 {}", tc.name)).size(10.0).color(egui::Color32::from_rgb(255, 200, 80)));
                                            if let Some(ref result) = tc.result {
                                                let short = if result.len() > 60 { format!("{}...", &result[..57]) } else { result.clone() };
                                                ui.label(egui::RichText::new(short).size(9.0).color(egui::Color32::from_gray(130)));
                                            }
                                        });
                                    }
                                });
                            ui.add_space(3.0);
                        }

                        if self.ai_loading {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(egui::RichText::new("Thinking...").size(11.0).color(egui::Color32::from_gray(150)));
                            });
                            ctx.request_repaint();
                        }
                    });

                // Input — fixed at bottom of the panel
                ui.add_space(4.0);
                let mut send = false;
                ui.horizontal(|ui| {
                    let response = ui.add_sized(
                        [ui.available_width() - 50.0, 28.0],
                        egui::TextEdit::singleline(&mut self.ai_input)
                            .hint_text("Message Claude...")
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        send = true;
                    }
                    if ui.add_enabled(!self.ai_loading && !self.ai_input.is_empty(), egui::Button::new("▶")).clicked() {
                        send = true;
                    }
                });

                if send && !self.ai_input.is_empty() && !self.ai_loading {
                    let user_msg = self.ai_input.clone();
                    self.ai_input.clear();
                    self.send_ai_message(user_msg);
                }
            });

        // Check for API response
        self.check_ai_response();
    }

    /// Send a message to Claude in a background thread.
    fn send_ai_message(&mut self, user_message: String) {
        self.ai_messages.push(ChatMessage {
            role: "user".into(),
            content: user_message,
            tool_calls: vec![],
        });

        self.ai_loading = true;

        let config = self.ai_config.clone();
        let messages = self.ai_messages.clone();
        let system_prompt = crate::ai::client::build_system_prompt(
            &config,
            &self.scene.name,
            self.scene.entities.len(),
            (self.scene.settings.viewport_width, self.scene.settings.viewport_height),
        );

        let (tx, rx) = std::sync::mpsc::channel();
        self.ai_response_rx = Some(rx);

        std::thread::spawn(move || {
            let result = crate::ai::client::call_api(&config, &messages, &system_prompt);
            let _ = tx.send(result);
        });
    }

    /// Check if the background API call finished.
    fn check_ai_response(&mut self) {
        if let Some(ref rx) = self.ai_response_rx {
            if let Ok(result) = rx.try_recv() {
                self.ai_loading = false;
                self.ai_response_rx = None;

                match result {
                    Ok(response) => {
                        if !response.tool_calls.is_empty() {
                            let mut tool_calls: Vec<ToolCall> = response.tool_calls;
                            for tc in &mut tool_calls {
                                let result = if tc.name == "get_game_logs" {
                                    let last_n = tc.input.get("last_n").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                                    let total = self.game_logs.len();
                                    let start = total.saturating_sub(last_n);
                                    let lines: Vec<&str> = self.game_logs[start..].iter().map(|s| s.as_str()).collect();
                                    serde_json::json!({
                                        "total_lines": total,
                                        "showing_last": lines.len(),
                                        "logs": lines,
                                    }).to_string()
                                } else if tc.name == "report_bug" {
                                    if !self.ai_config.auto_report_bugs {
                                        serde_json::json!({
                                            "error": "Bug reporting is disabled. The user can enable it in AI Settings > Auto-report bugs to GitHub."
                                        }).to_string()
                                    } else {
                                        let severity = tc.input.get("severity").and_then(|v| v.as_str()).unwrap_or("bug");
                                        let title = tc.input.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled bug");
                                        let description = tc.input.get("description").and_then(|v| v.as_str()).unwrap_or("");
                                        let component = tc.input.get("component").and_then(|v| v.as_str()).unwrap_or("other");
                                        let logs: Vec<String> = tc.input.get("logs")
                                            .and_then(|v| v.as_array())
                                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                                            .unwrap_or_default();
                                        match self.bug_reporter.report(
                                            &self.ai_config.github_repo,
                                            severity, title, description, component, &logs,
                                        ) {
                                            Ok(url) => serde_json::json!({"reported": true, "url": url}).to_string(),
                                            Err(e) => serde_json::json!({"reported": false, "error": e}).to_string(),
                                        }
                                    }
                                } else if tc.name == "search_assets" {
                                    let query = tc.input.get("query").and_then(|v| v.as_str()).unwrap_or("");
                                    let type_filter = tc.input.get("asset_type").and_then(|v| v.as_str());

                                    let results = if let Some(tf) = type_filter {
                                        let at = match tf {
                                            "sprite" => toile_asset_library::types::AssetType::Sprite,
                                            "tileset" => toile_asset_library::types::AssetType::Tileset,
                                            "background" => toile_asset_library::types::AssetType::Background,
                                            "gui" => toile_asset_library::types::AssetType::Gui,
                                            "icon" => toile_asset_library::types::AssetType::Icon,
                                            "vfx" => toile_asset_library::types::AssetType::Vfx,
                                            "prop" => toile_asset_library::types::AssetType::Prop,
                                            "audio" => toile_asset_library::types::AssetType::Audio,
                                            _ => toile_asset_library::types::AssetType::Sprite,
                                        };
                                        self.asset_browser.library.search_typed(query, at)
                                    } else {
                                        self.asset_browser.library.search(query)
                                    };

                                    let items: Vec<serde_json::Value> = results.iter().take(20).map(|a| {
                                        let mut item = serde_json::json!({
                                            "id": a.id,
                                            "name": a.name,
                                            "type": a.asset_type.label(),
                                            "subtype": a.subtype,
                                            "path": a.path,
                                            "tags": a.tags,
                                        });
                                        if let toile_asset_library::types::AssetMetadata::Sprite(ref sm) = a.metadata {
                                            item["frame_size"] = serde_json::json!(format!("{}x{}", sm.frame_width, sm.frame_height));
                                            item["frame_count"] = serde_json::json!(sm.frame_count);
                                            item["animations"] = serde_json::json!(
                                                sm.animations.iter().map(|an| {
                                                    serde_json::json!({"name": an.name, "frames": an.frames.len(), "fps": an.fps})
                                                }).collect::<Vec<_>>()
                                            );
                                        }
                                        item
                                    }).collect();

                                    serde_json::json!({
                                        "results": items,
                                        "total_matches": results.len(),
                                        "showing": items.len(),
                                    }).to_string()

                                } else if tc.name == "get_asset_details" {
                                    let asset_id = tc.input.get("asset_id").and_then(|v| v.as_str()).unwrap_or("");
                                    if let Some(asset) = self.asset_browser.library.by_id(asset_id) {
                                        let abs_path = self.asset_browser.library.absolute_path(asset)
                                            .map(|p| p.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        let mut detail = serde_json::json!({
                                            "id": asset.id,
                                            "name": asset.name,
                                            "type": asset.asset_type.label(),
                                            "subtype": asset.subtype,
                                            "path": asset.path,
                                            "absolute_path": abs_path,
                                            "pack_id": asset.pack_id,
                                            "tags": asset.tags,
                                        });
                                        if let toile_asset_library::types::AssetMetadata::Sprite(ref sm) = asset.metadata {
                                            detail["sprite"] = serde_json::json!({
                                                "frame_width": sm.frame_width,
                                                "frame_height": sm.frame_height,
                                                "frame_count": sm.frame_count,
                                                "columns": sm.columns,
                                                "rows": sm.rows,
                                                "source_format": sm.source_format,
                                                "animations": sm.animations.iter().map(|an| {
                                                    serde_json::json!({
                                                        "name": an.name,
                                                        "frames": an.frames,
                                                        "fps": an.fps,
                                                        "looping": an.looping,
                                                    })
                                                }).collect::<Vec<_>>(),
                                            });
                                        }
                                        detail.to_string()
                                    } else {
                                        serde_json::json!({"error": format!("Asset '{}' not found", asset_id)}).to_string()
                                    }

                                } else if tc.name == "add_entity_animation" {
                                    let eid = tc.input.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let asset_id = tc.input.get("asset_id").and_then(|v| v.as_str()).unwrap_or("");
                                    let anim_name = tc.input.get("animation_name").and_then(|v| v.as_str()).unwrap_or("idle");
                                    let fps = tc.input.get("fps").and_then(|v| v.as_f64()).unwrap_or(8.0) as f32;
                                    let looping = tc.input.get("looping").and_then(|v| v.as_bool()).unwrap_or(true);
                                    let set_default = tc.input.get("set_as_default").and_then(|v| v.as_bool()).unwrap_or(false);

                                    // Get asset info
                                    let asset_info = self.asset_browser.library.by_id(asset_id).map(|asset| {
                                        let abs = self.asset_browser.library.absolute_path(asset)
                                            .map(|p| p.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        let rel = if let Some(ref pd) = self.project_dir {
                                            let pd_str = pd.to_string_lossy();
                                            if abs.starts_with(pd_str.as_ref()) {
                                                abs[pd_str.len()..].trim_start_matches('/').to_string()
                                            } else { abs.clone() }
                                        } else { abs.clone() };
                                        (rel, abs)
                                    });

                                    if let Some((rel_path, abs_path)) = asset_info {
                                        // Detect frame count from image
                                        let frame_count = if let Ok((w, h)) = image::image_dimensions(&abs_path) {
                                            if w > h && h > 0 { w / h } else { 1 }
                                        } else { 1 };

                                        if let Some(entity) = self.scene.find_entity_mut(eid) {
                                            // Set base sprite_path if not yet set
                                            if entity.sprite_path.is_empty() {
                                                entity.sprite_path = rel_path.clone();
                                            }

                                            // Remove existing animation with same name
                                            entity.animations.retain(|a| a.name != anim_name);

                                            // Add strip animation
                                            entity.animations.push(toile_scene::AnimationData {
                                                name: anim_name.to_string(),
                                                frames: (0..frame_count).collect(),
                                                fps,
                                                looping,
                                                sprite_file: Some(rel_path.clone()),
                                                strip_frames: Some(frame_count),
                                            });

                                            if set_default || entity.default_animation.is_none() {
                                                entity.default_animation = Some(anim_name.to_string());
                                            }

                                            serde_json::json!({
                                                "added": true,
                                                "entity_id": eid,
                                                "animation": anim_name,
                                                "sprite_file": rel_path,
                                                "frames": frame_count,
                                                "fps": fps,
                                                "looping": looping,
                                                "is_default": set_default || entity.default_animation.as_deref() == Some(anim_name),
                                                "total_animations": entity.animations.len(),
                                            }).to_string()
                                        } else {
                                            serde_json::json!({"error": format!("Entity {} not found", eid)}).to_string()
                                        }
                                    } else {
                                        serde_json::json!({"error": format!("Asset '{}' not found in library", asset_id)}).to_string()
                                    }

                                } else if tc.name == "set_entity_sprite" {
                                    let eid = tc.input.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let asset_id = tc.input.get("asset_id").and_then(|v| v.as_str()).unwrap_or("");
                                    let default_anim = tc.input.get("default_animation").and_then(|v| v.as_str());

                                    // Look up asset and extract data before mutating scene
                                    let asset_data = self.asset_browser.library.by_id(asset_id).map(|asset| {
                                        let abs_path = self.asset_browser.library.absolute_path(asset)
                                            .map(|p| p.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        // Make path relative to project dir if possible
                                        let rel_path = if let Some(ref pd) = self.project_dir {
                                            let pd_str = pd.to_string_lossy();
                                            if abs_path.starts_with(pd_str.as_ref()) {
                                                abs_path[pd_str.len()..].trim_start_matches('/').to_string()
                                            } else {
                                                abs_path.clone()
                                            }
                                        } else {
                                            abs_path.clone()
                                        };
                                        (asset.clone(), rel_path)
                                    });

                                    if let Some((asset, sprite_path)) = asset_data {
                                        if let Some(entity) = self.scene.find_entity_mut(eid) {
                                            entity.sprite_path = sprite_path.clone();

                                            // Configure sprite_sheet from metadata
                                            if let toile_asset_library::types::AssetMetadata::Sprite(ref sm) = asset.metadata {
                                                if sm.frame_count > 1 {
                                                    entity.sprite_sheet = Some(toile_scene::SpriteSheetData {
                                                        frame_width: sm.frame_width,
                                                        frame_height: sm.frame_height,
                                                        columns: sm.columns,
                                                        rows: sm.rows,
                                                    });
                                                    entity.width = sm.frame_width as f32;
                                                    entity.height = sm.frame_height as f32;
                                                } else {
                                                    entity.sprite_sheet = None;
                                                    entity.width = sm.frame_width as f32;
                                                    entity.height = sm.frame_height as f32;
                                                }

                                                // Copy animations from metadata, or auto-generate from frames
                                                entity.animations.clear();

                                                if sm.animations.is_empty() && sm.frame_count > 1 {
                                                    // No pre-defined animations — infer name from filename
                                                    let anim_name = {
                                                        let fname = asset.name.to_lowercase();
                                                        let known = ["idle", "walk", "run", "jump", "attack", "die", "hurt", "dash", "fall", "climb", "swim", "cast", "shoot"];
                                                        known.iter()
                                                            .find(|k| fname.contains(*k))
                                                            .map(|s| s.to_string())
                                                            .unwrap_or_else(|| "idle".to_string())
                                                    };
                                                    let frames: Vec<u32> = (0..sm.frame_count).collect();
                                                    entity.animations.push(toile_scene::AnimationData {
                                                        name: anim_name,
                                                        frames,
                                                        fps: 8.0,
                                                        looping: true,
                                                        sprite_file: None,
                                                        strip_frames: None,
                                                    });
                                                } else {
                                                    for anim_def in &sm.animations {
                                                        entity.animations.push(toile_scene::AnimationData {
                                                            name: anim_def.name.clone(),
                                                            frames: anim_def.frames.clone(),
                                                            fps: anim_def.fps,
                                                            looping: anim_def.looping,
                                                            sprite_file: None,
                                                            strip_frames: None,
                                                        });
                                                    }
                                                }

                                                // Set default animation
                                                let chosen_default = default_anim
                                                    .map(String::from)
                                                    .or_else(|| entity.animations.iter().find(|a| a.name == "idle").map(|a| a.name.clone()))
                                                    .or_else(|| entity.animations.first().map(|a| a.name.clone()));
                                                entity.default_animation = chosen_default.clone();

                                                let anim_names: Vec<String> = entity.animations.iter().map(|a| {
                                                    format!("{} ({}f, {}fps)", a.name, a.frames.len(), a.fps)
                                                }).collect();

                                                serde_json::json!({
                                                    "assigned": true,
                                                    "entity_id": eid,
                                                    "sprite_path": sprite_path,
                                                    "frame_size": format!("{}x{}", sm.frame_width, sm.frame_height),
                                                    "animations": anim_names,
                                                    "default_animation": chosen_default,
                                                }).to_string()
                                            } else {
                                                // No SpriteMetadata — try to detect from image file
                                                let abs_path = self.asset_browser.library.absolute_path(&asset)
                                                    .unwrap_or_default();
                                                let detected = if abs_path.exists() {
                                                    image::image_dimensions(&abs_path).ok()
                                                } else {
                                                    None
                                                };

                                                if let Some((img_w, img_h)) = detected {
                                                    // Detect sprite grid from image dimensions
                                                    let (fw, fh, cols, rows) = toile_asset_library::heuristics::detect_sprite_grid(img_w, img_h);
                                                    let frame_count = cols * rows;

                                                    if frame_count > 1 {
                                                        entity.sprite_sheet = Some(toile_scene::SpriteSheetData {
                                                            frame_width: fw,
                                                            frame_height: fh,
                                                            columns: cols,
                                                            rows,
                                                        });
                                                        entity.width = fw as f32;
                                                        entity.height = fh as f32;

                                                        // Auto-generate animation
                                                        let anim_name = {
                                                            let fname = asset.name.to_lowercase();
                                                            let known = ["idle", "walk", "run", "jump", "attack", "die", "hurt", "dash", "fall"];
                                                            known.iter().find(|k| fname.contains(*k))
                                                                .map(|s| s.to_string())
                                                                .unwrap_or_else(|| "idle".to_string())
                                                        };
                                                        entity.animations.clear();
                                                        entity.animations.push(toile_scene::AnimationData {
                                                            name: anim_name.clone(),
                                                            frames: (0..frame_count).collect(),
                                                            fps: 8.0,
                                                            looping: true,
                                                            sprite_file: None,
                                                            strip_frames: None,
                                                        });
                                                        entity.default_animation = Some(anim_name.clone());

                                                        serde_json::json!({
                                                            "assigned": true,
                                                            "entity_id": eid,
                                                            "sprite_path": sprite_path,
                                                            "frame_size": format!("{}x{}", fw, fh),
                                                            "grid": format!("{}x{} ({} frames)", cols, rows, frame_count),
                                                            "animations": [format!("{} ({}f, 8fps)", anim_name, frame_count)],
                                                            "default_animation": anim_name,
                                                            "note": "Grid auto-detected from image dimensions"
                                                        }).to_string()
                                                    } else {
                                                        // Single frame
                                                        entity.sprite_sheet = None;
                                                        entity.width = img_w as f32;
                                                        entity.height = img_h as f32;
                                                        entity.animations.clear();
                                                        entity.default_animation = None;
                                                        serde_json::json!({
                                                            "assigned": true,
                                                            "entity_id": eid,
                                                            "sprite_path": sprite_path,
                                                            "size": format!("{}x{}", img_w, img_h),
                                                            "note": "Static sprite (single frame)"
                                                        }).to_string()
                                                    }
                                                } else {
                                                    // Can't read image — just set path
                                                    entity.sprite_sheet = None;
                                                    entity.animations.clear();
                                                    entity.default_animation = None;
                                                    serde_json::json!({
                                                        "assigned": true,
                                                        "entity_id": eid,
                                                        "sprite_path": sprite_path,
                                                        "note": "Sprite path set (could not read image for auto-detection)"
                                                    }).to_string()
                                                }
                                            }
                                        } else {
                                            serde_json::json!({"error": format!("Entity {} not found", eid)}).to_string()
                                        }
                                    } else {
                                        serde_json::json!({"error": format!("Asset '{}' not found in library", asset_id)}).to_string()
                                    }

                                } else {
                                    crate::ai::tools::execute_tool_with_dir(&mut self.scene, &tc.name, &tc.input, self.project_dir.as_deref())
                                };
                                tc.result = Some(result);
                            }

                            self.ai_messages.push(ChatMessage {
                                role: "assistant".into(),
                                content: response.text.clone(),
                                tool_calls: tool_calls.clone(),
                            });

                            self.sprite_cache.clear();

                            if response.stop_reason == "tool_use" {
                                let config = self.ai_config.clone();
                                let messages = self.ai_messages.clone();
                                let system_prompt = crate::ai::client::build_system_prompt(
                                    &config,
                                    &self.scene.name,
                                    self.scene.entities.len(),
                                    (self.scene.settings.viewport_width, self.scene.settings.viewport_height),
                                );

                                self.ai_loading = true;
                                let (tx, rx) = std::sync::mpsc::channel();
                                self.ai_response_rx = Some(rx);

                                std::thread::spawn(move || {
                                    let result = crate::ai::client::call_api(&config, &messages, &system_prompt);
                                    let _ = tx.send(result);
                                });
                            }
                        } else {
                            self.ai_messages.push(ChatMessage {
                                role: "assistant".into(),
                                content: response.text,
                                tool_calls: vec![],
                            });
                        }
                    }
                    Err(e) => {
                        self.ai_messages.push(ChatMessage {
                            role: "assistant".into(),
                            content: format!("❌ Error: {e}"),
                            tool_calls: vec![],
                        });
                    }
                }
            }
        }
    }
}
