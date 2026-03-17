//! Asset Browser UI — standalone egui application for browsing imported asset packs.

pub mod browser_panel;
pub mod detail_panel;

use std::collections::HashMap;

use egui_wgpu::ScreenDescriptor;
use toile_app::{App, Game, GameContext};
use toile_core::color::Color;
use winit::event::WindowEvent;
use winit::window::Window;

use crate::types::AssetType;
use crate::ToileAssetLibrary;

// ---------------------------------------------------------------------------
// EguiOverlay — inlined from toile-editor/src/overlay.rs
// ---------------------------------------------------------------------------

/// Wraps egui context, winit state, and wgpu renderer.
pub struct EguiOverlay {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl EguiOverlay {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
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

        Self {
            ctx,
            state,
            renderer,
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
    }

    pub fn renderer_mut(&mut self) -> &mut egui_wgpu::Renderer {
        &mut self.renderer
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        #[allow(deprecated)]
        self.ctx.begin_frame(raw_input);
    }

    pub fn end_frame_and_render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
        screen_size: (u32, u32),
    ) {
        #[allow(deprecated)]
        let full_output = self.ctx.end_frame();

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen = ScreenDescriptor {
            size_in_pixels: [screen_size.0, screen_size.1],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen);

        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui_pass"),
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

// ---------------------------------------------------------------------------
// AssetBrowserApp
// ---------------------------------------------------------------------------

/// The standalone asset browser application implementing the Game trait.
pub struct AssetBrowserApp {
    pub library: ToileAssetLibrary,
    pub filter_type: Option<AssetType>,
    pub search_text: String,
    pub selected_asset: Option<String>,
    pub thumbnail_cache: HashMap<String, egui::TextureHandle>,
    pub preview_texture: Option<egui::TextureHandle>,
    pub overlay: Option<EguiOverlay>,
    surface_format: Option<wgpu::TextureFormat>,
    /// Tracks which preview image path is currently loaded (to avoid reloading).
    preview_loaded_path: String,
}

impl AssetBrowserApp {
    pub fn new() -> Self {
        Self {
            library: ToileAssetLibrary::new(),
            filter_type: None,
            search_text: String::new(),
            selected_asset: None,
            thumbnail_cache: HashMap::new(),
            preview_texture: None,
            overlay: None,
            surface_format: None,
            preview_loaded_path: String::new(),
        }
    }

    /// Import a pack via native file dialog.
    fn import_pack_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Asset Pack Folder")
            .pick_folder()
        {
            match self.library.import_pack(&path) {
                Ok(count) => {
                    log::info!("Imported {} assets from '{}'", count, path.display());
                }
                Err(e) => {
                    log::error!("Import failed: {e}");
                }
            }
        }
    }

    /// Get filtered assets based on current search text and type filter.
    fn filtered_asset_ids(&self) -> Vec<String> {
        self.library
            .assets
            .iter()
            .filter(|a| {
                // Type filter
                if let Some(ft) = self.filter_type {
                    if a.asset_type != ft {
                        return false;
                    }
                }
                // Text search
                if !self.search_text.is_empty() {
                    let lower = self.search_text.to_lowercase();
                    let matches = a.name.to_lowercase().contains(&lower)
                        || a.path.to_lowercase().contains(&lower)
                        || a.tags.iter().any(|t| t.to_lowercase().contains(&lower));
                    if !matches {
                        return false;
                    }
                }
                true
            })
            .map(|a| a.id.clone())
            .collect()
    }

    /// Load a thumbnail into the egui texture cache if not already present.
    fn ensure_thumbnail(
        &mut self,
        asset_id: &str,
        ctx: &egui::Context,
    ) {
        if self.thumbnail_cache.contains_key(asset_id) {
            return;
        }

        // Find the asset
        let asset = self.library.assets.iter().find(|a| a.id == asset_id);
        let asset = match asset {
            Some(a) => a.clone(),
            None => return,
        };

        // Try loading thumbnail
        if let Some(thumb_path) = self.library.thumbnail_absolute_path(&asset) {
            if thumb_path.exists() {
                if let Ok(img) = image::open(&thumb_path) {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let tex = ctx.load_texture(
                        format!("thumb_{}", asset_id),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.thumbnail_cache.insert(asset_id.to_string(), tex);
                    return;
                }
            }
        }

        // Fallback: try loading the asset image directly and resize
        if let Some(abs_path) = self.library.absolute_path(&asset) {
            if abs_path.exists() {
                if let Ok(img) = image::open(&abs_path) {
                    let thumb = img.thumbnail(128, 128);
                    let rgba = thumb.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let tex = ctx.load_texture(
                        format!("thumb_{}", asset_id),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.thumbnail_cache.insert(asset_id.to_string(), tex);
                }
            }
        }
    }

    /// Load the full-size preview for the selected asset.
    fn load_preview(&mut self, asset_id: &str, ctx: &egui::Context) {
        let asset = self.library.assets.iter().find(|a| a.id == asset_id);
        let asset = match asset {
            Some(a) => a.clone(),
            None => return,
        };

        if let Some(abs_path) = self.library.absolute_path(&asset) {
            let path_str = abs_path.to_string_lossy().to_string();
            if path_str == self.preview_loaded_path {
                return; // Already loaded
            }

            if abs_path.exists() {
                if let Ok(img) = image::open(&abs_path) {
                    // Limit preview size to 512px
                    let preview = img.thumbnail(512, 512);
                    let rgba = preview.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let tex = ctx.load_texture(
                        format!("preview_{}", asset_id),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.preview_texture = Some(tex);
                    self.preview_loaded_path = path_str;
                }
            }
        }
    }
}

impl Game for AssetBrowserApp {
    fn init(&mut self, ctx: &mut GameContext) {
        self.surface_format = Some(ctx.surface_format());
        log::info!("Toile Asset Browser ready");
    }

    fn update(&mut self, _ctx: &mut GameContext, _dt: f64) {}

    fn draw(&mut self, _ctx: &mut GameContext) {}

    fn render_overlay(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
        size: (u32, u32),
    ) {
        let surface_format = self
            .surface_format
            .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);

        {
            let overlay = self.overlay.get_or_insert_with(|| {
                let o = EguiOverlay::new(device, surface_format, window);
                let mut style = (*o.ctx().style()).clone();
                style.visuals = egui::Visuals::dark();
                o.ctx().set_style(style);
                o
            });
            overlay.begin_frame(window);
        }
        let ctx = self.overlay.as_ref().unwrap().ctx().clone();

        // Main UI layout
        self.show_ui(&ctx);

        self.overlay
            .as_mut()
            .unwrap()
            .end_frame_and_render(device, queue, encoder, view, window, size);
    }

    fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        if let Some(overlay) = &mut self.overlay {
            overlay.handle_event(window, event)
        } else {
            false
        }
    }
}

impl AssetBrowserApp {
    /// Render the complete asset browser UI.
    fn show_ui(&mut self, ctx: &egui::Context) {
        let total = self.library.count();
        let filtered_ids = self.filtered_asset_ids();
        let filtered_count = filtered_ids.len();

        // Status bar at the bottom
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Total: {} assets | Showing: {} | Packs: {}",
                    total,
                    filtered_count,
                    self.library.packs.len(),
                ));
                if let Some(ref sel) = self.selected_asset {
                    ui.separator();
                    ui.label(format!("Selected: {}", sel));
                }
            });
        });

        // Detail panel on the right (only when an asset is selected)
        let has_selection = self.selected_asset.is_some();
        if has_selection {
            egui::SidePanel::right("detail_panel")
                .min_width(320.0)
                .max_width(400.0)
                .show(ctx, |ui| {
                    detail_panel::show_detail_panel(self, ui, ctx);
                });
        }

        // Central panel: browser
        egui::CentralPanel::default().show(ctx, |ui| {
            browser_panel::show_browser_panel(self, ui, ctx, &filtered_ids);
        });
    }
}

/// Launch the asset browser as a standalone application.
pub fn run_asset_browser() {
    App::new()
        .with_title("Toile Asset Browser")
        .with_size(1400, 900)
        .with_clear_color(Color::rgb(0.10, 0.10, 0.14))
        .run(AssetBrowserApp::new());
}
