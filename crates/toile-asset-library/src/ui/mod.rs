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

pub mod file_browser;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Assets,
    Files,
}

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
    pub registry: crate::registry::PackRegistry,
    pub filter_type: Option<AssetType>,
    pub filter_pack: Option<String>, // selected pack ID to filter by
    pub search_text: String,
    pub selected_asset: Option<String>,
    pub thumbnail_cache: HashMap<String, egui::TextureHandle>,
    pub preview_texture: Option<egui::TextureHandle>,
    /// Keep the previous preview texture alive for one extra frame to avoid wgpu crash.
    prev_preview_texture: Option<egui::TextureHandle>,
    pub overlay: Option<EguiOverlay>,
    pub status_msg: String,
    pub importing: bool,
    pub import_progress: std::sync::Arc<std::sync::atomic::AtomicU32>, // 0-1000 (permille)
    pub import_total: std::sync::Arc<std::sync::atomic::AtomicU32>,
    import_result: Option<std::sync::mpsc::Receiver<(String, Result<usize, String>)>>,
    pub view_mode: ViewMode,
    pub readme_content: Option<(String, String)>, // (filename, content)
    pub highlight_file: Option<String>, // relative path to highlight in file tree
    surface_format: Option<wgpu::TextureFormat>,
    preview_loaded_path: String,
    tex_counter: u64,
    initialized: bool,
}

impl AssetBrowserApp {
    pub fn new() -> Self {
        Self {
            library: ToileAssetLibrary::new(),
            registry: crate::registry::load_registry(),
            filter_type: None,
            filter_pack: None,
            search_text: String::new(),
            selected_asset: None,
            thumbnail_cache: HashMap::new(),
            preview_texture: None,
            prev_preview_texture: None,
            overlay: None,
            status_msg: String::new(),
            importing: false,
            import_progress: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            import_total: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            import_result: None,
            view_mode: ViewMode::Assets,
            readme_content: None,
            highlight_file: None,
            surface_format: None,
            preview_loaded_path: String::new(),
            tex_counter: 0,
            initialized: false,
        }
    }

    /// Reload all registered packs from their manifests.
    fn reload_registered_packs(&mut self) {
        let paths: Vec<String> = self.registry.packs.iter().map(|p| p.path.clone()).collect();
        for path_str in &paths {
            let path = std::path::Path::new(path_str);
            if path.is_dir() {
                match self.library.import_pack(path) {
                    Ok(count) => log::info!("Reloaded {} assets from '{}'", count, path_str),
                    Err(e) => log::warn!("Failed to reload '{}': {e}", path_str),
                }
            } else {
                log::warn!("Pack directory not found: '{}'", path_str);
            }
        }
        if !paths.is_empty() {
            self.status_msg = format!("Loaded {} pack(s), {} assets total", paths.len(), self.library.count());
        }
    }

