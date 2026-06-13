//! Headless wgpu context: a GPU device + an off-screen render target that can be
//! read back to CPU memory and saved as a PNG. No window or surface is created,
//! so this works without a display (the engine's [`toile_graphics::GpuContext`]
//! is surface-bound and cannot).
//!
//! This is the foundation of the visual test harness: render anything into
//! [`Headless::view`] with the engine's real renderers, then call
//! [`Headless::save_png`] / [`Headless::pixels`] to inspect the result.

use std::path::Path;

/// RGBA8 off-screen render target backed by a real GPU device.
pub struct Headless {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
    pub adapter_name: String,
    target: wgpu::Texture,
    /// Render into this view with `SpriteRenderer::draw`, `SdfTextRenderer`, etc.
    pub view: wgpu::TextureView,
}

impl Headless {
    /// Create an off-screen target of the given size. Returns an error string
    /// (rather than panicking) when no GPU adapter is available, so tests can
    /// skip gracefully on machines without a usable GPU.
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let width = width.max(1);
        let height = height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            // No surface: this is the whole point of "headless".
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("no suitable GPU adapter: {e:?}"))?;

        let adapter_name = adapter.get_info().name;
        log::info!(
            "Headless GPU adapter: {} ({:?}), target: {width}x{height}",
            adapter_name,
            adapter.get_info().backend,
        );

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("toile-harness-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .map_err(|e| format!("failed to create GPU device: {e:?}"))?;

        // Match the engine's default texture format so renderers behave identically.
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("toile-harness-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            // RENDER_ATTACHMENT so renderers can draw into it; COPY_SRC so we can read it back.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self {
            device,
            queue,
            format,
            width,
            height,
            adapter_name,
            target,
            view,
        })
    }

    /// Copy the off-screen target back to CPU memory as tightly-packed RGBA8
    /// (row stride == `width * 4`, padding removed).
    pub fn pixels(&self) -> Result<Vec<u8>, String> {
        let bpp = 4u32;
        let unpadded_bpr = self.width * bpp;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bpr = unpadded_bpr.div_ceil(align) * align;

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("toile-harness-readback"),
            size: (padded_bpr * self.height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("toile-harness-readback-encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bpr),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map the readback buffer and block until the GPU is done.
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                let _ = tx.send(res);
            });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("device poll failed: {e:?}"))?;
        rx.recv()
            .map_err(|e| format!("map channel dropped: {e}"))?
            .map_err(|e| format!("buffer map failed: {e:?}"))?;

        let data = buffer.slice(..).get_mapped_range();
        let mut out = Vec::with_capacity((unpadded_bpr * self.height) as usize);
        for row in 0..self.height {
            let start = (row * padded_bpr) as usize;
            let end = start + unpadded_bpr as usize;
            out.extend_from_slice(&data[start..end]);
        }
        drop(data);
        buffer.unmap();
        Ok(out)
    }

    /// Read the target back and write it to a PNG file.
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let path = path.as_ref();
        let pixels = self.pixels()?;
        let img = image::RgbaImage::from_raw(self.width, self.height, pixels)
            .ok_or_else(|| "pixel buffer size mismatch".to_string())?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        img.save(path)
            .map_err(|e| format!("failed to write PNG {}: {e}", path.display()))?;
        Ok(())
    }
}
