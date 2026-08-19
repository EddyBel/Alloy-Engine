use crate::core::camera3D::Camera3D;
use crate::core::drawer::Drawer;
use crate::core::models::mesh::Mesh;

// ==========================================
// ESTRUCTURA VEC3 (VECTOR Y PUNTO 3D)
// ==========================================
/// Representa una posición o vector en el espacio tridimensional $(X, Y, Z)$.
#[derive(Debug, Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Rotación simple en el eje X (Pitch)
    pub fn rotate_x(&self, angle: f32) -> Self {
        let rad = angle.to_radians();
        let cos = rad.cos();
        let sin = rad.sin();
        Self {
            x: self.x,
            y: self.y * cos - self.z * sin,
            z: self.y * sin + self.z * cos,
        }
    }

    /// Rotación simple en el eje Y (Yaw)
    pub fn rotate_y(&self, angle: f32) -> Self {
        let rad = angle.to_radians();
        let cos = rad.cos();
        let sin = rad.sin();
        Self {
            x: self.x * cos + self.z * sin,
            y: self.y,
            z: -self.x * sin + self.z * cos,
        }
    }

    /// Rotación simple en el eje Z (Roll)
    pub fn rotate_z(&self, angle: f32) -> Self {
        let rad = angle.to_radians();
        let cos = rad.cos();
        let sin = rad.sin();
        Self {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
            z: self.z,
        }
    }

    /// Cálculo del producto vectorial (Cross Product) para obtener normales de caras.
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Resta de vectores (P2 - P1)
    pub fn sub(&self, other: &Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

// ==========================================
// ESTRUCTURA DRAWER3D (MOTOR DE PROYECCIÓN 3D)
// ==========================================
/// Proyecta primitivas tridimensionales hacia un `Drawer` 2D existente.
pub struct Drawer3D<'a, 'b, 'c> {
    drawer: &'a mut Drawer<'b>,
    camera: &'c Camera3D, // Referencia inmutable a la cámara activa
}

impl<'a, 'b, 'c> Drawer3D<'a, 'b, 'c> {
    /// Crea un nuevo renderizador 3D envolviendo a `Drawer`.
    ///
    /// # Parámetros
    /// - `drawer`: Instancia activa del manipulador de buffer 2D.
    /// - `fov`: Factor de escala de la perspectiva (ej. 300.0 a 500.0 según el tamaño de ventana).
    /// Instancia un renderizador 3D vinculado a una cámara.
    pub fn new(drawer: &'a mut Drawer<'b>, camera: &'c Camera3D) -> Self {
        Self { drawer, camera }
    }

    // ==========================================
    // PROYECCIÓN PERSPECTIVA
    // ==========================================
    /// Convierte un punto 3D $(X, Y, Z)$ a coordenadas cartesianas 2D $(X_{screen}, Y_{screen})$.
    ///
    /// Retorna `None` si el punto está detrás o demasiado cerca de la cámara ($Z \le 0.1$).
    /// Transforma un punto de Espacio Mundo -> Espacio Cámara -> Pantalla 2D
    pub fn project(&self, point: Vec3) -> Option<(i32, i32)> {
        // 1. Traslación respecto a la cámara (Mundo -> Vista local)
        let translated = Vec3::new(
            point.x - self.camera.position.x,
            point.y - self.camera.position.y,
            point.z - self.camera.position.z,
        );

        // 2. Rotación inversa de la cámara (Yaw y Pitch)
        let rot_y = translated.rotate_y(-self.camera.yaw);
        let view_space = rot_y.rotate_x(-self.camera.pitch);

        // 3. Plane Clipping cercano (Near Plane)
        if view_space.z <= 0.1 {
            return None;
        }

        // 4. Proyección Perspectiva con el FOV de la cámara
        let center_x = self.drawer.width() as f32 / 2.0;
        let center_y = self.drawer.height() as f32 / 2.0;

        let screen_x = ((view_space.x / view_space.z) * self.camera.fov + center_x) as i32;
        let screen_y = ((-view_space.y / view_space.z) * self.camera.fov + center_y) as i32;

        Some((screen_x, screen_y))
    }

    // ==========================================
    // PRIMITIVA 3D: LÍNEA EN EL ESPACIO
    // ==========================================
    /// Dibuja un segmento de línea entre dos puntos 3D.
    pub fn draw_line_3d(&mut self, p0: Vec3, p1: Vec3, color: [u8; 4]) {
        if let (Some((x0, y0)), Some((x1, y1))) = (self.project(p0), self.project(p1)) {
            self.drawer.draw_line(x0, y0, x1, y1, color);
        }
    }

