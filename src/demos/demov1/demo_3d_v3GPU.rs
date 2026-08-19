mod core;

use std::env;
use std::sync::Arc;

use core::camera::camera_3d::Camera3D;
use core::camera::vec3::Vec3;
use core::controls::keyboard::{Key, KeyboardManager};
use core::rendering::drawers::drawer_backend::Drawer;
use core::rendering::drawers::drawer_manager::{BackendPreference, DrawerManager};
use core::rendering::drawers::gpu_drawer::GpuDrawer;
use core::rendering::primitives::capsule::{Capsule3D, CapsuleMesh};
use core::rendering::primitives::cube::{Cube3D, CubeColors};
use core::rendering::primitives::plane::Plane3D;
use core::rendering::primitives::sphere::{Sphere3D, SphereMesh};
use core::rendering::scene::{RenderOrderMode, SceneComposer};
use core::window::state::State;
use core::window::window;

pub struct DirectGpuApp {
    drawer_manager: DrawerManager,
    camera: Camera3D,
    plane_vertices: [Vec3; 4],
    cube_vertices: [Vec3; 8],
    sphere_mesh: SphereMesh,
    capsule_mesh: CapsuleMesh,
    render_mode: RenderOrderMode,
}

impl DirectGpuApp {
    pub fn new() -> Self {
        // Inicializa dinámicamente sin GPU; la GPU se acoplará en init_gpu
        let drawer_manager = DrawerManager::new(None);

        let mut camera = Camera3D::new(Vec3::new(0.0, 120.0, -320.0), 400.0);
        camera.yaw = 0.0;
        camera.pitch = 0.35;

        let plane_vertices = Plane3D::create_centered_vertices(500.0, 500.0);

        let mut cube_vertices = Cube3D::create_centered_vertices(60.0, 60.0, 60.0);
        for v in &mut cube_vertices {
            v.x -= 100.0;
            v.y += 30.0;
        }

        let mut sphere_mesh = Sphere3D::create_mesh(40.0, 12, 24);
        for v in &mut sphere_mesh.vertices {
            v.y += 40.0;
        }

        let mut capsule_mesh = Capsule3D::create_mesh(25.0, 60.0, 8, 16);
        for v in &mut capsule_mesh.vertices {
            v.x += 100.0;
            v.y += 55.0;
        }

        Self {
            drawer_manager,
            camera,
            plane_vertices,
            cube_vertices,
            sphere_mesh,
            capsule_mesh,
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
        // Instanciamos el GpuDrawer con la configuración de la superficie real
        let gpu_drawer = GpuDrawer::new(device, queue, surface, &config);

        // Reemplazamos el DrawerManager por una nueva instancia con GPU activa
        self.drawer_manager = DrawerManager::new(Some(gpu_drawer));
        self.drawer_manager.set_preference(BackendPreference::Auto);
    }

    fn update(&mut self, dt: f32, keyboard: &KeyboardManager) {
        let move_speed = 200.0 * dt;
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
        drawer.clear([15, 15, 20, 255]);

        let screen_w = 800.0;
        let screen_h = 600.0;

        // Composer de escena que acumula draw calls y las ordena según `render_mode`.
        let mut composer = SceneComposer::new(self.render_mode);

        // Helper: promedio de profundidad (Z en espacio cámara) para un conjunto de vértices
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

        // Plane (usar promedio de sus 4 vértices)
        let plane_depth = avg_depth_of_vertices(&self.camera, &self.plane_vertices);
        let plane_vertices_ref = &self.plane_vertices;
        composer.add_3d(plane_depth, |d: &mut dyn Drawer| {
            Plane3D::draw(
                d,
                plane_vertices_ref,
                [40, 45, 55, 255],
                &self.camera,
                screen_w,
                screen_h,
            );
        });

        // Cube
        let cube_depth = avg_depth_of_vertices(&self.camera, &self.cube_vertices);
        let cube_vertices_ref = &self.cube_vertices;
        let cube_colors = CubeColors::PerFace([
            [220, 50, 50, 255],
            [180, 40, 40, 255],
            [200, 60, 60, 255],
            [240, 70, 70, 255],
            [255, 100, 100, 255],
            [150, 30, 30, 255],
        ]);
        composer.add_3d(cube_depth, |d: &mut dyn Drawer| {
            Cube3D::draw(
                d,
                cube_vertices_ref,
                cube_colors,
                &self.camera,
                screen_w,
                screen_h,
            );
        });

        // Sphere (promedio de todos los vértices de la malla)
        let mut sphere_verts: Vec<Vec3> = Vec::new();
        sphere_verts.extend(self.sphere_mesh.vertices.iter().copied());
        let sphere_depth = avg_depth_of_vertices(&self.camera, &sphere_verts);
        let sphere_mesh_ref = &self.sphere_mesh;
        composer.add_3d(sphere_depth, |d: &mut dyn Drawer| {
            Sphere3D::draw(
                d,
                sphere_mesh_ref,
                [255, 140, 0, 255],
                &self.camera,
                screen_w,
                screen_h,
            );
        });

        // Capsule
        let mut cap_verts: Vec<Vec3> = Vec::new();
        cap_verts.extend(self.capsule_mesh.vertices.iter().copied());
        let capsule_depth = avg_depth_of_vertices(&self.camera, &cap_verts);
        let capsule_mesh_ref = &self.capsule_mesh;
        composer.add_3d(capsule_depth, |d: &mut dyn Drawer| {
            Capsule3D::draw(
                d,
                capsule_mesh_ref,
                [0, 200, 240, 255],
                &self.camera,
                screen_w,
                screen_h,
            );
        });

        // Ejecutar dibujo ordenado
        composer.render(drawer);
    }

    fn drawer_manager(&mut self) -> &mut DrawerManager {
        &mut self.drawer_manager
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = DirectGpuApp::new();
    window::run(app)
}
