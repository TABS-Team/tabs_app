use bevy::prelude::*;

pub mod ui_window;
pub use ui_window::{UiWindow, UiWindowContext, UiWindowOptions, UiWindowStyle};

pub mod layers;
pub use layers::{UiLayer, UiLayerStack};


// Common used structs throughout all widgets
#[derive(Clone)]
pub enum UiSize {
    Px(f32),
    Percent(f32),
}

pub struct UiLayerPlugin;

impl Plugin for UiLayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(UiLayerStack::default())
            .add_plugins(ui_window::UiWindowPlugin)
        ;
    }
}