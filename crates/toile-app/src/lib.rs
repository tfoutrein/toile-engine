use std::sync::Arc;
use std::time::Duration;

use glam::Vec2;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use toile_assets::font::Font;
use toile_core::color::Color;
use toile_core::time::GameClock;
use toile_graphics::camera::Camera2D;
use toile_graphics::sprite_renderer::{DrawSprite, RenderStats, SpriteRenderer};
use toile_graphics::GpuContext;
use toile_platform::input::Input;
use toile_platform::WindowConfig;

pub use toile_core as core;
pub use toile_graphics as graphics;
pub use toile_platform as platform;
pub use toile_audio as audio;
pub use toile_collision as collision;
pub use toile_ecs as ecs;
pub use toile_assets as assets;
pub use toile_scripting as scripting;

pub use toile_assets::font::FontHandle;
pub use toile_audio::{MusicId, PlaybackId, SoundId};
pub use toile_graphics::camera::Camera2D as Camera;
pub use toile_graphics::sprite_renderer::{DrawSprite as Sprite, COLOR_WHITE};
pub use toile_graphics::texture::TextureHandle;
pub use toile_platform::input::{Key, MouseButton};

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
    gpu: &'a GpuContext,
    renderer: &'a mut SpriteRenderer,
    fonts: &'a mut Vec<Font>,
    draw_list: &'a mut Vec<DrawSprite>,
}

impl<'a> GameContext<'a> {
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.gpu.surface_format()
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

    pub fn draw_sprite(&mut self, sprite: DrawSprite) {
        self.draw_list.push(sprite);
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
}

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
        let (w, h) = gpu.size();
        let camera = Camera2D::new(w as f32, h as f32);
        let audio = toile_audio::Audio::new().expect("Failed to initialize audio");

        log::info!("Window created: {}x{}", self.config.width, self.config.height);

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.renderer = Some(renderer);
        self.camera = Some(camera);
        self.audio = Some(audio);
        self.clock = Some(GameClock::new(self.update_hz));
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
                if let Some(camera) = &mut self.camera {
                    camera.resize(size.width as f32, size.height as f32);
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

                for tick_idx in 0..ticks {
                    let mut ctx = make_ctx!(self, fps);
                    ctx.first_tick = tick_idx == 0;
                    self.game.update(&mut ctx, dt);
                }

                self.draw_list.clear();
                {
                    let mut ctx = make_ctx!(self, fps);
                    self.game.draw(&mut ctx);
                }

                let gpu = self.gpu.as_mut().unwrap();
                if let Some((frame, view, mut encoder)) = gpu.begin_frame() {
                    let camera = self.camera.as_ref().unwrap();
                    let renderer = self.renderer.as_mut().unwrap();
                    self.last_stats = renderer.draw(
                        gpu.device(),
                        gpu.queue(),
                        &mut encoder,
                        &view,
                        camera,
                        &self.draw_list,
                        &self.clear_color,
                    );

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

                self.input.end_frame();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
