use bevy::picking::prelude::{Click, Drag, DragEnd, DragStart, Out, Over, Pointer, Press};
use bevy::{
    camera::RenderTarget,
    ecs::observer::ObservedBy,
    prelude::*,
    ui::{BackgroundColor, BorderColor, ComputedNode},
    window::{CursorIcon, PrimaryWindow, SystemCursorIcon, Window, WindowRef, WindowResolution},
    winit::WINIT_WINDOWS,
};
use winit::dpi::PhysicalPosition;

use crate::widgets::{UiContext, UiLayer, UiLayerStack};

const SPAWNMARGIN: i32 = 10;

pub struct UiWindowPlugin;

impl Plugin for UiWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, detect_os_window_reentry)
            .add_systems(PostUpdate, detect_os_window_promotion);
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
        let pos_type = if self.options.draggable {
            PositionType::Absolute
        } else {
            self.position
        };
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
            border: BorderColor::all(self.style.border_color),
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
        children: F,
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
                self.options.closeable,
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
        commands.entity(entity).observe(
            |trigger: On<Pointer<Press>>,
             mut commands: Commands,
             mut stack: ResMut<UiLayerStack>,
             layers: Query<&WindowLayer>| {
                if let Ok(WindowLayer(layer)) = layers.get(trigger.entity) {
                    stack.bring_to_front(*layer, trigger.entity, &mut commands);
                }
            },
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
        primary_window_entity: Entity,
        primary_window: &Window,
    ) {
        let Ok(children) = children_query.get(window_entity) else {
            return;
        };

        let Some(content_container_entity) = children
            .iter()
            .find(|child| content_container_query.get(*child).is_ok())
        else {
            return;
        };

        let Ok(size) = computed_node_query.get(content_container_entity) else {
            return;
        };

        if let Some(titlebar_entity) = children
            .iter()
            .find(|child| titlebar_query.get(*child).is_ok())
        {
            commands.entity(titlebar_entity).remove::<ObservedBy>();
            if let Ok(mut titlebar_node) = node_query.get_mut(titlebar_entity) {
                titlebar_node.display = Display::None;
            }
        }

        if let Some(footer_entity) = children
            .iter()
            .find(|child| footer_query.get(*child).is_ok())
        {
            if let Ok(mut footer_node) = node_query.get_mut(footer_entity) {
                footer_node.display = Display::None;
            }
        }

        let title = titlebar_query
            .iter()
            .find(|t| t.window_entity == window_entity)
            .map(|t| t.title.clone())
            .unwrap_or("Untitled".into());

        let scale = primary_window.scale_factor();
        let primary_position = WINIT_WINDOWS
            .with_borrow(|winit_windows| {
                winit_windows
                    .get_window(primary_window_entity)
                    .and_then(|window| window.outer_position().ok())
            })
            .unwrap_or(PhysicalPosition { x: 0, y: 0 });

        let Ok(node) = node_query.get(window_entity) else {
            return;
        };
        let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
            return;
        };

        let screen_w = primary_window.physical_width() as f32;
        let screen_h = primary_window.physical_height() as f32;
        let left_phys = left * scale;
        let top_phys = top * scale;
        let spawn_margin = (SPAWNMARGIN as f32 * scale).round() as i32;

        let mut spawn_x = primary_position.x + left_phys.round() as i32;
        let mut spawn_y = primary_position.y + top_phys.round() as i32;

        if left_phys < 0.0 {
            spawn_x -= (size.size.x.round() as i32) + spawn_margin;
        } else if left_phys + size.size.x > screen_w {
            spawn_x += spawn_margin;
        }

        if top_phys < 0.0 {
            spawn_y -= (size.size.y.round() as i32) + spawn_margin;
        } else if top_phys + size.size.y > screen_h {
            spawn_y += spawn_margin;
        }

        let new_window_position = IVec2::new(spawn_x, spawn_y);

        let resolution = WindowResolution::new(
            size.size.x.max(1.0).round() as u32,
            size.size.y.max(1.0).round() as u32,
        );

        let new_window_entity = commands
            .spawn(Window {
                resolution,
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
        commands
            .entity(window_entity)
            .insert(UiTargetCamera(camera_entity));
        commands
            .entity(new_window_entity)
            .insert(OsWindow::new(camera_entity, window_entity));
    }

    fn revert_to_ui_window(
        os_window_entity: Entity,
        commands: &mut Commands,
        children_query: &Query<&Children>,
        os_window_query: &Query<&OsWindow>,
        ui_window_query: &Query<&UiWindow>,
        titlebar_query: &Query<&Titlebar>,
        footer_query: &Query<&Footer>,
        node_query: &mut Query<&mut Node>,
        primary_window_entity: Entity,
    ) {
        let Ok(os_window) = os_window_query.get(os_window_entity) else {
            return;
        };
        let window_entity = os_window.ui_window_entity;
        let Ok(children) = children_query.get(window_entity) else {
            return;
        };

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

        let Some((size, position, primary_pos, scale, primary_inner_size)) = WINIT_WINDOWS
            .with_borrow(|winit_windows| {
                let os_window = winit_windows.get_window(os_window_entity)?;
                let primary_wnd = winit_windows.get_window(primary_window_entity)?;
                let position = os_window.outer_position().ok()?;
                let primary_pos = primary_wnd.outer_position().ok()?;
                let scale = primary_wnd.scale_factor() as f32;
                let inner_size = primary_wnd.inner_size();
                Some((
                    os_window.outer_size(),
                    position,
                    primary_pos,
                    scale,
                    inner_size,
                ))
            })
        else {
            return;
        };

        if let Ok(mut window_node) = node_query.get_mut(window_entity) {
            let ui_x = ((position.x - primary_pos.x) as f32) / scale;
            let ui_y = ((position.y - primary_pos.y) as f32) / scale;

            let ui_w = (size.width as f32) / scale;
            let ui_h = (size.height as f32) / scale;

            window_node.width = Val::Px(ui_w);
            window_node.height = Val::Px(ui_h);

            const SPAWNMARGIN: f32 = 10.0;

            let from_top = ui_y + ui_h * 0.5 < 0.0;
            let from_bottom = ui_y + ui_h * 0.5 > (primary_inner_size.height as f32) / scale;
            let from_left = ui_x + ui_w * 0.5 < 0.0;
            let from_right = ui_x + ui_w * 0.5 > (primary_inner_size.width as f32) / scale;
            let final_x = if from_left {
                SPAWNMARGIN
            } else if from_right {
                (primary_inner_size.width as f32) / scale - ui_w - SPAWNMARGIN
            } else {
                ui_x
            };

            let final_y = if from_top {
                SPAWNMARGIN
            } else if from_bottom {
                (primary_inner_size.height as f32) / scale - ui_h - SPAWNMARGIN
            } else {
                ui_y
            };
            window_node.left = Val::Px(final_x);
            window_node.top = Val::Px(final_y);
        }

        commands.entity(window_entity).remove::<UiTargetCamera>();
        if let Ok(ui_window) = ui_window_query.get(window_entity) {
            if let Some(camera_entity) = ui_window.options.camera {
                commands
                    .entity(window_entity)
                    .insert(UiTargetCamera(camera_entity));
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
        closeable: bool,
    ) -> Entity {
        let component = Titlebar {
            window_entity,
            title: label.clone(),
        };

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
                move |trigger: On<Pointer<Drag>>,
                      title_bars: Query<&Titlebar>,
                      mut nodes: Query<&mut Node>,
                      mut commands: Commands,
                      children_query: Query<&Children>,
                      content_container_query: Query<&Content>,
                      titlebar_query: Query<&Titlebar>,
                      footer_query: Query<&Footer>,
                      computed_node_query: Query<&ComputedNode>,
                      primary_window: Query<(Entity, &Window), With<PrimaryWindow>>| {
                    let drag = trigger.event();
                    let Ok(title_bar) = title_bars.get(trigger.entity) else {
                        return;
                    };
                    let window_entity = title_bar.window_entity;

                    if let Ok(mut node) = nodes.get_mut(window_entity) {
                        let Ok((primary_entity, primary_window)) = primary_window.single()
                        else {
                            return;
                        };

                        let scale = primary_window.scale_factor();
                        let screen_w = primary_window.resolution.width();
                        let screen_h = primary_window.resolution.height();

                        let logical_delta = drag.delta / scale;

                        if let Val::Px(ref mut left) = node.left {
                            *left += logical_delta.x;
                        }
                        if let Val::Px(ref mut top) = node.top {
                            *top += logical_delta.y;
                        }

                        if let Ok(computed) = computed_node_query.get(window_entity) {
                            let node_w = computed.size.x * computed.inverse_scale_factor;
                            let node_h = computed.size.y * computed.inverse_scale_factor;
                            if let (Val::Px(left), Val::Px(top)) = (node.left, node.top) {
                                let center_x = left + node_w * 0.5;
                                let center_y = top + node_h * 0.5;
                                let half_out = center_x < 0.0
                                    || center_y < 0.0
                                    || center_x > screen_w
                                    || center_y > screen_h;

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
                                        primary_entity,
                                        primary_window,
                                    );
                                }
                            }
                        }
                    }
                },
            )
            .observe(
                move |_: On<Pointer<DragEnd>>,
                      window: Single<Entity, With<Window>>,
                      mut commands: Commands| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::Grab));
                },
            )
            .observe(
                move |_: On<Pointer<DragStart>>,
                      window: Single<Entity, With<Window>>,
                      mut commands: Commands| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::Grabbing));
                },
            )
            .observe(
                move |_: On<Pointer<Over>>,
                      window: Single<Entity, With<Window>>,
                      mut commands: Commands| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::Grab));
                },
            )
            .observe(
                move |_: On<Pointer<Out>>,
                      window: Single<Entity, With<Window>>,
                      mut commands: Commands| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::Default));
                },
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
        window_entity: Entity,
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
                move |trigger: On<Pointer<Click>>,
                      mut commands: Commands,
                      window: Single<Entity, With<Window>>,
                      close_btn_query: Query<&CloseButton>| {
                    if let Ok(close_btn) = close_btn_query.get(trigger.entity) {
                        commands.entity(close_btn.window_entity).despawn();
                    }
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::Default));
                },
            )
            .observe(
                move |mut trigger: On<Pointer<Over>>,
                      mut commands: Commands,
                      window: Single<Entity, With<Window>>| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::Pointer));
                    trigger.propagate(false);
                },
            )
            .observe(
                move |_: On<Pointer<Out>>,
                      mut commands: Commands,
                      window: Single<Entity, With<Window>>| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::Default));
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
        ResizeCorner { window_entity }
    }

    pub fn spawn(
        commands: &mut Commands,
        ctx: &UiContext,
        style: &UiWindowStyle,
        window_entity: Entity,
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
                        scale: Vec3::new(1.0, -1.0, 1.0),
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
                move |mut trigger: On<Pointer<Over>>,
                      mut commands: Commands,
                      window: Single<Entity, With<Window>>| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::NwseResize));
                    trigger.propagate(false);
                },
            )
            .observe(
                move |_: On<Pointer<Out>>,
                      mut commands: Commands,
                      window: Single<Entity, With<Window>>| {
                    commands
                        .entity(*window)
                        .insert(CursorIcon::System(SystemCursorIcon::Default));
                },
            )
            .observe(
                move |trigger: On<Pointer<Drag>>,
                      mut nodes: Query<&mut Node>,
                      computed: Query<&ComputedNode>,
                      corners: Query<&ResizeCorner>| {
                    let drag = trigger.event();

                    if let Ok(resize) = corners.get(trigger.entity) {
                        if let Ok(mut node) = nodes.get_mut(resize.window_entity) {
                            if let Ok(layout) = computed.get(resize.window_entity) {
                                let logical_size = layout.size() * layout.inverse_scale_factor();
                                let dx = drag.delta.x;
                                let dy = drag.delta.y;
                                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                                    return;
                                }

                                let curr_w = match node.width {
                                    Val::Px(px) => px,
                                    _ => logical_size.x,
                                };
                                let curr_h = match node.height {
                                    Val::Px(px) => px,
                                    _ => logical_size.y,
                                };

                                let new_w = (curr_w + dx).max(50.0);
                                let new_h = (curr_h + dy).max(50.0);

                                node.width = Val::Px(new_w);
                                node.height = Val::Px(new_h);
                            }
                        }
                    }
                },
            );
    }
}

