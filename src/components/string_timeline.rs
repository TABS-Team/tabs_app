use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiTargetCamera};
use std::collections::{HashMap, HashSet};

use crate::file::settings::Settings;
use crate::file::song::Techniques;
use crate::file::theme::{fallback_instrument_key_palette, Themes};
use crate::scenes::MainCamera;
use crate::states::GameState;

const TIMELINE_WIDTH_PERCENT: f32 = 100.0;
const TIMELINE_HEIGHT_PERCENT: f32 = 75.0;
const DEFAULT_BLOCK_DURATION: f32 = 10.0;
const MIN_BLOCK_DURATION: f32 = 1.2;
const MAX_BLOCK_DURATION: f32 = 14.0;
const TARGET_NOTE_SPACING_PERCENT: f32 = 0.12;
const VISIBLE_BLOCKS: usize = 4;
const NOTE_DIAMETER_PX: f32 = 28.0;
const NOTE_FONT_SIZE: f32 = 16.0;
const BLOCK_GAP_PX: f32 = 16.0;
const BLOCK_PADDING_PX: f32 = 12.0;
const OVERLAY_ALPHA: f32 = 0.60;
const BLOCK_SHIFT_DURATION: f32 = 0.18;
const FRET_VIEW_HEIGHT_PERCENT: f32 = 100.0 - TIMELINE_HEIGHT_PERCENT;
const FRET_MARKER_DIAMETER_PX: f32 = 36.0;
const FRET_MARKER_SECONDARY_SCALE: f32 = 0.7;
const MIN_FRET_SPAN: i32 = 4;
const NOTE_GROUP_TOLERANCE: f32 = 0.08;
const FRET_ZOOM_DURATION: f32 = 0.28;
const FRET_ZOOM_START_SCALE: f32 = 0.85;
const SUSTAIN_LINE_HEIGHT_PX: f32 = 6.0;
const SLIDE_LINE_HEIGHT_PX: f32 = 8.0;
const SLIDE_ARROW_FONT_SIZE: f32 = 18.0;
const SLIDE_TARGET_DIAMETER_PX: f32 = 22.0;

pub struct StringTimelinePlugin;

impl Plugin for StringTimelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StringTimelineFeed>()
            .init_resource::<TechniqueVisualizationRegistry>()
            .init_resource::<StringTimelineView>()
            .add_systems(OnEnter(GameState::InGame), setup_timeline_ui)
            .add_systems(OnExit(GameState::InGame), teardown_timeline_ui)
            .add_systems(Update, update_timeline.run_if(in_state(GameState::InGame)));
    }
}

#[derive(Resource, Clone)]
pub struct StringTimelineFeed {
    pub string_count: usize,
    pub window_start: f32,
    pub window_end: f32,
    pub notes: Vec<TimelineNote>,
    pub current_time: f32,
    pub block_duration: f32,
    pub block_duration_locked: bool,
}

impl StringTimelineFeed {
    pub fn window_length(&self) -> f32 {
        (self.window_end - self.window_start).max(f32::EPSILON)
    }
}

impl Default for StringTimelineFeed {
    fn default() -> Self {
        let block_duration = DEFAULT_BLOCK_DURATION;
        Self {
            string_count: 0,
            window_start: 0.0,
            window_end: block_duration * VISIBLE_BLOCKS as f32,
            notes: Vec::new(),
            current_time: 0.0,
            block_duration,
            block_duration_locked: false,
        }
    }
}

#[derive(Clone)]
pub struct TimelineNote {
    pub time: f32,
    pub sustain: f32,
    pub string_index: usize,
    pub fret: i32,
    pub techniques: Vec<Techniques>,
    pub additional_frets: Vec<i32>,
    pub slide_target: Option<i32>,
    pub slide_unpitched_target: Option<i32>,
}

impl TimelineNote {
    fn primary_slide_target(&self) -> Option<i32> {
        self.slide_target.or(self.slide_unpitched_target)
    }

