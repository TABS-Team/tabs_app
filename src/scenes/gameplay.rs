use crate::components::{
    clamp_block_duration, default_block_duration, visible_block_count, StringTimelineFeed,
    TimelineNote,
};
use crate::file::song::{StringTab, TabNote, TabNoteChart, VocalPhrase};
use crate::file::Tab;
use crate::scenes::song_selection::SongSelectState;
use crate::states::GameState;
use bevy::prelude::*;
use bevy_kira_audio::prelude::{
    Audio as GameAudio, AudioControl, AudioInstance, AudioSource as KiraAudioSource, PlaybackState,
};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;

const DEFAULT_DIFFICULTY_PERCENT: f32 = 100.0;
const MIN_NOTES_FOR_TEMPO: usize = 8;
const MIN_INTERVAL_SECONDS: f32 = 0.01;
const MAX_INTERVAL_SECONDS: f32 = 4.0;
const BEATS_PER_BLOCK: f32 = 4.0;

#[derive(Resource, Default)]
pub struct GameplayAssets {
    audio_handle: Handle<KiraAudioSource>,
    tab_handle: Handle<Tab>,
}

#[derive(Resource, Default)]
pub struct SongPlayback {
    start_instant: Option<Instant>,
    instance_handle: Option<Handle<AudioInstance>>,
}

impl SongPlayback {
    pub fn reset(&mut self) {
        self.start_instant = None;
        self.instance_handle = None;
    }

    pub fn mark_started(&mut self, handle: Handle<AudioInstance>) {
        self.start_instant = Some(Instant::now());
        self.instance_handle = Some(handle);
    }

