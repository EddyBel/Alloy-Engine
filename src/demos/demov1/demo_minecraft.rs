mod core;
mod test_gpu;

use std::sync::Arc;
use std::env;

use core::camera::camera_3d::Camera3D;
use core::camera::vec3::Vec3;
use core::controls::keyboard::{Key, KeyboardManager};
use core::rendering::drawers::drawer_backend::Drawer;
use core::rendering::drawers::drawer_manager::{BackendPreference, DrawerManager};
use core::rendering::scene::{SceneComposer, RenderOrderMode};
use core::rendering::drawers::gpu_drawer::GpuDrawer;
use core::rendering::primitives::cube::{Cube3D, CubeColors};
use core::window::state::State;
use core::window::window;

/// Estructura para representar un bloque individual en el mundo de tipo Minecraft
struct Block {
    x: f32,
    y: f32,
    z: f32,
    size: f32,
    color: [u8; 4],
}

pub struct DirectGpuApp {
    drawer_manager: DrawerManager,
    camera: Camera3D,
    blocks: Vec<(f32, [Vec3; 8], CubeColors)>, // Almacena la profundidad estimada, 8 vértices y colores de cada cubo del terreno
    render_mode: RenderOrderMode,
}

impl DirectGpuApp {
    pub fn new() -> Self {
        let drawer_manager = DrawerManager::new(None);

        // Posicionamos la cámara elevada para ver el paisaje de cubos
        let mut camera = Camera3D::new(Vec3::new(0.0, 150.0, -400.0), 400.0);
        camera.yaw = 0.0;
        camera.pitch = 0.35;

        // --- GENERACIÓN DE TERRENO TIPO MINECRAFT (MAPA DE RUIDO) ---
        let block_size = 30.0;
        let map_width = 16;  // Número de bloques en el eje X
        let map_depth = 16;  // Número de bloques en el eje Z
        
        let mut blocks = Vec::new();

        let half_w = (map_width as f32 * block_size) / 2.0;
        let half_d = (map_depth as f32 * block_size) / 2.0;

        for x in 0..map_width {
            for z in 0..map_depth {
                let world_x = (x as f32 * block_size) - half_w;
                let world_z = (z as f32 * block_size) - half_d;

                // Función de ruido procedural simple basada en senos y cosenos (sin dependencias externas)
                let nx = x as f32 * 0.2;
                let nz = z as f32 * 0.2;
                let height_noise = (nx.sin() * 2.0 + nz.cos() * 1.5 + (nx * nz).sin() * 0.5) + 3.0;
                let column_height = height_noise.abs().round() as i32 + 1; // Altura en bloques

                // Generamos una columna de bloques (desde el suelo hacia arriba)
                for y in 0..column_height {
                    let world_y = (y as f32 * block_size) - 60.0;

                    let mut cube_vertices = Cube3D::create_centered_vertices(block_size, block_size, block_size);
                    for v in &mut cube_vertices {
                        v.x += world_x;
                        v.y += world_y;
                        v.z += world_z;
                    }

                    // Definir colores según la capa (Césped arriba, tierra/piedra abajo)
                    let cube_colors = if y == column_height - 1 {
                        // Bloque de pasto superior (Verde)
                        CubeColors::PerFace([
                            [100, 200, 80, 255],
                            [80, 160, 60, 255],
                            [90, 180, 70, 255],
                            [110, 220, 90, 255],
                            [120, 230, 100, 255], // Tapa superior más clara
                            [70, 140, 50, 255],
                        ])
                    } else {
                        // Bloque de tierra (Marrón)
                        CubeColors::PerFace([
                            [139, 90, 43, 255],
                            [110, 70, 30, 255],
                            [120, 80, 35, 255],
                            [150, 100, 50, 255],
                            [139, 90, 43, 255],
                            [90, 55, 20, 255],
                        ])
                    };

                    blocks.push((0.0, cube_vertices, cube_colors));
                }
            }
        }

        Self {
            drawer_manager,
            camera,
            blocks,
            render_mode: RenderOrderMode::Painter3D,
        }
    }
}

impl State for DirectGpuApp {
    fn init_gpu(
        &mut self,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface: Arc<wgpu::Surface<'static>>,
        config: wgpu::SurfaceConfiguration,
    ) {
        let gpu_drawer = GpuDrawer::new(device, queue, surface, &config);
        self.drawer_manager = DrawerManager::new(Some(gpu_drawer));
        self.drawer_manager.set_preference(BackendPreference::ForceGpu);
    }

    fn update(&mut self, dt: f32, keyboard: &KeyboardManager) {
        let move_speed = 300.0 * dt;
        let rotate_speed = 1.5 * dt;

        if keyboard.is_key_down(Key::Left) {
            self.camera.yaw -= rotate_speed;
        }
        if keyboard.is_key_down(Key::Right) {
            self.camera.yaw += rotate_speed;
        }
        if keyboard.is_key_down(Key::Up) {
            self.camera.pitch += rotate_speed;
        }
        if keyboard.is_key_down(Key::Down) {
            self.camera.pitch -= rotate_speed;
        }

        let forward = Vec3::new(self.camera.yaw.sin(), 0.0, self.camera.yaw.cos());
        let right = Vec3::new(-self.camera.yaw.cos(), 0.0, self.camera.yaw.sin());

        if keyboard.is_key_down(Key::W) {
            self.camera.position.x += forward.x * move_speed;
            self.camera.position.z += forward.z * move_speed;
        }
        if keyboard.is_key_down(Key::S) {
            self.camera.position.x -= forward.x * move_speed;
            self.camera.position.z -= forward.z * move_speed;
        }
        if keyboard.is_key_down(Key::A) {
            self.camera.position.x -= right.x * move_speed;
            self.camera.position.z -= right.z * move_speed;
        }
        if keyboard.is_key_down(Key::D) {
            self.camera.position.x += right.x * move_speed;
            self.camera.position.z += right.z * move_speed;
        }

        if keyboard.is_key_down(Key::Space) {
            self.camera.position.y += move_speed;
        }
        if keyboard.is_key_down(Key::Shift) {
            self.camera.position.y -= move_speed;
        }
    }

    fn render(&mut self, drawer: &mut dyn Drawer) {
        // Color de cielo estilo Minecraft diurno
        drawer.clear([135, 206, 235, 255]);

        let screen_w = 800.0;
        let screen_h = 600.0;

        let mut composer = SceneComposer::new(self.render_mode);

        fn avg_depth_of_vertices(camera: &Camera3D, verts: &[Vec3]) -> f32 {
            let mut sum = 0.0f32;
            let mut cnt = 0usize;
            for v in verts {
                let cv = camera.world_to_camera_space(*v);
                sum += cv.z;
                cnt += 1;
            }
            if cnt == 0 { 0.0 } else { sum / cnt as f32 }
        }

        // Renderizar cada bloque del terreno generado por el mapa de ruido
        for (_, vertices, colors) in &self.blocks {
            let depth = avg_depth_of_vertices(&self.camera, vertices);
            let verts_ref = vertices;
            let cols = *colors;
            let camera_ref = &self.camera;

            composer.add_3d(depth, move |d: &mut dyn Drawer| {
                Cube3D::draw(d, verts_ref, cols, camera_ref, screen_w, screen_h);
            });
        }

        composer.render(drawer);
    }

    fn drawer_manager(&mut self) -> &mut DrawerManager {
        &mut self.drawer_manager
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::args().nth(1).as_deref() == Some("test_gpu") {
        return test_gpu::run();
    }

    let app = DirectGpuApp::new();
    window::run(app)
}