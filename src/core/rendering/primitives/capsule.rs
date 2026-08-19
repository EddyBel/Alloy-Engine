use crate::core::camera::camera_3d::Camera3D;
use crate::core::camera::vec3::Vec3;
use crate::core::rendering::drawers::drawer_backend::Drawer;
use std::f32::consts::PI;

/// Representa una cara triangular de la malla de la cápsula.
#[derive(Clone, Copy)]
pub struct TriangleIndices {
    pub a: usize,
    pub b: usize,
    pub c: usize,
}

/// Datos de la malla de una cápsula.
pub struct CapsuleMesh {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<TriangleIndices>,
}

pub struct Capsule3D;

impl Capsule3D {
    /// Genera la malla procedural de una cápsula correctamente alineada en el eje Y.
    pub fn create_mesh(
        radius: f32,
        cylinder_height: f32,
        lat_rings: usize,
        lon_segments: usize,
    ) -> CapsuleMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let half_h = cylinder_height * 0.5;

        // 1. Semiesfera Superior: va desde la punta (+Y) hasta la base del domo (+half_h)
        for i in 0..=lat_rings {
            // phi va de PI/2 (punta superior) a 0 (base del domo)
            let phi = (PI * 0.5) * (1.0 - (i as f32 / lat_rings as f32));
            let y_offset = half_h + radius * phi.sin();
            let ring_r = radius * phi.cos();

            for j in 0..=lon_segments {
                let theta = 2.0 * PI * (j as f32 / lon_segments as f32);
                let x = ring_r * theta.cos();
                let z = ring_r * theta.sin();
                vertices.push(Vec3::new(x, y_offset, z));
            }
        }

        // 2. Semiesfera Inferior: va desde la base del domo (-half_h) hasta la punta inferior (-Y)
        for i in 0..=lat_rings {
            // phi va de 0 (base del domo inferior) a -PI/2 (punta inferior)
            let phi = -(PI * 0.5) * (i as f32 / lat_rings as f32);
            let y_offset = -half_h + radius * phi.sin();
            let ring_r = radius * phi.cos();

            for j in 0..=lon_segments {
                let theta = 2.0 * PI * (j as f32 / lon_segments as f32);
                let x = ring_r * theta.cos();
                let z = ring_r * theta.sin();
                vertices.push(Vec3::new(x, y_offset, z));
            }
        }

        let stride = lon_segments + 1;
        let total_rings = (lat_rings + 1) * 2;

        // 3. Generar la malla continua de cuadriláteros/triángulos a lo largo de todos los anillos
        for i in 0..(total_rings - 1) {
            for j in 0..lon_segments {
                let curr = i * stride + j;
                let next = curr + stride;

                indices.push(TriangleIndices {
                    a: curr,
                    b: next,
                    c: curr + 1,
                });
                indices.push(TriangleIndices {
                    a: curr + 1,
                    b: next,
                    c: next + 1,
                });
            }
        }

        CapsuleMesh { vertices, indices }
    }

    /// Renderiza la cápsula proyectando los triángulos en pantalla.
    pub fn draw(
        drawer: &mut dyn Drawer,
        mesh: &CapsuleMesh,
        color: [u8; 4],
        camera: &Camera3D,
        screen_width: f32,
        screen_height: f32,
    ) {
        let mut cam_vertices: Vec<Vec3> = Vec::with_capacity(mesh.vertices.len());
        let mut projected: Vec<Option<(f32, f32)>> = Vec::with_capacity(mesh.vertices.len());

        for v in &mesh.vertices {
            let cam_v = camera.world_to_camera_space(*v);
            cam_vertices.push(cam_v);
            projected.push(camera.project_to_screen(cam_v, screen_width, screen_height));
        }

        // Ordenar triángulos por profundidad Z promedio (Algoritmo del Pintor)
        let mut sorted_triangles: Vec<(&TriangleIndices, f32)> = mesh
            .indices
            .iter()
            .filter_map(|tri| {
                let _p0 = projected[tri.a]?;
                let _p1 = projected[tri.b]?;
                let _p2 = projected[tri.c]?;

                let z_avg =
                    (cam_vertices[tri.a].z + cam_vertices[tri.b].z + cam_vertices[tri.c].z) / 3.0;

                Some((tri, z_avg))
            })
            .collect();

        sorted_triangles.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Renderizado con sombreado plano (Flat Shading)
        for (tri, _) in sorted_triangles {
            let p0 = projected[tri.a].unwrap();
            let p1 = projected[tri.b].unwrap();
            let p2 = projected[tri.c].unwrap();

            let v0 = mesh.vertices[tri.a];
            let v1 = mesh.vertices[tri.b];
            let v2 = mesh.vertices[tri.c];

            let edge1 = Vec3::new(v1.x - v0.x, v1.y - v0.y, v1.z - v0.z);
            let edge2 = Vec3::new(v2.x - v0.x, v2.y - v0.y, v2.z - v0.z);

            let nx = edge1.y * edge2.z - edge1.z * edge2.y;
            let ny = edge1.z * edge2.x - edge1.x * edge2.z;
            let nz = edge1.x * edge2.y - edge1.y * edge2.x;

            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            let shade = if len > 0.0 {
                let factor = ((nx / len) * 0.4 + (ny / len) * 0.7 + (nz / len) * 0.3).max(0.25);
                factor.min(1.0)
            } else {
                1.0
            };

            let lit_color = [
                (color[0] as f32 * shade) as u8,
                (color[1] as f32 * shade) as u8,
                (color[2] as f32 * shade) as u8,
                color[3],
            ];

            drawer.draw_filled_triangle(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, lit_color);
        }
    }
}