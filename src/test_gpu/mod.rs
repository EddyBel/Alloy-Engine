use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window as WinitWindow, WindowAttributes};

pub struct TestApp {
    window: Option<Arc<WinitWindow>>,
    device: Option<Arc<wgpu::Device>>,
    queue: Option<Arc<wgpu::Queue>>,
    surface: Option<Arc<wgpu::Surface<'static>>>,
    config: Option<wgpu::SurfaceConfiguration>,
    pipeline: Option<Arc<wgpu::RenderPipeline>>,
    last_frame_time: Instant,
}

impl TestApp {
    pub fn new() -> Self {
        Self {
            window: None,
            device: None,
            queue: None,
            surface: None,
            config: None,
            pipeline: None,
            last_frame_time: Instant::now(),
        }
    }
}

impl ApplicationHandler for TestApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attributes = WindowAttributes::default()
                .with_title("Alloy GPU standalone test")
                .with_inner_size(PhysicalSize::new(800, 600));

            let winit_window = event_loop
                .create_window(attributes)
                .expect("failed create window");
            let window_arc = Arc::new(winit_window);

            // Inicializar WGPU
            let instance = wgpu::Instance::default();
            let surface = instance
                .create_surface(Arc::clone(&window_arc))
                .expect("create_surface");

            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                    ..Default::default()
                }))
                .expect("request_adapter");

            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    label: Some("test-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    ..Default::default()
                }))
                .expect("request_device");

            let caps = surface.get_capabilities(&adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(caps.formats[0]);

            let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                wgpu::PresentMode::Mailbox
            } else if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
                wgpu::PresentMode::Immediate
            } else {
                wgpu::PresentMode::Fifo
            };

            let size = window_arc.inner_size();
            let mut config = surface
                .get_default_config(&adapter, size.width, size.height)
                .expect("default_config");
            config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
            config.present_mode = present_mode;
            config.format = format;
            config.alpha_mode = caps.alpha_modes[0];
            surface.configure(&device, &config);

            println!(
                "[test_gpu] Adapter='{}' present_mode={:?} format={:?}",
                adapter.get_info().name,
                present_mode,
                config.format
            );

            self.window = Some(window_arc);
            self.device = Some(Arc::new(device));
            self.queue = Some(Arc::new(queue));
            self.surface = Some(Arc::new(surface));
            self.config = Some(config);

            // Crear shader + pipeline para triángulo fullscreen
            let shader_src = r#"
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    let p = positions[vi];
    return vec4<f32>(p, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.7, 0.2, 1.0);
}
"#;

            let shader =
                self.device
                    .as_ref()
                    .unwrap()
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("test-shader"),
                        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
                    });

            let pipeline_layout = self.device.as_ref().unwrap().create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("test-pipeline-layout"),
                    bind_group_layouts: &[],
                    immediate_size: 0,
                },
            );

            let pipeline = self.device.as_ref().unwrap().create_render_pipeline(
                &wgpu::RenderPipelineDescriptor {
                    label: Some("test-pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                },
            );

            self.pipeline = Some(Arc::new(pipeline));
            // Solicitar un primer redraw para que se lance el evento RedrawRequested
            if let Some(win) = self.window.as_ref() {
                win.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if let (Some(surface), Some(device)) = (self.surface.as_ref(), self.device.as_ref())
                {
                    if new_size.width > 0 && new_size.height > 0 {
                        if let Some(cfg) = self.config.as_mut() {
                            cfg.width = new_size.width;
                            cfg.height = new_size.height;
                            surface.configure(&device, cfg);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(surface), Some(device), Some(queue)) = (
                    self.surface.as_ref(),
                    self.device.as_ref(),
                    self.queue.as_ref(),
                ) {
                    let output = match surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(frame)
                        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                        other => {
                            eprintln!("[test_gpu] get_current_texture not available: {:?}", other);
                            return;
                        }
                    };

                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("test-encoder"),
                        });

                    if let Some(pipeline) = self.pipeline.as_ref() {
                        {
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("test-pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                                r: 0.0,
                                                g: 0.7,
                                                b: 0.2,
                                                a: 1.0,
                                            }),
                                            store: wgpu::StoreOp::Store,
                                        },
                                        depth_slice: None,
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                    multiview_mask: None,
                                });
                            rpass.set_pipeline(&pipeline);
                            rpass.draw(0..3, 0..1);
                        }

                        queue.submit(std::iter::once(encoder.finish()));
                        queue.present(output); //Linea clave para mostrar la renderizacion en la ventana

                        // IMPORTANTE: En wgpu moderno, la presentación del frame
                        // ocurre al soltar (drop) el objeto 'output'.
                        // Al estar dentro de este bloque, 'output' fenece aquí de forma limpia.
                    }
                }

                // Programar el siguiente frame de manera sincronizada
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let mut app = TestApp::new();
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
