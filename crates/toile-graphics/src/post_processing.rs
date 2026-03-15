// Toile Engine — Post-processing pipeline
// Offscreen render + configurable full-screen effect chain (ping-pong).

use bytemuck::{Pod, Zeroable};

// ── Public API types ─────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum PostEffect {
    /// Dark edges, clear centre.
    Vignette { intensity: f32, smoothness: f32 },
    /// CRT monitor: scanlines, barrel distortion, chromatic aberration.
    Crt { scanline_intensity: f32, curvature: f32, chromatic_aberration: f32 },
    /// Reduce resolution to large square pixels.
    Pixelate { pixel_size: f32 },
    /// Glow around bright areas (single-pass 5×5 kernel).
    Bloom { threshold: f32, intensity: f32, radius: f32 },
    /// Translate the image (UV offset) for trauma-based camera shake.
    ScreenShake { offset_x: f32, offset_y: f32 },
    /// Brightness, contrast, saturation adjustment.
    ColorGrading { saturation: f32, brightness: f32, contrast: f32 },
}

#[derive(Clone, Debug, Default)]
pub struct PostProcessingStack {
    pub effects: Vec<PostEffect>,
    pub enabled: bool,
}

// ── Internal uniform structs (16-byte aligned) ───────────────────────────────

#[repr(C)] #[derive(Copy, Clone, Pod, Zeroable)]
struct VignetteU { intensity: f32, smoothness: f32, _p: [f32; 2] }

#[repr(C)] #[derive(Copy, Clone, Pod, Zeroable)]
struct CrtU { scanline_intensity: f32, curvature: f32, chromatic_aberration: f32, screen_height: f32 }

#[repr(C)] #[derive(Copy, Clone, Pod, Zeroable)]
struct PixelateU { pixel_size: f32, screen_w: f32, screen_h: f32, _p: f32 }

#[repr(C)] #[derive(Copy, Clone, Pod, Zeroable)]
struct ShakeU { offset_x: f32, offset_y: f32, _p: [f32; 2] }

#[repr(C)] #[derive(Copy, Clone, Pod, Zeroable)]
struct BloomU { threshold: f32, intensity: f32, radius: f32, _p: f32 }

#[repr(C)] #[derive(Copy, Clone, Pod, Zeroable)]
struct GradingU { saturation: f32, brightness: f32, contrast: f32, _p: f32 }

// ── PostProcessor ────────────────────────────────────────────────────────────

pub struct PostProcessor {
    // Scene texture: sprites render here instead of the swapchain.
    pub scene_texture: wgpu::Texture,
    pub scene_view: wgpu::TextureView,

    // Ping-pong pair for the effect chain.
    ping_texture: wgpu::Texture,
    ping_view: wgpu::TextureView,
    pong_texture: wgpu::Texture,
    pong_view: wgpu::TextureView,

    // Shared texture bind-group layout (group 0).
    tex_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,

    // Bind groups for reading each offscreen texture.
    scene_bg: wgpu::BindGroup,
    ping_bg: wgpu::BindGroup,
    pong_bg: wgpu::BindGroup,

    // Shared params bind-group layout (group 1, uniform buffer).
    params_bgl: wgpu::BindGroupLayout,

    // Per-effect: pipeline, uniform buffer, params bind group.
    passthrough_pipeline: wgpu::RenderPipeline,

    vignette_pipeline: wgpu::RenderPipeline,
    vignette_buf: wgpu::Buffer,
    vignette_bg: wgpu::BindGroup,

    crt_pipeline: wgpu::RenderPipeline,
    crt_buf: wgpu::Buffer,
    crt_bg: wgpu::BindGroup,

    pixelate_pipeline: wgpu::RenderPipeline,
    pixelate_buf: wgpu::Buffer,
    pixelate_bg: wgpu::BindGroup,

    bloom_pipeline: wgpu::RenderPipeline,
    bloom_buf: wgpu::Buffer,
    bloom_bg: wgpu::BindGroup,

    shake_pipeline: wgpu::RenderPipeline,
    shake_buf: wgpu::Buffer,
    shake_bg: wgpu::BindGroup,

    grading_pipeline: wgpu::RenderPipeline,
    grading_buf: wgpu::Buffer,
    grading_bg: wgpu::BindGroup,

