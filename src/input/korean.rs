//! Hangul (Korean) composition engine.
//!
//! Implements a 2-set (두벌식) keyboard automaton. Instead of relying on the
//! operating system IME — which usually hands the terminal only the final,
//! fully composed syllable — Termleaf drives composition itself so it can reveal
//! the *process*: the lead consonant, then the vowel, then the tail consonant
//! appearing one jamo at a time (예: ㅎ → 하 → 한).
//!
//! A precomposed syllable is `0xAC00 + (cho*21 + jung)*28 + jong`.

/// Lead consonants (초성), 19 of them.
pub const CHO: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
    'ㅌ', 'ㅍ', 'ㅎ',
];

/// Vowels (중성), 21 of them.
pub const JUNG: [char; 21] = [
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ',
    'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];

/// Tail consonants (종성). Index 0 means "no tail".
pub const JONG: [Option<char>; 28] = [
    None,
    Some('ㄱ'),
    Some('ㄲ'),
    Some('ㄳ'),
    Some('ㄴ'),
    Some('ㄵ'),
    Some('ㄶ'),
    Some('ㄷ'),
    Some('ㄹ'),
    Some('ㄺ'),
    Some('ㄻ'),
    Some('ㄼ'),
    Some('ㄽ'),
    Some('ㄾ'),
    Some('ㄿ'),
    Some('ㅀ'),
    Some('ㅁ'),
    Some('ㅂ'),
    Some('ㅄ'),
    Some('ㅅ'),
    Some('ㅆ'),
    Some('ㅇ'),
    Some('ㅈ'),
    Some('ㅊ'),
    Some('ㅋ'),
    Some('ㅌ'),
    Some('ㅍ'),
    Some('ㅎ'),
];

/// Translate a keystroke (ASCII, dubeolsik layout) into a compatibility jamo.
/// Returns `None` for keys that are not part of the layout.
pub fn key_to_jamo(c: char) -> Option<char> {
    let j = match c {
        // consonants
        'r' => 'ㄱ',
        'R' => 'ㄲ',
        's' => 'ㄴ',
        'e' => 'ㄷ',
        'E' => 'ㄸ',
        'f' => 'ㄹ',
        'a' => 'ㅁ',
        'q' => 'ㅂ',
        'Q' => 'ㅃ',
        't' => 'ㅅ',
        'T' => 'ㅆ',
        'd' => 'ㅇ',
        'w' => 'ㅈ',
        'W' => 'ㅉ',
        'c' => 'ㅊ',
        'z' => 'ㅋ',
        'x' => 'ㅌ',
        'v' => 'ㅍ',
        'g' => 'ㅎ',
        // vowels
        'k' => 'ㅏ',
        'o' => 'ㅐ',
        'i' => 'ㅑ',
        'O' => 'ㅒ',
        'j' => 'ㅓ',
        'p' => 'ㅔ',
        'u' => 'ㅕ',
        'P' => 'ㅖ',
        'h' => 'ㅗ',
        'y' => 'ㅛ',
        'n' => 'ㅜ',
        'b' => 'ㅠ',
        'm' => 'ㅡ',
        'l' => 'ㅣ',
        _ => return None,
    };
    Some(j)
}

fn is_vowel(jamo: char) -> bool {
    JUNG.contains(&jamo)
}

/// Compatibility consonant → 초성 index.
fn cons_to_cho(c: char) -> Option<usize> {
    CHO.iter().position(|&x| x == c)
}

/// Compatibility consonant → 종성 index (if it can be a tail).
fn cons_to_jong(c: char) -> Option<usize> {
    JONG.iter().position(|&x| x == Some(c))
}

/// Basic vowel → 중성 index.
fn vowel_index(v: char) -> Option<usize> {
    JUNG.iter().position(|&x| x == v)
}

/// A single 종성 → its 초성 index (used by the ghost / 도깨비불 rule).
fn jong_to_cho(jong_idx: usize) -> Option<usize> {
    let c = JONG[jong_idx]?;
    cons_to_cho(c)
}

/// Combine a base vowel with an added vowel into a complex vowel (중성 index).
fn combine_jung(base: usize, add: usize) -> Option<usize> {
    // indices: ㅗ=8, ㅜ=13, ㅡ=18, ㅏ=0, ㅐ=1, ㅓ=4, ㅔ=5, ㅣ=20
    let combined = match (base, add) {
        (8, 0) => 9,    // ㅗ + ㅏ = ㅘ
        (8, 1) => 10,   // ㅗ + ㅐ = ㅙ
        (8, 20) => 11,  // ㅗ + ㅣ = ㅚ
        (13, 4) => 14,  // ㅜ + ㅓ = ㅝ
        (13, 5) => 15,  // ㅜ + ㅔ = ㅞ
        (13, 20) => 16, // ㅜ + ㅣ = ㅟ
        (18, 20) => 19, // ㅡ + ㅣ = ㅢ
        _ => return None,
    };
    Some(combined)
}

/// Reverse of [`combine_jung`]: a complex vowel → its base component.
fn uncombine_jung(v: usize) -> Option<usize> {
    let base = match v {
        9..=11 => 8,   // ㅘㅙㅚ → ㅗ
        14..=16 => 13, // ㅝㅞㅟ → ㅜ
        19 => 18,      // ㅢ → ㅡ
        _ => return None,
    };
    Some(base)
}

/// Combine an existing tail with an added consonant into a double tail.
fn combine_jong(base: usize, add: char) -> Option<usize> {
    let combined = match (base, add) {
        (1, 'ㅅ') => 3,   // ㄱ + ㅅ = ㄳ
        (4, 'ㅈ') => 5,   // ㄴ + ㅈ = ㄵ
        (4, 'ㅎ') => 6,   // ㄴ + ㅎ = ㄶ
        (8, 'ㄱ') => 9,   // ㄹ + ㄱ = ㄺ
        (8, 'ㅁ') => 10,  // ㄹ + ㅁ = ㄻ
        (8, 'ㅂ') => 11,  // ㄹ + ㅂ = ㄼ
        (8, 'ㅅ') => 12,  // ㄹ + ㅅ = ㄽ
        (8, 'ㅌ') => 13,  // ㄹ + ㅌ = ㄾ
        (8, 'ㅍ') => 14,  // ㄹ + ㅍ = ㄿ
        (8, 'ㅎ') => 15,  // ㄹ + ㅎ = ㅀ
        (17, 'ㅅ') => 18, // ㅂ + ㅅ = ㅄ
        _ => return None,
    };
    Some(combined)
}

/// Reverse of [`combine_jong`]: a double tail → its base single tail.
fn uncombine_jong(j: usize) -> Option<usize> {
    let base = match j {
        3 => 1,
        5 | 6 => 4,
        9..=15 => 8,
        18 => 17,
        _ => return None,
    };
    Some(base)
}

/// Split a double tail into (remaining tail index, migrating 초성 index) for
/// the ghost rule, e.g. 닭 + ㅏ → 달 + 가.
fn split_jong(j: usize) -> Option<(usize, usize)> {
    // (remaining single jong, moving consonant as compatibility char)
    let (rem, moving) = match j {
        3 => (1, 'ㅅ'),
        5 => (4, 'ㅈ'),
        6 => (4, 'ㅎ'),
        9 => (8, 'ㄱ'),
        10 => (8, 'ㅁ'),
        11 => (8, 'ㅂ'),
        12 => (8, 'ㅅ'),
        13 => (8, 'ㅌ'),
        14 => (8, 'ㅍ'),
        15 => (8, 'ㅎ'),
        18 => (17, 'ㅅ'),
        _ => return None,
    };
    Some((rem, cons_to_cho(moving)?))
}

/// The live Hangul composition state.
#[derive(Debug, Default, Clone)]
pub struct Composer {
    cho: Option<usize>,
    jung: Option<usize>,
    jong: usize,
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.cho.is_none() && self.jung.is_none() && self.jong == 0
    }

    /// Number of logical parts in the composing cluster (0..=3).
    pub fn stage(&self) -> usize {
        let mut stage = 0;
        if self.cho.is_some() {
            stage += 1;
        }
        if self.jung.is_some() {
            stage += 1;
        }
        if self.jong != 0 {
            stage += 1;
        }
        stage
    }

    /// The current cluster rendered as a single (possibly partial) glyph.
    pub fn composing_string(&self) -> String {
        match (self.cho, self.jung) {
            (Some(ci), Some(ji)) => {
                let code = 0xAC00 + (ci * 21 + ji) * 28 + self.jong;
                char::from_u32(code as u32)
                    .map(String::from)
                    .unwrap_or_default()
            }
            (Some(ci), None) => CHO[ci].to_string(),
            (None, Some(ji)) => JUNG[ji].to_string(),
            (None, None) => String::new(),
        }
    }

    /// Finalize the current cluster, returning the produced character(s) and
    /// clearing the composer.
    pub fn flush(&mut self) -> Vec<char> {
        let mut out = Vec::new();
        match (self.cho, self.jung) {
            (Some(ci), Some(ji)) => {
                let code = 0xAC00 + (ci * 21 + ji) * 28 + self.jong;
                if let Some(c) = char::from_u32(code as u32) {
                    out.push(c);
                }
            }
            (Some(ci), None) => out.push(CHO[ci]),
            (None, Some(ji)) => out.push(JUNG[ji]),
            (None, None) => {}
        }
        self.cho = None;
        self.jung = None;
        self.jong = 0;
        out
    }

    /// Feed one jamo (compatibility char) into the automaton. Returns any
    /// characters that got finalized as a result.
    pub fn input(&mut self, jamo: char) -> Vec<char> {
        if is_vowel(jamo) {
            self.input_vowel(jamo)
        } else {
            self.input_consonant(jamo)
        }
    }

    fn input_consonant(&mut self, c: char) -> Vec<char> {
        let mut out = Vec::new();
        match (self.cho, self.jung) {
            (None, None) => match cons_to_cho(c) {
                Some(ci) => self.cho = Some(ci),
                None => out.push(c),
            },
            (Some(_), None) | (None, Some(_)) => {
                out.extend(self.flush());
                match cons_to_cho(c) {
                    Some(ci) => self.cho = Some(ci),
                    None => out.push(c),
                }
            }
            (Some(_), Some(_)) => {
                if self.jong == 0 {
                    match cons_to_jong(c) {
                        Some(ji) => self.jong = ji,
                        None => {
                            out.extend(self.flush());
                            if let Some(ci) = cons_to_cho(c) {
                                self.cho = Some(ci);
                            } else {
                                out.push(c);
                            }
                        }
                    }
                } else if let Some(ji) = combine_jong(self.jong, c) {
                    self.jong = ji;
                } else {
                    out.extend(self.flush());
                    if let Some(ci) = cons_to_cho(c) {
                        self.cho = Some(ci);
                    } else {
                        out.push(c);
                    }
                }
            }
        }
        out
    }

    fn input_vowel(&mut self, v: char) -> Vec<char> {
        let mut out = Vec::new();
        let vi = match vowel_index(v) {
            Some(i) => i,
            None => {
                out.push(v);
                return out;
            }
        };

        // Ghost rule: a vowel after a syllable that already has a tail steals
        // (part of) that tail to become the next syllable's lead.
        if self.jong != 0 {
            if let Some((rem, moving_cho)) = split_jong(self.jong) {
                self.jong = rem;
                out.extend(self.flush());
                self.cho = Some(moving_cho);
                self.jung = Some(vi);
            } else {
                let moving = jong_to_cho(self.jong);
                self.jong = 0;
                out.extend(self.flush());
                self.cho = moving;
                self.jung = Some(vi);
            }
            return out;
        }

        match (self.cho, self.jung) {
            (Some(_), None) => self.jung = Some(vi),
            (Some(_), Some(cur)) | (None, Some(cur)) => {
                if let Some(comb) = combine_jung(cur, vi) {
                    self.jung = Some(comb);
                } else {
                    out.extend(self.flush());
                    self.jung = Some(vi);
                }
            }
            (None, None) => self.jung = Some(vi),
        }
        out
    }

    /// Remove one jamo from the composing cluster, disassembling complex vowels
    /// and double tails one step at a time. Returns `true` if it consumed part
    /// of the cluster (so the editor should NOT delete a committed character).
    pub fn backspace(&mut self) -> bool {
        if self.jong != 0 {
            self.jong = uncombine_jong(self.jong).unwrap_or(0);
            return true;
        }
        if let Some(v) = self.jung {
            match uncombine_jung(v) {
                Some(base) => self.jung = Some(base),
                None => self.jung = None,
            }
            return true;
        }
        if self.cho.is_some() {
            self.cho = None;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_str(keys: &str) -> String {
        let mut c = Composer::new();
        let mut out = String::new();
        for ch in keys.chars() {
            if let Some(j) = key_to_jamo(ch) {
                for committed in c.input(j) {
                    out.push(committed);
                }
            }
        }
        out.push_str(&c.composing_string());
        out
    }

    #[test]
    fn simple_syllable() {
        assert_eq!(type_str("gks"), "한"); // ㅎ ㅏ ㄴ
    }

    #[test]
    fn intermediate_stages() {
        let mut c = Composer::new();
        c.input('ㅎ');
        assert_eq!(c.composing_string(), "ㅎ");
        assert_eq!(c.stage(), 1);
        c.input('ㅏ');
        assert_eq!(c.composing_string(), "하");
        assert_eq!(c.stage(), 2);
        c.input('ㄴ');
        assert_eq!(c.composing_string(), "한");
        assert_eq!(c.stage(), 3);
    }

    #[test]
    fn word_annyeong() {
        // 안녕 = d k s / s u d
        assert_eq!(type_str("dkssud"), "안녕");
    }

    #[test]
    fn complex_vowel() {
        // ㅎ ㅗ ㅏ : ㅗ+ㅏ combine into ㅘ -> 화
        assert_eq!(type_str("ghk"), "화");
    }

    #[test]
    fn double_tail() {
        // 값 = ㄱ ㅏ ㅂ ㅅ  (ㅂ+ㅅ = ㅄ)
        assert_eq!(type_str("rkqt"), "값");
    }

    #[test]
    fn ghost_rule_migration() {
        // 간 + ㅏ  ->  가나  (single tail migrates to the new lead)
        assert_eq!(type_str("rksk"), "가나");
    }

    #[test]
    fn double_final_then_vowel_splits() {
        // 닭 + ㅏ -> 달가 : ㄷㅏㄹㄱ then ㅏ
        assert_eq!(type_str("ekfrk"), "달가");
    }

    #[test]
    fn backspace_disassembles() {
        let mut c = Composer::new();
        for j in ['ㅎ', 'ㅏ', 'ㄴ'] {
            c.input(j);
        }
        assert_eq!(c.composing_string(), "한");
        assert!(c.backspace());
        assert_eq!(c.composing_string(), "하");
        assert!(c.backspace());
        assert_eq!(c.composing_string(), "ㅎ");
        assert!(c.backspace());
        assert!(c.is_empty());
        assert!(!c.backspace());
    }
}
