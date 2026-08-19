// ==========================================
// ESTRUCTURA DRAWER (MANIPULADOR DE BUFFER 2D)
// ==========================================
/// Envoltorio temporal para realizar operaciones de dibujo en 2D sobre un buffer lineal de píxeles.
///
/// Posee un *lifetime* (`'a`) asignado al buffer de bytes mutable (`&'a mut [u8]`) para
/// garantizar al compilador de Rust que las referencias son válidas mientras dure el frame.
pub struct CPUDrawer<'a> {
    frame: &'a mut [u8], // Slice mutable donde reside la información RGBA de cada píxel en memoria.
    width: u32,          // Ancho total del lienzo en píxeles.
    height: u32,         // Alto total del lienzo en píxeles.
    // Z-buffer por software (opcional). Tiene la misma resolución que el framebuffer y
    // almacena la profundidad mínima (más cercana) registrada por píxel.
    depth_buffer: Option<Vec<f32>>,
    // Indica si el depth test está activo.
    depth_enabled: bool,
    // Profundidad actual usada por las primitivas que se dibujen a continuación.
    current_depth: f32,
}

impl<'a> CPUDrawer<'a> {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Instancia un nuevo `Drawer` asociándolo al buffer del frame actual.
    ///
    /// # Parámetros
    /// - `frame`: Slice de bytes mutable que representa el framebuffer (formato de 4 bytes por píxel: [R, G, B, A]).
    /// - `width` y `height`: Dimensiones del lienzo en píxeles.
    ///
    /// # Ejemplo de Uso
    /// ```rust
    /// let mut drawer = Drawer::new(pixels.frame_mut(), 640, 480);
    /// ```
    pub fn new(frame: &'a mut [u8], width: u32, height: u32) -> Self {
        Self {
            frame,
            width,
            height,
            depth_buffer: None,
            depth_enabled: false,
            current_depth: std::f32::INFINITY,
        }
    }

    /// Habilita o deshabilita el uso del Z-buffer por software.
    pub fn enable_depth_mode(&mut self, enabled: bool) {
        self.depth_enabled = enabled;
        if enabled && self.depth_buffer.is_none() {
            let size = (self.width * self.height) as usize;
            self.depth_buffer = Some(vec![std::f32::INFINITY; size]);
        }
    }

    /// Limpia el Z-buffer por software (establece +inf en todos los píxeles).
    pub fn clear_depth_buffer(&mut self) {
        if let Some(buf) = &mut self.depth_buffer {
            for v in buf.iter_mut() {
                *v = std::f32::INFINITY;
            }
        }
    }

    /// Establece la profundidad actual para las primitivas siguientes.
    pub fn set_current_depth_value(&mut self, depth: f32) {
        self.current_depth = depth;
    }

    // ==========================================
    // MÉTODO: CLEAR (LIMPIEZA DE PANTALLA)
    // ==========================================
    /// Rellena absolutamente todo el buffer con un color uniforme.
    ///
    /// ### ¿Cómo funciona?
    /// Divide el buffer de bytes en bloques de 4 bytes exactos mediante `chunks_exact_mut(4)`.
    /// Cada bloque representa un píxel (RGBA). Luego, usando `copy_from_slice`, reemplaza
    /// de manera eficiente cada bloque con el color indicado.
    ///
    /// # Parámetros
    /// - `color`: Arreglo de 4 bytes `[R, G, B, A]`.
    ///
    /// # Ejemplo de Uso
    /// ```rust
    /// // Limpia la pantalla con un color azul oscuro (R: 20, G: 20, B: 35, A: 255)
    /// drawer.clear([20, 20, 35, 255]);
    /// ```
    pub fn clear(&mut self, color: [u8; 4]) {
        for pixel in self.frame.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
        if let Some(buf) = &mut self.depth_buffer {
            for v in buf.iter_mut() {
                *v = std::f32::INFINITY;
            }
        }
    }

