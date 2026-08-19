// use std::collections::HashMap;
// use crate::core::controls::keyboard::KeyboardManager;
// use crate::core::drawer::Drawer;
// use crate::core::scenes::scene::Scene;
// use crate::core::state::State;

// type SceneFactory = Box<dyn Fn() -> Box<dyn Scene>>;

// pub struct SceneManager {
//     current_scene: Option<Box<dyn Scene>>,
//     registry: HashMap<String, SceneFactory>,
// }

// impl SceneManager {
//     pub fn new() -> Self {
//         Self {
//             current_scene: None,
//             registry: HashMap::new(),
//         }
//     }

//     pub fn register_scene<F>(&mut self, key: &str, factory: F)
//     where
//         F: Fn() -> Box<dyn Scene> + 'static,
//     {
//         self.registry.insert(key.to_string(), Box::new(factory));
//     }

//     pub fn load_scene(&mut self, key: &str) {
//         if let Some(factory) = self.registry.get(key) {
//             if let Some(mut old_scene) = self.current_scene.take() {
//                 old_scene.destroy();
//             }

//             let mut new_scene = factory();
//             new_scene.init();
//             self.current_scene = Some(new_scene);
//         } else {
//             eprintln!("Error: La escena '{}' no está registrada.", key);
//         }
//     }
// }

// // Implementamos State en SceneManager para que 'window::run' pueda ejecutarlo directamente
// impl State for SceneManager {
//     fn update(&mut self, dt: f32, keyboard: &KeyboardManager) {
//         if let Some(scene) = &mut self.current_scene {
//             scene.update(dt, keyboard);
//         }
//     }

//     fn render(&mut self, drawer: &mut Drawer) {
//         if let Some(scene) = &mut self.current_scene {
//             scene.render(drawer);
//         }
//     }
// }