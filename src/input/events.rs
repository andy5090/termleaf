//! Maps raw terminal key events into high-level editor [`Action`]s.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::korean::key_to_jamo;

/// A high-level action derived from a keypress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Insert a literal character (Latin / punctuation / space).
    InsertChar(char),
    /// Feed a jamo keystroke into the Hangul composer.
    Jamo(char),
    Backspace,
    Newline,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    ToggleHangul,
    ToggleFocus,
    ToggleBigFont,
    ToggleSound,
    ToggleTheme,
    FontInc,
    FontDec,
    Save,
    Quit,
    Ignore,
}

/// Translate a key event into an [`Action`]. `hangul` selects whether letter
/// keys are treated as jamo or as literal Latin input.
pub fn map_key(key: KeyEvent, hangul: bool) -> Action {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Char(c) if ctrl => match c.to_ascii_lowercase() {
            'q' | 'c' => Action::Quit,
            's' => Action::Save,
            _ => Action::Ignore,
        },
        KeyCode::F(2) => Action::ToggleHangul,
        KeyCode::F(3) => Action::ToggleFocus,
        KeyCode::F(4) => Action::ToggleBigFont,
        KeyCode::F(5) => Action::ToggleSound,
        KeyCode::F(6) => Action::ToggleTheme,
        KeyCode::F(7) => Action::FontDec,
        KeyCode::F(8) => Action::FontInc,
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Enter => Action::Newline,
        KeyCode::Left => Action::Left,
        KeyCode::Right => Action::Right,
        KeyCode::Up => Action::Up,
        KeyCode::Down => Action::Down,
        KeyCode::Home => Action::Home,
        KeyCode::End => Action::End,
        KeyCode::Tab => Action::InsertChar('\t'),
        KeyCode::Char(c) => {
            if hangul {
                match key_to_jamo(c) {
                    Some(j) => Action::Jamo(j),
                    None => Action::InsertChar(c),
                }
            } else {
                Action::InsertChar(c)
            }
        }
        _ => Action::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_control_and_function_shortcuts() {
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
                false
            ),
            Action::Save
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE), false),
            Action::ToggleHangul
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
                false
            ),
            Action::Ignore
        );
    }

    #[test]
    fn maps_letters_to_jamo_only_in_hangul_mode() {
        let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        assert_eq!(map_key(key, false), Action::InsertChar('g'));
        assert_eq!(map_key(key, true), Action::Jamo('ㅎ'));

        let punctuation = KeyEvent::new(KeyCode::Char('!'), KeyModifiers::SHIFT);
        assert_eq!(map_key(punctuation, true), Action::InsertChar('!'));
    }
}
