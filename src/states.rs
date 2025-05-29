use bevy::{ prelude::* };
use crate::file::theme::setup_theme;
use crate::file::settings::setup_settings;
use crate::scenes::{
    setup_song_select,
    setup_song_preview,
    check_song_assets_ready,
    setup_camera,
    song_selection::SongHandles,
};
use crate::file::{ Song, SongLoader };
use thiserror::Error;

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    InitialLoad,
    Startup,
    SongSelect,
    SongPreview,
    Gameplay,
}

// Latches will work as synchronization tools for states. So if two functions need to work before state transitioning, we will use the latch system

#[derive(Resource, Default)]
pub struct StartupLatch {
    pub settings_loaded: bool,
    pub theme_loaded: bool,
}

pub fn check_startup_complete(
    latch: Res<StartupLatch>,
    mut next_state: ResMut<NextState<AppState>>
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
            .add_systems(OnEnter(AppState::InitialLoad), setup_theme)
            .add_systems(OnEnter(AppState::InitialLoad), setup_settings)
            .add_systems(OnEnter(AppState::InitialLoad), setup_camera)
            .add_systems(Update, check_startup_complete.run_if(in_state(AppState::InitialLoad)));
    }
}

pub struct SongSelectPlugin;

impl Plugin for SongSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::SongSelect), setup_song_select)
            .add_systems(
                Update,
                check_song_assets_ready
                    .run_if(resource_exists::<SongHandles>)
                    .run_if(in_state(AppState::SongSelect))
                    .after(setup_song_select)
            )
            .add_systems(OnEnter(AppState::SongPreview), setup_song_preview);
    }
}
