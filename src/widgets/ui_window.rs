use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor},
    ecs::{
        system::SystemParam,
        observer::ObservedBy
    },
    winit::{
        cursor::{CursorIcon},
        WinitWindows,
    },
    window::{SystemCursorIcon, PrimaryWindow, WindowRef},
    render::camera::RenderTarget,
};

use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::widgets::UiSize;
use crate::widgets::{UiLayer, UiLayerStack};
use crate::materials::{AbaaMaterial};

const LINE_HEIGHT: f32 = 20.0;

pub struct UiWindowPlugin;

impl Plugin for UiWindowPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, update_scrollbar_height)
            .add_event::<ScrollbarMovedEvent>()
            .add_event::<PromoteToOsWindowEvent>()
            .add_systems(Update, sync_scrollbar_to_content)
            .add_systems(Update, detect_os_window_reentry)
        ;
    }
}

#[derive(Bundle)]
pub struct UiWindowBundle {
    pub node: Node,
    pub background: BackgroundColor,
    pub border: BorderColor,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowLayer(pub UiLayer);

#[derive(Component)]
pub struct Titlebar {
    pub window_entity: Entity,
    pub title: String,
}

#[derive(Component)]
pub struct ScrollContainer{
    pub scroll_bar_entity: Entity,
}

#[derive(Component)]
pub struct ScrollBar {
    pub max_scroll_position: f32,
    pub content_height: f32,
    pub container_height: f32,
    pub scrollbar_height: f32,
    pub scroll_content_entity: Entity,
}

#[derive(Component)]
pub struct ScrollContent;

#[derive(Component)]
pub struct CloseButton {
    pub window_entity: Entity,
}

#[derive(Component)]
pub struct ResizeCorner {
    pub window_entity: Entity,
}

#[derive(Component)]
pub struct Footer;

#[derive(Component, Clone)]
pub struct UiWindow {
    pub title: String,
    pub width: UiSize,
    pub height: UiSize,
    pub layer: UiLayer,
    pub position: PositionType,
    pub style: UiWindowStyle,
    pub options: UiWindowOptions,
}

#[derive(Component)]
pub struct OsWindow{
    pub os_window_camera_entity: Entity,
    pub ui_window_entity: Entity,
}

impl OsWindow{
    fn new(camera: Entity, ui_window: Entity) -> Self{
        OsWindow{
            os_window_camera_entity: camera,
            ui_window_entity: ui_window,
        }
    }
}

#[derive(Default, Clone)]
pub struct UiWindowOptions {
    pub resizeable: bool,
    pub closeable: bool,
    pub draggable: bool,
    pub show_titlebar: bool,
}

#[derive(Clone)]
pub struct UiWindowStyle {
    pub background_color: Color,
    pub border_color: Color,
    pub border_size: f32,
    pub title_font_size: f32,
    pub title_color: Color,
    pub titlebar_color: Color,
    pub close_button_color: Color,
    pub resize_handle_color: Color,
    pub titlebar_padding: [f32; 4],
    pub content_padding: [f32; 4],
    pub scrollbar_color: Color,
    pub scrollbar_width: f32,
}

impl Default for UiWindowStyle {
    fn default() -> Self {
        UiWindowStyle {
            background_color: Color::srgb(0.1, 0.1, 0.1),
            border_color: Color::WHITE,
            border_size: 2.0,
            title_font_size: 14.0,
            title_color: Color::WHITE,
            close_button_color: Color::WHITE,
            resize_handle_color: Color::srgb(0.66, 0.66, 0.66),
            titlebar_color: Color::srgb(0.15, 0.15, 0.15),
            titlebar_padding: [6.0, 6.0, 6.0, 6.0],
            content_padding: [8.0, 8.0, 8.0, 8.0],
            scrollbar_color: Color::srgb(0.3, 0.3, 0.5),
            scrollbar_width: 6.0,
        }
    }
}

pub struct UiWindowBuilder {
    title: String,
    width: UiSize,
    height: UiSize,
    layer: UiLayer,
    position: PositionType,
    style: UiWindowStyle,
    options: UiWindowOptions,
}

#[derive(Event)]
pub struct ScrollbarMovedEvent {
    pub scrollbar: Entity,
}

#[derive(Event)]
pub struct PromoteToOsWindowEvent {
    pub window_entity: Entity,
    pub window_title: String,
}

#[derive(SystemParam)]
pub struct UiWindowContext<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub materials: ResMut<'w, Assets<AbaaMaterial>>,
    pub stack: ResMut<'w, UiLayerStack>,
    pub children_query: Query<'w, 's, &'static Children>,
    pub asset_server: Res<'w, AssetServer>
}

