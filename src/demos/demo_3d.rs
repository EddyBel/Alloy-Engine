mod core;

use core::drawer::Drawer;
use core::render3D::{Drawer3D, Vec3};
use core::state::State;
use core::window;

struct MyGame {
    cube_rot: Vec3, // Ángulos de rotación en grados (Pitch, Yaw, Roll)
}

impl State for MyGame {
    fn update(&mut self, dt: f32) {
        // Incremento angular continuo en grados/segundo
        self.cube_rot.x += 45.0 * dt;
        self.cube_rot.y += 90.0 * dt;
        self.cube_rot.z += 30.0 * dt;

        // Limpieza de desbordamiento angular
        self.cube_rot.x %= 360.0;
        self.cube_rot.y %= 360.0;
        self.cube_rot.z %= 360.0;
    }

    fn render(&mut self, drawer: &mut Drawer) {
        // 1. Limpieza del framebuffer con color de fondo oscuro
        drawer.clear([15, 15, 25, 255]);

        // 2. Creación de la instancia de proyección 3D (FOV de 400.0)
        let mut drawer3d = Drawer3D::new(drawer, 400.0);

        // =======================================================
        // CUBO 1: Wireframe (Estructura alámbrica a la izquierda)
        // =======================================================
        drawer3d.draw_cube(
            Vec3::new(-2.2, 0.0, 5.0),
            1.2,
            self.cube_rot,
            [0, 255, 200, 255], // Cyan
            true,               // Wireframe activo
        );

        // =======================================================
        // CUBO 2: Relleno Monocolor (Al centro)
        // =======================================================
        drawer3d.draw_cube(
            Vec3::new(0.0, 0.0, 5.0),
            1.2,
            self.cube_rot,
            [255, 100, 50, 255], // Naranja
            false,              // Relleno sólido
        );

        // =======================================================
        // CUBO 3: Relleno Multicolor (Cada cara un color a la derecha)
        // =======================================================
        let rubik_palette = [
            [255, 50, 50, 255],   // Cara Frontal: Rojo
            [50, 255, 50, 255],   // Cara Trasera: Verde
            [50, 100, 255, 255],  // Cara Izquierda: Azul
            [255, 255, 50, 255],  // Cara Derecha: Amarillo
            [255, 150, 50, 255],  // Cara Superior: Naranja
            [200, 50, 255, 255],  // Cara Inferior: Púrpura
        ];

        drawer3d.draw_multicolor_cube(
            Vec3::new(2.2, 0.0, 5.0),
            1.2,
            self.cube_rot,
            rubik_palette,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game = MyGame {
        cube_rot: Vec3::new(0.0, 0.0, 0.0),
    };

    window::run(game)
}