//! Presentation layer: screen layout and color themes.

pub mod layout;
pub mod themes;

pub use layout::Layout;
pub use themes::Theme;

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