impl UiWindowBuilder {
    pub fn size(mut self, width: UiSize, height: UiSize) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn position(mut self, position: PositionType) -> Self {
        self.position = position;
        self
    }

    pub fn style(mut self, style: UiWindowStyle) -> Self {
        self.style = style;
        self
    }

    pub fn resizable(mut self, value: bool) -> Self {
        self.options.resizeable = value;
        self
    }

    pub fn draggable(mut self, value: bool) -> Self {
        self.options.draggable = value;
        self
    }

    pub fn closeable(mut self, value: bool) -> Self {
        self.options.closeable = value;
        self
    }

    pub fn show_titlebar(mut self, value: bool) -> Self {
        self.options.show_titlebar = value;
        self
    }

    pub fn build(self) -> UiWindow {
        UiWindow {
            title: self.title,
            width: self.width,
            height: self.height,
            layer: self.layer,
            position: self.position,
            style: self.style,
            options: self.options,
        }
    }
}

impl UiWindow {

    pub fn builder(title: impl Into<String>, layer: UiLayer) -> UiWindowBuilder {
        UiWindowBuilder {
            title: title.into(),
            width: UiSize::Percent(50.0),
            height: UiSize::Percent(50.0),
            layer,
            position: PositionType::Absolute,
            style: UiWindowStyle::default(),
            options: UiWindowOptions::default(),
        }
    }

    pub fn bundle(&self, left_margin: UiSize, top_margin: UiSize) -> UiWindowBundle {
        let width = match self.width {
            UiSize::Px(px) => Val::Px(px),
            UiSize::Percent(pct) => Val::Percent(pct),
        };

        let height = match self.height {
            UiSize::Px(px) => Val::Px(px),
            UiSize::Percent(pct) => Val::Percent(pct),
        };

        let left = match left_margin {
            UiSize::Px(px) => Val::Px(px),
            UiSize::Percent(pct) => Val::Percent(pct),
        };

        let top = match top_margin {
            UiSize::Px(px) => Val::Px(px),
            UiSize::Percent(pct) => Val::Percent(pct),
        };

        let position = match self.options.draggable{
            true => PositionType::Absolute,
            false => self.position
        };

        UiWindowBundle {
            node: Node {
                width,
                height,
                position_type: position,
                flex_direction: FlexDirection::Column,
                left,
                top,
                border: UiRect::all(Val::Px(self.style.border_size)),
                overflow: Overflow::clip_y(),
                ..default()
            },
            background: BackgroundColor(self.style.background_color),
            border: BorderColor(self.style.border_color),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
        }
    }

