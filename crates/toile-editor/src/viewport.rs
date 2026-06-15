use glam::Vec2;
use toile_app::{GameContext, Key, Sprite, COLOR_WHITE};
use toile_graphics::sprite_renderer::DrawSprite;
use toile_graphics::sprite_renderer::pack_color;

use toile_behaviors::BehaviorConfig;

use crate::editor_app::{EditorApp, EditorMode, SPLASH_DURATION};

// ── Splash-screen animation helpers ────────────────────────────────────────
fn c01(x: f32) -> f32 { x.clamp(0.0, 1.0) }
fn ease_out_cubic(t: f32) -> f32 { let t = c01(t); 1.0 - (1.0 - t).powi(3) }
fn ease_in_cubic(t: f32) -> f32 { let t = c01(t); t * t * t }

impl EditorApp {
    /// Draw the entire viewport: grid, background tiles, viewport guide,
    /// tilemap layers, entities, selection handles, and particles.
    pub(crate) fn draw_viewport(&mut self, ctx: &mut GameContext) {
        // Splash screen: the whole logo (spiral + "TOILE") is built from sampled pixels that stream
        // in from the LEFT and assemble fluidly (no rotation), shimmer, then stream back off to the
        // left to reveal the app. See the per-particle choreography below.
        if self.show_splash {
            if let Some(white) = self.white_tex {
                let total = SPLASH_DURATION;
                let e = (total - self.splash_timer).max(0.0); // elapsed seconds

                // Dissolve over the last ~0.85s: pixels stream back off + fade.
                let diss = ease_in_cubic((e - 1.95) / 0.85);
                let diss_alpha = 1.0 - diss;

                // ── The WHOLE logo (spiral + "TOILE") forms from fluid pixels streaming IN FROM
                //    THE LEFT — no rotation. Spiral: right-first so the left trail arrives last
                //    (the logo's fanned trail). Text: reveals left→right ("T"…"E"). Then every
                //    pixel is blown OFF TO THE RIGHT on dissolve (a natural rightward scatter). ──
                for p in self.splash_particles.iter() {
                    let xn = ((p.target.x + 128.0) / 256.0).clamp(0.0, 1.0); // 0=left, 1=right
                    let (delay, in_dist, form_dur) = if p.is_text {
                        (0.55 + xn * 0.45 + p.seed * 0.06, 70.0 + p.seed * 50.0, 0.40)
                    } else {
                        (
                            (1.0 - xn) * 0.5 + p.seed * 0.1,
                            110.0 + (1.0 - xn) * 250.0 + p.seed * 60.0,
                            0.55,
                        )
                    };
                    let pf = ease_out_cubic((e - delay) / form_dur);
                    let alpha = c01(pf) * diss_alpha;
                    if alpha <= 0.01 {
                        continue;
                    }
                    // Arrival from the left (trail pixels start furthest → longer streak).
                    let form_off = Vec2::new(
                        -in_dist * (1.0 - pf),
                        (p.seed - 0.5) * 42.0 * (1.0 - pf),
                    );
                    // Dissolve — every pixel is blown OFF TO THE RIGHT + vertical spread, so the
                    // logo scatters naturally rightward to reveal the app.
                    let out = Vec2::new(260.0 + p.seed * 240.0, (p.seed - 0.5) * 150.0)
                        * ease_in_cubic(diss);
                    let settled = pf * (1.0 - diss);
                    let wob = settled * (e * 5.0 + p.seed * 25.0).sin() * 1.1; // alive shimmer
                    let pos = p.target + form_off + out + Vec2::new(0.0, wob);
                    ctx.draw_sprite(Sprite {
                        texture: white,
                        position: pos,
                        size: Vec2::splat(4.5 * (1.0 - 0.3 * diss)),
                        rotation: 0.0,
                        color: pack_color(p.color[0], p.color[1], p.color[2], (alpha * 255.0) as u8),
                        layer: 99,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }
            }
            return;
        }

        let tex = match self.white_tex {
            Some(t) => t,
            None => return,
        };

        // Draw grid — use actual viewport size from camera
        if self.show_grid {
            let grid_size = 32.0;
            let vp = ctx.camera.viewport_size();
            let half_view = Vec2::new(
                vp.x / (2.0 * self.camera_zoom),
                vp.y / (2.0 * self.camera_zoom),
            );
            let min_x = ((self.camera_pos.x - half_view.x) / grid_size).floor() as i32;
            let max_x = ((self.camera_pos.x + half_view.x) / grid_size).ceil() as i32;
            let min_y = ((self.camera_pos.y - half_view.y) / grid_size).floor() as i32;
            let max_y = ((self.camera_pos.y + half_view.y) / grid_size).ceil() as i32;

            let grid_color = pack_color(60, 60, 80, 80);
            for x in min_x..=max_x {
                let wx = x as f32 * grid_size;
                ctx.draw_sprite(Sprite {
                    texture: tex,
                    position: Vec2::new(wx, self.camera_pos.y),
                    size: Vec2::new(1.0 / self.camera_zoom, half_view.y * 2.0),
                    rotation: 0.0,
                    color: grid_color,
                    layer: -10,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
            for y in min_y..=max_y {
                let wy = y as f32 * grid_size;
                ctx.draw_sprite(Sprite {
                    texture: tex,
                    position: Vec2::new(self.camera_pos.x, wy),
                    size: Vec2::new(half_view.x * 2.0, 1.0 / self.camera_zoom),
                    rotation: 0.0,
                    color: grid_color,
                    layer: -10,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }

        // ── Background tiles ─────────────────────────────────────────────
        if let Some(ref bg_path) = self.scene.settings.background_image {
            // Load texture if needed
            if self.background_path_loaded != *bg_path {
                let full = self.project_path(bg_path);
                if full.exists() {
                    self.background_tex = Some(ctx.load_texture(&full));
                } else {
                    self.background_tex = None;
                }
                self.background_path_loaded = bg_path.clone();
            }
            // Ensure at least one tile exists
            if self.scene.settings.background_tiles.is_empty() {
                let cp = self.scene.settings.camera_position;
                self.scene.settings.background_tiles.push(cp);
            }
            let s = &self.scene.settings;
            let tile_w = s.viewport_width as f32 / s.camera_zoom;
            let tile_h = s.viewport_height as f32 / s.camera_zoom;

            if let Some(bg_tex) = self.background_tex {
                // Draw all tiles
                for pos in &s.background_tiles {
                    ctx.draw_sprite(Sprite {
                        texture: bg_tex,
                        position: Vec2::new(pos[0], pos[1]),
                        size: Vec2::new(tile_w, tile_h),
                        rotation: 0.0,
                        color: COLOR_WHITE,
                        layer: -100,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                // Draw "+" buttons on edges of outer tiles
                let btn_size = 16.0 / self.camera_zoom;
                let btn_color = pack_color(80, 200, 80, 200);
                let tiles = s.background_tiles.clone();
                let mut new_tile: Option<[f32; 2]> = None;

                for pos in &tiles {
                    let cx = pos[0];
                    let cy = pos[1];
                    // Check if adjacent positions are already occupied
                    let has_right = tiles.iter().any(|t| (t[0] - (cx + tile_w)).abs() < 1.0 && (t[1] - cy).abs() < 1.0);
                    let has_left  = tiles.iter().any(|t| (t[0] - (cx - tile_w)).abs() < 1.0 && (t[1] - cy).abs() < 1.0);
                    let has_up    = tiles.iter().any(|t| (t[0] - cx).abs() < 1.0 && (t[1] - (cy + tile_h)).abs() < 1.0);
                    let has_down  = tiles.iter().any(|t| (t[0] - cx).abs() < 1.0 && (t[1] - (cy - tile_h)).abs() < 1.0);

                    // Draw "+" sprites on empty edges
                    let candidates = [
                        (!has_right, Vec2::new(cx + tile_w * 0.5, cy), [cx + tile_w, cy]),
                        (!has_left,  Vec2::new(cx - tile_w * 0.5, cy), [cx - tile_w, cy]),
                        (!has_up,    Vec2::new(cx, cy + tile_h * 0.5), [cx, cy + tile_h]),
                        (!has_down,  Vec2::new(cx, cy - tile_h * 0.5), [cx, cy - tile_h]),
                    ];

                    for (show, btn_pos, new_pos) in &candidates {
                        if !show { continue; }
                        // Draw the "+" marker
                        ctx.draw_sprite(Sprite {
                            texture: tex,
                            position: *btn_pos,
                            size: Vec2::splat(btn_size),
                            rotation: 0.0,
                            color: btn_color,
                            layer: 98,
                            uv_min: Vec2::ZERO,
                            uv_max: Vec2::ONE,
                        });
                        // Check click
                        let world_mouse = ctx.camera.screen_to_world(ctx.input.mouse_position());
                        let d = (world_mouse - *btn_pos).abs();
                        if d.x < btn_size && d.y < btn_size
                            && ctx.input.is_mouse_just_pressed(toile_app::MouseButton::Left)
                            && new_tile.is_none()
                        {
                            new_tile = Some(*new_pos);
                        }
                    }
                }

                // Shift + Right-click on a tile to remove it (keep at least one)
                let world_mouse = ctx.camera.screen_to_world(ctx.input.mouse_position());
                let mut remove_tile: Option<usize> = None;
                let shift_held = ctx.input.is_key_down(Key::ShiftLeft) || ctx.input.is_key_down(Key::ShiftRight);
                if ctx.input.is_mouse_just_pressed(toile_app::MouseButton::Right) && shift_held && tiles.len() > 1 {
                    for (i, pos) in tiles.iter().enumerate() {
                        let dx = (world_mouse.x - pos[0]).abs();
                        let dy = (world_mouse.y - pos[1]).abs();
                        if dx < tile_w * 0.5 && dy < tile_h * 0.5 {
                            remove_tile = Some(i);
                            break;
                        }
                    }
                }

                if let Some(idx) = remove_tile {
                    if self.scene.settings.background_tiles.len() > 1 {
                        self.scene.settings.background_tiles.remove(idx);
                        self.auto_update_bounds_from_tiles();
                        self.status_msg = format!("Removed background tile. {} remaining.", self.scene.settings.background_tiles.len());
                    } else {
                        self.status_msg = "Cannot remove last background tile. Use Clear in Scene Settings.".to_string();
                    }
                }
                if let Some(pos) = new_tile {
                    self.scene.settings.background_tiles.push(pos);
                    self.auto_update_bounds_from_tiles();
                }
            }
        } else {
            if !self.background_path_loaded.is_empty() {
                self.background_tex = None;
                self.background_path_loaded.clear();
            }
        }

        // ── Player viewport guide ─────────────────────────────────────────
        // Fixed rectangle representing the game camera view from scene settings.
        if self.show_viewport_guide {
            let s = &self.scene.settings;
            let vp_w = s.viewport_width as f32 / s.camera_zoom;
            let vp_h = s.viewport_height as f32 / s.camera_zoom;
            let vp_cx = s.camera_position[0];
            let vp_cy = s.camera_position[1];

            let thickness = 1.5 / self.camera_zoom;
            let guide_color = pack_color(255, 200, 50, 180);

            // Top
            ctx.draw_sprite(Sprite {
                texture: tex, position: Vec2::new(vp_cx, vp_cy + vp_h * 0.5),
                size: Vec2::new(vp_w + thickness, thickness), rotation: 0.0,
                color: guide_color, layer: 99, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
            // Bottom
            ctx.draw_sprite(Sprite {
                texture: tex, position: Vec2::new(vp_cx, vp_cy - vp_h * 0.5),
                size: Vec2::new(vp_w + thickness, thickness), rotation: 0.0,
                color: guide_color, layer: 99, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
            // Left
            ctx.draw_sprite(Sprite {
                texture: tex, position: Vec2::new(vp_cx - vp_w * 0.5, vp_cy),
                size: Vec2::new(thickness, vp_h), rotation: 0.0,
                color: guide_color, layer: 99, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
            // Right
            ctx.draw_sprite(Sprite {
                texture: tex, position: Vec2::new(vp_cx + vp_w * 0.5, vp_cy),
                size: Vec2::new(thickness, vp_h), rotation: 0.0,
                color: guide_color, layer: 99, uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
            });
        }

        // Draw tilemap layers and entities — skipped in Particle mode
        if self.editor_mode != EditorMode::Particle {

        if let Some(tilemap) = &self.scene.tilemap {
            if let Some(tileset_tex) = self.tilemap_editor.tileset_tex {
                let ts = tilemap.tile_size as f32;
                let map_w = tilemap.width as f32 * ts;
                let map_h = tilemap.height as f32 * ts;
                let offset_x = -map_w * 0.5;
                let offset_y = map_h * 0.5;

                for layer in &tilemap.layers {
                    if !layer.visible {
                        continue;
                    }
                    for row in 0..tilemap.height {
                        for col in 0..tilemap.width {
                            let gid = layer.tiles[(row * tilemap.width + col) as usize];
                            if gid == 0 {
                                continue;
                            }
                            let (uv_min, uv_max) = self.tilemap_editor.tile_uv(gid);
                            let x = offset_x + col as f32 * ts + ts * 0.5;
                            let y = offset_y - (row as f32 * ts + ts * 0.5);
                            ctx.draw_sprite(Sprite {
                                texture: tileset_tex,
                                position: Vec2::new(x, y),
                                size: Vec2::new(ts, ts),
                                rotation: 0.0,
                                color: COLOR_WHITE,
                                layer: -5,
                                uv_min,
                                uv_max,
                            });
                        }
                    }
                }
            }
        }

        // Load sprite textures for entities
        let sprite_paths: Vec<(usize, String)> = self.scene.entities.iter().enumerate()
            .filter(|(_, e)| !e.sprite_path.is_empty() && !self.sprite_cache.contains_key(&e.sprite_path))
            .map(|(i, e)| (i, e.sprite_path.clone()))
            .collect();
        for (_i, path) in sprite_paths {
            let full = self.project_path(&path);
            if full.exists() {
                let handle = ctx.load_texture(&full);
                self.sprite_cache.insert(path, handle);
            }
        }

        // Draw entities
        for entity in &self.scene.entities {
            let selected = self.selected_id == Some(entity.id);
            let hovered = self.hovered_id == Some(entity.id) && !selected;
            let is_player_ent = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("player"));
            let is_solid = entity.behaviors.iter().any(|b| matches!(b, BehaviorConfig::Solid));
            let is_coin = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("coin"));
            let is_enemy = entity.tags.iter().any(|t| t.eq_ignore_ascii_case("enemy"));

            let has_sprite = !entity.sprite_path.is_empty() && self.sprite_cache.contains_key(&entity.sprite_path);
            let entity_tex = if has_sprite {
                self.sprite_cache[&entity.sprite_path]
            } else {
                tex
            };

            // Alpha: invisible entities shown as semi-transparent in editor
            let alpha: u8 = if !entity.visible { 60 } else { 255 };

            // Lighten colors when hovered (add ~40 to each channel)
            let brighten = |r: u8, g: u8, b: u8, a: u8| -> u32 {
                if hovered {
                    pack_color(r.saturating_add(50), g.saturating_add(50), b.saturating_add(50), a)
                } else {
                    pack_color(r, g, b, a)
                }
            };

            let color = if has_sprite {
                if selected { pack_color(255, 255, 200, alpha) }
                else { brighten(255, 255, 255, alpha) }
            } else if selected {
                pack_color(255, 220, 80, alpha)
            } else if is_player_ent {
                brighten(80, 220, 120, alpha)
            } else if is_solid {
                brighten(160, 160, 180, alpha)
            } else if is_coin {
                brighten(255, 220, 50, alpha.min(200))
            } else if is_enemy {
                brighten(220, 80, 80, alpha)
            } else {
                brighten(100, 150, 220, alpha)
            };

            // Compute UV from sprite sheet (show first frame or idle frame 0)
            let (uv_min, uv_max) = if let Some(ref sheet) = entity.sprite_sheet {
                // Live animated preview (transient, ADR-038): for the selected entity in
                // SpriteAnim mode, drive the frame from the preview timer instead of the
                // persisted preview_frame — so the scene data isn't mutated.
                let live_frame = if selected && self.editor_mode == EditorMode::SpriteAnim {
                    self.sprite_editor_preview_anim.as_ref().and_then(|(name, elapsed)| {
                        entity.animations.iter().find(|a| a.name == *name).and_then(|a| {
                            if a.frames.is_empty() {
                                None
                            } else {
                                let i = (*elapsed * a.fps) as usize % a.frames.len();
                                a.frames.get(i).copied()
                            }
                        })
                    })
                } else {
                    None
                };
                let frame_idx = live_frame.or(entity.preview_frame).unwrap_or_else(|| {
                    entity.default_animation.as_ref()
                        .and_then(|anim_name| entity.animations.iter().find(|a| a.name == *anim_name))
                        .and_then(|a| a.frames.first().copied())
                        .unwrap_or(0)
                });
                let col = frame_idx % sheet.columns;
                let row = frame_idx / sheet.columns;
                let u_step = 1.0 / sheet.columns as f32;
                let v_step = 1.0 / sheet.rows as f32;
                (
                    Vec2::new(col as f32 * u_step, row as f32 * v_step),
                    Vec2::new((col + 1) as f32 * u_step, (row + 1) as f32 * v_step),
                )
            } else {
                (Vec2::ZERO, Vec2::ONE)
            };

            // Render size: use frame size if sprite sheet, else entity size
            let render_size = if has_sprite {
                if let Some(ref sheet) = entity.sprite_sheet {
                    Vec2::new(sheet.frame_width as f32 * entity.scale_x,
                              sheet.frame_height as f32 * entity.scale_y)
                } else {
                    Vec2::new(entity.width * entity.scale_x, entity.height * entity.scale_y)
                }
            } else {
                Vec2::new(entity.width * entity.scale_x, entity.height * entity.scale_y)
            };

            ctx.draw_sprite(Sprite {
                texture: entity_tex,
                position: Vec2::new(entity.x, entity.y),
                size: render_size,
                rotation: entity.rotation,
                color,
                layer: entity.layer,
                uv_min,
                uv_max,
            });

            // Hover outline (thin, white, semi-transparent)
            if hovered {
                let hw = entity.width * entity.scale_x * 0.5 + 1.0;
                let hh = entity.height * entity.scale_y * 0.5 + 1.0;
                let thickness = 1.0 / self.camera_zoom;
                let rot = entity.rotation;
                let center = Vec2::new(entity.x, entity.y);
                let hover_color = pack_color(255, 255, 255, 120);
                let rotated = |local: Vec2| -> Vec2 {
                    let (sin, cos) = rot.sin_cos();
                    center + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos)
                };
                // Top/Bottom/Left/Right edges
                for (pos, size) in [
                    (rotated(Vec2::new(0.0, hh)), Vec2::new(hw * 2.0, thickness)),
                    (rotated(Vec2::new(0.0, -hh)), Vec2::new(hw * 2.0, thickness)),
                    (rotated(Vec2::new(-hw, 0.0)), Vec2::new(thickness, hh * 2.0)),
                    (rotated(Vec2::new(hw, 0.0)), Vec2::new(thickness, hh * 2.0)),
                ] {
                    ctx.draw_sprite(Sprite {
                        texture: tex, position: pos, size, rotation: rot,
                        color: hover_color, layer: 89,
                        uv_min: Vec2::ZERO, uv_max: Vec2::ONE,
                    });
                }
            }

            // Collider bounds preview for the selected entity — visualises the
            // collision shape (green), rotated with the entity, for quick debugging.
            if selected {
                if let Some(col) = &entity.collider {
                    let (chw, chh) = match col {
                        toile_scene::ColliderData::Aabb { half_w, half_h } => (*half_w, *half_h),
                        toile_scene::ColliderData::Circle { radius } => (*radius, *radius),
                    };
                    let t = 1.5 / self.camera_zoom;
                    let cc = pack_color(80, 220, 120, 220);
                    let rot = entity.rotation;
                    let (sin, cos) = rot.sin_cos();
                    let center = Vec2::new(entity.x, entity.y);
                    let rot_off = |lx: f32, ly: f32| {
                        center + Vec2::new(lx * cos - ly * sin, lx * sin + ly * cos)
                    };
                    for (lx, ly, sx, sy) in [
                        (0.0, chh, chw * 2.0, t),
                        (0.0, -chh, chw * 2.0, t),
                        (-chw, 0.0, t, chh * 2.0),
                        (chw, 0.0, t, chh * 2.0),
                    ] {
                        ctx.draw_sprite(DrawSprite {
                            texture: tex,
                            position: rot_off(lx, ly),
                            size: Vec2::new(sx, sy),
                            rotation: rot,
                            color: cc,
                            layer: 88,
                            uv_min: Vec2::ZERO,
                            uv_max: Vec2::ONE,
                        });
                    }
                }
            }

            // Selection outline + resize handles (rotated with entity)
            if selected {
                let hw = entity.width * entity.scale_x * 0.5;
                let hh = entity.height * entity.scale_y * 0.5;
                let ow = hw + 2.0;
                let oh = hh + 2.0;
                let thickness = 2.0 / self.camera_zoom;
                let handle_size = 8.0 / self.camera_zoom;
                let outline_color = pack_color(255, 255, 100, 200);
                let handle_color = pack_color(255, 255, 255, 255);
                let rot = entity.rotation;
                let center = Vec2::new(entity.x, entity.y);

                // Helper: rotate a local offset around entity center
                let rotated = |local: Vec2| -> Vec2 {
                    let (sin, cos) = rot.sin_cos();
                    center + Vec2::new(
                        local.x * cos - local.y * sin,
                        local.x * sin + local.y * cos,
                    )
                };

                // Outline edges (4 lines, each rotated)
                let edges = [
                    (Vec2::new(0.0, oh), Vec2::new(ow * 2.0, thickness)),   // top
                    (Vec2::new(0.0, -oh), Vec2::new(ow * 2.0, thickness)),  // bottom
                    (Vec2::new(-ow, 0.0), Vec2::new(thickness, oh * 2.0)),  // left
                    (Vec2::new(ow, 0.0), Vec2::new(thickness, oh * 2.0)),   // right
                ];
                for (local_pos, size) in edges {
                    ctx.draw_sprite(DrawSprite {
                        texture: tex,
                        position: rotated(local_pos),
                        size,
                        rotation: rot,
                        color: outline_color,
                        layer: 90,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                // Corner handles (4 squares)
                let corners_local = [
                    Vec2::new(hw, hh),
                    Vec2::new(hw, -hh),
                    Vec2::new(-hw, -hh),
                    Vec2::new(-hw, hh),
                ];
                for local in corners_local {
                    ctx.draw_sprite(DrawSprite {
                        texture: tex,
                        position: rotated(local),
                        size: Vec2::splat(handle_size),
                        rotation: rot,
                        color: handle_color,
                        layer: 91,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                // Edge midpoint handles
                let edge_color = pack_color(200, 220, 255, 255);
                let edge_handles = [
                    (Vec2::new(0.0, hh), Vec2::new(handle_size * 2.0, handle_size * 0.6)),   // top
                    (Vec2::new(0.0, -hh), Vec2::new(handle_size * 2.0, handle_size * 0.6)),  // bottom
                    (Vec2::new(-hw, 0.0), Vec2::new(handle_size * 0.6, handle_size * 2.0)),  // left
                    (Vec2::new(hw, 0.0), Vec2::new(handle_size * 0.6, handle_size * 2.0)),   // right
                ];
                for (local, size) in edge_handles {
                    ctx.draw_sprite(DrawSprite {
                        texture: tex,
                        position: rotated(local),
                        size,
                        rotation: rot,
                        color: edge_color,
                        layer: 91,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }

                // Rotation handle: line + diamond above top edge
                let rot_arm = hh + handle_size * 4.0;
                let rot_color = pack_color(120, 220, 255, 255);

                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: rotated(Vec2::new(0.0, hh + handle_size * 2.0)),
                    size: Vec2::new(thickness, handle_size * 4.0),
                    rotation: rot,
                    color: rot_color,
                    layer: 91,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });

                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: rotated(Vec2::new(0.0, rot_arm)),
                    size: Vec2::splat(handle_size * 1.5),
                    rotation: rot + std::f32::consts::FRAC_PI_4,
                    color: rot_color,
                    layer: 92,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }
        // Render preview particles on entities
        for pool in self.preview_particles.values() {
            for (pos, size, rot, color) in pool.render_data() {
                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: pos,
                    size: Vec2::splat(size),
                    rotation: rot,
                    color,
                    layer: 50,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }

        } // end `if self.editor_mode != EditorMode::Particle`

        // Render particles in Particle mode
        if self.editor_mode == EditorMode::Particle {
            for (pos, size, rot, color) in self.particle_editor.render_data() {
                ctx.draw_sprite(DrawSprite {
                    texture: tex,
                    position: pos,
                    size: Vec2::splat(size),
                    rotation: rot,
                    color,
                    layer: 0,
                    uv_min: Vec2::ZERO,
                    uv_max: Vec2::ONE,
                });
            }
        }
    }
}
