use bevy::prelude::*;
use bevy::ui::UiTargetCamera;
use std::collections::HashMap;

use crate::file::settings::Settings;
use crate::file::theme::{fallback_instrument_key_palette, Themes};
use crate::scenes::MainCamera;
use crate::states::GameState;

const TIMELINE_WIDTH_PERCENT: f32 = 100.0;
const TIMELINE_HEIGHT_PERCENT: f32 = 75.0;
const TIMELINE_BLOCK_DURATION: f32 = 10.0;
const VISIBLE_BLOCKS: usize = 4;
const NOTE_DIAMETER_PX: f32 = 28.0;
const NOTE_FONT_SIZE: f32 = 16.0;
const BLOCK_GAP_PX: f32 = 16.0;
const BLOCK_PADDING_PX: f32 = 12.0;
const OVERLAY_ALPHA: f32 = 0.60;

pub struct StringTimelinePlugin;

impl Plugin for StringTimelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StringTimelineFeed>()
            .init_resource::<StringTimelineView>()
            .add_systems(OnEnter(GameState::InGame), setup_timeline_ui)
            .add_systems(OnExit(GameState::InGame), teardown_timeline_ui)
            .add_systems(Update, update_timeline.run_if(in_state(GameState::InGame)));
    }
}

#[derive(Resource, Default, Clone)]
pub struct StringTimelineFeed {
    pub string_count: usize,
    pub window_start: f32,
    pub window_end: f32,
    pub notes: Vec<TimelineNote>,
    pub current_time: f32,
}

impl StringTimelineFeed {
    pub fn window_length(&self) -> f32 {
        (self.window_end - self.window_start).max(f32::EPSILON)
    }
}

#[derive(Clone)]
pub struct TimelineNote {
    pub time: f32,
    pub sustain: f32,
    pub string_index: usize,
    pub fret: i32,
}

#[derive(Resource, Default)]
struct StringTimelineView {
    root: Option<Entity>,
    block_stack: Option<Entity>,
    indicator: Option<Entity>,
    blocks: Vec<BlockView>,
    cached_string_count: usize,
    base_block_index: i32,
    string_colors: Vec<Color>,
}

struct BlockView {
    index: i32,
    root: Entity,
    overlay: Entity,
    rows: Vec<BlockRow>,
}

struct BlockRow {
    note_container: Entity,
    rendered_notes: HashMap<NoteKey, Entity>,
}

#[derive(Component)]
struct TimelineRoot;

#[derive(Component)]
struct CurrentTimeMarker;

#[derive(Component)]
struct BlockOverlay;

#[derive(Component)]
struct BlockStack;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct NoteKey {
    time_bits: u32,
    string_index: usize,
    fret: i32,
}

impl NoteKey {
    fn new(note: &TimelineNote) -> Self {
        Self {
            time_bits: note.time.to_bits(),
            string_index: note.string_index,
            fret: note.fret,
        }
    }
}

