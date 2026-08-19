use crate::core::camera::vec3::Vec3;

/// Representa una cámara en un entorno 3D con proyección perspectiva.
///
/// La cámara define la posición del observador y la orientación mediante ángulos de Euler.
/// Transforma coordenadas del **Espacio del Mundo** al **Espacio de Vista (Cámara)**.
pub struct Camera3D {
    /// Posición 3D de la cámara en el mundo `(X, Y, Z)`.
    pub position: Vec3,
    /// Rotación en el eje X (Pitch / Cabeceo) en radianes.
    pub pitch: f32,
    /// Rotación en el eje Y (Yaw / Guiñada) en radianes.
    pub yaw: f32,
    /// Rotación en el eje Z (Roll / Alabeo) en radianes.
    pub roll: f32,
    /// Campo de visión / Distancia focal para la proyección perspectiva.
    pub fov: f32,
}

impl Camera3D {
    /// Crea una nueva cámara 3D ubicada en una posición específica.
    pub fn new(position: Vec3, fov: f32) -> Self {
        Self {
            position,
            pitch: 0.0,
            yaw: 0.0,
            roll: 0.0,
            fov,
        }
    }

    /// Transforma un punto del Espacio del Mundo al Espacio de la Cámara.
    ///
    /// Aplica primero la translación inversa de la posición de la cámara y luego
    /// las matrices de rotación inversas (Yaw, Pitch, Roll).
    pub fn world_to_camera_space(&self, world_pos: Vec3) -> Vec3 {
        // 1. Traslación relativa a la posición de la cámara
        let mut x = world_pos.x - self.position.x;
        let mut y = world_pos.y - self.position.y;
        let mut z = world_pos.z - self.position.z;

        // 2. Aplicar rotación inversa en Yaw (eje Y)
        if self.yaw != 0.0 {
            let cos_y = (-self.yaw).cos();
            let sin_y = (-self.yaw).sin();
            let nx = x * cos_y - z * sin_y;
            let nz = x * sin_y + z * cos_y;
            x = nx;
            z = nz;
        }

        // 3. Aplicar rotación inversa en Pitch (eje X)
        if self.pitch != 0.0 {
            let cos_p = (-self.pitch).cos();
            let sin_p = (-self.pitch).sin();
            let ny = y * cos_p - z * sin_p;
            let nz = y * sin_p + z * cos_p;
            y = ny;
            z = nz;
        }

        // 4. Aplicar rotación inversa en Roll (eje Z)
        if self.roll != 0.0 {
            let cos_r = (-self.roll).cos();
            let sin_r = (-self.roll).sin();
            let nx = x * cos_r - y * sin_r;
            let ny = x * sin_r + y * cos_r;
            x = nx;
            y = ny;
        }

        Vec3::new(x, y, z)
    }

    /// Proyecta un punto en Espacio de Cámara a coordenadas 2D de pantalla `(X, Y)`.
    ///
    /// Devuelve `None` si el punto se encuentra detrás de la cámara ($Z \le 0$).
    /// Proyecta un punto en Espacio de Cámara a coordenadas 2D de pantalla `(X, Y)`.
    /// Devuelve `None` si el punto se encuentra detrás de la cámara ($Z \le 0.1$).
    pub fn project_to_screen(
        &self,
        cam_pos: Vec3,
        screen_w: f32,
        screen_h: f32,
    ) -> Option<(f32, f32)> {
        // 1. Near Plane estricto: rechaza puntos demasiado cercanos o detrás de la cámara
        const NEAR_PLANE: f32 = 1.0;
        if cam_pos.z < NEAR_PLANE {
            return None;
        }

        let half_w = screen_w * 0.5;
        let half_h = screen_h * 0.5;

        let screen_x = (cam_pos.x / cam_pos.z) * self.fov + half_w;
        let screen_y = half_h - (cam_pos.y / cam_pos.z) * self.fov;

        // 2. Limitar (clamp) los valores a un rango amplio pero seguro para evitar overflow al convertir a i32/usize
        let safe_x = screen_x.clamp(-10_000.0, 10_000.0);
        let safe_y = screen_y.clamp(-10_000.0, 10_000.0);

        Some((safe_x, safe_y))
    }
}
