use std::collections::HashSet;

// ==========================================
// ENUMERACIÓN DE TECLAS ABSTRAÍDAS
// ==========================================
/// Representa las teclas del teclado de forma agnóstica a la librería de ventana.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    // Letras principales
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Números superiores
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

    // Teclas de dirección / Modificadores
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Escape,
    Shift,
    Control,
    Alt,
    Tab,

    // Tecla no mapeada o desconocida
    Unknown,
}

// ==========================================
// ESTRUCTURA INPUTMANAGER
// ==========================================
/// Gestiona el estado del teclado cuadro por cuadro (Frame by Frame).
pub struct KeyboardManager {
    keys_down: HashSet<Key>,     // Teclas mantenidas presionadas actualmente
    keys_pressed: HashSet<Key>,  // Teclas presionadas ÚNICAMENTE en este frame
    keys_released: HashSet<Key>, // Teclas soltadas ÚNICAMENTE en este frame
}

impl KeyboardManager {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
        }
    }

    /// Limpia los eventos temporales del frame (debe llamarse al inicio de cada update)
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
    }

    /// Registra la pulsación de una tecla desde el evento nativo de la ventana
    pub fn register_press(&mut self, key: Key) {
        if !self.keys_down.contains(&key) {
            self.keys_down.insert(key);
            self.keys_pressed.insert(key); // Evento "Just Pressed"
        }
    }

    /// Registra la liberación de una tecla desde el evento nativo de la ventana
    pub fn register_release(&mut self, key: Key) {
        if self.keys_down.remove(&key) {
            self.keys_released.insert(key);
        }
    }

    // ==========================================
    // MÉTODOS DE CONSULTA (API PÚBLICA)
    // ==========================================

    /// Retorna `true` mientras la tecla esté siendo mantenida presionada.
    pub fn is_key_down(&self, key: Key) -> bool {
        self.keys_down.contains(&key)
    }

    /// Retorna `true` ÚNICAMENTE en el primer cuadro en que la tecla fue presionada.
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Retorna `true` ÚNICAMENTE en el cuadro en que la tecla fue soltada.
    pub fn is_key_released(&self, key: Key) -> bool {
        self.keys_released.contains(&key)
    }
}
