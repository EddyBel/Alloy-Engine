use pixels::{Pixels, SurfaceTexture};
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window as WinitWindow, WindowAttributes, WindowId};

use crate::core::controls::keyboard::{Key, KeyboardManager};
use crate::core::window::state::State;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

struct FpsCounter {
    last_update: Instant,
    frames: u32,
    fps: f32,
    last_valid_fps: f32,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            last_update: Instant::now(),
            frames: 0,
            fps: 60.0,
            last_valid_fps: 60.0,
        }
    }

    fn tick(&mut self) -> f32 {
        self.frames += 1;

        let elapsed = self.last_update.elapsed();
        if elapsed >= std::time::Duration::from_millis(250) {
            let elapsed_secs = elapsed.as_secs_f32().max(0.0001);
            let measured = self.frames as f32 / elapsed_secs;

            self.fps = measured;
            self.last_valid_fps = measured;
            self.frames = 0;
            self.last_update = Instant::now();
        }

        if self.fps <= 0.0 {
            self.last_valid_fps
        } else {
            self.fps
        }
    }
}

pub struct Window {
    window: Arc<WinitWindow>,
    title: String,
}

impl Window {
    pub fn new(
        event_loop: &ActiveEventLoop,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn Error>> {
        let attributes = WindowAttributes::default()
            .with_title(title)
            .with_inner_size(PhysicalSize::new(width, height));

        let winit_window = event_loop.create_window(attributes)?;
        let window_arc = Arc::new(winit_window);

        Ok(Self {
            window: window_arc,
            title: title.to_string(),
        })
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.window.inner_size()
    }

    pub fn raw_window(&self) -> Arc<WinitWindow> {
        Arc::clone(&self.window)
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }
}

pub struct App<S: State> {
    window: Option<Window>,
    pixels: Option<Pixels<'static>>,
    state: S,
    keyboard: KeyboardManager,
    last_frame_time: Instant,
    last_redraw_request: Instant,
    target_frame_duration: std::time::Duration,
    fps_counter: FpsCounter,
}

impl<S: State> App<S> {
    pub fn new(state: S) -> Self {
        Self {
            window: None,
            pixels: None,
            state,
            keyboard: KeyboardManager::new(),
            last_frame_time: Instant::now(),
            last_redraw_request: Instant::now(),
            target_frame_duration: std::time::Duration::from_secs_f32(1.0 / 60.0),
            fps_counter: FpsCounter::new(),
        }
    }
}

fn map_winit_key(event: &winit::event::KeyEvent) -> Key {
    if let PhysicalKey::Code(code) = event.physical_key {
        use winit::keyboard::KeyCode::*;
        match code {
            KeyA => Key::A,
            KeyB => Key::B,
            KeyC => Key::C,
            KeyD => Key::D,
            KeyE => Key::E,
            KeyF => Key::F,
            KeyG => Key::G,
            KeyH => Key::H,
            KeyI => Key::I,
            KeyJ => Key::J,
            KeyK => Key::K,
            KeyL => Key::L,
            KeyM => Key::M,
            KeyN => Key::N,
            KeyO => Key::O,
            KeyP => Key::P,
            KeyQ => Key::Q,
            KeyR => Key::R,
            KeyS => Key::S,
            KeyT => Key::T,
            KeyU => Key::U,
            KeyV => Key::V,
            KeyW => Key::W,
            KeyX => Key::X,
            KeyY => Key::Y,
            KeyZ => Key::Z,
            Digit0 => Key::Num0,
            Digit1 => Key::Num1,
            Digit2 => Key::Num2,
            Digit3 => Key::Num3,
            Digit4 => Key::Num4,
            Digit5 => Key::Num5,
            Digit6 => Key::Num6,
            Digit7 => Key::Num7,
            Digit8 => Key::Num8,
            Digit9 => Key::Num9,
            ArrowUp => Key::Up,
            ArrowDown => Key::Down,
            ArrowLeft => Key::Left,
            ArrowRight => Key::Right,
            Space => Key::Space,
            Enter => Key::Enter,
            Escape => Key::Escape,
            ShiftLeft | ShiftRight => Key::Shift,
            ControlLeft | ControlRight => Key::Control,
            AltLeft | AltRight => Key::Alt,
            Tab => Key::Tab,
            _ => Key::Unknown,
        }
    } else {
        Key::Unknown
    }
}

impl<S: State> ApplicationHandler for App<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Window::new(event_loop, "Alloy Engine", 800, 600)
                .expect("Error al crear la ventana nativa");

            let window_size = window.size();

