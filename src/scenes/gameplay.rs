use crate::file::song::{TabNote, VocalPhrase};
use crate::file::Tab;
use crate::scenes::song_selection::SongSelectState;
use crate::states::GameState;
use bevy::prelude::*;
use bevy::time::{Timer, TimerMode};
use std::time::Instant;

#[derive(Resource, Default)]
pub struct GameplayAssets {
    audio_handle: Handle<AudioSource>,
    tab_handle: Handle<Tab>,
}

#[derive(Resource, Default)]
pub struct SongPlayback {
    start_instant: Option<Instant>,
}

impl SongPlayback {
    pub fn mark_started(&mut self) {
        self.start_instant = Some(Instant::now());
    }

    pub fn current_time(&self) -> Option<f32> {
        self.start_instant
            .as_ref()
            .map(|instant| instant.elapsed().as_secs_f32())
    }
}

#[derive(Component)]
pub struct LoadingUI;

pub fn setup_loading_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            LoadingUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Loading..."),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn start_loading_assets(
    mut loading: ResMut<GameplayAssets>,
    selected_song: Res<SongSelectState>,
    asset_server: Res<AssetServer>,
    mut gameplay_state: ResMut<NextState<GameState>>,
) {
    gameplay_state.set(GameState::Loading);
    let song_metadata_path = if let Some(song_handle) = &selected_song.selected_song {
        let metadata_path = if let Some(path) = song_handle.path() {
            path.path()
        } else {
            panic!("Provided song has an invalid path!");
        };
        metadata_path
    } else {
        panic!("Provided no song!");
    };

    let instrument_name = if let Some(instrument) = &selected_song.selected_instrument {
        instrument
    } else {
        panic!("No instrument provided for the song!");
    };

    let song_folder_path = song_metadata_path.parent().unwrap();

    let audio_path = song_folder_path.join("song.ogg");
    let instrument_file = format!("{}.tab", instrument_name);
    let instrument_path = song_folder_path.join(instrument_file);

    info!("Song folder {}", song_folder_path.display());
    info!("Audio path {}", audio_path.display());
    info!("Instrument path {}", instrument_path.display());

    let audio_handle: Handle<AudioSource> = asset_server.load(audio_path);
    loading.audio_handle = audio_handle;

    let tab_handle: Handle<Tab> = asset_server.load(instrument_path);
    loading.tab_handle = tab_handle;
}

pub fn check_loading_progress(
    loading: Res<GameplayAssets>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut all_loaded = true;
    if !asset_server.load_state(&loading.audio_handle).is_loaded() {
        all_loaded = false;
    }

    if !asset_server.load_state(&loading.tab_handle).is_loaded() {
        all_loaded = false;
    }

    if all_loaded {
        next_state.set(GameState::InGame);
    }
}
pub fn update_loading_ui(mut query: Query<&mut Visibility, With<LoadingUI>>) {
    for mut vis in &mut query {
        *vis = Visibility::Hidden;
    }
}

pub fn start_game_session(
    mut commands: Commands,
    assets: Res<GameplayAssets>,
    mut song_clock: ResMut<SongPlayback>,
) {
    commands.spawn(AudioPlayer::new(assets.audio_handle.clone()));
    song_clock.mark_started();
}

pub fn track_timeline(
    assets: Res<GameplayAssets>,
    song_clock: Res<SongPlayback>,
    tabs: Res<Assets<Tab>>,
    time: Res<Time>,
    mut timer: Local<Timer>,
) {
    if timer.duration().is_zero() {
        *timer = Timer::from_seconds(0.5, TimerMode::Repeating);
    }

    if !timer.tick(time.delta()).just_finished() {
        return;
    }

    let Some(current_time) = song_clock.current_time() else {
        return;
    };

    if assets.tab_handle == Handle::default() {
        return;
    }

    let Some(tab) = tabs.get(&assets.tab_handle) else {
        return;
    };

    match tab {
        Tab::Strings(tab_data) => {
            let mut upcoming: Vec<(f32, i32, &TabNote)> = tab_data
                .note_charts
                .iter()
                .flat_map(|chart| chart.notes.iter().map(move |note| (chart.difficulty, note)))
                .filter(|(_, note)| note.time >= current_time)
                .map(|(difficulty, note)| (note.time - current_time, difficulty, note))
                .collect();

            upcoming.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            upcoming.truncate(3);

            if upcoming.is_empty() {
                info!(
                    "[TabSync] t={:.3}s | no upcoming notes ({} charts)",
                    current_time,
                    tab_data.note_charts.len()
                );
            } else {
                let notes_summary = upcoming
                    .iter()
                    .map(|(delta, difficulty, note)| {
                        format!(
                            "+{:.3}s d{} str:{} fret:{}",
                            delta, difficulty, note.string, note.fret
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");

                info!("[TabSync] t={:.3}s | {}", current_time, notes_summary);
            }
        }
        Tab::Vocals(vocals) => {
            let mut upcoming: Vec<(f32, &VocalPhrase)> = vocals
                .vocals
                .iter()
                .filter(|phrase| phrase.time >= current_time)
                .map(|phrase| (phrase.time - current_time, phrase))
                .collect();

            upcoming.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            upcoming.truncate(3);

            if upcoming.is_empty() {
                info!("[TabSync] t={:.3}s | no upcoming lyrics", current_time);
            } else {
                let lyric_summary = upcoming
                    .iter()
                    .map(|(delta, phrase)| format!("+{:.3}s '{}'", delta, phrase.lyric))
                    .collect::<Vec<_>>()
                    .join(" | ");
                info!("[TabSync] t={:.3}s | {}", current_time, lyric_summary);
            }
        }
    }
}
