//! Chat panel UI — renders the AI copilot conversation panel.

use crate::editor_app::{EditorApp, EditorMode};
use crate::ai::client::{ChatMessage, ToolCall};
use crate::ai::config::AVAILABLE_MODELS;

impl EditorApp {
    /// Render the AI copilot panel (full-screen mode).
    pub(crate) fn show_ai_copilot(&mut self, ctx: &egui::Context) {
        // Left panel — settings + conversation history
        egui::SidePanel::left("ai_settings").default_width(120.0).show(ctx, |ui| {
            if ui.button("← Back").clicked() {
                self.editor_mode = EditorMode::Entity;
            }
            ui.separator();
            ui.label(egui::RichText::new("AI Copilot").strong());
            ui.add_space(4.0);

            // Config status
            if self.ai_config.is_configured() {
                ui.label(egui::RichText::new("✅ Connected").color(egui::Color32::from_rgb(80, 220, 80)).size(11.0));
                let model_short = self.ai_config.model.split('-').take(2).collect::<Vec<_>>().join("-");
                ui.label(egui::RichText::new(model_short).size(10.0).color(egui::Color32::from_gray(140)));
            } else {
                ui.label(egui::RichText::new("⚠ Not configured").color(egui::Color32::YELLOW).size(11.0));
            }

            ui.add_space(8.0);
            if ui.button("⚙ Settings").clicked() {
                self.ai_show_settings = !self.ai_show_settings;
            }
            if ui.button("🗑 Clear chat").clicked() {
                self.ai_messages.clear();
            }
        });

        // Settings window (modal)
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
                        egui::ComboBox::from_id_salt("ai_model")
                            .selected_text(&self.ai_config.model)
                            .width(280.0)
                            .show_ui(ui, |ui| {
                                for (id, desc) in AVAILABLE_MODELS {
                                    ui.selectable_value(&mut self.ai_config.model, id.to_string(), format!("{id} — {desc}"));
                                }
                            });
                        ui.end_row();
                    });

                    ui.add_space(8.0);
                    ui.label("Custom system prompt (optional):");
                    ui.add(egui::TextEdit::multiline(&mut self.ai_config.custom_system_prompt)
                        .hint_text("Additional instructions for the AI...")
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

        // Central panel — chat
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.ai_config.is_configured() {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.heading("🤖 AI Copilot");
                    ui.add_space(8.0);
                    ui.label("Configure your Anthropic API key to start.");
                    ui.label("Click ⚙ Settings on the left.");
                });
                return;
            }

            // Message history
            let scroll_area = egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true);

            scroll_area.show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                for msg in &self.ai_messages {
                    let is_user = msg.role == "user";
                    let bg = if is_user {
                        egui::Color32::from_rgba_unmultiplied(40, 60, 90, 80)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(50, 50, 60, 60)
                    };

                    egui::Frame::NONE
                        .fill(bg)
                        .inner_margin(egui::Margin::same(8))
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            let label = if is_user { "You" } else { "Claude" };
                            let color = if is_user { egui::Color32::from_rgb(100, 180, 255) } else { egui::Color32::from_rgb(180, 140, 255) };
                            ui.label(egui::RichText::new(label).strong().color(color).size(12.0));

                            if !msg.content.is_empty() {
                                ui.label(&msg.content);
                            }

                            // Show tool calls
                            for tc in &msg.tool_calls {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("🔧 {}", tc.name)).size(11.0).color(egui::Color32::from_rgb(255, 200, 80)));
                                    if let Some(ref result) = tc.result {
                                        let short = if result.len() > 80 { format!("{}...", &result[..77]) } else { result.clone() };
                                        ui.label(egui::RichText::new(format!("→ {short}")).size(10.0).color(egui::Color32::from_gray(140)));
                                    }
                                });
                            }
                        });
                    ui.add_space(4.0);
                }

                // Loading indicator
                if self.ai_loading {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(egui::RichText::new("Claude is thinking...").color(egui::Color32::from_gray(150)));
                    });
                    ctx.request_repaint();
                }
            });

            ui.separator();

            // Input field
            let mut send = false;
            ui.horizontal(|ui| {
                let response = ui.add_sized(
                    [ui.available_width() - 60.0, 30.0],
                    egui::TextEdit::singleline(&mut self.ai_input)
                        .hint_text("Ask Claude to create or modify the scene...")
                );
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    send = true;
                }
                if ui.add_enabled(!self.ai_loading && !self.ai_input.is_empty(), egui::Button::new("Send")).clicked() {
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
                            // Execute tool calls on the scene
                            let mut tool_calls: Vec<ToolCall> = response.tool_calls;
                            for tc in &mut tool_calls {
                                let result = crate::ai::tools::execute_tool(&mut self.scene, &tc.name, &tc.input);
                                tc.result = Some(result);
                            }

                            // Add assistant message with tool calls
                            self.ai_messages.push(ChatMessage {
                                role: "assistant".into(),
                                content: response.text.clone(),
                                tool_calls: tool_calls.clone(),
                            });

                            // Clear sprite cache to show new entities
                            self.sprite_cache.clear();

                            // If stop_reason is "tool_use", Claude wants to continue — send tool results back
                            if response.stop_reason == "tool_use" {
                                // Auto-continue the conversation with tool results
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
                            // Text-only response
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
