use crate::widgets::{UiBorder, UiContext, UiIcon};
use bevy::prelude::*;
use bevy::window::SystemCursorIcon;
use bevy::winit::cursor::CursorIcon;

#[derive(Component, Clone, Copy, Debug)]
pub struct UiButton;

pub fn default_button_setup(mut commands: Commands, query: Query<Entity, Added<UiButton>>) {
    for entity in query.iter() {
        commands.entity(entity).observe(
            move |_: Trigger<Pointer<Over>>, mut cmds: Commands, ctx: UiContext| {
                cmds.entity(*ctx.window)
                    .insert(CursorIcon::System(SystemCursorIcon::Pointer));
            },
        );

        commands.entity(entity).observe(
            move |_: Trigger<Pointer<Out>>, mut cmds: Commands, ctx: UiContext| {
                cmds.entity(*ctx.window)
                    .insert(CursorIcon::System(SystemCursorIcon::Default));
            },
        );
    }
}

#[derive(Clone, Debug)]
pub enum ButtonType {
    Labeled(String),
    Icon(UiIcon),
}

#[derive(Component, Clone, Copy, Debug)]
pub struct GenericButton {
    pub current_color: Color,
    pub color: Color,
    pub hover_color: Color,
    pub press_color: Color,
    pub stay_active: bool,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Active;

pub struct ButtonBuilder {
    label: ButtonType,
    style: ButtonStyle,
    stay_active: bool,
}

#[derive(Clone, Debug)]
pub struct ButtonStyle {
    pub stretch: bool,
    pub color: Color,
    pub press_color: Color,
    pub hover_color: Color,
    pub label_color: Color,
    pub font_size: f32,
    pub border: Option<UiBorder>,
    pub box_shadow: Option<BoxShadow>,
    pub padding: UiRect,
    pub margin: UiRect,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        ButtonStyle {
            stretch: false,
            color: Color::srgb(0.65, 0.65, 0.65),
            press_color: Color::srgb(0.55, 0.55, 0.55),
            hover_color: Color::srgb(0.75, 0.75, 0.75),
            label_color: Color::BLACK,
            font_size: 16.0,
            border: None,
            box_shadow: None,
            padding: UiRect::all(Val::Px(0.0)),
            margin: UiRect::all(Val::Px(0.0)),
        }
    }
}

impl ButtonBuilder {
    pub fn new(label: ButtonType) -> Self {
        ButtonBuilder {
            label: label,
            style: ButtonStyle::default(),
            stay_active: false,
        }
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn stay_active(mut self, should_stay_active: bool) -> Self {
        self.stay_active = should_stay_active;
        self
    }

    pub fn spawn(&self, commands: &mut ChildSpawnerCommands, ctx: &UiContext) -> Entity {
        let width = if self.style.stretch {
            Val::Percent(100.0)
        } else {
            Val::Auto
        };
        let mut node = Node {
            width: width,
            height: Val::Auto,
            padding: self.style.padding,
            margin: self.style.margin,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        };
        let entity = commands
            .spawn((
                BackgroundColor(self.style.color),
                GenericButton {
                    current_color: self.style.color,
                    color: self.style.color,
                    hover_color: self.style.hover_color,
                    press_color: self.style.press_color,
                    stay_active: self.stay_active,
                },
                UiButton,
            ))
            .id();

        if let Some(border) = &self.style.border {
            node.border = border.size;
            commands
                .commands()
                .entity(entity)
                .insert((BorderColor(border.color), border.radius));
        }

        if let Some(box_shadow) = &self.style.box_shadow {
            commands
                .commands()
                .entity(entity)
                .insert(box_shadow.clone());
        }

        let text_comp = match &self.label {
            ButtonType::Labeled(ref label) => Text::new(label),
            ButtonType::Icon(ref icon) => {
                let glyph = icon.glyph(&ctx.icons).unwrap_or_else(|| {
                    warn!("Material icon '{}' not found", icon.name());
                    "?"
                });
                Text::new(glyph)
            }
        };

        let text_font_comp = match &self.label {
            ButtonType::Labeled(label) => TextFont {
                font_size: self.style.font_size,
                ..default()
            },
            ButtonType::Icon(_icon) => {
                let icon_font = ctx.asset_server.load("fonts/GoogleMaterialIcons.ttf");
                TextFont {
                    font_size: self.style.font_size,
                    font: icon_font,
                    ..default()
                }
            }
        };

        commands
            .commands()
            .entity(entity)
            .insert(node)
            .with_children(|container| {
                container.spawn((text_comp, text_font_comp, TextColor(self.style.label_color)));
            });

        GenericButton::register_observers(entity, &mut commands.commands_mut());
        entity
    }
}

impl GenericButton {
    pub fn builder(label: ButtonType) -> ButtonBuilder {
        ButtonBuilder {
            stay_active: false,
            label: label,
            style: ButtonStyle::default(),
        }
    }

