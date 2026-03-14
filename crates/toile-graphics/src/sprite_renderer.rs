use std::collections::HashMap;

use glam::Vec2;

use crate::camera::{Camera2D, CameraUniform};
use crate::texture::{self, TextureEntry, TextureHandle};
use toile_core::color::Color;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: u32,
}

impl SpriteVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Uint32,
        ],
    };
}

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
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
    sort_order: Vec<usize>,
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
                buffers: &[SpriteVertex::LAYOUT],
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

        let cap = 256;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_vbo"),
            size: (cap * 4 * std::mem::size_of::<SpriteVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_ibo"),
            size: (cap * 6 * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
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
            vertex_buffer,
            index_buffer,
            vertex_capacity: cap * 4,
            index_capacity: cap * 6,
            sort_order: Vec::new(),
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

    fn build_quad(sprite: &DrawSprite, base_vertex: u32) -> ([SpriteVertex; 4], [u32; 6]) {
        let half = sprite.size * 0.5;
        let (sin, cos) = sprite.rotation.sin_cos();

        let corners = [
            Vec2::new(-half.x, half.y),
            Vec2::new(half.x, half.y),
            Vec2::new(half.x, -half.y),
            Vec2::new(-half.x, -half.y),
        ];
        let uvs = [
            [sprite.uv_min.x, sprite.uv_min.y],
            [sprite.uv_max.x, sprite.uv_min.y],
            [sprite.uv_max.x, sprite.uv_max.y],
            [sprite.uv_min.x, sprite.uv_max.y],
        ];

        let mut verts = [SpriteVertex {
            position: [0.0; 2],
            uv: [0.0; 2],
            color: 0,
        }; 4];

        for i in 0..4 {
            let c = corners[i];
            let rotated = Vec2::new(c.x * cos - c.y * sin, c.x * sin + c.y * cos);
            let world = rotated + sprite.position;
            verts[i] = SpriteVertex {
                position: [world.x, world.y],
                uv: uvs[i],
                color: sprite.color,
            };
        }

        let b = base_vertex;
        let indices = [b, b + 1, b + 2, b, b + 2, b + 3];
        (verts, indices)
    }

    /// Render all sprites with sort-and-batch. Returns render statistics.
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

        // Build vertex/index data in sorted order
        let total_verts = sprites.len() * 4;
        let total_indices = sprites.len() * 6;
        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(total_verts);
        let mut indices: Vec<u32> = Vec::with_capacity(total_indices);

        for (quad_idx, &sprite_idx) in self.sort_order.iter().enumerate() {
            let (verts, idx) = Self::build_quad(&sprites[sprite_idx], (quad_idx * 4) as u32);
            vertices.extend_from_slice(&verts);
            indices.extend_from_slice(&idx);
        }

        // Grow buffers if needed
        if total_verts > self.vertex_capacity {
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_vbo"),
                size: (total_verts * std::mem::size_of::<SpriteVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.vertex_capacity = total_verts;
        }
        if total_indices > self.index_capacity {
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_ibo"),
                size: (total_indices * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.index_capacity = total_indices;
        }

        // Upload
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

        // Render pass with batched draw calls
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
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            // Batched draw: group consecutive sprites with same texture
            let mut batch_start: u32 = 0;
            let mut batch_texture = sprites[self.sort_order[0]].texture;

            for i in 1..=self.sort_order.len() {
                let at_end = i == self.sort_order.len();
                let texture_changed =
                    !at_end && sprites[self.sort_order[i]].texture != batch_texture;

                if at_end || texture_changed {
                    let batch_end = i as u32;
                    let index_start = batch_start * 6;
                    let index_end = batch_end * 6;

                    if let Some(bg) = self.texture_bind_groups.get(&batch_texture) {
                        pass.set_bind_group(1, bg, &[]);
                        pass.draw_indexed(index_start..index_end, 0, 0..1);
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
