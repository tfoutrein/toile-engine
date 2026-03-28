use glam::Vec2;
use toile_app::{GameContext, Key};
use toile_core::particles::{ParticleEmitter, ParticlePool};

use crate::editor_app::{EditorMode, EditorApp, ResizeHandle};
use crate::tilemap_tool::TileTool;

impl EditorApp {
    /// Handle all per-frame update logic: camera, input, selection, drag,
    /// resize, rotate, tilemap painting, and particle ticks.
    pub(crate) fn handle_update(&mut self, ctx: &mut GameContext, _dt: f64) {
        // Splash screen countdown
        if self.show_splash {
            self.splash_timer -= _dt as f32;
            if self.splash_timer <= 0.0 || ctx.input.is_key_just_pressed(Key::Space) || ctx.input.is_key_just_pressed(Key::Escape) {
                self.show_splash = false;
            }
            return;
        }

        // Camera zoom with scroll
        let scroll = ctx.input.scroll_delta();
        if scroll.y != 0.0 {
            self.camera_zoom *= 1.0 + scroll.y * 0.1;
            self.camera_zoom = self.camera_zoom.clamp(0.2, 5.0);
        }

        // Camera pan with middle mouse button drag
        let mouse_pos = ctx.input.mouse_position();
        if ctx.input.is_mouse_down(toile_app::MouseButton::Middle) {
            let delta = mouse_pos - self.last_mouse_pos;
            self.camera_pos.x -= delta.x / self.camera_zoom;
            self.camera_pos.y += delta.y / self.camera_zoom; // y-up
            self.panning = true;
        } else {
            self.panning = false;
        }
        self.last_mouse_pos = mouse_pos;

        ctx.camera.position = self.camera_pos;
        ctx.camera.zoom = self.camera_zoom;

        // In SpriteAnim mode, center camera on selected entity and zoom in
        if self.editor_mode == EditorMode::SpriteAnim {
            if let Some(id) = self.selected_id {
                if let Some(entity) = self.scene.entities.iter().find(|e| e.id == id) {
                    ctx.camera.position = Vec2::new(entity.x, entity.y);
                    // Zoom to show entity nicely (fit ~3x the entity size)
                    let ent_size = entity.width.max(entity.height) * entity.scale_x.max(entity.scale_y);
                    if let Some(ref sheet) = entity.sprite_sheet {
                        let frame_size = sheet.frame_width.max(sheet.frame_height) as f32;
                        let vp = ctx.camera.viewport_size();
                        ctx.camera.zoom = (vp.x.min(vp.y) / (frame_size * 4.0)).max(1.0);
                    } else if ent_size > 0.0 {
                        let vp = ctx.camera.viewport_size();
                        ctx.camera.zoom = (vp.x.min(vp.y) / (ent_size * 4.0)).max(1.0);
                    }
                }
            }
        }

        // Keyboard shortcuts (Cmd on Mac = SuperLeft, Ctrl on PC = ControlLeft)
        let modifier = ctx.input.is_key_down(Key::SuperLeft)
            || ctx.input.is_key_down(Key::SuperRight)
            || ctx.input.is_key_down(Key::ControlLeft)
            || ctx.input.is_key_down(Key::ControlRight);

        if modifier && self.editor_mode == EditorMode::Entity {
            // Cmd+C / Ctrl+C — Copy selected entity
            if ctx.input.is_key_just_pressed(Key::KeyC) {
                if let Some(id) = self.selected_id {
                    if let Some(entity) = self.scene.entities.iter().find(|e| e.id == id) {
                        self.clipboard_entity = Some(entity.clone());
                        self.status_msg = format!("Copied '{}'", entity.name);
                    }
                }
            }

            // Cmd+V / Ctrl+V — Paste entity (offset by 20px)
            if ctx.input.is_key_just_pressed(Key::KeyV) {
                if let Some(ref source) = self.clipboard_entity.clone() {
                    let id = self.scene.next_id;
                    self.scene.next_id += 1;
                    let mut new_entity = source.clone();
                    new_entity.id = id;
                    new_entity.name = format!("{}_copy", source.name);
                    new_entity.x += 20.0;
                    new_entity.y -= 20.0;
                    self.scene.entities.push(new_entity);
                    self.selected_id = Some(id);
                    self.status_msg = format!("Pasted '{}_copy'", source.name);
                }
            }

            // Cmd+D / Ctrl+D — Duplicate selected entity in place
            if ctx.input.is_key_just_pressed(Key::KeyD) {
                if let Some(sel_id) = self.selected_id {
                    if let Some(source) = self.scene.entities.iter().find(|e| e.id == sel_id).cloned() {
                        let id = self.scene.next_id;
                        self.scene.next_id += 1;
                        let mut new_entity = source.clone();
                        new_entity.id = id;
                        new_entity.name = format!("{}_dup", source.name);
                        new_entity.x += 20.0;
                        new_entity.y -= 20.0;
                        self.scene.entities.push(new_entity);
                        self.selected_id = Some(id);
                        self.status_msg = format!("Duplicated '{}'", source.name);
                    }
                }
            }

            // Cmd+S / Ctrl+S — Quick Save
            if ctx.input.is_key_just_pressed(Key::KeyS) {
                if !self.current_file.is_empty() {
                    if let Some(ref dir) = self.project_dir {
                        let path = dir.join(&self.current_file);
                        if let Ok(json) = serde_json::to_string_pretty(&self.scene) {
                            match std::fs::write(&path, &json) {
                                Ok(()) => self.status_msg = format!("Saved {}", self.current_file),
                                Err(e) => self.status_msg = format!("Save failed: {e}"),
                            }
                        }
                    }
                }
            }
        }

        // Delete key — delete selected entity
        if (ctx.input.is_key_just_pressed(Key::Delete) || ctx.input.is_key_just_pressed(Key::Backspace))
            && self.editor_mode == EditorMode::Entity
        {
            if let Some(id) = self.selected_id.take() {
                self.scene.remove_entity(id);
                self.status_msg = format!("Deleted entity {id}");
            }
        }

        // Hover detection — find entity under mouse cursor
        self.hovered_id = None;
        if self.editor_mode == EditorMode::Entity && !self.panning {
            let world_mouse = ctx.camera.screen_to_world(ctx.input.mouse_position());
            // Check entities in reverse order (top-most first)
            for entity in self.scene.entities.iter().rev() {
                let hw = entity.width * entity.scale_x * 0.5;
                let hh = entity.height * entity.scale_y * 0.5;
                let dx = (world_mouse.x - entity.x).abs();
                let dy = (world_mouse.y - entity.y).abs();
                if dx <= hw && dy <= hh {
                    self.hovered_id = Some(entity.id);
                    break;
                }
            }
        }

        // Entity selection, drag, and resize in Entity mode
        if self.editor_mode == EditorMode::Entity {
            let world_pos = ctx.camera.screen_to_world(ctx.input.mouse_position());
            let handle_size = 8.0 / self.camera_zoom; // handle size in world units

            // Start interaction: detect transition from mouse-up to mouse-down
            if ctx.input.is_mouse_down(toile_app::MouseButton::Left)
                && self.dragging.is_none()
                && self.resizing.is_none()
                && !self.rotating
            {
                // First check: are we clicking on a resize handle of the selected entity?
                let mut hit_handle = None;
                if let Some(sel_id) = self.selected_id {
                    if let Some(entity) = self.scene.entities.iter().find(|e| e.id == sel_id) {
                        let hw = entity.width * entity.scale_x * 0.5;
                        let hh = entity.height * entity.scale_y * 0.5;
                        let rot = entity.rotation;
                        let center = Vec2::new(entity.x, entity.y);

                        // Rotate local offset around entity center
                        let rotated = |local: Vec2| -> Vec2 {
                            let (sin, cos) = rot.sin_cos();
                            center + Vec2::new(
                                local.x * cos - local.y * sin,
                                local.x * sin + local.y * cos,
                            )
                        };

                        let handles = [
                            (rotated(Vec2::new(hw, hh)), ResizeHandle::TopRight),
                            (rotated(Vec2::new(hw, -hh)), ResizeHandle::BottomRight),
                            (rotated(Vec2::new(-hw, -hh)), ResizeHandle::BottomLeft),
                            (rotated(Vec2::new(-hw, hh)), ResizeHandle::TopLeft),
                            (rotated(Vec2::new(0.0, hh)), ResizeHandle::Top),
                            (rotated(Vec2::new(0.0, -hh)), ResizeHandle::Bottom),
                            (rotated(Vec2::new(-hw, 0.0)), ResizeHandle::Left),
                            (rotated(Vec2::new(hw, 0.0)), ResizeHandle::Right),
                        ];
                        for (pos, handle) in &handles {
                            if (world_pos - *pos).length() < handle_size * 1.5 {
                                hit_handle = Some(*handle);
                                break;
                            }
                        }
                    }
                }

                // Check rotation handle (diamond above top edge, rotated)
                let mut hit_rotate = false;
                if hit_handle.is_none() {
                    if let Some(sel_id) = self.selected_id {
                        if let Some(entity) = self.scene.entities.iter().find(|e| e.id == sel_id) {
                            let hh = entity.height * entity.scale_y * 0.5;
                            let rot = entity.rotation;
                            let center = Vec2::new(entity.x, entity.y);
                            let local = Vec2::new(0.0, hh + handle_size * 4.0);
                            let (sin, cos) = rot.sin_cos();
                            let rotate_handle_pos = center + Vec2::new(
                                local.x * cos - local.y * sin,
                                local.x * sin + local.y * cos,
                            );
                            if (world_pos - rotate_handle_pos).length() < handle_size * 2.0 {
                                hit_rotate = true;
                            }
                        }
                    }
                }

                if hit_rotate {
                    // Start rotation
                    self.rotating = true;
                    if let Some(sel_id) = self.selected_id {
                        if let Some(entity) = self.scene.entities.iter().find(|e| e.id == sel_id) {
                            self.rotate_start_angle = entity.rotation;
                            let to_mouse = world_pos - Vec2::new(entity.x, entity.y);
                            self.rotate_start_mouse_angle = to_mouse.y.atan2(to_mouse.x);
                        }
                    }
                } else if let Some(handle) = hit_handle {
                    // Start resize
                    self.resizing = Some(handle);
                    self.resize_start_mouse = world_pos;
                    if let Some(sel_id) = self.selected_id {
                        if let Some(entity) = self.scene.entities.iter().find(|e| e.id == sel_id) {
                            self.resize_start_size = Vec2::new(entity.width, entity.height);
                            self.resize_start_pos = Vec2::new(entity.x, entity.y);
                            self.resize_start_rot = entity.rotation;
                        }
                    }
                } else {
                    // Try to pick an entity for drag (rotation-aware hit test)
                    let mut clicked_id = None;
                    for entity in self.scene.entities.iter().rev() {
                        let hw = entity.width * entity.scale_x * 0.5;
                        let hh = entity.height * entity.scale_y * 0.5;
                        // Transform mouse into entity's local space (undo rotation)
                        let d = world_pos - Vec2::new(entity.x, entity.y);
                        let (sin, cos) = (-entity.rotation).sin_cos();
                        let local = Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
                        if local.x >= -hw && local.x <= hw && local.y >= -hh && local.y <= hh {
                            clicked_id = Some(entity.id);
                            break;
                        }
                    }

                    if let Some(id) = clicked_id {
                        self.selected_id = Some(id);
                        if let Some(entity) = self.scene.entities.iter().find(|e| e.id == id) {
                            self.drag_offset =
                                Vec2::new(entity.x - world_pos.x, entity.y - world_pos.y);
                        }
                        self.dragging = Some(id);
                        self.status_msg = format!("Selected entity {id}");
                    } else {
                        self.selected_id = None;
                        self.dragging = Some(u64::MAX); // sentinel
                    }
                }
            }

            // Continue drag
            if ctx.input.is_mouse_down(toile_app::MouseButton::Left) {
                if let Some(drag_id) = self.dragging {
                    if drag_id != u64::MAX {
                        if let Some(entity) = self.scene.find_entity_mut(drag_id) {
                            entity.x = world_pos.x + self.drag_offset.x;
                            entity.y = world_pos.y + self.drag_offset.y;
                        }
                    }
                }

                // Continue resize
                // Transform mouse delta into entity's local space (undo rotation)
                // Default: asymmetric (only the dragged face moves)
                // Shift: symmetric (both faces move, center stays)
                if let Some(handle) = self.resizing {
                    if let Some(sel_id) = self.selected_id {
                        let world_delta = world_pos - self.resize_start_mouse;
                        // Project delta into entity's local axes
                        let rot = self.resize_start_rot;
                        let (sin, cos) = (-rot).sin_cos();
                        let ld = Vec2::new(
                            world_delta.x * cos - world_delta.y * sin,
                            world_delta.x * sin + world_delta.y * cos,
                        );

                        let symmetric = ctx.input.is_key_down(Key::ShiftLeft)
                            || ctx.input.is_key_down(Key::ShiftRight);

                        if let Some(entity) = self.scene.find_entity_mut(sel_id) {
                            let sw = self.resize_start_size.x;
                            let sh = self.resize_start_size.y;
                            let sp = self.resize_start_pos;

                            // Compute size deltas in local space based on handle
                            let (dw, dh) = match handle {
                                ResizeHandle::Right => (ld.x, 0.0),
                                ResizeHandle::Left => (-ld.x, 0.0),
                                ResizeHandle::Top => (0.0, ld.y),
                                ResizeHandle::Bottom => (0.0, -ld.y),
                                ResizeHandle::TopRight => (ld.x, ld.y),
                                ResizeHandle::BottomRight => (ld.x, -ld.y),
                                ResizeHandle::BottomLeft => (-ld.x, -ld.y),
                                ResizeHandle::TopLeft => (-ld.x, ld.y),
                            };

                            if symmetric {
                                entity.width = (sw + dw * 2.0).max(4.0);
                                entity.height = (sh + dh * 2.0).max(4.0);
                                entity.x = sp.x;
                                entity.y = sp.y;
                            } else {
                                entity.width = (sw + dw).max(4.0);
                                entity.height = (sh + dh).max(4.0);

                                // Shift position in world space so the opposite edge stays fixed
                                // Local offset = half the size change along each axis
                                let local_shift = Vec2::new(
                                    match handle {
                                        ResizeHandle::Right | ResizeHandle::TopRight | ResizeHandle::BottomRight => dw * 0.5,
                                        ResizeHandle::Left | ResizeHandle::TopLeft | ResizeHandle::BottomLeft => -dw * 0.5,
                                        _ => 0.0,
                                    },
                                    match handle {
                                        ResizeHandle::Top | ResizeHandle::TopRight | ResizeHandle::TopLeft => dh * 0.5,
                                        ResizeHandle::Bottom | ResizeHandle::BottomRight | ResizeHandle::BottomLeft => -dh * 0.5,
                                        _ => 0.0,
                                    },
                                );
                                // Rotate the local shift back to world space
                                let (sin, cos) = rot.sin_cos();
                                let world_shift = Vec2::new(
                                    local_shift.x * cos - local_shift.y * sin,
                                    local_shift.x * sin + local_shift.y * cos,
                                );
                                entity.x = sp.x + world_shift.x;
                                entity.y = sp.y + world_shift.y;
                            }
                        }
                    }
                }

                // Continue rotation
                if self.rotating {
                    if let Some(sel_id) = self.selected_id {
                        if let Some(entity) = self.scene.find_entity_mut(sel_id) {
                            let to_mouse = world_pos - Vec2::new(entity.x, entity.y);
                            let current_angle = to_mouse.y.atan2(to_mouse.x);
                            let delta_angle = current_angle - self.rotate_start_mouse_angle;
                            entity.rotation = self.rotate_start_angle + delta_angle;

                            // Snap to 15° increments when Shift is held
                            if ctx.input.is_key_down(Key::ShiftLeft)
                                || ctx.input.is_key_down(Key::ShiftRight)
                            {
                                let snap = std::f32::consts::PI / 12.0; // 15°
                                entity.rotation = (entity.rotation / snap).round() * snap;
                            }
                        }
                    }
                }
            }

            // End drag/resize/rotate on mouse release
            if !ctx.input.is_mouse_down(toile_app::MouseButton::Left) {
                self.dragging = None;
                self.resizing = None;
                self.rotating = false;
            }
        }

        // Tilemap painting with mouse
        if self.editor_mode == EditorMode::Tilemap {
            if ctx.input.is_mouse_down(toile_app::MouseButton::Left) {
                let world_pos = ctx.camera.screen_to_world(ctx.input.mouse_position());
                if let Some(tilemap) = &mut self.scene.tilemap {
                    let w = tilemap.width;
                    let h = tilemap.height;
                    if let Some((col, row)) = self.tilemap_editor.world_to_tile(world_pos, w, h) {
                        match self.tilemap_editor.tool {
                            TileTool::Brush => self.tilemap_editor.paint(tilemap, col, row),
                            TileTool::Eraser => self.tilemap_editor.erase(tilemap, col, row),
                            TileTool::Fill => {} // fill on click, not drag
                        }
                    }
                }
            }
            // Fill on single click
            if ctx.input.is_mouse_just_pressed(toile_app::MouseButton::Left) {
                if self.tilemap_editor.tool == TileTool::Fill {
                    let world_pos = ctx.camera.screen_to_world(ctx.input.mouse_position());
                    if let Some(tilemap) = &mut self.scene.tilemap {
                        let w = tilemap.width;
                        let h = tilemap.height;
                        if let Some((col, row)) = self.tilemap_editor.world_to_tile(world_pos, w, h) {
                            self.tilemap_editor.flood_fill(tilemap, col, row);
                        }
                    }
                }
            }
        }

        // Particle simulation tick
        if self.editor_mode == EditorMode::Particle {
            self.particle_editor.update(_dt as f32);
        }

        // Update preview particles for entities with emitters
        if self.editor_mode == EditorMode::Entity {
            let dt_f = _dt as f32;
            // Collect entity ids and their emitter paths + positions
            let mut active: Vec<(u64, String, Vec2)> = Vec::new();
            for e in &self.scene.entities {
                if let Some(ref path) = e.particle_emitter {
                    active.push((e.id, path.clone(), Vec2::new(e.x, e.y)));
                }
            }
            // Remove pools for entities that no longer have emitters
            self.preview_particles.retain(|id, _| active.iter().any(|(eid, _, _)| eid == id));
            self.preview_particle_paths.retain(|id, _| active.iter().any(|(eid, _, _)| eid == id));

            for (eid, path, pos) in &active {
                // Check if pool exists and matches the path
                let needs_reload = match self.preview_particle_paths.get(eid) {
                    Some(existing) => existing != path,
                    None => true,
                };
                if needs_reload {
                    let full = self.project_path(path);
                    if let Ok(json) = std::fs::read_to_string(&full) {
                        if let Ok(emitter) = serde_json::from_str::<ParticleEmitter>(&json) {
                            self.preview_particles.insert(*eid, ParticlePool::new(emitter, *pos));
                            self.preview_particle_paths.insert(*eid, path.clone());
                        }
                    }
                }
                // Update position and tick
                if let Some(pool) = self.preview_particles.get_mut(eid) {
                    pool.position = *pos;
                    pool.update(dt_f);
                }
            }
        }

        // ── Apply pending input map mutations ──
        if let Some((action_name, binding)) = self.input_map_pending_add_binding.take() {
            ctx.actions.add_binding(&action_name, binding);
        }
        if let Some((action_name, idx)) = self.input_map_pending_remove_binding.take() {
            ctx.actions.remove_binding(&action_name, idx);
        }
        if let Some(action) = self.input_map_pending_add_action.take() {
            ctx.actions.add_action(action);
        }
        if let Some(name) = self.input_map_pending_remove_action.take() {
            ctx.actions.remove_action(&name);
        }
        if self.input_map_save_requested {
            self.input_map_save_requested = false;
            if let Some(ref dir) = self.project_dir {
                let path = dir.join("input_map.json");
                if let Err(e) = ctx.actions.save_to_file(&path) {
                    self.status_msg = format!("Failed to save input map: {e}");
                } else {
                    self.status_msg = "Input map saved".into();
                }
            }
        }

        // ── "Press any key/button" capture ──
        if let Some(ref action_name) = self.input_map_listening.clone() {
            if let Some(source) = ctx.input.take_last_pressed_source() {
                let binding = toile_app::platform::input_actions::InputBinding {
                    source,
                    dead_zone: 0.2,
                    composite: None,
                };
                ctx.actions.add_binding(action_name, binding);
                self.input_map_listening = None;
                self.status_msg = format!("Binding added to '{}'", action_name);
            }
        }

        // ── Update gamepad/actions snapshot for UI ──
        if self.show_input_map {
            self.gamepad_snapshot = ctx.input.connected_gamepads()
                .into_iter()
                .map(|(i, s)| (i, s.clone()))
                .collect();

            self.actions_snapshot = ctx.actions.actions.iter().map(|a| {
                let type_str = match a.action_type {
                    toile_app::ActionType::Button => "Button",
                    toile_app::ActionType::Axis => "Axis",
                    toile_app::ActionType::Vec2 => "Vec2",
                };
                let pressed = ctx.actions.is_pressed(&a.name);
                let value = ctx.actions.get_value(&a.name);
                let v2 = ctx.actions.get_vec2(&a.name);
                (a.name.clone(), type_str.to_string(), pressed, value, [v2.x, v2.y])
            }).collect();

            self.actions_bindings_snapshot = ctx.actions.actions.iter().map(|a| {
                let type_str = match a.action_type {
                    toile_app::ActionType::Button => "Button",
                    toile_app::ActionType::Axis => "Axis",
                    toile_app::ActionType::Vec2 => "Vec2",
                };
                let bindings: Vec<String> = a.bindings.iter().map(|b| {
                    let src = match &b.source {
                        toile_app::platform::InputSource::Key { key } => format!("Key: {}", key),
                        toile_app::platform::InputSource::MouseButton { button } => format!("Mouse: {}", button),
                        toile_app::platform::InputSource::GamepadButton { button } => format!("Pad: {}", button),
                        toile_app::platform::InputSource::GamepadAxis { axis } => format!("Axis: {}", axis),
                    };
                    if let Some(ref c) = b.composite {
                        format!("{} ({:?})", src, c)
                    } else {
                        src
                    }
                }).collect();
                (a.name.clone(), type_str.to_string(), bindings)
            }).collect();
        }
    }
}
