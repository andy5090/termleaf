//! The editor core: text buffer + cursor + live Hangul composer.

pub mod buffer;
pub mod state;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::input::Composer;
use buffer::Buffer;
use state::{Cursor, DocState};

/// Ties together the text buffer, the cursor, and the in-progress Hangul
/// composition. All editing goes through here.
pub struct Editor {
    pub buffer: Buffer,
    pub doc: DocState,
    pub composer: Composer,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            buffer: Buffer::new(),
            doc: DocState::default(),
            composer: Composer::new(),
        }
    }
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a file into a new editor. A missing file yields an empty buffer
    /// that will be created on first save.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let buffer = match fs::read_to_string(&path) {
            Ok(text) => Buffer::from_str(&text),
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => Buffer::new(),
            Err(e) => return Err(e),
        };
        Ok(Self {
            buffer,
            doc: DocState {
                path: Some(path),
                ..Default::default()
            },
            composer: Composer::new(),
        })
    }

    pub fn cursor(&self) -> Cursor {
        self.doc.cursor
    }

    pub fn lines(&self) -> &[Vec<char>] {
        &self.buffer.lines
    }

    /// The current (possibly partial) composing glyph, e.g. "하".
    pub fn composing(&self) -> String {
        self.composer.composing_string()
    }

    pub fn composer(&self) -> &Composer {
        &self.composer
    }

    // --- editing ---------------------------------------------------------

    fn insert_committed(&mut self, c: char) {
        let Cursor { row, col } = self.doc.cursor;
        self.buffer.lines[row].insert(col, c);
        self.doc.cursor.col += 1;
        self.doc.dirty = true;
    }

    /// Insert a literal (non-jamo) character, finalizing any pending Hangul
    /// composition first.
    pub fn insert_char(&mut self, c: char) {
        self.flush();
        self.insert_committed(c);
    }

    /// Feed a jamo keystroke into the composer; any finalized syllables land
    /// in the buffer.
    pub fn input_jamo(&mut self, jamo: char) {
        let committed = self.composer.input(jamo);
        for c in committed {
            self.insert_committed(c);
        }
        self.doc.dirty = true;
    }

    /// Finalize pending composition into the buffer.
    pub fn flush(&mut self) {
        let committed = self.composer.flush();
        for c in committed {
            self.insert_committed(c);
        }
    }

    pub fn backspace(&mut self) {
        if !self.composer.is_empty() {
            self.composer.backspace();
            self.doc.dirty = true;
            return;
        }
        let Cursor { row, col } = self.doc.cursor;
        if col > 0 {
            self.buffer.lines[row].remove(col - 1);
            self.doc.cursor.col -= 1;
            self.doc.dirty = true;
        } else if row > 0 {
            let cur = self.buffer.lines.remove(row);
            let prev_len = self.buffer.lines[row - 1].len();
            self.buffer.lines[row - 1].extend(cur);
            self.doc.cursor.row -= 1;
            self.doc.cursor.col = prev_len;
            self.doc.dirty = true;
        }
    }

    pub fn newline(&mut self) {
        self.flush();
        let Cursor { row, col } = self.doc.cursor;
        let tail = self.buffer.lines[row].split_off(col);
        self.buffer.lines.insert(row + 1, tail);
        self.doc.cursor.row += 1;
        self.doc.cursor.col = 0;
        self.doc.dirty = true;
    }

    // --- movement --------------------------------------------------------

    pub fn move_left(&mut self) {
        self.flush();
        let Cursor { row, col } = self.doc.cursor;
        if col > 0 {
            self.doc.cursor.col -= 1;
        } else if row > 0 {
            self.doc.cursor.row -= 1;
            self.doc.cursor.col = self.buffer.line_len(self.doc.cursor.row);
        }
    }

    pub fn move_right(&mut self) {
        self.flush();
        let Cursor { row, col } = self.doc.cursor;
        if col < self.buffer.line_len(row) {
            self.doc.cursor.col += 1;
        } else if row + 1 < self.buffer.line_count() {
            self.doc.cursor.row += 1;
            self.doc.cursor.col = 0;
        }
    }

    pub fn move_up(&mut self) {
        self.flush();
        if self.doc.cursor.row > 0 {
            self.doc.cursor.row -= 1;
            self.doc.cursor.col = self
                .doc
                .cursor
                .col
                .min(self.buffer.line_len(self.doc.cursor.row));
        }
    }

    pub fn move_down(&mut self) {
        self.flush();
        if self.doc.cursor.row + 1 < self.buffer.line_count() {
            self.doc.cursor.row += 1;
            self.doc.cursor.col = self
                .doc
                .cursor
                .col
                .min(self.buffer.line_len(self.doc.cursor.row));
        }
    }

    pub fn move_home(&mut self) {
        self.flush();
        self.doc.cursor.col = 0;
    }

    pub fn move_end(&mut self) {
        self.flush();
        self.doc.cursor.col = self.buffer.line_len(self.doc.cursor.row);
    }

    // --- persistence -----------------------------------------------------

    /// Save the document. Falls back to `untitled.txt` in the current
    /// directory when no path is set. Returns the path written.
    pub fn save(&mut self) -> io::Result<PathBuf> {
        self.flush();
        let path = self
            .doc
            .path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.txt"));
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&path, self.buffer.to_text())?;
        self.doc.path = Some(path.clone());
        self.doc.dirty = false;
        Ok(path)
    }

    pub fn word_count(&self) -> usize {
        self.buffer
            .to_text()
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .count()
    }

    pub fn char_count(&self) -> usize {
        self.buffer.char_count()
    }
}
