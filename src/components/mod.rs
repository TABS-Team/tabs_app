pub mod string_timeline;

pub use string_timeline::{
    clamp_block_duration, default_block_duration, timeline_block_duration, timeline_window_seconds,
    visible_block_count, StringTimelineFeed, StringTimelinePlugin, TimelineNote,
};
