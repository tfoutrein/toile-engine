use std::sync::Arc;
use std::time::Duration;

use glam::Vec2;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use toile_assets::font::Font;
use toile_assets::sdf_font::SdfFont;
use toile_core::color::Color;
use toile_core::time::GameClock;
use toile_graphics::camera::Camera2D;
use toile_graphics::lighting::LightingSystem;
use toile_graphics::post_processing::PostProcessor;
use toile_graphics::sdf_text::{DrawSdfGlyph, SdfTextRenderer};
use toile_graphics::sprite_renderer::{DrawSprite, RenderStats, SpriteRenderer};
use toile_graphics::GpuContext;
use toile_platform::input::Input;
use toile_platform::WindowConfig;

pub mod scene;

pub use toile_core as core;
pub use toile_graphics as graphics;
pub use toile_platform as platform;
pub use toile_audio as audio;
pub use toile_collision as collision;
pub use toile_ecs as ecs;
pub use toile_assets as assets;
pub use toile_scripting as scripting;

pub use toile_assets::font::FontHandle;
pub use toile_assets::sdf_font::MsdfFontHandle;
pub use toile_audio::{MusicId, PlaybackId, SoundId};
pub use toile_graphics::camera::Camera2D as Camera;
pub use toile_graphics::custom_shader::CustomShaderPipeline;
pub use toile_graphics::lighting::{Light, LightingConfig, ShadowConfig};
pub use toile_graphics::post_processing::{PostEffect, PostProcessingStack};
pub use toile_graphics::shader_graph::{NodeKind, ShaderEdge, ShaderGraph, ShaderNode};
pub use toile_graphics::sprite_renderer::{DrawSprite as Sprite, COLOR_WHITE};
pub use toile_graphics::texture::TextureHandle;
pub use toile_platform::input::{Key, MouseButton};

/// Style parameters for `draw_text_msdf`.
#[derive(Clone)]
pub struct TextStyle {
    /// Display size in world units.
    pub size:          f32,
    /// Fill colour — packed RGBA (use `Color::pack()` or `0xRRGGBBAA`).
    pub color:         u32,
    /// Outline thickness in SDF fraction space (0.0 = none, 0.15 = thick).
    pub outline_width: f32,
    /// Outline colour.
    pub outline_color: u32,
    /// Shadow offset in world units.  Zero = no shadow.
    pub shadow_offset: Vec2,
    /// Shadow fill colour (alpha controls opacity).
    pub shadow_color:  u32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size:          16.0,
            color:         0xFFFF_FFFF,   // white, fully opaque
            outline_width: 0.0,
            outline_color: 0xFF00_0000,   // black, a=255  (packed: r=0,g=0,b=0,a=0xFF in high byte)
            shadow_offset: Vec2::ZERO,
            shadow_color:  0x80_000000,   // 50% transparent black
        }
    }
}

/// Context passed to all `Game` trait methods.
pub struct GameContext<'a> {
    pub input: &'a mut Input,
    pub camera: &'a mut Camera2D,
    pub audio: &'a mut toile_audio::Audio,
    pub stats: &'a RenderStats,
    pub fps: f64,
    /// True only during the first fixed-update tick of each frame.
    /// Use this to guard one-shot actions (toggles) in update().
    pub first_tick: bool,
    /// Configure post-processing effects applied after sprite rendering.
    pub post_processing: &'a mut PostProcessingStack,
    /// Configure per-frame point lighting.  Set `lighting.enabled = true` and push lights.
    pub lighting: &'a mut LightingConfig,
    gpu: &'a GpuContext,
    renderer: &'a mut SpriteRenderer,
    fonts: &'a mut Vec<Font>,
    draw_list: &'a mut Vec<DrawSprite>,
    post_processor: &'a Option<toile_graphics::post_processing::PostProcessor>,
    sdf_renderer: &'a mut SdfTextRenderer,
    sdf_fonts: &'a mut Vec<SdfFont>,
    sdf_draw_list: &'a mut Vec<DrawSdfGlyph>,
}

