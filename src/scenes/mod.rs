use bevy::prelude::*;

pub mod song_selection;

pub use song_selection::{ setup_song_select, check_song_assets_ready, setup_song_preview };

use crate::widgets::UiLayer;

#[derive(Resource, Debug, Clone)]
pub struct MainCamera {
    pub ui_camera: Entity,
    pub gameplay_camera: Entity,
}

pub fn setup_camera(mut commands: Commands) {
    let gameplay_camera = commands.spawn(Camera2d::default()).id();
    let main_ui_camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: UiLayer::Menus.base_camera_order(),
                clear_color: ClearColorConfig::Custom(Color::WHITE),
                ..default()
            },
        ))
        .id();
    commands.insert_resource(MainCamera {
        ui_camera: main_ui_camera,
        gameplay_camera: gameplay_camera,
    });
}
