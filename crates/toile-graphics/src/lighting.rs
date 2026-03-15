// Toile Engine — 2D point lighting system
//
// Usage (per-frame in draw()):
//   ctx.lighting.enabled = true;
//   ctx.lighting.ambient = [0.05, 0.05, 0.1, 1.0];
//   ctx.lighting.lights.push(Light { position: Vec2::new(0.0, 0.0), radius: 200.0, ..Default::default() });

use bytemuck::{Pod, Zeroable};
use glam::Vec2;

use crate::camera::Camera2D;

const MAX_LIGHTS: usize = 64;

// ── Public API ────────────────────────────────────────────────────────────────

/// A single point light in world space.
#[derive(Clone, Debug)]
pub struct Light {
    pub position:  Vec2,
    /// Radius in world units.
    pub radius:    f32,
    /// Falloff exponent: 1 = linear, 2 = smooth quadratic.
    pub falloff:   f32,
    /// Linear RGB colour (0–1).
    pub color:     [f32; 3],
    pub intensity: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            position:  Vec2::ZERO,
            radius:    150.0,
            falloff:   2.0,
            color:     [1.0, 1.0, 1.0],
            intensity: 1.0,
        }
    }
}

/// Per-frame lighting configuration — write this from `draw()` via `ctx.lighting`.
#[derive(Clone, Debug)]
pub struct LightingConfig {
    pub enabled: bool,
    /// Ambient light: rgb = colour (0–1 each), w = intensity.
    pub ambient: [f32; 4],
    pub lights:  Vec<Light>,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ambient: [0.05, 0.05, 0.1, 1.0],
            lights:  Vec::new(),
        }
    }
}

// ── GPU uniform structs (must match light_apply.wgsl exactly) ─────────────────

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct LightEntryGpu {
    position:  [f32; 2],  //  8 bytes
    radius:    f32,        //  4 bytes
    falloff:   f32,        //  4 bytes — total 16
    color:     [f32; 3],  // 12 bytes
    intensity: f32,        //  4 bytes — total 32
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct LightsUniformGpu {
    ambient:       [f32; 4],                   // 16 bytes  (offset   0)
    camera_pos:    [f32; 2],                   //  8 bytes  (offset  16)
    viewport_half: [f32; 2],                   //  8 bytes  (offset  24)
    light_count:   u32,                        //  4 bytes  (offset  32)
    _pad:          [u32; 3],                   // 12 bytes  (offset  36)
    lights:        [LightEntryGpu; MAX_LIGHTS], // 2048 bytes (offset 48)
}                                              // total: 2096 bytes

// ── LightingSystem ────────────────────────────────────────────────────────────

pub struct LightingSystem {
    /// Output texture containing scene × lightmap.
    pub output_texture: wgpu::Texture,
    pub output_view:    wgpu::TextureView,
    /// Bind group for `output_texture` using the PostProcessor's shared `tex_bgl`.
    /// Pass this to `PostProcessor::apply_from()` as the source override.
    pub output_bg:      wgpu::BindGroup,

    pipeline:   wgpu::RenderPipeline,
    lights_buf: wgpu::Buffer,
    lights_bg:  wgpu::BindGroup,

    pub size: (u32, u32),
    surface_format: wgpu::TextureFormat,
}

impl LightingSystem {
    /// Create the lighting system.
    ///
    /// `pp_tex_bgl` and `pp_sampler` must come from `PostProcessor` (they are
    /// shared so the output bind-group is usable in PP's pipelines).
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        pp_tex_bgl: &wgpu::BindGroupLayout,
        pp_sampler: &wgpu::Sampler,
    ) -> Self {
        let (w, h) = (width.max(1), height.max(1));

        // ── Output texture ────────────────────────────────────────────────
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("lighting_output"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ── Output bind group (readable as PostProcessor pass input) ──────
        let output_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting_output_bg"),
            layout: pp_tex_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(pp_sampler),
                },
            ],
        });

        // ── Lights uniform buffer & bind group layout ─────────────────────
        let lights_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting_uniform_buf"),
            size: std::mem::size_of::<LightsUniformGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let lights_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lighting_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let lights_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting_bg"),
            layout: &lights_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights_buf.as_entire_binding(),
            }],
        });

        // ── Pipeline ──────────────────────────────────────────────────────
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lighting_layout"),
            bind_group_layouts: &[pp_tex_bgl, &lights_bgl],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("light_apply"),
            source: wgpu::ShaderSource::Wgsl(include_str!("light_apply.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lighting_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            output_texture,
            output_view,
            output_bg,
            pipeline,
            lights_buf,
            lights_bg,
            size: (w, h),
            surface_format,
        }
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        pp_tex_bgl: &wgpu::BindGroupLayout,
        pp_sampler: &wgpu::Sampler,
    ) {
        *self = Self::new(device, self.surface_format, width, height, pp_tex_bgl, pp_sampler);
    }

    /// Apply lighting: reads `scene_bg`, writes scene×lightmap to `output_texture`.
    pub fn apply(
        &self,
        config: &LightingConfig,
        camera: &Camera2D,
        scene_bg: &wgpu::BindGroup,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // Build GPU uniform
        let mut gpu_lights = [LightEntryGpu {
            position: [0.0; 2],
            radius: 0.0,
            falloff: 0.0,
            color: [0.0; 3],
            intensity: 0.0,
        }; MAX_LIGHTS];

        let count = config.lights.len().min(MAX_LIGHTS);
        for (i, l) in config.lights.iter().take(MAX_LIGHTS).enumerate() {
            gpu_lights[i] = LightEntryGpu {
                position:  [l.position.x, l.position.y],
                radius:    l.radius,
                falloff:   l.falloff,
                color:     l.color,
                intensity: l.intensity,
            };
        }

        let half = camera.half_viewport();
        let uniform = LightsUniformGpu {
            ambient:       config.ambient,
            camera_pos:    [camera.position.x, camera.position.y],
            viewport_half: [half.x, half.y],
            light_count:   count as u32,
            _pad:          [0; 3],
            lights:        gpu_lights,
        };

        queue.write_buffer(&self.lights_buf, 0, bytemuck::bytes_of(&uniform));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("lighting_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, scene_bg, &[]);
        pass.set_bind_group(1, &self.lights_bg, &[]);
        pass.draw(0..3, 0..1);
    }
}