    // ==========================================
    // MÉTODO: DRAW_PIXEL (PRIMITIVA BÁSICA)
    // ==========================================
    /// Pinta un único píxel en las coordenadas (x, y) de la pantalla.
    ///
    /// ### ¿Cómo funciona?
    /// 1. **Clipping / Validaciones:** Verifica que (x, y) esté dentro de los límites visibles
    ///    `[0, width - 1]` y `[0, height - 1]`. Si está fuera, descarta la operación evitando pánicos.
    /// 2. **Cálculo de Offset 2D a 1D:** Transforma la coordenada bidimensional a un índice lineal en memoria:
    ///    `índice = (y * width + x) * 4`
    /// 3. Asigna individualmente los canales de color Rojo, Verde, Azul y Alfa.
    ///
    /// # Parámetros
    /// - `x`, `y`: Posición cartesiana en el pantalla (origen 0,0 en la esquina superior izquierda).
    /// - `color`: Arreglo de 4 bytes `[R, G, B, A]`.
    ///
    /// # Ejemplo de Uso
    /// ```rust
    /// // Dibuja un píxel rojo en la coordenada (100, 150)
    /// drawer.draw_pixel(100, 150, [255, 0, 0, 255]);
    /// ```
    pub fn draw_pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        // Validar que esté dentro de los límites de la pantalla (evita out of bounds)
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }

        let pix_idx_linear = (y as u32 * self.width + x as u32) as usize;
        let pixel_index = pix_idx_linear * 4;

        // Si hay depth test activo, comparar current_depth contra el valor almacenado
        if self.depth_enabled {
            if let Some(buf) = &mut self.depth_buffer {
                let stored = buf[pix_idx_linear];
                // Convención: valores menores = más cercanos. Inicializamos con +inf.
                if self.current_depth >= stored {
                    return; // No escribir: objeto más lejos o igual
                }
                // Escribir color y actualizar depth buffer
                buf[pix_idx_linear] = self.current_depth;
            }
        }

        self.frame[pixel_index] = color[0]; // Rojo (R)
        self.frame[pixel_index + 1] = color[1]; // Verde (G)
        self.frame[pixel_index + 2] = color[2]; // Azul (B)
        self.frame[pixel_index + 3] = color[3]; // Transparencia/Alfa (A)
    }

    // ==========================================
    // MÉTODO: DRAW_RECT (RECTÁNGULO RELLENO)
    // ==========================================
    /// Rellena una región rectangular en pantalla definiendo su posición inicial y tamaño.
    ///
    /// ### ¿Cómo funciona?
    /// Itera mediante dos bucles anidados desde las filas `[y, y + h]` hasta las columnas `[x, x + w]`.
    /// En cada celda de la rejilla invoca a `draw_pixel`, lo que asegura la reutilización de las
    /// verificaciones de bordes.
    ///
    /// # Parámetros
    /// - `x`, `y`: Esquina superior izquierda del rectángulo.
    /// - `w`, `h`: Ancho y alto del rectángulo en píxeles.
    /// - `color`: Arreglo RGBA `[R, G, B, A]`.
    ///
    /// # Ejemplo de Uso
    /// ```rust
    /// // Dibuja un rectángulo verde de 50x50 comenzando en (20, 20)
    /// drawer.draw_rect(20, 20, 50, 50, [0, 255, 0, 255]);
    /// ```
    pub fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: [u8; 4]) {
        for row in y..(y + h as i32) {
            for col in x..(x + w as i32) {
                self.draw_pixel(col, row, color);
            }
        }
    }

    // ==========================================
    // MÉTODO: DRAW_LINE (ALGORITMO DE BRESENHAM)
    // ==========================================
    /// Traza una línea recta contigua conectando un punto inicial (x0, y0) con uno final (x1, y1).
    ///
    /// ### ¿Cómo funciona?
    /// Utiliza el **Algoritmo de Líneas de Bresenham**, un método altamente eficiente que opera
    /// exclusivamente con suma y resta de números enteros (evitando divisiones y números flotantes).
    /// 1. `dx` y `dy`: Calculan las distancias absolutas en los ejes X e Y.
    /// 2. `sx` y `sy`: Determinan el sentido del avance (+1 o -1 en los ejes).
    /// 3. `err`: Mantiene el acumulador del margen de error de la recta para decidir en qué paso
    ///    avanzar horizontalmente, verticalmente o en diagonal.
    ///
    /// # Parámetros
    /// - `x0`, `y0`: Punto inicial de origen.
    /// - `x1`, `y1`: Punto final de destino.
    /// - `color`: Arreglo RGBA `[R, G, B, A]`.
    ///
    /// # Ejemplo de Uso
    /// ```rust
    /// // Traza una línea diagonal blanca desde (0,0) hasta (100,100)
    /// drawer.draw_line(0, 0, 100, 100, [255, 255, 255, 255]);
    /// ```
    pub fn draw_line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: [u8; 4]) {
        // Trazado ultrasarrápido de líneas horizontales (Scanlines del triángulo)
        if y0 == y1 {
            if y0 < 0 || y0 >= self.height as i32 {
                return;
            }
            let mut start_x = x0.min(x1).max(0);
            let mut end_x = x0.max(x1).min(self.width as i32 - 1);

            if start_x > end_x {
                return;
            }

            let row_offset = (y0 as usize) * (self.width as usize) * 4;
            for x in start_x..=end_x {
                let pixel_index = row_offset + (x as usize) * 4;
                self.frame[pixel_index..pixel_index + 4].copy_from_slice(&color);
            }
            return;
        }

        // Algoritmo de Bresenham estándar para líneas diagonales o verticales
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.draw_pixel(x0, y0, color);

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    // ==========================================
    // MÉTODO: DRAW_TRIANGLE (ARISTAS DE TRIÁNGULO)
    // ==========================================
    /// Dibuja el contorno (alámbrico) de un triángulo especificando la posición de sus 3 vértices.
    ///
    /// ### ¿Cómo funciona?
    /// Encadena tres llamadas secuenciales al método `draw_line`, trazando las tres aristas:
    /// 1. Del Vértice 0 al Vértice 1
    /// 2. Del Vértice 1 al Vértice 2
    /// 3. Del Vértice 2 al Vértice 0 (cerrando la figura)
    ///
    /// # Parámetros
    /// - `x0, y0`: Primer vértice (ej. cúspide del triángulo).
    /// - `x1, y1`: Segundo vértice (ej. esquina inferior izquierda).
    /// - `x2, y2`: Tercer vértice (ej. esquina inferior derecha).
    /// - `color`: Arreglo RGBA `[R, G, B, A]`.
    ///
    /// # Ejemplo de Uso
    /// ```rust
    /// // Dibuja un triángulo cian en el centro de la pantalla
    /// drawer.draw_triangle(
    ///     320, 100,  // Arriba
    ///     200, 300,  // Izquierda
    ///     440, 300,  // Derecha
    ///     [0, 255, 255, 255]
    /// );
    /// ```
    pub fn draw_triangle(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: [u8; 4],
    ) {
        self.draw_line(x0, y0, x1, y1, color);
        self.draw_line(x1, y1, x2, y2, color);
        self.draw_line(x2, y2, x0, y0, color);
    }

    // ==========================================
    // MÉTODO: DRAW_FILLED_TRIANGLE (TRIÁNGULO RELLENO)
    // ==========================================
    /// Rellena completamente el interior de un triángulo definido por 3 vértices.
    ///
    /// ### ¿Cómo funciona?
    /// Usa el **Algoritmo de Scanline por División de Base Plana**:
    /// 1. Ordena los vértices verticalmente $y0 \le y1 \le y2$.
    /// 2. Divide el triángulo en dos partes a la altura del vértice medio ($y1$).
    /// 3. Interpola $X_{inicio}$ y $X_{fin}$ para cada scanline horizontal ($Y$) y traza líneas de relleno.
    ///
    /// # Parámetros
    /// - `x0, y0`: Primer vértice.
    /// - `x1, y1`: Segundo vértice.
    /// - `x2, y2`: Tercer vértice.
    /// - `color`: Arreglo RGBA `[R, G, B, A]`.
    ///
    /// # Ejemplo de Uso
    /// ```rust
    /// drawer.draw_filled_triangle(
    ///     320, 100,
    ///     150, 350,
    ///     480, 400,
    ///     [255, 100, 50, 255]
    /// );
    /// ```
    pub fn draw_filled_triangle(
        &mut self,
        mut x0: i32,
        mut y0: i32,
        mut x1: i32,
        mut y1: i32,
        mut x2: i32,
        mut y2: i32,
        color: [u8; 4],
    ) {
        // 1. Ordenar los vértices por coordenada Y ascendente (y0 <= y1 <= y2)
        if y0 > y1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }
        if y0 > y2 {
            std::mem::swap(&mut x0, &mut x2);
            std::mem::swap(&mut y0, &mut y2);
        }
        if y1 > y2 {
            std::mem::swap(&mut x1, &mut x2);
            std::mem::swap(&mut y1, &mut y2);
        }

        // Caso límite: triángulo fuera de la pantalla verticalmente
        if y2 < 0 || y0 >= self.height as i32 || y0 == y2 {
            return;
        }

        // Interpolación segura utilizando enteros de 64 bits (i64) para prevenir overflow
        let interpolate_x = |y: i32, xa: i32, ya: i32, xb: i32, yb: i32| -> i32 {
            if ya == yb {
                return xa;
            }
            let xa64 = xa as i64;
            let xb64 = xb as i64;
            let ya64 = ya as i64;
            let yb64 = yb as i64;
            let y64 = y as i64;

            let result = xa64 + (xb64 - xa64) * (y64 - ya64) / (yb64 - ya64);
            result.clamp(i32::MIN as i64, i32::MAX as i64) as i32
        };

        // Recorte en Y (Screen Clipping) para no iterar por píxeles fuera de la pantalla
        let y_start1 = y0.max(0);
        let y_end1 = y1.min(self.height as i32 - 1);

        // 2. Parte superior del triángulo
        for y in y_start1..=y_end1 {
            let xa = interpolate_x(y, x0, y0, x2, y2);
            let xb = interpolate_x(y, x0, y0, x1, y1);
            self.draw_line(xa, y, xb, y, color);
        }

        let y_start2 = (y1 + 1).max(0);
        let y_end2 = y2.min(self.height as i32 - 1);

        // 3. Parte inferior del triángulo
        for y in y_start2..=y_end2 {
            let xa = interpolate_x(y, x0, y0, x2, y2);
            let xb = interpolate_x(y, x1, y1, x2, y2);
            self.draw_line(xa, y, xb, y, color);
        }
    }
}
