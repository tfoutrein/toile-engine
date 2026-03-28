//! Input Map panel — visualize gamepads, configure action bindings.

use crate::editor_app::EditorApp;
use toile_app::platform::{GamepadButton, GamepadAxis, GamepadType};

impl EditorApp {
    pub(crate) fn show_input_map_panel(&mut self, ctx: &egui::Context) {
        if !self.show_input_map { return; }

        let mut open = true;
        egui::Window::new("Input Map")
            .open(&mut open)
            .default_width(520.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                // ── Connected controllers ──
                ui.label(egui::RichText::new("Connected Controllers").strong().size(14.0));
                ui.separator();

                if self.gamepad_snapshot.is_empty() {
                    ui.label(egui::RichText::new("No gamepad connected.")
                        .color(egui::Color32::from_gray(130)));
                } else {
                    for (player_idx, state) in &self.gamepad_snapshot {
                        let type_str = match state.gamepad_type {
                            GamepadType::Xbox => "Xbox",
                            GamepadType::PlayStation => "PlayStation",
                            GamepadType::SwitchPro => "Switch Pro",
                            GamepadType::Generic => "Generic",
                        };

                        egui::CollapsingHeader::new(
                            egui::RichText::new(format!("P{}: {} ({})", player_idx, state.name, type_str))
                                .color(egui::Color32::from_rgb(80, 220, 120)).size(12.0)
                        )
                        .default_open(true)
                        .id_salt(format!("gp_{}", player_idx))
                        .show(ui, |ui| {
                            // Button grid
                            ui.horizontal_wrapped(|ui| {
                                let buttons: &[(&str, GamepadButton)] = &[
                                    ("A", GamepadButton::South), ("B", GamepadButton::East),
                                    ("X", GamepadButton::West), ("Y", GamepadButton::North),
                                    ("LB", GamepadButton::LeftShoulder), ("RB", GamepadButton::RightShoulder),
                                    ("LT", GamepadButton::LeftTrigger), ("RT", GamepadButton::RightTrigger),
                                    ("Up", GamepadButton::DPadUp), ("Dn", GamepadButton::DPadDown),
                                    ("Lt", GamepadButton::DPadLeft), ("Rt", GamepadButton::DPadRight),
                                    ("Sel", GamepadButton::Select), ("Sta", GamepadButton::Start),
                                    ("L3", GamepadButton::LeftStick), ("R3", GamepadButton::RightStick),
                                ];
                                for (label, btn) in buttons {
                                    let pressed = state.buttons_down.contains(btn);
                                    let (fg, bg) = if pressed {
                                        (egui::Color32::from_rgb(0, 255, 120), egui::Color32::from_rgb(0, 80, 40))
                                    } else {
                                        (egui::Color32::from_gray(100), egui::Color32::from_gray(40))
                                    };
                                    egui::Frame::NONE.fill(bg)
                                        .inner_margin(egui::Margin::symmetric(5, 2))
                                        .corner_radius(3.0)
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new(*label).size(10.0).color(fg).monospace());
                                        });
                                }
                            });

                            // Axis bars
                            ui.add_space(4.0);
                            let axes: &[(&str, GamepadAxis)] = &[
                                ("L Stick X", GamepadAxis::LeftStickX), ("L Stick Y", GamepadAxis::LeftStickY),
                                ("R Stick X", GamepadAxis::RightStickX), ("R Stick Y", GamepadAxis::RightStickY),
                                ("L Trigger", GamepadAxis::LeftTrigger), ("R Trigger", GamepadAxis::RightTrigger),
                            ];
                            egui::Grid::new(format!("axes_{}", player_idx)).num_columns(3).spacing([6.0, 2.0]).show(ui, |ui| {
                                for (label, axis) in axes {
                                    let val = state.axes.get(axis).copied().unwrap_or(0.0);
                                    ui.label(egui::RichText::new(*label).size(10.0).color(egui::Color32::from_gray(160)));
                                    let bar_color = if val.abs() > 0.05 { egui::Color32::from_rgb(80, 200, 120) } else { egui::Color32::from_gray(50) };
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(100.0, 10.0), egui::Sense::hover());
                                    ui.painter().rect_filled(rect, 2.0, egui::Color32::from_gray(30));
                                    let is_trigger = *axis == GamepadAxis::LeftTrigger || *axis == GamepadAxis::RightTrigger;
                                    if is_trigger {
                                        let w = rect.width() * val.clamp(0.0, 1.0);
                                        let fill = egui::Rect::from_min_size(rect.left_top(), egui::vec2(w, rect.height()));
                                        ui.painter().rect_filled(fill, 2.0, bar_color);
                                    } else {
                                        let center = rect.center().x;
                                        let offset = val * rect.width() / 2.0;
                                        let x1 = center.min(center + offset);
                                        let x2 = center.max(center + offset);
                                        let fill = egui::Rect::from_x_y_ranges(x1..=x2, rect.y_range());
                                        ui.painter().rect_filled(fill, 2.0, bar_color);
                                    }
                                    ui.label(egui::RichText::new(format!("{:.2}", val)).size(9.0).color(egui::Color32::from_gray(130)).monospace());
                                    ui.end_row();
                                }
                            });
                        });
                    }
                    ctx.request_repaint();
                }

                ui.add_space(12.0);

                // ── Action bindings ──
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Action Bindings").strong().size(14.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Save").on_hover_text("Save input_map.json").clicked() {
                            self.input_map_save_requested = true;
                        }
                    });
                });
                ui.separator();

                // Listening indicator
                let listening_name = self.input_map_listening.clone();
                if let Some(action_name) = &listening_name {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(egui::RichText::new(format!("Press any key/button for '{}'...", action_name))
                            .color(egui::Color32::from_rgb(255, 200, 80)));
                        if ui.small_button("Cancel").clicked() {
                            self.input_map_listening = None;
                        }
                    });
                    ui.add_space(4.0);
                    ctx.request_repaint();
                }

                // Collect what to remove (can't mutate while iterating)
                let mut remove_binding: Option<(String, usize)> = None;
                let mut remove_action: Option<String> = None;
                let mut listen_for: Option<String> = None;

                let snapshot = self.actions_bindings_snapshot.clone();
                let states = self.actions_snapshot.clone();

                for (i, (name, type_str, bindings)) in snapshot.iter().enumerate() {
                    let (_, _, pressed, value, vec2) = states.get(i)
                        .cloned()
                        .unwrap_or_default();

                    let header_color = if pressed {
                        egui::Color32::from_rgb(80, 255, 120)
                    } else {
                        egui::Color32::from_gray(190)
                    };

                    let status = match type_str.as_str() {
                        "Vec2" => format!("({:.1}, {:.1})", vec2[0], vec2[1]),
                        "Axis" => format!("{:.2}", value),
                        _ => if pressed { "ACTIVE".into() } else { String::new() },
                    };

                    egui::CollapsingHeader::new(
                        egui::RichText::new(format!("{} [{}] {}", name, type_str, status))
                            .color(header_color).size(12.0)
                    )
                    .id_salt(format!("action_{}", name))
                    .default_open(false)
                    .show(ui, |ui| {
                        for (j, binding_str) in bindings.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                                ui.label(egui::RichText::new(binding_str).size(11.0).color(egui::Color32::from_gray(170)));
                                if ui.small_button("x").on_hover_text("Remove this binding").clicked() {
                                    remove_binding = Some((name.clone(), j));
                                }
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            if ui.small_button("+ Add binding").on_hover_text("Press any key or button to add").clicked() {
                                listen_for = Some(name.clone());
                            }
                            if ui.small_button("Delete action").on_hover_text("Remove this action entirely").clicked() {
                                remove_action = Some(name.clone());
                            }
                        });
                    });
                }

                // Apply deferred mutations
                if let Some((name, idx)) = remove_binding {
                    self.input_map_pending_remove_binding = Some((name, idx));
                }
                if let Some(name) = remove_action {
                    self.input_map_pending_remove_action = Some(name);
                }
                if let Some(name) = listen_for {
                    self.input_map_listening = Some(name);
                }

                // Add new action
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("+ Add Action").clicked() {
                        // Create a new empty Button action with a unique name
                        let n = snapshot.len();
                        let new_name = format!("action_{}", n);
                        self.input_map_pending_add_action = Some(toile_app::platform::input_actions::InputAction {
                            name: new_name,
                            action_type: toile_app::ActionType::Button,
                            bindings: vec![],
                        });
                    }
                });
            });

        if !open {
            self.show_input_map = false;
            self.input_map_listening = None;
        }
    }
}
