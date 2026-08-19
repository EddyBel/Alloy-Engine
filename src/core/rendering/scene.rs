use crate::core::rendering::drawers::drawer_backend::Drawer;

/// Modo de ordenamiento para la escena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderOrderMode {
    /// Algoritmo del pintor 3D: ordenar por profundidad (Z)
    Painter3D,
    /// Sistema 2D de capas / Z-Index: ordenar por entero (mayor = delante)
    Layers2D,
    /// Usar Z-buffer de la GPU (si el backend es GPU) — no se ordena por painter.
    HardwareZBuffer,
    /// Usar Z-buffer por software (CPUDrawer) — el CPUDrawer realizará tests por píxel.
    SoftwareZBuffer,
}

/// Compositor ligero que acumula llamadas de dibujo junto a una clave de ordenamiento.
///
/// - Para 3D se usa `depth: f32` (valores comparados descendente para pintar de lejos a cerca).
/// - Para 2D se usa `z_index: i32` (orden ascendente: menor detrás, mayor adelante).
pub struct SceneComposer<'a> {
    pub mode: RenderOrderMode,
    items_3d: Vec<(f32, Box<dyn Fn(&mut dyn Drawer) + 'a>)>,
    items_3d_vertices: Vec<Box<dyn Fn(&mut dyn Drawer) + 'a>>,
    items_2d: Vec<(i32, Box<dyn Fn(&mut dyn Drawer) + 'a>)>,
}

impl<'a> SceneComposer<'a> {
    pub fn new(mode: RenderOrderMode) -> Self {
        Self {
            mode,
            items_3d: Vec::new(),
            items_2d: Vec::new(),
            items_3d_vertices: Vec::new(),
        }
    }

    /// Añade un elemento 3D con su profundidad (en espacio cámara). Valores mayores se consideran más "lejanos" según convenciones internas.
    pub fn add_3d<F>(&mut self, depth: f32, draw: F)
    where
        F: Fn(&mut dyn Drawer) + 'a,
    {
        self.items_3d.push((depth, Box::new(draw)));
    }

    /// Añade un elemento 3D que dibuja sus primitivas con profundidad por vértice.
    /// La clausura debe usar las variantes `*_3d` del `Drawer` para proporcionar Z por vértice.
    pub fn add_3d_vertices<F>(&mut self, draw: F)
    where
        F: Fn(&mut dyn Drawer) + 'a,
    {
        self.items_3d_vertices.push(Box::new(draw));
    }

    /// Añade un elemento 2D con su `z_index` (capas). Valores mayores se dibujan después (encima).
    pub fn add_2d<F>(&mut self, z_index: i32, draw: F)
    where
        F: Fn(&mut dyn Drawer) + 'a,
    {
        self.items_2d.push((z_index, Box::new(draw)));
    }

    /// Ejecuta el renderizado ordenado en el `drawer` según el modo configurado.
    pub fn render(self, drawer: &mut dyn Drawer) {
        match self.mode {
            RenderOrderMode::Painter3D => {
                let mut items = self.items_3d;
                // Orden descendente por depth (mismo patrón usado en primitivas existentes)
                items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                for (_depth, draw) in items {
                    draw(drawer);
                }
            }
            RenderOrderMode::HardwareZBuffer => {
                // Para Z-buffer por hardware dejamos que el backend gestione la prueba de profundidad.
                // Activamos el modo depth en el drawer (implementaciones pueden ignorarlo si no aplicable).
                drawer.enable_depth(true);
                drawer.clear_depth_buffer();
                // Primero dibujamos las primitivas que ya vienen con profundidad por vértice
                for draw in self.items_3d_vertices {
                    draw(drawer);
                }

                // Luego dibujamos los elementos legados que usan depth escalar (se establece como flat depth)
                for (depth, draw) in self.items_3d {
                    drawer.set_current_depth(depth);
                    draw(drawer);
                }
                drawer.enable_depth(false);
            }
            RenderOrderMode::SoftwareZBuffer => {
                // Z-buffer por software: el CPUDrawer mantiene un depth buffer por píxel.
                drawer.enable_depth(true);
                drawer.clear_depth_buffer();
                // No reordenamos: cada primitiva aporta su 'depth' escalar que el CPUDrawer usará
                for (depth, draw) in self.items_3d {
                    drawer.set_current_depth(depth);
                    draw(drawer);
                }
                drawer.enable_depth(false);
            }
            RenderOrderMode::Layers2D => {
                let mut items = self.items_2d;
                // Orden ascendente por z_index: menores primero (detrás), mayores después (delante)
                items.sort_by(|a, b| a.0.cmp(&b.0));
                for (_z, draw) in items {
                    draw(drawer);
                }
            }
        }
    }
}
