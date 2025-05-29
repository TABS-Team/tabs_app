use thiserror::Error;
use std::fs;
use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use std::io;
use serde::{ self, Deserialize, Deserializer };
use serde_json::Value;

use bevy::{
    prelude::*,
    asset::{ Asset, Handle, LoadContext, AssetLoader, io::Reader, ReadAssetBytesError },
    reflect::TypePath,
    tasks::BoxedFuture,
};

#[derive(Asset, TypePath, Debug)]
pub struct Song {
    pub metadata: SongMetadata,
    pub album_art: Handle<Image>,
    pub audio_preview: Handle<AudioSource>,
}

#[derive(Debug, Deserialize)]
pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: i32,
    pub length: f32,
    #[serde(deserialize_with = "arrangement_keys_only")]
    pub arrangements: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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
                    let metadata_path = path.join("metadata.json");
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

fn arrangement_keys_only<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
    where D: Deserializer<'de>
{
    let map: HashMap<String, Value> = HashMap::deserialize(deserializer)?;
    Ok(map.into_keys().collect())
}

#[derive(Default)]
pub struct SongLoader;

#[derive(Debug, Error)]
pub enum SongLoaderError {
    #[error("I/O error while loading asset: {0}")] Io(#[from] std::io::Error),

    #[error("Failed to parse metadata.json: {0}")] Json(#[from] serde_json::Error),

    #[error("Missing parent directory for asset path")]
    MissingParentDirectory,

    #[error("Failed to read asset bytes: {0}")] ReadBytes(#[from] ReadAssetBytesError),
}

impl AssetLoader for SongLoader {
    type Asset = Song;
    type Settings = ();
    type Error = SongLoaderError;

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>
    ) -> Result<Self::Asset, Self::Error> {
        let metadata_path: PathBuf = load_context.path().to_path_buf();
        let metadata_bytes = load_context.read_asset_bytes(metadata_path).await?;
        let metadata: SongMetadata = serde_json::from_slice(&metadata_bytes)?;

        let folder = load_context
            .path()
            .parent()
            .ok_or(SongLoaderError::MissingParentDirectory)?
            .to_path_buf();

        let album_art: Handle<Image> = load_context
            .loader()
            .load::<Image>(folder.join("album_art.png"));
        let audio_preview: Handle<AudioSource> = load_context
            .loader()
            .load::<AudioSource>(folder.join("preview.ogg"));

        Ok(Song {
            metadata,
            album_art,
            audio_preview,
        })
    }
}
