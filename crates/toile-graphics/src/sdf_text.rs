use std::collections::HashMap;

use glam::Vec2;

use crate::camera::{Camera2D, CameraUniform};
use crate::texture::TextureHandle;

// ─── Vertex ──────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SdfVertex {
    pub position:      [f32; 2],  // offset  0
    pub uv:            [f32; 2],  // offset  8
    pub fill_color:    u32,       // offset 16  — packed RGBA8 (same as pack_color)
    pub outline_color: u32,       // offset 20
    pub outline_width: f32,       // offset 24  — 0.0..0.5 in SDF fraction space
}  // total 28 bytes, no padding

impl SdfVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x2,   // position
            1 => Float32x2,   // uv
            2 => Uint32,      // fill_color
            3 => Uint32,      // outline_color
            4 => Float32,     // outline_width
        ],
    };
}

// ─── Draw command ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DrawSdfGlyph {
    /// Index into `SdfTextRenderer::textures`.
    pub texture_idx:   usize,
    pub position:      Vec2,   // world-space centre
    pub size:          Vec2,
    pub layer:         i32,
    pub uv_min:        Vec2,
    pub uv_max:        Vec2,
    pub fill_color:    u32,
    pub outline_color: u32,
    pub outline_width: f32,
}

// ─── Texture store ────────────────────────────────────────────────────────────

struct SdfTexEntry {
    _texture:   wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

// ─── WGSL shader ─────────────────────────────────────────────────────────────

const SDF_SHADER: &str = r#"
struct Camera { proj: mat4x4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var t_sdf: texture_2d<f32>;
@group(1) @binding(1) var s_sdf: sampler;

struct VertIn {
    @location(0) position:      vec2<f32>,
    @location(1) uv:            vec2<f32>,
    @location(2) fill_color:    u32,
    @location(3) outline_color: u32,
    @location(4) outline_width: f32,
}

struct VertOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:            vec2<f32>,
    @location(1) fill_color:    vec4<f32>,
    @location(2) outline_color: vec4<f32>,
    @location(3) outline_width: f32,
}

@vertex
fn vs_main(in: VertIn) -> VertOut {
    var out: VertOut;
    out.clip_pos      = camera.proj * vec4(in.position, 0.0, 1.0);
    out.uv            = in.uv;
    out.fill_color    = unpack4x8unorm(in.fill_color);
    out.outline_color = unpack4x8unorm(in.outline_color);
    out.outline_width = in.outline_width;
    return out;
}

@fragment
fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    // dist ≈ 0.0 deep outside | 0.5 edge | 1.0 deep inside
    let dist = textureSample(t_sdf, s_sdf, in.uv).r;

    // Antialiasing half-width based on screen-space derivative
    let w = fwidth(dist) * 0.7;

    // Fill coverage
    let fill_cov = smoothstep(0.5 - w, 0.5 + w, dist);
    let fill_a   = fill_cov * in.fill_color.a;

    // Outline ring: dist in [0.5 - outline_width, 0.5]
    let ol_inner = 0.5 - in.outline_width;
    let ol_cov   = smoothstep(ol_inner - w, ol_inner + w, dist);
    let ol_only  = max(ol_cov - fill_cov, 0.0);
    let ol_a     = ol_only * in.outline_color.a;

    let alpha = fill_a + ol_a;
    if alpha < 0.004 { discard; }

    let rgb = (in.fill_color.rgb * fill_a + in.outline_color.rgb * ol_a) / alpha;
    return vec4(rgb, alpha);
}
"#;

// ─── Renderer ────────────────────────────────────────────────────────────────

pub struct SdfTextRenderer {
    pipeline:               wgpu::RenderPipeline,
    camera_buffer:          wgpu::Buffer,
    camera_bind_group:      wgpu::BindGroup,
    texture_bgl:            wgpu::BindGroupLayout,
    sampler:                wgpu::Sampler,
    textures:               Vec<SdfTexEntry>,
    // Bind-group cache keyed by TextureHandle — not used here since we have
    // direct index access, but left for future atlas expansion.
    _tex_bg_cache:          HashMap<usize, ()>,
    vertex_buffer:          wgpu::Buffer,
    index_buffer:           wgpu::Buffer,
    vertex_capacity:        usize,
    index_capacity:         usize,
    sort_order:             Vec<usize>,
}

