use bevy::prelude::*;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::fs;
use crate::file::config::AppConfig;
use std::path::{ Path, PathBuf };
use crate::states::StartupLatch;

#[derive(Resource, Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    #[serde(with = "srgb_float")]
    pub primary: Color,
    #[serde(with = "srgb_float")]
    pub secondary_light: Color,
    #[serde(with = "srgb_float")]
    pub third_light: Color,
    #[serde(with = "srgb_float")]
    pub secondary_dark: Color,
    #[serde(with = "srgb_float")]
    pub third_dark: Color,
    #[serde(with = "srgb_float")]
    pub text_primary: Color,
    #[serde(with = "srgb_float")]
    pub text_secondary: Color,
    #[serde(with = "srgb_float")]
    pub text_third: Color,
    #[serde(with = "srgb_float")]
    pub background_default: Color,
    #[serde(with = "srgb_float")]
    pub background_paper: Color,
    #[serde(with = "srgb_float")]
    pub divider: Color,
    #[serde(with = "srgb_float")]
    pub error_main: Color,
}

#[derive(Debug, Deserialize, Serialize, Resource)]
pub struct Themes {
    pub themes: HashMap<String, Theme>,
}

impl Themes {
    pub fn get(&self, name: &str) -> Option<&Theme> {
        self.themes.get(name)
    }
}

pub fn setup_theme(
    mut commands: Commands,
    config: Res<AppConfig>,
    mut latch: ResMut<StartupLatch>
) {
    let theme_path = PathBuf::from(&config.saves.directory).join(&config.saves.theme_file);

    if !Path::new(&theme_path).exists() {
        warn!("Theme file not found at '{}', creating default theme file...", theme_path.display());
        let default_themes = create_default_themes();
        let yaml = serde_yaml
            ::to_string(&default_themes)
            .expect("Failed to serialize default themes");
        fs::write(&theme_path, yaml).expect("Failed to write default theme file");
    }

    let content = fs
        ::read_to_string(&theme_path)
        .unwrap_or_else(|_| panic!("Failed to read theme file at: {}", theme_path.display()));

    let parsed: Themes = serde_yaml
        ::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse theme YAML: {e}"));

    commands.insert_resource(parsed);
    latch.theme_loaded = true;
}

fn create_default_themes() -> Themes {
    let mut themes = HashMap::new();

    themes.insert("default".to_string(), Theme {
        primary: Color::srgb(1.0, 0.7216, 0.0), // #ffb800
        secondary_light: Color::srgb(0.7686, 0.2627, 0.0706), // #C44312
        third_light: Color::srgb(0.5922, 0.7098, 0.7059), // #97B5B4
        secondary_dark: Color::srgb(0.0627, 0.0667, 0.0627), // #101110
        third_dark: Color::srgb(0.2235, 0.1765, 0.1961), // #392d32
        text_primary: Color::srgb(0.8196, 0.8118, 0.8118), // #d1cfcf
        text_secondary: Color::srgb(0.8196, 0.8118, 0.8118), // #d1cfcf
        text_third: Color::srgb(0.0471, 0.0471, 0.0471), // #0c0c0c
        background_default: Color::srgb(0.149, 0.1529, 0.1451), // #262725
        background_paper: Color::srgb(0.0627, 0.0667, 0.0627), // #101110
        divider: Color::srgb(0.8196, 0.8118, 0.8118), // #d1cfcf
        error_main: Color::srgb(0.9569, 0.2627, 0.2118), // #f44336
    });

    Themes { themes }
}

mod srgb_float {
    use bevy::prelude::Color;
    use serde::de::{ Deserializer };
    use serde::ser::{ SerializeSeq, Serializer };
    use serde::{ Deserialize };

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let srgba = color.to_srgba();
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&srgba.red)?;
        seq.serialize_element(&srgba.green)?;
        seq.serialize_element(&srgba.blue)?;
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
        where D: Deserializer<'de>
    {
        let rgb: [f32; 3] = <[f32; 3]>::deserialize(deserializer)?;
        Ok(Color::srgb(rgb[0], rgb[1], rgb[2]))
    }
}