    fn is_slide(&self) -> bool {
        self.techniques
            .iter()
            .any(|technique| matches!(technique, Techniques::Slide))
            && self.slide_target.or(self.slide_unpitched_target).is_some()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct NoteKey {
    time_bits: u32,
    string_index: usize,
    fret: i32,
    metadata_hash: u64,
}

impl NoteKey {
    fn new(note: &TimelineNote) -> Self {
        let mut technique_hash = 0u64;
        for technique in &note.techniques {
            technique_hash = technique_hash
                .wrapping_mul(31)
                .wrapping_add((*technique as u8 as u64) + 1);
        }
        let mut extra_hash = 0u64;
        for extra in &note.additional_frets {
            extra_hash = extra_hash
                .wrapping_mul(41)
                .wrapping_add((*extra as i64 + 64) as u64);
        }
        let mut slide_hash = 0u64;
        if let Some(target) = note.slide_target {
            slide_hash = slide_hash
                .wrapping_mul(53)
                .wrapping_add((target as i64 + 64) as u64);
        }
        if let Some(target) = note.slide_unpitched_target {
            slide_hash = slide_hash
                .wrapping_mul(67)
                .wrapping_add((target as i64 + 64) as u64);
        }
        let metadata_hash = technique_hash.wrapping_mul(131).wrapping_add(extra_hash);
        Self {
            time_bits: note.time.to_bits(),
            string_index: note.string_index,
            fret: note.fret,
            metadata_hash: metadata_hash.wrapping_mul(73).wrapping_add(slide_hash),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum FretMarkerRole {
    Primary,
    Additional(i32),
    SlideBar,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SustainSegmentKey {
    note: NoteKey,
    block_index: i32,
}

impl SustainSegmentKey {
    fn new(note: &TimelineNote, block_index: i32) -> Self {
        SustainSegmentKey {
            note: NoteKey::new(note),
            block_index,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FretMarkerKey {
    note: NoteKey,
    role: FretMarkerRole,
}

impl FretMarkerKey {
    fn primary(note: &TimelineNote) -> Self {
        FretMarkerKey {
            note: NoteKey::new(note),
            role: FretMarkerRole::Primary,
        }
    }

    fn additional(note: &TimelineNote, fret: i32) -> Self {
        FretMarkerKey {
            note: NoteKey::new(note),
            role: FretMarkerRole::Additional(fret),
        }
    }

    fn slide_bar(note: &TimelineNote) -> Self {
        FretMarkerKey {
            note: NoteKey::new(note),
            role: FretMarkerRole::SlideBar,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FretRange {
    start: i32,
    end: i32,
}

impl FretRange {
    fn new(mut start: i32, mut end: i32) -> Self {
        if start > end {
            std::mem::swap(&mut start, &mut end);
        }
        while end - start < MIN_FRET_SPAN {
            if start > 0 {
                start -= 1;
            }
            if end - start >= MIN_FRET_SPAN {
                break;
            }
            end += 1;
        }
        FretRange { start, end }
    }

    fn span(&self) -> usize {
        (self.end - self.start + 1).max(1) as usize
    }
}

struct FretZoomAnimation {
    duration: f32,
    elapsed: f32,
    start_scale: f32,
    target_scale: f32,
}

impl FretZoomAnimation {
    fn new(start_scale: f32, target_scale: f32, duration: f32) -> Self {
        FretZoomAnimation {
            duration,
            elapsed: 0.0,
            start_scale,
            target_scale,
        }
    }
}
#[derive(Resource, Default)]
struct StringTimelineView {
    root: Option<Entity>,
    block_stack: Option<Entity>,
    indicator: Option<Entity>,
    indicator_block_index: Option<i32>,
    indicator_block_progress: f32,
    blocks: Vec<BlockView>,
    cached_string_count: usize,
    base_block_index: i32,
    string_colors: Vec<Color>,
    shift_animation: Option<ShiftAnimation>,
    fret_area: Option<Entity>,
    fret_neck: Option<Entity>,
    fret_grid_layer: Option<Entity>,
    fret_marker_layer: Option<Entity>,
    fret_string_layer: Option<Entity>,
    fret_label: Option<Entity>,
    fret_string_lines: Vec<Entity>,
    fret_fret_lines: Vec<Entity>,
    fret_markers: HashMap<FretMarkerKey, Entity>,
    fret_current_signature: Vec<NoteKey>,
    fret_current_range: Option<FretRange>,
    fret_zoom_animation: Option<FretZoomAnimation>,
}

struct BlockView {
    index: i32,
    root: Entity,
    overlay: Entity,
    fade_overlay: Entity,
    rows: Vec<BlockRow>,
    is_removing: bool,
    stored_height: Option<f32>,
}

struct BlockRow {
    note_container: Entity,
    rendered_notes: HashMap<NoteKey, Entity>,
    rendered_sustain_segments: HashMap<SustainSegmentKey, Entity>,
    rendered_slide_segments: HashMap<SustainSegmentKey, SlideSegmentView>,
}

struct SlideSegmentView {
    line: Entity,
    target_circle: Option<Entity>,
}

struct SlideSegmentRender<'a> {
    key: SustainSegmentKey,
    left_percent: f32,
    width_percent: f32,
    terminal: bool,
    terminal_percent: f32,
    note: &'a TimelineNote,
    target_fret: Option<i32>,
}

#[derive(Component)]
struct TimelineRoot;

#[derive(Component)]
struct CurrentTimeMarker;

#[derive(Component)]
struct BlockOverlay;

#[derive(Component)]
struct BlockStack;

#[derive(Component)]
struct FretViewRoot;

#[derive(Component)]
struct FretNeckNode;

#[derive(Component)]
struct FretMarkerLayerNode;

#[derive(Component)]
struct FretStringLayerNode;

#[derive(Component)]
struct FretGridLayerNode;

#[derive(Component)]
struct FretMarker;

struct ShiftAnimation {
    target_base_index: i32,
    removing_index: i32,
    duration: f32,
    elapsed: f32,
    initial_height: f32,
    initial_padding: f32,
}

#[derive(Clone, Copy)]
struct NoteVisualStyle {
    timeline_background: Option<Color>,
    timeline_border_color: Option<Color>,
    timeline_border_width: Option<f32>,
    fret_background: Option<Color>,
    fret_border_color: Option<Color>,
    fret_border_width: Option<f32>,
}

impl NoteVisualStyle {
    fn merged_with(self, other: NoteVisualStyle) -> NoteVisualStyle {
        NoteVisualStyle {
            timeline_background: other.timeline_background.or(self.timeline_background),
            timeline_border_color: other.timeline_border_color.or(self.timeline_border_color),
            timeline_border_width: other.timeline_border_width.or(self.timeline_border_width),
            fret_background: other.fret_background.or(self.fret_background),
            fret_border_color: other.fret_border_color.or(self.fret_border_color),
            fret_border_width: other.fret_border_width.or(self.fret_border_width),
        }
    }
}

impl Default for NoteVisualStyle {
    fn default() -> Self {
        NoteVisualStyle {
            timeline_background: None,
            timeline_border_color: None,
            timeline_border_width: None,
            fret_background: None,
            fret_border_color: None,
            fret_border_width: None,
        }
    }
}

#[derive(Resource)]
struct TechniqueVisualizationRegistry {
    fallback: NoteVisualStyle,
    styles: HashMap<Techniques, NoteVisualStyle>,
}

impl TechniqueVisualizationRegistry {
    fn style_for(&self, techniques: &[Techniques]) -> NoteVisualStyle {
        let mut style = self.fallback;
        for technique in techniques {
            if let Some(entry) = self.styles.get(technique) {
                style = style.merged_with(*entry);
            }
        }
        style
    }
}

impl Default for TechniqueVisualizationRegistry {
    fn default() -> Self {
        let mut styles = HashMap::new();

        styles.insert(
            Techniques::Slide,
            NoteVisualStyle {
                timeline_border_color: Some(Color::srgb(0.85, 0.7, 0.25)),
                timeline_border_width: Some(3.0),
                fret_border_color: Some(Color::srgb(0.9, 0.75, 0.3)),
                fret_border_width: Some(3.0),
                ..default()
            },
        );

        styles.insert(
            Techniques::Bend,
            NoteVisualStyle {
                timeline_background: Some(Color::srgb(0.6, 0.3, 0.9)),
                fret_background: Some(Color::srgb(0.55, 0.25, 0.85)),
                ..default()
            },
        );

        styles.insert(
            Techniques::HammerOn,
            NoteVisualStyle {
                timeline_background: Some(Color::srgb(0.25, 0.7, 0.85)),
                fret_border_color: Some(Color::srgb(0.2, 0.65, 0.8)),
                fret_border_width: Some(2.0),
                ..default()
            },
        );

        styles.insert(
            Techniques::PalmMute,
            NoteVisualStyle {
                timeline_background: Some(Color::srgb(0.35, 0.35, 0.35)),
                fret_background: Some(Color::srgb(0.25, 0.25, 0.25)),
                ..default()
            },
        );

        Self {
            fallback: NoteVisualStyle::default(),
            styles,
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
    feed.block_duration = DEFAULT_BLOCK_DURATION;
    feed.block_duration_locked = false;
    feed.window_end = feed.block_duration * VISIBLE_BLOCKS as f32;
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

    let fret_area = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(FRET_VIEW_HEIGHT_PERCENT.max(0.0)),
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(TIMELINE_HEIGHT_PERCENT),
                padding: UiRect {
                    left: Val::Px(48.0),
                    right: Val::Px(48.0),
                    top: Val::Px(16.0),
                    bottom: Val::Px(24.0),
                },
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.07, 0.92)),
        ))
        .id();

    let fret_root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Stretch,
                row_gap: Val::Px(12.0),
                ..default()
            },
            FretViewRoot,
        ))
        .id();

    let fret_neck = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(75.0),
                position_type: PositionType::Relative,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Stretch,
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.21, 0.16, 0.12, 0.95)),
            BorderColor::all(Color::srgba(0.12, 0.09, 0.07, 1.0)),
            FretNeckNode,
        ))
        .id();

    let fret_string_layer = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Stretch,
                position_type: PositionType::Relative,
                ..default()
            },
            FretStringLayerNode,
            ZIndex(2),
        ))
        .id();

    let fret_grid_layer = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                ..default()
            },
            FretGridLayerNode,
            ZIndex(1),
        ))
        .id();

    let fret_marker_layer = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                ..default()
            },
            FretMarkerLayerNode,
            ZIndex(2),
        ))
        .id();

    commands
        .entity(fret_neck)
        .add_child(fret_grid_layer)
        .add_child(fret_string_layer)
        .add_child(fret_marker_layer);

    let fret_label_container = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Auto,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .id();

    let fret_label = commands
        .spawn((
            Text::new(""),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.95)),
        ))
        .id();

    commands.entity(fret_label_container).add_child(fret_label);

    commands
        .entity(fret_root)
        .add_child(fret_neck)
        .add_child(fret_label_container);

    commands.entity(fret_area).add_child(fret_root);

    commands.entity(root).add_child(timeline_area);
    commands.entity(root).add_child(fret_area);
    commands
        .entity(timeline_area)
        .add_child(block_stack)
        .add_child(indicator);

    view.root = Some(root);
    view.block_stack = Some(block_stack);
    view.indicator = Some(indicator);
    view.indicator_block_index = None;
    view.indicator_block_progress = 0.0;
    view.blocks.clear();
    view.cached_string_count = 0;
    view.base_block_index = 0;
    view.string_colors = resolve_string_palette(&settings, &themes);
    view.shift_animation = None;
    view.fret_area = Some(fret_area);
    view.fret_neck = Some(fret_neck);
    view.fret_grid_layer = Some(fret_grid_layer);
    view.fret_marker_layer = Some(fret_marker_layer);
    view.fret_string_layer = Some(fret_string_layer);
    view.fret_label = Some(fret_label);
    view.fret_string_lines.clear();
    view.fret_fret_lines.clear();
    view.fret_markers.clear();
    view.fret_current_signature.clear();
    view.fret_current_range = None;
    view.fret_zoom_animation = None;
}

