//! A simple line-based text buffer.
//!
//! Lines are stored as `Vec<char>` so the cursor can address columns in
//! character units — important because Hangul syllables are multi-byte.

/// The text buffer: a non-empty list of lines.
#[derive(Debug, Clone)]
pub struct Buffer {
    pub lines: Vec<Vec<char>>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            lines: vec![Vec::new()],
        }
    }
}

impl Buffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a buffer from raw text (splitting on `\n`).
    pub fn from_str(text: &str) -> Self {
        let mut lines: Vec<Vec<char>> = text.split('\n').map(|l| l.chars().collect()).collect();
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        Self { lines }
    }

    /// Serialize the buffer back into a `String`.
    pub fn to_text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line_len(&self, row: usize) -> usize {
        self.lines.get(row).map(|l| l.len()).unwrap_or(0)
    }

    /// Total number of characters (excluding newlines).
    pub fn char_count(&self) -> usize {
        self.lines.iter().map(|l| l.len()).sum()
    }
}