impl<'a> GameContext<'a> {
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.gpu.surface_format()
    }

    /// Compile a `ShaderGraph` into a `CustomShaderPipeline` ready for use as
    /// `PostEffect::Custom(pipeline)`.  Returns `None` and logs an error on failure.
    pub fn compile_shader_graph(
        &self,
        graph: &ShaderGraph,
    ) -> Option<std::sync::Arc<CustomShaderPipeline>> {
        let wgsl = match graph.compile() {
            Ok(w) => w,
            Err(e) => { log::error!("Shader graph compile error: {e}"); return None; }
        };
        self.compile_shader_wgsl(&graph.name, &wgsl)
    }

    /// Compile raw WGSL (must conform to the custom-shader layout) into a pipeline.
    pub fn compile_shader_wgsl(
        &self,
        name: &str,
        wgsl: &str,
    ) -> Option<std::sync::Arc<CustomShaderPipeline>> {
        let pp = self.post_processor.as_ref()?;
        match CustomShaderPipeline::new(
            name,
            self.gpu.device(),
            self.gpu.surface_format(),
            &pp.tex_bgl,
            wgsl,
        ) {
            Ok(p)  => Some(p),
            Err(e) => { log::error!("Custom shader WGSL error: {e}"); None }
        }
    }

    pub fn load_texture(&mut self, path: &std::path::Path) -> TextureHandle {
        self.renderer
            .load_texture(self.gpu.device(), self.gpu.queue(), path)
    }

    pub fn create_texture_from_rgba(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> TextureHandle {
        self.renderer
            .create_texture_from_rgba(self.gpu.device(), self.gpu.queue(), data, width, height)
    }

    /// Load a TTF font, rasterizing ASCII glyphs at the given pixel size.
    pub fn load_ttf(&mut self, path: &std::path::Path, px_size: f32) -> FontHandle {
        let ttf_bytes = std::fs::read(path)
            .unwrap_or_else(|e| panic!("Failed to read TTF {}: {e}", path.display()));
        let result = toile_assets::ttf::rasterize_ascii(&ttf_bytes, px_size);
        let tex = self.create_texture_from_rgba(
            &result.atlas_rgba,
            result.atlas_width,
            result.atlas_height,
        );
        let font = Font {
            texture: tex,
            line_height: result.line_height,
            glyphs: result.glyphs,
        };
        let handle = FontHandle(self.fonts.len() as u32);
        self.fonts.push(font);
        log::info!("Loaded TTF: {} at {}px -> {:?}", path.display(), px_size, handle);
        handle
    }

    /// Load a TTF as an SDF atlas (rasterizes ASCII at `px_size` into a distance-field texture).
    /// The returned `MsdfFontHandle` is used with `draw_text_msdf`.
    pub fn load_msdf_font(&mut self, path: &std::path::Path, px_size: f32) -> MsdfFontHandle {
        let ttf_bytes = std::fs::read(path)
            .unwrap_or_else(|e| panic!("Failed to read TTF {}: {e}", path.display()));
        let result = toile_assets::sdf_font::rasterize_sdf(&ttf_bytes, px_size);
        let texture_idx = self.sdf_renderer.create_sdf_texture(
            self.gpu.device(),
            self.gpu.queue(),
            &result.atlas_r8,
            result.atlas_width,
            result.atlas_height,
        );
        let font = SdfFont {
            texture_idx,
            line_height: result.line_height,
            glyphs:      result.glyphs,
            spread_px:   result.spread_px,
            ref_px:      result.ref_px,
        };
        let handle = MsdfFontHandle(self.sdf_fonts.len() as u32);
        self.sdf_fonts.push(font);
        log::info!("Loaded MSDF font: {} at {}px -> {:?}", path.display(), px_size, handle);
        handle
    }

    /// Draw text using the SDF font renderer.  Supports crisp scaling, outline, and drop shadow.
    pub fn draw_text_msdf(
        &mut self,
        text:   &str,
        position: Vec2,
        font:   MsdfFontHandle,
        style:  &TextStyle,
        layer:  i32,
    ) {
        let sdf_font = &self.sdf_fonts[font.0 as usize];
        let scale        = style.size / sdf_font.ref_px;
        let texture_idx  = sdf_font.texture_idx;

        let mut pen_x = position.x;
        let     pen_y = position.y;

        // Collect glyph data first to avoid borrowing sdf_fonts while pushing
        let mut quads: Vec<(Vec2, Vec2, Vec2, Vec2, f32)> = Vec::new(); // (pos, size, uv_min, uv_max, advance_x)
        for ch in text.chars() {
            let Some(glyph) = sdf_font.glyphs.get(&(ch as u32)) else { continue };
            if glyph.size.x > 0.0 && glyph.size.y > 0.0 {
                let gw = glyph.size.x * scale;
                let gh = glyph.size.y * scale;
                let cx = pen_x + glyph.offset.x * scale + gw / 2.0;
                let cy = pen_y + glyph.offset.y * scale + gh / 2.0;
                quads.push((
                    Vec2::new(cx, cy),
                    Vec2::new(gw, gh),
                    glyph.uv_min,
                    glyph.uv_max,
                    glyph.advance * scale,
                ));
            } else {
                // Non-drawing glyph (e.g. space): just advance
                quads.push((Vec2::ZERO, Vec2::ZERO, Vec2::ZERO, Vec2::ZERO, glyph.advance * scale));
            }
            pen_x += glyph.advance * scale;
        }

        let shadow_a = ((style.shadow_color >> 24) & 0xFF) as u8;
        let has_shadow = shadow_a > 0
            && (style.shadow_offset.x != 0.0 || style.shadow_offset.y != 0.0);

        for (pos, sz, uv_min, uv_max, _) in &quads {
            if sz.x <= 0.0 { continue; }
            // Shadow quad (drawn first → renders behind fill)
            if has_shadow {
                self.sdf_draw_list.push(DrawSdfGlyph {
                    texture_idx,
                    position:      *pos + style.shadow_offset,
                    size:          *sz,
                    layer,
                    uv_min:        *uv_min,
                    uv_max:        *uv_max,
                    fill_color:    style.shadow_color,
                    outline_color: 0,
                    outline_width: 0.0,
                });
            }
            // Main fill + outline
            self.sdf_draw_list.push(DrawSdfGlyph {
                texture_idx,
                position:      *pos,
                size:          *sz,
                layer,
                uv_min:        *uv_min,
                uv_max:        *uv_max,
                fill_color:    style.color,
                outline_color: style.outline_color,
                outline_width: style.outline_width,
            });
        }
    }

    pub fn draw_sprite(&mut self, sprite: DrawSprite) {
        self.draw_list.push(sprite);
    }

    /// Access the last sprite in the draw list (for post-modification).
    pub fn draw_list_last_mut(&mut self) -> Option<&mut DrawSprite> {
        self.draw_list.last_mut()
    }

    /// Draw text at the given position (top-left corner in world space).
    pub fn draw_text(
        &mut self,
        text: &str,
        position: Vec2,
        font_handle: FontHandle,
        size: f32,
        color: u32,
        layer: i32,
    ) {
        let font = &self.fonts[font_handle.0 as usize];
        let scale = size / font.line_height;
        let texture = font.texture;

        let mut pen_x = position.x;
        let pen_y = position.y;

        for ch in text.chars() {
            let codepoint = ch as u32;
            let Some(glyph) = font.glyphs.get(&codepoint) else {
                continue;
            };

            if glyph.size.x > 0.0 && glyph.size.y > 0.0 {
                let gw = glyph.size.x * scale;
                let gh = glyph.size.y * scale;

                // Center position for the sprite renderer
                let cx = pen_x + glyph.offset.x * scale + gw / 2.0;
                let cy = pen_y + glyph.offset.y * scale + gh / 2.0;

                self.draw_list.push(DrawSprite {
                    texture,
                    position: Vec2::new(cx, cy),
                    size: Vec2::new(gw, gh),
                    rotation: 0.0,
                    color,
                    layer,
                    uv_min: glyph.uv_min,
                    uv_max: glyph.uv_max,
                });
            }

            pen_x += glyph.advance * scale;
        }
    }
}

