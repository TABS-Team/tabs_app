use std::collections::HashMap;

use crate::widgets::{Active, ButtonStyle, ButtonType, GenericButton, UiBorder, UiContext};
use bevy::picking::prelude::*;
use bevy::prelude::*;

#[derive(EntityEvent, Message)]
pub struct SelectedEvent {
    #[event_target]
    pub selectable: Entity,
    pub id: String,
    pub item: Entity,
    pub selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectableType {
    Checkbox,
    Radio,
}

#[derive(Component, Clone, Debug)]
pub struct Selectable {
    pub selected: HashMap<Entity, bool>,
    pub button_id_map: HashMap<String, Entity>,
    pub selectable_type: SelectableType,
}

#[derive(Component, Clone, Debug)]
pub struct SelectableItem {
    pub id: String,
    pub selected: bool,
}

pub struct SelectableStyle {
    pub border: UiBorder,
    pub flex_direction: FlexDirection,
    pub width: Val,
    pub button_style: ButtonStyle,
}

#[derive(Clone, Debug)]
pub struct SelectableButton {
    pub id: String,
    pub button_type: ButtonType,
}

impl Default for SelectableStyle {
    fn default() -> Self {
        SelectableStyle {
            border: UiBorder::default(),
            flex_direction: FlexDirection::Row,
            width: Val::Auto,
            button_style: ButtonStyle::default(),
        }
    }
}

pub struct SelectableBuilder {
    selectable_type: SelectableType,
    buttons: Vec<SelectableButton>,
    style: SelectableStyle,
    default_selected: Vec<usize>,
}

impl SelectableBuilder {
    pub fn new(
        selectable_type: SelectableType,
        buttons: &Vec<SelectableButton>,
        default_selected: &Vec<usize>,
    ) -> Self {
        SelectableBuilder {
            selectable_type: selectable_type,
            buttons: buttons.to_vec(),
            style: SelectableStyle::default(),
            default_selected: default_selected.clone(),
        }
    }

    pub fn style(mut self, style: SelectableStyle) -> Self {
        self.style = style;
        self
    }

    pub fn spawn(&mut self, commands: &mut ChildSpawnerCommands, ctx: &UiContext) -> Entity {
        self.style.button_style.stretch = true;
        if let Some(ref mut border) = &mut self.style.button_style.border {
            border.radius = BorderRadius::all(Val::Px(0.0));
        } else {
            self.style.button_style.border = Some(UiBorder {
                size: UiRect::all(Val::Px(0.0)),
                ..default()
            });
        }
        self.style.button_style.box_shadow = None;
        let mut comp = Selectable {
            selectable_type: self.selectable_type,
            selected: HashMap::new(),
            button_id_map: HashMap::new(),
        };
        let entity = commands
            .spawn((
                Node {
                    flex_direction: self.style.flex_direction,
                    justify_content: JustifyContent::SpaceBetween,
                    width: self.style.width,
                    border: self.style.border.size,
                    ..default()
                },
                BorderColor::all(self.style.border.color),
                self.style.border.radius,
            ))
            .with_children(|container| {
                let mut entities: Vec<Entity> = vec![];
                let mut entity_comps: Vec<SelectableItem> = vec![];
                for (i, button) in self.buttons.iter().enumerate() {
                    let btn_entity = GenericButton::builder(button.button_type.clone())
                        .style(self.style.button_style.clone())
                        .spawn(container, ctx);

                    entity_comps.push(SelectableItem {
                        id: button.id.clone(),
                        selected: false,
                    });
                    comp.selected.insert(btn_entity, false);
                    comp.button_id_map.insert(button.id.clone(), btn_entity);

                    // Border radius is currently bugged in Bevy, sometimes children do not inherit it
                    let border_radius = if i == 0 {
                        BorderRadius {
                            top_left: Val::Px(8.0),
                            bottom_left: Val::Px(8.0),
                            ..Default::default()
                        }
                    } else if i == self.buttons.len() - 1 {
                        BorderRadius {
                            top_right: Val::Px(8.0),
                            bottom_right: Val::Px(8.0),
                            ..Default::default()
                        }
                    } else {
                        BorderRadius::default()
                    };
                    container
                        .commands()
                        .entity(btn_entity)
                        .insert(border_radius);
                    entities.push(btn_entity);
                    Selectable::register_observers(btn_entity, &mut container.commands_mut());
                }

                for index in &self.default_selected {
                    container
                        .commands_mut()
                        .entity(entities[*index])
                        .insert(Active);
                    entity_comps[*index].selected = true;
                }

                for (i, entity) in entities.iter().enumerate() {
                    container
                        .commands_mut()
                        .entity(*entity)
                        .insert(entity_comps[i].clone());
                }
            })
            .insert(comp)
            .id();

        entity
    }
}

impl Selectable {
    pub fn builder(
        selectable_type: SelectableType,
        buttons: &Vec<SelectableButton>,
        default_selected: &Vec<usize>,
    ) -> SelectableBuilder {
        SelectableBuilder::new(selectable_type, buttons, default_selected)
    }

    fn register_observers(entity: Entity, commands: &mut Commands) {
        commands.entity(entity).observe(
            |trigger: On<Pointer<Click>>,
             mut selectable_query: Query<&mut Selectable>,
             parents: Query<&ChildOf>,
             mut commands: Commands| {
                let entity = trigger.entity;
                let Ok(parent) = parents.get(entity) else {
                    return;
                };
                if let Ok(mut selectable) = selectable_query.get_mut(parent.parent()) {
                    match selectable.selectable_type {
                        SelectableType::Radio => {
                            for (curr_entity, value) in selectable.selected.iter_mut() {
                                if entity != *curr_entity {
                                    *value = false;
                                    commands.entity(*curr_entity).remove::<Active>();
                                }
                            }
                        }
                        SelectableType::Checkbox => {}
                    }

                    let selectable_type = selectable.selectable_type;

                    if let Some(is_selected) = selectable.selected.get_mut(&entity) {
                        let previous_select = *is_selected;
                        if selectable_type == SelectableType::Checkbox {
                            *is_selected = !*is_selected;
                        } else {
                            *is_selected = true;
                        }
                        if *is_selected == previous_select {
                            return;
                        }

                        if *is_selected {
                            commands.entity(entity).insert(Active);
                        } else {
                            commands.entity(entity).remove::<Active>();
                        }
                    }
                }
            },
        );
    }
}

pub fn active_change_listener(
    mut commands: Commands,
    mut query: Query<(Entity, &ChildOf, Ref<Active>, &mut SelectableItem), Changed<Active>>,
) {
    for (entity, child_of, active_ref, mut item_comp) in &mut query {
        if active_ref.is_added() {
            item_comp.selected = true;

            let parent = child_of.parent();
            commands.trigger(SelectedEvent {
                selectable: parent,
                id: item_comp.id.clone(),
                item: entity,
                selected: item_comp.selected,
            });
        }
    }
}

pub fn active_removed_listener(
    mut commands: Commands,
    mut removed: RemovedComponents<Active>,
    selected_item_query: Query<&SelectableItem>,
    child_of_query: Query<&ChildOf>,
) {
    for entity in removed.read() {
        if let Ok(item_comp) = selected_item_query.get(entity) {
            if let Ok(child_of) = child_of_query.get(entity) {
                let parent = child_of.parent();
                commands.trigger(SelectedEvent {
                    selectable: parent,
                    id: item_comp.id.clone(),
                    item: entity,
                    selected: false,
                });
            }
        }
    }
}
