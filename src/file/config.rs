use serde::Deserialize;
use bevy::prelude::*;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Resource)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub paths: PathConfig,
    pub saves: SaveConfig,
}

#[derive(Debug, Deserialize, Resource)]
pub struct WindowConfig {
    pub title: String,
}

#[derive(Debug, Deserialize, Resource)]
pub struct PathConfig {
    pub song_directory: String,
}

#[derive(Debug, Deserialize, Resource)]
pub struct SaveConfig {
    pub directory: String,
    pub theme_file: String,
    pub settings_file: String,
}

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let mut config = load_config("tabs.cfg");
        let save_path = get_save_directory(&config.saves.directory);
        if !save_path.exists() {
            fs::create_dir_all(&save_path).expect("Failed to create save directory");
        }
        config.saves.directory = save_path.into_os_string().into_string().unwrap();
        app.insert_resource(config);
    }
}

fn load_config(path: &str) -> AppConfig {
    let content = fs
        ::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read config file at: {path}"));

    serde_yaml::from_str(&content).unwrap_or_else(|e| panic!("Failed to parse YAML: {e}"))
}

fn get_save_directory(save_dir: &String) -> PathBuf {
    let mut path = dirs::config_dir().expect("Could not find local data directory");
    path.push(save_dir);
    path
}
