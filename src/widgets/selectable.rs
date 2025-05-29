use std::collections::HashMap;

use bevy::prelude::*;
use bevy::window::{ SystemCursorIcon };
use bevy::winit::cursor::CursorIcon;
use crate::widgets::{ UiContext, UiBorder, ButtonStyle, ButtonType, GenericButton, Active };

#[derive(Event)]
pub struct SelectedEvent {
    pub id: String,
    pub entity: Entity,
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
        default_selected: &Vec<usize>
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
                BorderColor(self.style.border.color),
                self.style.border.radius,
            ))
            .with_children(|container| {
                let mut entities: Vec<Entity> = vec![];
                let mut entity_comps: Vec<SelectableItem> = vec![];
                for (i, button) in self.buttons.iter().enumerate() {
                    let btn_comp = SelectableItem { id: button.id.clone(), selected: false };
                    let btn_entity = GenericButton::builder(button.button_type.clone())
                        .style(self.style.button_style.clone())
                        .spawn(container, ctx);

                    entity_comps.push(SelectableItem { id: button.id.clone(), selected: false });
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
                    container.commands().entity(btn_entity).insert(border_radius);
                    entities.push(btn_entity);
                    Selectable::register_observers(btn_entity, &mut container.commands_mut());
                }

                for index in &self.default_selected {
                    container.commands_mut().entity(entities[*index]).insert(Active);
                    entity_comps[*index].selected = true;
                }

                for (i, entity) in entities.iter().enumerate() {
                    container.commands_mut().entity(*entity).insert(entity_comps[i].clone());
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
        default_selected: &Vec<usize>
    ) -> SelectableBuilder {
        SelectableBuilder::new(selectable_type, buttons, default_selected)
    }

    fn register_observers(entity: Entity, mut commands: &mut Commands) {
        commands
            .entity(entity)
            .observe(
                |
                    trigger: Trigger<Pointer<Click>>,
                    mut selectable_query: Query<&mut Selectable>,
                    parents: Query<&ChildOf>,
                    mut commands: Commands
                | {
                    let entity = trigger.target();
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

                        if let Some(mut is_selected) = selectable.selected.get_mut(&entity) {
                            let previous_select = is_selected.clone();
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
                }
            );
    }
}

pub fn active_added_listener(
    mut commands: Commands,
    query: Query<Entity, Added<Active>>,
    mut btn_query: Query<&mut GenericButton>,
    mut ev_writer: EventWriter<SelectedEvent>,
    selected_item_query: Query<&SelectableItem>
) {
    for entity in query.iter() {
        let Ok(item_comp) = selected_item_query.get(entity) else {
            return;
        };
        ev_writer.write(SelectedEvent {
            id: item_comp.id.clone(),
            entity: entity,
            selected: item_comp.selected,
        });
    }
}

pub fn active_removed_listener(
    mut commands: Commands,
    mut query: RemovedComponents<Active>,
    mut btn_query: Query<&mut GenericButton>,
    mut ev_writer: EventWriter<SelectedEvent>,
    selected_item_query: Query<&SelectableItem>
) {
    for entity in query.read() {
        let Ok(item_comp) = selected_item_query.get(entity) else {
            return;
        };
        ev_writer.write(SelectedEvent {
            id: item_comp.id.clone(),
            entity: entity,
            selected: item_comp.selected,
        });
    }
}
