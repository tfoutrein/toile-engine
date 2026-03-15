pub mod camera;
pub mod lighting;
pub mod post_processing;
pub mod sprite_renderer;
pub mod texture;

use std::sync::Arc;
use toile_core::color::Color;
use winit::window::Window;

pub struct GpuContext {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (u32, u32),
}

impl GpuContext {
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let scale = window.scale_factor();
        let (width, height) = (size.width.max(1), size.height.max(1));

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create wgpu surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find a suitable GPU adapter");

        log::info!(
            "GPU adapter: {} ({:?}), surface: {}x{}, scale_factor: {}",
            adapter.get_info().name,
            adapter.get_info().backend,
            width,
            height,
            scale,
        );

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("toile-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
        ))
        .expect("Failed to create GPU device");

        let config = surface
            .get_default_config(&adapter, width, height)
            .expect("Surface not supported by adapter");
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size: (width, height),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.size = (width, height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn begin_frame(
        &mut self,
    ) -> Option<(wgpu::SurfaceTexture, wgpu::TextureView, wgpu::CommandEncoder)> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return None;
            }
            Err(e) => {
                log::error!("Surface error: {e:?}");
                return None;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });
        Some((frame, view, encoder))
    }

    pub fn end_frame(&self, frame: wgpu::SurfaceTexture, encoder: wgpu::CommandEncoder) {
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn clear(&mut self, color: &Color) {
        if let Some((frame, view, mut encoder)) = self.begin_frame() {
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear((*color).into()),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
            }
            self.end_frame(frame, encoder);
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}
