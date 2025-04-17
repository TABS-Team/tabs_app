use bevy::prelude::*;
use bevy::ui::{BackgroundColor, Node, Overflow, Val, FlexDirection, AlignItems, AlignSelf, UiRect};
use crate::core::widgets::spawn_button;

pub fn setup_song_select(mut commands: Commands){
    commands.spawn(Camera2d::default());
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(20.0)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            ..default()
        },
        BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
    )).with_children(|parent| {
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_self: AlignSelf::Stretch,
                height: Val::Percent(50.),
                overflow: Overflow::scroll_y(),
                ..default() 
            },
            BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
        )).with_children(|list| {
            for i in 0..20 {
                let label = format!("Button {}", i);

                spawn_button(
                    list,
                    format!("btn_{}", i),
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    Some(Interaction::default()),
                    |btn| {
                        btn.spawn((
                            Text::new(label.clone()),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 1.0, 1.0)),
                        ));
                    },
                );
            }
        });
    });
}