    // ==========================================
    // PRIMITIVA 3D: TRIÁNGULO EN WIREFRAME
    // ==========================================
    /// Traza los tres bordes de un triángulo ubicado en el espacio 3D.
    pub fn draw_triangle_3d(&mut self, v0: Vec3, v1: Vec3, v2: Vec3, color: [u8; 4]) {
        self.draw_line_3d(v0, v1, color);
        self.draw_line_3d(v1, v2, color);
        self.draw_line_3d(v2, v0, color);
    }

    // ==========================================
    // PRIMITIVA 3D: TRIÁNGULO RELLENO CON CULLING Y ILUMINACIÓN
    // ==========================================
    /// Renderiza un triángulo 3D relleno evaluando **Backface Culling** (descarte de caras traseras).
    pub fn draw_filled_triangle_3d(&mut self, v0: Vec3, v1: Vec3, v2: Vec3, color: [u8; 4]) {
        // Cálculo del Vector Normal de la cara (v1 - v0) x (v2 - v0)
        let line1 = v1.sub(&v0);
        let line2 = v2.sub(&v0);
        let normal = line1.cross(&line2);

        // Vector de visión desde la cara hacia la cámara en (0, 0, 0)
        let view_dir = Vec3::new(-v0.x, -v0.y, -v0.z);

        // Dot Product (Producto Punto): Determina si la cara apunta hacia la cámara
        let dot = normal.x * view_dir.x + normal.y * view_dir.y + normal.z * view_dir.z;

        // Si dot > 0, la cara apunta hacia adelante y debe renderizarse
        if dot > 0.0 {
            if let (Some((x0, y0)), Some((x1, y1)), Some((x2, y2))) =
                (self.project(v0), self.project(v1), self.project(v2))
            {
                self.drawer
                    .draw_filled_triangle(x0, y0, x1, y1, x2, y2, color);
            }
        }
    }

    // ==========================================
    // ELEMENTO GEOMÉTRICO 3D: CUBO / PRISMA
    // ==========================================
    /// Renderiza un cubo de tamaño uniforme con traslación y rotaciones especificadas en grados.
    pub fn draw_cube(
        &mut self,
        position: Vec3,
        size: f32,
        rotation: Vec3,
        color: [u8; 4],
        wireframe: bool,
    ) {
        let half = size / 2.0;

        // Vértices locales del cubo en su propio centro de masa (Origen 0,0,0)
        let raw_vertices = [
            Vec3::new(-half, -half, -half), // 0: Izq-Abajo-Atrás
            Vec3::new(half, -half, -half),  // 1: Der-Abajo-Atrás
            Vec3::new(half, half, -half),   // 2: Der-Arriba-Atrás
            Vec3::new(-half, half, -half),  // 3: Izq-Arriba-Atrás
            Vec3::new(-half, -half, half),  // 4: Izq-Abajo-Frente
            Vec3::new(half, -half, half),   // 5: Der-Abajo-Frente
            Vec3::new(half, half, half),    // 6: Der-Arriba-Frente
            Vec3::new(-half, half, half),   // 7: Izq-Arriba-Frente
        ];

        // Aplica transformaciones de Rotación y Traslación
        let mut transformed = Vec::with_capacity(8);
        for v in raw_vertices {
            let rot = v
                .rotate_x(rotation.x)
                .rotate_y(rotation.y)
                .rotate_z(rotation.z);

            // Mueve el punto a la posición deseada en el espacio mundo
            transformed.push(Vec3::new(
                rot.x + position.x,
                rot.y + position.y,
                rot.z + position.z,
            ));
        }

        // Definición de las 12 caras triangulares compuestas por los 8 vértices
        let indices = [
            // Cara Frontal
            (4, 5, 6),
            (4, 6, 7),
            // Cara Trasera
            (1, 0, 3),
            (1, 3, 2),
            // Cara Izquierda
            (0, 4, 7),
            (0, 7, 3),
            // Cara Derecha
            (5, 1, 2),
            (5, 2, 6),
            // Cara Superior
            (7, 6, 2),
            (7, 2, 3),
            // Cara Inferior
            (0, 1, 5),
            (0, 5, 4),
        ];

        // Renderizado del cubo
        for (i0, i1, i2) in indices {
            let v0 = transformed[i0];
            let v1 = transformed[i1];
            let v2 = transformed[i2];

            if wireframe {
                self.draw_triangle_3d(v0, v1, v2, color);
            } else {
                self.draw_filled_triangle_3d(v0, v1, v2, color);
            }
        }
    }

