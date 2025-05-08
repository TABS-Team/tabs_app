use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::states::StartupLatch;
use crate::core::config::AppConfig;

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub start_theme: String,
    pub window: WindowSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: f32,
    pub height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start_theme: "default".to_string(),
            window: WindowSettings {
                width: 800.0,
                height: 600.0,
            },
        }
    }
}

pub fn load_or_create_settings(path: &PathBuf) -> Settings {
    if !path.exists() {
        warn!("Settings file not found at '{}', creating default...", path.display());
        let default = Settings::default();
        let yaml = serde_yaml::to_string(&default).expect("Failed to serialize default settings");
        fs::write(path, yaml).expect("Failed to write default settings file");
        return default;
    }

    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read settings file at '{}'", path.display()));

    serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse settings YAML: {e}"))
}

fn change_window(
    mut windows: Query<&mut Window>,
    settings: &Settings,
) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(settings.window.width, settings.window.height);
    } else {
        warn!("Primary window not available to apply settings");
    }
}

pub fn setup_settings(mut commands: Commands, windows: Query<&mut Window>, config: Res<AppConfig>, mut latch: ResMut<StartupLatch>,) {
    let path = PathBuf::from(&config.saves.directory).join(&config.saves.settings_file);

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect("Failed to create save directory");
        }
    }

    let settings = load_or_create_settings(&path);
    change_window(windows, &settings);
    commands.insert_resource(settings);
    latch.settings_loaded = true;
}