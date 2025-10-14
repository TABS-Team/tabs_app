use bevy::prelude::*;

pub mod widget;
pub use widget::{UiBorder, UiContext};

pub mod icons;
pub use icons::{MaterialIcons, UiIcon};

pub mod button;
pub use button::{Active, ButtonStyle, ButtonType, GenericButton, UiButton};

pub mod selectable;
pub use selectable::{
    Selectable, SelectableButton, SelectableStyle, SelectableType, SelectedEvent,
};

pub mod scrollable_container;
pub use scrollable_container::{ScrollContainer, ScrollContainerStyle, ScrollbarMovedEvent};

pub mod ui_window;
pub use ui_window::{UiWindow, UiWindowOptions, UiWindowStyle};

pub mod card_button;
pub use card_button::{Card, CardStyle};

pub mod layers;
pub use layers::{UiLayer, UiLayerStack};

pub struct UiLayerPlugin;

impl Plugin for UiLayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiLayerStack::default())
            .init_resource::<MaterialIcons>()
            .add_plugins(ui_window::UiWindowPlugin)
            .add_systems(
                Update,
                (
                    button::default_button_setup,
                    button::add_active_listener,
                    button::remove_active_listener,
                    selectable::active_change_listener,
                    selectable::active_removed_listener,
                ),
            )
            .add_plugins(scrollable_container::ScrollContainerPlugin);
    }
}
