use bevy::prelude::*;

pub mod song_selection;

pub use song_selection::setup_song_select;

pub fn setup_camera(mut commands: Commands){
    commands.spawn(Camera2d::default());
}