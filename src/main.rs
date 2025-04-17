use bevy::prelude::*;
use bevy::window::{WindowPlugin, WindowResolution};
use tabs_game::constants::{DEFAULT_WIN_WIDTH, DEFAULT_WIN_HEIGHT, DEFAULT_TITLE};
use tabs_game::scenes::song_selection::setup_song_select;
use tabs_game::states::AppState;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: DEFAULT_TITLE.to_string(),
                resolution: WindowResolution::new(DEFAULT_WIN_WIDTH, DEFAULT_WIN_HEIGHT),
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        .add_systems(OnEnter(AppState::SongSelect), setup_song_select)
        // .add_systems(Update, handle_ui_button_interaction.run_if(in_state(AppState::SongSelect)))
        // .add_systems(OnExit(AppState::SongSelect), cleanup_song_select)
        // .add_systems(OnEnter(AppState::Gameplay), setup_gameplay)
        .run();
}