fn teardown_timeline_ui(mut commands: Commands, mut view: ResMut<StringTimelineView>) {
    if let Some(root) = view.root.take() {
        commands.entity(root).despawn();
    }
    for entity in view.fret_string_lines.drain(..) {
        commands.entity(entity).despawn();
    }
    for (_, entity) in view.fret_markers.drain() {
        commands.entity(entity).despawn();
    }
    for entity in view.fret_fret_lines.drain(..) {
        commands.entity(entity).despawn();
    }
    view.block_stack = None;
    view.indicator = None;
    view.fret_area = None;
    view.fret_neck = None;
    view.fret_grid_layer = None;
    view.fret_marker_layer = None;
    view.fret_string_layer = None;
    view.fret_label = None;
    view.blocks.clear();
    view.cached_string_count = 0;
    view.base_block_index = 0;
    view.string_colors.clear();
    view.shift_animation = None;
    view.fret_string_lines.clear();
    view.fret_markers.clear();
    view.fret_fret_lines.clear();
    view.fret_current_signature.clear();
    view.fret_current_range = None;
    view.fret_zoom_animation = None;
}

fn update_timeline(
    mut commands: Commands,
    time: Res<Time>,
    feed: Res<StringTimelineFeed>,
    technique_registry: Res<TechniqueVisualizationRegistry>,
    mut view: ResMut<StringTimelineView>,
    mut node_query: Query<&mut Node>,
    mut background_query: Query<&mut BackgroundColor>,
    mut text_query: Query<&mut Text>,
    mut transform_query: Query<&mut Transform>,
    computed_query: Query<&ComputedNode>,
) {
    let Some(block_stack) = view.block_stack else {
        return;
    };

    let delta_seconds = time.delta_secs();
    progress_shift_animation(
        &mut commands,
        &mut view,
        delta_seconds,
        &mut node_query,
        &mut background_query,
    );

    let string_count_zero = feed.string_count == 0;

    if string_count_zero {
        clear_all_blocks(&mut commands, &mut view);
        reset_indicator(&view, &mut node_query);
    } else {
        if feed.string_count != view.cached_string_count {
            clear_all_blocks(&mut commands, &mut view);
            view.cached_string_count = feed.string_count;
        }

        let block_duration = feed.block_duration.max(MIN_BLOCK_DURATION);
        let current_block_index = (feed.current_time / block_duration).floor().max(0.0) as i32;
        let block_progress = ((feed.current_time - current_block_index as f32 * block_duration)
            / block_duration)
            .clamp(0.0, 1.0);

        if view.blocks.is_empty() {
            view.base_block_index = current_block_index;
        }

        if current_block_index < view.base_block_index && view.shift_animation.is_none() {
            view.base_block_index = current_block_index;
        }

        if view.shift_animation.is_none() {
            let visible_blocks = VISIBLE_BLOCKS.max(1) as i32;
            let last_block_index = view.base_block_index + visible_blocks - 1;
            let threshold_block = view.base_block_index + (visible_blocks - 2).max(0);
            let mut desired_base = view.base_block_index;

            if current_block_index > last_block_index {
                desired_base = current_block_index - (visible_blocks - 1);
            } else if current_block_index >= threshold_block && block_progress >= 0.95 {
                desired_base = (view.base_block_index + 1).max(0);
            }

            if desired_base > view.base_block_index {
                let started = start_shift_animation(
                    &mut view,
                    desired_base,
                    &mut node_query,
                    &mut background_query,
                    &computed_query,
                );
                if !started {
                    view.base_block_index = desired_base;
                }
            } else {
                view.base_block_index = desired_base;
            }
        } else if view.shift_animation.is_some() {
            let visible_blocks = VISIBLE_BLOCKS.max(1) as i32;
            let last_block_index = view.base_block_index + visible_blocks - 1;
            if current_block_index > last_block_index {
                if let Some(animation) = view.shift_animation.as_mut() {
                    animation.target_base_index = (current_block_index - (visible_blocks - 1))
                        .max(animation.target_base_index);
                }
            }
        }

        ensure_blocks(&mut commands, &mut view, block_stack, feed.string_count);

        render_notes(
            &mut commands,
            &mut view,
            &feed,
            current_block_index,
            technique_registry.as_ref(),
        );
        let overlay_progress = if let Some(idx) = view.indicator_block_index {
            if idx == current_block_index {
                view.indicator_block_progress
            } else if idx < current_block_index {
                1.0
            } else {
                block_progress
            }
        } else {
            block_progress
        };

        update_overlays(
            &view,
            current_block_index,
            overlay_progress,
            &mut node_query,
        );
        update_indicator(
            &mut commands,
            &mut view,
            current_block_index,
            block_progress,
            &mut node_query,
        );
    }

    update_fret_view(
        &mut commands,
        &mut view,
        &feed,
        technique_registry.as_ref(),
        delta_seconds,
        &mut node_query,
        &mut text_query,
        &mut transform_query,
    );

    if string_count_zero {
        return;
    }
}

