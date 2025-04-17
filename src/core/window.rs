use bevy::prelude::*;
use bevy::window::{WindowResolution, PrimaryWindow};

#[derive(Resource)]
pub struct WindowConfig {
    pub title: String,
    pub dimensions: (f32, f32),
    pub primary_window: bool,
}

pub fn spawn_window(mut commands: Commands, config: &WindowConfig,) {
    let window = Window {
        title: config.title.clone(),
        resolution: WindowResolution::new(config.dimensions.0, config.dimensions.1),
        ..default()
    };

    if config.primary_window {
        commands.spawn((window, PrimaryWindow));
    } else {
        commands.spawn(window);
    }
}