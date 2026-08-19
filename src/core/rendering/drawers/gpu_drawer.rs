use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::core::gpu_vertex::Gpu2DVertex;
use crate::core::rendering::drawers::drawer_backend::{Drawer, RenderBackend};
use wgpu::util::DeviceExt;

/// Backend de renderizado 2D acelerado por hardware basado en `wgpu`.
///
/// `GpuDrawer` implementa el patrón **Immediate Mode con Batching Retardado**:
/// en lugar de emitir llamadas de dibujo (*draw calls*) a la GPU por cada primitiva (línea,
/// rectángulo, triángulo), acumula los vértices en búferes en memoria RAM (`triangle_vertices`
/// y `line_vertices`). Al llamar a [`GpuDrawer::render`], transfiere toda la geometría a la GPU
/// en solo 2 llamadas de dibujo en lote (*draw batches*).
///
/// # Arquitectura Interna
/// - **Coordenadas de Pantalla**: Trabaja en espacio de pantalla 2D `(0, 0)` a `(WIDTH, HEIGHT)`.
///   El shader WGSL se encarga de convertir este espacio a *Normalized Device Coordinates* (NDC).
/// - **Uniforms**: Pasa el tamaño de la pantalla a la GPU mediante un *Uniform Buffer*.
/// - **Pipelines Separados**: Utiliza un pipeline con topología `TriangleList` para rellenos y otro
///   con `LineList` para contornos.
///
/// # Ejemplo de Uso
/// ```rust,ignore
/// let mut drawer = GpuDrawer::new(&device, &surface_config);
///
/// // En cada frame:
/// drawer.begin_frame();
/// drawer.clear([15, 15, 20, 255]);
/// drawer.draw_rect(50.0, 50.0, 200.0, 100.0, [255, 0, 0, 255]);
/// drawer.draw_line(0.0, 0.0, 800.0, 600.0, [0, 255, 0, 255]);
///
/// drawer.render(&device, &queue, &frame_view);
/// ```
pub struct GpuDrawer {
    /// Búfer en RAM para acumular vértices de primitivas rellenas (Triángulos/Rectángulos).
    triangle_vertices: Vec<Gpu2DVertex>,
    /// Búfer en RAM para acumular vértices de primitivas lineales (Líneas/Contornos).
    line_vertices: Vec<Gpu2DVertex>,

    /// Pipeline de renderizado configurado con topología `TriangleList`.
    triangle_pipeline: wgpu::RenderPipeline,
    /// Pipeline usado solo para dibujar un triángulo fullscreen de debug (asume posiciones en NDC).
    fullscreen_pipeline: wgpu::RenderPipeline,
    /// Pipeline de renderizado configurado con topología `LineList`.
    line_pipeline: wgpu::RenderPipeline,
    /// Textura de profundidad usada cuando `depth_enabled` es true.
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
    depth_width: u32,
    depth_height: u32,

    /// Búfer de la GPU que almacena la resolución actual de la pantalla `[ancho, alto]`.
    uniform_buffer: wgpu::Buffer,
    /// Grupo de enlace (*Bind Group*) que expone el `uniform_buffer` al *Vertex Shader*.
    uniform_bind_group: wgpu::BindGroup,

    /// Ancho actual de la superficie de renderizado en píxeles.
    width: f32,
    /// Alto actual de la superficie de renderizado en píxeles.
    height: f32,
    /// Configuración activa de la superficie para poder reconfigurarla en caso de `Outdated`.
    config: wgpu::SurfaceConfiguration,
    /// Color de fondo para limpiar la pantalla al inicio del paso de renderizado.
    clear_color: wgpu::Color,

    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface: Arc<wgpu::Surface<'static>>,
    /// Indica si el drawer debe usar la prueba de profundidad (el código crea un depth attachment por frame).
    depth_enabled: bool,
}

impl GpuDrawer {
    /// Crea e inicializa una nueva instancia de `GpuDrawer` configurando los shaders,
    /// layouts, bind groups y pipelines de WGPU.
    ///
    /// # Parámetros
    /// - `device`: Referencia lógica al dispositivo de la GPU (`wgpu::Device`).
    /// - `config`: Configuración actual de la superficie de renderizado (`wgpu::SurfaceConfiguration`).
    ///
    /// # Panics
    /// Hace *panic* si el archivo de shader WGSL no existe o falla al compilar.
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface: Arc<wgpu::Surface<'static>>,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        println!("[GpuDrawer LOG] Iniciando compilación de shaders y pipelines...");

