use bevy::prelude::*;
use std::path::Path;

use crate::file::{AppConfig, Settings, Song, Themes};
use crate::widgets::{UiLayer, UiWindow, UiContext, UiWindowStyle, Card, CardStyle};

pub fn setup_song_select(
    mut commands: Commands,
    mut ctx: UiContext,
    themes: Res<Themes>,
    settings: Res<Settings>,
    config: Res<AppConfig>,
    asset_server: Res<AssetServer>,
) {
    let mut songs = Vec::new();
    let root_dir = Path::new(&config.paths.song_directory);
    Song::find_all(root_dir, &mut songs);

    let theme = themes
        .get(&settings.start_theme)
        .expect("Theme not found");

    let mut cards = Vec::new();
    for song in &songs {
        let album_art_path = song.folder.join("album_art.png");
        let texture_handle: Handle<Image> = asset_server.load(album_art_path.to_str().unwrap());

        let card_entity = Card::builder(&song.metadata.title, &song.metadata.artist)
            .image(texture_handle)
            .style(CardStyle {
                background_color: theme.background_paper,
                text_color: theme.text_secondary,
                ..default()
            })
            .build()
            .spawn(&mut commands, &mut ctx, Val::Px(0.0), Val::Px(0.0), |_| {});

            cards.push(card_entity);
    }

    let song_select_style = UiWindowStyle {
        background_color: theme.background_default,
        background_color: Color::srgb(1.0, 0.0, 0.0),
        border_size: 0.0,
        ..Default::default()
    };

    let window = UiWindow::builder("Song Select", UiLayer::Overlay)
    .size(Val::Percent(100.0), Val::Percent(100.0))
    .style(song_select_style)
    .resizable(false)
    .draggable(false)
    .closeable(false)
    .show_titlebar(false)
    .build()
    .spawn(&mut commands, &mut ctx, Val::Px(0.0), Val::Px(0.0), |_| {});

    commands.entity(window).add_children(&cards);
}
