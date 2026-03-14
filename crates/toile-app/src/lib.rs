use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use toile_core::color::Color;
use toile_core::time::GameClock;
use toile_graphics::camera::Camera2D;
use toile_graphics::sprite_renderer::{DrawSprite, SpriteRenderer};
use toile_graphics::GpuContext;
use toile_platform::input::Input;
use toile_platform::WindowConfig;

pub use toile_core as core;
pub use toile_graphics as graphics;
pub use toile_platform as platform;

// Re-export commonly used types for convenience.
pub use toile_graphics::camera::Camera2D as Camera;
pub use toile_graphics::sprite_renderer::{DrawSprite as Sprite, COLOR_WHITE};
pub use toile_graphics::texture::TextureHandle;
pub use toile_platform::input::{Key, MouseButton};

/// Context passed to all `Game` trait methods.
/// Provides access to input, rendering, and camera.
pub struct GameContext<'a> {
    pub input: &'a Input,
    pub camera: &'a mut Camera2D,
    gpu: &'a GpuContext,
    renderer: &'a mut SpriteRenderer,
    draw_list: &'a mut Vec<DrawSprite>,
}

impl<'a> GameContext<'a> {
    /// Load a texture from a PNG file. Returns a handle for use in `draw_sprite`.
    pub fn load_texture(&mut self, path: &std::path::Path) -> TextureHandle {
        self.renderer
            .load_texture(self.gpu.device(), self.gpu.queue(), path)
    }

    /// Queue a sprite for drawing this frame.
    pub fn draw_sprite(&mut self, sprite: DrawSprite) {
        self.draw_list.push(sprite);
    }
}

/// Implement this trait to define your game.
pub trait Game {
    /// Called once after GPU context is ready. Load assets here.
    fn init(&mut self, ctx: &mut GameContext);

    /// Called at fixed timestep (default 60 Hz). Update game logic here.
    fn update(&mut self, ctx: &mut GameContext, dt: f64);

    /// Called each frame before rendering. Queue draw commands here.
    fn draw(&mut self, ctx: &mut GameContext);
}

/// Builder for the Toile application.
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

    /// Run the application with the given game. Blocks until the window is closed.
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
            input: Input::new(),
            clock: None,
            draw_list: Vec::new(),
            initialized: false,
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
    input: Input,
    clock: Option<GameClock>,
    draw_list: Vec<DrawSprite>,
    initialized: bool,
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

        log::info!("Window created: {}x{}", self.config.width, self.config.height);

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.renderer = Some(renderer);
        self.camera = Some(camera);
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

            WindowEvent::KeyboardInput { event, .. } => {
                self.input.handle_key_event(&event);
            }

            WindowEvent::MouseInput { button, state, .. } => {
                self.input.handle_mouse_button(button, state);
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.input.handle_cursor_moved(position.x, position.y);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.input.handle_mouse_wheel(&delta);
            }

            WindowEvent::RedrawRequested => {
                // Init game on first frame (GPU is ready)
                if !self.initialized {
                    self.initialized = true;
                    let gpu = self.gpu.as_ref().unwrap();
                    let mut ctx = GameContext {
                        input: &self.input,
                        camera: self.camera.as_mut().unwrap(),
                        gpu,
                        renderer: self.renderer.as_mut().unwrap(),
                        draw_list: &mut self.draw_list,
                    };
                    self.game.init(&mut ctx);
                }

                // Fixed timestep updates
                if let Some(clock) = &mut self.clock {
                    let (ticks, _alpha) = clock.advance();
                    let dt = clock.fixed_dt_secs();

                    for _ in 0..ticks {
                        let gpu = self.gpu.as_ref().unwrap();
                        let mut ctx = GameContext {
                            input: &self.input,
                            camera: self.camera.as_mut().unwrap(),
                            gpu,
                            renderer: self.renderer.as_mut().unwrap(),
                            draw_list: &mut self.draw_list,
                        };
                        self.game.update(&mut ctx, dt);
                    }
                }

                // Draw phase
                self.draw_list.clear();
                {
                    let gpu = self.gpu.as_ref().unwrap();
                    let mut ctx = GameContext {
                        input: &self.input,
                        camera: self.camera.as_mut().unwrap(),
                        gpu,
                        renderer: self.renderer.as_mut().unwrap(),
                        draw_list: &mut self.draw_list,
                    };
                    self.game.draw(&mut ctx);
                }

                // Render
                let gpu = self.gpu.as_mut().unwrap();
                if let Some((frame, view, mut encoder)) = gpu.begin_frame() {
                    let camera = self.camera.as_ref().unwrap();
                    let renderer = self.renderer.as_mut().unwrap();
                    renderer.draw(
                        gpu.device(),
                        gpu.queue(),
                        &mut encoder,
                        &view,
                        camera,
                        &self.draw_list,
                        &self.clear_color,
                    );
                    gpu.end_frame(frame, encoder);
                }

                // End-of-frame input snapshot
                self.input.end_frame();

                // Request next frame
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}
