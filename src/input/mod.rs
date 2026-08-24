//! Input handling plus the live Korean and Japanese composition engines.

pub mod events;
pub mod japanese;
pub mod korean;

pub use events::{map_key, Action};
pub use japanese::Composer as JapaneseComposer;
pub use korean::Composer as KoreanComposer;
