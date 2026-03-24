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
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Anthropic API Configuration").strong());
                    ui.separator();

                    egui::Grid::new("ai_settings_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                        ui.label("API Key:");
                        ui.add(egui::TextEdit::singleline(&mut self.ai_config.api_key)
                            .password(true)
                            .hint_text("sk-ant-...")
                            .desired_width(280.0));
                        ui.end_row();

                        ui.label("Model:");
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("ai_model")
                                .selected_text(&self.ai_config.model)
                                .width(220.0)
                                .show_ui(ui, |ui| {
                                    for model in &self.ai_available_models {
                                        let label = if model.name != model.id {
                                            format!("{} ({})", model.name, model.id)
                                        } else {
                                            model.id.clone()
                                        };
                                        ui.selectable_value(&mut self.ai_config.model, model.id.clone(), label);
                                    }
                                    if self.ai_available_models.is_empty() {
                                        ui.label(egui::RichText::new("Click 🔄 to load").color(egui::Color32::from_gray(130)));
                                    }
                                });
                            if ui.small_button("🔄").on_hover_text("Refresh models from API").clicked() {
                                if !self.ai_config.api_key.is_empty() {
                                    match crate::ai::config::fetch_models(&self.ai_config.api_key) {
                                        Ok(models) => {
                                            self.ai_available_models = models;
                                            self.ai_models_loaded = true;
                                        }
                                        Err(e) => { self.status_msg = format!("Failed: {e}"); }
                                    }
                                }
                            }
                        });
                        ui.end_row();
                    });

                    ui.add_space(8.0);
                    ui.label("Custom system prompt:");
                    ui.add(egui::TextEdit::multiline(&mut self.ai_config.custom_system_prompt)
                        .hint_text("Additional instructions...")
                        .desired_rows(3)
                        .desired_width(380.0));

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
                            if self.ai_show_settings && !self.ai_models_loaded && self.ai_config.is_configured() {
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
                    let model_short = self.ai_config.model.split('-').take(2).collect::<Vec<_>>().join("-");
                    ui.label(egui::RichText::new(format!("✅ {model_short}")).size(10.0).color(egui::Color32::from_rgb(80, 200, 80)));
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
                                        ui.label(egui::RichText::new(&msg.content).size(12.0));
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
                                let result = crate::ai::tools::execute_tool_with_dir(&mut self.scene, &tc.name, &tc.input, self.project_dir.as_deref());
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