            // Configuración de Pixels para el buffer de CPU fallback
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, window.raw_window());

            let pixels =
                Pixels::new(WIDTH, HEIGHT, surface_texture).expect("Error al inicializar Pixels");

            // ----------------------------------------------------------------------
            // INICIALIZACIÓN GPU EN EL MOMENTO QUE NACE LA VENTANA (wgpu 30.0+)
            // ----------------------------------------------------------------------
            // DENTRO DE resumed() en window.rs:
            let raw_win = window.raw_window();
            pollster::block_on(async {
                let instance = wgpu::Instance::default();
                // Importante: La superficie debe tener vida 'static vinculada a la ventana Arc
                if let Ok(surface) = instance.create_surface(Arc::clone(&raw_win)) {
                    if let Ok(adapter) = instance
                        .request_adapter(&wgpu::RequestAdapterOptions {
                            power_preference: wgpu::PowerPreference::HighPerformance,
                            compatible_surface: Some(&surface),
                            force_fallback_adapter: false,
                            ..Default::default()
                        })
                        .await
                    {
                        let info = adapter.get_info();
                        let caps = surface.get_capabilities(&adapter);

                        if let Ok((device, queue)) = adapter
                            .request_device(&wgpu::DeviceDescriptor {
                                label: Some("Alloy GPU Device"),
                                required_features: wgpu::Features::empty(),
                                required_limits: wgpu::Limits::default(),
                                memory_hints: wgpu::MemoryHints::default(),
                                ..Default::default()
                            })
                            .await
                        {
                            let surface_caps = surface.get_capabilities(&adapter);

                            if let Some(mut config) = surface.get_default_config(
                                &adapter,
                                window_size.width,
                                window_size.height,
                            ) {
                                config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
                                // Elegir el mejor present_mode disponible para diagnóstico (Mailbox -> Immediate -> Fifo)
                                if surface_caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                                    config.present_mode = wgpu::PresentMode::Mailbox;
                                } else if surface_caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
                                    config.present_mode = wgpu::PresentMode::Immediate;
                                } else {
                                    config.present_mode = wgpu::PresentMode::Fifo;
                                }
                                config.alpha_mode = surface_caps.alpha_modes[0];
                                config.desired_maximum_frame_latency = 2;
                                config.format = surface_caps
                                    .formats
                                    .iter()
                                    .copied()
                                    .find(|f| f.is_srgb())
                                    .unwrap_or(surface_caps.formats[0]);

                                // 1. CONFIGURACIÓN VITAL DE LA SUPERFICIE
                                surface.configure(&device, &config);

                                // 2. Entregamos los recursos encapsulados en Arc
                                self.state.init_gpu(
                                    Arc::new(device),
                                    Arc::new(queue),
                                    Arc::new(surface),
                                    config,
                                );
                            }
                        }
                    }
                }
            });

            self.pixels = Some(pixels);
            self.window = Some(window);
            self.last_frame_time = Instant::now();

            // Importante: sin esto la ventana se crea pero nunca vuelve a pedir
            // un `RedrawRequested`, por lo que la imagen queda estática tras la inicialización.
            if let Some(window) = &self.window {
                window.request_redraw();
                self.last_redraw_request = Instant::now();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let mapped_key = map_winit_key(&event);
                if mapped_key != Key::Unknown {
                    match event.state {
                        ElementState::Pressed => self.keyboard.register_press(mapped_key),
                        ElementState::Released => self.keyboard.register_release(mapped_key),
                    }
                }
            }

            WindowEvent::Resized(new_size) => {
                if let Some(pixels) = &mut self.pixels {
                    if new_size.width > 0 && new_size.height > 0 {
                        let _ = pixels.resize_surface(new_size.width, new_size.height);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame_time).as_secs_f32();
                self.last_frame_time = now;

                self.state.update(dt, &self.keyboard);
                self.keyboard.begin_frame();

                let fps = self.fps_counter.tick();
                if let Some(window) = &self.window {
                    let display_fps = if fps > 0.0 { fps } else { self.fps_counter.last_valid_fps };
                    window.set_title(&format!("Alloy Engine - FPS: {:.1}", display_fps));
                }

                let state_ptr: *mut S = &mut self.state;

                unsafe {
                    let drawer_manager = (&mut *state_ptr).drawer_manager();

                    if drawer_manager.is_using_gpu() {
                        // Usamos la nueva API de conveniencia en GpuDrawer que gestiona
                        // device/queue/surface internamente y presenta el frame.
                        if let Some(gpu) = drawer_manager.gpu_mut() {
                            gpu.render_frame_auto(|drawer| {
                                (&mut *state_ptr).render(drawer);
                            });
                        }
                    } else if let Some(pixels) = &mut self.pixels {
                        // Renderizado por Software (CPU)
                        let frame = pixels.frame_mut();
                        drawer_manager.render_frame(frame, WIDTH, HEIGHT, |drawer| {
                            (&mut *state_ptr).render(drawer);
                        });

                        if pixels.render().is_err() {
                            eprintln!("Error al renderizar píxeles");
                            event_loop.exit();
                            return;
                        }
                    }
                }

                if let Some(window) = &self.window {
                    // Mantener el loop de refresco activo en cada frame. Si no se vuelve a pedir
                    // `RedrawRequested`, la ventana queda congelada aunque GPU y lógica sigan vivos.
                    window.request_redraw();
                    self.last_redraw_request = Instant::now();
                }
            }

            _ => (),
        }
    }
}

pub fn run<S: State + 'static>(game: S) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(game);
    event_loop.run_app(&mut app)?;

    Ok(())
}
