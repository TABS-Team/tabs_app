use bevy::{
    prelude::*,
    ui::{ BackgroundColor, BorderColor },
    ecs::{ observer::ObservedBy },
    winit::{ cursor::{ CursorIcon }, WinitWindows },
    window::{ SystemCursorIcon, PrimaryWindow, WindowRef },
    render::camera::RenderTarget,
};
use winit::dpi::{ PhysicalPosition, PhysicalSize };

use crate::widgets::{ UiLayer, UiLayerStack, UiContext };

const SPAWNMARGIN: i32 = 10;

pub struct UiWindowPlugin;

impl Plugin for UiWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PromoteToOsWindowEvent>().add_systems(Update, detect_os_window_reentry);
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
struct Content;

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

#[derive(Event)]
pub struct PromoteToOsWindowEvent {
    pub window_entity: Entity,
    pub window_title: String,
}

#[derive(Component, Clone)]
pub struct UiWindow {
    pub title: String,
    pub width: Val,
    pub height: Val,
    pub layer: UiLayer,
    pub position: PositionType,
    pub style: UiWindowStyle,
    pub options: UiWindowOptions,
}

#[derive(Component)]
pub struct OsWindow {
    pub os_window_camera_entity: Entity,
    pub ui_window_entity: Entity,
}

impl OsWindow {
    fn new(camera: Entity, ui_window: Entity) -> Self {
        OsWindow {
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
    pub camera: Option<Entity>,
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
            titlebar_padding: [6.0; 4],
            content_padding: [8.0; 4],
        }
    }
}

pub struct UiWindowBuilder {
    title: String,
    width: Val,
    height: Val,
    layer: UiLayer,
    position: PositionType,
    style: UiWindowStyle,
    options: UiWindowOptions,
}

impl UiWindowBuilder {
    pub fn size(mut self, width: Val, height: Val) -> Self {
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

    pub fn camera(mut self, camera: Entity) -> Self {
        self.options.camera = Some(camera);
        self
    }

    pub fn build(self) -> UiWindow {
        if self.options.camera.is_none() {
            warn!("UiWindowBuilder: No camera set, using default camera.");
        }

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
            width: Val::Percent(50.0),
            height: Val::Percent(50.0),
            layer,
            position: PositionType::Absolute,
            style: UiWindowStyle::default(),
            options: UiWindowOptions::default(),
        }
    }

    pub fn bundle(&self, left: Val, top: Val) -> UiWindowBundle {
        let pos_type = if self.options.draggable { PositionType::Absolute } else { self.position };
        UiWindowBundle {
            node: Node {
                width: self.width,
                height: self.height,
                position_type: pos_type,
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
        &self,
        commands: &mut Commands,
        ctx: &UiContext,
        layer_stack: &mut UiLayerStack,
        left: Val,
        top: Val,
        children: F
    ) -> Entity {
        let window = commands
            .spawn(self.bundle(left, top))
            .insert(self.clone())
            .insert(WindowLayer(self.layer))
            .id();

        let mut modules = Vec::new();

        if self.options.show_titlebar {
            let tb = Titlebar::spawn(
                commands,
                ctx,
                &self.title,
                &self.style,
                window,
                self.options.closeable
            );
            modules.push(tb);
            if self.options.draggable {
                Titlebar::register_observers(tb, commands);
            }
        }
        let content_entity = commands
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .insert(Content)
            .with_children(|parent| {
                children(parent);
            })
            .id();

        modules.push(content_entity);

        if self.options.resizeable {
            let resize_corner = ResizeCorner::spawn(commands, ctx, &self.style, window);
            let footer = commands
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(self.style.content_padding[0])),
                    ..default()
                })
                .insert(BackgroundColor(self.style.background_color))
                .insert(Footer)
                .add_children(&[resize_corner])
                .id();
            modules.push(footer);
        }

        commands.entity(window).add_children(&modules);
        UiWindow::register_observers(window, commands);
        layer_stack.push(self.layer, window, commands);

