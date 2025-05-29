use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use crate::shaders::{ AbaaMaterial };
use crate::file::{ AppConfig, Settings, Themes };

pub mod icons;
pub use icons::{ UiIcon };

pub mod button;
pub use button::{ UiButton, GenericButton, ButtonStyle, ButtonType, Active };

pub mod selectable;
pub use selectable::{
    Selectable,
    SelectableType,
    SelectableStyle,
    SelectableButton,
    SelectedEvent,
};

pub mod scrollable_container;
pub use scrollable_container::{ ScrollbarMovedEvent, ScrollContainer, ScrollContainerStyle };

pub mod ui_window;
pub use ui_window::{ UiWindow, UiWindowOptions, UiWindowStyle };

pub mod card_button;
pub use card_button::{ Card, CardStyle };

pub mod layers;
pub use layers::{ UiLayer, UiLayerStack };

#[derive(SystemParam)]
pub struct UiContext<'w, 's> {
    pub themes: Res<'w, Themes>,
    pub settings: Res<'w, Settings>,
    pub config: Res<'w, AppConfig>,
    pub materials: Res<'w, Assets<AbaaMaterial>>,
    pub children_query: Query<'w, 's, &'static Children>,
    pub asset_server: Res<'w, AssetServer>,
    pub window: Single<'w, Entity, With<Window>>,
}

#[derive(Debug, Clone)]
pub struct UiBorder {
    pub color: Color,
    pub size: UiRect,
    pub radius: BorderRadius,
}

impl Default for UiBorder {
    fn default() -> Self {
        UiBorder {
            color: Color::BLACK,
            size: UiRect::all(Val::Px(1.0)),
            radius: BorderRadius::all(Val::Px(0.0)),
        }
    }
}

pub struct UiLayerPlugin;

impl Plugin for UiLayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiLayerStack::default())
            .add_plugins(ui_window::UiWindowPlugin)
            .add_systems(Update, (
                button::default_button_setup,
                button::add_active_listener,
                button::remove_active_listener,
                selectable::active_added_listener,
                selectable::active_removed_listener,
            ))
            .add_event::<SelectedEvent>()
            .add_plugins(scrollable_container::ScrollContainerPlugin);
    }
}
