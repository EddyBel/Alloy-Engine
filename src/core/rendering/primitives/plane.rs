use crate::core::camera::camera_3d::Camera3D;
use crate::core::camera::vec3::Vec3;
use crate::core::rendering::drawers::drawer_backend::Drawer;

/// Representa la primitiva geométrica de un plano 3D (cuadricula/suelo).
pub struct Plane3D;

impl Plane3D {
    /// Genera los 4 vértices de un plano en el eje XZ (piso horizontal) centrado en `(0, 0, 0)`.
    ///
    /// # Parámetros
    /// - `width`: Dimensión a lo largo del eje X.
    /// - `depth`: Dimensión a lo largo del eje Z.
    pub fn create_centered_vertices(width: f32, depth: f32) -> [Vec3; 4] {
        let hw = width * 0.5;
        let hd = depth * 0.5;

        [
            Vec3::new(-hw, 0.0, -hd), // 0: Izq-Atrás
            Vec3::new(hw, 0.0, -hd),  // 1: Der-Atrás
            Vec3::new(hw, 0.0, hd),   // 2: Der-Frente
            Vec3::new(-hw, 0.0, hd),  // 3: Izq-Frente
        ]
    }

    /// Renderiza un plano 3D proyectándolo a través del punto de vista de la `Camera3D`.
    pub fn draw(
        drawer: &mut dyn Drawer,
        vertices: &[Vec3; 4],
        color: [u8; 4],
        camera: &Camera3D,
        screen_width: f32,
        screen_height: f32,
    ) {
        // 1. Transformar los 4 vértices al Espacio de la Cámara y proyectarlos
        let mut projected: [Option<(f32, f32)>; 4] = [None; 4];

        for (i, v) in vertices.iter().enumerate() {
            let cam_v = camera.world_to_camera_space(*v);
            projected[i] = camera.project_to_screen(cam_v, screen_width, screen_height);
        }

        // 2. Comprobar que los 4 vértices están visibles al frente de la cámara
        let p0 = match projected[0] {
            Some(p) => p,
            None => return,
        };
        let p1 = match projected[1] {
            Some(p) => p,
            None => return,
        };
        let p2 = match projected[2] {
            Some(p) => p,
            None => return,
        };
        let p3 = match projected[3] {
            Some(p) => p,
            None => return,
        };

        // 3. Renderizar los dos triángulos que forman la cara cuadrilátera
        // Triángulo 1: (v0 -> v1 -> v2)
        drawer.draw_filled_triangle(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, color);
        // Triángulo 2: (v0 -> v2 -> v3)
        drawer.draw_filled_triangle(p0.0, p0.1, p2.0, p2.1, p3.0, p3.1, color);
    }
}