pub struct PointLight {
    pub position: [f32; 3], // Coordenadas en el mundo (x, y, z)
    pub color: [f32; 3],    // Color de la luz (ej: [1.0, 0.9, 0.7] para luz cálida)
    pub intensity: f32,     // Brillo o multiplicador de intensidad
    pub radius: f32,        // Radio máximo de alcance de la luz
}

impl PointLight {
    pub fn new(position: [f32; 3], color: [f32; 3], intensity: f32, radius: f32) -> Self {
        Self { position, color, intensity, radius }
    }
}