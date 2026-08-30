//! Live Japanese input with romaji composition and contextual kana-to-kanji
//! conversion backed by the Akaza model shipped in the Japanese language pack.

#[cfg(test)]
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use libakaza::config::EngineConfig;
use libakaza::engine::base::HenkanEngine;
use libakaza::engine::bigram_word_viterbi_engine::{
    BigramWordViterbiEngine, BigramWordViterbiEngineBuilder,
};
use libakaza::graph::reranking::ReRankingWeights;
use libakaza::kana_kanji::marisa_kana_kanji_dict::MarisaKanaKanjiDict;
use libakaza::lm::system_bigram::MarisaSystemBigramLM;
use libakaza::lm::system_unigram_lm::MarisaSystemUnigramLM;

const MAX_CANDIDATES_PER_SEGMENT: usize = 9;

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

type AkazaEngine =
    BigramWordViterbiEngine<MarisaSystemUnigramLM, MarisaSystemBigramLM, MarisaKanaKanjiDict>;

#[derive(Debug, Clone)]
struct Segment {
    candidates: Vec<String>,
    selected: usize,
}

#[derive(Debug, Clone)]
struct Conversion {
    segments: Vec<Segment>,
    active: usize,
}

#[derive(Debug, Default)]
pub struct Composer {
    pending: String,
    reading: String,
    katakana: bool,
    engine: Option<Rc<AkazaEngine>>,
    conversion: Option<Conversion>,
    #[cfg(test)]
    fixture_results: HashMap<String, Vec<Vec<String>>>,
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_model(&mut self, path: &Path) -> Result<(), String> {
        seed_prebuilt_cache(path)?;
        let config = EngineConfig {
            dicts: Vec::new(),
            dict_cache: true,
            model: path.to_string_lossy().into_owned(),
            reranking_weights: ReRankingWeights::default(),
            convert_k: 10,
        };
        let engine = BigramWordViterbiEngineBuilder::new(config)
            .build()
            .map_err(|error| format!("cannot load Japanese conversion model: {error:#}"))?;
        self.engine = Some(Rc::new(engine));
        Ok(())
    }

    pub fn clear_model(&mut self) {
        self.engine = None;
        self.conversion = None;
    }

    #[cfg(test)]
    fn set_fixture_result(&mut self, reading: &str, segments: &[&[&str]]) {
        self.fixture_results.insert(
            reading.to_string(),
            segments
                .iter()
                .map(|candidates| candidates.iter().map(|value| value.to_string()).collect())
                .collect(),
        );
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty() || !self.reading.is_empty() || self.conversion.is_some()
    }

    pub fn is_converting(&self) -> bool {
        self.conversion.is_some()
    }

    pub fn candidate_position(&self) -> Option<(usize, usize)> {
        let conversion = self.conversion.as_ref()?;
        let segment = conversion.segments.get(conversion.active)?;
        Some((segment.selected + 1, segment.candidates.len()))
    }

    pub fn segment_position(&self) -> Option<(usize, usize)> {
        let conversion = self.conversion.as_ref()?;
        Some((conversion.active + 1, conversion.segments.len()))
    }

    pub fn composing_string(&self) -> String {
        if let Some(conversion) = &self.conversion {
            return conversion
                .segments
                .iter()
                .filter_map(|segment| segment.candidates.get(segment.selected))
                .cloned()
                .collect();
        }
        let mut text = to_display_script(&self.reading, self.katakana);
        text.push_str(&self.pending);
        text
    }

    pub fn input(&mut self, character: char, katakana: bool) -> Vec<char> {
        self.katakana = katakana;
        let mut committed = self.confirm_selected();
        if character == '-' {
            self.finish_pending(false);
            self.reading.push('ー');
            return committed;
        }
        if character == '\'' {
            self.pending.push(character);
        } else if character.is_ascii_alphabetic() {
            self.pending.push(character.to_ascii_lowercase());
        } else {
            committed.extend(self.flush());
            committed.push(character);
            return committed;
        }
        self.finish_pending(false);
        committed
    }

    pub fn convert_next(&mut self) -> bool {
        self.finish_pending(true);
        if self.reading.is_empty() {
            return false;
        }
        if self.conversion.is_none() {
            #[cfg(test)]
            if let Some(segments) = self.fixture_results.get(&self.reading).cloned() {
                self.conversion = conversion_from_surfaces(segments);
                return self.conversion.is_some();
            }
            let Some(engine) = self.engine.as_ref() else {
                return false;
            };
            let Ok(result) = engine.convert(&self.reading, None) else {
                return false;
            };
            let segments = result
                .into_iter()
                .filter_map(|candidates| {
                    candidates.first()?;
                    let mut surfaces = Vec::new();
                    for candidate in candidates {
                        let surface = candidate.surface_with_dynamic();
                        if !surfaces.contains(&surface) {
                            surfaces.push(surface);
                        }
                        if surfaces.len() == MAX_CANDIDATES_PER_SEGMENT {
                            break;
                        }
                    }
                    (!surfaces.is_empty()).then_some(Segment {
                        candidates: surfaces,
                        selected: 0,
                    })
                })
                .collect::<Vec<_>>();
            if segments.is_empty() {
                return false;
            }
            self.conversion = Some(Conversion {
                segments,
                active: 0,
            });
            return true;
        }

        let conversion = self.conversion.as_mut().expect("checked above");
        let segment = &mut conversion.segments[conversion.active];
        segment.selected = (segment.selected + 1) % segment.candidates.len();
        true
    }

