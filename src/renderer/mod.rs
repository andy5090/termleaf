//! Rendering: the bitmap font and ANSI terminal painting.

pub mod font;
pub mod terminal;

pub use terminal::{draw, TerminalGuard};