    pub fn spawn<F: FnOnce(&mut ChildSpawnerCommands)>(
        self,
        ctx: &mut UiWindowContext,
        left_margin: UiSize,
        top_margin: UiSize,
        children: F,
    ) -> Entity {

        let entity = ctx.commands
        .spawn(self.bundle(left_margin, top_margin))
        .insert(self.clone())
        .insert(WindowLayer(self.layer))
        .id();

        let mut modules: Vec<Entity> = Vec::new();

        if self.options.show_titlebar{
            let title_bar_entity = Titlebar::spawn(ctx, &self.title, &self.style, entity, self.options.closeable);
            modules.push(title_bar_entity);
            
            if self.options.draggable{
                Titlebar::register_observers(title_bar_entity, &mut ctx.commands);
            }
        }

        let scroll_container_entity = ScrollContainer::spawn(ctx, &self.style, children);
        modules.push(scroll_container_entity);

        if self.options.resizeable{
            let resize_corner = ResizeCorner::spawn(ctx, &self.style, entity);
            let footer = ctx.commands
            .spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Px(24.0),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(8.0)),
                ..default()
            })
            .insert(BackgroundColor(self.style.background_color))
            .insert(Footer)
            .add_children(&[resize_corner]).id();
            modules.push(footer);
        }

        ctx.commands.entity(entity).add_children(&modules);

        UiWindow::register_observers(entity, &mut ctx.commands);

        ctx.stack.push(self.layer, entity, &mut ctx.commands);
        entity
    }

    fn register_observers(
        entity: Entity, 
        commands: &mut Commands,
    ) {
        commands.entity(entity)
            .observe(
                |trigger: Trigger<Pointer<Pressed>>, 
                 mut commands: Commands, 
                 mut stack: ResMut<UiLayerStack>, 
                 layers: Query<&WindowLayer>| 
            {
                let clicked = trigger.target();

                if let Ok(layer) = layers.get(clicked) {
                    stack.bring_to_front(layer.0, clicked, &mut commands);
                }
            });

    }

    fn convert_to_os_window(
        window_entity: Entity,
        commands: &mut Commands,
        children_query: &Query<&Children>,
        scroll_container_query: &Query<&ScrollContainer>,
        titlebar_query: &Query<&Titlebar>,
        footer_query: &Query<&Footer>,
        computed_node_query: &Query<&ComputedNode>,
        transform_query: &Query<&GlobalTransform>,
        node_query: &mut Query<&mut Node>,
        winit_windows: &NonSend<WinitWindows>,
        primary_window_entity: Entity,
    ) {
        let Ok(children) = children_query.get(window_entity) else { return };
    
        let Some(scroll_container_entity) = children
            .iter()
            .find(|child| scroll_container_query.get(*child).is_ok()) else { return };
    
        let Ok(size) = computed_node_query.get(scroll_container_entity) else { return };
    
        if let Some(titlebar_entity) = children
            .iter()
            .find(|child| titlebar_query.get(*child).is_ok())
        {
            commands.entity(titlebar_entity).remove::<ObservedBy>();
            if let Ok(mut titlebar_node) = node_query.get_mut(titlebar_entity){
                titlebar_node.display = Display::None;
            }
        }
    
        if let Some(footer_entity) = children
            .iter()
            .find(|child| footer_query.get(*child).is_ok())
        {
            if let Ok(mut footer_node) = node_query.get_mut(footer_entity){
                footer_node.display = Display::None;
            }
        }
    
        let title = titlebar_query
            .iter()
            .find(|t| t.window_entity == window_entity)
            .map(|t| t.title.clone())
            .unwrap_or("Untitled".into());
        
        let Some(primary_window) = winit_windows.get_window(primary_window_entity) else {return};
        let primary_position = primary_window.outer_position().unwrap();
    
        let Ok(transform) = transform_query.get(window_entity) else { return };
        let translation = transform.translation();
        let ui_window_position = Vec2::new(translation.x, translation.y);
        let new_window_position = IVec2::new(
            primary_position.x + ui_window_position.x as i32,
            primary_position.y + ui_window_position.y as i32,
        );
    
        let new_window_entity = commands
            .spawn(Window {
                resolution: (size.size.x, size.size.y).into(),
                title,
                position: WindowPosition::At(new_window_position),
                decorations: true,
                ..default()
            }).id();
    
        let camera_entity = commands
            .spawn((
                Camera {
                    target: RenderTarget::Window(WindowRef::Entity(new_window_entity)),
                    ..default()
                },
                Camera2d::default(),
            ))
            .id();
    
        if let Ok(mut window_node) = node_query.get_mut(window_entity) {
            window_node.width = Val::Percent(100.0);
            window_node.height = Val::Percent(100.0);
            window_node.top = Val::Px(0.0);
            window_node.left = Val::Px(0.0);
        }
        commands.entity(window_entity).insert(UiTargetCamera(camera_entity));
        commands.entity(new_window_entity).insert(OsWindow::new(camera_entity, window_entity));
    }

    fn revert_to_ui_window(
        os_window_entity: Entity,
        commands: &mut Commands,
        winit_windows: &NonSend<WinitWindows>,
        children_query: &Query<&Children>,
        os_window_query: &Query<&OsWindow>,
        titlebar_query: &Query<&Titlebar>,
        footer_query: &Query<&Footer>,
        node_query: &mut Query<&mut Node>,
    ) {
        let Ok(os_window) = os_window_query.get(os_window_entity) else { return };
        let window_entity = os_window.ui_window_entity;
        let Ok(children) = children_query.get(window_entity) else { return };
    
        if let Some(titlebar_entity) = children
            .iter()
            .find(|child| titlebar_query.get(*child).is_ok())
        {
            Titlebar::register_observers(titlebar_entity, commands);
            if let Ok(mut titlebar_node) = node_query.get_mut(titlebar_entity) {
                titlebar_node.display = Display::Flex;
            }
        }
    
        if let Some(footer_entity) = children
            .iter()
            .find(|child| footer_query.get(*child).is_ok())
        {
            if let Ok(mut footer_node) = node_query.get_mut(footer_entity) {
                footer_node.display = Display::Flex;
            }
        }
    
        let Some(winit_window) = winit_windows.get_window(os_window_entity) else { return };
        let size: PhysicalSize<u32> = winit_window.outer_size();
        let Ok(position): Result<PhysicalPosition<i32>, _> = winit_window.outer_position() else { return };
    
        if let Ok(mut window_node) = node_query.get_mut(window_entity) {
            window_node.width = Val::Px(size.width as f32);
            window_node.height = Val::Px(size.height as f32);
            window_node.left = Val::Px(position.x as f32);
            window_node.top = Val::Px(position.y as f32);
        }
    
        commands.entity(window_entity).remove::<UiTargetCamera>();
        commands.entity(os_window.os_window_camera_entity).despawn();
        commands.entity(os_window_entity).despawn();
    }
}

