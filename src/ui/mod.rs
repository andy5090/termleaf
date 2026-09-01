//! Presentation layer: screen layout and color themes.

pub mod file_prompt;
pub mod help;
pub mod language_settings;
pub mod layout;
pub mod sound_settings;
pub mod themes;
pub mod touch;

pub use file_prompt::{FilePrompt, FilePromptError, FilePromptKind};
pub use help::HelpOverlay;
pub use language_settings::LanguageSettings;
pub use layout::Layout;
pub use sound_settings::SoundSettings;
pub use themes::Theme;
pub use touch::{TouchCommand, TouchContext, TouchPage};

/// Display width of a character in terminal cells (Hangul / CJK are 2).
pub fn char_width(c: char) -> u16 {
    let cp = c as u32;
    let wide = matches!(cp,
        0x1100..=0x115F   // Hangul Jamo
        | 0x2E80..=0x303E // CJK radicals, Kangxi, punctuation
        | 0x3041..=0x33FF // Hiragana .. CJK compatibility
        | 0x3400..=0x4DBF // CJK Ext A
        | 0x4E00..=0x9FFF // CJK Unified
        | 0xA000..=0xA4CF // Yi
        | 0xAC00..=0xD7A3 // Hangul syllables
        | 0xF900..=0xFAFF // CJK compatibility ideographs
        | 0xFE30..=0xFE4F // CJK compatibility forms
        | 0xFF00..=0xFF60 // Fullwidth forms
        | 0xFFE0..=0xFFE6
    );
    if wide {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_terminal_cell_width_for_latin_and_hangul() {
        assert_eq!(char_width('A'), 1);
        assert_eq!(char_width('한'), 2);
        assert_eq!(char_width('ㄱ'), 2);
    }
}
