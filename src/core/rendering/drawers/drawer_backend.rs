use crate::core::rendering::drawers::cpu_drawers::CPUDrawer;

/// Identificador explícito del tipo de backend de renderizado activo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    /// Renderizado por software ejecutado en la CPU.
    Cpu,
    /// Renderizado acelerado por hardware mediante la GPU.
    Gpu,
}

/// Contrato e interfaz unificada para los backends de renderizado vectorial 2D.
///
/// El trait `DrawerBackend` expone una API abstracta agnóstica al hardware de renderizado.
/// Permite que la lógica principal del juego o aplicación emita primitivas de dibujo
/// (puntos, líneas, rectángulos, triángulos) en coordenadas continuas de punto flotante (`f32`)
/// sin importar si el renderizado final se realiza por software ([`CPUDrawer`]) o acelerado
/// por hardware ([`GpuDrawer`](crate::core::drawers::gpu_drawer::GpuDrawer)).
///
/// # Formato de Color
/// Todas las funciones reciben los colores en formato **RGBA de 8 bits por canal** (`[u8; 4]`),
/// donde:
/// - `color[0]` = Rojo (0 a 255)
/// - `color[1]` = Verde (0 a 255)
/// - `color[2]` = Azul (0 a 255)
/// - `color[3]` = Alfa / Transparencia (0 a 255, donde 255 es opaco)
///
/// # Sistema de Coordenadas
/// - El origen `(0.0, 0.0)` se ubica en la esquina **superior izquierda** de la pantalla/buffer.
/// - El eje **X** crece hacia la derecha.
/// - El eje **Y** crece hacia abajo.
pub trait Drawer {
    /// Devuelve el tipo de backend de renderizado activo ([`RenderBackend::Cpu`] o [`RenderBackend::Gpu`]).
    fn backend_type(&self) -> RenderBackend;

    /// Indica si el backend actual está utilizando la GPU.
    fn is_gpu(&self) -> bool {
        self.backend_type() == RenderBackend::Gpu
    }

    /// Rellena o limpia todo el buffer de renderizado con un color sólido especificado.
    ///
    /// # Parámetros
    /// - `color`: Color en formato RGBA `[r, g, b, a]`.
    fn clear(&mut self, color: [u8; 4]);

    /// Dibuja un único píxel en la posición bidimensional dada.
    ///
    /// # Parámetros
    /// - `x`: Posición horizontal en coordenadas de pantalla.
    /// - `y`: Posición vertical en coordenadas de pantalla.
    /// - `color`: Color RGBA del píxel.
    fn draw_pixel(&mut self, x: f32, y: f32, color: [u8; 4]);

    /// Dibuja una línea de un píxel de grosor entre dos puntos.
    ///
    /// # Parámetros
    /// - `x0`, `y0`: Coordenadas del punto inicial del segmento.
    /// - `x1`, `y1`: Coordenadas del punto final del segmento.
    /// - `color`: Color RGBA de la línea.
    fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: [u8; 4]);
    /// Dibuja una línea 3D con profundidad por vértice (z0, z1).
    fn draw_line_3d(&mut self, x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, color: [u8; 4]) {}

    /// Dibuja un rectángulo relleno alineado con los ejes (*AABB*).
    ///
    /// # Parámetros
    /// - `x`, `y`: Coordenadas de la esquina superior izquierda.
    /// - `w`: Ancho (*width*) del rectángulo.
    /// - `h`: Alto (*height*) del rectángulo.
    /// - `color`: Color RGBA de relleno.
    fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]);

    /// Dibuja únicamente el contorno (tres líneas) de un triángulo definido por tres vértices.
    ///
    /// # Parámetros
    /// - `x0`, `y0`: Coordenadas del primer vértice.
    /// - `x1`, `y1`: Coordenadas del segundo vértice.
    /// - `x2`, `y2`: Coordenadas del tercer vértice.
    /// - `color`: Color RGBA de los bordes del triángulo.
    fn draw_triangle(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32, color: [u8; 4]);

    /// Dibuja un triángulo sólido completamente relleno definido por tres vértices.
    ///
    /// # Parámetros
    /// - `x0`, `y0`: Coordenadas del primer vértice.
    /// - `x1`, `y1`: Coordenadas del segundo vértice.
    /// - `x2`, `y2`: Coordenadas del tercer vértice.
    /// - `color`: Color RGBA de relleno.
    fn draw_filled_triangle(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32, color: [u8; 4]);
    /// Dibuja un triángulo 3D proporcionando profundidad por vértice (z0,z1,z2).
    fn draw_filled_triangle_3d(&mut self, x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32, color: [u8; 4]) {}

    /// Habilita o deshabilita el uso del Z-buffer en el backend.
    /// - En `Cpu` activa un buffer de profundidad en memoria.
    /// - En `Gpu` activa la lógica de adjuntar/limpiar la textura de profundidad (si está soportado).
    fn enable_depth(&mut self, _enabled: bool) {}

    /// Limpia/ reinicia el Z-buffer interno (si existe en el backend).
    fn clear_depth_buffer(&mut self) {}

    /// Establece la profundidad (depth) que se usará para las primitivas siguientes.
    /// En CPUDrawer será el valor escalar que se comparará contra el depth buffer por píxel.
    fn set_current_depth(&mut self, _depth: f32) {}
}