impl Titlebar{
    pub fn default() -> Self{
        Titlebar{
            window_entity: Entity::PLACEHOLDER,
            title: "".to_string(),
        }
    }

    pub fn spawn(
        ctx: &mut UiWindowContext,
        label: &String,
        style: &UiWindowStyle,
        window_entity: Entity,
        closeable: bool
    ) -> Entity {
        let component = Titlebar{window_entity, title: label.clone()};
        
        let entity = ctx.commands
            .spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Px(24.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect {
                    left: Val::Px(style.titlebar_padding[0]),
                    right: Val::Px(style.titlebar_padding[1]),
                    top: Val::Px(style.titlebar_padding[2]),
                    bottom: Val::Px(style.titlebar_padding[3]),
                },
                ..default()
            })
            .insert(BackgroundColor(style.titlebar_color))
            .insert(component)
            .with_children(|titlebar| {
                titlebar
                    .spawn_empty()
                    .insert(Text::new(label.as_str()))
                    .insert(TextFont {
                        font_size: style.title_font_size,
                        ..default()
                    })
                    .insert(TextColor(style.title_color));
            }).id();

            if closeable{
                let close_btn_entity = CloseButton::spawn(ctx, style, window_entity);
                CloseButton::register_observers(close_btn_entity, &mut ctx.commands);
                ctx.commands.entity(entity).add_children(&[close_btn_entity]);
            }