fn detect_os_window_reentry(
    mut commands: Commands,
    primary_window_query: Query<(Entity, &Window), With<PrimaryWindow>>,
    promoted_windows: Query<Entity, With<OsWindow>>,
    children_query: Query<&Children>,
    os_window_query: Query<&OsWindow>,
    ui_window_query: Query<&UiWindow>,
    titlebar_query: Query<&Titlebar>,
    footer_query: Query<&Footer>,
    mut node_query: Query<&mut Node>,
) {
    if promoted_windows.is_empty() {
        return;
    }

    let Ok((primary_entity, _primary_window)) = primary_window_query.single() else {
        return;
    };

    let mut to_revert = Vec::new();

    WINIT_WINDOWS.with_borrow(|winit_windows| {
        let Some(primary_winit_window) = winit_windows.get_window(primary_entity) else {
            return;
        };

        let Ok(primary_position) = primary_winit_window.outer_position() else {
            return;
        };
        let primary_size = primary_winit_window.outer_size();

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
            let os_right = os_left + (os_size.width as i32) / 2;
            let os_bottom = os_top + (os_size.height as i32) / 2;

            let within_bounds = os_left > primary_left
                && os_top > primary_top
                && os_right < primary_right
                && os_bottom < primary_bottom;

            if within_bounds {
                to_revert.push(entity);
            }
        }
    });

    for entity in to_revert {
        UiWindow::revert_to_ui_window(
            entity,
            &mut commands,
            &children_query,
            &os_window_query,
            &ui_window_query,
            &titlebar_query,
            &footer_query,
            &mut node_query,
            primary_entity,
        );
    }
}

