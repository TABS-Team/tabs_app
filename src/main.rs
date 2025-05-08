use bevy::{
    prelude::*,
    window::{WindowPlugin, WindowResolution, ExitCondition},
};
use tabs_app::states::{AppState, StartupPlugin, SongSelectPlugin};
use tabs_app::core::config::ConfigPlugin;
use tabs_app::widgets::UiLayerPlugin;
use tabs_app::materials::RegisterShadersPlugin;

#[cfg(not(feature = "production"))]
use tabs_app::debug::{DebugPlugin};

fn main() {
    App::new()
        .add_plugins((
            ConfigPlugin,
            #[cfg(not(feature = "production"))]
            DebugPlugin,
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "TABS".to_string(),
                    resolution: WindowResolution::new(800.0, 600.0),
                    ..default()
                }),
                exit_condition: ExitCondition::OnPrimaryClosed,
                ..default()
            }),
            RegisterShadersPlugin,
            UiLayerPlugin,
            StartupPlugin,
            SongSelectPlugin,
        ))
        .init_state::<AppState>()
        .run();
}