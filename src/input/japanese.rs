//! Lightweight live Japanese input for terminals that cannot expose IME
//! composition events. It converts common romaji sequences to kana; kanji
//! conversion remains the operating system IME's responsibility.

const ROMAJI: &[(&str, &str)] = &[
    ("xtsu", "っ"),
    ("ltsu", "っ"),
    ("kya", "きゃ"),
    ("kyu", "きゅ"),
    ("kyo", "きょ"),
    ("gya", "ぎゃ"),
    ("gyu", "ぎゅ"),
    ("gyo", "ぎょ"),
    ("sha", "しゃ"),
    ("shu", "しゅ"),
    ("sho", "しょ"),
    ("sya", "しゃ"),
    ("syu", "しゅ"),
    ("syo", "しょ"),
    ("ja", "じゃ"),
    ("ju", "じゅ"),
    ("jo", "じょ"),
    ("jya", "じゃ"),
    ("jyu", "じゅ"),
    ("jyo", "じょ"),
    ("cha", "ちゃ"),
    ("chu", "ちゅ"),
    ("cho", "ちょ"),
    ("tya", "ちゃ"),
    ("tyu", "ちゅ"),
    ("tyo", "ちょ"),
    ("nya", "にゃ"),
    ("nyu", "にゅ"),
    ("nyo", "にょ"),
    ("hya", "ひゃ"),
    ("hyu", "ひゅ"),
    ("hyo", "ひょ"),
    ("bya", "びゃ"),
    ("byu", "びゅ"),
    ("byo", "びょ"),
    ("pya", "ぴゃ"),
    ("pyu", "ぴゅ"),
    ("pyo", "ぴょ"),
    ("mya", "みゃ"),
    ("myu", "みゅ"),
    ("myo", "みょ"),
    ("rya", "りゃ"),
    ("ryu", "りゅ"),
    ("ryo", "りょ"),
    ("shi", "し"),
    ("chi", "ち"),
    ("tsu", "つ"),
    ("dya", "ぢゃ"),
    ("dyu", "ぢゅ"),
    ("dyo", "ぢょ"),
    ("fa", "ふぁ"),
    ("fi", "ふぃ"),
    ("fe", "ふぇ"),
    ("fo", "ふぉ"),
    ("va", "ゔぁ"),
    ("vi", "ゔぃ"),
    ("vu", "ゔ"),
    ("ve", "ゔぇ"),
    ("vo", "ゔぉ"),
    ("xya", "ゃ"),
    ("xyu", "ゅ"),
    ("xyo", "ょ"),
    ("lya", "ゃ"),
    ("lyu", "ゅ"),
    ("lyo", "ょ"),
    ("xa", "ぁ"),
    ("xi", "ぃ"),
    ("xu", "ぅ"),
    ("xe", "ぇ"),
    ("xo", "ぉ"),
    ("la", "ぁ"),
    ("li", "ぃ"),
    ("lu", "ぅ"),
    ("le", "ぇ"),
    ("lo", "ぉ"),
    ("ka", "か"),
    ("ki", "き"),
    ("ku", "く"),
    ("ke", "け"),
    ("ko", "こ"),
    ("ga", "が"),
    ("gi", "ぎ"),
    ("gu", "ぐ"),
    ("ge", "げ"),
    ("go", "ご"),
    ("sa", "さ"),
    ("si", "し"),
    ("su", "す"),
    ("se", "せ"),
    ("so", "そ"),
    ("za", "ざ"),
    ("zi", "じ"),
    ("zu", "ず"),
    ("ze", "ぜ"),
    ("zo", "ぞ"),
    ("ta", "た"),
    ("ti", "ち"),
    ("tu", "つ"),
    ("te", "て"),
    ("to", "と"),
    ("da", "だ"),
    ("di", "ぢ"),
    ("du", "づ"),
    ("de", "で"),
    ("do", "ど"),
    ("na", "な"),
    ("ni", "に"),
    ("nu", "ぬ"),
    ("ne", "ね"),
    ("no", "の"),
    ("ha", "は"),
    ("hi", "ひ"),
    ("hu", "ふ"),
    ("fu", "ふ"),
    ("he", "へ"),
    ("ho", "ほ"),
    ("ba", "ば"),
    ("bi", "び"),
    ("bu", "ぶ"),
    ("be", "べ"),
    ("bo", "ぼ"),
    ("pa", "ぱ"),
    ("pi", "ぴ"),
    ("pu", "ぷ"),
    ("pe", "ぺ"),
    ("po", "ぽ"),
    ("ma", "ま"),
    ("mi", "み"),
    ("mu", "む"),
    ("me", "め"),
    ("mo", "も"),
    ("ya", "や"),
    ("yu", "ゆ"),
    ("yo", "よ"),
    ("ra", "ら"),
    ("ri", "り"),
    ("ru", "る"),
    ("re", "れ"),
    ("ro", "ろ"),
    ("wa", "わ"),
    ("wo", "を"),
    ("a", "あ"),
    ("i", "い"),
    ("u", "う"),
    ("e", "え"),
    ("o", "お"),
];

