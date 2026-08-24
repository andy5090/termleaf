//! The editor core: text buffer + cursor + live Hangul composer.

pub mod buffer;
pub mod state;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::input::{JapaneseComposer, KoreanComposer};
use buffer::Buffer;
use state::{Cursor, DocState};

/// Ties together the text buffer, the cursor, and the in-progress Hangul
/// composition. All editing goes through here.
pub struct Editor {
    pub buffer: Buffer,
    pub doc: DocState,
    pub composer: KoreanComposer,
    pub japanese_composer: JapaneseComposer,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            buffer: Buffer::new(),
            doc: DocState::default(),
            composer: KoreanComposer::new(),
            japanese_composer: JapaneseComposer::new(),
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
            composer: KoreanComposer::new(),
            japanese_composer: JapaneseComposer::new(),
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
        if self.composer.is_empty() {
            self.japanese_composer.composing_string()
        } else {
            self.composer.composing_string()
        }
    }

    pub fn composer(&self) -> &KoreanComposer {
        &self.composer
    }

    /// A short slice of the current line for the big-pixel focus zone.
    ///
    /// Pending Hangul composition is inserted at the cursor as one evolving
    /// glyph (`ㅎ` → `하` → `한`), so it always occupies a single character
    /// slot in the enlarged view.
    /// When the line is longer than `max_chars`, the window follows the cursor
    /// and favors the text immediately before it.
    pub fn focus_text(&self, max_chars: usize) -> Vec<char> {
        if max_chars == 0 {
            return Vec::new();
        }

        let Cursor { row, col } = self.doc.cursor;
        let mut line = self.buffer.lines.get(row).cloned().unwrap_or_default();
        let composing: Vec<char> = self.composing().chars().collect();
        let insert_at = col.min(line.len());
        line.splice(insert_at..insert_at, composing.iter().copied());

        let caret = insert_at + composing.len();
        let end = caret.min(line.len());
        let start = end.saturating_sub(max_chars);
        line[start..end].to_vec()
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

    /// Feed a romaji key into the live Japanese composer. Completed kana are
    /// committed immediately while an incomplete sequence remains visible.
    pub fn input_romaji(&mut self, character: char, katakana: bool) {
        let committed = self.japanese_composer.input(character, katakana);
        for character in committed {
            self.insert_committed(character);
        }
        self.doc.dirty = true;
    }

    /// Finalize pending composition into the buffer.
    pub fn flush(&mut self) {
        let committed = self.composer.flush();
        for c in committed {
            self.insert_committed(c);
        }
        let committed = self.japanese_composer.flush();
        for character in committed {
            self.insert_committed(character);
        }
    }

    pub fn backspace(&mut self) -> bool {
        if !self.composer.is_empty() {
            self.composer.backspace();
            self.doc.dirty = true;
            return true;
        }
        if self.japanese_composer.backspace() {
            self.doc.dirty = true;
            return true;
        }
        let Cursor { row, col } = self.doc.cursor;
        if col > 0 {
            self.buffer.lines[row].remove(col - 1);
            self.doc.cursor.col -= 1;
            self.doc.dirty = true;
            true
        } else if row > 0 {
            let cur = self.buffer.lines.remove(row);
            let prev_len = self.buffer.lines[row - 1].len();
            self.buffer.lines[row - 1].extend(cur);
            self.doc.cursor.row -= 1;
            self.doc.cursor.col = prev_len;
            self.doc.dirty = true;
            true
        } else {
            false
        }
    }

    /// Delete the character under the cursor. At the end of a line, join the
    /// following line just like a conventional editor's Delete key.
    pub fn delete_forward(&mut self) -> bool {
        self.flush();
        let Cursor { row, col } = self.doc.cursor;
        if col < self.buffer.line_len(row) {
            self.buffer.lines[row].remove(col);
            self.doc.dirty = true;
            true
        } else if row + 1 < self.buffer.line_count() {
            let next = self.buffer.lines.remove(row + 1);
            self.buffer.lines[row].extend(next);
            self.doc.dirty = true;
            true
        } else {
            false
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
        let path = self
            .doc
            .path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.txt"));
        self.write_to(&path)
    }

    fn write_to(&mut self, path: &Path) -> io::Result<PathBuf> {
        self.flush();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, self.buffer.to_text())?;
        self.doc.path = Some(path.to_path_buf());
        self.doc.dirty = false;
        Ok(path.to_path_buf())
    }

    /// Save to an explicitly chosen path and make it the document's current
    /// path for subsequent `save` calls.
    pub fn save_as<P: AsRef<Path>>(&mut self, path: P) -> io::Result<PathBuf> {
        self.write_to(path.as_ref())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn edits_lines_and_moves_across_boundaries() {
        let mut editor = Editor::new();
        for c in "가ab".chars() {
            editor.insert_char(c);
        }
        editor.move_left();
        editor.newline();

        assert_eq!(editor.buffer.to_text(), "가a\nb");
        assert_eq!(editor.cursor(), Cursor { row: 1, col: 0 });

        editor.backspace();
        assert_eq!(editor.buffer.to_text(), "가ab");
        assert_eq!(editor.cursor(), Cursor { row: 0, col: 2 });

        editor.move_home();
        editor.move_left();
        assert_eq!(editor.cursor(), Cursor { row: 0, col: 0 });
        editor.move_end();
        editor.move_right();
        assert_eq!(editor.cursor(), Cursor { row: 0, col: 3 });
    }

    #[test]
    fn movement_clamps_to_the_target_line() {
        let mut editor = Editor::new();
        for c in "long".chars() {
            editor.insert_char(c);
        }
        editor.newline();
        editor.insert_char('x');
        editor.move_up();
        editor.move_end();
        editor.move_down();
        assert_eq!(editor.cursor(), Cursor { row: 1, col: 1 });
    }

    #[test]
    fn composition_is_committed_before_literal_input() {
        let mut editor = Editor::new();
        for jamo in ['ㅎ', 'ㅏ', 'ㄴ'] {
            editor.input_jamo(jamo);
        }
        assert_eq!(editor.composing(), "한");
        assert_eq!(editor.buffer.to_text(), "");

        editor.insert_char('!');
        assert_eq!(editor.buffer.to_text(), "한!");
        assert!(editor.composer.is_empty());
        assert!(editor.doc.dirty);
    }

    #[test]
    fn focus_text_follows_cursor_and_includes_pending_composition() {
        let mut editor = Editor::new();
        for c in "123456789".chars() {
            editor.insert_char(c);
        }
        for jamo in ['ㅎ', 'ㅏ', 'ㄴ'] {
            editor.input_jamo(jamo);
        }

        assert_eq!(editor.focus_text(6).iter().collect::<String>(), "56789한");

        editor.move_left();
        assert_eq!(editor.focus_text(4).iter().collect::<String>(), "6789");
        assert!(editor.composer.is_empty());
    }

    #[test]
    fn focus_text_updates_hangul_inside_one_character_slot() {
        let mut editor = Editor::new();

        editor.input_jamo('ㅎ');
        assert_eq!(editor.focus_text(4), ['ㅎ']);
        editor.input_jamo('ㅏ');
        assert_eq!(editor.focus_text(4), ['하']);
        editor.input_jamo('ㄴ');
        assert_eq!(editor.focus_text(4), ['한']);
    }

    #[test]
    fn live_japanese_keeps_pending_romaji_visible_and_commits_kana() {
        let mut editor = Editor::new();

        editor.input_romaji('k', false);
        assert_eq!(editor.composing(), "k");
        assert_eq!(editor.focus_text(4), ['k']);

        editor.input_romaji('a', false);
        assert_eq!(editor.buffer.to_text(), "か");
        assert!(editor.composing().is_empty());
    }

    #[test]
    fn backspace_reports_whether_it_changed_the_document() {
        let mut editor = Editor::new();
        assert!(!editor.backspace());

        editor.input_jamo('ㅎ');
        assert!(editor.backspace());
        assert!(!editor.backspace());

        editor.insert_char('a');
        assert!(editor.backspace());
        assert!(!editor.backspace());
    }

    #[test]
    fn delete_removes_the_next_character_and_joins_lines() {
        let mut editor = Editor::new();
        for character in "abc".chars() {
            editor.insert_char(character);
        }
        editor.move_home();
        assert!(editor.delete_forward());
        assert_eq!(editor.buffer.to_text(), "bc");
        assert_eq!(editor.cursor(), Cursor { row: 0, col: 0 });

        editor.move_end();
        editor.newline();
        editor.insert_char('d');
        editor.move_home();
        editor.move_left();
        assert_eq!(editor.cursor(), Cursor { row: 0, col: 2 });
        assert!(editor.delete_forward());
        assert_eq!(editor.buffer.to_text(), "bcd");
        assert_eq!(editor.cursor(), Cursor { row: 0, col: 2 });

        editor.move_end();
        assert!(!editor.delete_forward());
    }

    #[test]
    fn save_creates_parent_directories_and_clears_dirty_flag() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("termleaf-test-{}-{unique}", std::process::id()));
        let path = root.join("nested").join("note.txt");

        let mut editor = Editor::open(&path).expect("a missing document should open empty");
        editor.insert_char('한');
        let saved = editor.save().expect("document should save");

        assert_eq!(saved, path);
        assert_eq!(
            fs::read_to_string(&path).expect("saved file should exist"),
            "한"
        );
        assert!(!editor.doc.dirty);

        fs::remove_dir_all(root).expect("temporary test directory should be removable");
    }

    #[test]
    fn save_as_updates_the_document_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("termleaf-save-as-{unique}"));
        let path = root.join("chosen.txt");

        let mut editor = Editor::new();
        editor.insert_char('글');
        let saved = editor
            .save_as(&path)
            .expect("save-as should write the file");

        assert_eq!(saved, path);
        assert_eq!(editor.doc.path.as_deref(), Some(path.as_path()));
        assert_eq!(fs::read_to_string(&path).unwrap(), "글");

        fs::remove_dir_all(root).expect("temporary test directory should be removable");
    }
}