fn setup_timeline_ui(
    mut commands: Commands,
    mut view: ResMut<StringTimelineView>,
    mut feed: ResMut<StringTimelineFeed>,
    main_camera: Res<MainCamera>,
    settings: Res<Settings>,
    themes: Res<Themes>,
) {
    feed.window_start = 0.0;
    feed.window_end = timeline_window_seconds();
    feed.current_time = 0.0;
    feed.string_count = 0;
    feed.notes.clear();

    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            UiTargetCamera(main_camera.ui_camera),
            TimelineRoot,
        ))
        .id();

    let timeline_area = commands
        .spawn(Node {
            width: Val::Percent(TIMELINE_WIDTH_PERCENT),
            height: Val::Percent(TIMELINE_HEIGHT_PERCENT),
            position_type: PositionType::Absolute,
            left: Val::Percent(0.0),
            top: Val::Percent(0.0),
            ..default()
        })
        .insert(BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.85)))
        .id();

    let block_stack = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Stretch,
                padding: UiRect::horizontal(Val::Px(48.0)),

                row_gap: Val::Px(BLOCK_GAP_PX),
                position_type: PositionType::Relative,
                ..default()
            },
            BlockStack,
        ))
        .id();

    let indicator = commands
        .spawn((
            Node {
                width: Val::Px(2.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                margin: UiRect {
                    left: Val::Px(-1.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                },
                ..default()
            },
            BackgroundColor(Color::srgb_u8(255, 90, 90)),
            CurrentTimeMarker,
            ZIndex(2),
        ))
        .id();

    commands.entity(block_stack).add_child(indicator);

    commands.entity(root).add_child(timeline_area);
    commands
        .entity(timeline_area)
        .add_child(block_stack)
        .add_child(indicator);

    view.root = Some(root);
    view.block_stack = Some(block_stack);
    view.indicator = Some(indicator);
    view.blocks.clear();
    view.cached_string_count = 0;
    view.base_block_index = 0;
    view.string_colors = resolve_string_palette(&settings, &themes);
}

fn teardown_timeline_ui(mut commands: Commands, mut view: ResMut<StringTimelineView>) {
    if let Some(root) = view.root.take() {
        commands.entity(root).despawn();
    }
    view.block_stack = None;
    view.indicator = None;
    view.blocks.clear();
    view.cached_string_count = 0;
    view.base_block_index = 0;
    view.string_colors.clear();
}

fn update_timeline(
    mut commands: Commands,
    feed: Res<StringTimelineFeed>,
    mut view: ResMut<StringTimelineView>,
    children_query: Query<&Children>,
    mut node_query: Query<&mut Node>,
) {
    let Some(block_stack) = view.block_stack else {
        return;
    };

    if feed.string_count == 0 {
        clear_all_blocks(&mut commands, &mut view);
        reset_indicator(&view, &mut node_query);
        return;
    }

    if feed.string_count != view.cached_string_count {
        clear_all_blocks(&mut commands, &mut view);
        view.cached_string_count = feed.string_count;
    }

    let block_duration = TIMELINE_BLOCK_DURATION;
    let current_block_index = (feed.current_time / block_duration).floor().max(0.0) as i32;
    let block_progress = ((feed.current_time - current_block_index as f32 * block_duration)
        / block_duration)
        .clamp(0.0, 1.0);

    if view.blocks.is_empty() {
        view.base_block_index = current_block_index;
    }

    if current_block_index < view.base_block_index {
        view.base_block_index = current_block_index;
    }

    let visible_blocks = VISIBLE_BLOCKS.max(1) as i32;
    let last_block_index = view.base_block_index + visible_blocks - 1;
    let threshold_block = view.base_block_index + (visible_blocks - 2).max(0);

    if current_block_index > last_block_index {
        view.base_block_index = current_block_index - (visible_blocks - 1);
    } else if current_block_index >= threshold_block && block_progress >= 0.95 {
        view.base_block_index = (view.base_block_index + 1).max(0);
    }

    ensure_blocks(&mut commands, &mut view, block_stack, feed.string_count);

    render_notes(
        &mut commands,
        &mut view,
        &feed,
        current_block_index,
        &children_query,
    );
    let view_ref: &StringTimelineView = &view;
    update_overlays(
        view_ref,
        current_block_index,
        block_progress,
        &mut node_query,
    );
    update_indicator(
        &mut commands,
        view_ref,
        current_block_index,
        block_progress,
        &mut node_query,
    );
}

fn ensure_blocks(
    commands: &mut Commands,
    view: &mut StringTimelineView,
    block_stack: Entity,
    string_count: usize,
) {
    let mut desired_indices = Vec::new();
    for offset in 0..VISIBLE_BLOCKS {
        desired_indices.push(view.base_block_index + offset as i32);
    }

    view.blocks.retain(|block| {
        if desired_indices.contains(&block.index) {
            true
        } else {
            commands.entity(block.root).despawn();
            false
        }
    });

    for index in desired_indices {
        if !view.blocks.iter().any(|block| block.index == index) {
            let block = spawn_block(
                commands,
                block_stack,
                index,
                string_count,
                &view.string_colors,
            );
            view.blocks.push(block);
        }
    }

    view.blocks.sort_by_key(|block| block.index);

    let ordered_children: Vec<Entity> = view.blocks.iter().map(|block| block.root).collect();
    commands
        .entity(block_stack)
        .replace_children(&ordered_children);
}

fn spawn_block(
    commands: &mut Commands,
    parent: Entity,
    index: i32,
    string_count: usize,
    string_colors: &[Color],
) -> BlockView {
    let block_root = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Auto,
            flex_grow: 1.0,
            position_type: PositionType::Relative,
            padding: UiRect::all(Val::Px(BLOCK_PADDING_PX)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        })
        .insert(BackgroundColor(Color::srgba(0.07, 0.07, 0.1, 0.85)))
        .insert(BorderColor::all(Color::srgba(0.25, 0.25, 0.3, 1.0)))
        .id();

    commands.entity(parent).add_child(block_root);

    let overlay = commands
        .spawn((
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, OVERLAY_ALPHA)),
            BlockOverlay,
            ZIndex(1),
        ))
        .id();

    commands.entity(block_root).add_child(overlay);

    let rows_container = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Stretch,
            position_type: PositionType::Relative,
            ..default()
        })
        .id();

    commands.entity(block_root).add_child(rows_container);

    let mut rows = Vec::new();
    for string_idx in 0..string_count {
        let row_entity = commands
            .spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_grow: 1.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                position_type: PositionType::Relative,
                ..default()
            })
            .id();

        let string_line = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(
                    string_colors
                        .get(string_idx % string_colors.len())
                        .copied()
                        .unwrap_or(Color::srgb(0.5, 0.5, 0.5))
                        .with_alpha(0.45),
                ),
            ))
            .id();

        let note_container = commands
            .spawn((Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },))
            .id();

        commands
            .entity(row_entity)
            .add_child(string_line)
            .add_child(note_container);
        commands.entity(rows_container).add_child(row_entity);

        rows.push(BlockRow {
            note_container,
            rendered_notes: HashMap::new(),
        });
    }

    BlockView {
        index,
        root: block_root,
        overlay,
        rows,
    }
}