#[derive(Debug, Default, Clone)]
pub struct Composer {
    pending: String,
    katakana: bool,
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn composing_string(&self) -> String {
        self.pending.clone()
    }

    pub fn input(&mut self, character: char, katakana: bool) -> Vec<char> {
        self.katakana = katakana;
        if character == '-' {
            let mut committed = self.flush();
            committed.push('ー');
            return committed;
        }
        if character == '\'' {
            self.pending.push(character);
        } else if character.is_ascii_alphabetic() {
            self.pending.push(character.to_ascii_lowercase());
        } else {
            let mut committed = self.flush();
            committed.push(character);
            return committed;
        }
        self.consume(false)
    }

    pub fn flush(&mut self) -> Vec<char> {
        self.consume(true)
    }

    pub fn backspace(&mut self) -> bool {
        self.pending.pop().is_some()
    }

    fn consume(&mut self, final_input: bool) -> Vec<char> {
        let mut committed = Vec::new();
        loop {
            if self.pending.is_empty() {
                break;
            }

            if self.pending.starts_with("n'") {
                committed.extend(kana_chars("ん", self.katakana));
                self.pending.drain(..2);
                continue;
            }

            let mut chars = self.pending.chars();
            let first = chars.next().unwrap_or_default();
            let second = chars.next();
            if second == Some(first) && is_consonant(first) && first != 'n' {
                committed.extend(kana_chars("っ", self.katakana));
                self.pending.remove(0);
                continue;
            }
            if self.pending.starts_with("nn") {
                committed.extend(kana_chars("ん", self.katakana));
                self.pending.remove(0);
                continue;
            }
            if first == 'n'
                && second.is_some_and(|next| !matches!(next, 'a' | 'i' | 'u' | 'e' | 'o' | 'y'))
            {
                committed.extend(kana_chars("ん", self.katakana));
                self.pending.remove(0);
                continue;
            }

            if let Some((_, kana)) = ROMAJI.iter().find(|(romaji, _)| *romaji == self.pending) {
                committed.extend(kana_chars(kana, self.katakana));
                self.pending.clear();
                continue;
            }

            if ROMAJI
                .iter()
                .any(|(romaji, _)| romaji.starts_with(&self.pending))
                || (!final_input && self.pending == "n")
            {
                break;
            }

            if final_input && self.pending == "n" {
                committed.extend(kana_chars("ん", self.katakana));
                self.pending.clear();
                continue;
            }

            committed.push(self.pending.remove(0));
        }
        committed
    }
}

fn is_consonant(character: char) -> bool {
    character.is_ascii_alphabetic() && !matches!(character, 'a' | 'i' | 'u' | 'e' | 'o')
}

fn kana_chars(text: &str, katakana: bool) -> Vec<char> {
    text.chars()
        .map(|character| {
            if katakana && ('ぁ'..='ゖ').contains(&character) {
                char::from_u32(character as u32 + 0x60).unwrap_or(character)
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compose(text: &str, katakana: bool) -> String {
        let mut composer = Composer::new();
        let mut output = Vec::new();
        for character in text.chars() {
            output.extend(composer.input(character, katakana));
        }
        output.extend(composer.flush());
        output.into_iter().collect()
    }

    #[test]
    fn converts_common_romaji_to_hiragana() {
        assert_eq!(compose("konnichiha", false), "こんにちは");
        assert_eq!(compose("nihongo", false), "にほんご");
    }

    #[test]
    fn handles_small_tsu_and_combined_kana() {
        assert_eq!(compose("gakkou", false), "がっこう");
        assert_eq!(compose("kyoushitsu", false), "きょうしつ");
    }

    #[test]
    fn converts_the_same_stream_to_katakana() {
        assert_eq!(compose("taipuraita-", true), "タイプライター");
    }

    #[test]
    fn keeps_an_incomplete_sequence_visible_and_editable() {
        let mut composer = Composer::new();
        assert!(composer.input('k', false).is_empty());
        assert_eq!(composer.composing_string(), "k");
        assert!(composer.backspace());
        assert!(composer.composing_string().is_empty());
    }
}
