use crate::audio::StreamingAudio;
use crate::components::StringTimelinePlugin;
use crate::file::settings::setup_settings;
use crate::file::theme::setup_theme;
use crate::file::{Song, SongLoader, Tab, TabLoader};
use crate::scenes::gameplay::{
    check_loading_progress, setup_loading_ui, start_game_session, start_loading_assets,
    track_timeline, update_loading_ui, GameplayAssets, SongPlayback,
};
use crate::scenes::{
    check_song_assets_ready, cleanup_song_preview, handle_close_preview_input, setup_camera,
    setup_song_preview, setup_song_select, song_selection::SongHandles,
    transition_preview_to_gameplay,
};
use bevy::prelude::*;

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    InitialLoad,
    Startup,
    SongSelect,
    SongPreview,
    Gameplay,
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    InGame,
    Pause,
}

// Latches will work as synchronization tools for states. So if two functions need to work before state transitioning, we will use the latch system

#[derive(Resource, Default)]
pub struct StartupLatch {
    pub settings_loaded: bool,
    pub theme_loaded: bool,
}

pub fn check_startup_complete(
    latch: Res<StartupLatch>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if latch.settings_loaded && latch.theme_loaded {
        next_state.set(AppState::Startup);
    }
}

pub struct StartupPlugin;

impl Plugin for StartupPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StartupLatch::default())
            .init_asset::<Song>()
            .init_asset_loader::<SongLoader>()
            .init_asset::<Tab>()
            .init_asset_loader::<TabLoader>()
            .add_systems(OnEnter(AppState::InitialLoad), setup_theme)
            .add_systems(OnEnter(AppState::InitialLoad), setup_settings)
            .add_systems(OnEnter(AppState::InitialLoad), setup_camera)
            .add_systems(
                Update,
                check_startup_complete.run_if(in_state(AppState::InitialLoad)),
            );
    }
}

pub struct SongSelectPlugin;

impl Plugin for SongSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnTransition {
                exited: AppState::Startup,
                entered: AppState::SongSelect,
            },
            setup_song_select,
        )
        .add_systems(
            Update,
            check_song_assets_ready
                .run_if(resource_exists::<SongHandles>)
                .run_if(in_state(AppState::SongSelect))
                .after(setup_song_select),
        )
        .add_systems(OnEnter(AppState::SongPreview), setup_song_preview)
        .add_systems(OnExit(AppState::SongPreview), cleanup_song_preview)
        .add_systems(
            OnTransition {
                exited: AppState::SongPreview,
                entered: AppState::Gameplay,
            },
            transition_preview_to_gameplay,
        )
        .add_systems(
            Update,
            handle_close_preview_input.run_if(in_state(AppState::SongPreview)),
        );
    }
}

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StreamingAudio>()
            .init_resource::<GameplayAssets>()
            .init_resource::<SongPlayback>()
            .add_plugins(StringTimelinePlugin)
            .add_systems(OnEnter(AppState::Gameplay), setup_loading_ui)
            .add_systems(OnEnter(AppState::Gameplay), start_loading_assets)
            .add_systems(
                Update,
                check_loading_progress
                    .run_if(in_state(GameState::Loading).and(in_state(AppState::Gameplay))),
            )
            .add_systems(
                OnTransition {
                    exited: GameState::Loading,
                    entered: GameState::InGame,
                },
                update_loading_ui,
            )
            .add_systems(OnEnter(GameState::InGame), start_game_session)
            .add_systems(Update, track_timeline.run_if(in_state(GameState::InGame)));
    }
}
