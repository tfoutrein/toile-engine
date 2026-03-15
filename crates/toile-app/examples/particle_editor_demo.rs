//! Toile Engine -- Particle Editor Demo (ADR-030)
//!
//! Live particle viewport with egui inspector panel.
//!
//! Run with: `cargo run --example particle_editor_demo -p toile-app`

use std::path::Path;

use glam::Vec2;
use toile_app::egui;
use toile_app::{App, Game, GameContext, TextureHandle};
use toile_app::core::particles::{presets, BlendMode, EmitterShape, ParticleEmitter, ParticlePool};
use toile_app::core::curve::Curve;
use toile_app::core::gradient::Gradient;
use toile_app::core::color::Color;
use toile_app::wgpu;
use toile_graphics::sprite_renderer::DrawSprite;
use winit::event::WindowEvent;
use winit::window::Window;

// Re-use the EguiOverlay helper from toile-editor by duplicating a minimal version here
// (toile-editor is a binary crate; we inline the pattern).

use egui_wgpu::ScreenDescriptor;

struct EguiOverlay {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl EguiOverlay {
    fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, window: &Window) -> Self {
        let ctx = egui::Context::default();
        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let renderer = egui_wgpu::Renderer::new(device, surface_format, Default::default());
        Self { ctx, state, renderer }
    }

    fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.state.on_window_event(window, event).consumed
    }

    fn begin_frame(&mut self, window: &Window) {
        let raw = self.state.take_egui_input(window);
        self.ctx.begin_frame(raw);
    }

    fn end_frame_and_render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
        screen_size: (u32, u32),
    ) {
        let full_output = self.ctx.end_frame();
        self.state.handle_platform_output(window, full_output.platform_output);
        let tris = self.ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen = ScreenDescriptor {
            size_in_pixels: [screen_size.0, screen_size.1],
            pixels_per_point: window.scale_factor() as f32,
        };
        for (id, delta) in &full_output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, delta);
        }
        self.renderer.update_buffers(device, queue, encoder, &tris, &screen);
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui_particle_editor"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        let mut pass = pass.forget_lifetime();
        self.renderer.render(&mut pass, &tris, &screen);
        drop(pass);
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}

// ----- Preset names -----

const PRESET_NAMES: &[&str] = &[
    "Fire", "Smoke", "Sparks", "Rain", "Snow", "Dust", "Explosion", "Confetti",
];

fn load_preset(name: &str) -> ParticleEmitter {
    match name {
        "Fire"      => presets::fire(),
        "Smoke"     => presets::smoke(),
        "Sparks"    => presets::sparks(),
        "Rain"      => presets::rain(),
        "Snow"      => presets::snow(),
        "Dust"      => presets::dust(),
        "Explosion" => presets::explosion(),
        "Confetti"  => presets::confetti(),
        _           => ParticleEmitter::default(),
    }
}

// ----- Curve editor widget -----

