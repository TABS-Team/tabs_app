use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use crate::core::config::AppConfig;
use std::path::{Path, PathBuf};
use crate::states::StartupLatch;

#[derive(Resource, Debug, Clone, Deserialize, Serialize)]
pub struct ThemeGroup {
    pub debug: DebugTheme,
}

#[derive(Resource, Debug, Clone, Deserialize, Serialize)]
pub struct DebugTheme {
    #[serde(with = "srgb_float")]
    pub background: Color,
    #[serde(with = "srgb_float")]
    pub text_color: Color,
    #[serde(with = "srgb_float")]
    pub titlebar_color: Color,
    #[serde(with = "srgb_float")]
    pub title_text_color: Color,
    #[serde(with = "srgb_float")]
    pub close_button_color: Color,
    #[serde(with = "srgb_float")]
    pub resize_handle_color: Color,
    #[serde(with = "srgb_float")]
    pub border_color: Color,
    pub border_thickness: f32,
    pub titlebar_padding: [f32; 4],
    pub title_font_size: f32,
    pub content_padding: [f32; 4],
    #[serde(with = "srgb_float")]
    pub scrollbar_color: Color,
    pub scrollbar_width: f32,
}

#[derive(Resource, Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    #[serde(with = "srgb_float")]
    pub background: Color,
    #[serde(with = "srgb_float")]
    pub text_color: Color,
    #[serde(with = "srgb_float")]
    pub action_button_color: Color,
    #[serde(with = "srgb_float")]
    pub action_button_hover_color: Color,
    #[serde(with = "srgb_float")]
    pub action_button_text_color: Color,
    #[serde(with = "srgb_float")]
    pub menu_button_text_color: Color,
    #[serde(with = "srgb_float")]
    pub menu_button_text_hover_color: Color,
    #[serde(with = "srgb_float")]
    pub container_background: Color,
    #[serde(with = "srgb_float")]
    pub container_border_color: Color,
    pub container_border_size: f32,
}

impl Theme {
    pub fn new_default() -> Self {
        Self {
            background: Color::srgb(0.05, 0.05, 0.05),
            text_color: Color::srgb(1.0, 1.0, 1.0),
            action_button_color: Color::srgb(1.0, 0.2, 0.2),
            action_button_hover_color: Color::srgb(0.3, 0.3, 0.3),
            action_button_text_color: Color::srgb(1.0, 1.0, 1.0),
            menu_button_text_color: Color::srgb(1.0, 1.0, 1.0),
            menu_button_text_hover_color: Color::srgb(0.2, 0.2, 0.2),
            container_background: Color::srgb(0.1, 0.1, 0.1),
            container_border_color: Color::srgb(0.2, 0.2, 0.2),
            container_border_size: 2.0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Resource)]
pub struct Themes {
    pub themes: HashMap<String, ThemeGroup>,
}

impl Themes {
    pub fn get(&self, name: &str) -> Option<&ThemeGroup> {
        self.themes.get(name)
    }
}

fn load_theme(path: &str) -> Themes {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read theme file at: {path}"));

    let parsed: Themes = serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse YAML: {e}"));

    parsed
}

pub fn setup_theme(mut commands: Commands, config: Res<AppConfig>, mut latch: ResMut<StartupLatch>,) {
    let theme_path = PathBuf::from(&config.saves.directory).join(&config.saves.theme_file);

    if !Path::new(&theme_path).exists() {
        warn!("Theme file not found at '{}', creating default theme file...", theme_path.display());
        let default_themes = create_default_themes();
        let yaml = serde_yaml::to_string(&default_themes).expect("Failed to serialize default themes");
        fs::write(&theme_path, yaml).expect("Failed to write default theme file");
    }

    let content = fs::read_to_string(&theme_path)
        .unwrap_or_else(|_| panic!("Failed to read theme file at: {}", theme_path.display()));

    let parsed: Themes = serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse theme YAML: {e}"));

    commands.insert_resource(parsed);
    latch.theme_loaded = true;
}

fn create_default_themes() -> Themes {
    let mut themes = HashMap::new();

    themes.insert("default".to_string(), ThemeGroup {
        debug: DebugTheme {
            background: Color::srgb(0.1, 0.1, 0.1),
            text_color: Color::srgb(1.0, 1.0, 1.0),
            titlebar_color: Color::srgb(0.2, 0.2, 0.25),
            title_text_color: Color::srgb(1.0, 1.0, 1.0),
            close_button_color: Color::srgb(0.8, 0.1, 0.1),
            resize_handle_color: Color::srgb(0.66, 0.66, 0.66),
            border_color: Color::srgb(0.2, 0.2, 0.2),
            border_thickness: 2.0,
            titlebar_padding: [8.0, 8.0, 0.0, 0.0],
            title_font_size: 12.0,
            content_padding: [4.0, 4.0, 0.0, 0.0],
            scrollbar_color: Color::srgb(0.3, 0.3, 0.5),
            scrollbar_width: 6.0,
        },
    });

    Themes { themes }
}

mod srgb_float {
    use bevy::prelude::Color;
    use serde::de::{Deserializer};
    use serde::ser::{SerializeSeq, Serializer};
    use serde::{Deserialize};

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let srgba = color.to_srgba();
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&srgba.red)?;
        seq.serialize_element(&srgba.green)?;
        seq.serialize_element(&srgba.blue)?;
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let rgb: [f32; 3] = <[f32; 3]>::deserialize(deserializer)?;
        Ok(Color::srgb(rgb[0], rgb[1], rgb[2]))
    }
}