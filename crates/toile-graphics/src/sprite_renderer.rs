use std::collections::HashMap;

use glam::Vec2;

use crate::camera::{Camera2D, CameraUniform};
use crate::texture::{self, TextureEntry, TextureHandle};
use toile_core::color::Color;

/// Per-instance sprite data (audit P4: instanced rendering). One record per
/// sprite; the vertex shader expands a shared unit quad, scales it by `size`,
/// rotates by `rotation`, translates to `position`, and picks the UV rect.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub rotation: f32,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub color: u32,
}

impl SpriteInstance {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        // NOTE: order must match the struct field order so offsets line up.
        attributes: &wgpu::vertex_attr_array![
            2 => Float32x2, // position
            3 => Float32x2, // size
            4 => Float32,   // rotation
            5 => Float32x2, // uv_min
            6 => Float32x2, // uv_max
            7 => Uint32,    // color
        ],
    };
}

/// Static unit-quad vertex shared by every sprite instance.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadVertex {
    corner: [f32; 2], // ±0.5 around the centre
    uv_sel: [f32; 2], // 0 or 1 per axis (selects between uv_min and uv_max)
}

impl QuadVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x2, // corner
            1 => Float32x2, // uv_sel
        ],
    };
}

// Unit quad (y-up) + matching UV selectors, in the same corner order the old
// per-sprite CPU path used (TL, TR, BR, BL), drawn via UNIT_QUAD_INDICES.
const UNIT_QUAD: [QuadVertex; 4] = [
    QuadVertex { corner: [-0.5,  0.5], uv_sel: [0.0, 0.0] },
    QuadVertex { corner: [ 0.5,  0.5], uv_sel: [1.0, 0.0] },
    QuadVertex { corner: [ 0.5, -0.5], uv_sel: [1.0, 1.0] },
    QuadVertex { corner: [-0.5, -0.5], uv_sel: [0.0, 1.0] },
];
const UNIT_QUAD_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

#[derive(Clone)]
pub struct DrawSprite {
    pub texture: TextureHandle,
    pub position: Vec2,
    pub size: Vec2,
    pub rotation: f32,
    pub color: u32,
    pub layer: i32,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
}

impl DrawSprite {
    pub fn new(texture: TextureHandle, position: Vec2, size: Vec2) -> Self {
        Self {
            texture,
            position,
            size,
            rotation: 0.0,
            color: COLOR_WHITE,
            layer: 0,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
        }
    }
}

pub fn pack_color(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

pub const COLOR_WHITE: u32 = 0xFFFF_FFFF;

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStats {
    pub sprite_count: u32,
    pub draw_calls: u32,
    pub batch_count: u32,
}

pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture_bind_groups: HashMap<TextureHandle, wgpu::BindGroup>,
    textures: Vec<TextureEntry>,
    next_texture_id: u32,
    // Static geometry shared by all sprites.
    quad_vbo: wgpu::Buffer,
    quad_ibo: wgpu::Buffer,
    // Per-frame instance data.
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    sort_order: Vec<usize>,
    // Persistent CPU scratch reused each frame to avoid per-frame allocation.
    instances: Vec<SpriteInstance>,
}