fn start_shift_animation(
    view: &mut StringTimelineView,
    target_base_index: i32,
    node_query: &mut Query<&mut Node>,
    background_query: &mut Query<&mut BackgroundColor>,
    computed_query: &Query<&ComputedNode>,
) -> bool {
    if view.shift_animation.is_some() {
        return false;
    }

    let removing_index = view.base_block_index;
    if !view
        .blocks
        .iter()
        .any(|block| block.index == removing_index && !block.is_removing)
    {
        return false;
    }

    let mut initial_height = None;
    for block in &mut view.blocks {
        let is_target = block.index == removing_index;
        let Some(height) = freeze_block_layout(block, node_query, computed_query, is_target) else {
            continue;
        };

        if is_target {
            if let Ok(mut node) = node_query.get_mut(block.root) {
                node.padding = UiRect::all(Val::Px(BLOCK_PADDING_PX));
            }
            if let Ok(mut fade_color) = background_query.get_mut(block.fade_overlay) {
                fade_color.0.set_alpha(0.0);
            }
            block.is_removing = true;
            initial_height = Some(height);
        }
    }

    let Some(initial_height) = initial_height else {
        unfreeze_remaining_blocks(view, node_query);
        return false;
    };

    view.shift_animation = Some(ShiftAnimation {
        target_base_index,
        removing_index,
        duration: BLOCK_SHIFT_DURATION,
        elapsed: 0.0,
        initial_height,
        initial_padding: BLOCK_PADDING_PX,
    });

    true
}

fn progress_shift_animation(
    commands: &mut Commands,
    view: &mut StringTimelineView,
    delta_seconds: f32,
    node_query: &mut Query<&mut Node>,
    background_query: &mut Query<&mut BackgroundColor>,
) {
    let Some(animation) = view.shift_animation.as_mut() else {
        return;
    };

    animation.elapsed += delta_seconds;
    let progress = (animation.elapsed / animation.duration).clamp(0.0, 1.0);

    let Some(position) = view
        .blocks
        .iter()
        .position(|block| block.index == animation.removing_index)
    else {
        view.base_block_index = animation.target_base_index;
        view.shift_animation = None;
        unfreeze_remaining_blocks(view, node_query);
        return;
    };

    if let Some(block) = view.blocks.get_mut(position) {
        if let Ok(mut node) = node_query.get_mut(block.root) {
            let current_height = animation.initial_height * (1.0 - progress);
            node.height = Val::Px(current_height.max(0.0));
            node.max_height = Val::Px(current_height.max(0.0));
            let padding = animation.initial_padding * (1.0 - progress);
            node.padding = UiRect::all(Val::Px(padding.max(0.0)));
        }

        if let Ok(mut fade_color) = background_query.get_mut(block.fade_overlay) {
            fade_color.0.set_alpha(progress.clamp(0.0, 1.0));
        }
    }

    if progress >= 1.0 {
        if position < view.blocks.len() {
            let block = view.blocks.remove(position);
            clear_block(commands, block, view.indicator, view.block_stack);
        }
        view.base_block_index = animation.target_base_index;
        view.shift_animation = None;
        unfreeze_remaining_blocks(view, node_query);
    }
}

fn freeze_block_layout(
    block: &mut BlockView,
    node_query: &mut Query<&mut Node>,
    computed_query: &Query<&ComputedNode>,
    is_removing: bool,
) -> Option<f32> {
    let height = computed_query
        .get(block.root)
        .map(|computed| computed.size().y)
        .unwrap_or(0.0)
        .max(1.0);

    if let Ok(mut node) = node_query.get_mut(block.root) {
        node.flex_grow = 0.0;
        node.height = Val::Px(height);
        node.min_height = if is_removing {
            Val::Px(0.0)
        } else {
            Val::Px(height)
        };
        node.max_height = Val::Px(height);
    }

    block.stored_height = Some(height);
    Some(height)
}

fn unfreeze_remaining_blocks(view: &mut StringTimelineView, node_query: &mut Query<&mut Node>) {
    for block in &mut view.blocks {
        block.is_removing = false;
        if let Ok(mut node) = node_query.get_mut(block.root) {
            node.flex_grow = 1.0;
            node.height = Val::Auto;
            node.min_height = Val::Auto;
            node.max_height = Val::Auto;
            node.padding = UiRect::all(Val::Px(BLOCK_PADDING_PX));
        }
        block.stored_height = None;
    }
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

    let fade_overlay = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.0)),
            ZIndex(3),
        ))
        .id();

    commands.entity(block_root).add_child(fade_overlay);

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
            rendered_sustain_segments: HashMap::new(),
            rendered_slide_segments: HashMap::new(),
        });
    }

    BlockView {
        index,
        root: block_root,
        overlay,
        fade_overlay,
        rows,
        is_removing: false,
        stored_height: None,
    }
}

