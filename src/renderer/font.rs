//! Galmuri9-backed bitmap glyphs for the enlarged focus zone.
//!
//! Termleaf embeds a converted subset of Galmuri9 2.40.4 containing printable
//! ASCII, Hangul Compatibility Jamo, and all 11,172 precomposed Hangul
//! syllables. Glyphs share a 10-row baseline-aware canvas while retaining
//! their original advance widths.
//!
//! The subset is licensed under the SIL Open Font License 1.1; see
//! `THIRD_PARTY_LICENSES.md` and `assets/OFL-1.1.txt`.

const FONT_DATA: &[u8] = include_bytes!("../../assets/galmuri9-2.40.4-termleaf.bin");
const ROWS_PER_GLYPH: usize = 10;
const BYTES_PER_GLYPH: usize = 1 + ROWS_PER_GLYPH * 2;

const ASCII_START: u32 = 0x0020;
const ASCII_END: u32 = 0x007E;
const JAMO_START: u32 = 0x3131;
const JAMO_END: u32 = 0x3163;
const HANGUL_START: u32 = 0xAC00;
const HANGUL_END: u32 = 0xD7A3;

const ASCII_COUNT: usize = (ASCII_END - ASCII_START + 1) as usize;
const JAMO_COUNT: usize = (JAMO_END - JAMO_START + 1) as usize;
const HANGUL_COUNT: usize = (HANGUL_END - HANGUL_START + 1) as usize;
const GLYPH_COUNT: usize = ASCII_COUNT + JAMO_COUNT + HANGUL_COUNT;

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
pub fn glyph_for(c: char) -> Glyph {
    glyph_index(c)
        .map(decode_glyph)
        .unwrap_or_else(fallback_glyph)
}

fn glyph_index(c: char) -> Option<usize> {
    let codepoint = c as u32;
    match codepoint {
        ASCII_START..=ASCII_END => Some((codepoint - ASCII_START) as usize),
        JAMO_START..=JAMO_END => Some(ASCII_COUNT + (codepoint - JAMO_START) as usize),
        HANGUL_START..=HANGUL_END => {
            Some(ASCII_COUNT + JAMO_COUNT + (codepoint - HANGUL_START) as usize)
        }
        _ => None,
    }
}

fn decode_glyph(index: usize) -> Glyph {
    debug_assert_eq!(FONT_DATA.len(), GLYPH_COUNT * BYTES_PER_GLYPH);
    let start = index * BYTES_PER_GLYPH;
    let width = FONT_DATA[start] as usize;
    let rows = FONT_DATA[start + 1..start + BYTES_PER_GLYPH]
        .chunks_exact(2)
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
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

    #[test]
    fn embedded_subset_has_the_expected_shape() {
        assert_eq!(GLYPH_COUNT, 11_318);
        assert_eq!(FONT_DATA.len(), GLYPH_COUNT * BYTES_PER_GLYPH);
    }

    #[test]
    fn ascii_preserves_letter_case() {
        let upper = glyph_for('A');
        let lower = glyph_for('a');
        assert_eq!(upper.height, 10);
        assert_eq!(lower.height, 10);
        assert_ne!(upper.rows, lower.rows);
    }

    #[test]
    fn completed_hangul_comes_from_distinct_full_size_glyphs() {
        let han = glyph_for('한');
        let geul = glyph_for('글');
        let ga = glyph_for('가');
        let kka = glyph_for('까');

        for glyph in [&han, &geul, &ga, &kka] {
            assert_eq!((glyph.width, glyph.height), (10, 10));
            assert!(glyph.rows.iter().any(|&row| row != 0));
        }
        assert_ne!(han.rows, geul.rows);
        assert_ne!(ga.rows, kka.rows);
    }

    #[test]
    fn every_modern_hangul_syllable_is_present() {
        for codepoint in HANGUL_START..=HANGUL_END {
            let character = char::from_u32(codepoint).expect("Hangul code point should be valid");
            let glyph = glyph_for(character);
            assert_eq!(glyph.width, 10, "unexpected width for U+{codepoint:04X}");
            assert!(
                glyph.rows.iter().any(|&row| row != 0),
                "empty glyph for U+{codepoint:04X}"
            );
        }
    }

    #[test]
    fn latin_descenders_reach_the_shared_bottom_row() {
        for character in ['g', 'j', 'p', 'q', 'y'] {
            assert_ne!(
                glyph_for(character).rows[9],
                0,
                "{character} should retain its descender"
            );
        }
    }

    #[test]
    fn whitespace_is_blank_and_unknown_characters_use_a_box() {
        let space = glyph_for(' ');
        assert!(space.rows.iter().all(|&row| row == 0));

        let fallback = glyph_for('🦀');
        assert_eq!((fallback.width, fallback.height), (8, 10));
        assert!(fallback.lit(0, 0));
        assert!(fallback.lit(7, 9));
    }
}