fn curve_editor_widget(
    ui: &mut egui::Ui,
    curve: &mut Curve,
    selected: &mut Option<usize>,
) {
    let desired_size = egui::vec2(200.0, 80.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 2.0, egui::Color32::from_gray(30));
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(70)), egui::StrokeKind::Outside);

    // Draw grid lines
    for i in 1..4 {
        let x = rect.min.x + (i as f32 / 4.0) * rect.width();
        painter.line_segment(
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            egui::Stroke::new(0.5, egui::Color32::from_gray(50)),
        );
    }
    for i in 1..4 {
        let y = rect.min.y + (i as f32 / 4.0) * rect.height();
        painter.line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
            egui::Stroke::new(0.5, egui::Color32::from_gray(50)),
        );
    }

    // Draw curve
    if curve.points.len() >= 2 {
        let pts: Vec<egui::Pos2> = (0..=60)
            .map(|i| {
                let t = i as f32 / 60.0;
                let v = curve.sample(t).clamp(0.0, 1.0);
                egui::pos2(
                    rect.min.x + t * rect.width(),
                    rect.max.y - v * rect.height(),
                )
            })
            .collect();
        painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 200, 255))));
    }

    // Handle interaction
    let mouse_pos = response.interact_pointer_pos();

    // Right-click to remove point
    if response.secondary_clicked() {
        if let Some(mp) = mouse_pos {
            // Find closest point
            let closest = curve.points.iter().enumerate().min_by_key(|(_, p)| {
                let px = egui::pos2(rect.min.x + p.0 * rect.width(), rect.max.y - p.1 * rect.height());
                let dx = (px.x - mp.x) as i32;
                let dy = (px.y - mp.y) as i32;
                dx * dx + dy * dy
            });
            if let Some((idx, p)) = closest {
                let (t, v) = *p;
                let px = egui::pos2(rect.min.x + t * rect.width(), rect.max.y - v * rect.height());
                let dist = ((px.x - mp.x).powi(2) + (px.y - mp.y).powi(2)).sqrt();
                if dist < 12.0 && curve.points.len() > 2 {
                    curve.points.remove(idx);
                    if *selected == Some(idx) {
                        *selected = None;
                    }
                }
            }
        }
    }

    // Left-click: select or add
    if response.clicked() {
        if let Some(mp) = mouse_pos {
            // Check if clicking near an existing point
            let closest = curve.points.iter().enumerate().min_by_key(|(_, p)| {
                let px = egui::pos2(rect.min.x + p.0 * rect.width(), rect.max.y - p.1 * rect.height());
                let dx = (px.x - mp.x) as i32;
                let dy = (px.y - mp.y) as i32;
                dx * dx + dy * dy
            });
            let mut hit = false;
            if let Some((idx, p)) = closest {
                let (t, v) = *p;
                let px = egui::pos2(rect.min.x + t * rect.width(), rect.max.y - v * rect.height());
                let dist = ((px.x - mp.x).powi(2) + (px.y - mp.y).powi(2)).sqrt();
                if dist < 10.0 {
                    *selected = Some(idx);
                    hit = true;
                }
            }
            if !hit {
                // Add new point
                let t = ((mp.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                let v = ((rect.max.y - mp.y) / rect.height()).clamp(0.0, 1.0);
                curve.points.push((t, v));
                curve.points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                *selected = None;
            }
        }
    }

    // Drag selected point
    if response.dragged() {
        if let Some(sel) = *selected {
            if let Some(mp) = mouse_pos {
                let t = ((mp.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                let v = ((rect.max.y - mp.y) / rect.height()).clamp(0.0, 1.0);
                if sel < curve.points.len() {
                    curve.points[sel] = (t, v);
                    curve.points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                }
            }
        }
    }

    // Draw control points
    for (i, &(t, v)) in curve.points.iter().enumerate() {
        let pos = egui::pos2(
            rect.min.x + t * rect.width(),
            rect.max.y - v.clamp(0.0, 1.0) * rect.height(),
        );
        let color = if *selected == Some(i) {
            egui::Color32::YELLOW
        } else {
            egui::Color32::WHITE
        };
        painter.circle_filled(pos, 4.0, color);
        painter.circle_stroke(pos, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(80)));
    }

    ui.add_space(2.0);
    ui.label(egui::RichText::new("Click: add/select | Right-click: remove | Drag: move").size(9.0).color(egui::Color32::from_gray(120)));
}

// ----- Gradient editor widget -----

fn gradient_editor_widget(
    ui: &mut egui::Ui,
    gradient: &mut Gradient,
    selected: &mut Option<usize>,
) {
    let bar_size = egui::vec2(200.0, 24.0);
    let (bar_rect, bar_response) = ui.allocate_exact_size(bar_size, egui::Sense::click());
    let painter = ui.painter_at(bar_rect);

    // Draw gradient bar using many segments
    let steps = 64usize;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let c = gradient.sample(t0);
        let x0 = bar_rect.min.x + t0 * bar_rect.width();
        let x1 = bar_rect.min.x + t1 * bar_rect.width();
        let color = egui::Color32::from_rgba_unmultiplied(
            (c[0] * 255.0) as u8,
            (c[1] * 255.0) as u8,
            (c[2] * 255.0) as u8,
            (c[3] * 255.0) as u8,
        );
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x0, bar_rect.min.y),
                egui::pos2(x1, bar_rect.max.y),
            ),
            0.0,
            color,
        );
    }
    painter.rect_stroke(bar_rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_gray(80)), egui::StrokeKind::Outside);

    // Click on bar to add stop
    if bar_response.clicked() {
        if let Some(mp) = bar_response.interact_pointer_pos() {
            let t = ((mp.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
            let sampled = gradient.sample(t);
            gradient.stops.push((t, sampled));
            gradient.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
    }

    // Draw handles below bar
    let handle_y = bar_rect.max.y + 6.0;
    for (i, &(t, _)) in gradient.stops.iter().enumerate() {
        let hx = bar_rect.min.x + t * bar_rect.width();
        let handle_pos = egui::pos2(hx, handle_y);
        let color = if *selected == Some(i) {
            egui::Color32::YELLOW
        } else {
            egui::Color32::WHITE
        };
        // Triangle handle
        painter.add(egui::Shape::convex_polygon(
            vec![
                handle_pos,
                egui::pos2(hx - 5.0, handle_y + 10.0),
                egui::pos2(hx + 5.0, handle_y + 10.0),
            ],
            color,
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
        ));
    }

    // Invisible sense rects over handles for interaction
    // Collect stop positions first to avoid borrow conflict when mutating stops inside the loop.
    let stop_ts: Vec<f32> = gradient.stops.iter().map(|(t, _)| *t).collect();
    let mut remove_idx: Option<usize> = None;
    let mut drag_update: Option<(usize, f32)> = None;
    for (i, t) in stop_ts.iter().enumerate() {
        let hx = bar_rect.min.x + t * bar_rect.width();
        let hrect = egui::Rect::from_center_size(
            egui::pos2(hx, handle_y + 5.0),
            egui::vec2(12.0, 16.0),
        );
        let hid = ui.id().with(("gstop", i));
        let hr = ui.interact(hrect, hid, egui::Sense::click_and_drag());
        if hr.clicked() {
            *selected = Some(i);
        }
        if hr.secondary_clicked() && gradient.stops.len() > 2 {
            remove_idx = Some(i);
            break;
        }
        if hr.dragged() {
            *selected = Some(i);
            if let Some(mp) = hr.interact_pointer_pos() {
                let new_t = ((mp.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                drag_update = Some((i, new_t));
            }
        }
    }
    if let Some(i) = remove_idx {
        gradient.stops.remove(i);
        if *selected == Some(i) {
            *selected = None;
        }
    }
    if let Some((i, new_t)) = drag_update {
        if i < gradient.stops.len() {
            gradient.stops[i].0 = new_t;
            gradient.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            *selected = None;
        }
    }

    // Extra vertical space for the handles
    ui.add_space(18.0);
    ui.add_space(2.0);
    ui.label(egui::RichText::new("Click bar: add stop | Click handle: select | Drag: move | Right-click: remove").size(9.0).color(egui::Color32::from_gray(120)));

    // Color picker for selected stop
    if let Some(sel) = *selected {
        if sel < gradient.stops.len() {
            let c = &mut gradient.stops[sel].1;
            let mut rgba = egui::Rgba::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
            ui.horizontal(|ui| {
                ui.label("Color:");
                egui::widgets::color_picker::color_edit_button_rgba(
                    ui,
                    &mut rgba,
                    egui::widgets::color_picker::Alpha::OnlyBlend,
                );
            });
            c[0] = rgba.r();
            c[1] = rgba.g();
            c[2] = rgba.b();
            c[3] = rgba.a();
        }
    }
}

// ----- Main state -----

struct ParticleEditor {
    overlay: Option<EguiOverlay>,
    surface_format: Option<wgpu::TextureFormat>,
    // Particle simulation
    pool: ParticlePool,
    sub_pool: Option<ParticlePool>, // for on_death particles when sub_emitter_enabled=true
    emitter: ParticleEmitter,
    texture: Option<TextureHandle>,
    elapsed: f32,
    // Editor state
    selected_curve_point: Option<usize>,
    selected_gradient_stop: Option<usize>,
    burst_enabled: bool,
    burst_count: u32,
    sub_emitter_enabled: bool,
    sub_emitter_preset: String,
    current_preset: String,
    // Dirty flag -- if emitter changed, rebuild pool
    emitter_dirty: bool,
}

impl ParticleEditor {
    fn new() -> Self {
        let emitter = presets::fire();
        let pool = ParticlePool::new(emitter.clone(), Vec2::ZERO);
        Self {
            overlay: None,
            surface_format: None,
            pool,
            sub_pool: None,
            emitter,
            texture: None,
            elapsed: 0.0,
            selected_curve_point: None,
            selected_gradient_stop: None,
            burst_enabled: false,
            burst_count: 10,
            sub_emitter_enabled: false,
            sub_emitter_preset: "Sparks".to_string(),
            current_preset: "Fire".to_string(),
            emitter_dirty: false,
        }
    }

    fn rebuild_pool(&mut self) {
        let mut new_emitter = self.emitter.clone();
        // Handle burst
        if self.burst_enabled {
            new_emitter.burst = Some(self.burst_count);
        } else {
            new_emitter.burst = None;
        }
        // Handle sub-emitter
        if self.sub_emitter_enabled {
            let mut sub = load_preset(&self.sub_emitter_preset);
            sub.on_death = None; // no recursion
            new_emitter.on_death = Some(Box::new(sub));
        } else {
            new_emitter.on_death = None;
        }
        self.pool = ParticlePool::new(new_emitter, Vec2::ZERO);
        self.emitter_dirty = false;
    }

    fn load_preset_by_name(&mut self, name: &str) {
        self.current_preset = name.to_string();
        self.emitter = load_preset(name);
        self.burst_enabled = self.emitter.burst.is_some();
        self.burst_count = self.emitter.burst.unwrap_or(10);
        self.emitter_dirty = true;
    }

    fn draw_inspector(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .min_width(320.0)
            .max_width(320.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Particle Editor");
                    ui.separator();

                    // Preset buttons
                    ui.label("Presets:");
                    ui.horizontal_wrapped(|ui| {
                        for name in PRESET_NAMES {
                            if ui.button(*name).clicked() {
                                self.load_preset_by_name(name);
                            }
                        }
                    });
                    ui.separator();

                    // Emission section
                    egui::CollapsingHeader::new("Emission")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::Grid::new("emission_grid").num_columns(2).show(ui, |ui| {
                                ui.label("Rate:");
                                let mut rate = self.emitter.rate;
                                if ui.add(egui::DragValue::new(&mut rate).range(0.0..=500.0).speed(1.0)).changed() {
                                    self.emitter.rate = rate;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Burst:");
                                let mut burst_en = self.burst_enabled;
                                if ui.checkbox(&mut burst_en, "").changed() {
                                    self.burst_enabled = burst_en;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                if self.burst_enabled {
                                    ui.label("Burst Count:");
                                    let mut bc = self.burst_count;
                                    if ui.add(egui::DragValue::new(&mut bc).range(1..=1000)).changed() {
                                        self.burst_count = bc;
                                        self.emitter_dirty = true;
                                    }
                                    ui.end_row();
                                }

                                ui.label("Lifetime min:");
                                let mut lmin = self.emitter.lifetime.0;
                                if ui.add(egui::DragValue::new(&mut lmin).range(0.0..=30.0).speed(0.05)).changed() {
                                    self.emitter.lifetime.0 = lmin;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Lifetime max:");
                                let mut lmax = self.emitter.lifetime.1;
                                if ui.add(egui::DragValue::new(&mut lmax).range(0.0..=30.0).speed(0.05)).changed() {
                                    self.emitter.lifetime.1 = lmax;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Particles:");
                                ui.label(format!("{}", self.pool.particle_count()));
                                ui.end_row();
                            });
                        });

                    // Shape section
                    egui::CollapsingHeader::new("Shape")
                        .default_open(true)
                        .show(ui, |ui| {
                            let shape_name = match &self.emitter.shape {
                                EmitterShape::Point              => "Point",
                                EmitterShape::Circle { .. }      => "Circle",
                                EmitterShape::Rectangle { .. }   => "Rectangle",
                                EmitterShape::Line { .. }        => "Line",
                            };
                            egui::ComboBox::from_label("Shape")
                                .selected_text(shape_name)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(shape_name == "Point", "Point").clicked() {
                                        self.emitter.shape = EmitterShape::Point;
                                        self.emitter_dirty = true;
                                    }
                                    if ui.selectable_label(shape_name == "Circle", "Circle").clicked() {
                                        self.emitter.shape = EmitterShape::Circle { radius: 50.0 };
                                        self.emitter_dirty = true;
                                    }
                                    if ui.selectable_label(shape_name == "Rectangle", "Rectangle").clicked() {
                                        self.emitter.shape = EmitterShape::Rectangle { half_extents: Vec2::new(50.0, 50.0) };
                                        self.emitter_dirty = true;
                                    }
                                    if ui.selectable_label(shape_name == "Line", "Line").clicked() {
                                        self.emitter.shape = EmitterShape::Line { length: 200.0 };
                                        self.emitter_dirty = true;
                                    }
                                });

                            match &mut self.emitter.shape {
                                EmitterShape::Circle { radius } => {
                                    ui.horizontal(|ui| {
                                        ui.label("Radius:");
                                        if ui.add(egui::Slider::new(radius, 1.0..=500.0)).changed() {
                                            self.emitter_dirty = true;
                                        }
                                    });
                                }
                                EmitterShape::Rectangle { half_extents } => {
                                    ui.horizontal(|ui| {
                                        ui.label("Half W:");
                                        if ui.add(egui::DragValue::new(&mut half_extents.x).range(1.0..=500.0)).changed() {
                                            self.emitter_dirty = true;
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Half H:");
                                        if ui.add(egui::DragValue::new(&mut half_extents.y).range(1.0..=500.0)).changed() {
                                            self.emitter_dirty = true;
                                        }
                                    });
                                }
                                EmitterShape::Line { length } => {
                                    ui.horizontal(|ui| {
                                        ui.label("Length:");
                                        if ui.add(egui::Slider::new(length, 1.0..=1000.0)).changed() {
                                            self.emitter_dirty = true;
                                        }
                                    });
                                }
                                EmitterShape::Point => {}
                            }
                        });

                    // Velocity section
                    egui::CollapsingHeader::new("Velocity")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::Grid::new("velocity_grid").num_columns(2).show(ui, |ui| {
                                ui.label("Direction (deg):");
                                let mut dir_deg = self.emitter.direction.to_degrees();
                                if ui.add(egui::Slider::new(&mut dir_deg, 0.0..=360.0)).changed() {
                                    self.emitter.direction = dir_deg.to_radians();
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Spread (deg):");
                                let mut spread_deg = self.emitter.spread_angle.to_degrees();
                                if ui.add(egui::Slider::new(&mut spread_deg, 0.0..=360.0)).changed() {
                                    self.emitter.spread_angle = spread_deg.to_radians();
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Speed min:");
                                let mut smin = self.emitter.initial_speed.0;
                                if ui.add(egui::DragValue::new(&mut smin).range(0.0..=2000.0).speed(1.0)).changed() {
                                    self.emitter.initial_speed.0 = smin;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Speed max:");
                                let mut smax = self.emitter.initial_speed.1;
                                if ui.add(egui::DragValue::new(&mut smax).range(0.0..=2000.0).speed(1.0)).changed() {
                                    self.emitter.initial_speed.1 = smax;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Gravity X:");
                                let mut gx = self.emitter.gravity.x;
                                if ui.add(egui::DragValue::new(&mut gx).speed(1.0)).changed() {
                                    self.emitter.gravity.x = gx;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Gravity Y:");
                                let mut gy = self.emitter.gravity.y;
                                if ui.add(egui::DragValue::new(&mut gy).speed(1.0)).changed() {
                                    self.emitter.gravity.y = gy;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();
                            });
                        });

                    // Size section
                    egui::CollapsingHeader::new("Size")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::Grid::new("size_grid").num_columns(2).show(ui, |ui| {
                                ui.label("Size min:");
                                let mut smin = self.emitter.size_start.0;
                                if ui.add(egui::DragValue::new(&mut smin).range(0.5..=200.0).speed(0.5)).changed() {
                                    self.emitter.size_start.0 = smin;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Size max:");
                                let mut smax = self.emitter.size_start.1;
                                if ui.add(egui::DragValue::new(&mut smax).range(0.5..=200.0).speed(0.5)).changed() {
                                    self.emitter.size_start.1 = smax;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();
                            });

                            ui.label("Size over life:");
                            let mut curve = self.emitter.size_over_life.clone();
                            curve_editor_widget(ui, &mut curve, &mut self.selected_curve_point);
                            if curve.points != self.emitter.size_over_life.points {
                                self.emitter.size_over_life = curve;
                                self.emitter_dirty = true;
                            }
                        });

                    // Color section
                    egui::CollapsingHeader::new("Color")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label("Color over life:");
                            let mut grad = self.emitter.color_over_life.clone();
                            gradient_editor_widget(ui, &mut grad, &mut self.selected_gradient_stop);
                            if grad.stops != self.emitter.color_over_life.stops {
                                self.emitter.color_over_life = grad;
                                self.emitter_dirty = true;
                            }
                        });

                    // Rotation section
                    egui::CollapsingHeader::new("Rotation")
                        .default_open(false)
                        .show(ui, |ui| {
                            egui::Grid::new("rot_grid").num_columns(2).show(ui, |ui| {
                                ui.label("Rot speed min:");
                                let mut rmin = self.emitter.rotation_speed.0;
                                if ui.add(egui::DragValue::new(&mut rmin).speed(0.1)).changed() {
                                    self.emitter.rotation_speed.0 = rmin;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();

                                ui.label("Rot speed max:");
                                let mut rmax = self.emitter.rotation_speed.1;
                                if ui.add(egui::DragValue::new(&mut rmax).speed(0.1)).changed() {
                                    self.emitter.rotation_speed.1 = rmax;
                                    self.emitter_dirty = true;
                                }
                                ui.end_row();
                            });
                        });

                    // Blend mode section
                    egui::CollapsingHeader::new("Blend Mode")
                        .default_open(false)
                        .show(ui, |ui| {
                            let mut blend = self.emitter.blend_mode;
                            ui.horizontal(|ui| {
                                if ui.radio_value(&mut blend, BlendMode::Alpha, "Alpha").changed() {
                                    self.emitter.blend_mode = blend;
                                    self.emitter_dirty = true;
                                }
                                if ui.radio_value(&mut blend, BlendMode::Additive, "Additive").changed() {
                                    self.emitter.blend_mode = blend;
                                    self.emitter_dirty = true;
                                }
                            });
                        });

                    // Sub-emitter section
                    egui::CollapsingHeader::new("Sub-emitter (on_death)")
                        .default_open(false)
                        .show(ui, |ui| {
                            let mut en = self.sub_emitter_enabled;
                            if ui.checkbox(&mut en, "Enable on_death sub-emitter").changed() {
                                self.sub_emitter_enabled = en;
                                self.emitter_dirty = true;
                            }
                            if self.sub_emitter_enabled {
                                let mut sub_preset = self.sub_emitter_preset.clone();
                                egui::ComboBox::from_label("Sub-emitter preset")
                                    .selected_text(&sub_preset)
                                    .show_ui(ui, |ui| {
                                        for name in PRESET_NAMES {
                                            if ui.selectable_label(sub_preset == *name, *name).clicked() {
                                                sub_preset = name.to_string();
                                                self.emitter_dirty = true;
                                            }
                                        }
                                    });
                                if sub_preset != self.sub_emitter_preset {
                                    self.sub_emitter_preset = sub_preset;
                                    self.emitter_dirty = true;
                                }
                            }
                        });

                    // Save / Load section
                    egui::CollapsingHeader::new("Save / Load")
                        .default_open(false)
                        .show(ui, |ui| {
                            if ui.button("Save JSON").clicked() {
                                self.save_json();
                            }
                            if ui.button("Load JSON").clicked() {
                                self.load_json();
                            }
                        });
                });
            });
    }

    fn save_json(&self) {
        let path = "assets/particles/custom.particles.json";
        if let Ok(json) = serde_json::to_string_pretty(&self.emitter) {
            if let Err(e) = std::fs::create_dir_all("assets/particles") {
                log::error!("Could not create assets/particles dir: {e}");
                return;
            }
            match std::fs::write(path, &json) {
                Ok(_) => log::info!("Saved particle config to {path}"),
                Err(e) => log::error!("Save failed: {e}"),
            }
        }
    }

    fn load_json(&mut self) {
        let path = "assets/particles/custom.particles.json";
        match std::fs::read_to_string(path) {
            Ok(s) => {
                match serde_json::from_str::<ParticleEmitter>(&s) {
                    Ok(emitter) => {
                        self.emitter = emitter;
                        self.emitter_dirty = true;
                        log::info!("Loaded particle config from {path}");
                    }
                    Err(e) => log::error!("Parse error: {e}"),
                }
            }
            Err(e) => log::error!("Load failed: {e}"),
        }
    }
}

impl Game for ParticleEditor {
    fn init(&mut self, ctx: &mut GameContext) {
        self.texture = Some(ctx.load_texture(Path::new("assets/white.png")));
        // camera zoom = 2.0 for macOS Retina
        ctx.camera.zoom = 2.0;
        // Start with fire preset
        self.rebuild_pool();
        log::info!("Particle Editor ready. Use the inspector panel on the right.");
    }

    fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        self.elapsed += dt as f32;
        let dt_f = dt as f32;

        if self.emitter_dirty {
            self.rebuild_pool();
        }

        self.pool.update(dt_f);
    }

    fn draw(&mut self, ctx: &mut GameContext) {
        let tex = match self.texture {
            Some(t) => t,
            None => return,
        };

        for (pos, size, rot, color) in self.pool.render_data() {
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

    fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        if let Some(overlay) = &mut self.overlay {
            overlay.handle_event(window, event)
        } else {
            false
        }
    }

    fn render_overlay(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
        size: (u32, u32),
    ) {
        // Lazily initialize the overlay on first render
        if self.overlay.is_none() {
            let fmt = self.surface_format.unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);
            self.overlay = Some(EguiOverlay::new(device, fmt, window));
        }

        self.overlay.as_mut().unwrap().begin_frame(window);

        let ctx = self.overlay.as_ref().unwrap().ctx.clone();
        self.draw_inspector(&ctx);

        self.overlay.as_mut().unwrap().end_frame_and_render(device, queue, encoder, view, window, size);
    }
}

// We need surface_format to init egui renderer. Store it from init.
impl ParticleEditor {
    fn set_surface_format(&mut self, fmt: wgpu::TextureFormat) {
        self.surface_format = Some(fmt);
    }
}

fn main() {
    App::new()
        .with_title("Toile -- Particle Editor (ADR-030)")
        .with_size(1280, 720)
        .with_clear_color(Color::new(0.06, 0.06, 0.10, 1.0))
        .run(ParticleEditor::new());
}
