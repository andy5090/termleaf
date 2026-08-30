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
    /// Feed a romaji keystroke into the Japanese kana composer.
    Romaji(char),
    JapaneseConvertNext,
    JapaneseConvertPrev,
    CancelComposition,
    Backspace,
    Delete,
    Newline,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    CycleLiveInput,
    CycleLiveInputReverse,
    ToggleJapaneseScript,
    ShowHelp,
    ToggleFocus,
    ToggleBigFont,
    ToggleTheme,
    ShowLanguageSettings,
    ShowSoundSettings,
    CycleSoundProfile,
    FontInc,
    FontDec,
    CycleLineSpacing,
    TogglePageWidth,
    Open,
    Save,
    SaveAs,
    Quit,
    Ignore,
}

/// Translate a key event into an [`Action`]. `live_composition` opts into
/// Termleaf's raw two-set mapping; otherwise all text is left to the OS IME.
pub fn map_key(key: KeyEvent, live_composition: bool, live_japanese: bool) -> Action {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Char(c) if ctrl => match c.to_ascii_lowercase() {
            'q' | 'c' => Action::Quit,
            'o' => Action::Open,
            's' if key.modifiers.contains(KeyModifiers::SHIFT) => Action::SaveAs,
            's' => Action::Save,
            'k' if live_japanese => Action::ToggleJapaneseScript,
            _ => Action::Ignore,
        },
        KeyCode::Char(c)
            if key.modifiers.contains(KeyModifiers::ALT) && c.eq_ignore_ascii_case(&'l') =>
        {
            Action::CycleLineSpacing
        }
        KeyCode::Char(c)
            if key.modifiers.contains(KeyModifiers::ALT) && c.eq_ignore_ascii_case(&'p') =>
        {
            Action::TogglePageWidth
        }
        KeyCode::F(1) => Action::ShowHelp,
        KeyCode::F(2) if key.modifiers.contains(KeyModifiers::SHIFT) => {
            Action::CycleLiveInputReverse
        }
        // Some macOS terminals encode Shift+F2 as the legacy F14 key instead
        // of preserving F2 with a Shift modifier.
        KeyCode::F(14) => Action::CycleLiveInputReverse,
        KeyCode::F(2) => Action::CycleLiveInput,
        KeyCode::F(3) => Action::ToggleFocus,
        KeyCode::F(4) => Action::ToggleBigFont,
        KeyCode::F(5) if key.modifiers.contains(KeyModifiers::SHIFT) => Action::CycleLineSpacing,
        KeyCode::F(5) => Action::TogglePageWidth,
        KeyCode::F(6) => Action::ToggleTheme,
        KeyCode::F(7) => Action::FontDec,
        KeyCode::F(8) => Action::FontInc,
        KeyCode::F(9) => Action::ShowLanguageSettings,
        KeyCode::F(10) => Action::ShowSoundSettings,
        KeyCode::F(11) => Action::CycleSoundProfile,
        KeyCode::F(12) => Action::SaveAs,
        KeyCode::BackTab if live_japanese => Action::JapaneseConvertPrev,
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Delete => Action::Delete,
        KeyCode::Esc if live_japanese => Action::CancelComposition,
        KeyCode::Enter => Action::Newline,
        KeyCode::Left => Action::Left,
        KeyCode::Right => Action::Right,
        KeyCode::Up => Action::Up,
        KeyCode::Down => Action::Down,
        KeyCode::Home => Action::Home,
        KeyCode::End => Action::End,
        KeyCode::Tab if live_japanese => Action::JapaneseConvertNext,
        KeyCode::Tab => Action::InsertChar('\t'),
        KeyCode::Char(' ') if live_japanese => Action::JapaneseConvertNext,
        KeyCode::Char(c) => {
            if live_japanese && (c.is_ascii_alphabetic() || matches!(c, '-' | '\'')) {
                Action::Romaji(c)
            } else if live_composition {
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
                false,
                false
            ),
            Action::Save
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
                false,
                false
            ),
            Action::Open
        );
        assert_eq!(
            map_key(
                KeyEvent::new(
                    KeyCode::Char('S'),
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT
                ),
                false,
                false
            ),
            Action::SaveAs
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE),
                false,
                false
            ),
            Action::ShowHelp
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE),
                false,
                false
            ),
            Action::CycleLiveInput
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(2), KeyModifiers::SHIFT),
                false,
                false
            ),
            Action::CycleLiveInputReverse
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(14), KeyModifiers::NONE),
                true,
                false
            ),
            Action::CycleLiveInputReverse
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE),
                false,
                false
            ),
            Action::ShowLanguageSettings
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE),
                false,
                false
            ),
            Action::ShowSoundSettings
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE),
                false,
                false
            ),
            Action::TogglePageWidth
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(5), KeyModifiers::SHIFT),
                false,
                false
            ),
            Action::CycleLineSpacing
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(11), KeyModifiers::NONE),
                false,
                false
            ),
            Action::CycleSoundProfile
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE),
                false,
                false
            ),
            Action::SaveAs
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Char('l'), KeyModifiers::ALT),
                false,
                false
            ),
            Action::CycleLineSpacing
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::ALT),
                false,
                false
            ),
            Action::TogglePageWidth
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
                false,
                false
            ),
            Action::Delete
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
                false,
                false
            ),
            Action::Ignore
        );
    }

    #[test]
    fn maps_letters_to_jamo_only_in_live_composition_mode() {
        let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        assert_eq!(map_key(key, false, false), Action::InsertChar('g'));
        assert_eq!(map_key(key, true, false), Action::Jamo('ㅎ'));

        let punctuation = KeyEvent::new(KeyCode::Char('!'), KeyModifiers::SHIFT);
        assert_eq!(map_key(punctuation, true, false), Action::InsertChar('!'));
    }

    #[test]
    fn os_ime_committed_hangul_is_inserted_without_remapping() {
        let committed = KeyEvent::new(KeyCode::Char('한'), KeyModifiers::NONE);
        assert_eq!(map_key(committed, false, false), Action::InsertChar('한'));
        assert_eq!(map_key(committed, true, false), Action::InsertChar('한'));
    }

    #[test]
    fn maps_romaji_and_script_toggle_only_in_live_japanese_mode() {
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(map_key(key, false, true), Action::Romaji('k'));
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
                false,
                true
            ),
            Action::ToggleJapaneseScript
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
                false,
                true
            ),
            Action::JapaneseConvertNext
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), false, true),
            Action::JapaneseConvertNext
        );
        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
                false,
                true
            ),
            Action::JapaneseConvertPrev
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), false, true),
            Action::CancelComposition
        );
    }
}
