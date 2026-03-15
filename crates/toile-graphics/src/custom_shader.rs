// Toile Engine — Custom shader pipeline (ADR-027)
//
// A `CustomShaderPipeline` wraps a wgpu render pipeline compiled from a
// user-provided WGSL string (typically produced by `ShaderGraph::compile()`).
//
// It fits into the PostProcessor chain as `PostEffect::Custom(Arc<...>)`.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

/// 16-byte uniform uploaded to @group(1) each frame.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CustomParamsGpu {
    time: f32,
    _pad0: f32,
    screen_w: f32,
    screen_h: f32,
}

/// A GPU-compiled custom post-processing shader.
///
/// Wrap in `Arc` and push as `PostEffect::Custom(Arc::clone(&pipeline))`.
pub struct CustomShaderPipeline {
    pub(crate) pipeline:   wgpu::RenderPipeline,
    pub(crate) params_buf: wgpu::Buffer,
    pub(crate) params_bg:  wgpu::BindGroup,
    /// Store the name for debugging.
    pub name: String,
}

impl std::fmt::Debug for CustomShaderPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CustomShaderPipeline({})", self.name)
    }
}

impl CustomShaderPipeline {
    /// Compile a WGSL shader into a GPU pipeline.
    ///
    /// `tex_bgl` must be the same `BindGroupLayout` that `PostProcessor`
    /// uses for group 0 (texture + sampler).  Use `pp.tex_bgl`.
    pub fn new(
        name: impl Into<String>,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        tex_bgl: &wgpu::BindGroupLayout,
        wgsl: &str,
    ) -> Result<Arc<Self>, String> {
        // Group 1: time + screen size
        let params_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("custom_shader_params_bgl"),
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

        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("custom_shader_params"),
            size:  std::mem::size_of::<CustomParamsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("custom_shader_params_bg"),
            layout: &params_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buf.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("custom_shader_layout"),
            bind_group_layouts: &[tex_bgl, &params_bgl],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom_shader_module"),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("custom_shader_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vs_main"),
                buffers: &[], compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"),
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

        Ok(Arc::new(Self {
            pipeline, params_buf, params_bg,
            name: name.into(),
        }))
    }

    /// Upload current time + screen dimensions to the GPU.
    pub fn update_params(
        &self,
        queue: &wgpu::Queue,
        time: f32,
        screen_w: f32,
        screen_h: f32,
    ) {
        queue.write_buffer(
            &self.params_buf, 0,
            bytemuck::bytes_of(&CustomParamsGpu { time, _pad0: 0.0, screen_w, screen_h }),
        );
    }
}
