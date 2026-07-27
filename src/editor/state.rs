//! Editor cursor and document metadata.

use std::path::PathBuf;

/// Cursor position in character units.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

/// Metadata about the document being edited.
#[derive(Debug, Default, Clone)]
pub struct DocState {
    pub cursor: Cursor,
    pub dirty: bool,
    pub path: Option<PathBuf>,
}