    /// Import a pack via native file dialog (folder or ZIP).
    fn import_pack_dialog(&mut self) {
        // Show a dialog that accepts both folders and ZIP files
        // Try folder first, then file
        let result = rfd::FileDialog::new()
            .set_title("Select Asset Pack (Folder or ZIP)")
            .add_filter("ZIP Archive", &["zip"])
            .pick_file();

        if let Some(file_path) = result {
            if file_path.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
                // Extract ZIP next to itself
                let stem = file_path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "pack".into());
                let extract_dir = file_path.parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join(&stem);

                if !extract_dir.exists() {
                    self.status_msg = format!("Extracting '{}'...", stem);
                    if let Err(e) = crate::scanner::extract_zip(&file_path, &extract_dir) {
                        self.status_msg = format!("ZIP extraction failed: {e}");
                        return;
                    }
                }

                self.import_directory(&extract_dir);
            } else {
                // Not a ZIP — treat as a file in a pack directory
                if let Some(parent) = file_path.parent() {
                    self.import_directory(parent);
                }
            }
            return;
        }

        // If no file selected, try folder picker
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Asset Pack Folder")
            .pick_folder()
        {
            self.import_directory(&path);
        }
    }

    /// Import a directory as a pack (in a background thread).
    fn import_directory(&mut self, path: &std::path::Path) {
        let path_owned = path.to_path_buf();
        let (tx, rx) = std::sync::mpsc::channel();
        self.importing = true;
        self.import_progress.store(0, std::sync::atomic::Ordering::Relaxed);
        self.import_total.store(1, std::sync::atomic::Ordering::Relaxed);
        self.status_msg = format!("Importing '{}'...", path.file_name().unwrap_or_default().to_string_lossy());
        self.import_result = Some(rx);

        let progress_current = self.import_progress.clone();
        let progress_total = self.import_total.clone();

        std::thread::spawn(move || {
            let mut lib = crate::ToileAssetLibrary::new();
            let progress_cb = |current: u32, total: u32| {
                progress_current.store(current, std::sync::atomic::Ordering::Relaxed);
                progress_total.store(total.max(1), std::sync::atomic::Ordering::Relaxed);
            };
            let result = lib.import_pack_with_progress(&path_owned, Some(&progress_cb));
            let name = path_owned.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "pack".into());
            let _ = tx.send((name, result));
        });

        // Register immediately (manifest will be ready when thread finishes)
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "pack".into());
        crate::registry::register_pack(&mut self.registry, &name, path);
    }

    /// Check if background import finished and load results.
    fn check_import_result(&mut self) {
        if let Some(ref rx) = self.import_result {
            if let Ok((name, result)) = rx.try_recv() {
                match result {
                    Ok(count) => {
                        // Reload from manifest
                        let paths: Vec<String> = self.registry.packs.iter().map(|p| p.path.clone()).collect();
                        for p in &paths {
                            let path = std::path::Path::new(p);
                            if path.is_dir() {
                                let _ = self.library.import_pack(path);
                            }
                        }
                        self.status_msg = format!("Imported '{}' — {} assets", name, count);
                    }
                    Err(e) => {
                        self.status_msg = format!("Import '{}' failed: {e}", name);
                    }
                }
                self.importing = false;
                self.import_result = None;
                self.thumbnail_cache.clear();
            }
        }
    }

    /// Remove a pack from the library and registry.
    pub fn remove_pack(&mut self, pack_path: &str) {
        // Find pack_id from registry name
        let pack_name = self.registry.packs.iter()
            .find(|p| p.path == pack_path)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let pack_id = pack_name.replace(' ', "_").to_lowercase();

        // Remove all assets belonging to this pack
        let before = self.library.assets.len();
        self.library.assets.retain(|a| a.pack_id != pack_id);
        let removed = before - self.library.assets.len();
        self.library.packs.remove(&pack_id);

        // Also try removing by matching pack_roots
        self.library.pack_roots.remove(&pack_id);

        // Delete cached manifest so re-import does a fresh scan
        let manifest = std::path::Path::new(pack_path).join("toile-asset-manifest.json");
        if manifest.exists() {
            let _ = std::fs::remove_file(&manifest);
        }

        // Delete thumbnail cache folder
        let thumb_dir = std::path::Path::new(pack_path).join(".toile");
        if thumb_dir.exists() {
            let _ = std::fs::remove_dir_all(&thumb_dir);
        }

        crate::registry::unregister_pack(&mut self.registry, pack_path);
        self.thumbnail_cache.clear();
        self.preview_texture = None;
        self.preview_loaded_path.clear();
        self.selected_asset = None;
        self.filter_pack = None;
        self.highlight_file = None;
        self.readme_content = None;
        self.status_msg = format!("Removed '{}' ({} assets)", pack_name, removed);
    }

    /// Get filtered assets based on current search text and type filter.
    fn filtered_asset_ids(&self) -> Vec<String> {
        self.library
            .assets
            .iter()
            .filter(|a| {
                // Pack filter
                if let Some(ref fp) = self.filter_pack {
                    if a.pack_id != *fp {
                        return false;
                    }
                }
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
                        egui::TextureOptions::NEAREST,
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
                        egui::TextureOptions::NEAREST,
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
                    // Load at full resolution — egui handles display sizing
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    self.tex_counter += 1;
                    let tex = ctx.load_texture(
                        format!("preview_{}", self.tex_counter),
                        color_image,
                        egui::TextureOptions::NEAREST,
                    );
                    // Keep old texture alive one more frame
                    self.prev_preview_texture = self.preview_texture.take();
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

        // Load registered packs on first frame
        if !self.initialized {
            self.reload_registered_packs();
            self.initialized = true;
        }

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
        // Drop the previous preview texture (kept alive for one frame to avoid wgpu crash)
        self.prev_preview_texture = None;

        // Check for background import completion
        self.check_import_result();
        if self.importing {
            ctx.request_repaint(); // keep checking
        }

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
                if self.importing {
                    ui.separator();
                    let current = self.import_progress.load(std::sync::atomic::Ordering::Relaxed);
                    let total = self.import_total.load(std::sync::atomic::Ordering::Relaxed).max(1);
                    let pct = current as f32 / total as f32;
                    ui.add(egui::ProgressBar::new(pct)
                        .desired_width(200.0)
                        .text(format!("{}/{} files ({:.0}%)", current, total, pct * 100.0)));
                    ui.label(egui::RichText::new(&self.status_msg).color(egui::Color32::from_rgb(100, 200, 255)));
                } else if !self.status_msg.is_empty() {
                    ui.separator();
                    ui.label(egui::RichText::new(&self.status_msg).color(egui::Color32::YELLOW));
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

        // Left panel: imported packs list
        egui::SidePanel::left("packs_panel").default_width(200.0).show(ctx, |ui| {
            ui.heading("Packs");
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("📁 Folder").on_hover_text("Import from a folder").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Select Asset Pack Folder")
                        .pick_folder()
                    {
                        self.import_directory(&path);
                    }
                }
                if ui.button("📦 ZIP").on_hover_text("Import from a ZIP archive").clicked() {
                    self.import_pack_dialog();
                }
            });
            ui.add_space(8.0);

            if self.registry.packs.is_empty() {
                ui.label(egui::RichText::new("No packs imported.\nClick Import to add one.").color(egui::Color32::from_gray(130)));
            } else {
                // "All packs" button
                let all_selected = self.filter_pack.is_none();
                if ui.selectable_label(all_selected, egui::RichText::new("📦 All Packs").strong()).clicked() {
                    self.filter_pack = None;
                }
                ui.separator();

                let mut remove_path: Option<String> = None;
                let avail_w = ui.available_width();
                for pack in &self.registry.packs {
                    let pack_id = pack.name.replace(' ', "_").to_lowercase();
                    let is_selected = self.filter_pack.as_deref() == Some(&pack_id);
                    let count = self.library.assets.iter().filter(|a| a.pack_id == pack_id).count();

                    let btn_w = 18.0;
                    let label_w = avail_w - btn_w - 12.0;

                    ui.horizontal(|ui| {
                        // Truncate name to fit panel
                        let max_chars = (label_w / 7.0) as usize;
                        let display_name = if pack.name.len() > max_chars && max_chars > 3 {
                            format!("{}...", &pack.name[..max_chars - 3])
                        } else {
                            pack.name.clone()
                        };
                        let text = format!("📁 {} ({})", display_name, count);
                        let label = if is_selected {
                            egui::RichText::new(&text).strong().color(egui::Color32::YELLOW).size(12.0)
                        } else {
                            egui::RichText::new(&text).size(12.0)
                        };

                        // Left-aligned selectable label
                        let resp = ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.set_min_width(label_w);
                            ui.selectable_label(is_selected, label)
                        });
                        if resp.inner.clicked() {
                            if is_selected { self.filter_pack = None; }
                            else { self.filter_pack = Some(pack_id); }
                        }
                        // Remove button — fixed width
                        if ui.add_sized([btn_w, 18.0], egui::Button::new("x")).on_hover_text("Remove pack").clicked() {
                            remove_path = Some(pack.path.clone());
                        }
                    });
                }
                if let Some(path) = remove_path {
                    self.remove_pack(&path);
                    self.filter_pack = None;
                }
            }
        });

        // Central panel: browser or file view
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.view_mode {
                ViewMode::Assets => {
                    browser_panel::show_browser_panel(self, ui, ctx, &filtered_ids);
                }
                ViewMode::Files => {
                    file_browser::show_file_browser(self, ui, ctx);
                }
            }
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