fn render_notes(
    commands: &mut Commands,
    view: &mut StringTimelineView,
    feed: &StringTimelineFeed,
    current_block_index: i32,
    technique_registry: &TechniqueVisualizationRegistry,
) {
    let block_duration = feed.block_duration.max(MIN_BLOCK_DURATION);
    let string_colors = view.string_colors.clone();
    let default_string_color = Color::srgb(0.235, 0.549, 1.0);

    for block in &mut view.blocks {
        if block.rows.is_empty() {
            continue;
        }

        let block_start = block.index as f32 * block_duration;
        let block_end = block_start + block_duration;
        let block_is_past = block.index < current_block_index;

        let mut desired_bubbles: Vec<Vec<(NoteKey, f32, &TimelineNote)>> =
            vec![Vec::new(); block.rows.len()];
        let mut desired_sustains: Vec<Vec<(SustainSegmentKey, f32, f32, &TimelineNote)>> =
            vec![Vec::new(); block.rows.len()];
        let mut desired_slide_segments: Vec<Vec<SlideSegmentRender>> =
            (0..block.rows.len()).map(|_| Vec::new()).collect();

        for note in &feed.notes {
            if note.string_index >= block.rows.len() {
                continue;
            }

            let sustain = note.sustain.max(0.0);
            let note_start = note.time;
            let note_end = (note.time + sustain).max(note_start);

            if note_end <= block_start || note_start >= block_end {
                continue;
            }

            let overlap_start = note_start.max(block_start);
            let overlap_end = note_end.min(block_end);

            if note_start >= block_start && note_start < block_end {
                let progress = ((note_start - block_start) / block_duration).clamp(0.0, 1.0);
                let left_percent = progress * 100.0;
                let key = NoteKey::new(note);
                desired_bubbles[note.string_index].push((key, left_percent, note));
            }

            if overlap_end > overlap_start {
                let start_percent =
                    ((overlap_start - block_start) / block_duration).clamp(0.0, 1.0) * 100.0;
                let end_percent =
                    ((overlap_end - block_start) / block_duration).clamp(0.0, 1.0) * 100.0;
                let width_percent = (end_percent - start_percent).max(0.0);
                if width_percent > 0.0 {
                    let segment_key = SustainSegmentKey::new(note, block.index);
                    let is_slide = note.is_slide();
                    let slide_target = note.primary_slide_target();
                    if is_slide && slide_target.is_some() {
                        let terminal = note_end <= block_end + f32::EPSILON;
                        desired_slide_segments[note.string_index].push(SlideSegmentRender {
                            key: segment_key,
                            left_percent: start_percent,
                            width_percent,
                            terminal,
                            terminal_percent: end_percent,
                            note,
                            target_fret: slide_target,
                        });
                    } else {
                        desired_sustains[note.string_index].push((
                            segment_key,
                            start_percent,
                            width_percent,
                            note,
                        ));
                    }
                }
            }
        }

        for (string_idx, row) in block.rows.iter_mut().enumerate() {
            let desired = desired_bubbles
                .get(string_idx)
                .map(|entries| entries.as_slice())
                .unwrap_or_default();

            let desired_segments = desired_sustains
                .get(string_idx)
                .map(|entries| entries.as_slice())
                .unwrap_or_default();

            let desired_slide = desired_slide_segments
                .get(string_idx)
                .map(|entries| entries.as_slice())
                .unwrap_or_default();

            let mut desired_note_keys = Vec::with_capacity(desired.len());
            let mut desired_segment_keys = Vec::with_capacity(desired_segments.len());
            let mut desired_slide_keys = Vec::with_capacity(desired_slide.len());

            for (key, left_percent, note) in desired {
                desired_note_keys.push(*key);
                if row.rendered_notes.contains_key(key) {
                    continue;
                }

                let palette_color = if string_colors.is_empty() {
                    default_string_color
                } else {
                    string_colors[string_idx % string_colors.len()]
                };
                let style = technique_registry.style_for(&note.techniques);
                let note_color = style.timeline_background.unwrap_or(palette_color);
                let border_width = style
                    .timeline_border_width
                    .or_else(|| style.timeline_border_color.map(|_| 2.0))
                    .unwrap_or(0.0);
                let border_color = style.timeline_border_color.unwrap_or(note_color);

                let mut node = Node {
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
                };
                if border_width > 0.0 {
                    node.border = UiRect::all(Val::Px(border_width));
                }

                let mut note_commands = commands.spawn((
                    node,
                    BackgroundColor(note_color),
                    BorderRadius::all(Val::Px(NOTE_DIAMETER_PX / 2.0)),
                ));

                if border_width > 0.0 {
                    note_commands.insert(BorderColor::all(border_color));
                }

                let note_entity = note_commands.id();

                note_commands.with_children(|parent| {
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

            for (segment_key, left_percent, width_percent, note) in desired_segments {
                desired_segment_keys.push(*segment_key);
                if row.rendered_sustain_segments.contains_key(segment_key) {
                    continue;
                }

                let palette_color = if string_colors.is_empty() {
                    default_string_color
                } else {
                    string_colors[string_idx % string_colors.len()]
                };
                let style = technique_registry.style_for(&note.techniques);
                let sustain_color = style.timeline_background.unwrap_or(palette_color);

                let line_entity = commands
                    .spawn((
                        Node {
                            width: Val::Percent(width_percent.clamp(0.0, 100.0)),
                            height: Val::Px(SUSTAIN_LINE_HEIGHT_PX),
                            position_type: PositionType::Absolute,
                            left: Val::Percent(left_percent.clamp(0.0, 100.0)),
                            top: Val::Percent(50.0),
                            margin: UiRect {
                                left: Val::Px(0.0),
                                right: Val::Px(0.0),
                                top: Val::Px(-(SUSTAIN_LINE_HEIGHT_PX / 2.0)),
                                bottom: Val::Px(0.0),
                            },
                            ..default()
                        },
                        BackgroundColor(sustain_color),
                        BorderRadius::all(Val::Px(SUSTAIN_LINE_HEIGHT_PX / 2.0)),
                        ZIndex(1),
                    ))
                    .id();

                commands.entity(row.note_container).add_child(line_entity);
                row.rendered_sustain_segments
                    .insert(*segment_key, line_entity);
            }

            for slide_data in desired_slide {
                desired_slide_keys.push(slide_data.key);
                if row.rendered_slide_segments.contains_key(&slide_data.key) {
                    continue;
                }

                let palette_color = if string_colors.is_empty() {
                    default_string_color
                } else {
                    string_colors[string_idx % string_colors.len()]
                };
                let style = technique_registry.style_for(&slide_data.note.techniques);
                let slide_color = style.timeline_background.unwrap_or(palette_color);

                let mut line_node = Node {
                    width: Val::Percent(slide_data.width_percent.clamp(0.0, 100.0)),
                    height: Val::Px(SLIDE_LINE_HEIGHT_PX),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(slide_data.left_percent.clamp(0.0, 100.0)),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(-(SLIDE_LINE_HEIGHT_PX / 2.0)),
                        bottom: Val::Px(0.0),
                    },
                    align_items: AlignItems::Center,
                    ..default()
                };
                if slide_data.terminal {
                    line_node.justify_content = JustifyContent::FlexEnd;
                }

                let line_entity = commands
                    .spawn((
                        line_node,
                        BackgroundColor(slide_color),
                        BorderRadius::all(Val::Px(SLIDE_LINE_HEIGHT_PX / 2.0)),
                        ZIndex(1),
                    ))
                    .id();

                let mut segment_view = SlideSegmentView {
                    line: line_entity,
                    target_circle: None,
                };

                if slide_data.terminal {
                    let arrow_entity = commands
                        .spawn((
                            Node {
                                width: Val::Auto,
                                height: Val::Auto,
                                margin: UiRect {
                                    left: Val::Px(4.0),
                                    right: Val::Px(0.0),
                                    top: Val::Px(-(SLIDE_ARROW_FONT_SIZE * 0.15)),
                                    bottom: Val::Px(0.0),
                                },
                                ..default()
                            },
                            ZIndex(2),
                        ))
                        .with_children(|arrow_parent| {
                            arrow_parent.spawn((
                                Text::new(">"),
                                TextFont {
                                    font_size: SLIDE_ARROW_FONT_SIZE,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        })
                        .id();
                    commands.entity(line_entity).add_child(arrow_entity);

                    if let Some(target_fret) = slide_data.target_fret {
                        let circle_entity = commands
                            .spawn((
                                Node {
                                    width: Val::Px(SLIDE_TARGET_DIAMETER_PX),
                                    height: Val::Px(SLIDE_TARGET_DIAMETER_PX),
                                    position_type: PositionType::Absolute,
                                    left: Val::Percent(
                                        slide_data.terminal_percent.clamp(0.0, 100.0),
                                    ),
                                    top: Val::Percent(50.0),
                                    margin: UiRect {
                                        left: Val::Px(-(SLIDE_TARGET_DIAMETER_PX / 2.0)),
                                        right: Val::Px(0.0),
                                        top: Val::Px(-(SLIDE_TARGET_DIAMETER_PX / 2.0)),
                                        bottom: Val::Px(0.0),
                                    },
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(slide_color),
                                BorderRadius::all(Val::Px(SLIDE_TARGET_DIAMETER_PX / 2.0)),
                                ZIndex(2),
                            ))
                            .id();

                        commands.entity(circle_entity).with_children(|parent| {
                            parent.spawn((
                                Text::new(target_fret.to_string()),
                                TextFont {
                                    font_size: NOTE_FONT_SIZE,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                        commands.entity(row.note_container).add_child(circle_entity);
                        segment_view.target_circle = Some(circle_entity);
                    }
                }

                commands.entity(row.note_container).add_child(line_entity);

                row.rendered_slide_segments
                    .insert(slide_data.key, segment_view);
            }

            if !block_is_past {
                let mut stale_keys = Vec::new();
                for key in row.rendered_notes.keys() {
                    if !desired_note_keys.contains(key) {
                        stale_keys.push(*key);
                    }
                }

                for key in stale_keys {
                    if let Some(entity) = row.rendered_notes.remove(&key) {
                        commands.entity(entity).despawn();
                    }
                }

                let mut stale_segments = Vec::new();
                for key in row.rendered_sustain_segments.keys() {
                    if !desired_segment_keys.contains(key) {
                        stale_segments.push(*key);
                    }
                }

                for key in stale_segments {
                    if let Some(entity) = row.rendered_sustain_segments.remove(&key) {
                        commands.entity(entity).despawn();
                    }
                }

                let mut stale_slide_segments = Vec::new();
                for key in row.rendered_slide_segments.keys() {
                    if !desired_slide_keys.contains(key) {
                        stale_slide_segments.push(*key);
                    }
                }

                for key in stale_slide_segments {
                    if let Some(segment) = row.rendered_slide_segments.remove(&key) {
                        if let Some(circle) = segment.target_circle {
                            commands.entity(circle).despawn();
                        }
                        commands.entity(segment.line).despawn();
                    }
                }
            }
        }
    }
}

fn update_fret_view(
    commands: &mut Commands,
    view: &mut StringTimelineView,
    feed: &StringTimelineFeed,
    technique_registry: &TechniqueVisualizationRegistry,
    delta_seconds: f32,
    node_query: &mut Query<&mut Node>,
    text_query: &mut Query<&mut Text>,
    transform_query: &mut Query<&mut Transform>,
) {
    let (Some(fret_neck), Some(grid_layer), Some(marker_layer), Some(string_layer)) = (
        view.fret_neck,
        view.fret_grid_layer,
        view.fret_marker_layer,
        view.fret_string_layer,
    ) else {
        return;
    };

    ensure_fret_strings(commands, view, string_layer, feed.string_count);

    let active_notes = collect_active_notes(feed);

    if active_notes.is_empty() {
        clear_fret_markers(commands, view);
        clear_fret_grid(commands, view);
        update_fret_label(view, text_query, "");
        if !view.fret_current_signature.is_empty() {
            view.fret_current_signature.clear();
        }
        view.fret_current_range = None;
        progress_fret_zoom_animation(view, delta_seconds, transform_query);
        return;
    }

    let mut new_signature: Vec<NoteKey> =
        active_notes.iter().map(|note| NoteKey::new(note)).collect();
    new_signature.sort_by(|a, b| {
        a.string_index
            .cmp(&b.string_index)
            .then(a.fret.cmp(&b.fret))
            .then(a.metadata_hash.cmp(&b.metadata_hash))
    });

    if new_signature != view.fret_current_signature {
        view.fret_current_signature = new_signature.clone();
        view.fret_zoom_animation = Some(FretZoomAnimation::new(
            FRET_ZOOM_START_SCALE,
            1.0,
            FRET_ZOOM_DURATION,
        ));
        if let Ok(mut transform) = transform_query.get_mut(fret_neck) {
            transform.scale = Vec3::splat(FRET_ZOOM_START_SCALE);
        }
    }

    let Some(range) = compute_fret_range(&active_notes) else {
        clear_fret_markers(commands, view);
        clear_fret_grid(commands, view);
        if !view.fret_current_signature.is_empty() {
            view.fret_current_signature.clear();
        }
        update_fret_label(view, text_query, "");
        view.fret_current_range = None;
        progress_fret_zoom_animation(view, delta_seconds, transform_query);
        return;
    };

    if view.fret_current_range != Some(range) {
        view.fret_current_range = Some(range);
    }

    clear_fret_markers(commands, view);
    ensure_fret_grid(commands, view, grid_layer, &range, node_query);

    let mut occupied = HashSet::new();
    let string_colors = view.string_colors.clone();

    for note in &active_notes {
        if note.string_index >= feed.string_count {
            continue;
        }

        let mut maybe_spawn_marker = |fret_value: i32, role: FretMarkerRole, primary: bool| {
            if fret_value < 0 {
                return;
            }
            if !occupied.insert((note.string_index, fret_value, primary)) {
                return;
            }

            let key = match role {
                FretMarkerRole::Primary => FretMarkerKey::primary(note),
                FretMarkerRole::Additional(extra) => FretMarkerKey::additional(note, extra),
                FretMarkerRole::SlideBar => FretMarkerKey::slide_bar(note),
            };

            let palette_color = if string_colors.is_empty() {
                Color::srgb(0.55, 0.75, 0.95)
            } else {
                string_colors[note.string_index % string_colors.len()]
            };

            let style = technique_registry.style_for(&note.techniques);
            let marker_color = style.fret_background.unwrap_or(palette_color);
            let diameter = if primary {
                FRET_MARKER_DIAMETER_PX
            } else {
                FRET_MARKER_DIAMETER_PX * FRET_MARKER_SECONDARY_SCALE
            };
            let border_width = style
                .fret_border_width
                .or_else(|| {
                    style
                        .fret_border_color
                        .map(|_| if primary { 2.0 } else { 1.5 })
                })
                .unwrap_or(if primary { 2.0 } else { 1.5 });
            let border_color = style
                .fret_border_color
                .unwrap_or(Color::srgb(0.95, 0.95, 0.98));

            let left_percent = fret_left_percent(fret_value, &range);
            let top_percent = string_position_percent(note.string_index, feed.string_count);

            let mut node = Node {
                width: Val::Px(diameter),
                height: Val::Px(diameter),
                position_type: PositionType::Absolute,
                left: Val::Percent(left_percent.clamp(0.0, 100.0)),
                top: Val::Percent(top_percent.clamp(0.0, 100.0)),
                margin: UiRect {
                    left: Val::Px(-(diameter / 2.0)),
                    right: Val::Px(0.0),
                    top: Val::Px(-(diameter / 2.0)),
                    bottom: Val::Px(0.0),
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            };
            if border_width > 0.0 {
                node.border = UiRect::all(Val::Px(border_width));
            }

            let mut marker_commands = commands.spawn((
                node,
                BackgroundColor(marker_color),
                BorderRadius::all(Val::Px(diameter / 2.0)),
                FretMarker,
                ZIndex(3),
            ));

            if border_width > 0.0 {
                marker_commands.insert(BorderColor::all(border_color));
            }

            let marker_entity = marker_commands.id();

            commands.entity(marker_layer).add_child(marker_entity);
            view.fret_markers.insert(key, marker_entity);
        };

        maybe_spawn_marker(note.fret, FretMarkerRole::Primary, true);

        for extra in &note.additional_frets {
            maybe_spawn_marker(*extra, FretMarkerRole::Additional(*extra), false);
        }

        if note.is_slide() {
            if let Some(target_fret) = note.primary_slide_target() {
                if note.sustain > f32::EPSILON {
                    let progress = ((feed.current_time - note.time) / note.sustain).clamp(0.0, 1.0);
                    if progress > 0.0 {
                        let start_percent = fret_left_percent(note.fret, &range);
                        let target_percent = fret_left_percent(target_fret, &range);
                        let current_percent =
                            start_percent + (target_percent - start_percent) * progress;
                        let left_percent = start_percent.min(current_percent);
                        let width_percent = (current_percent - start_percent).abs();

                        if width_percent > 0.0 {
                            let palette_color = if string_colors.is_empty() {
                                Color::srgb(0.55, 0.75, 0.95)
                            } else {
                                string_colors[note.string_index % string_colors.len()]
                            };

                            let style = technique_registry.style_for(&note.techniques);
                            let bar_color = style.fret_background.unwrap_or(palette_color);

                            let bar_entity = commands
                                .spawn((
                                    Node {
                                        width: Val::Percent(width_percent.clamp(0.0, 100.0)),
                                        height: Val::Px(SLIDE_LINE_HEIGHT_PX),
                                        position_type: PositionType::Absolute,
                                        left: Val::Percent(left_percent.clamp(0.0, 100.0)),
                                        top: Val::Percent(
                                            string_position_percent(
                                                note.string_index,
                                                feed.string_count,
                                            )
                                            .clamp(0.0, 100.0),
                                        ),
                                        margin: UiRect {
                                            left: Val::Px(0.0),
                                            right: Val::Px(0.0),
                                            top: Val::Px(-(SLIDE_LINE_HEIGHT_PX / 2.0)),
                                            bottom: Val::Px(0.0),
                                        },
                                        ..default()
                                    },
                                    BackgroundColor(bar_color),
                                    BorderRadius::all(Val::Px(SLIDE_LINE_HEIGHT_PX / 2.0)),
                                    FretMarker,
                                    ZIndex(2),
                                ))
                                .id();

                            commands.entity(marker_layer).add_child(bar_entity);
                            view.fret_markers
                                .insert(FretMarkerKey::slide_bar(note), bar_entity);
                        }
                    }
                }
            }
        }
    }

    let label_text = if range.start == range.end {
        format!("Fret {}", range.start)
    } else {
        format!("Frets {} - {}", range.start, range.end)
    };
    update_fret_label(view, text_query, &label_text);

    progress_fret_zoom_animation(view, delta_seconds, transform_query);
}

fn ensure_fret_strings(
    commands: &mut Commands,
    view: &mut StringTimelineView,
    string_layer: Entity,
    string_count: usize,
) {
    if view.fret_string_lines.len() == string_count {
        return;
    }

    for entity in view.fret_string_lines.drain(..) {
        commands.entity(entity).despawn();
    }

    if string_count == 0 {
        return;
    }

    let string_colors = view.string_colors.clone();

    for string_index in 0..string_count {
        let base_color = if string_colors.is_empty() {
            Color::srgb(0.75, 0.75, 0.78)
        } else {
            string_colors[string_index % string_colors.len()]
        };

        let top_percent = string_position_percent(string_index, string_count).clamp(0.0, 100.0);
        let line_entity = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(0.0),
                    top: Val::Percent(top_percent),
                    margin: UiRect {
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(-1.0),
                        bottom: Val::Px(0.0),
                    },
                    ..default()
                },
                BackgroundColor(base_color.with_alpha(0.55)),
                ZIndex(2),
            ))
            .id();

        commands.entity(string_layer).add_child(line_entity);
        view.fret_string_lines.push(line_entity);
    }
}

fn ensure_fret_grid(
    commands: &mut Commands,
    view: &mut StringTimelineView,
    grid_layer: Entity,
    range: &FretRange,
    node_query: &mut Query<&mut Node>,
) {
    let span = range.span();
    let required_lines = span + 1;
    if view.fret_fret_lines.len() != required_lines {
        clear_fret_grid(commands, view);
        view.fret_fret_lines.reserve(required_lines);
        let span_value = span.max(1) as f32;
        for i in 0..=span {
            let line_width = if i == 0 { 4.0 } else { 2.0 };
            let left_percent = (i as f32 / span_value) * 100.0;
            let margin_left = if i == 0 {
                0.0
            } else if i == span {
                -line_width
            } else {
                -(line_width * 0.5)
            };

            let line_entity = commands
                .spawn((
                    Node {
                        width: Val::Px(line_width),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        left: Val::Percent(left_percent.clamp(0.0, 100.0)),
                        top: Val::Percent(0.0),
                        margin: UiRect {
                            left: Val::Px(margin_left),
                            right: Val::Px(0.0),
                            top: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                        },
                        ..default()
                    },
                    BackgroundColor(Color::srgba(
                        if i == 0 { 0.95 } else { 0.85 },
                        if i == 0 { 0.95 } else { 0.85 },
                        if i == 0 { 0.98 } else { 0.92 },
                        if i == 0 { 0.8 } else { 0.55 },
                    )),
                    ZIndex(1),
                ))
                .id();

            commands.entity(grid_layer).add_child(line_entity);
            view.fret_fret_lines.push(line_entity);
        }
    } else {
        let span_value = span.max(1) as f32;
        for (i, entity) in view.fret_fret_lines.iter().enumerate() {
            if let Ok(mut node) = node_query.get_mut(*entity) {
                let line_width = if i == 0 { 4.0 } else { 2.0 };
                let left_percent = (i as f32 / span_value) * 100.0;
                let margin_left = if i == 0 {
                    0.0
                } else if i == span {
                    -line_width
                } else {
                    -(line_width * 0.5)
                };
                node.width = Val::Px(line_width);
                node.left = Val::Percent(left_percent.clamp(0.0, 100.0));
                node.margin.left = Val::Px(margin_left);
            }
        }
    }
}

fn clear_fret_grid(commands: &mut Commands, view: &mut StringTimelineView) {
    for entity in view.fret_fret_lines.drain(..) {
        commands.entity(entity).despawn();
    }
}

fn collect_active_notes<'a>(feed: &'a StringTimelineFeed) -> Vec<&'a TimelineNote> {
    if feed.notes.is_empty() {
        return Vec::new();
    }

    let mut active: Vec<&TimelineNote> = feed
        .notes
        .iter()
        .filter(|note| {
            let start = note.time;
            let end = note.time + note.sustain.max(0.0);
            feed.current_time + NOTE_GROUP_TOLERANCE >= start
                && feed.current_time <= end + NOTE_GROUP_TOLERANCE
        })
        .collect();

    if !active.is_empty() {
        let min_time = active.iter().map(|note| note.time).fold(f32::MAX, f32::min);
        active.retain(|note| (note.time - min_time).abs() <= NOTE_GROUP_TOLERANCE);
        return active;
    }

    let next_time = feed
        .notes
        .iter()
        .filter(|note| note.time + NOTE_GROUP_TOLERANCE >= feed.current_time)
        .map(|note| note.time)
        .fold(None, |acc: Option<f32>, time| match acc {
            Some(existing) => Some(existing.min(time)),
            None => Some(time),
        });

    let Some(target_time) = next_time else {
        return Vec::new();
    };

    feed.notes
        .iter()
        .filter(|note| (note.time - target_time).abs() <= NOTE_GROUP_TOLERANCE)
        .collect()
}

fn compute_fret_range(notes: &[&TimelineNote]) -> Option<FretRange> {
    let mut min_fret = i32::MAX;
    let mut max_fret = i32::MIN;

    for note in notes {
        if note.fret >= 0 {
            min_fret = min_fret.min(note.fret);
            max_fret = max_fret.max(note.fret);
        }
        for extra in &note.additional_frets {
            if *extra >= 0 {
                min_fret = min_fret.min(*extra);
                max_fret = max_fret.max(*extra);
            }
        }
    }

    if min_fret == i32::MAX {
        return None;
    }

    Some(FretRange::new(min_fret, max_fret))
}

fn clear_fret_markers(commands: &mut Commands, view: &mut StringTimelineView) {
    for (_, entity) in view.fret_markers.drain() {
        commands.entity(entity).despawn();
    }
}

fn update_fret_label(view: &StringTimelineView, text_query: &mut Query<&mut Text>, content: &str) {
    let Some(label_entity) = view.fret_label else {
        return;
    };

    if let Ok(mut text) = text_query.get_mut(label_entity) {
        *text = Text::new(content.to_string());
    }
}

fn string_position_percent(string_index: usize, string_count: usize) -> f32 {
    if string_count <= 1 {
        50.0
    } else {
        (string_index as f32 / (string_count - 1) as f32) * 100.0
    }
}

fn fret_left_percent(fret: i32, range: &FretRange) -> f32 {
    let span = range.span() as f32;
    if span <= f32::EPSILON {
        50.0
    } else {
        ((fret - range.start) as f32 + 0.5) / span * 100.0
    }
}

fn progress_fret_zoom_animation(
    view: &mut StringTimelineView,
    delta_seconds: f32,
    transform_query: &mut Query<&mut Transform>,
) {
    let Some(fret_neck) = view.fret_neck else {
        return;
    };

    if let Some(animation) = view.fret_zoom_animation.as_mut() {
        animation.elapsed += delta_seconds;
        let progress = (animation.elapsed / animation.duration).clamp(0.0, 1.0);
        let eased = ease_out_quad(progress);
        let scale =
            animation.start_scale + (animation.target_scale - animation.start_scale) * eased;
        if let Ok(mut transform) = transform_query.get_mut(fret_neck) {
            transform.scale = Vec3::splat(scale);
        }
        if progress >= 1.0 {
            view.fret_zoom_animation = None;
        }
    } else if let Ok(mut transform) = transform_query.get_mut(fret_neck) {
        transform.scale = Vec3::splat(1.0);
    }
}

fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(2)
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
    view: &mut StringTimelineView,
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
        let smoothed_progress = if let Some(previous_index) = view.indicator_block_index {
            if previous_index == current_block_index {
                let blended = view.indicator_block_progress
                    + (block_progress - view.indicator_block_progress) * 0.3;
                view.indicator_block_progress = blended;
                blended
            } else {
                view.indicator_block_index = Some(current_block_index);
                view.indicator_block_progress = block_progress;
                block_progress
            }
        } else {
            view.indicator_block_index = Some(current_block_index);
            view.indicator_block_progress = block_progress;
            block_progress
        };
        let width_percent = (smoothed_progress * 100.0).clamp(0.0, 100.0);
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
    view.shift_animation = None;
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

pub fn timeline_window_seconds(feed: &StringTimelineFeed) -> f32 {
    feed.block_duration * VISIBLE_BLOCKS as f32
}

pub fn timeline_block_duration(feed: &StringTimelineFeed) -> f32 {
    feed.block_duration
}

pub fn clamp_block_duration(duration: f32) -> f32 {
    duration.clamp(MIN_BLOCK_DURATION, MAX_BLOCK_DURATION)
}

pub fn default_block_duration() -> f32 {
    DEFAULT_BLOCK_DURATION
}

pub fn target_note_spacing_percent() -> f32 {
    TARGET_NOTE_SPACING_PERCENT
}

pub fn visible_block_count() -> usize {
    VISIBLE_BLOCKS
}