pub trait Game {
    fn init(&mut self, ctx: &mut GameContext);
    fn update(&mut self, ctx: &mut GameContext, dt: f64);
    fn draw(&mut self, ctx: &mut GameContext);

    /// Optional: render an overlay (e.g., egui) after sprites, before frame submission.
    fn render_overlay(
        &mut self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
        _view: &wgpu::TextureView,
        _window: &winit::window::Window,
        _size: (u32, u32),
    ) {
    }

    /// Optional: filter a window event through overlay (return true if consumed).
    fn handle_window_event(
        &mut self,
        _window: &winit::window::Window,
        _event: &WindowEvent,
    ) -> bool {
        false
    }

    /// Optional egui UI built each frame. Called during render_overlay by EguiGameApp helper.
    fn egui_ui(&mut self, _ctx: &egui::Context, _game_ctx: &mut GameContext) {}
}

/// Re-export egui for use in game examples without adding it as a direct dependency.
pub use egui;

pub use wgpu;

pub struct App {
    config: WindowConfig,
    clear_color: Color,
    update_hz: u32,
}

impl App {
    pub fn new() -> Self {
        Self {
            config: WindowConfig::default(),
            clear_color: Color::CORNFLOWER_BLUE,
            update_hz: 60,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    pub fn with_clear_color(mut self, color: Color) -> Self {
        self.clear_color = color;
        self
    }

    pub fn with_update_hz(mut self, hz: u32) -> Self {
        self.update_hz = hz;
        self
    }

    pub fn run(self, game: impl Game + 'static) {
        env_logger::init();
        log::info!("Toile Engine v{}", env!("CARGO_PKG_VERSION"));

        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let mut handler = AppHandler {
            config: self.config,
            clear_color: self.clear_color,
            update_hz: self.update_hz,
            game: Box::new(game),
            window: None,
            gpu: None,
            renderer: None,
            camera: None,
            audio: None,
            fonts: Vec::new(),
            input: Input::new(),
            clock: None,
            draw_list: Vec::new(),
            last_stats: RenderStats::default(),
            initialized: false,
            debug_overlay: false,
            debug_title_timer: Duration::ZERO,
            post_processor: None,
            post_stack: PostProcessingStack::default(),
            lighting_system: None,
            lighting_config: LightingConfig::default(),
            elapsed_secs: 0.0,
            sdf_renderer: None,
            sdf_fonts: Vec::new(),
            sdf_draw_list: Vec::new(),
        };

        event_loop.run_app(&mut handler).expect("Event loop error");
    }
}

struct AppHandler {
    config: WindowConfig,
    clear_color: Color,
    update_hz: u32,
    game: Box<dyn Game>,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    renderer: Option<SpriteRenderer>,
    camera: Option<Camera2D>,
    audio: Option<toile_audio::Audio>,
    fonts: Vec<Font>,
    input: Input,
    clock: Option<GameClock>,
    draw_list: Vec<DrawSprite>,
    last_stats: RenderStats,
    initialized: bool,
    debug_overlay: bool,
    debug_title_timer: Duration,
    post_processor: Option<PostProcessor>,
    post_stack: PostProcessingStack,
    lighting_system: Option<LightingSystem>,
    lighting_config: LightingConfig,
    elapsed_secs: f32,
    sdf_renderer: Option<SdfTextRenderer>,
    sdf_fonts: Vec<SdfFont>,
    sdf_draw_list: Vec<DrawSdfGlyph>,
}

macro_rules! make_ctx {
    ($self:ident, $fps:expr) => {
        GameContext {
            input: &mut $self.input,
            camera: $self.camera.as_mut().unwrap(),
            audio: $self.audio.as_mut().unwrap(),
            stats: &$self.last_stats,
            fps: $fps,
            gpu: $self.gpu.as_ref().unwrap(),
            renderer: $self.renderer.as_mut().unwrap(),
            first_tick: true,
            fonts: &mut $self.fonts,
            draw_list: &mut $self.draw_list,
            post_processing: &mut $self.post_stack,
            lighting: &mut $self.lighting_config,
            post_processor: &$self.post_processor,
            sdf_renderer: $self.sdf_renderer.as_mut().unwrap(),
            sdf_fonts: &mut $self.sdf_fonts,
            sdf_draw_list: &mut $self.sdf_draw_list,
        }
    };
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
            .with_resizable(self.config.resizable);

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );

        let gpu = GpuContext::new(window.clone());
        let renderer = SpriteRenderer::new(gpu.device(), gpu.surface_format());
        let sdf_renderer = SdfTextRenderer::new(gpu.device(), gpu.surface_format());
        // Use logical size (not physical) for camera so content scales correctly on HiDPI
        let camera = Camera2D::new(self.config.width as f32, self.config.height as f32);
        self.input.set_scale_factor(window.scale_factor());
        let audio = toile_audio::Audio::new().expect("Failed to initialize audio");
        let (pw, ph) = gpu.size();
        let post_processor = PostProcessor::new(gpu.device(), gpu.surface_format(), pw, ph);
        let lighting_system = LightingSystem::new(
            gpu.device(), gpu.surface_format(), pw, ph,
            &post_processor.tex_bgl, &post_processor.sampler,
        );

        log::info!("Window created: {}x{}", self.config.width, self.config.height);

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.renderer = Some(renderer);
        self.sdf_renderer = Some(sdf_renderer);
        self.camera = Some(camera);
        self.audio = Some(audio);
        self.clock = Some(GameClock::new(self.update_hz));
        self.post_processor = Some(post_processor);
        self.lighting_system = Some(lighting_system);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Exiting");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                // Camera uses logical pixels (divide physical by scale factor)
                if let Some(camera) = &mut self.camera {
                    let scale = self.window.as_ref()
                        .map(|w| w.scale_factor() as f32)
                        .unwrap_or(1.0);
                    camera.resize(size.width as f32 / scale, size.height as f32 / scale);
                }
                // Recreate post-processing and lighting textures for the new physical size
                if let Some(gpu) = &self.gpu {
                    let (w, h) = (size.width.max(1), size.height.max(1));
                    if let Some(pp) = &mut self.post_processor {
                        pp.resize(gpu.device(), w, h);
                    }
                    if let (Some(pp), Some(ls)) =
                        (&self.post_processor, &mut self.lighting_system)
                    {
                        ls.resize(gpu.device(), w, h, &pp.tex_bgl, &pp.sampler);
                    }
                }
            }
            // Let overlay (egui) consume events first
            ref e @ (WindowEvent::KeyboardInput { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::ModifiersChanged { .. }) => {
                let consumed = if let Some(window) = &self.window {
                    self.game.handle_window_event(window, e)
                } else {
                    false
                };
                if !consumed {
                    match e {
                        WindowEvent::KeyboardInput { event, .. } => {
                            self.input.handle_key_event(event);
                        }
                        WindowEvent::MouseInput { button, state, .. } => {
                            self.input.handle_mouse_button(*button, *state);
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            self.input.handle_cursor_moved(position.x, position.y);
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            self.input.handle_mouse_wheel(delta);
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if !self.initialized {
                    self.initialized = true;
                    let mut ctx = make_ctx!(self, 0.0);
                    self.game.init(&mut ctx);
                }

                let clock = self.clock.as_mut().unwrap();
                let (ticks, _alpha) = clock.advance();
                let dt = clock.fixed_dt_secs();
                let fps = clock.fps();
                self.elapsed_secs += (ticks as f64 * dt) as f32;

                for tick_idx in 0..ticks {
                    let mut ctx = make_ctx!(self, fps);
                    ctx.first_tick = tick_idx == 0;
                    self.game.update(&mut ctx, dt);
                }

                self.draw_list.clear();
                self.sdf_draw_list.clear();
                self.lighting_config.lights.clear();
                {
                    let mut ctx = make_ctx!(self, fps);
                    self.game.draw(&mut ctx);
                }

                let gpu = self.gpu.as_mut().unwrap();
                if let Some((frame, view, mut encoder)) = gpu.begin_frame() {
                    let camera = self.camera.as_ref().unwrap();
                    let use_lighting = self.lighting_config.enabled
                        && self.lighting_system.is_some()
                        && self.post_processor.is_some();
                    let use_pp = self.post_stack.enabled
                        && !self.post_stack.effects.is_empty()
                        && self.post_processor.is_some();
                    let need_offscreen = use_lighting || use_pp;

                    // 1a. Render sprites — into scene texture if any post/lighting, else direct
                    {
                        let render_target = if need_offscreen {
                            &self.post_processor.as_ref().unwrap().scene_view
                        } else {
                            &view
                        };
                        let renderer = self.renderer.as_mut().unwrap();
                        self.last_stats = renderer.draw(
                            gpu.device(),
                            gpu.queue(),
                            &mut encoder,
                            render_target,
                            camera,
                            &self.draw_list,
                            &self.clear_color,
                        );
                    }

                    // 1b. SDF text on top of sprites (LoadOp::Load — no clear)
                    if !self.sdf_draw_list.is_empty() {
                        let render_target = if need_offscreen {
                            &self.post_processor.as_ref().unwrap().scene_view
                        } else {
                            &view
                        };
                        let sdf = self.sdf_renderer.as_mut().unwrap();
                        sdf.draw(
                            gpu.device(),
                            gpu.queue(),
                            &mut encoder,
                            render_target,
                            camera,
                            &self.sdf_draw_list,
                        );
                    }

                    // 2. Lighting pass: scene_bg → lighting output texture
                    if use_lighting {
                        let ls = self.lighting_system.as_ref().unwrap();
                        let pp = self.post_processor.as_ref().unwrap();
                        ls.apply(
                            &self.lighting_config, camera,
                            &pp.scene_bg, gpu.queue(), &mut encoder,
                        );
                    }

                    // 3. Post-processing (or passthrough blit if only lighting)
                    if need_offscreen {
                        let pp = self.post_processor.as_ref().unwrap();
                        let src = if use_lighting {
                            Some(&self.lighting_system.as_ref().unwrap().output_bg)
                        } else {
                            None
                        };
                        pp.apply_from(&self.post_stack, src, &view, self.elapsed_secs, gpu.queue(), &mut encoder);
                    }

                    // Overlay rendering (egui for editor, no-op for regular games)
                    let size = gpu.size();
                    let device = gpu.device();
                    let queue = gpu.queue();
                    if let Some(window) = &self.window {
                        self.game
                            .render_overlay(device, queue, &mut encoder, &view, window, size);
                    }

                    gpu.end_frame(frame, encoder);
                }

                if self.input.is_key_just_pressed(toile_platform::input::Key::F3) {
                    self.debug_overlay = !self.debug_overlay;
                    if !self.debug_overlay {
                        if let Some(window) = &self.window {
                            window.set_title(&self.config.title);
                        }
                    }
                }

                if self.debug_overlay {
                    self.debug_title_timer +=
                        Duration::from_secs_f64(clock.frame_time_ms() / 1000.0);
                    if self.debug_title_timer >= Duration::from_millis(250) {
                        self.debug_title_timer = Duration::ZERO;
                        let s = &self.last_stats;
                        if let Some(window) = &self.window {
                            window.set_title(&format!(
                                "{} | FPS: {:.0} | {:.1}ms | sprites: {} | batches: {} | draws: {}",
                                self.config.title,
                                fps,
                                clock.frame_time_ms(),
                                s.sprite_count,
                                s.batch_count,
                                s.draw_calls,
                            ));
                        }
                    }
                }

                self.input.end_frame(ticks > 0);

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