            entity
            
    }

    pub fn register_observers(entity: Entity, commands: &mut Commands){
        commands.entity(entity).observe(
            move |trigger: Trigger<Pointer<Drag>>, 
                  title_bars: Query<&Titlebar>,
                  mut nodes: Query<&mut Node>,
                  mut commands: Commands,
                  children_query: Query<&Children>,
                  scroll_container_query: Query<&ScrollContainer>,
                  titlebar_query: Query<&Titlebar>,
                  footer_query: Query<&Footer>,
                  computed_node_query: Query<&ComputedNode>,
                  winit_windows: NonSend<WinitWindows>,
                  primary_window: Query<Entity, With<PrimaryWindow>>,
                  transform_query: Query<&GlobalTransform>,
                  window_query: Query<&Window>| 
            {
                let drag = trigger.event();
                let Ok(title_bar) = title_bars.get(trigger.target()) else {return};
                let window_entity = title_bar.window_entity;
        
                if let Ok(mut node) = nodes.get_mut(window_entity) {
                    if let Val::Px(ref mut left) = node.left {
                        *left += drag.delta.x;
                    }
                    if let Val::Px(ref mut top) = node.top {
                        *top += drag.delta.y;
                    }
        
                    if let Ok(primary_window_entity) = primary_window.single() {
                        let Ok(window) = window_query.get(primary_window_entity) else {return};
                        let width = window.resolution.physical_width() as f32;
                        let height = window.resolution.physical_height() as f32;
        
                        if let (Val::Px(left), Val::Px(top)) = (node.left, node.top) {
                            if left < 0.0 || top < 0.0 || left > width || top > height {
                                UiWindow::convert_to_os_window(
                                    window_entity,
                                    &mut commands,
                                    &children_query,
                                    &scroll_container_query,
                                    &titlebar_query,
                                    &footer_query,
                                    &computed_node_query,
                                    &transform_query,
                                    &mut nodes,
                                    &winit_windows,
                                    primary_window_entity
                                );
                            }
                        }
                    }
                }
            },
        )
        .observe(
            move |_: Trigger<Pointer<DragEnd>>, window: Single<Entity, With<Window>>, mut commands: Commands| {
                commands
                .entity(*window)
                .insert(CursorIcon::System(SystemCursorIcon::Grab));
            },
        )
        .observe(
            move |_: Trigger<Pointer<DragStart>>, window: Single<Entity, With<Window>>, mut commands: Commands| {
                commands
                .entity(*window)
                .insert(CursorIcon::System(SystemCursorIcon::Grabbing));
            },
        )
        .observe(
            move |_: Trigger<Pointer<Over>>, window: Single<Entity, With<Window>>, mut commands: Commands| {
                commands
                .entity(*window)
                .insert(CursorIcon::System(SystemCursorIcon::Grab));
            },
        )
        .observe(
            move |_: Trigger<Pointer<Out>>, window: Single<Entity, With<Window>>, mut commands: Commands| {
                commands
                .entity(*window)
                .insert(CursorIcon::System(SystemCursorIcon::Default));
            },
        );
    }
}

impl ScrollContainer{
    pub fn default() -> Self{
        ScrollContainer{
            scroll_bar_entity: Entity::PLACEHOLDER,
        }
    }
    
    pub fn new(scroll_bar_entity: Entity) -> Self{
        let mut scroll_container = ScrollContainer::default();
        scroll_container.scroll_bar_entity = scroll_bar_entity;
        scroll_container
    }

    pub fn spawn<F: FnOnce(&mut ChildSpawnerCommands)>(
        ctx: &mut UiWindowContext,
        style: &UiWindowStyle,
        children: F
    ) -> Entity {
        let content_entity = ctx.commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Auto,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .insert(ScrollContent)
        .with_children(|scroll_content| {
            children(scroll_content);
        }).id();
        let scroll_bar_entity = ScrollBar::spawn(ctx, style, content_entity);
        let component = ScrollContainer::new(scroll_bar_entity);

        let entity = ctx.commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            ..default()
        })
        .insert(BackgroundColor(style.background_color))
        .insert(component)
        .add_children(&[content_entity, scroll_bar_entity])
        .id();

        ScrollBar::register_observers(scroll_bar_entity, &mut ctx.commands);
        ScrollContainer::register_observers(entity, &mut ctx.commands);
        entity
    }
    pub fn register_observers(entity: Entity, commands: &mut Commands){
        commands.entity(entity).observe(
            move |event: Trigger<Pointer<Scroll>>,
                    scroll_container_query: Query<&ScrollContainer>,
                    mut scrollbar_query: Query<(&mut Node, &ScrollBar)>,
                    mut event_writer: EventWriter<ScrollbarMovedEvent>| {
                let scroll = event.event();
                let Ok(scroll_container) = scroll_container_query.get(event.target()) else {return;};
                if let Ok((node, scrollbar)) = scrollbar_query.get_mut(scroll_container.scroll_bar_entity) {
                    match node.height {
                        Val::Px(h) if h <= 0.0 => return,
                        Val::Percent(p) if p <= 0.0 => return,
                        _ => {}
                    }
                    ScrollBar::move_scrollbar(node, scrollbar, scroll.y * LINE_HEIGHT);
                    event_writer.write(ScrollbarMovedEvent { scrollbar: scroll_container.scroll_bar_entity });
                }

            },
        );
    }
}