    // ==========================================
    // MÉTODO: DRAW_MULTICOLOR_CUBE (CUBO MULTICOLOR)
    // ==========================================
    /// Renderiza un cubo donde cada una de sus 6 caras tiene un color RGBA distinto.
    ///
    /// # Parámetros
    /// - `position`: Ubicación del centro del cubo en el espacio mundo.
    /// - `size`: Dimensión de las aristas del cubo.
    /// - `rotation`: Ángulos de rotación en grados `Vec3::new(pitch, yaw, roll)`.
    /// - `face_colors`: Arreglo de 6 colores RGBA `[[u8; 4]; 6]` para las caras:
    ///   `[Frontal, Trasera, Izquierda, Derecha, Superior, Inferior]`.
    pub fn draw_multicolor_cube(
        &mut self,
        position: Vec3,
        size: f32,
        rotation: Vec3,
        face_colors: [[u8; 4]; 6],
    ) {
        let half = size / 2.0;

        // Vértices locales del cubo respecto a su origen (0,0,0)
        let raw_vertices = [
            Vec3::new(-half, -half, -half), // 0
            Vec3::new(half, -half, -half),  // 1
            Vec3::new(half, half, -half),   // 2
            Vec3::new(-half, half, -half),  // 3
            Vec3::new(-half, -half, half),  // 4
            Vec3::new(half, -half, half),   // 5
            Vec3::new(half, half, half),    // 6
            Vec3::new(-half, half, half),   // 7
        ];

        // Transformar vértices (Rotación local -> Traslación mundo)
        let mut transformed = Vec::with_capacity(8);
        for v in raw_vertices {
            let rot = v
                .rotate_x(rotation.x)
                .rotate_y(rotation.y)
                .rotate_z(rotation.z);

            transformed.push(Vec3::new(
                rot.x + position.x,
                rot.y + position.y,
                rot.z + position.z,
            ));
        }

        // Definición de caras cuadradas compuestas por 2 triángulos cada una.
        // Asignamos un índice de color (0 a 5) a cada grupo de 2 triángulos.
        let faces = [
            // (Triángulo 1, Triángulo 2, Índice de Color)
            ((4, 5, 6), (4, 6, 7), 0), // Frontal
            ((1, 0, 3), (1, 3, 2), 1), // Trasera
            ((0, 4, 7), (0, 7, 3), 2), // Izquierda
            ((5, 1, 2), (5, 2, 6), 3), // Derecha
            ((7, 6, 2), (7, 2, 3), 4), // Superior
            ((0, 1, 5), (0, 5, 4), 5), // Inferior
        ];

        // Renderizado de las 6 caras con su respectivo color
        for (t1, t2, color_idx) in faces {
            let color = face_colors[color_idx];

            // Triángulo 1 de la cara
            self.draw_filled_triangle_3d(
                transformed[t1.0],
                transformed[t1.1],
                transformed[t1.2],
                color,
            );

            // Triángulo 2 de la cara
            self.draw_filled_triangle_3d(
                transformed[t2.0],
                transformed[t2.1],
                transformed[t2.2],
                color,
            );
        }
    }

    /// Renderiza una malla 3D aplicándole posición (translation) y rotación
    /// Renderiza una malla 3D aplicándole posición (traslación) y rotación
    pub fn draw_mesh(&mut self, mesh: &Mesh, position: Vec3, rotation: Vec3, wireframe: bool) {
        for triangle in &mesh.triangles {
            // 1. Aplicar rotación a los vértices usando los métodos de Vec3
            let r0 = triangle.v0.rotate_x(rotation.x).rotate_y(rotation.y).rotate_z(rotation.z);
            let r1 = triangle.v1.rotate_x(rotation.x).rotate_y(rotation.y).rotate_z(rotation.z);
            let r2 = triangle.v2.rotate_x(rotation.x).rotate_y(rotation.y).rotate_z(rotation.z);

            // 2. Aplicar traslación al espacio mundo
            let t0 = Vec3::new(r0.x + position.x, r0.y + position.y, r0.z + position.z);
            let t1 = Vec3::new(r1.x + position.x, r1.y + position.y, r1.z + position.z);
            let t2 = Vec3::new(r2.x + position.x, r2.y + position.y, r2.z + position.z);

            // 3. Renderizar en wireframe o relleno usando tus métodos existentes
            if wireframe {
                self.draw_triangle_3d(t0, t1, t2, triangle.color);
            } else {
                self.draw_filled_triangle_3d(t0, t1, t2, triangle.color);
            }
        }
    }
}
