use bevy::{
    prelude::*,
    diagnostic::{FrameTimeDiagnosticsPlugin}
};

use crate::states::AppState;

pub mod fps_counter;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Update, fps_counter::update_fps_text)
            .add_systems(OnEnter(AppState::Startup), fps_counter::spawn_fps_counter)
        ;
    }
}