use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use crate::shaders::{AbaaMaterial};

pub mod ui_window;
pub use ui_window::{UiWindow, UiWindowOptions, UiWindowStyle};

pub mod card_button;
pub use card_button::{Card, CardStyle};

pub mod layers;
pub use layers::{UiLayer, UiLayerStack};

#[derive(SystemParam)]
pub struct UiContext<'w, 's> {
    pub materials: Res<'w, Assets<AbaaMaterial>>,
    pub stack: ResMut<'w, UiLayerStack>,
    pub children_query: Query<'w, 's, &'static Children>,
    pub asset_server: Res<'w, AssetServer>
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