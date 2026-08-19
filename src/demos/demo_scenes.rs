mod core;

use core::camera3D::Camera3D;
use core::controls::keyboard::{Key, KeyboardManager};
use core::drawer::Drawer;
use core::render3D::{Drawer3D, Vec3};
use core::scenes::scene::{DimensionMode, Scene, SceneConfig};
use core::scenes::scenes_manager::SceneManager;
use core::window;

// ==========================================
// ESCENA 1: TRES CUBOS (DEMO)
// ==========================================
pub struct ThreeCubesScene {
    config: SceneConfig,
    camera: Camera3D,
    cube_rot: Vec3,
}

impl ThreeCubesScene {
    pub fn new() -> Self {
        Self {
            config: SceneConfig {
                name: "Three Cubes Scene".to_string(),
                dimension: DimensionMode::Mode3D,
                gravity: (0.0, -9.8, 0.0),
            },
            camera: Camera3D::new(Vec3::new(0.0, 0.0, -10.0), 800.0),
            cube_rot: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

impl Scene for ThreeCubesScene {
    fn config(&self) -> &SceneConfig {
        &self.config
    }

    fn update(&mut self, dt: f32, keyboard: &KeyboardManager) {
        let move_speed = 5.0 * dt;

        if keyboard.is_key_down(Key::W) { self.camera.position.z += move_speed; }
        if keyboard.is_key_down(Key::S) { self.camera.position.z -= move_speed; }
        if keyboard.is_key_down(Key::A) { self.camera.position.x -= move_speed; }
        if keyboard.is_key_down(Key::D) { self.camera.position.x += move_speed; }
        if keyboard.is_key_down(Key::Space) { self.camera.position.y += move_speed; }
        if keyboard.is_key_down(Key::Shift) { self.camera.position.y -= move_speed; }

        self.cube_rot.x = (self.cube_rot.x + 45.0 * dt) % 360.0;
        self.cube_rot.y = (self.cube_rot.y + 90.0 * dt) % 360.0;
        self.cube_rot.z = (self.cube_rot.z + 30.0 * dt) % 360.0;
    }

    fn render(&mut self, drawer: &mut Drawer) {
        drawer.clear([15, 15, 25, 255]);

        let mut drawer3d = Drawer3D::new(drawer, &self.camera);

        // Cubo 1: Cyan wireframe
        drawer3d.draw_cube(
            Vec3::new(-2.2, 0.0, 5.0),
            1.2,
            self.cube_rot,
            [0, 255, 200, 255],
            true,
        );

        // Cubo 2: Naranja sólido
        drawer3d.draw_cube(
            Vec3::new(0.0, 0.0, 5.0),
            1.2,
            self.cube_rot,
            [255, 100, 50, 255],
            false,
        );

        // Cubo 3: Multicolor Rubik
        let rubik_palette = [
            [255, 50, 50, 255],
            [50, 255, 50, 255],
            [50, 100, 255, 255],
            [255, 255, 50, 255],
            [255, 150, 50, 255],
            [200, 50, 255, 255],
        ];

        drawer3d.draw_multicolor_cube(Vec3::new(2.2, 0.0, 5.0), 1.2, self.cube_rot, rubik_palette);
    }
}

// ==========================================
// ESCENA 2: UN SOLO CUBO VERDE
// ==========================================
pub struct SingleGreenCubeScene {
    config: SceneConfig,
    camera: Camera3D,
    cube_rot: Vec3,
}

impl SingleGreenCubeScene {
    pub fn new() -> Self {
        Self {
            config: SceneConfig {
                name: "Single Green Cube Scene".to_string(),
                dimension: DimensionMode::Mode3D,
                gravity: (0.0, -9.8, 0.0),
            },
            camera: Camera3D::new(Vec3::new(0.0, 0.0, -10.0), 800.0),
            cube_rot: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

impl Scene for SingleGreenCubeScene {
    fn config(&self) -> &SceneConfig {
        &self.config
    }

    fn update(&mut self, dt: f32, keyboard: &KeyboardManager) {
        let move_speed = 5.0 * dt;

        if keyboard.is_key_down(Key::W) { self.camera.position.z += move_speed; }
        if keyboard.is_key_down(Key::S) { self.camera.position.z -= move_speed; }
        if keyboard.is_key_down(Key::A) { self.camera.position.x -= move_speed; }
        if keyboard.is_key_down(Key::D) { self.camera.position.x += move_speed; }
        if keyboard.is_key_down(Key::Space) { self.camera.position.y += move_speed; }
        if keyboard.is_key_down(Key::Shift) { self.camera.position.y -= move_speed; }

        self.cube_rot.x = (self.cube_rot.x + 30.0 * dt) % 360.0;
        self.cube_rot.y = (self.cube_rot.y + 60.0 * dt) % 360.0;
    }

    fn render(&mut self, drawer: &mut Drawer) {
        drawer.clear([10, 20, 10, 255]); // Fondo verdoso oscuro

        let mut drawer3d = Drawer3D::new(drawer, &self.camera);

        // Único cubo verde al centro
        drawer3d.draw_cube(
            Vec3::new(0.0, 0.0, 5.0),
            1.5,
            self.cube_rot,
            [0, 255, 100, 255],
            false,
        );
    }
}

// ==========================================
// SCENE MANAGER CON TEMPORIZADOR DE 10 SEGUNDOS
// ==========================================
pub struct TimedSceneManager {
    manager: SceneManager,
    timer: f32,
    current_key: &'static str,
}

impl TimedSceneManager {
    pub fn new() -> Self {
        let mut manager = SceneManager::new();

        manager.register_scene("three_cubes", || Box::new(ThreeCubesScene::new()));
        manager.register_scene("single_green_cube", || Box::new(SingleGreenCubeScene::new()));

        let initial_key = "three_cubes";
        manager.load_scene(initial_key);

        Self {
            manager,
            timer: 0.0,
            current_key: initial_key,
        }
    }
}

impl core::state::State for TimedSceneManager {
    fn update(&mut self, dt: f32, keyboard: &KeyboardManager) {
        // Incrementar el temporizador con el delta_time
        self.timer += dt;

        // Cada 10 segundos alternamos la escena
        if self.timer >= 10.0 {
            self.timer = 0.0;
            self.current_key = match self.current_key {
                "three_cubes" => "single_green_cube",
                _ => "three_cubes",
            };

            println!("Cambiando automáticamente a la escena: {}", self.current_key);
            self.manager.load_scene(self.current_key);
        }

        // Actualizar la escena activa
        self.manager.update(dt, keyboard);
    }

    fn render(&mut self, drawer: &mut Drawer) {
        self.manager.render(drawer);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timed_manager = TimedSceneManager::new();
    window::run(timed_manager)
}