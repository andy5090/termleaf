//! Galmuri9-backed bitmap glyphs for the enlarged focus zone.
//!
//! Termleaf embeds printable ASCII. Optional data-only language packs add
//! Hangul, kana, fullwidth characters, and the CJK glyphs available in
//! Galmuri9 2.40.4.
//!
//! The subset is licensed under the SIL Open Font License 1.1; see
//! `THIRD_PARTY_LICENSES.md` and `assets/OFL-1.1.txt`.

use crate::language::LanguageRegistry;

const FONT_DATA: &[u8] = include_bytes!("../../assets/galmuri9-2.40.4-core.bin");
const ROWS_PER_GLYPH: usize = 10;
const BYTES_PER_GLYPH: usize = 1 + ROWS_PER_GLYPH * 2;

const ASCII_START: u32 = 0x0020;
const ASCII_END: u32 = 0x007E;
const ASCII_COUNT: usize = (ASCII_END - ASCII_START + 1) as usize;

/// A monochrome bitmap glyph. `rows[y]` stores one row; bit `width - 1` is
/// the leftmost pixel.
#[derive(Debug, Clone)]
pub struct Glyph {
    pub width: usize,
    pub height: usize,
    pub rows: Vec<u16>,
}

impl Glyph {
    /// Whether the pixel at `(x, y)` is lit.
    pub fn lit(&self, x: usize, y: usize) -> bool {
        if y >= self.height || x >= self.width {
            return false;
        }
        (self.rows[y] >> (self.width - 1 - x)) & 1 == 1
    }
}

/// Look up a glyph, falling back to an outlined replacement box for code
/// points outside Termleaf's embedded subset.
pub fn glyph_for(c: char, languages: &LanguageRegistry) -> Glyph {
    glyph_index(c)
        .map(decode_glyph)
        .or_else(|| {
            languages.glyph(c).map(|glyph| Glyph {
                width: glyph.width,
                height: ROWS_PER_GLYPH,
                rows: glyph.rows.to_vec(),
            })
        })
        .unwrap_or_else(fallback_glyph)
}

fn glyph_index(c: char) -> Option<usize> {
    let codepoint = c as u32;
    match codepoint {
        ASCII_START..=ASCII_END => Some((codepoint - ASCII_START) as usize),
        _ => None,
    }
}

fn decode_glyph(index: usize) -> Glyph {
    debug_assert_eq!(FONT_DATA.len(), ASCII_COUNT * BYTES_PER_GLYPH);
    let start = index * BYTES_PER_GLYPH;
    let width = FONT_DATA[start] as usize;
    let rows = FONT_DATA[start + 1..start + BYTES_PER_GLYPH]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| u16::from_be_bytes(*bytes))
        .collect();
    Glyph {
        width,
        height: ROWS_PER_GLYPH,
        rows,
    }
}

fn fallback_glyph() -> Glyph {
    Glyph {
        width: 8,
        height: 10,
        rows: vec![0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::{Language, LanguageRegistry};
    use std::path::Path;

    fn registry_with(language: Language) -> LanguageRegistry {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should follow Unix epoch")
            .as_nanos();
        let mut registry = LanguageRegistry::load_from(std::env::temp_dir().join(format!(
            "termleaf-font-test-{}-{}-{unique}",
            language.code(),
            std::process::id()
        )));
        registry
            .install_from_source(
                language,
                Path::new(&format!("language-packs/{}", language.code())),
            )
            .unwrap();
        registry
    }

    #[test]
    fn embedded_subset_has_the_expected_shape() {
        assert_eq!(ASCII_COUNT, 95);
        assert_eq!(FONT_DATA.len(), ASCII_COUNT * BYTES_PER_GLYPH);
    }

    #[test]
    fn ascii_preserves_letter_case() {
        let languages =
            LanguageRegistry::load_from(std::env::temp_dir().join("termleaf-empty-font"));
        let upper = glyph_for('A', &languages);
        let lower = glyph_for('a', &languages);
        assert_eq!(upper.height, 10);
        assert_eq!(lower.height, 10);
        assert_ne!(upper.rows, lower.rows);
    }

    #[test]
    fn completed_hangul_comes_from_distinct_full_size_glyphs() {
        let languages = registry_with(Language::Korean);
        let han = glyph_for('한', &languages);
        let geul = glyph_for('글', &languages);
        let ga = glyph_for('가', &languages);
        let kka = glyph_for('까', &languages);

        for glyph in [&han, &geul, &ga, &kka] {
            assert_eq!((glyph.width, glyph.height), (10, 10));
            assert!(glyph.rows.iter().any(|&row| row != 0));
        }
        assert_ne!(han.rows, geul.rows);
        assert_ne!(ga.rows, kka.rows);
    }

    #[test]
    fn every_modern_hangul_syllable_is_present() {
        let languages = registry_with(Language::Korean);
        for codepoint in 0xAC00..=0xD7A3 {
            let character = char::from_u32(codepoint).expect("Hangul code point should be valid");
            let glyph = glyph_for(character, &languages);
            assert_eq!(glyph.width, 10, "unexpected width for U+{codepoint:04X}");
            assert!(
                glyph.rows.iter().any(|&row| row != 0),
                "empty glyph for U+{codepoint:04X}"
            );
        }
    }

    #[test]
    fn latin_descenders_reach_the_shared_bottom_row() {
        let languages =
            LanguageRegistry::load_from(std::env::temp_dir().join("termleaf-empty-font-3"));
        for character in ['g', 'j', 'p', 'q', 'y'] {
            assert_ne!(
                glyph_for(character, &languages).rows[9],
                0,
                "{character} should retain its descender"
            );
        }
    }

    #[test]
    fn whitespace_is_blank_and_unknown_characters_use_a_box() {
        let languages =
            LanguageRegistry::load_from(std::env::temp_dir().join("termleaf-empty-font-2"));
        let space = glyph_for(' ', &languages);
        assert!(space.rows.iter().all(|&row| row == 0));

        let fallback = glyph_for('🦀', &languages);
        assert_eq!((fallback.width, fallback.height), (8, 10));
        assert!(fallback.lit(0, 0));
        assert!(fallback.lit(7, 9));
    }
}
