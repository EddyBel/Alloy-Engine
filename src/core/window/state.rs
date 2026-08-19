use crate::core::controls::keyboard::KeyboardManager;
use crate::core::rendering::drawers::drawer_backend::Drawer;
use crate::core::rendering::drawers::drawer_manager::DrawerManager;
use std::sync::Arc;

/// Trait que define el contrato de ciclo de vida para un estado de la aplicación/juego.
///
/// El trait `State` abstrae cualquier "pantalla" o "escena" del sistema (por ejemplo: menú principal,
/// nivel de juego, pantalla de pausa). Implementa el patrón **State / Game Loop** dividiendo
/// la ejecución de cada frame en tres responsabilidades claras:
///
/// 1. **Manejo de Lógica e Inputs** ([`State::update`]): Procesamiento independiente del renderizado.
/// 2. **Renderizado de Escena** ([`State::render`]): Emisión de primitivas de dibujo agnósticas al hardware.
/// 3. **Gestión de Backend de Video** ([`State::drawer_manager`]): Acceso y control dinámico sobre el pipeline CPU/GPU.
///
/// # Ejemplo de Implementación
/// ```rust,ignore
/// struct GameState {
///     player_x: f32,
///     drawer_mgr: DrawerManager,
/// }
///
/// impl State for GameState {
///     fn update(&mut self, dt: f32, keyboard: &KeyboardManager) {
///         if keyboard.is_key_pressed(KeyCode::ArrowRight) {
///             self.player_x += 100.0 * dt; // Movimiento independiente del framerate
///         }
///     }
///
///     fn render(&mut self, drawer: &mut dyn Drawer) {
///         drawer.clear([0, 0, 0, 255]);
///         drawer.draw_rect(self.player_x, 50.0, 32.0, 32.0, [255, 0, 0, 255]);
///     }
///
///     fn drawer_manager(&mut self) -> &mut DrawerManager {
///         &mut self.drawer_mgr
///     }
/// }
/// ```
pub trait State {
    /// Se ejecuta en cada frame para actualizar la lógica interna del estado antes de la fase de renderizado.
    ///
    /// Debe utilizarse para procesar la entrada del usuario, actualizar físicas, posiciones de entidades,
    /// colisiones y temporizadores.
    ///
    /// # Parámetros
    /// - `dt` (*Delta Time*): El tiempo transcurrido en segundos (o fracción de segundo) desde el frame anterior.
    ///   *Nota de diseño*: Multiplicar la velocidad de movimiento o físicas por `dt` garantiza que la velocidad
    ///   del juego sea constante independientemente del número de FPS de la máquina.
    /// - `keyboard`: Referencia al gestor de teclado ([`KeyboardManager`]), permitiendo consultar el estado
    ///   de teclas presionadas, liberadas o mantenidas en el frame actual.
    fn update(&mut self, dt: f32, keyboard: &KeyboardManager);

    /// Se ejecuta en la fase de dibujo para emitir las órdenes de renderizado de la escena.
    ///
    /// Recibe una referencia mutable trait object (`&mut dyn Drawer`) que abstrae el backend
    /// activo (CPU o GPU), permitiendo que el estado dibuje sin preocuparse por la implementación subyacente.
    ///
    /// # Parámetros
    /// - `drawer`: Trait object que expone la API unificada de dibujo vectorial 2D (`draw_line`, `draw_rect`, etc.).
    fn render(&mut self, drawer: &mut dyn Drawer);

    /// Proporciona acceso mutable al [`DrawerManager`] interno asociado a este estado.
    ///
    /// Permite que el motor principal (*Engine/Game Loop*) pueda:
    /// - Consultar o alternar la preferencia de backend (`ForceCpu`, `ForceGpu`, `Auto`).
    /// - Preparar los buffers de renderizado según el backend activo en cada ciclo de frame.
    /// - Invocar métodos específicos de la GPU (como `begin_frame` o `render`) directamente.
    fn drawer_manager(&mut self) -> &mut DrawerManager;

    /// Se invoca automátiamente en el evento `resumed` tan pronto como la ventana nativa está creada.
    /// Permite al estado inicializar pipelines gráficos y pasar el `GpuDrawer` al `DrawerManager`.
    fn init_gpu(
        &mut self,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface: Arc<wgpu::Surface<'static>>,
        config: wgpu::SurfaceConfiguration,
    ) {
    }
}