        window
    }

    fn register_observers(entity: Entity, commands: &mut Commands) {
        commands
            .entity(entity)
            .observe(
                |
                    trigger: Trigger<Pointer<Pressed>>,
                    mut commands: Commands,
                    mut stack: ResMut<UiLayerStack>,
                    layers: Query<&WindowLayer>
                | {
                    if let Ok(WindowLayer(layer)) = layers.get(trigger.target()) {
                        stack.bring_to_front(*layer, trigger.target(), &mut commands);
                    }
                }
            );
    }

    fn convert_to_os_window(
        window_entity: Entity,
        commands: &mut Commands,
        children_query: &Query<&Children>,
        content_container_query: &Query<&Content>,
        titlebar_query: &Query<&Titlebar>,
        footer_query: &Query<&Footer>,
        computed_node_query: &Query<&ComputedNode>,
        node_query: &mut Query<&mut Node>,
        winit_windows: &NonSend<WinitWindows>,
        primary_window_entity: Entity
    ) {
        let Ok(children) = children_query.get(window_entity) else {
            return;
        };

        let Some(content_container_entity) = children
            .iter()
            .find(|child| content_container_query.get(*child).is_ok()) else {
            return;
        };

        let Ok(size) = computed_node_query.get(content_container_entity) else {
            return;
        };

        if
            let Some(titlebar_entity) = children
                .iter()
                .find(|child| titlebar_query.get(*child).is_ok())
        {
            commands.entity(titlebar_entity).remove::<ObservedBy>();
            if let Ok(mut titlebar_node) = node_query.get_mut(titlebar_entity) {
                titlebar_node.display = Display::None;
            }
        }

        if let Some(footer_entity) = children.iter().find(|child| footer_query.get(*child).is_ok()) {
            if let Ok(mut footer_node) = node_query.get_mut(footer_entity) {
                footer_node.display = Display::None;
            }
        }

        let title = titlebar_query
            .iter()
            .find(|t| t.window_entity == window_entity)
            .map(|t| t.title.clone())
            .unwrap_or("Untitled".into());

        let Some(primary_window) = winit_windows.get_window(primary_window_entity) else {
            return;
        };
        let primary_position = primary_window.outer_position().unwrap();

        let Ok(node) = node_query.get(window_entity) else {
            return;
        };
        let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
            return;
        };

        let PhysicalSize { width: screen_w, height: screen_h } = primary_window.inner_size();
        let screen_w = screen_w as f32;
        let screen_h = screen_h as f32;

        let mut spawn_x = primary_position.x + (left as i32);
        let mut spawn_y = primary_position.y + (top as i32);

        if left < 0.0 {
            spawn_x -= (size.size.x as i32) + SPAWNMARGIN;
        } else if left + size.size.x > screen_w {
            spawn_x += SPAWNMARGIN;
        }

        if top < 0.0 {
            spawn_y -= (size.size.y as i32) + SPAWNMARGIN;
        } else if top + size.size.y > screen_h {
            spawn_y += SPAWNMARGIN;
        }

        let new_window_position = IVec2::new(spawn_x, spawn_y);

        let new_window_entity = commands
            .spawn(Window {
                resolution: (size.size.x, size.size.y).into(),
                title,
                position: WindowPosition::At(new_window_position),
                decorations: true,
                ..default()
            })
            .id();

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
        commands.entity(window_entity).remove::<UiTargetCamera>();
        commands.entity(window_entity).insert(UiTargetCamera(camera_entity));
        commands.entity(new_window_entity).insert(OsWindow::new(camera_entity, window_entity));
    }

    fn revert_to_ui_window(
        os_window_entity: Entity,
        commands: &mut Commands,
        winit_windows: &NonSend<WinitWindows>,
        children_query: &Query<&Children>,
        os_window_query: &Query<&OsWindow>,
        ui_window_query: &Query<&UiWindow>,
        titlebar_query: &Query<&Titlebar>,
        footer_query: &Query<&Footer>,
        node_query: &mut Query<&mut Node>,
        primary_window_entity: Entity
    ) {
        let Ok(os_window) = os_window_query.get(os_window_entity) else {
            return;
        };
        let window_entity = os_window.ui_window_entity;
        let Ok(children) = children_query.get(window_entity) else {
            return;
        };

        if
            let Some(titlebar_entity) = children
                .iter()
                .find(|child| titlebar_query.get(*child).is_ok())
        {
            Titlebar::register_observers(titlebar_entity, commands);
            if let Ok(mut titlebar_node) = node_query.get_mut(titlebar_entity) {
                titlebar_node.display = Display::Flex;
            }
        }

        if let Some(footer_entity) = children.iter().find(|child| footer_query.get(*child).is_ok()) {
            if let Ok(mut footer_node) = node_query.get_mut(footer_entity) {
                footer_node.display = Display::Flex;
            }
        }

        let Some(winit_window) = winit_windows.get_window(os_window_entity) else {
            return;
        };
        let size: PhysicalSize<u32> = winit_window.outer_size();
        let Ok(position): Result<PhysicalPosition<i32>, _> = winit_window.outer_position() else {
            return;
        };

        if let Ok(mut window_node) = node_query.get_mut(window_entity) {
            let primary_wnd = winit_windows
                .get_window(primary_window_entity)
                .expect("Primary OS window not found");
            let primary_pos = primary_wnd.outer_position().unwrap();
            let scale = primary_wnd.scale_factor() as f32;
            let ui_x = ((position.x - primary_pos.x) as f32) / scale;
            let ui_y = ((position.y - primary_pos.y) as f32) / scale;

            let ui_w = (size.width as f32) / scale;
            let ui_h = (size.height as f32) / scale;

            window_node.width = Val::Px(ui_w);
            window_node.height = Val::Px(ui_h);

            const SPAWNMARGIN: f32 = 10.0;

            let from_top = ui_y + ui_h * 0.5 < 0.0;
            let from_bottom = ui_y + ui_h * 0.5 > (primary_wnd.inner_size().height as f32) / scale;
            let from_left = ui_x + ui_w * 0.5 < 0.0;
            let from_right = ui_x + ui_w * 0.5 > (primary_wnd.inner_size().width as f32) / scale;
            let final_x = if from_left {
                SPAWNMARGIN
            } else if from_right {
                (primary_wnd.inner_size().width as f32) / scale - ui_w - SPAWNMARGIN
            } else {
                ui_x
            };

            let final_y = if from_top {
                SPAWNMARGIN
            } else if from_bottom {
                (primary_wnd.inner_size().height as f32) / scale - ui_h - SPAWNMARGIN
            } else {
                ui_y
            };
            window_node.left = Val::Px(final_x);
            window_node.top = Val::Px(final_y);
        }

        commands.entity(window_entity).remove::<UiTargetCamera>();
        if let Ok(ui_window) = ui_window_query.get(window_entity) {
            if let Some(camera_entity) = ui_window.options.camera {
                commands.entity(window_entity).insert(UiTargetCamera(camera_entity));
            }
        }
        commands.entity(os_window.os_window_camera_entity).despawn();
        commands.entity(os_window_entity).despawn();
    }
}

