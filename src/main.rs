use bevy::{
    prelude::*,
    window::{WindowPlugin, ExitCondition, PrimaryWindow, Window},
    winit::{WinitWindows},
};

use tabs_app::states::{AppState, StartupPlugin, SongSelectPlugin};
use tabs_app::file::config::ConfigPlugin;
use tabs_app::widgets::UiLayerPlugin;
use tabs_app::shaders::RegisterShadersPlugin;

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
                    position: WindowPosition::At(IVec2::new(0, 0)),
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
        .add_systems(OnEnter(AppState::InitialLoad), set_window_to_monitor_size)
        .run();
}

fn set_window_to_monitor_size(
    winit_windows: NonSend<WinitWindows>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    mut windows: Query<&mut Window>,
) {
    if let Ok(primary_entity) = primary_window_query.single() {
        if let Some(winit_window) = winit_windows.get_window(primary_entity) {
            if let Some(monitor) = winit_window.current_monitor() {
                let size = monitor.size();
                if let Ok(mut window) = windows.get_mut(primary_entity) {
                    window.resolution.set(size.width as f32, size.height as f32);
                }
            }
        }
    }
}