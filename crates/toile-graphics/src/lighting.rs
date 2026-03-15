// Toile Engine — 2D lighting + shadow system (v0.4)
//
// Usage (per-frame in draw()):
//   ctx.lighting.enabled        = true;
//   ctx.lighting.ambient        = [0.03, 0.03, 0.06, 1.0];
//   ctx.lighting.shadow.enabled = true;           // enable 1D shadow maps
//   ctx.lighting.lights.push(Light {
//       position: Vec2::new(0.0, 0.0),
//       radius: 200.0,
//       cast_shadow: true,        // this light casts shadows
//       ..Default::default()
//   });
//
// Shadow occlusion convention:
//   The background must be cleared with alpha = 0 (transparent) so it does NOT
//   block light.  Sprite pixels are always alpha = 255, so they block light.
//   Use `App::with_clear_color(Color::new(r, g, b, 0.0))` in shadow demos.

use bytemuck::{Pod, Zeroable};
use glam::Vec2;

use crate::camera::Camera2D;

const MAX_LIGHTS:        usize = 64;
const MAX_SHADOW_LIGHTS: usize = 8;
/// Default number of ray-march steps when building shadow maps.
const SHADOW_STEPS: u32 = 64;

// ── Public API ────────────────────────────────────────────────────────────────

/// A single point light in world space.
#[derive(Clone, Debug)]
pub struct Light {
    pub position:     Vec2,
    /// Radius in world units.
    pub radius:       f32,
    /// Falloff exponent: 1 = linear, 2 = smooth quadratic.
    pub falloff:      f32,
    /// Linear RGB colour (0–1 each channel).
    pub color:        [f32; 3],
    pub intensity:    f32,
    /// Whether this light casts shadows via 1D shadow maps.
    /// Requires `LightingConfig::shadow.enabled = true`.
    pub cast_shadow:  bool,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            position:    Vec2::ZERO,
            radius:      150.0,
            falloff:     2.0,
            color:       [1.0, 1.0, 1.0],
            intensity:   1.0,
            cast_shadow: false,
        }
    }
}

/// Shadow configuration — part of `LightingConfig`.
#[derive(Clone, Debug)]
pub struct ShadowConfig {
    pub enabled:    bool,
    /// Angular resolution (rays per light, = width of the 1D shadow map).
    pub resolution: u32,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self { enabled: true, resolution: 360 }
    }
}

/// Per-frame lighting configuration — write this from `draw()` via `ctx.lighting`.
#[derive(Clone, Debug, Default)]
pub struct LightingConfig {
    pub enabled: bool,
    /// Ambient light: rgb = colour (0–1 each), w = intensity multiplier.
    pub ambient: [f32; 4],
    pub lights:  Vec<Light>,
    pub shadow:  ShadowConfig,
}

// ── GPU uniform structs (byte layout must match light_apply.wgsl exactly) ─────

/// LightEntry — 48 bytes, stride 48, align 16.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct LightEntryGpu {
    position:    [f32; 2],   // offset  0   (8 bytes)
    radius:      f32,        // offset  8
    falloff:     f32,        // offset 12
    color:       [f32; 3],   // offset 16  (12 bytes, WGSL vec3 aligns to 16)
    intensity:   f32,        // offset 28
    cast_shadow: u32,        // offset 32
    shadow_row:  i32,        // offset 36  (-1 = none)
    _pad0:       u32,        // offset 40
    _pad1:       u32,        // offset 44
}                            // total 48 bytes

/// LightsUniform — 48-byte header + 64 × 48 = 3120 bytes total.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct LightsUniformGpu {
    ambient:           [f32; 4],                    // offset   0  (16 bytes)
    camera_pos:        [f32; 2],                    // offset  16
    viewport_half:     [f32; 2],                    // offset  24
    light_count:       u32,                         // offset  32
    shadow_resolution: u32,                         // offset  36
    _pad0:             u32,                         // offset  40
    _pad1:             u32,                         // offset  44
    lights:            [LightEntryGpu; MAX_LIGHTS], // offset  48
}                                                   // total 3120 bytes

/// ShadowBuildParams — 48 bytes (matches shadow_build.wgsl ShadowBuildParams).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ShadowBuildParamsGpu {
    light_pos:     [f32; 2],   // offset  0
    light_radius:  f32,        // offset  8
    resolution:    f32,        // offset 12
    camera_pos:    [f32; 2],   // offset 16
    viewport_half: [f32; 2],   // offset 24
    steps:         u32,        // offset 32
    start_frac:    f32,        // offset 36 — skip near-light zone (avoids glow sprite self-occlusion)
    _pad1:         u32,        // offset 40
    _pad2:         u32,        // offset 44
}                              // total 48 bytes

// ── LightingSystem ────────────────────────────────────────────────────────────

pub struct LightingSystem {
    /// Lit output texture: scene × lightmap result.
    pub output_texture: wgpu::Texture,
    pub output_view:    wgpu::TextureView,
    /// Bind group for sampling `output_texture` in PostProcessor pipelines.
    pub output_bg:      wgpu::BindGroup,

