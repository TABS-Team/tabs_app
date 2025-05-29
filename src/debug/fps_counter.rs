use bevy::{ prelude::*, diagnostic::{ DiagnosticsStore, FrameTimeDiagnosticsPlugin } };
use crate::states::AppState;
use crate::widgets::{
    UiWindow,
    UiContext,
    UiLayer,
    UiLayerStack,
    UiWindowStyle,
    ScrollContainer,
    ScrollContainerStyle,
};

use crate::debug::DebugCamera;

#[derive(Component)]
pub struct FpsText;

pub fn spawn_fps_counter(
    mut commands: Commands,
    ctx: UiContext,
    mut layer_stack: ResMut<UiLayerStack>,
    mut next_state: ResMut<NextState<AppState>>,
    debug_camera: Res<DebugCamera>
) {
    let theme = &ctx.themes
        .get(ctx.settings.start_theme.as_str())
        .expect("Theme 'default' not found");

    let debug_window_style = UiWindowStyle {
        background_color: theme.background_default,
        border_color: theme.third_light,
        title_color: theme.text_primary,
        titlebar_color: theme.secondary_dark,
        ..default()
    };

    UiWindow::builder("Debug Menu", UiLayer::Debug)
        .size(Val::Percent(30.0), Val::Px(200.0))
        .style(debug_window_style.clone())
        .resizable(true)
        .draggable(true)
        .closeable(true)
        .show_titlebar(true)
        .camera(debug_camera.entity)
        .build()
        .spawn(&mut commands, &ctx, &mut layer_stack, Val::Px(20.0), Val::Px(20.0), |parent| {
            ScrollContainer::builder()
                .style(ScrollContainerStyle {
                    background_color: theme.background_default,
                    scrollbar_color: theme.primary,
                    scrollbar_width: 6.0,
                    ..default()
                })
                .build()
                .spawn(parent, &ctx, |scroll_parent| {
                    scroll_parent
                        .spawn((
                            Text::new("FPS: "),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(Color::WHITE),
                        ))
                        .with_child((
                            TextSpan::default(),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(Color::WHITE),
                            FpsText,
                        ));
                });
        });

    next_state.set(AppState::SongSelect);
}

pub fn update_fps_text(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut TextSpan, With<FpsText>>
) {
    for mut span in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.average() {
                **span = format!("{value:.0}");
            }
        }
    }
}