        // Carga y compilación del sombreador WGSL desde el sistema de archivos en tiempo de compilación.
        let shader = device.create_shader_module(wgpu::include_wgsl!("../../../shader.wgsl"));
        println!("[GpuDrawer LOG] Shader WGSL cargado y compilado correctamente.");

        // ----------------------------------------------------------------------
        // 1. CONFIGURACIÓN DEL UNIFORM BUFFER (Matriz de Transformación 4x4)
        // ----------------------------------------------------------------------
        // El shader espera una estructura `Uniforms { transform: mat4x4<f32> }` (16 floats, 64 bytes).
        // Construimos una matriz column-major que convierte coordenadas de pantalla
        // (0..width, 0..height) a NDC (-1..1, -1..1) con Y invertida.
        let w = config.width as f32;
        let h = config.height as f32;

        let scale_x = 2.0 / w;
        let scale_y = -2.0 / h;
        let tx = -1.0;
        let ty = 1.0;

        // Column-major layout: columns are [c0, c1, c2, c3], each with 4 floats.
        let transform: [f32; 16] = [
            scale_x, 0.0, 0.0, 0.0, // column 0
            0.0, scale_y, 0.0, 0.0, // column 1
            0.0, 0.0, 1.0, 0.0,     // column 2
            tx, ty, 0.0, 1.0,       // column 3
        ];

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Drawer Uniform Buffer (transform mat4)"),
            contents: bytemuck::cast_slice(&transform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        println!(
            "[GpuDrawer LOG] Uploaded uniform transform ({} bytes)",
            std::mem::size_of_val(&transform)
        );

        // Definición del esquema/diseño del Bind Group para el sombreador
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Drawer Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX, // Solo visible en el Vertex Shader
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Creación del Bind Group concreto vinculando el buffer a la posición binding(0)
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Drawer Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Esquema global del pipeline (declara qué Bind Groups utilizará)
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Drawer Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        // ----------------------------------------------------------------------
        // 2. PIPELINE DE TRIÁNGULOS (Primitivas con Relleno)
        // ----------------------------------------------------------------------
        let triangle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Triangle Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Gpu2DVertex::desc())], // Layout del formato de vértice en memoria
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // Interpreta cada 3 vértices como un triángulo
                cull_mode: None,                                // No descarta caras (renderizado 2D)
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Pipeline de debug: vertex shader que ya recibe posiciones en clip-space (NDC)
        let fullscreen_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fullscreen Debug Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                buffers: &[Some(Gpu2DVertex::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // ----------------------------------------------------------------------
        // 3. PIPELINE DE LÍNEAS (Contornos y Bordes)
        // ----------------------------------------------------------------------
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Gpu2DVertex::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList, // Interpreta cada 2 vértices como un segmento
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        println!(
            "[GpuDrawer LOG] GpuDrawer inicializado de forma exitosa en el formato de superficie: {:?}",
            config.format
        );

        let mut inst = Self {
            // Reserva de capacidad inicial para minimizar asignaciones dinámicas en memoria por frame
            triangle_vertices: Vec::with_capacity(10_000),
            line_vertices: Vec::with_capacity(5_000),
            triangle_pipeline,
            fullscreen_pipeline,
            line_pipeline,
            uniform_buffer,
            uniform_bind_group,
            width: config.width as f32,
            height: config.height as f32,
            config: config.clone(),
            clear_color: wgpu::Color::BLACK,
            device,
            queue,
            surface,
            depth_enabled: false,
            depth_texture: None,
            depth_view: None,
            depth_width: config.width,
            depth_height: config.height,
        };

        // Crear los recursos de profundidad inicialmente para que el render pass
        // sea compatible con los pipelines que esperan un depth-stencil.
        let device_clone = Arc::clone(&inst.device);
        inst.create_depth_resources(&*device_clone);

        inst
    }

    /// Crea o recrea la textura y la vista de profundidad usando la `config` actual.
    fn create_depth_resources(&mut self, device: &wgpu::Device) {
        let size = wgpu::Extent3d {
            width: self.config.width,
            height: self.config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let depth_tex = device.create_texture(&desc);
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.depth_texture = Some(depth_tex);
        self.depth_view = Some(depth_view);
        self.depth_width = self.config.width;
        self.depth_height = self.config.height;
    }

    /// Llamar cuando la ventana cambie de tamaño; actualiza `config` y recrea recursos de profundidad.
    pub fn resize(&mut self, new_config: &wgpu::SurfaceConfiguration, device: &wgpu::Device) {
        self.config = new_config.clone();
        self.width = new_config.width as f32;
        self.height = new_config.height as f32;
        self.create_depth_resources(device);
    }

    /// Prepara el drawer para iniciar la acumulación de geometría de un nuevo fotograma.
    ///
    /// Limpia los búferes de vértices en memoria RAM sin liberar la capacidad reservada.
    /// Debe invocarse obligatoriamente antes de dibujar las primitivas de cada frame.
    pub fn begin_frame(&mut self) {
        self.triangle_vertices.clear();
        self.line_vertices.clear();
    }

    /// Procesa y envía toda la geometría acumulada hacia la GPU dentro de un `RenderPass`.
    ///
    /// Crea los Vertex Buffers temporales en la GPU para enviar las ráfagas de triángulos
    /// y líneas, codifica las órdenes de dibujo y las envía a la cola (`Queue`) del hardware.
    ///
    /// # Parámetros
    /// - `device`: Referencia al dispositivo GPU para instanciar buffers dinámicos.
    /// - `queue`: Cola de comandos de la GPU para enviar el `CommandBuffer` procesado.
    /// - `view`: Vista del objetivo de renderizado (`TextureView`) donde se dibujará la imagen final.
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, view: &wgpu::TextureView) {
        // Logs de diagnóstico desactivados para no penalizar el rendimiento por frame.
        // Se deja solo un punto de control muy puntual con una cuenta de frames si se necesita depurar.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GpuDrawer Encoder"),
        });

        {
            // Inicio del pase de renderizado (Render Pass)
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GpuDrawer Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color), // Limpia la pantalla con el clear_color
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                // Adjuntar siempre la vista de profundidad (los pipelines usan Depth32Float).
                // Aseguramos que existe y coincide el tamaño; si no, la recreamos.
                depth_stencil_attachment: {
                    if self.depth_view.is_none() || self.depth_width != self.config.width || self.depth_height != self.config.height {
                        self.create_depth_resources(device);
                    }
                    Some(wgpu::RenderPassDepthStencilAttachment {
                        view: self.depth_view.as_ref().unwrap(),
                        depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                        stencil_ops: None,
                    })
                },
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Asignación del Bind Group con los uniforms de resolución en el slot 0
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            // ------------------------------------------------------------------
            // 1. DIBUJO EN LOTE DE TRIÁNGULOS (Batch Render)
            // ------------------------------------------------------------------
            if !self.triangle_vertices.is_empty() {
                let tri_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Triangle Vertex Buffer"),
                    contents: bytemuck::cast_slice(&self.triangle_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                render_pass.set_pipeline(&self.triangle_pipeline);
                render_pass.set_vertex_buffer(0, tri_buffer.slice(..));
                render_pass.draw(0..self.triangle_vertices.len() as u32, 0..1);
            }

            // ------------------------------------------------------------------
            // 2. DIBUJO EN LOTE DE LÍNEAS (Batch Render)
            // ------------------------------------------------------------------
            if !self.line_vertices.is_empty() {
                let line_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Line Vertex Buffer"),
                    contents: bytemuck::cast_slice(&self.line_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                render_pass.set_pipeline(&self.line_pipeline);
                render_pass.set_vertex_buffer(0, line_buffer.slice(..));
                render_pass.draw(0..self.line_vertices.len() as u32, 0..1);
            }
        }

        // Envío final del búfer de comandos empaquetado a la GPU
        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Obtiene la textura actual de la ventana desde la superficie y crea su vista (`TextureView`).
    ///
    /// Maneja el enum `wgpu::CurrentSurfaceTexture` propio de las versiones recientes de `wgpu`.
    pub fn begin_surface_frame(&mut self) -> Option<(wgpu::SurfaceTexture, wgpu::TextureView)> {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => return None,
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        Some((output, view))
    }

    /// Presenta la textura de la superficie en la pantalla.
    pub fn present_surface_frame(&self, output: wgpu::SurfaceTexture) {
        // En wgpu 0.30 la presentación hay que hacerla sobre la cola de comandos.
        // Es la misma lógica que funciona en la mini prueba: queue.submit(...); queue.present(output);
        self.queue.present(output);
    }

    /// Conveniencia: ejecuta un ciclo completo de renderizado usando los `Arc` internos
    /// de `device`, `queue` y `surface` que esta instancia ya almacena.
    ///
    /// - Llama a `begin_frame()` para limpiar buffers de vértices internos.
    /// - Ejecuta la clausura `render_fn` para acumular primitivas en los búferes.
    /// - Obtiene la textura de la superficie, ejecuta `render()` y presenta el frame.
    pub fn render_frame_auto<F>(&mut self, mut render_fn: F)
    where
        F: FnMut(&mut dyn Drawer),
    {
        // 1. Preparar el frame interno
        self.begin_frame();

        // 2. Ejecutar las llamadas de dibujo del usuario
        render_fn(self);

        // 3. Clonar los Arcs necesarios para evitar conflictos de préstamo
        let device_arc = Arc::clone(&self.device);
        let queue_arc = Arc::clone(&self.queue);
        let surface_arc = Arc::clone(&self.surface);

        // 4. Obtener la textura actual de la superficie desde la copia clonada.
        // Si no está disponible, reintentamos un par de veces y reconfiguramos la superficie
        // cuando el backbuffer quedó obsoleto. Esto evita que el render GPU se corte en silencio.
        let mut attempts: u8 = 0;
        let output = loop {
            let res = surface_arc.get_current_texture();
            match res {
                wgpu::CurrentSurfaceTexture::Success(frame)
                | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => break frame,
                wgpu::CurrentSurfaceTexture::Timeout => {
                    attempts += 1;
                    eprintln!("[GpuDrawer WARN] get_current_texture timeout (attempt {})", attempts);
                    if attempts >= 3 {
                        eprintln!("[GpuDrawer WARN] surface timed out; skipping frame to recover");
                        return;
                    }
                    thread::sleep(Duration::from_millis(8));
                    continue;
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    eprintln!("[GpuDrawer WARN] surface outdated; reconfiguring");
                    surface_arc.configure(&*device_arc, &self.config);
                    attempts += 1;
                    if attempts >= 3 {
                        eprintln!("[GpuDrawer WARN] surface still outdated after retries; skipping frame");
                        return;
                    }
                    thread::sleep(Duration::from_millis(8));
                    continue;
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    eprintln!("[GpuDrawer WARN] surface lost; recreating surface/config required");
                    return;
                }
                wgpu::CurrentSurfaceTexture::Occluded => {
                    eprintln!("[GpuDrawer WARN] surface occluded; skipping frame");
                    return;
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    eprintln!("[GpuDrawer WARN] validation error while acquiring surface texture");
                    return;
                }
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // 5. Ejecutar el render usando device/queue clonados y la vista recién creada
        self.render(&*device_arc, &*queue_arc, &view);

        // 6. Presentar la textura
        self.present_surface_frame(output);
    }
}

// =========================================================================
// IMPLEMENTACIÓN DEL TRAIT Drawer (API Agnóstica de Dibujo 2D)
// =========================================================================
impl Drawer for GpuDrawer {
    /// Identifica explícitamente a este backend como acelerado por hardware (GPU).
    fn backend_type(&self) -> RenderBackend {
        RenderBackend::Gpu
    }

    /// Define el color de limpieza que se aplicará al framebuffer al llamar a [`GpuDrawer::render`].
    ///
    /// Convierte el formato normalizado en enteros de 8 bits `[R, G, B, A]` (0-255)
    /// al formato de punto flotante de `wgpu::Color` (0.0 - 1.0).
    fn clear(&mut self, color: [u8; 4]) {
        self.clear_color = wgpu::Color {
            r: color[0] as f64 / 255.0,
            g: color[1] as f64 / 255.0,
            b: color[2] as f64 / 255.0,
            a: color[3] as f64 / 255.0,
        };
    }

    /// Dibuja un único píxel en la posición `(x, y)` especificada.
    ///
    /// *Nota de implementación*: En la GPU, un píxel individual se simula internamente
    /// dibujando un rectángulo de dimensiones 1x1.
    fn draw_pixel(&mut self, x: f32, y: f32, color: [u8; 4]) {
        self.draw_rect(x, y, 1.0, 1.0, color);
    }

    /// Acumula un segmento de línea desde `(x0, y0)` hasta `(x1, y1)`.
    ///
    /// Inserta 2 vértices mapeados con colores normalizados en `line_vertices`.
    fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: [u8; 4]) {
        let col = [
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
            color[3] as f32 / 255.0,
        ];
        self.line_vertices.push(Gpu2DVertex {
            position: [x0, y0, 0.0],
            color: col,
        });
        self.line_vertices.push(Gpu2DVertex {
            position: [x1, y1, 0.0],
            color: col,
        });
    }

    /// Dibuja el contorno de un triángulo trazando sus 3 segmentos de línea.
    fn draw_triangle(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [u8; 4],
    ) {
        self.draw_line(x0, y0, x1, y1, color);
        self.draw_line(x1, y1, x2, y2, color);
        self.draw_line(x2, y2, x0, y0, color);
    }

    /// Acumula un triángulo sólido relleno definido por tres puntos.
    ///
    /// Inserta 3 vértices normalizados en `triangle_vertices`.
    fn draw_filled_triangle(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [u8; 4],
    ) {
        let col = [
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
            color[3] as f32 / 255.0,
        ];
        self.triangle_vertices.push(Gpu2DVertex {
            position: [x0, y0, 0.0],
            color: col,
        });
        self.triangle_vertices.push(Gpu2DVertex {
            position: [x1, y1, 0.0],
            color: col,
        });
        self.triangle_vertices.push(Gpu2DVertex {
            position: [x2, y2, 0.0],
            color: col,
        });
    }

    fn draw_line_3d(&mut self, x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, color: [u8; 4]) {
        let col = [
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
            color[3] as f32 / 255.0,
        ];
        self.line_vertices.push(Gpu2DVertex {
            position: [x0, y0, z0],
            color: col,
        });
        self.line_vertices.push(Gpu2DVertex {
            position: [x1, y1, z1],
            color: col,
        });
    }

    fn draw_filled_triangle_3d(
        &mut self,
        x0: f32,
        y0: f32,
        z0: f32,
        x1: f32,
        y1: f32,
        z1: f32,
        x2: f32,
        y2: f32,
        z2: f32,
        color: [u8; 4],
    ) {
        let col = [
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
            color[3] as f32 / 255.0,
        ];
        self.triangle_vertices.push(Gpu2DVertex {
            position: [x0, y0, z0],
            color: col,
        });
        self.triangle_vertices.push(Gpu2DVertex {
            position: [x1, y1, z1],
            color: col,
        });
        self.triangle_vertices.push(Gpu2DVertex {
            position: [x2, y2, z2],
            color: col,
        });
    }

    /// Dibuja un rectángulo relleno.
    ///
    /// Descompone el rectángulo en 2 triángulos y los acumula en `triangle_vertices`.
    ///
    /// ```text
    /// (x, y) ------- (x2, y)
    ///   |   \           |
    ///   |     \   T2    |
    ///   |  T1   \       |
    /// (x, y2) ------ (x2, y2)
    /// ```
    fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
        let x2 = x + w;
        let y2 = y + h;

        // Primer triángulo del cuadrilátero (Top-Left, Bottom-Left, Top-Right)
        self.draw_filled_triangle(x, y, x, y2, x2, y, color);
        // Segundo triángulo del cuadrilátero (Top-Right, Bottom-Left, Bottom-Right)
        self.draw_filled_triangle(x2, y, x, y2, x2, y2, color);
    }

    fn enable_depth(&mut self, enabled: bool) {
        self.depth_enabled = enabled;
    }

    fn clear_depth_buffer(&mut self) {
        // En GPU creamos y limpiamos la textura de profundidad por frame en `render()`.
        // No hay acción necesaria aquí en el acumulador inmediato.
    }

    fn set_current_depth(&mut self, _depth: f32) {
        // En este diseño actual el GpuDrawer no usa la profundidad escalar por primitiva.
        // Implementación futura: pasar depth por vértice o uniform.
    }
}