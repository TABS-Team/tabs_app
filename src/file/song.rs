use serde::{self, Deserialize, Deserializer};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use bevy::{
    asset::{io::Reader, Asset, AssetLoader, Handle, LoadContext, ReadAssetBytesError},
    prelude::*,
    reflect::TypePath,
};
use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

#[derive(Asset, TypePath, Debug)]
pub struct Song {
    pub metadata: SongMetadata,
    pub album_art: Handle<Image>,
    pub audio_preview: Handle<KiraAudioSource>,
}

#[derive(Asset, TypePath, Debug, Clone)]
pub enum Tab {
    Strings(StringTab),
    Vocals(VocalTab),
}

#[derive(Debug, Clone, Deserialize)]
pub struct StringTab {
    pub sections: Vec<TabSection>,
    pub chords: Vec<TabChord>,
    #[serde(default)]
    pub note_charts: Vec<TabNoteChart>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TabSection {
    pub name: String,
    pub start_time: f32,
    pub end_time: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TabChord {
    pub name: String,
    pub fingers: Vec<i32>,
    pub frets: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TabNoteChart {
    pub difficulty: i32,
    #[serde(default)]
    pub notes: Vec<TabNote>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TabNote {
    pub time: f32,
    #[serde(default)]
    pub techniques: Vec<Techniques>,
    pub chord_index: i32,
    pub string: i32,
    pub fret: i32,
    pub anchor_fret: i32,
    pub sustain: f32,
    pub slide_to: i32,
    pub slide_unpitch_to: i32,
    pub vibrato: i32,
    pub max_bend: f32,
    pub slap: i32,
    pub pluck: i32,
    pub tap: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VocalTab {
    pub vocals: Vec<VocalPhrase>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VocalPhrase {
    pub time: f32,
    pub length: f32,
    pub lyric: String,
}

#[derive(Debug, Deserialize)]
pub struct SongArrangementMetadata {
    pub name: String,
    pub capo_fret: Option<i32>,
    pub instrument: TabsInstrument,
    pub string_count: Option<i32>,
    pub string_semitone_offset: Option<Vec<i32>>,
    pub techniques: Vec<Techniques>,
}

#[derive(Debug, Deserialize)]
pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: i32,
    pub length: f32,
    pub arrangements: HashMap<String, SongArrangementMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Techniques {
    Slide,
    Bend,
    Tremolo,
    Harmonic,
    HammerOn,
    PullOff,
    PalmMute,
    Vibrato,
    Tap,
    Slap,
    Pop,
    PinchHarmonic,
    Chord,
    ChordNote,
    Arpeggio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum TabsInstrument {
    Guitar,
    Bass,
    Vocals,
}

impl Song {
    pub fn get_all_songs(root_folder: &Path) -> Vec<PathBuf> {
        let mut songs = Vec::new();
        let full_root = Path::new("assets").join(root_folder);
        if !full_root.exists() {
            warn!("Root folder does not exist: {}", full_root.display());
            return songs;
        }

        if !full_root.is_dir() {
            warn!("Root folder is not a directory: {}", full_root.display());
            return songs;
        }

        if let Ok(entries) = fs::read_dir(full_root) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    let metadata_path = path.join("song.metadata");
                    if metadata_path.exists() {
                        let relative_path = metadata_path
                            .strip_prefix("assets")
                            .unwrap_or(&metadata_path);
                        songs.push(relative_path.to_path_buf());
                    }
                }
            }
        }
        songs
    }
}

#[allow(dead_code)]
fn arrangement_keys_only<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: HashMap<String, Value> = HashMap::deserialize(deserializer)?;
    Ok(map.into_keys().collect())
}

#[derive(Default)]
pub struct SongLoader;

#[derive(Debug, Error)]
pub enum SongLoaderError {
    #[error("I/O error while loading asset: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse song.metadata: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Missing parent directory for asset path")]
    MissingParentDirectory,

    #[error("Failed to read asset bytes: {0}")]
    ReadBytes(#[from] ReadAssetBytesError),
}

impl AssetLoader for SongLoader {
    type Asset = Song;
    type Settings = ();
    type Error = SongLoaderError;

    fn extensions(&self) -> &[&str] {
        &[".metadata"]
    }
    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let metadata_path: PathBuf = load_context.path().to_path_buf();
        let metadata_bytes = load_context.read_asset_bytes(metadata_path).await?;
        let metadata: SongMetadata = serde_yaml::from_slice(&metadata_bytes)?;
        let folder = load_context
            .path()
            .parent()
            .ok_or(SongLoaderError::MissingParentDirectory)?
            .to_path_buf();

        let album_art: Handle<Image> = load_context
            .loader()
            .load::<Image>(folder.join("album_art.png"));
        let audio_preview_path = folder.join("preview.wav");
        let audio_preview: Handle<KiraAudioSource> = load_context
            .loader()
            .load::<KiraAudioSource>(audio_preview_path);

        Ok(Song {
            metadata,
            album_art,
            audio_preview,
        })
    }
}

#[derive(Default)]
pub struct TabLoader;

#[derive(Debug, Error)]
pub enum TabLoaderError {
    #[error("Failed to read asset bytes: {0}")]
    ReadBytes(#[from] ReadAssetBytesError),

    #[error("Failed to parse tab file: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Unsupported tab format in {0}")]
    UnsupportedFormat(String),
}

impl AssetLoader for TabLoader {
    type Asset = Tab;
    type Settings = ();
    type Error = TabLoaderError;

    fn extensions(&self) -> &[&str] {
        &[".tab"]
    }

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let tab_path: PathBuf = load_context.path().to_path_buf();
        let path_string = tab_path.to_string_lossy().to_string();
        let tab_bytes = load_context.read_asset_bytes(tab_path).await?;
        let document: Value = serde_yaml::from_slice(&tab_bytes)?;

        let vocals_key = Value::String("vocals".to_string());
        let sections_key = Value::String("sections".to_string());

        let tab_asset = match &document {
            Value::Mapping(map) if map.contains_key(&vocals_key) => {
                let vocal_tab: VocalTab = serde_yaml::from_slice(&tab_bytes)?;
                Tab::Vocals(vocal_tab)
            }
            Value::Mapping(map) if map.contains_key(&sections_key) => {
                let string_tab: StringTab = serde_yaml::from_slice(&tab_bytes)?;
                Tab::Strings(string_tab)
            }
            _ => return Err(TabLoaderError::UnsupportedFormat(path_string)),
        };

        Ok(tab_asset)
    }
}
