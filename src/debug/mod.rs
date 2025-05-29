use bevy::{ prelude::*, diagnostic::{ FrameTimeDiagnosticsPlugin } };

use crate::states::AppState;

pub mod fps_counter;

pub struct DebugPlugin;

use crate::widgets::UiLayer;

#[derive(Resource, Debug, Clone)]
pub struct DebugCamera {
    pub entity: Entity,
}

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Update, fps_counter::update_fps_text)
            .add_systems(OnExit(AppState::InitialLoad), setup_debug_camera)
            .add_systems(OnEnter(AppState::Startup), fps_counter::spawn_fps_counter);
    }
}

fn setup_debug_camera(mut commands: Commands) {
    let camera_entity = commands
        .spawn((
            Camera2d,
            Camera {
                order: UiLayer::Debug.base_camera_order(),
                ..default()
            },
        ))
        .id();
    commands.insert_resource(DebugCamera { entity: camera_entity });
}
