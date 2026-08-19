mod core;
mod test_gpu;

use std::env;
use std::sync::Arc;

use core::camera::camera_3d::Camera3D;
use core::camera::vec3::Vec3;
use core::controls::keyboard::{Key, KeyboardManager};
use core::rendering::drawers::drawer_backend::Drawer;
use core::rendering::drawers::drawer_manager::{BackendPreference, DrawerManager};
use core::rendering::drawers::gpu_drawer::GpuDrawer;
use core::rendering::primitives::cube::{Cube3D, CubeColors};
use core::rendering::scene::RenderOrderMode;
use core::window::state::State;
use core::window::window;

pub struct DirectGpuApp {
    drawer_manager: DrawerManager,
    camera: Camera3D,
    blocks: Vec<(f32, [Vec3; 8], CubeColors)>,
    render_mode: RenderOrderMode,
}

impl DirectGpuApp {
    pub fn new() -> Self {
        let drawer_manager = DrawerManager::new(None);

        // Posicionamos la cámara más retirada para apreciar el terreno amplio
        let mut camera = Camera3D::new(Vec3::new(0.0, 300.0, -700.0), 500.0);
        camera.yaw = 0.0;
        camera.pitch = 0.45;

        // --- GENERACIÓN DE TERRENO AMPLIO TIPO MINECRAFT ---
        let block_size = 25.0;
        let map_width = 32; // Terreno más amplio en X
        let map_depth = 32; // Terreno más amplio en Z

        let mut blocks = Vec::new();

        let half_w = (map_width as f32 * block_size) / 2.0;
        let half_d = (map_depth as f32 * block_size) / 2.0;

        // Paso 1: Precalcular las alturas de toda la matriz para conocer las vecinas
        let mut height_map = vec![vec![0; map_depth]; map_width];
        
        for x in 0..map_width {
            for z in 0..map_depth {
                let nx = x as f32 * 0.15;
                let nz = z as f32 * 0.15;
                let height_noise = (nx.sin() * 3.0 + nz.cos() * 2.5 + (nx * nz).sin() * 1.0) + 4.0;
                height_map[x][z] = height_noise.abs().round() as i32 + 2;
            }
        }

        // Paso 2: Generar solo bloques visibles (Superficie y paredes expuestas)
        for x in 0..map_width {
            for z in 0..map_depth {
                let column_height = height_map[x][z];
                let world_x = (x as f32 * block_size) - half_w;
                let world_z = (z as f32 * block_size) - half_d;

                // Solo necesitamos renderizar desde una profundidad razonable o la superficie
                // para evitar rellenar el interior masivamente.
                let min_y = (column_height - 3).max(0); // Capas superficiales visibles

                for y in min_y..=column_height {
                    let world_y = (y as f32 * block_size) - 80.0;

                    let mut cube_vertices =
                        Cube3D::create_centered_vertices(block_size, block_size, block_size);
                    for v in &mut cube_vertices {
                        v.x += world_x;
                        v.y += world_y;
                        v.z += world_z;
                    }

                    // Definir colores según si es la capa más alta (pasto) o tierra expuesta
                    let cube_colors = if y == column_height {
                        CubeColors::PerFace([
                            [100, 200, 80, 255],
                            [80, 160, 60, 255],
                            [90, 180, 70, 255],
                            [110, 220, 90, 255],
                            [120, 230, 100, 255], // Tapa superior
                            [70, 140, 50, 255],
                        ])
                    } else {
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
        self.drawer_manager
            .set_preference(BackendPreference::ForceGpu);
    }

    fn update(&mut self, dt: f32, keyboard: &KeyboardManager) {
        let move_speed = 400.0 * dt;
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
        drawer.clear([135, 206, 235, 255]);

        let screen_w = 800.0;
        let screen_h = 600.0;

        for (_, vertices, colors) in &self.blocks {
            Cube3D::draw(drawer, vertices, *colors, &self.camera, screen_w, screen_h);
        }
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