    // ── Light pass resources ──────────────────────────────────────────────
    light_pipeline:  wgpu::RenderPipeline,
    lights_buf:      wgpu::Buffer,
    lights_bg:       wgpu::BindGroup,   // group 1
    shadow_bg:       wgpu::BindGroup,   // group 2  (shadow texture array)

    // ── Shadow map resources ──────────────────────────────────────────────
    shadow_texture:      wgpu::Texture,
    /// Per-layer views for rendering into individual shadow map rows.
    shadow_layer_views:  Vec<wgpu::TextureView>,

    shadow_build_pipeline:  wgpu::RenderPipeline,
    shadow_build_params_bgl: wgpu::BindGroupLayout,
    /// One uniform buffer + bind group per shadow slot.
    shadow_build_bufs:  Vec<wgpu::Buffer>,
    shadow_build_bgs:   Vec<wgpu::BindGroup>,
    /// Bind group for the scene texture used in the shadow build pass.
    shadow_scene_bg:    wgpu::BindGroup,

    pub size:           (u32, u32),
    surface_format:     wgpu::TextureFormat,
    shadow_resolution:  u32,
}

impl LightingSystem {
    /// Create the lighting + shadow system.
    ///
    /// `pp_tex_bgl` / `pp_sampler` come from `PostProcessor` so the
    /// output bind-group is usable in PP's pipelines.
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        pp_tex_bgl: &wgpu::BindGroupLayout,
        pp_sampler: &wgpu::Sampler,
    ) -> Self {
        let (w, h) = (width.max(1), height.max(1));
        let shadow_resolution = 360u32;

        // ── Lit output texture ────────────────────────────────────────────
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("lighting_output"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let output_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting_output_bg"),
            layout: pp_tex_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(pp_sampler) },
            ],
        });

        // ── Shadow map texture array (R32Float, resolution × 1 × MAX_SHADOW_LIGHTS) ─
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_maps"),
            size: wgpu::Extent3d {
                width:                 shadow_resolution,
                height:                1,
                depth_or_array_layers: MAX_SHADOW_LIGHTS as u32,
            },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // Per-layer views for rendering into each shadow map row
        let shadow_layer_views: Vec<wgpu::TextureView> = (0..MAX_SHADOW_LIGHTS)
            .map(|i| shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                label:             Some("shadow_layer"),
                format:            Some(wgpu::TextureFormat::R32Float),
                dimension:         Some(wgpu::TextureViewDimension::D2),
                base_array_layer:  i as u32,
                array_layer_count: Some(1),
                ..Default::default()
            }))
            .collect();

        // Whole-array view for sampling in the light shader
        let shadow_array_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor {
            label:     Some("shadow_array"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        // ── Bind group layouts ────────────────────────────────────────────

        // Group 1: lights uniform
        let lights_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lights_bgl"),
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

        // Group 2: shadow texture array (non-filterable, accessed via textureLoad)
        let shadow_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type:    wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled:   false,
                },
                count: None,
            }],
        });

        // Shadow build: group 1 = per-light params
        let shadow_build_params_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow_build_params_bgl"),
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

        // ── Buffers & bind groups ─────────────────────────────────────────

        let lights_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lights_buf"),
            size: std::mem::size_of::<LightsUniformGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lights_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lights_bg"),
            layout: &lights_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: lights_buf.as_entire_binding() }],
        });

        let shadow_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_bg"),
            layout: &shadow_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&shadow_array_view),
            }],
        });

        // Per-slot shadow build buffers + bind groups
        let shadow_build_bufs: Vec<wgpu::Buffer> = (0..MAX_SHADOW_LIGHTS)
            .map(|_| device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shadow_build_buf"),
                size: std::mem::size_of::<ShadowBuildParamsGpu>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }))
            .collect();

        let shadow_build_bgs: Vec<wgpu::BindGroup> = shadow_build_bufs
            .iter()
            .map(|buf| device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow_build_bg"),
                layout: &shadow_build_params_bgl,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }],
            }))
            .collect();

        // Scene bind group for the shadow build pass (group 0, same layout as PP)
        let shadow_scene_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_scene_bg"),
            layout: pp_tex_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(pp_sampler) },
            ],
        });

        // ── Pipelines ─────────────────────────────────────────────────────

        // Light apply pipeline  (groups 0 = pp_tex_bgl, 1 = lights_bgl, 2 = shadow_bgl)
        let light_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("light_apply_layout"),
            bind_group_layouts: &[pp_tex_bgl, &lights_bgl, &shadow_bgl],
            push_constant_ranges: &[],
        });
        let light_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("light_apply"),
            source: wgpu::ShaderSource::Wgsl(include_str!("light_apply.wgsl").into()),
        });
        let light_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("light_pipeline"),
            layout: Some(&light_layout),
            vertex: wgpu::VertexState {
                module: &light_shader, entry_point: Some("vs_main"),
                buffers: &[], compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &light_shader, entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format, blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None, cache: None,
        });

        // Shadow build pipeline  (groups 0 = pp_tex_bgl, 1 = shadow_build_params_bgl)
        // Output format: R32Float (shadow map)
        let shadow_build_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_build_layout"),
            bind_group_layouts: &[pp_tex_bgl, &shadow_build_params_bgl],
            push_constant_ranges: &[],
        });
        let shadow_build_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_build"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow_build.wgsl").into()),
        });
        let shadow_build_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_build_pipeline"),
            layout: Some(&shadow_build_layout),
            vertex: wgpu::VertexState {
                module: &shadow_build_shader, entry_point: Some("vs_main"),
                buffers: &[], compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shadow_build_shader, entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Float, blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None, cache: None,
        });

        Self {
            output_texture, output_view, output_bg,
            light_pipeline, lights_buf, lights_bg, shadow_bg,
            shadow_texture, shadow_layer_views,
            shadow_build_pipeline, shadow_build_params_bgl,
            shadow_build_bufs, shadow_build_bgs, shadow_scene_bg,
            size: (w, h),
            surface_format,
            shadow_resolution,
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

    /// Apply lighting (and optionally shadows).
    ///
    /// `scene_bg` must be the PostProcessor's `scene_bg` — reads the scene texture
    /// for both shadow ray-marching and the final light composite.
    pub fn apply(
        &self,
        config: &LightingConfig,
        camera: &Camera2D,
        scene_bg: &wgpu::BindGroup,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let half = camera.half_viewport();
        let cam_pos = [camera.position.x, camera.position.y];
        let viewport_half = [half.x, half.y];
        let shadow_res = self.shadow_resolution;
        let shadows_on = config.shadow.enabled;

        // ── 1. Assign shadow rows to shadow-casting lights ────────────────
        let mut shadow_assignments: Vec<i32> = vec![-1; config.lights.len()];
        let mut next_row: usize = 0;

        if shadows_on {
            for (i, light) in config.lights.iter().enumerate() {
                if light.cast_shadow && next_row < MAX_SHADOW_LIGHTS {
                    shadow_assignments[i] = next_row as i32;
                    next_row += 1;
                }
            }
        }

        // ── 2. Build shadow maps ──────────────────────────────────────────
        for (i, light) in config.lights.iter().enumerate() {
            let row = shadow_assignments[i];
            if row < 0 { continue; }

            // Skip the first ~8% of the radius to avoid self-occlusion by the
            // glow sprite drawn at the light position. 8% of radius=140 → 11 wu,
            // well beyond any glow dot (max radius ≈ 6 wu for size=12 sprite).
            let start_frac = (12.0_f32 / light.radius).min(0.15);
            let params = ShadowBuildParamsGpu {
                light_pos:     [light.position.x, light.position.y],
                light_radius:  light.radius,
                resolution:    shadow_res as f32,
                camera_pos:    cam_pos,
                viewport_half,
                steps:         SHADOW_STEPS,
                start_frac,
                _pad1: 0, _pad2: 0,
            };
            queue.write_buffer(&self.shadow_build_bufs[row as usize], 0, bytemuck::bytes_of(&params));

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow_build_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view:           &self.shadow_layer_views[row as usize],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            // Restrict rasterization to the 1D strip we actually need
            pass.set_viewport(0.0, 0.0, shadow_res as f32, 1.0, 0.0, 1.0);
            pass.set_pipeline(&self.shadow_build_pipeline);
            pass.set_bind_group(0, scene_bg, &[]);
            pass.set_bind_group(1, &self.shadow_build_bgs[row as usize], &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 3. Build lights uniform ───────────────────────────────────────
        let mut gpu_lights = [LightEntryGpu {
            position: [0.0; 2], radius: 0.0, falloff: 0.0,
            color: [0.0; 3], intensity: 0.0,
            cast_shadow: 0, shadow_row: -1, _pad0: 0, _pad1: 0,
        }; MAX_LIGHTS];

        let count = config.lights.len().min(MAX_LIGHTS);
        for (i, l) in config.lights.iter().take(MAX_LIGHTS).enumerate() {
            gpu_lights[i] = LightEntryGpu {
                position:    [l.position.x, l.position.y],
                radius:      l.radius,
                falloff:     l.falloff,
                color:       l.color,
                intensity:   l.intensity,
                cast_shadow: u32::from(l.cast_shadow && shadows_on),
                shadow_row:  shadow_assignments[i],
                _pad0: 0, _pad1: 0,
            };
        }

        let uniform = LightsUniformGpu {
            ambient:           config.ambient,
            camera_pos:        cam_pos,
            viewport_half,
            light_count:       count as u32,
            shadow_resolution: shadow_res,
            _pad0: 0, _pad1: 0,
            lights:            gpu_lights,
        };
        queue.write_buffer(&self.lights_buf, 0, bytemuck::bytes_of(&uniform));

        // ── 4. Light apply pass ───────────────────────────────────────────
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("light_apply_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view:           &self.output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load:  wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_pipeline(&self.light_pipeline);
        pass.set_bind_group(0, scene_bg, &[]);
        pass.set_bind_group(1, &self.lights_bg, &[]);
        pass.set_bind_group(2, &self.shadow_bg, &[]);
        pass.draw(0..3, 0..1);
    }
}
