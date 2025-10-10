use crate::widgets::UiContext;
use bevy::{
    prelude::*,
    ui::{BackgroundColor, Overflow},
};

const LINE_HEIGHT: f32 = 20.0;

#[derive(Event)]
pub struct ScrollbarMovedEvent {
    pub scrollbar_entity: Entity,
}

pub struct ScrollContainerPlugin;

impl Plugin for ScrollContainerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ScrollbarMovedEvent>()
            .add_systems(Update, update_scrollbar_height)
            .add_systems(Update, sync_scrollbar_to_content);
    }
}

#[derive(Clone)]
pub struct ScrollContainerStyle {
    pub width: Val,
    pub height: Val,
    pub background_color: Color,
    pub scrollbar_color: Color,
    pub scrollbar_width: f32,
    pub margin: UiRect,
    pub padding: UiRect,
}

impl Default for ScrollContainerStyle {
    fn default() -> Self {
        ScrollContainerStyle {
            width: Val::Percent(100.0),
            height: Val::Auto,
            background_color: Color::WHITE,
            scrollbar_color: Color::srgb(0.5, 0.5, 0.5),
            scrollbar_width: 6.0,
            margin: UiRect::all(Val::Px(0.0)),
            padding: UiRect::all(Val::Px(0.0)),
        }
    }
}

pub struct ScrollContainerBuilder {
    style: ScrollContainerStyle,
}

impl ScrollContainerBuilder {
    pub fn new() -> Self {
        ScrollContainerBuilder {
            style: ScrollContainerStyle::default(),
        }
    }
    pub fn style(mut self, style: ScrollContainerStyle) -> Self {
        self.style = style;
        self
    }
    pub fn build(self) -> ScrollContainer {
        ScrollContainer {
            scrollbar_entity: Entity::PLACEHOLDER,
            style: self.style,
        }
    }
}

#[derive(Component, Clone)]
pub struct ScrollContainer {
    pub scrollbar_entity: Entity,
    pub style: ScrollContainerStyle,
}

#[derive(Component)]
pub struct ScrollBar {
    pub max_scroll_offset: f32,
    pub total_content_height: f32,
    pub viewport_height: f32,
    pub scrollbar_height: f32,
    pub scroll_content_entity: Entity,
}

#[derive(Component)]
pub struct ScrollContent;

impl ScrollContainer {
    pub fn builder() -> ScrollContainerBuilder {
        ScrollContainerBuilder {
            style: ScrollContainerStyle::default(),
        }
    }
    pub fn new(scrollbar_entity: Entity, style: &ScrollContainerStyle) -> Self {
        ScrollContainer {
            scrollbar_entity,
            style: style.clone(),
        }
    }
    pub fn spawn<F: FnOnce(&mut ChildSpawnerCommands)>(
        &mut self,
        commands: &mut ChildSpawnerCommands,
        _ui_context: &UiContext,
        children: F,
    ) -> Entity {
        let content_node = commands
            .spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                margin: self.style.margin,
                padding: self.style.padding,
                ..default()
            })
            .insert(ScrollContent)
            .with_children(|content| {
                children(content);
            })
            .id();
        let scrollbar_thumb = commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                right: Val::Px(2.0),
                top: Val::Px(2.0),
                width: Val::Px(self.style.scrollbar_width),
                height: Val::Percent(0.0),
                ..default()
            })
            .insert(BackgroundColor(self.style.scrollbar_color))
            .insert(ScrollBar::new(content_node))
            .id();
        self.scrollbar_entity = scrollbar_thumb;
        let container_node = commands
            .spawn(Node {
                width: self.style.width,
                height: self.style.height,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .insert(BackgroundColor(self.style.background_color))
            .insert(self.clone())
            .add_children(&[content_node, scrollbar_thumb])
            .id();
        ScrollBar::register_scrollbar_drag_observers(scrollbar_thumb, commands.commands_mut());
        ScrollContainer::register_scrollwheel_observers(container_node, commands.commands_mut());
        container_node
    }

    fn register_scrollwheel_observers(container_entity: Entity, commands: &mut Commands) {
        commands.entity(container_entity).observe(
            move |mut trigger: Trigger<Pointer<Scroll>>,
                  scroll_container_query: Query<&ScrollContainer>,
                  mut scrollbar_query: Query<(&mut Node, &ScrollBar)>,
                  mut scrollbar_event_writer: EventWriter<ScrollbarMovedEvent>| {
                let scroll_delta = trigger.event().y * LINE_HEIGHT;
                if let Ok(scroll_container) = scroll_container_query.get(trigger.target()) {
                    if let Ok((thumb_node, scrollbar)) =
                        scrollbar_query.get_mut(scroll_container.scrollbar_entity)
                    {
                        if matches!(thumb_node.height, Val::Px(h) if h <= 0.0)
                            || matches!(thumb_node.height, Val::Percent(p) if p <= 0.0)
                        {
                            return;
                        }
                        ScrollBar::move_thumb(thumb_node, scrollbar, scroll_delta);
                        scrollbar_event_writer.write(ScrollbarMovedEvent {
                            scrollbar_entity: scroll_container.scrollbar_entity,
                        });
                        trigger.propagate(false);
                    }
                }
            },
        );
    }
}

