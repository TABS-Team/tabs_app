use bevy::{
    prelude::*,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}
};

use crate::states::{AppState};
use crate::widgets::{UiWindow, UiContext, UiLayer, UiWindowStyle};
use crate::file::{
    settings::Settings,
    theme::Themes,
};

#[derive(Component)]
pub struct FpsText;

pub fn spawn_fps_counter(mut commands: Commands, mut ctx: UiContext, themes: Res<Themes>, config: Res<Settings>, mut next_state: ResMut<NextState<AppState>>) {
    let theme = &themes.get(config.start_theme.as_str()).expect("Theme 'default' not found");

    let debug_window_style = UiWindowStyle {
        background_color: theme.background_default,
        border_color: theme.third,
        title_color: theme.text_primary,
        titlebar_color: theme.secondary,
        scrollbar_color: theme.primary,
        ..default()
    };

    UiWindow::builder("Debug Menu", UiLayer::Debug)
    .size(Val::Percent(30.0), Val::Px(200.0))
    .style(debug_window_style)
    .resizable(true)
    .draggable(true)
    .closeable(true)
    .show_titlebar(true)
    .build()
    .spawn(
        &mut commands,
        &mut ctx,
        Val::Px(20.0),
        Val::Px(20.0),
        |parent| {
            parent.spawn((
                Text::new("FPS: ",),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            )).with_child((
                TextSpan::default(),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                FpsText,
            ));

            for i in 0..20 {
                parent.spawn((Text::new(
                    format!("Debug Info Line {}", i + 1)),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ));
            }
        },
    );

    next_state.set(AppState::SongSelect);
    
}

pub fn update_fps_text(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut TextSpan, With<FpsText>>,
) {
    for mut span in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                **span = format!("{value:.0}");
            }
        }
    }
}