use crate::core::rendering::drawers::cpu_drawers::CPUDrawer;
use crate::core::rendering::drawers::drawer_backend::{Drawer, RenderBackend};
use crate::core::rendering::drawers::gpu_drawer::GpuDrawer;

/// Configuración de preferencia de backend para el renderizado.
///
/// Define la política de selección de hardware/software que utilizará [`DrawerManager`]
/// para determinar qué motor de dibujado se usará en cada frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPreference {
    /// Selección automática: usa GPU si está disponible e inicializada; de lo contrario, usa CPU.
    Auto,
    /// Fuerza el uso de la GPU. Si la GPU no está disponible, el renderizado no se ejecutará.
    ForceGpu,
    /// Fuerza el uso exclusivo del renderizado por software en CPU, ignorando la GPU aunque exista.
    ForceCpu,
}

/// Gestor y despachador dinámico de backends de renderizado (CPU / GPU).
///
/// `DrawerManager` encapsula la lógica de fallback y selección entre renderizado
/// acelerado por hardware ([`GpuDrawer`]) y renderizado por software ([`CPUDrawer`]).
/// Oculta los detalles del backend al resto del motor y expone una interfaz unificada
/// mediante el trait [`DrawerBackend`].
///
/// # Estrategia de Lifetimes y Buffers
/// - **GPU**: Mantiene una instancia persistente de [`GpuDrawer`] con sus pipelines y buffers.
/// - **CPU**: [`CPUDrawer`] se instancia dinámicamente *por frame* dentro de [`DrawerManager::render_frame`],
///   ya que requiere un préstamo mutable directo (`&mut [u8]`) sobre el framebuffer del ciclo actual.
///
/// # Ejemplo de Uso
/// ```rust,ignore
/// let mut manager = DrawerManager::new(some_gpu_drawer);
/// manager.set_preference(BackendPreference::Auto);
///
/// // En el evento RedrawRequested:
/// manager.render_frame(frame_buffer, 640, 480, |drawer| {
///     drawer.clear([10, 10, 15, 255]);
///     drawer.draw_rect(20.0, 20.0, 100.0, 50.0, [255, 0, 0, 255]);
/// });
/// ```
pub struct DrawerManager {
    /// Instancia opcional del backend de GPU acelerado por hardware.
    gpu_drawer: Option<GpuDrawer>,
    /// Criterio o preferencia actual para seleccionar el backend.
    preference: BackendPreference,
    /// Flag para registrar el estado una sola vez y evitar saturar la consola en cada frame
    logged_status: bool,
}

impl DrawerManager {
    /// Instancia un nuevo `DrawerManager` con un `GpuDrawer` opcional y preferencia por defecto en `Auto`.
    ///
    /// # Parámetros
    /// - `gpu_drawer`: Instancia inicializada de `GpuDrawer`, o `None` si la GPU no se pudo crear o no se usará.
    pub fn new(gpu_drawer: Option<GpuDrawer>) -> Self {
        if gpu_drawer.is_none() {
            eprintln!(
                "[DrawerManager LOG] ADVERTENCIA: DrawerManager fue instanciado sin 'gpu_drawer' (gpu_drawer = None)."
            );
        } else {
            println!(
                "[DrawerManager LOG] ÉXITO: DrawerManager recibió una instancia válida de 'GpuDrawer'."
            );
        }

        Self {
            gpu_drawer,
            preference: BackendPreference::Auto,
            logged_status: false,
        }
    }

    /// Modifica la preferencia de backend en tiempo de ejecución (CPU, GPU o Auto).
    ///
    /// Permite alternar dinámicamente el motor de renderizado según configuraciones del usuario
    /// o cambios de rendimiento.
    pub fn set_preference(&mut self, preference: BackendPreference) {
        self.preference = preference;
    }

    /// Obtiene la preferencia de backend configurada actualmente.
    pub fn preference(&self) -> BackendPreference {
        self.preference
    }

    /// Devuelve el tipo de backend de renderizado activo actualmente ([`RenderBackend::Gpu`] o [`RenderBackend::Cpu`]).
    pub fn active_backend(&self) -> RenderBackend {
        if self.is_using_gpu() {
            RenderBackend::Gpu
        } else {
            RenderBackend::Cpu
        }
    }

    /// Evalúa en tiempo real si el frame actual debe renderizarse mediante GPU.
    ///
    /// # Criterio de Selección
    /// - `ForceGpu`: Retorna `true` solo si `gpu_drawer` está disponible (`Some`).
    /// - `ForceCpu`: Retorna siempre `false`.
    /// - `Auto`: Retorna `true` si `gpu_drawer` está disponible (`Some`); `false` si es `None`.
    pub fn is_using_gpu(&self) -> bool {
        let has_gpu = self.gpu_drawer.is_some();

        let uses_gpu = match self.preference {
            BackendPreference::ForceGpu => {
                if !has_gpu && !self.logged_status {
                    eprintln!(
                        "[DrawerManager ERROR CRÍTICO] Se configuró 'ForceGpu', pero 'gpu_drawer' es None. Forzando modo CPU."
                    );
                }
                has_gpu
            }
            BackendPreference::ForceCpu => false,
            BackendPreference::Auto => has_gpu,
        };

        if !self.logged_status {
            println!(
                "[DrawerManager DIAGNÓSTICO] Estado actual -> Preferencia: {:?}, Instancia GPU Presente: {}, Backend Seleccionado: {:?}",
                self.preference,
                has_gpu,
                if uses_gpu {
                    RenderBackend::Gpu
                } else {
                    RenderBackend::Cpu
                }
            );
        }

        uses_gpu
    }

