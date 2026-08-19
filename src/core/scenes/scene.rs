// use std::fmt::Debug;
// use crate::core::controls::keyboard::KeyboardManager;
// use crate::core::drawer::Drawer;

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum DimensionMode {
//     Mode2D,
//     Mode3D,
// }

// pub struct SceneConfig {
//     pub name: String,
//     pub dimension: DimensionMode,
//     pub gravity: (f32, f32, f32),
// }

// pub trait Scene {
//     fn config(&self) -> &SceneConfig;
//     fn init(&mut self) {}
//     fn update(&mut self, dt: f32, keyboard: &KeyboardManager);
//     fn render(&mut self, drawer: &mut Drawer);
//     fn destroy(&mut self) {}
// }