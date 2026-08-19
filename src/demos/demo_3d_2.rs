mod core;

use core::camera3D::Camera3D;
// 1. IMPORTANTE: Importamos Key junto a KeyboardManager
use core::controls::keyboard::{Key, KeyboardManager};
use core::drawer::Drawer;
use core::render3D::{Drawer3D, Vec3};
use core::state::State;
use core::window;

struct MyGame {
    camera: Camera3D, // Objeto Cámara
    cube_rot: Vec3,
}

impl State for MyGame {
    fn update(&mut self, dt: f32, keyboard: &KeyboardManager) {
        // --------------------------------------------------
        // CONTROL DE MOVIMIENTO DE CÁMARA CON WASD
        // --------------------------------------------------
        let move_speed = 5.0 * dt; // Velocidad ajustada por tiempo delta

        // W / S: Avanzar / Retroceder en el eje Z
        if keyboard.is_key_down(Key::W) {
            self.camera.position.z += move_speed;
        }
        if keyboard.is_key_down(Key::S) {
            self.camera.position.z -= move_speed;
        }

        // A / D: Mover a la Izquierda / Derecha en el eje X
        if keyboard.is_key_down(Key::A) {
            self.camera.position.x -= move_speed;
        }
        if keyboard.is_key_down(Key::D) {
            self.camera.position.x += move_speed;
        }

        // Barra Espaciadora / Shift: Subir / Bajar en el eje Y
        if keyboard.is_key_down(Key::Space) {
            self.camera.position.y += move_speed;
        }
        if keyboard.is_key_down(Key::Shift) {
            self.camera.position.y -= move_speed;
        }

        // --------------------------------------------------
        // ROTACIÓN CONTINUA DE LOS CUBOS
        // --------------------------------------------------
        self.cube_rot.x = (self.cube_rot.x + 45.0 * dt) % 360.0;
        self.cube_rot.y = (self.cube_rot.y + 90.0 * dt) % 360.0;
        self.cube_rot.z = (self.cube_rot.z + 30.0 * dt) % 360.0;
    }

    fn render(&mut self, drawer: &mut Drawer) {
        drawer.clear([15, 15, 25, 255]);

        let mut drawer3d = Drawer3D::new(drawer, &self.camera);

        drawer3d.draw_cube(
            Vec3::new(-2.2, 0.0, 5.0),
            1.2,
            self.cube_rot,
            [0, 255, 200, 255],
            true,
        );

        drawer3d.draw_cube(
            Vec3::new(0.0, 0.0, 5.0),
            1.2,
            self.cube_rot,
            [255, 100, 50, 255],
            false,
        );

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game = MyGame {
        camera: Camera3D::new(Vec3::new(0.0, 0.0, -10.0), 800.0),
        cube_rot: Vec3::new(0.0, 0.0, 0.0),
    };

    window::run(game)
}
