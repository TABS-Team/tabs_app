use bevy::prelude::*;
use bevy::ui::{BackgroundColor, Interaction};

#[derive(Component)]
pub struct UiButton {
    pub id: String,
}


pub fn spawn_button<F: FnOnce(&mut ChildBuilder)>(
    parent: &mut ChildBuilder,
    id: impl Into<String>,
    background: BackgroundColor,
    interaction: Option<Interaction>,
    content: F,
) {
    let mut entity = parent.spawn((
        Node::default(),
        background,
        UiButton { id: id.into() },
    ));

    if let Some(i) = interaction {
        entity.insert(i);
    }

    entity.with_children(content);
}