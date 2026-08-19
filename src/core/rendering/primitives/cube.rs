use crate::core::camera::camera_3d::Camera3D;
use crate::core::camera::vec3::Vec3;
use crate::core::rendering::drawers::drawer_backend::Drawer;

/// Define el esquema de color aplicado a las caras del cubo.
#[derive(Clone, Copy)]
pub enum CubeColors {
    /// Aplica un solo color RGBA `[r, g, b, a]` a todas las caras.
    Solid([u8; 4]),
    /// Define un color RGBA `[r, g, b, a]` específico para cada una de las 6 caras.
    PerFace([[u8; 4]; 6]),
}

/// Representa la primitiva geométrica de un cubo 3D.
pub struct Cube3D;

impl Cube3D {
    /// Genera los 8 vértices locales de un cubo centrado en el origen `(0, 0, 0)`.
    pub fn create_centered_vertices(width: f32, height: f32, depth: f32) -> [Vec3; 8] {
        let hw = width * 0.5;
        let hh = height * 0.5;
        let hd = depth * 0.5;

        [
            Vec3::new(-hw, -hh, -hd), // 0: Izq-Abajo-Frente
            Vec3::new(hw, -hh, -hd),  // 1: Der-Abajo-Frente
            Vec3::new(hw, hh, -hd),   // 2: Der-Arriba-Frente
            Vec3::new(-hw, hh, -hd),  // 3: Izq-Arriba-Frente
            Vec3::new(-hw, -hh, hd),  // 4: Izq-Abajo-Atrás
            Vec3::new(hw, -hh, hd),   // 5: Der-Abajo-Atrás
            Vec3::new(hw, hh, hd),    // 6: Der-Arriba-Atrás
            Vec3::new(-hw, hh, hd),   // 7: Izq-Arriba-Atrás
        ]
    }

    /// Renderiza un cubo proyectándolo a través del punto de vista de la `Camera3D`.
    pub fn draw(
        drawer: &mut dyn Drawer,
        vertices: &[Vec3; 8],
        colors: CubeColors,
        camera: &Camera3D,
        screen_width: f32,
        screen_height: f32,
    ) {
        let faces: [[usize; 4]; 6] = [
            [0, 1, 2, 3], // Frontal
            [5, 4, 7, 6], // Trasera
            [4, 0, 3, 7], // Izquierda
            [1, 5, 6, 2], // Derecha
            [4, 5, 1, 0], // Superior
            [3, 2, 6, 7], // Inferior
        ];

        let face_colors = match colors {
            CubeColors::Solid(col) => [col; 6],
            CubeColors::PerFace(cols) => cols,
        };

        // 1. Transformar los 8 vértices al Espacio de la Cámara y proyectarlos
        let mut cam_vertices: [Vec3; 8] = [Vec3::new(0.0, 0.0, 0.0); 8];
        let mut projected: [Option<(f32, f32)>; 8] = [None; 8];

        for (i, v) in vertices.iter().enumerate() {
            let cam_v = camera.world_to_camera_space(*v);
            cam_vertices[i] = cam_v;
            projected[i] = camera.project_to_screen(cam_v, screen_width, screen_height);
        }

        // 2. Ordenar las caras por profundidad relativa a la cámara (Algoritmo del Pintor)
        let mut sorted_faces: Vec<(usize, f32)> = faces
            .iter()
            .enumerate()
            .filter_map(|(idx, face_indices)| {
                // Verificar que los vértices estén al frente de la cámara
                let _p0 = projected[face_indices[0]]?;
                let _p1 = projected[face_indices[1]]?;
                let _p2 = projected[face_indices[2]]?;
                let _p3 = projected[face_indices[3]]?;

                // Calcular Z promedio en el Espacio de la Cámara
                let z_avg = (cam_vertices[face_indices[0]].z
                    + cam_vertices[face_indices[1]].z
                    + cam_vertices[face_indices[2]].z
                    + cam_vertices[face_indices[3]].z)
                    * 0.25;

                Some((idx, z_avg))
            })
            .collect();

        // Ordenar de mayor Z (más lejano) a menor Z (más cercano)
        sorted_faces.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 3. Renderizar triángulos rellenos
        for (face_idx, _) in sorted_faces {
            let indices = faces[face_idx];
            let color = face_colors[face_idx];

            let p0 = projected[indices[0]].unwrap();
            let p1 = projected[indices[1]].unwrap();
            let p2 = projected[indices[2]].unwrap();
            let p3 = projected[indices[3]].unwrap();

            drawer.draw_filled_triangle(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, color);
            drawer.draw_filled_triangle(p0.0, p0.1, p2.0, p2.1, p3.0, p3.1, color);
        }
    }
}