// =========================================================================
// IMPLEMENTACIÓN DEL TRAIT PARA RENDERIZADO POR SOFTWARE (CPU)
// =========================================================================

/// Adaptador para conectar la implementación de `CPUDrawer` con la API unificada `DrawerBackend`.
///
/// Dado que `CPUDrawer` trabaja internamente con enteros (`i32` para posiciones discretas
/// de píxeles y `u32` para dimensiones en memoria de rasterizado), esta implementación
/// realiza la conversión explícita (*casting* mediante `as`) desde las coordenadas unificadas de punto flotante `f32`.
impl<'a> Drawer for CPUDrawer<'a> {
    fn backend_type(&self) -> RenderBackend {
        RenderBackend::Cpu
    }

    /// Delega directamente la limpieza del buffer por software al método nativo de `CPUDrawer`.
    fn clear(&mut self, color: [u8; 4]) {
        self.clear(color);
    }

    /// Trunca las coordenadas flotantes `(f32)` a enteros discretos `(i32)` y dibuja el píxel en el raster.
    fn draw_pixel(&mut self, x: f32, y: f32, color: [u8; 4]) {
        self.draw_pixel(x as i32, y as i32, color);
    }

    /// Trunca los puntos inicial y final a enteros e invoca el algoritmo de rasterizado de líneas (p. ej., Bresenham).
    fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: [u8; 4]) {
        self.draw_line(x0 as i32, y0 as i32, x1 as i32, y1 as i32, color);
    }

    /// Convierte la posición a `i32` y las dimensiones `(w, h)` a enteros sin signo `u32` para dibujar el rectángulo en el buffer de píxeles.
    fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
        self.draw_rect(x as i32, y as i32, w as u32, h as u32, color);
    }

    /// Trunca las coordenadas de los tres vértices a `i32` y dibuja las tres líneas del contorno.
    fn draw_triangle(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32, color: [u8; 4]) {
        self.draw_triangle(x0 as i32, y0 as i32, x1 as i32, y1 as i32, x2 as i32, y2 as i32, color);
    }

    /// Trunca las coordenadas de los vértices a `i32` y ejecuta el algoritmo de rasterizado e interpolación de triángulos rellenos por software.
    fn draw_filled_triangle(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32, color: [u8; 4]) {
        self.draw_filled_triangle(x0 as i32, y0 as i32, x1 as i32, y1 as i32, x2 as i32, y2 as i32, color);
    }

    fn draw_line_3d(&mut self, x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, color: [u8; 4]) {
        // Fallback por software: usamos la profundidad promedio de los extremos como flat depth
        let avg = (z0 + z1) * 0.5;
        // Establecer la profundidad actual en el CPUDrawer y usar el trazado 2D existente
        self.set_current_depth(avg);
        self.draw_line(x0 as i32, y0 as i32, x1 as i32, y1 as i32, color);
    }

    fn draw_filled_triangle_3d(&mut self, x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32, color: [u8; 4]) {
        // Fallback plano: usamos la profundidad promedio de los tres vértices
        let avg = (z0 + z1 + z2) / 3.0;
        self.set_current_depth(avg);
        self.draw_filled_triangle(x0 as i32, y0 as i32, x1 as i32, y1 as i32, x2 as i32, y2 as i32, color);
    }

    fn enable_depth(&mut self, enabled: bool) {
        self.enable_depth_mode(enabled);
    }

    fn clear_depth_buffer(&mut self) {
        self.clear_depth_buffer();
    }

    fn set_current_depth(&mut self, depth: f32) {
        self.set_current_depth_value(depth);
    }
}
