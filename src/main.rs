use bevy::{
    prelude::*,
    window::{ExitCondition, PrimaryWindow, Window, WindowPlugin, WindowResolution},
    winit::WinitWindows,
};
use bevy_kira_audio::prelude::AudioPlugin as KiraAudioPlugin;

use tabs_app::shaders::RegisterShadersPlugin;
use tabs_app::states::{AppState, GameplayPlugin, SongSelectPlugin, StartupPlugin};
use tabs_app::widgets::UiLayerPlugin;
use tabs_app::{file::config::ConfigPlugin, states::GameState};

#[cfg(not(feature = "production"))]
use tabs_app::debug::DebugPlugin;

fn main() {
    App::new()
        .add_plugins((
            ConfigPlugin,
            #[cfg(not(feature = "production"))]
            DebugPlugin,
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "TABS".to_string(),
                        resolution: WindowResolution::new(600, 400),
                        ..default()
                    }),
                    exit_condition: ExitCondition::OnPrimaryClosed,
                    ..default()
                })
                .disable::<bevy::audio::AudioPlugin>(),
            KiraAudioPlugin,
            RegisterShadersPlugin,
            UiLayerPlugin,
            StartupPlugin,
            SongSelectPlugin,
            GameplayPlugin,
        ))
        .init_state::<AppState>()
        .init_state::<GameState>()
        .add_systems(OnEnter(AppState::InitialLoad), start_maximized)
        .run();
}

fn start_maximized(
    winit_windows: Option<NonSend<WinitWindows>>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    mut windows: Query<&mut Window>,
) {
    let Some(winit_windows) = winit_windows else {
        return;
    };

    let Some(window_entity) = primary_window_query.iter().next() else {
        return;
    };

    if let Some(window) = winit_windows.get_window(window_entity) {
        if window.is_maximized() {
            return;
        }

        if let Ok(mut window) = windows.get_mut(window_entity) {
            window.set_maximized(true);
        }
    }
}
