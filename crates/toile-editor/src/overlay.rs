use egui_wgpu::ScreenDescriptor;
use winit::event::WindowEvent;
use winit::window::Window;

/// Wraps egui context, winit state, and wgpu renderer.
pub struct EguiOverlay {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl EguiOverlay {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        let ctx = egui::Context::default();
        let viewport_id = egui::ViewportId::ROOT;
        let state = egui_winit::State::new(
            ctx.clone(),
            viewport_id,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let renderer = egui_wgpu::Renderer::new(
            device,
            surface_format,
            Default::default(),
        );

        Self {
            ctx,
            state,
            renderer,
        }
    }

    /// Feed a winit event to egui. Returns true if consumed.
    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    /// Get the egui context for building UI.
    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
    }

    /// Begin an egui frame.
    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.ctx.begin_frame(raw_input);
    }

    /// End the frame and render egui on top of existing content.
    /// Uses a separate command encoder to avoid lifetime issues.
    pub fn end_frame_and_render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
        screen_size: (u32, u32),
    ) {
        let full_output = self.ctx.end_frame();

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen = ScreenDescriptor {
            size_in_pixels: [screen_size.0, screen_size.1],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        // egui-wgpu's Renderer expects owned encoder for the render pass
        // due to lifetime constraints. We create a fresh one and submit separately.
        let mut egui_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("egui_encoder"),
            });

        self.renderer
            .update_buffers(device, queue, &mut egui_encoder, &tris, &screen);

        // Submit the buffer updates
        queue.submit(std::iter::once(egui_encoder.finish()));

        let mut render_encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("egui_render") },
        );

        let pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        // forget_lifetime converts RenderPass<'a> to RenderPass<'static>
        // which is required by egui-wgpu's Renderer::render signature.
        let mut pass = pass.forget_lifetime();
        self.renderer.render(&mut pass, &tris, &screen);
        drop(pass);

        queue.submit(std::iter::once(render_encoder.finish()));

        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}
