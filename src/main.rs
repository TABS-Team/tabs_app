use bevy::{
    prelude::*,
    window::{ WindowPlugin, ExitCondition, PrimaryWindow, Window, WindowResolution },
    winit::{ WinitWindows },
};

use tabs_app::states::{ AppState, StartupPlugin, SongSelectPlugin };
use tabs_app::file::config::ConfigPlugin;
use tabs_app::widgets::UiLayerPlugin;
use tabs_app::shaders::RegisterShadersPlugin;

#[cfg(not(feature = "production"))]
use tabs_app::debug::{ DebugPlugin };

fn main() {
    App::new()
        .add_plugins((
            ConfigPlugin,
            #[cfg(not(feature = "production"))] DebugPlugin,
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "TABS".to_string(),
                    resolution: WindowResolution::new(600.0, 400.0),
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
        .add_systems(OnEnter(AppState::InitialLoad), start_maximized)
        .run();
}

fn start_maximized(
    winit_windows: NonSend<WinitWindows>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    mut windows: Query<&mut Window>
) {
    if let Ok(window_entity) = primary_window_query.single() {
        if let Some(window) = winit_windows.get_window(window_entity) {
            if !window.is_maximized() {
                if let Ok(mut window) = windows.get_mut(window_entity) {
                    window.set_maximized(true);
                }
            }
        }
    }
}