fn render_notes(
    commands: &mut Commands,
    view: &mut StringTimelineView,
    feed: &StringTimelineFeed,
    current_block_index: i32,
    children_query: &Query<&Children>,
) {
    let block_duration = TIMELINE_BLOCK_DURATION;
    let string_colors = view.string_colors.clone();
    let default_string_color = Color::srgb(0.235, 0.549, 1.0);

    for block in &mut view.blocks {
        if block.rows.is_empty() {
            continue;
        }

        let block_start = block.index as f32 * block_duration;
        let block_end = block_start + block_duration;
        let block_is_past = block.index < current_block_index;

        let mut desired_by_string: Vec<Vec<(NoteKey, f32, &TimelineNote)>> =
            vec![Vec::new(); block.rows.len()];

        for note in &feed.notes {
            if note.time < block_start || note.time >= block_end {
                continue;
            }

            if note.string_index >= block.rows.len() {
                continue;
            }

            let progress = ((note.time - block_start) / block_duration).clamp(0.0, 1.0);
            let left_percent = progress * 100.0;
            let key = NoteKey::new(note);
            desired_by_string[note.string_index].push((key, left_percent, note));
        }

        for (string_idx, row) in block.rows.iter_mut().enumerate() {
            let desired = desired_by_string
                .get(string_idx)
                .map(|entries| entries.as_slice())
                .unwrap_or_default();

            let mut desired_keys = Vec::with_capacity(desired.len());

            for (key, left_percent, note) in desired {
                desired_keys.push(*key);
                if row.rendered_notes.contains_key(key) {
                    continue;
                }

                let palette_color = if string_colors.is_empty() {
                    default_string_color
                } else {
                    string_colors[string_idx % string_colors.len()]
                };
                let note_color = palette_color;

                let note_entity = commands
                    .spawn((
                        Node {
                            width: Val::Px(NOTE_DIAMETER_PX),
                            height: Val::Px(NOTE_DIAMETER_PX),
                            position_type: PositionType::Absolute,
                            left: Val::Percent(left_percent.min(100.0)),
                            top: Val::Percent(50.0),
                            margin: UiRect {
                                left: Val::Px(-(NOTE_DIAMETER_PX / 2.0)),
                                right: Val::Px(0.0),
                                top: Val::Px(-(NOTE_DIAMETER_PX / 2.0)),
                                bottom: Val::Px(0.0),
                            },
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(note_color),
                        BorderRadius::all(Val::Px(NOTE_DIAMETER_PX / 2.0)),
                    ))
                    .id();

                commands.entity(note_entity).with_children(|parent| {
                    parent.spawn((
                        Text::new(note.fret.to_string()),
                        TextFont {
                            font_size: NOTE_FONT_SIZE,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

                commands.entity(row.note_container).add_child(note_entity);

                row.rendered_notes.insert(*key, note_entity);
            }

            if !block_is_past {
                let mut stale_keys = Vec::new();
                for key in row.rendered_notes.keys() {
                    if !desired_keys.contains(key) {
                        stale_keys.push(*key);
                    }
                }

                for key in stale_keys {
                    if let Some(entity) = row.rendered_notes.remove(&key) {
                        if let Ok(children) = children_query.get(entity) {
                            for child in children.iter() {
                                commands.entity(child.clone()).despawn();
                            }
                        }
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }
}

fn update_overlays(
    view: &StringTimelineView,
    current_block_index: i32,
    block_progress: f32,
    node_query: &mut Query<&mut Node>,
) {
    for block in &view.blocks {
        let Some(mut node) = node_query.get_mut(block.overlay).ok() else {
            continue;
        };

        let coverage = if block.index < current_block_index {
            1.0
        } else if block.index == current_block_index {
            block_progress
        } else {
            0.0
        };

        node.width = Val::Percent((coverage * 100.0).clamp(0.0, 100.0));
    }
}

fn update_indicator(
    commands: &mut Commands,
    view: &StringTimelineView,
    current_block_index: i32,
    block_progress: f32,
    node_query: &mut Query<&mut Node>,
) {
    let Some(indicator) = view.indicator else {
        return;
    };

    let maybe_block = view
        .blocks
        .iter()
        .find(|block| block.index == current_block_index);

    if let Some(block) = maybe_block {
        commands.entity(block.root).add_child(indicator);
    }

    if let Ok(mut node) = node_query.get_mut(indicator) {
        if maybe_block.is_some() {
            node.top = Val::Px(0.0);
            node.height = Val::Percent(100.0);
        }
        let width_percent = (block_progress * 100.0).clamp(0.0, 100.0);
        node.left = Val::Percent(width_percent);
    }
}

fn reset_indicator(view: &StringTimelineView, node_query: &mut Query<&mut Node>) {
    if let Some(indicator) = view.indicator {
        if let Ok(mut node) = node_query.get_mut(indicator) {
            node.left = Val::Percent(0.0);
            node.top = Val::Px(0.0);
            node.height = Val::Percent(100.0);
        }
    }
}

fn clear_all_blocks(commands: &mut Commands, view: &mut StringTimelineView) {
    for block in view.blocks.drain(..) {
        clear_block(commands, block, view.indicator, view.block_stack);
    }
    view.base_block_index = 0;
}

fn clear_block(
    commands: &mut Commands,
    block: BlockView,
    indicator: Option<Entity>,
    fallback_parent: Option<Entity>,
) {
    if let (Some(indicator), Some(parent)) = (indicator, fallback_parent) {
        commands.entity(parent).add_child(indicator);
    }
    commands.entity(block.root).despawn();
}

fn resolve_string_palette(settings: &Settings, themes: &Themes) -> Vec<Color> {
    let theme_name = &settings.start_theme;
    if let Some(theme) = themes.get(theme_name) {
        if !theme.instrument_keys.is_empty() {
            return theme.instrument_keys.clone();
        }
    }

    if let Some((_name, theme)) = themes.themes.iter().next() {
        if !theme.instrument_keys.is_empty() {
            return theme.instrument_keys.clone();
        }
    }

    fallback_instrument_key_palette()
}

pub fn timeline_window_seconds() -> f32 {
    TIMELINE_BLOCK_DURATION * VISIBLE_BLOCKS as f32
}

pub fn timeline_block_duration() -> f32 {
    TIMELINE_BLOCK_DURATION
}