    pub fn convert_prev(&mut self) -> bool {
        let Some(conversion) = self.conversion.as_mut() else {
            return false;
        };
        let segment = &mut conversion.segments[conversion.active];
        segment.selected = if segment.selected == 0 {
            segment.candidates.len() - 1
        } else {
            segment.selected - 1
        };
        true
    }

    pub fn move_segment_left(&mut self) -> bool {
        let Some(conversion) = self.conversion.as_mut() else {
            return false;
        };
        conversion.active = conversion.active.saturating_sub(1);
        true
    }

    pub fn move_segment_right(&mut self) -> bool {
        let Some(conversion) = self.conversion.as_mut() else {
            return false;
        };
        conversion.active = (conversion.active + 1).min(conversion.segments.len() - 1);
        true
    }

    pub fn flush(&mut self) -> Vec<char> {
        self.finish_pending(true);
        let committed = if self.conversion.is_some() {
            self.composing_string()
        } else {
            to_display_script(&self.reading, self.katakana)
        };
        self.clear_state();
        committed.chars().collect()
    }

    pub fn backspace(&mut self) -> bool {
        if self.pending.pop().is_some() {
            return true;
        }
        if self.is_converting() {
            self.conversion = None;
            return true;
        }
        self.reading.pop().is_some()
    }

    pub fn cancel(&mut self) -> bool {
        if self.is_converting() {
            self.conversion = None;
            return true;
        }
        if self.pending.is_empty() && self.reading.is_empty() {
            return false;
        }
        self.pending.clear();
        self.reading.clear();
        true
    }

    fn confirm_selected(&mut self) -> Vec<char> {
        if self.conversion.is_some() {
            let committed = self.composing_string();
            self.clear_state();
            committed.chars().collect()
        } else {
            Vec::new()
        }
    }

    fn clear_state(&mut self) {
        self.pending.clear();
        self.reading.clear();
        self.conversion = None;
    }

    fn finish_pending(&mut self, final_input: bool) {
        loop {
            if self.pending.is_empty() {
                break;
            }

            if self.pending.starts_with("n'") {
                self.reading.push('ん');
                self.pending.drain(..2);
                continue;
            }

            let mut chars = self.pending.chars();
            let first = chars.next().unwrap_or_default();
            let second = chars.next();
            if second == Some(first) && is_consonant(first) && first != 'n' {
                self.reading.push('っ');
                self.pending.remove(0);
                continue;
            }
            if self.pending.starts_with("nn") {
                self.reading.push('ん');
                self.pending.remove(0);
                continue;
            }
            if first == 'n'
                && second.is_some_and(|next| !matches!(next, 'a' | 'i' | 'u' | 'e' | 'o' | 'y'))
            {
                self.reading.push('ん');
                self.pending.remove(0);
                continue;
            }

            if let Some((_, kana)) = ROMAJI.iter().find(|(romaji, _)| *romaji == self.pending) {
                self.reading.push_str(kana);
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
                self.reading.push('ん');
                self.pending.clear();
                continue;
            }

            self.reading.push(self.pending.remove(0));
        }
    }
}

fn seed_prebuilt_cache(model: &Path) -> Result<(), String> {
    let source = model.join("prebuilt-cache");
    if !source.is_dir() {
        return Ok(());
    }
    let destination = akaza_cache_dir()
        .ok_or_else(|| "cannot determine a cache directory for Japanese conversion".to_string())?;
    fs::create_dir_all(&destination)
        .map_err(|error| format!("cannot create Japanese conversion cache: {error}"))?;
    let dictionary_modified = fs::metadata(model.join("SKK-JISYO.akaza"))
        .and_then(|metadata| metadata.modified())
        .ok();

    for name in [
        "kana_kanji_cache.marisa",
        "kana_trie_cache.marisa",
        "single_term_cache.marisa",
    ] {
        let from = source.join(name);
        if !from.is_file() {
            continue;
        }
        let to = destination.join(name);
        let cache_is_current = fs::metadata(&to).ok().is_some_and(|metadata| {
            metadata.len() == fs::metadata(&from).map_or(0, |source| source.len())
                && dictionary_modified.is_none_or(|modified| {
                    metadata.modified().is_ok_and(|cached| cached >= modified)
                })
        });
        if !cache_is_current {
            fs::copy(&from, &to)
                .map_err(|error| format!("cannot seed Japanese conversion cache: {error}"))?;
        }
    }
    Ok(())
}

fn akaza_cache_dir() -> Option<PathBuf> {
    if let Some(path) = env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(path).join("akaza"));
    }
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache/akaza"))
}

