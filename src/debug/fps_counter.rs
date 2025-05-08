use bevy::{
    prelude::*,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}
};

use crate::widgets::{UiSize, UiWindow, UiWindowContext, UiLayer, UiWindowStyle};
use crate::core::{
    settings::Settings,
    theme::Themes,
};

#[derive(Component)]
pub struct FpsText;

pub fn spawn_fps_counter(mut ctx: UiWindowContext, themes: Res<Themes>, config: Res<Settings>) {
    let theme = &themes.get(config.start_theme.as_str()).expect("Theme 'default' not found");

    let debug_window_style = UiWindowStyle {
        background_color: theme.debug.background,
        border_color: theme.debug.border_color,
        border_size: theme.debug.border_thickness,
        title_font_size: theme.debug.title_font_size,
        title_color: theme.debug.title_text_color,
        titlebar_color: theme.debug.titlebar_color,
        titlebar_padding: theme.debug.titlebar_padding,
        content_padding: theme.debug.content_padding,
        scrollbar_color: theme.debug.scrollbar_color,
        scrollbar_width: theme.debug.scrollbar_width,
        ..default()
    };

    UiWindow::builder("Debug Menu", UiLayer::Overlay)
    .size(UiSize::Percent(30.0), UiSize::Px(200.0))
    .style(debug_window_style)
    .resizable(true)
    .draggable(true)
    .closeable(true)
    .show_titlebar(true)
    .build()
    .spawn(
        &mut ctx,
        UiSize::Px(20.0),
        UiSize::Px(20.0),
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