impl ScrollBar{
    pub fn default() -> Self{
        ScrollBar{
            max_scroll_position: 0.0,
            content_height: 0.0,
            container_height: 0.0,
            scrollbar_height: 0.0,
            scroll_content_entity: Entity::PLACEHOLDER,
        }
    }

    pub fn new(scroll_container: Entity) -> Self{
        let mut component = ScrollBar::default();
        component.scroll_content_entity = scroll_container;
        component
    }

    pub fn spawn(ctx: &mut UiWindowContext, style: &UiWindowStyle, scroll_content_entity: Entity) -> Entity{
        let component = ScrollBar::new(scroll_content_entity);
        ctx.commands.spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(2.0),
            top: Val::Px(2.0),
            width: Val::Px(style.scrollbar_width),
            height: Val::Percent(0.0),
            ..default()
        })
        .insert(BackgroundColor(style.scrollbar_color))
        .insert(component)
        .id()
    }

    pub fn register_observers(entity: Entity, commands: &mut Commands){
        commands.entity(entity).observe(
            move |trigger: Trigger<Pointer<Drag>>,
                    mut node_query: Query<(&mut Node, &ScrollBar), With<ScrollBar>>,
                    mut ev_writer: EventWriter<ScrollbarMovedEvent>| {
                let drag = trigger.event();
                if let Ok((node, scrollbar)) = node_query.get_mut(trigger.target()){
                    ScrollBar::move_scrollbar(node, scrollbar, -drag.delta.y);
                    ev_writer.write(ScrollbarMovedEvent { scrollbar: trigger.target() });
                }
            },
        );
    }

    fn move_scrollbar(
        mut node: Mut<Node>,
        scrollbar: &ScrollBar,
        delta_y: f32,
    ) {
        let current_top = match node.top {
            Val::Px(current) => current,
            _ => 0.0,
        };
    
        let new_top = (current_top - delta_y).clamp(0.0, scrollbar.max_scroll_position);
        node.top = Val::Px(new_top);
    }
}

impl CloseButton{
    pub fn default() -> Self{
        CloseButton{
            window_entity: Entity::PLACEHOLDER,
        }
    }

    pub fn new(window_entity: Entity) -> Self{
        let mut component = CloseButton::default();
        component.window_entity = window_entity;
        component
    }

    pub fn spawn(
        ctx: &mut UiWindowContext,
        style: &UiWindowStyle,
        window_entity: Entity
    ) -> Entity {
        let icon_font = ctx.asset_server.load("fonts/GoogleMaterialIcons.ttf");
        let component = CloseButton::new(window_entity);
        ctx.commands.spawn(Node {
            width: Val::Px(24.0),
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .insert(component)
        .with_children(|close| {
            close.spawn_empty()
            .insert(Text::new("\u{e5cd}"))
            .insert(TextFont {
                font_size: style.title_font_size + 2.0,
                font: icon_font,
                ..default()
            })
            .insert(TextColor(style.close_button_color));
        }).id()
    }

    pub fn register_observers(entity: Entity, commands: &mut Commands){
        commands.entity(entity)
        .observe(
            move |trigger: Trigger<Pointer<Click>>,
                  mut commands: Commands,
                  window: Single<Entity, With<Window>>,
                  close_btn_query: Query<&CloseButton>| {
                    if let Ok(close_btn) = close_btn_query.get(trigger.target()){
                        commands.entity(close_btn.window_entity).despawn();
                    }
                    commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Default));
            },
        )
        .observe(
            move |mut trigger: Trigger<Pointer<Over>>, mut commands: Commands, window: Single<Entity, With<Window>>| {
                commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Pointer));
                trigger.propagate(false);
            },
        )
        .observe(
            move |_: Trigger<Pointer<Out>>, mut commands: Commands, window: Single<Entity, With<Window>>| {
                commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Default));
            },
        );
    }
}

