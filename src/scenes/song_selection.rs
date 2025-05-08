use bevy::prelude::*;
use bevy::ui::{BackgroundColor, Node, Overflow, Val, FlexDirection, AlignItems, AlignSelf, UiRect};
use crate::core::theme::{Themes, Theme};

pub fn setup_song_select(mut commands: Commands, themes: Res<Themes>){
    // let theme = themes.get("default").expect("Theme 'default' not found");
    // commands.spawn((
    //     Node {
    //         width: Val::Percent(100.0),
    //         height: Val::Percent(100.0),
    //         padding: UiRect::all(Val::Px(20.0)),
    //         flex_direction: FlexDirection::Column,
    //         align_items: AlignItems::Stretch,
    //         ..default()
    //     },
    //     BackgroundColor(theme.background),
    // )).with_children(|parent| {
    //     parent.spawn((
    //         Node {
    //             flex_direction: FlexDirection::Column,
    //             align_self: AlignSelf::Stretch,
    //             height: Val::Percent(50.),
    //             overflow: Overflow::scroll_y(),
    //             ..default() 
    //         },
    //         BackgroundColor(theme.container_background),
    //     )).with_children(|list| {
    //         for i in 0..20 {
    //             let label = format!("Button {}", i);
    //         }
    //     });
    // });
}