    /// Evalúa en tiempo real si el frame actual se está renderizando por software en la CPU.
    pub fn is_using_cpu(&self) -> bool {
        !self.is_using_gpu()
    }

    /// Devuelve una referencia mutable a la instancia de [`GpuDrawer`] únicamente si la GPU está activa.
    ///
    /// Útil cuando la ventana o el motor necesitan invocar métodos específicos de la GPU
    /// (como `begin_frame()` o `render()`) que no forman parte del trait genérico [`DrawerBackend`].
    pub fn gpu_mut(&mut self) -> Option<&mut GpuDrawer> {
        if self.is_using_gpu() {
            self.gpu_drawer.as_mut()
        } else {
            None
        }
    }

    /// Ejecuta una clausura de dibujado pasando una instancia genérica de [`DrawerBackend`].
    ///
    /// Este método permite realizar operaciones de dibujo si ya se dispone externamente de una
    /// instancia opcional de [`CPUDrawer`].
    ///
    /// # Parámetros
    /// - `cpu_drawer`: Referencia mutable opcional a un `CPUDrawer` preexistente.
    /// - `action`: Clausura que recibe una referencia mutable `&mut dyn DrawerBackend`.
    pub fn with_drawer<F>(&mut self, cpu_drawer: Option<&mut CPUDrawer>, mut action: F)
    where
        F: FnMut(&mut dyn Drawer),
    {
        // 1. Prioridad GPU: Si la evaluación determina uso de GPU y existe la instancia
        if self.is_using_gpu() {
            if let Some(ref mut gpu) = self.gpu_drawer {
                action(gpu);
            }
        // 2. Fallback CPU: Si no usa GPU y se proporcionó una instancia válida de CPU
        } else if let Some(cpu) = cpu_drawer {
            action(cpu);
        }
    }

    /// Coordina y ejecuta el ciclo completo de renderizado de un frame sobre el buffer proporcionado.
    ///
    /// Abstrae por completo el backend subyacente. Si está en modo GPU, delega las llamadas
    /// de dibujo a `GpuDrawer`. Si está en modo CPU, instancia un [`CPUDrawer`] temporal
    /// amarrado a la vida del buffer `frame` de este ciclo.
    ///
    /// # Parámetros
    /// - `frame`: Slice de píxeles mutables en memoria RAM (`&mut [u8]`) en formato RGBA.
    /// - `width`: Ancho del framebuffer en píxeles.
    /// - `height`: Alto del framebuffer en píxeles.
    /// - `render_fn`: Clausura de renderizado donde el usuario emite las órdenes de dibujo.
    ///
    /// # Ejemplo de Uso
    /// ```rust,ignore
    /// drawer_manager.render_frame(frame, 640, 480, |drawer| {
    ///     drawer.clear([0, 0, 0, 255]);
    ///     drawer.draw_line(0.0, 0.0, 100.0, 100.0, [255, 255, 255, 255]);
    /// });
    /// ```
    pub fn render_frame<F>(&mut self, frame: &mut [u8], width: u32, height: u32, mut render_fn: F)
    where
        F: FnMut(&mut dyn Drawer),
    {
        if self.is_using_gpu() {
            // Renderizado por GPU: Pasa la instancia persistente de GpuDrawer a la clausura
            if let Some(ref mut gpu) = self.gpu_drawer {
                render_fn(gpu);
            }
        } else {
            // Renderizado por Software (CPU):
            // Se instancia temporalmente el CPUDrawer vinculado únicamente al ciclo de vida del buffer 'frame'
            let mut cpu = CPUDrawer::new(frame, width, height);
            render_fn(&mut cpu);
        }
    }

    /// Ejecuta el renderizado acelerado por hardware (GPU).
    /// Pasa el `GpuDrawer` interno a la clausura de dibujo y gestiona el ciclo del frame.
    /// Ejecuta el renderizado acelerado por hardware (GPU).
    /// Pasa el `GpuDrawer` interno a la clausura de dibujo y gestiona el ciclo del frame.
    /// Ejecuta el renderizado acelerado por hardware (GPU).
    /// Pasa el `GpuDrawer` interno a la clausura de dibujo y gestiona el ciclo del frame.
    pub fn render_frame_gpu<F>(
        &mut self,
        surface: &wgpu::Surface<'static>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mut render_fn: F,
    ) where
        F: FnMut(&mut dyn Drawer),
    {
        if let Some(ref mut gpu) = self.gpu_drawer {
            // 1. Limpiamos los buffers locales de vértices
            gpu.begin_frame();

            // 2. Ejecutamos las llamadas de dibujo del usuario
            render_fn(gpu);

            // 3. Obtenemos la textura actual de la superficie
            if let Some((output, view)) = gpu.begin_surface_frame() {
                // 4. Renderizamos los vértices acumulados
                gpu.render(device, queue, &view);

                // 5. Presentamos la textura para que el frame sea visible en pantalla.
                gpu.present_surface_frame(output);
            }
        }
    }
}