impl Titlebar {
    pub fn default() -> Self {
        Titlebar {
            window_entity: Entity::PLACEHOLDER,
            title: "".to_string(),
        }
    }

    pub fn spawn(
        commands: &mut Commands,
        ctx: &UiContext,
        label: &String,
        style: &UiWindowStyle,
        window_entity: Entity,
        closeable: bool
    ) -> Entity {
        let component = Titlebar { window_entity, title: label.clone() };

        let entity = commands
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
            })
            .id();

        if closeable {
            let close_btn_entity = CloseButton::spawn(commands, ctx, style, window_entity);
            CloseButton::register_observers(close_btn_entity, commands);
            commands.entity(entity).add_children(&[close_btn_entity]);
        }

        entity
    }

    pub fn register_observers(entity: Entity, commands: &mut Commands) {
        commands
            .entity(entity)
            .observe(
                move |
                    trigger: Trigger<Pointer<Drag>>,
                    title_bars: Query<&Titlebar>,
                    mut nodes: Query<&mut Node>,
                    mut commands: Commands,
                    children_query: Query<&Children>,
                    content_container_query: Query<&Content>,
                    titlebar_query: Query<&Titlebar>,
                    footer_query: Query<&Footer>,
                    computed_node_query: Query<&ComputedNode>,
                    winit_windows: NonSend<WinitWindows>,
                    primary_window: Query<Entity, With<PrimaryWindow>>
                | {
                    let drag = trigger.event();
                    let Ok(title_bar) = title_bars.get(trigger.target()) else {
                        return;
                    };
                    let window_entity = title_bar.window_entity;

                    if let Ok(mut node) = nodes.get_mut(window_entity) {
                        if let Val::Px(ref mut left) = node.left {
                            *left += drag.delta.x;
                        }
                        if let Val::Px(ref mut top) = node.top {
                            *top += drag.delta.y;
                        }

                        if let Ok(primary_window_entity) = primary_window.single() {
                            let winit_window = winit_windows
                                .get_window(primary_window_entity)
                                .expect("Primary window not found");

                            let PhysicalSize { width, height } = winit_window.inner_size();
                            let screen_w = width as f32;
                            let screen_h = height as f32;
                            let computed = computed_node_query
                                .get(window_entity)
                                .expect("ComputedNode missing");
                            let node_w = computed.size.x;
                            let node_h = computed.size.y;
                            if let (Val::Px(left), Val::Px(top)) = (node.left, node.top) {
                                let center_x = left + node_w * 0.5;
                                let center_y = top + node_h * 0.5;
                                let half_out =
                                    center_x < 0.0 ||
                                    center_y < 0.0 ||
                                    center_x > screen_w ||
                                    center_y > screen_h;

                                if half_out {
                                    UiWindow::convert_to_os_window(
                                        window_entity,
                                        &mut commands,
                                        &children_query,
                                        &content_container_query,
                                        &titlebar_query,
                                        &footer_query,
                                        &computed_node_query,
                                        &mut nodes,
                                        &winit_windows,
                                        primary_window_entity
                                    );
                                }
                            }
                        }
                    }
                }
            )
            .observe(
                move |
                    _: Trigger<Pointer<DragEnd>>,
                    window: Single<Entity, With<Window>>,
                    mut commands: Commands
                | {
                    commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Grab));
                }
            )
            .observe(
                move |
                    _: Trigger<Pointer<DragStart>>,
                    window: Single<Entity, With<Window>>,
                    mut commands: Commands
                | {
                    commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Grabbing));
                }
            )
            .observe(
                move |
                    _: Trigger<Pointer<Over>>,
                    window: Single<Entity, With<Window>>,
                    mut commands: Commands
                | {
                    commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Grab));
                }
            )
            .observe(
                move |
                    _: Trigger<Pointer<Out>>,
                    window: Single<Entity, With<Window>>,
                    mut commands: Commands
                | {
                    commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Default));
                }
            );
    }
}