impl ScrollBar {
    pub fn new(scroll_content_entity: Entity) -> Self {
        ScrollBar {
            max_scroll_offset: 0.0,
            total_content_height: 0.0,
            viewport_height: 0.0,
            scrollbar_height: 0.0,
            scroll_content_entity,
        }
    }
    pub fn register_scrollbar_drag_observers(scrollbar_entity: Entity, commands: &mut Commands) {
        commands.entity(scrollbar_entity).observe(
            move |mut trigger: Trigger<Pointer<Drag>>,
                  mut scrollbar_query: Query<(&mut Node, &ScrollBar)>,
                  mut scrollbar_event_writer: EventWriter<ScrollbarMovedEvent>| {
                let drag_delta = trigger.event().delta.y;
                let thumb_entity = trigger.target();
                if let Ok((thumb_node, scrollbar)) = scrollbar_query.get_mut(thumb_entity) {
                    ScrollBar::move_thumb(thumb_node, scrollbar, -drag_delta);
                    scrollbar_event_writer.write(ScrollbarMovedEvent {
                        scrollbar_entity: thumb_entity,
                    });
                    trigger.propagate(false);
                }
            },
        );
    }
    fn move_thumb(mut thumb_node: Mut<Node>, scrollbar: &ScrollBar, delta_y: f32) {
        let current_top = if let Val::Px(pos) = thumb_node.top {
            pos
        } else {
            0.0
        };
        let clamped_top = (current_top - delta_y).clamp(0.0, scrollbar.max_scroll_offset);
        thumb_node.top = Val::Px(clamped_top);
    }
}

pub fn update_scrollbar_height(
    query: Query<(&Children, &ComputedNode), (With<ScrollContainer>, Changed<ComputedNode>)>,
    computed_node_query: Query<&ComputedNode>,
    content_query: Query<(), With<ScrollContent>>,
    mut scrollbar_event_writer: EventWriter<ScrollbarMovedEvent>,
    mut scrollbar_query: Query<(&mut Node, &mut ScrollBar)>,
) {
    for (child_entities, container_computed) in &query {
        let viewport_height = container_computed.size.y;
        let mut total_height = 0.0;
        for child in child_entities.iter() {
            if content_query.get(child).is_ok() {
                if let Ok(child_computed) = computed_node_query.get(child) {
                    total_height += child_computed.size.y;
                }
            }
        }
        if total_height == 0.0 {
            continue;
        }
        let height_ratio = (viewport_height / total_height).clamp(0.05, 1.0);
        let percent_height = height_ratio * 100.0;
        for child in child_entities.iter() {
            if let Ok((mut thumb_node, mut scrollbar)) = scrollbar_query.get_mut(child) {
                let thumb_height = viewport_height * height_ratio;
                thumb_node.height = if percent_height == 100.0 {
                    Val::Px(0.0)
                } else {
                    Val::Percent(percent_height)
                };
                scrollbar.max_scroll_offset = viewport_height - thumb_height;
                scrollbar.scrollbar_height = thumb_height;
                scrollbar.viewport_height = viewport_height;
                scrollbar.total_content_height = total_height;
                let current_top = if let Val::Px(t) = thumb_node.top {
                    t
                } else {
                    0.0
                };
                if current_top + thumb_height > viewport_height {
                    thumb_node.top = Val::Px(viewport_height - thumb_height);
                }
                scrollbar_event_writer.write(ScrollbarMovedEvent {
                    scrollbar_entity: child,
                });
            }
        }
    }
}

pub fn sync_scrollbar_to_content(
    mut scrollbar_move_events: EventReader<ScrollbarMovedEvent>,
    mut node_query: Query<&mut Node>,
    scrollbar_query: Query<&ScrollBar>,
) {
    for event in scrollbar_move_events.read() {
        if let Ok(scrollbar) = scrollbar_query.get(event.scrollbar_entity) {
            if let Ok(thumb_node) = node_query.get_mut(event.scrollbar_entity) {
                let scroll_ratio = if scrollbar.max_scroll_offset != 0.0 {
                    if let Val::Px(top) = thumb_node.top {
                        top / scrollbar.max_scroll_offset
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                let max_overflow =
                    (scrollbar.total_content_height - scrollbar.viewport_height).max(0.0);
                let content_offset = if max_overflow > 0.0 {
                    -max_overflow * scroll_ratio
                } else {
                    0.0
                };
                if let Ok(mut content_node) = node_query.get_mut(scrollbar.scroll_content_entity) {
                    content_node.top = Val::Px(content_offset);
                }
            }
        }
    }
}