impl ResizeCorner {
    pub fn default() -> Self {
        ResizeCorner {
            window_entity: Entity::PLACEHOLDER,
        }
    }

    pub fn new(window_entity: Entity) -> Self {
        ResizeCorner {
            window_entity,
        }
    }

    pub fn spawn(
        ctx: &mut UiWindowContext,
        style: &UiWindowStyle,
        window_entity: Entity,
    ) -> Entity {
        let icon_font = ctx.asset_server.load("fonts/GoogleMaterialIcons.ttf");
        let component = ResizeCorner::new(window_entity);

        let entity = ctx.commands
            .spawn(Node {
                width: Val::Px(24.0),
                height: Val::Px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            })
            .insert(component)
            .with_children(|corner| {
                corner
                    .spawn_empty()
                    .insert(Text::new("\u{f8ab}"))
                    .insert(TextFont {
                        font_size: style.title_font_size + 2.0,
                        font: icon_font,
                        ..default()
                    })
                    .insert(TextColor(style.resize_handle_color))
                    .insert(Transform {
                        scale: Vec3::new(-1.0, 1.0, 1.0),
                        ..default()
                    });
            })
            .id();

            ResizeCorner::register_observers(entity, &mut ctx.commands);

            entity


    }

    pub fn register_observers(entity: Entity, commands: &mut Commands) {
        commands
            .entity(entity)
            .observe(
                move |mut trigger: Trigger<Pointer<Over>>, mut commands: Commands, window: Single<Entity, With<Window>>| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::NwseResize));
                    trigger.propagate(false);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Out>>, mut commands: Commands, window: Single<Entity, With<Window>>| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::Default));
                },
            )
            .observe(
                move |trigger: Trigger<Pointer<Drag>>,
                      mut nodes: Query<&mut Node>,
                      computed: Query<&ComputedNode>,
                      corners: Query<&ResizeCorner>| {
                    let drag = trigger.event();
        
                    if let Ok(resize_corner) = corners.get(trigger.target()) {
                        if let Ok(mut node) = nodes.get_mut(resize_corner.window_entity) {
                            if let Ok(layout) = computed.get(resize_corner.window_entity) {
                                let new_width_px = (layout.size.x + drag.delta.x).max(50.0);
                                match node.width {
                                    Val::Px(_) => node.width = Val::Px(new_width_px),
                                    Val::Percent(pct) => {
                                        let total = layout.unrounded_size.x / (pct / 100.0);
                                        node.width = Val::Percent((new_width_px / total) * 100.0);
                                    }
                                    _ => node.width = Val::Px(new_width_px),
                                }
        
                                let new_height_px = (layout.size.y + drag.delta.y).max(50.0);
                                match node.height {
                                    Val::Px(_) => node.height = Val::Px(new_height_px),
                                    Val::Percent(pct) => {
                                        let total = layout.unrounded_size.y / (pct / 100.0);
                                        node.height = Val::Percent((new_height_px / total) * 100.0);
                                    }
                                    _ => node.height = Val::Px(new_height_px),
                                }
                            }
                        }
                    }
                },
            );
    }
}