    pub fn current_time(&self, instances: &Assets<AudioInstance>) -> Option<f32> {
        if let Some(handle) = self.instance_handle.as_ref() {
            if let Some(instance) = instances.get(handle) {
                match instance.state() {
                    PlaybackState::Playing { position }
                    | PlaybackState::Paused { position }
                    | PlaybackState::Pausing { position }
                    | PlaybackState::Stopping { position }
                    | PlaybackState::WaitingToResume { position }
                    | PlaybackState::Resuming { position } => {
                        return Some(position as f32);
                    }
                    _ => {}
                }
            }
        }

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
    mut song_clock: ResMut<SongPlayback>,
) {
    gameplay_state.set(GameState::Loading);
    song_clock.reset();
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

    let audio_handle: Handle<KiraAudioSource> = asset_server.load(audio_path);
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
    assets: Res<GameplayAssets>,
    mut song_clock: ResMut<SongPlayback>,
    audio: Res<GameAudio>,
) {
    song_clock.reset();

    if assets.audio_handle == Handle::default() {
        warn!("No audio handle available to start gameplay audio");
        return;
    }

    audio.stop();
    let instance_handle = audio.play(assets.audio_handle.clone()).handle();
    song_clock.mark_started(instance_handle);
}

pub fn track_timeline(
    assets: Res<GameplayAssets>,
    song_clock: Res<SongPlayback>,
    audio_instances: Res<Assets<AudioInstance>>,
    tabs: Res<Assets<Tab>>,
    mut timeline: ResMut<StringTimelineFeed>,
) {
    let Some(current_time) = song_clock.current_time(audio_instances.as_ref()) else {
        return;
    };

    if assets.tab_handle == Handle::default() {
        return;
    }

    let Some(tab) = tabs.get(&assets.tab_handle) else {
        return;
    };

    timeline.current_time = current_time;
    if timeline.block_duration <= 0.0 {
        timeline.block_duration = default_block_duration();
    }

    match tab {
        Tab::Strings(tab_data) => {
            let charts = select_charts_up_to(tab_data, DEFAULT_DIFFICULTY_PERCENT);
            if charts.is_empty() {
                timeline.block_duration = default_block_duration();
                timeline.block_duration_locked = false;
                update_timeline_window(&mut timeline, current_time);
                timeline.string_count = 0;
                timeline.notes.clear();
                return;
            }

            if !timeline.block_duration_locked {
                timeline.block_duration = determine_initial_block_duration(&charts);
                timeline.block_duration_locked = true;
            }
            update_timeline_window(&mut timeline, current_time);

            let window_start = timeline.window_start;
            let window_end = timeline.window_end;

            let string_count = charts
                .iter()
                .flat_map(|chart| chart.notes.iter())
                .filter_map(|note| {
                    if note.string >= 0 {
                        let idx = note.string as usize;
                        idx.checked_add(1)
                    } else {
                        None
                    }
                })
                .max()
                .unwrap_or(0);

            let mut sorted_charts: Vec<&TabNoteChart> = charts;
            sorted_charts.sort_by_key(|chart| chart.difficulty);

            let mut merged: HashMap<(u32, i32, i32), TabNote> = HashMap::new();
            for chart in sorted_charts {
                for note in &chart.notes {
                    if note.string < 0 {
                        continue;
                    }
                    let note_start = note.time;
                    let note_end = (note.time + note.sustain.max(0.0)).max(note_start);
                    if note_end < window_start {
                        continue;
                    }
                    if note_start > window_end {
                        continue;
                    }
                    let key = (note.time.to_bits(), note.string, note.fret);
                    merged.insert(key, note.clone());
                }
            }

            let mut merged_notes: Vec<TabNote> = merged.into_values().collect();
            merged_notes.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(Ordering::Equal)
                    .then(a.string.cmp(&b.string))
                    .then(a.fret.cmp(&b.fret))
            });

            let mut visible_notes = Vec::with_capacity(merged_notes.len());
            for note in merged_notes {
                let mut additional_frets = Vec::new();
                let mut push_additional = |candidate: i32| {
                    if candidate < 0 || candidate == note.fret {
                        return;
                    }
                    if !additional_frets.contains(&candidate) {
                        additional_frets.push(candidate);
                    }
                };
                if note.slide_to >= 0 {
                    push_additional(note.slide_to);
                }
                if note.slide_unpitch_to >= 0 {
                    push_additional(note.slide_unpitch_to);
                }
                if note.anchor_fret >= 0 {
                    push_additional(note.anchor_fret);
                }
                if note.max_bend > 0.0 {
                    let bend_target = note.fret + note.max_bend.ceil() as i32;
                    push_additional(bend_target);
                }

                visible_notes.push(TimelineNote {
                    time: note.time,
                    sustain: note.sustain.max(0.0),
                    string_index: note.string as usize,
                    fret: note.fret,
                    techniques: note.techniques.clone(),
                    additional_frets,
                    slide_target: (note.slide_to >= 0).then_some(note.slide_to),
                    slide_unpitched_target: (note.slide_unpitch_to >= 0)
                        .then_some(note.slide_unpitch_to),
                });
            }

            timeline.string_count = string_count;
            timeline.notes = visible_notes;
        }
        Tab::Vocals(vocals) => {
            timeline.block_duration = default_block_duration();
            timeline.block_duration_locked = false;
            update_timeline_window(&mut timeline, current_time);
            timeline.string_count = 0;
            timeline.notes.clear();
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

fn determine_initial_block_duration(charts: &[&TabNoteChart]) -> f32 {
    let mut times = collect_unique_note_times(charts);
    if times.len() < MIN_NOTES_FOR_TEMPO {
        return default_block_duration();
    }

    if let Some(beat_duration) = estimate_beat_duration(&mut times) {
        let block_duration = beat_duration * BEATS_PER_BLOCK;
        clamp_block_duration(block_duration)
    } else {
        default_block_duration()
    }
}

fn collect_unique_note_times(charts: &[&TabNoteChart]) -> Vec<f32> {
    let mut times = Vec::new();
    for chart in charts {
        for note in &chart.notes {
            if note.string < 0 {
                continue;
            }
            times.push(note.time);
        }
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    times.dedup_by(|a, b| (*a - *b).abs() < 0.0005);
    times
}

fn estimate_beat_duration(times: &mut [f32]) -> Option<f32> {
    if times.len() < 2 {
        return None;
    }

    let mut bpms = Vec::new();
    for window in times.windows(2) {
        let diff = (window[1] - window[0]).max(0.0);
        if diff < MIN_INTERVAL_SECONDS || diff > MAX_INTERVAL_SECONDS {
            continue;
        }
        let mut bpm = 60.0 / diff.max(0.0001);
        while bpm < 60.0 {
            bpm *= 2.0;
        }
        while bpm > 240.0 {
            bpm *= 0.5;
        }
        bpms.push(bpm);
    }

    if bpms.len() < MIN_NOTES_FOR_TEMPO {
        return None;
    }

    bpms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let median_bpm = if bpms.len() % 2 == 0 {
        (bpms[bpms.len() / 2 - 1] + bpms[bpms.len() / 2]) / 2.0
    } else {
        bpms[bpms.len() / 2]
    };

    let beat = 60.0 / median_bpm.max(0.0001);
    Some(beat.clamp(0.25, 1.5))
}

fn update_timeline_window(timeline: &mut StringTimelineFeed, current_time: f32) {
    let block_duration = clamp_block_duration(if timeline.block_duration > 0.0 {
        timeline.block_duration
    } else {
        default_block_duration()
    });
    timeline.block_duration = block_duration;

    let window_length = block_duration * visible_block_count() as f32;
    let current_block_index = (current_time / block_duration).floor().max(0.0) as i32;
    let window_start_block = (current_block_index - 1).max(0);
    let window_start = window_start_block as f32 * block_duration;

    timeline.window_start = window_start;
    timeline.window_end = window_start + window_length;
}

fn select_charts_up_to<'a>(tab: &'a StringTab, difficulty_percent: f32) -> Vec<&'a TabNoteChart> {
    if tab.note_charts.is_empty() {
        return Vec::new();
    }

    let mut charts: Vec<&TabNoteChart> = tab.note_charts.iter().collect();
    charts.sort_by_key(|chart| chart.difficulty);

    let min_difficulty = charts
        .iter()
        .map(|chart| chart.difficulty)
        .min()
        .unwrap_or(0);
    let max_difficulty = charts
        .iter()
        .map(|chart| chart.difficulty)
        .max()
        .unwrap_or(min_difficulty);

    let clamped = difficulty_percent.clamp(0.0, 100.0);
    let mut threshold = if max_difficulty <= 0 {
        min_difficulty
    } else {
        ((max_difficulty as f32) * (clamped / 100.0)).ceil() as i32
    };
    if threshold < min_difficulty {
        threshold = min_difficulty;
    }

    charts
        .into_iter()
        .filter(|chart| chart.difficulty <= threshold)
        .collect()
}