    pub size: (u32, u32),
    surface_format: wgpu::TextureFormat,
}

impl PostProcessor {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let (w, h) = (width.max(1), height.max(1));

        // ── Offscreen textures ────────────────────────────────────────────
        let mk_tex = |label: &str| {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: surface_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            (tex, view)
        };
        let (scene_texture, scene_view) = mk_tex("pp_scene");
        let (ping_texture, ping_view)   = mk_tex("pp_ping");
        let (pong_texture, pong_view)   = mk_tex("pp_pong");

        // ── Sampler ───────────────────────────────────────────────────────
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pp_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        // ── Bind group layouts ────────────────────────────────────────────
        let tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pp_tex_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let params_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pp_params_bgl"),
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

        // ── Texture bind groups ───────────────────────────────────────────
        let mk_tex_bg = |view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("pp_tex_bg"),
                layout: &tex_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            })
        };
        let scene_bg = mk_tex_bg(&scene_view);
        let ping_bg  = mk_tex_bg(&ping_view);
        let pong_bg  = mk_tex_bg(&pong_view);

        // ── Pipeline helpers ──────────────────────────────────────────────
        let passthrough_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pp_passthrough_layout"),
            bind_group_layouts: &[&tex_bgl],
            push_constant_ranges: &[],
        });
        let effect_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pp_effect_layout"),
            bind_group_layouts: &[&tex_bgl, &params_bgl],
            push_constant_ranges: &[],
        });

        let mk_pipeline = |src: &str, layout: &wgpu::PipelineLayout| {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("pp_shader"),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pp_pipeline"),
                layout: Some(layout),
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
            })
        };

        let passthrough_pipeline = mk_pipeline(
            include_str!("post_passthrough.wgsl"), &passthrough_layout);
        let vignette_pipeline = mk_pipeline(include_str!("post_vignette.wgsl"), &effect_layout);
        let crt_pipeline      = mk_pipeline(include_str!("post_crt.wgsl"),      &effect_layout);
        let pixelate_pipeline = mk_pipeline(include_str!("post_pixelate.wgsl"), &effect_layout);
        let bloom_pipeline    = mk_pipeline(include_str!("post_bloom.wgsl"),    &effect_layout);
        let shake_pipeline    = mk_pipeline(include_str!("post_screenshake.wgsl"), &effect_layout);
        let grading_pipeline  = mk_pipeline(include_str!("post_colorgrading.wgsl"), &effect_layout);

        // ── Uniform buffers ───────────────────────────────────────────────
        let mk_ubuf = |size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("pp_ubuf"),
                size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let vignette_buf = mk_ubuf(std::mem::size_of::<VignetteU>() as u64);
        let crt_buf      = mk_ubuf(std::mem::size_of::<CrtU>()      as u64);
        let pixelate_buf = mk_ubuf(std::mem::size_of::<PixelateU>() as u64);
        let bloom_buf    = mk_ubuf(std::mem::size_of::<BloomU>()    as u64);
        let shake_buf    = mk_ubuf(std::mem::size_of::<ShakeU>()    as u64);
        let grading_buf  = mk_ubuf(std::mem::size_of::<GradingU>()  as u64);

        let mk_params_bg = |buf: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("pp_params_bg"),
                layout: &params_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                }],
            })
        };
        let vignette_bg = mk_params_bg(&vignette_buf);
        let crt_bg      = mk_params_bg(&crt_buf);
        let pixelate_bg = mk_params_bg(&pixelate_buf);
        let bloom_bg    = mk_params_bg(&bloom_buf);
        let shake_bg    = mk_params_bg(&shake_buf);
        let grading_bg  = mk_params_bg(&grading_buf);

        Self {
            scene_texture, scene_view,
            ping_texture, ping_view,
            pong_texture, pong_view,
            tex_bgl, sampler,
            scene_bg, ping_bg, pong_bg,
            params_bgl,
            passthrough_pipeline,
            vignette_pipeline, vignette_buf, vignette_bg,
            crt_pipeline, crt_buf, crt_bg,
            pixelate_pipeline, pixelate_buf, pixelate_bg,
            bloom_pipeline, bloom_buf, bloom_bg,
            shake_pipeline, shake_buf, shake_bg,
            grading_pipeline, grading_buf, grading_bg,
            size: (w, h),
            surface_format,
        }
    }

    /// Recreate with a new size (call on window resize).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        *self = Self::new(device, self.surface_format, width, height);
    }

    /// Apply the post-processing stack. Reads from `scene_texture`, writes final result to
    /// `final_view` (the swapchain). If the stack is empty, does a plain blit.
    pub fn apply(
        &self,
        stack: &PostProcessingStack,
        final_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if stack.effects.is_empty() {
            run_pass(encoder, &self.passthrough_pipeline, &self.scene_bg, None, final_view);
            return;
        }

        let (w, h) = self.size;
        let n = stack.effects.len();

        for (i, effect) in stack.effects.iter().enumerate() {
            let is_last = i == n - 1;

            // Source: pass 0 → scene, odd passes → ping, even (>0) → pong
            let src_bg: &wgpu::BindGroup = match i {
                0 => &self.scene_bg,
                x if x % 2 == 1 => &self.ping_bg,
                _ => &self.pong_bg,
            };

            // Destination: last pass → final_view, even → ping, odd → pong
            let dst_view: &wgpu::TextureView = if is_last {
                final_view
            } else if i % 2 == 0 {
                &self.ping_view
            } else {
                &self.pong_view
            };

            match effect {
                PostEffect::Vignette { intensity, smoothness } => {
                    queue.write_buffer(&self.vignette_buf, 0, bytemuck::bytes_of(
                        &VignetteU { intensity: *intensity, smoothness: *smoothness, _p: [0.0; 2] }
                    ));
                    run_pass(encoder, &self.vignette_pipeline, src_bg, Some(&self.vignette_bg), dst_view);
                }
                PostEffect::Crt { scanline_intensity, curvature, chromatic_aberration } => {
                    queue.write_buffer(&self.crt_buf, 0, bytemuck::bytes_of(
                        &CrtU {
                            scanline_intensity: *scanline_intensity,
                            curvature: *curvature,
                            chromatic_aberration: *chromatic_aberration,
                            screen_height: h as f32,
                        }
                    ));
                    run_pass(encoder, &self.crt_pipeline, src_bg, Some(&self.crt_bg), dst_view);
                }
                PostEffect::Pixelate { pixel_size } => {
                    queue.write_buffer(&self.pixelate_buf, 0, bytemuck::bytes_of(
                        &PixelateU { pixel_size: *pixel_size, screen_w: w as f32, screen_h: h as f32, _p: 0.0 }
                    ));
                    run_pass(encoder, &self.pixelate_pipeline, src_bg, Some(&self.pixelate_bg), dst_view);
                }
                PostEffect::Bloom { threshold, intensity, radius } => {
                    queue.write_buffer(&self.bloom_buf, 0, bytemuck::bytes_of(
                        &BloomU { threshold: *threshold, intensity: *intensity, radius: *radius, _p: 0.0 }
                    ));
                    run_pass(encoder, &self.bloom_pipeline, src_bg, Some(&self.bloom_bg), dst_view);
                }
                PostEffect::ScreenShake { offset_x, offset_y } => {
                    queue.write_buffer(&self.shake_buf, 0, bytemuck::bytes_of(
                        &ShakeU { offset_x: *offset_x, offset_y: *offset_y, _p: [0.0; 2] }
                    ));
                    run_pass(encoder, &self.shake_pipeline, src_bg, Some(&self.shake_bg), dst_view);
                }
                PostEffect::ColorGrading { saturation, brightness, contrast } => {
                    queue.write_buffer(&self.grading_buf, 0, bytemuck::bytes_of(
                        &GradingU { saturation: *saturation, brightness: *brightness, contrast: *contrast, _p: 0.0 }
                    ));
                    run_pass(encoder, &self.grading_pipeline, src_bg, Some(&self.grading_bg), dst_view);
                }
            }
        }
    }
}

// ── Full-screen triangle helper ───────────────────────────────────────────────

fn run_pass(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    tex_bg: &wgpu::BindGroup,
    params_bg: Option<&wgpu::BindGroup>,
    dst_view: &wgpu::TextureView,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("pp_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: dst_view,
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
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, tex_bg, &[]);
    if let Some(pbg) = params_bg {
        pass.set_bind_group(1, pbg, &[]);
    }
    pass.draw(0..3, 0..1);
}
