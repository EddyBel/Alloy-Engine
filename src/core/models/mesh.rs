// use crate::core::render3D::Vec3;
// use std::fs::File;
// use std::io::{BufRead, BufReader};
// use std::path::Path;

// /// Representa un triángulo 3D compuesto por 3 vértices y un color
// #[derive(Debug, Clone)]
// pub struct Triangle {
//     pub v0: Vec3,
//     pub v1: Vec3,
//     pub v2: Vec3,
//     pub color: [u8; 4],
// }

// /// Colección de triángulos que forman un modelo 3D
// #[derive(Debug, Clone)]
// pub struct Mesh {
//     pub triangles: Vec<Triangle>,
// }

// impl Mesh {
//     pub fn new() -> Self {
//         Self {
//             triangles: Vec::new(),
//         }
//     }

//     /// Carga un archivo `.obj` desde la ruta especificada
//     pub fn load_from_obj<P: AsRef<Path>>(path: P, color: [u8; 4]) -> Result<Self, String> {
//         let file = File::open(path).map_err(|e| format!("No se pudo abrir el archivo: {}", e))?;
//         let reader = BufReader::new(file);

//         let mut temp_vertices: Vec<Vec3> = Vec::new();
//         let mut triangles: Vec<Triangle> = Vec::new();

//         for line_result in reader.lines() {
//             let line = line_result.map_err(|e| format!("Error al leer línea: {}", e))?;
//             let line = line.trim();

//             // Ignorar líneas vacías o comentarios (#)
//             if line.is_empty() || line.starts_with('#') {
//                 continue;
//             }

//             let parts: Vec<&str> = line.split_whitespace().collect();
//             if parts.is_empty() {
//                 continue;
//             }

//             match parts[0] {
//                 // Parsear Vértice: "v x y z"
//                 "v" => {
//                     if parts.len() >= 4 {
//                         let x: f32 = parts[1].parse().map_err(|_| "Error al parsear X del vértice")?;
//                         let y: f32 = parts[2].parse().map_err(|_| "Error al parsear Y del vértice")?;
//                         let z: f32 = parts[3].parse().map_err(|_| "Error al parsear Z del vértice")?;
//                         temp_vertices.push(Vec3::new(x, y, z));
//                     }
//                 }
//                 // Parsear Cara: "f v1 v2 v3" o "f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3"
//                 "f" => {
//                     if parts.len() >= 4 {
//                         let mut face_indices: Vec<usize> = Vec::new();

//                         for part in &parts[1..] {
//                             // Extraer solo la primera parte si viene en formato "v/vt/vn"
//                             let index_str = part.split('/').next().unwrap_or("");
//                             if let Ok(idx) = index_str.parse::<usize>() {
//                                 // Los índices en .obj son basados en 1, los pasamos a base 0
//                                 if idx > 0 {
//                                     face_indices.push(idx - 1);
//                                 }
//                             }
//                         }

//                         // Triangulación básica (fan triangulation para polígonos de más de 3 vértices)
//                         for i in 1..(face_indices.len() - 1) {
//                             let idx0 = face_indices[0];
//                             let idx1 = face_indices[i];
//                             let idx2 = face_indices[i + 1];

//                             if idx0 < temp_vertices.len()
//                                 && idx1 < temp_vertices.len()
//                                 && idx2 < temp_vertices.len()
//                             {
//                                 triangles.push(Triangle {
//                                     v0: temp_vertices[idx0],
//                                     v1: temp_vertices[idx1],
//                                     v2: temp_vertices[idx2],
//                                     color,
//                                 });
//                             }
//                         }
//                     }
//                 }
//                 _ => (), // Ignorar otras etiquetas como vt, vn, s, usemtl, etc.
//             }
//         }

//         println!(
//             "Modelo cargado: {} vértices, {} triángulos",
//             temp_vertices.len(),
//             triangles.len()
//         );

//         Ok(Self { triangles })
//     }
// }


// // src/core/models/mesh.rs o en un nuevo modulo
// use bytemuck::{Pod, Zeroable};

// #[repr(C)]
// #[derive(Copy, Clone, Debug, Pod, Zeroable)]
// pub struct GPUVertex {
//     pub position: [f32; 3],
//     pub color: [f32; 4],
// }

// impl GPUVertex {
//     pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
//         wgpu::VertexBufferLayout {
//             array_stride: std::mem::size_of::<GPUVertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &[
//                 // Posición (Vec3 en Rust -> vec3<f32> en WGSL)
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 0,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 // Color (RGBA u8/f32 -> vec4<f32> en WGSL)
//                 wgpu::VertexAttribute {
//                     offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
//                     shader_location: 1,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//             ],
//         }
//     }
// }