impl CloseButton {
    pub fn default() -> Self {
        CloseButton {
            window_entity: Entity::PLACEHOLDER,
        }
    }

    pub fn new(window_entity: Entity) -> Self {
        let mut component = CloseButton::default();
        component.window_entity = window_entity;
        component
    }

    pub fn spawn(
        commands: &mut Commands,
        ctx: &UiContext,
        style: &UiWindowStyle,
        window_entity: Entity
    ) -> Entity {
        let icon_font = ctx.asset_server.load("fonts/GoogleMaterialIcons.ttf");
        let component = CloseButton::new(window_entity);
        commands
            .spawn(Node {
                width: Val::Px(24.0),
                height: Val::Px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .insert(component)
            .with_children(|close| {
                close
                    .spawn_empty()
                    .insert(Text::new("\u{e5cd}"))
                    .insert(TextFont {
                        font_size: style.title_font_size + 2.0,
                        font: icon_font,
                        ..default()
                    })
                    .insert(TextColor(style.close_button_color));
            })
            .id()
    }

    pub fn register_observers(entity: Entity, commands: &mut Commands) {
        commands
            .entity(entity)
            .observe(
                move |
                    trigger: Trigger<Pointer<Click>>,
                    mut commands: Commands,
                    window: Single<Entity, With<Window>>,
                    close_btn_query: Query<&CloseButton>
                | {
                    if let Ok(close_btn) = close_btn_query.get(trigger.target()) {
                        commands.entity(close_btn.window_entity).despawn();
                    }
                    commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Default));
                }
            )
            .observe(
                move |
                    mut trigger: Trigger<Pointer<Over>>,
                    mut commands: Commands,
                    window: Single<Entity, With<Window>>
                | {
                    commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Pointer));
                    trigger.propagate(false);
                }
            )
            .observe(
                move |
                    _: Trigger<Pointer<Out>>,
                    mut commands: Commands,
                    window: Single<Entity, With<Window>>
                | {
                    commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Default));
                }
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
        commands: &mut Commands,
        ctx: &UiContext,
        style: &UiWindowStyle,
        window_entity: Entity
    ) -> Entity {
        let icon_font = ctx.asset_server.load("fonts/GoogleMaterialIcons.ttf");
        let component = ResizeCorner::new(window_entity);

        let entity = commands
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

        ResizeCorner::register_observers(entity, commands);

        entity
    }

    pub fn register_observers(entity: Entity, commands: &mut Commands) {
        commands
            .entity(entity)
            .observe(
                move |
                    mut trigger: Trigger<Pointer<Over>>,
                    mut commands: Commands,
                    window: Single<Entity, With<Window>>
                | {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::NwseResize));
                    trigger.propagate(false);
                }
            )
            .observe(
                move |
                    _: Trigger<Pointer<Out>>,
                    mut commands: Commands,
                    window: Single<Entity, With<Window>>
                | {
                    commands.entity(*window).insert(CursorIcon::System(SystemCursorIcon::Default));
                }
            )
            .observe(
                move |
                    trigger: Trigger<Pointer<Drag>>,
                    mut nodes: Query<&mut Node>,
                    computed: Query<&ComputedNode>,
                    corners: Query<&ResizeCorner>
                | {
                    let drag = trigger.event();

                    if let Ok(resize_corner) = corners.get(trigger.target()) {
                        if let Ok(mut node) = nodes.get_mut(resize_corner.window_entity) {
                            if let Ok(layout) = computed.get(resize_corner.window_entity) {
                                let new_width_px = (layout.size.x + drag.delta.x).max(50.0);
                                match node.width {
                                    Val::Px(_) => {
                                        node.width = Val::Px(new_width_px);
                                    }
                                    Val::Percent(pct) => {
                                        let total = layout.unrounded_size.x / (pct / 100.0);
                                        node.width = Val::Percent((new_width_px / total) * 100.0);
                                    }
                                    _ => {
                                        node.width = Val::Px(new_width_px);
                                    }
                                }

                                let new_height_px = (layout.size.y + drag.delta.y).max(50.0);
                                match node.height {
                                    Val::Px(_) => {
                                        node.height = Val::Px(new_height_px);
                                    }
                                    Val::Percent(pct) => {
                                        let total = layout.unrounded_size.y / (pct / 100.0);
                                        node.height = Val::Percent((new_height_px / total) * 100.0);
                                    }
                                    _ => {
                                        node.height = Val::Px(new_height_px);
                                    }
                                }
                            }
                        }
                    }
                }
            );
    }
}

fn detect_os_window_reentry(
    mut commands: Commands,
    winit_windows: NonSend<WinitWindows>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    promoted_windows: Query<Entity, With<OsWindow>>,
    children_query: Query<&Children>,
    os_window_query: Query<&OsWindow>,
    ui_window_query: Query<&UiWindow>,
    titlebar_query: Query<&Titlebar>,
    footer_query: Query<&Footer>,
    mut node_query: Query<&mut Node>
) {
    if promoted_windows.is_empty() {
        return;
    }

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
    let primary_right = primary_left + (primary_size.width as i32);
    let primary_bottom = primary_top + (primary_size.height as i32);

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
        let os_right = os_left + (os_size.width as i32) / 2;
        let os_bottom = os_top + (os_size.height as i32) / 2;

        let within_bounds =
            os_left > primary_left &&
            os_top > primary_top &&
            os_right < primary_right &&
            os_bottom < primary_bottom;

        if within_bounds {
            UiWindow::revert_to_ui_window(
                entity,
                &mut commands,
                &winit_windows,
                &children_query,
                &os_window_query,
                &ui_window_query,
                &titlebar_query,
                &footer_query,
                &mut node_query,
                primary_entity
            );
        }
    }
}