impl SpriteRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });

        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bgl"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[&camera_bgl, &texture_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[QuadVertex::LAYOUT, SpriteInstance::LAYOUT],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bg"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Static unit-quad vertex + index buffers, initialised at creation (no
        // queue available here, so map-at-creation and copy).
        let quad_vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_quad_vbo"),
            size: std::mem::size_of_val(&UNIT_QUAD) as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });
        quad_vbo
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(bytemuck::cast_slice(&UNIT_QUAD));
        quad_vbo.unmap();

        let quad_ibo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_quad_ibo"),
            size: std::mem::size_of_val(&UNIT_QUAD_INDICES) as u64,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: true,
        });
        quad_ibo
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(bytemuck::cast_slice(&UNIT_QUAD_INDICES));
        quad_ibo.unmap();

        let instance_capacity = 256;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_instances"),
            size: (instance_capacity * std::mem::size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            texture_bind_group_layout: texture_bgl,
            sampler,
            texture_bind_groups: HashMap::new(),
            textures: Vec::new(),
            next_texture_id: 0,
            quad_vbo,
            quad_ibo,
            instance_buffer,
            instance_capacity,
            sort_order: Vec::new(),
            instances: Vec::new(),
        }
    }

    pub fn load_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &std::path::Path,
    ) -> TextureHandle {
        let entry = texture::load_png(device, queue, path);
        let handle = TextureHandle(self.next_texture_id);
        self.next_texture_id += 1;

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bg"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&entry.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.texture_bind_groups.insert(handle, bind_group);
        self.textures.push(entry);
        handle
    }

    pub fn create_texture_from_rgba(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> TextureHandle {
        let entry = texture::create_texture_from_rgba(device, queue, data, width, height);
        let handle = TextureHandle(self.next_texture_id);
        self.next_texture_id += 1;

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bg"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&entry.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.texture_bind_groups.insert(handle, bind_group);
        self.textures.push(entry);
        handle
    }

    /// Render all sprites with sort-and-batch via instancing. Returns stats.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        camera: &Camera2D,
        sprites: &[DrawSprite],
        clear_color: &Color,
    ) -> RenderStats {
        let mut stats = RenderStats {
            sprite_count: sprites.len() as u32,
            ..Default::default()
        };

        // Update camera
        let cam_uniform = CameraUniform::from_camera(camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&cam_uniform));

        if sprites.is_empty() {
            // Still need to clear
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear((*clear_color).into()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            drop(_clear_pass);
            return stats;
        }

        // Sort by (layer, texture) for batching. Stable sort preserves submission order.
        self.sort_order.clear();
        self.sort_order.extend(0..sprites.len());
        self.sort_order.sort_by(|&a, &b| {
            let sa = &sprites[a];
            let sb = &sprites[b];
            sa.layer
                .cmp(&sb.layer)
                .then(sa.texture.0.cmp(&sb.texture.0))
        });

        // Build one instance per sprite into the reused scratch buffer.
        self.instances.clear();
        for &sprite_idx in &self.sort_order {
            let s = &sprites[sprite_idx];
            self.instances.push(SpriteInstance {
                position: [s.position.x, s.position.y],
                size: [s.size.x, s.size.y],
                rotation: s.rotation,
                uv_min: [s.uv_min.x, s.uv_min.y],
                uv_max: [s.uv_max.x, s.uv_max.y],
                color: s.color,
            });
        }

        // Grow the instance buffer if needed, then upload this frame's instances.
        let count = self.instances.len();
        if count > self.instance_capacity {
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_instances"),
                size: (count * std::mem::size_of::<SpriteInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_capacity = count;
        }
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&self.instances));

        // Render pass: one shared unit quad, one draw call per texture batch,
        // each drawing a contiguous range of instances.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear((*clear_color).into()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.quad_vbo.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.set_index_buffer(self.quad_ibo.slice(..), wgpu::IndexFormat::Uint32);

            // Batched draw: group consecutive sprites with the same texture and
            // draw them as one instanced call.
            let mut batch_start: u32 = 0;
            let mut batch_texture = sprites[self.sort_order[0]].texture;

            for i in 1..=self.sort_order.len() {
                let at_end = i == self.sort_order.len();
                let texture_changed =
                    !at_end && sprites[self.sort_order[i]].texture != batch_texture;

                if at_end || texture_changed {
                    let batch_end = i as u32;
                    if let Some(bg) = self.texture_bind_groups.get(&batch_texture) {
                        pass.set_bind_group(1, bg, &[]);
                        pass.draw_indexed(0..6, 0, batch_start..batch_end);
                        stats.draw_calls += 1;
                    }
                    stats.batch_count += 1;

                    if !at_end {
                        batch_start = batch_end;
                        batch_texture = sprites[self.sort_order[i]].texture;
                    }
                }
            }
        }

        stats
    }
}
