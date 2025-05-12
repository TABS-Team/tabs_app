use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{self, Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug)]
pub struct Song {
    pub folder: PathBuf,
    pub metadata: SongMetadata,
}

#[derive(Debug, Deserialize)]
pub struct SongMetadata{
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
    pub fn find_all(root_folder: &Path, results: &mut Vec<Song>) {
        let full_root = Path::new("assets").join(root_folder);
        if let Ok(entries) = fs::read_dir(full_root) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    let metadata_path = path.join("metadata.json");
                    match fs::read_to_string(&metadata_path) {
                        Ok(json_str) => match serde_json::from_str::<SongMetadata>(&json_str) {
                            Ok(metadata) => {
                                let relative_path = path
                                    .strip_prefix("assets")
                                    .unwrap_or(&path)
                                    .to_path_buf();
                                results.push(Song {
                                    folder: relative_path,
                                    metadata,
                                });
                            }
                            Err(e) => {
                                eprintln!("Failed to parse {}: {}", metadata_path.display(), e);
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to read {}: {}", metadata_path.display(), e);
                        }
                    }
                }
            }
        }
    }
}


fn arrangement_keys_only<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: HashMap<String, Value> = HashMap::deserialize(deserializer)?;
    Ok(map.into_keys().collect())
}