    fn register_observers(entity: Entity, mut commands: &mut Commands) {
        commands
            .entity(entity)
            .observe(
                |trigger: Trigger<Pointer<Over>>,
                 btn_query: Query<&GenericButton>,
                 mut commands: Commands| {
                    let entity = trigger.target();
                    if let Ok(btn_comp) = btn_query.get(entity) {
                        commands
                            .entity(entity)
                            .insert(BackgroundColor(btn_comp.hover_color));
                    }
                },
            )
            .observe(
                |trigger: Trigger<Pointer<Out>>,
                 btn_query: Query<&GenericButton>,
                 active_query: Query<&Active>,
                 mut commands: Commands| {
                    let entity = trigger.target();
                    if let Ok(btn_comp) = btn_query.get(entity) {
                        commands
                            .entity(entity)
                            .insert(BackgroundColor(btn_comp.current_color));
                    }
                },
            )
            .observe(
                |trigger: Trigger<Pointer<Pressed>>,
                 btn_query: Query<&GenericButton>,
                 mut commands: Commands| {
                    let entity = trigger.target();
                    if let Ok(btn_comp) = btn_query.get(entity) {
                        commands
                            .entity(entity)
                            .insert(BackgroundColor(btn_comp.press_color));
                    }
                },
            )
            .observe(
                |trigger: Trigger<Pointer<Released>>,
                 btn_query: Query<&GenericButton>,
                 active_query: Query<&Active>,
                 mut commands: Commands| {
                    let entity = trigger.target();
                    if let Ok(btn_comp) = btn_query.get(entity) {
                        commands
                            .entity(entity)
                            .insert(BackgroundColor(btn_comp.hover_color));

                        if btn_comp.stay_active {
                            if let Ok(active_comp) = active_query.get(entity) {
                                commands.entity(entity).remove::<Active>();
                            } else {
                                commands.entity(entity).insert(Active);
                            }
                        }
                    }
                },
            );
    }
}

// Add active component manually to a button if you want this effect or build it with stay_active(bool)
pub fn add_active_listener(
    mut commands: Commands,
    query: Query<Entity, Added<Active>>,
    mut btn_query: Query<&mut GenericButton>,
) {
    for entity in query.iter() {
        if let Ok(mut button_comp) = btn_query.get_mut(entity) {
            button_comp.current_color = button_comp.hover_color;
            commands
                .entity(entity)
                .insert(BackgroundColor(button_comp.current_color));
        }
    }
}

pub fn remove_active_listener(
    mut commands: Commands,
    mut removed: RemovedComponents<Active>,
    mut btn_query: Query<&mut GenericButton>,
) {
    for entity in removed.read() {
        if let Ok(mut button_comp) = btn_query.get_mut(entity) {
            button_comp.current_color = button_comp.color;
            commands
                .entity(entity)
                .insert(BackgroundColor(button_comp.current_color));
        }
    }
}