impl SdfTextRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("sdf_shader"),
            source: wgpu::ShaderSource::Wgsl(SDF_SHADER.into()),
        });

        // Group 0: camera
        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("sdf_camera_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding:    0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty:                 wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size:   None,
                },
                count: None,
            }],
        });

        // Group 1: SDF texture + sampler (linear filtering for smooth SDF)
        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("sdf_tex_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding:    0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type:    wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled:   false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding:    1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:                Some("sdf_pipeline_layout"),
            bind_group_layouts:   &[&camera_bgl, &texture_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:  Some("sdf_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module:              &shader,
                entry_point:         Some("vs_main"),
                compilation_options: Default::default(),
                buffers:             &[SdfVertex::LAYOUT],
            },
            primitive: wgpu::PrimitiveState {
                topology:  wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample:   wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module:              &shader,
                entry_point:         Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format:     surface_format,
                    blend:      Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache:     None,
        });

        // Camera uniform buffer
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("sdf_camera_uniform"),
            size:               std::mem::size_of::<CameraUniform>() as u64,
            usage:              wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("sdf_camera_bg"),
            layout:  &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding:  0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Linear sampler for smooth SDF interpolation
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:      Some("sdf_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let cap = 128;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("sdf_vbo"),
            size:               (cap * 4 * std::mem::size_of::<SdfVertex>()) as u64,
            usage:              wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("sdf_ibo"),
            size:               (cap * 6 * std::mem::size_of::<u32>()) as u64,
            usage:              wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            texture_bgl,
            sampler,
            textures: Vec::new(),
            _tex_bg_cache: HashMap::new(),
            vertex_buffer,
            index_buffer,
            vertex_capacity: cap * 4,
            index_capacity:  cap * 6,
            sort_order: Vec::new(),
        }
    }

    /// Upload an R8 SDF atlas to the GPU and return its texture index.
    pub fn create_sdf_texture(
        &mut self,
        device: &wgpu::Device,
        queue:  &wgpu::Queue,
        data:   &[u8],
        width:  u32,
        height: u32,
    ) -> usize {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label:           Some("sdf_atlas"),
            size,
            mip_level_count: 1,
            sample_count:    1,
            dimension:       wgpu::TextureDimension::D2,
            // R8Unorm: linear (no sRGB gamma) — essential for correct SDF thresholding
            format:          wgpu::TextureFormat::R8Unorm,
            usage:           wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats:    &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture:   &texture,
                mip_level: 0,
                origin:    wgpu::Origin3d::ZERO,
                aspect:    wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset:         0,
                bytes_per_row:  Some(width),   // 1 byte per texel for R8
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("sdf_tex_bg"),
            layout:  &self.texture_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding:  0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding:  1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let idx = self.textures.len();
        self.textures.push(SdfTexEntry { _texture: texture, bind_group });
        idx
    }

    fn build_quad(g: &DrawSdfGlyph, base: u32) -> ([SdfVertex; 4], [u32; 6]) {
        let half = g.size * 0.5;
        let corners = [
            Vec2::new(-half.x,  half.y),
            Vec2::new( half.x,  half.y),
            Vec2::new( half.x, -half.y),
            Vec2::new(-half.x, -half.y),
        ];
        let uvs = [
            [g.uv_min.x, g.uv_min.y],
            [g.uv_max.x, g.uv_min.y],
            [g.uv_max.x, g.uv_max.y],
            [g.uv_min.x, g.uv_max.y],
        ];
        let mut verts = [SdfVertex {
            position: [0.0; 2], uv: [0.0; 2],
            fill_color: 0, outline_color: 0, outline_width: 0.0,
        }; 4];
        for i in 0..4 {
            verts[i] = SdfVertex {
                position:      [g.position.x + corners[i].x, g.position.y + corners[i].y],
                uv:            uvs[i],
                fill_color:    g.fill_color,
                outline_color: g.outline_color,
                outline_width: g.outline_width,
            };
        }
        let b = base;
        (verts, [b, b+1, b+2, b, b+2, b+3])
    }

    /// Render SDF glyphs into `view`.  Must be called AFTER the sprite renderer
    /// has already cleared the target (uses `LoadOp::Load`).
    pub fn draw(
        &mut self,
        device:  &wgpu::Device,
        queue:   &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view:    &wgpu::TextureView,
        camera:  &Camera2D,
        glyphs:  &[DrawSdfGlyph],
    ) {
        if glyphs.is_empty() {
            return;
        }

        // Update camera
        let cam_uniform = CameraUniform::from_camera(camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&cam_uniform));

        // Sort by (layer, texture_idx) — stable so shadows (inserted first) stay before fill
        self.sort_order.clear();
        self.sort_order.extend(0..glyphs.len());
        self.sort_order.sort_by(|&a, &b| {
            glyphs[a].layer.cmp(&glyphs[b].layer)
                .then(glyphs[a].texture_idx.cmp(&glyphs[b].texture_idx))
        });

        // Build geometry
        let n = glyphs.len();
        let mut vertices: Vec<SdfVertex> = Vec::with_capacity(n * 4);
        let mut indices:  Vec<u32>       = Vec::with_capacity(n * 6);

        for (qi, &gi) in self.sort_order.iter().enumerate() {
            let (v, idx) = Self::build_quad(&glyphs[gi], (qi * 4) as u32);
            vertices.extend_from_slice(&v);
            indices.extend_from_slice(&idx);
        }

        // Grow buffers if needed
        if n * 4 > self.vertex_capacity {
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label:              Some("sdf_vbo"),
                size:               (n * 4 * std::mem::size_of::<SdfVertex>()) as u64,
                usage:              wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.vertex_capacity = n * 4;
        }
        if n * 6 > self.index_capacity {
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label:              Some("sdf_ibo"),
                size:               (n * 6 * std::mem::size_of::<u32>()) as u64,
                usage:              wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.index_capacity = n * 6;
        }

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        queue.write_buffer(&self.index_buffer,  0, bytemuck::cast_slice(&indices));

        // Render pass — LoadOp::Load to composite on top of sprites
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sdf_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load:  wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Batched draw by texture
        let mut batch_start: u32 = 0;
        let mut batch_tex = glyphs[self.sort_order[0]].texture_idx;

        for i in 1..=self.sort_order.len() {
            let at_end = i == self.sort_order.len();
            let tex_changed = !at_end && glyphs[self.sort_order[i]].texture_idx != batch_tex;

            if at_end || tex_changed {
                let batch_end = i as u32;
                if let Some(entry) = self.textures.get(batch_tex) {
                    pass.set_bind_group(1, &entry.bind_group, &[]);
                    pass.draw_indexed(batch_start * 6..batch_end * 6, 0, 0..1);
                }
                if !at_end {
                    batch_start = batch_end;
                    batch_tex   = glyphs[self.sort_order[i]].texture_idx;
                }
            }
        }
    }
}
