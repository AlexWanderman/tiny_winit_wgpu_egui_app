use egui::Context;
use pollster::block_on;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

fn main() {
    simple_logger::init_with_level(log::Level::Warn).unwrap();

    let event_loop = EventLoop::new().unwrap();
    let control_flow = ControlFlow::Poll;
    let mut app = App::INITIAL;

    event_loop.set_control_flow(control_flow);
    event_loop.run_app(&mut app).unwrap();
}

enum App {
    INITIAL,
    ACTIVE {
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        render_pipeline: wgpu::RenderPipeline,
        egui_state: egui_winit::State,
        egui_renderer: egui_wgpu::Renderer,
    },
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_title("Tiny WINIT WGPU EGUI app")
            .with_inner_size(PhysicalSize::new(800, 600));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .unwrap();

        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let size = window.clone().inner_size();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let shader = device.create_shader_module(wgpu::include_wgsl!("main.wgsl"));

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor::default());

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip, //  Attention!!!
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let egui_context = Context::default();

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024), // default dimension is 2048
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            None,
            1,
            true,
        );

        *self = Self::ACTIVE {
            window,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            egui_state,
            egui_renderer,
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match self {
            Self::INITIAL => return,
            Self::ACTIVE { window, egui_state, .. } => {
                let response = egui_state.on_window_event(window, &event);

                if response.consumed {
                    return;
                }
            },
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(new_size) => match self {
                Self::INITIAL => return,
                Self::ACTIVE {
                    surface,
                    device,
                    config,
                    ..
                } => {
                    config.width = new_size.width;
                    config.height = new_size.height;

                    surface.configure(&device, &config);
                }
            },

            WindowEvent::RedrawRequested => match self {
                Self::INITIAL => return,
                Self::ACTIVE {
                    window,
                    surface,
                    device,
                    queue,
                    config,
                    render_pipeline,
                    egui_state,
                    egui_renderer,
                } => {
                    let output = surface.get_current_texture().unwrap();

                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                    // WGPU

                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("render_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    render_pass.set_pipeline(&render_pipeline);
                    render_pass.draw(0..4, 0..1);

                    drop(render_pass);

                    // EGUI

                    let raw_input = egui_state.take_egui_input(window);
                    egui_state.egui_ctx().begin_pass(raw_input);

                    egui::Window::new("WINIT + WGPU + EGUI = ♥")
                        .collapsible(false)
                        .resizable(false)
                        .show(egui_state.egui_ctx(), |ui| {
                            ui.label("Minimal working example.");
                            ui.spacing();
                            if ui.button("Exit").clicked() {
                                event_loop.exit();
                            }
                        });

                    let screen_descriptor = egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [config.width, config.height],
                        pixels_per_point: window.scale_factor() as f32,
                    };

                    let full_output = egui_state.egui_ctx().end_pass();
            
                    egui_state.handle_platform_output(window, full_output.platform_output);
            
                    let tris = egui_state
                        .egui_ctx()
                        .tessellate(full_output.shapes, egui_state.egui_ctx().pixels_per_point());
            
                    for (id, image_delta) in &full_output.textures_delta.set {
                        egui_renderer
                            .update_texture(&device, &queue, *id, image_delta);
                    }
            
                    egui_renderer.update_buffers(&device, &queue, &mut encoder, &tris, &screen_descriptor);
            
                    let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        label: Some("egui main render pass"),
                        occlusion_query_set: None,
                    });
            
                    egui_renderer.render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
                    for x in &full_output.textures_delta.free {
                        egui_renderer.free_texture(x)
                    }

                    // Present stuff

                    queue.submit(std::iter::once(encoder.finish()));
                    output.present();

                    window.request_redraw();
                }
            },

            _ => (),
        }
    }
}
