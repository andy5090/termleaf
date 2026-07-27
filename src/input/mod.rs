//! Input handling: raw keystroke mapping and the Hangul composition engine.

pub mod events;
pub mod korean;

pub use events::{map_key, Action};
pub use korean::Composer;