fn update_scrollbar_height(
    query: Query<(&Children, &ComputedNode), (With<ScrollContainer>, Changed<ComputedNode>)>,
    computed_node_query: Query<&ComputedNode>,
    scroll_content_query: Query<(), With<ScrollContent>>,
    mut ev_writer: EventWriter<ScrollbarMovedEvent>,
    mut scrollbar_query: Query<(&mut Node, &mut ScrollBar)>,
) {
    for (children, container_computed) in &query {
        let container_height = container_computed.size.y;

        let mut total_content_height = 0.0;
        for child in children.iter() {
            if let Ok(child_computed) = computed_node_query.get(child) {
                if scroll_content_query.get(child).is_ok() {
                    total_content_height += child_computed.size.y;
                }
            }
        }

        if total_content_height == 0.0 {
            continue;
        }

        let scroll_ratio = (container_height / total_content_height).clamp(0.05, 1.0);
        let scroll_percent = scroll_ratio * 100.0;

        for child in children.iter() {
            if let Ok((mut node, mut scrollbar)) = scrollbar_query.get_mut(child) {
                let height_px = container_height * scroll_ratio;
                node.height = if scroll_percent == 100.0 {
                    Val::Px(0.0)
                } else {
                    Val::Percent(scroll_percent)
                };

                scrollbar.max_scroll_position = container_height - height_px;
                scrollbar.scrollbar_height = height_px;
                scrollbar.container_height = container_height;
                scrollbar.content_height = total_content_height;

                let current_top = match node.top {
                    Val::Px(px) => px,
                    _ => 0.0,
                };
                if current_top + height_px > container_height {
                    node.top = Val::Px(container_height - height_px);
                }

                ev_writer.write(ScrollbarMovedEvent { scrollbar: child });
            }
        }
    }
}

fn sync_scrollbar_to_content(
    mut events: EventReader<ScrollbarMovedEvent>,
    mut node_query: Query<&mut Node>,
    scrollbar_query: Query<&ScrollBar>,
) {
    for event in events.read() {
        if let Ok(scrollbar) = scrollbar_query.get(event.scrollbar) {
            if let Ok(scrollbar_node) = node_query.get(event.scrollbar) {
                let scroll_ratio = match scrollbar_node.top {
                    Val::Px(px) => {
                        if scrollbar.max_scroll_position != 0.0 {
                            px / scrollbar.max_scroll_position
                        } else {
                            0.0
                        }
                    }
                    _ => continue,
                };

                let overflow = (scrollbar.content_height - scrollbar.container_height).max(0.0);

                let new_top = if overflow > 0.0 {
                    -overflow * scroll_ratio
                } else {
                    0.0
                };

                if let Ok(mut content_node) = node_query.get_mut(scrollbar.scroll_content_entity) {
                    content_node.top = Val::Px(new_top);
                }
            }
        }
    }
}

fn detect_os_window_reentry(
    mut commands: Commands,
    winit_windows: NonSend<WinitWindows>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    promoted_windows: Query<Entity, With<OsWindow>>,
    children_query: Query<&Children>,
    os_window_query: Query<&OsWindow>,
    titlebar_query: Query<&Titlebar>,
    footer_query: Query<&Footer>,
    mut node_query: Query<&mut Node>,
) {
    let Ok(primary_entity) = primary_window_query.single() else {
        return;
    };

    let Some(primary_window) = winit_windows.get_window(primary_entity) else {
        return;
    };

    let Ok(primary_position) = primary_window.outer_position() else {
        return;
    };
    let primary_size = primary_window.outer_size();

    let primary_left = primary_position.x;
    let primary_top = primary_position.y;
    let primary_right = primary_left + primary_size.width as i32;
    let primary_bottom = primary_top + primary_size.height as i32;

    for entity in promoted_windows.iter() {
        let Some(os_window) = winit_windows.get_window(entity) else {
            continue;
        };

        let Ok(os_position) = os_window.outer_position() else {
            continue;
        };
        let os_size = os_window.outer_size();

        let os_left = os_position.x;
        let os_top = os_position.y;
        let os_right = os_left + os_size.width as i32;
        let os_bottom = os_top + os_size.height as i32;

        let within_bounds = os_left >= primary_left
            && os_top >= primary_top
            && os_right <= primary_right
            && os_bottom <= primary_bottom;

        if within_bounds {
            UiWindow::revert_to_ui_window(
                entity,
                &mut commands,
                &winit_windows,
                &children_query,
                &os_window_query,
                &titlebar_query,
                &footer_query,
                &mut node_query,
            );
        }
    }
}