fn detect_os_window_promotion(
    mut commands: Commands,
    primary_window_query: Query<(Entity, &Window), With<PrimaryWindow>>,
    ui_windows: Query<(Entity, &UiWindow), Without<OsWindow>>,
    children_query: Query<&Children>,
    content_container_query: Query<&Content>,
    titlebar_query: Query<&Titlebar>,
    footer_query: Query<&Footer>,
    computed_node_query: Query<&ComputedNode>,
    mut node_query: Query<&mut Node>,
) {
    let Ok((primary_entity, primary_window)) = primary_window_query.single() else {
        return;
    };

    let screen_w = primary_window.resolution.width();
    let screen_h = primary_window.resolution.height();

    for (window_entity, ui_window) in ui_windows.iter() {
        if !ui_window.options.draggable {
            continue;
        }

        let (left, top) = {
            let Ok(node) = node_query.get(window_entity) else {
                continue;
            };
            let left = match node.left.clone() {
                Val::Px(value) => value,
                _ => continue,
            };
            let top = match node.top.clone() {
                Val::Px(value) => value,
                _ => continue,
            };
            (left, top)
        };

        let children = match children_query.get(window_entity) {
            Ok(children) => children,
            Err(_) => continue,
        };

        let mut content_entity = None;
        for child in children.iter() {
            if content_container_query.get(child).is_ok() {
                content_entity = Some(child);
                break;
            }
        }
        let Some(content_entity) = content_entity else {
            continue;
        };

        let Ok(layout) = computed_node_query.get(content_entity) else {
            continue;
        };
        let logical_size = layout.size() * layout.inverse_scale_factor;
        let width = logical_size.x;
        let height = logical_size.y;

        let center_x = left + width * 0.5;
        let center_y = top + height * 0.5;
        let out_of_bounds =
            center_x < 0.0 || center_y < 0.0 || center_x > screen_w || center_y > screen_h;

        if out_of_bounds {
            UiWindow::convert_to_os_window(
                window_entity,
                &mut commands,
                &children_query,
                &content_container_query,
                &titlebar_query,
                &footer_query,
                &computed_node_query,
                &mut node_query,
                primary_entity,
                primary_window,
            );
        }
    }
}