#[cfg(test)]
fn conversion_from_surfaces(segments: Vec<Vec<String>>) -> Option<Conversion> {
    let segments = segments
        .into_iter()
        .filter(|candidates| !candidates.is_empty())
        .map(|candidates| Segment {
            candidates,
            selected: 0,
        })
        .collect::<Vec<_>>();
    (!segments.is_empty()).then_some(Conversion {
        segments,
        active: 0,
    })
}

fn is_consonant(character: char) -> bool {
    character.is_ascii_alphabetic() && !matches!(character, 'a' | 'i' | 'u' | 'e' | 'o')
}

fn to_display_script(text: &str, katakana: bool) -> String {
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
    use std::path::PathBuf;

    fn compose(text: &str, katakana: bool) -> String {
        let mut composer = Composer::new();
        for character in text.chars() {
            composer.input(character, katakana);
        }
        composer.flush().into_iter().collect()
    }

    fn composer_with_fixture() -> Composer {
        let mut composer = Composer::new();
        composer.set_fixture_result(
            "わたしはがくせいです",
            &[&["私", "わたし"], &["は"], &["学生"], &["です"]],
        );
        composer.set_fixture_result("にほんご", &[&["日本語", "にほんご"]]);
        composer.set_fixture_result("かみがながい", &[&["髪", "紙"], &["が"], &["長い"]]);
        composer.set_fixture_result("かみがしろい", &[&["紙", "髪"], &["が"], &["白い"]]);
        composer
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

    #[test]
    fn converts_a_sentence_to_the_best_candidate() {
        let mut composer = composer_with_fixture();
        for character in "watashihagakuseidesu".chars() {
            composer.input(character, false);
        }

        assert_eq!(composer.composing_string(), "わたしはがくせいです");
        assert!(composer.convert_next());
        assert_eq!(composer.composing_string(), "私は学生です");
        assert!(composer
            .candidate_position()
            .is_some_and(|(current, total)| current == 1 && total >= 1));
    }

    #[test]
    fn cycles_candidates_and_cancels_back_to_kana() {
        let mut composer = composer_with_fixture();
        for character in "nihongo".chars() {
            composer.input(character, false);
        }

        assert!(composer.convert_next());
        assert_eq!(composer.composing_string(), "日本語");
        assert!(composer.convert_next());
        assert_eq!(composer.composing_string(), "にほんご");
        assert!(composer.backspace());
        assert_eq!(composer.composing_string(), "にほんご");
    }

    #[test]
    fn flush_commits_the_selected_conversion() {
        let mut composer = composer_with_fixture();
        for character in "watashihagakuseidesu".chars() {
            composer.input(character, false);
        }
        assert!(composer.convert_next());

        let committed: String = composer.flush().into_iter().collect();
        assert_eq!(committed, "私は学生です");
        assert!(!composer.has_pending());
    }

    #[test]
    fn contextual_skip_bigram_changes_the_homophone_choice() {
        let mut hair = composer_with_fixture();
        for character in "kamiganagai".chars() {
            hair.input(character, false);
        }
        assert!(hair.convert_next());
        assert_eq!(hair.composing_string(), "髪が長い");

        let mut paper = composer_with_fixture();
        for character in "kamigashiroi".chars() {
            paper.input(character, false);
        }
        assert!(paper.convert_next());
        assert_eq!(paper.composing_string(), "紙が白い");
    }

    #[test]
    fn moves_between_clause_candidate_lists() {
        let mut composer = composer_with_fixture();
        for character in "kamiganagai".chars() {
            composer.input(character, false);
        }
        assert!(composer.convert_next());
        assert_eq!(composer.segment_position(), Some((1, 3)));
        assert!(composer.move_segment_right());
        assert_eq!(composer.segment_position(), Some((2, 3)));
        assert!(composer.move_segment_left());
        assert_eq!(composer.segment_position(), Some((1, 3)));
    }

    #[test]
    #[ignore = "set TERMLEAF_AKAZA_MODEL_DIR to run against the released model"]
    fn released_model_uses_sentence_context() {
        let path = std::env::var_os("TERMLEAF_AKAZA_MODEL_DIR")
            .map(PathBuf::from)
            .expect("TERMLEAF_AKAZA_MODEL_DIR must point to akaza-default-model");
        let mut composer = Composer::new();
        composer
            .load_model(&path)
            .expect("load released Akaza model");
        for character in "kyouhaiitenkidesune".chars() {
            composer.input(character, false);
        }
        assert!(composer.convert_next());
        assert_eq!(composer.composing_string(), "今日はいい天気ですね");
    }

    #[test]
    fn falls_back_to_raw_kana_without_a_model() {
        let mut composer = Composer::new();
        for character in "nihongo".chars() {
            composer.input(character, false);
        }
        assert!(!composer.convert_next());
        assert_eq!(composer.flush().into_iter().collect::<String>(), "にほんご");
    }

    #[test]
    fn cancel_clears_unconfirmed_input() {
        let mut composer = Composer::new();
        for character in "nihongo".chars() {
            composer.input(character, false);
        }
        assert!(composer.cancel());
        assert!(!composer.has_pending());
    }
}
