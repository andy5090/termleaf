//! User-configurable settings, persisted as a tiny `key = value` file.
//!
//! We intentionally avoid a serialization crate to keep configuration simple
//! and the dependency tree small.

use std::fs;
use std::path::PathBuf;

pub const MIN_FONT: u16 = 1;
pub const MAX_FONT: u16 = 5;
pub const MIN_LINE_SPACING: u16 = 1;
pub const MAX_LINE_SPACING: u16 = 3;

/// Runtime configuration for the app.
#[derive(Debug, Clone)]
pub struct Config {
    /// Whether Termleaf's optional live two-set composer handles ASCII keys.
    /// When false, input is left entirely to the operating-system IME.
    pub live_composition: bool,
    /// Focus mode hides the status bar and other chrome.
    pub focus_mode: bool,
    /// Enable typing sound effects and their optional delete/return sounds.
    pub sound: bool,
    /// Play the separate backspace effect when typing sound is enabled.
    pub backspace_sound: bool,
    /// Play the carriage-return bell when typing sound is enabled.
    pub return_sound: bool,
    /// Selected printing-key sound (`classic`, `deep`, or `soft`).
    pub sound_profile: String,
    /// Show the big-pixel focus zone.
    pub big_font: bool,
    /// Pixel scale for the big-font renderer (`MIN_FONT..=MAX_FONT`).
    pub font_size: u16,
    /// Terminal rows between document baselines (`1` is compact).
    pub line_spacing: u16,
    /// Center writing inside a readable-width page on wide terminals.
    pub page_width: bool,
    /// Theme name (resolved by `ui::themes`).
    pub theme: String,
    /// Interface language (`en`, `ko`, or `ja`).
    pub language: String,
    /// Show the welcome/help overlay when Termleaf starts.
    pub show_welcome: bool,
    /// Autosave interval in seconds; `0` disables autosave.
    pub autosave_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            live_composition: false,
            focus_mode: false,
            sound: true,
            backspace_sound: true,
            return_sound: true,
            sound_profile: "classic".to_string(),
            big_font: true,
            font_size: 2,
            line_spacing: 2,
            page_width: false,
            theme: "paper".to_string(),
            language: "en".to_string(),
            show_welcome: true,
            autosave_secs: 30,
        }
    }
}

impl Config {
    /// Path to the config file (`$HOME/.config/termleaf/config`).
    pub fn path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("termleaf")
                .join("config"),
        )
    }

    /// Load config, falling back to defaults for anything missing or invalid.
    pub fn load() -> Self {
        let Some(path) = Config::path() else {
            return Config::default();
        };
        let Ok(text) = fs::read_to_string(path) else {
            return Config::default();
        };
        Config::from_text(&text)
    }

    fn from_text(text: &str) -> Self {
        let mut cfg = Config::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            match key {
                "live_composition" => {
                    cfg.live_composition = parse_bool(value, cfg.live_composition)
                }
                "focus_mode" => cfg.focus_mode = parse_bool(value, cfg.focus_mode),
                "sound" => cfg.sound = parse_bool(value, cfg.sound),
                "backspace_sound" => cfg.backspace_sound = parse_bool(value, cfg.backspace_sound),
                "return_sound" => cfg.return_sound = parse_bool(value, cfg.return_sound),
                "sound_profile" => {
                    cfg.sound_profile = match value {
                        "deep" => "deep",
                        "soft" => "soft",
                        _ => "classic",
                    }
                    .to_string();
                }
                "big_font" => cfg.big_font = parse_bool(value, cfg.big_font),
                "font_size" => {
                    if let Ok(n) = value.parse::<u16>() {
                        cfg.font_size = n.clamp(MIN_FONT, MAX_FONT);
                    }
                }
                "line_spacing" => {
                    if let Ok(n) = value.parse::<u16>() {
                        cfg.line_spacing = n.clamp(MIN_LINE_SPACING, MAX_LINE_SPACING);
                    }
                }
                "page_width" => cfg.page_width = parse_bool(value, cfg.page_width),
                "theme" => cfg.theme = value.to_string(),
                "language" => {
                    cfg.language = match value {
                        "ko" => "ko",
                        "ja" => "ja",
                        _ => "en",
                    }
                    .to_string();
                }
                "show_welcome" => cfg.show_welcome = parse_bool(value, cfg.show_welcome),
                "autosave_secs" => {
                    if let Ok(n) = value.parse::<u64>() {
                        cfg.autosave_secs = n;
                    }
                }
                _ => {}
            }
        }
        cfg
    }

    /// Persist the current config. Errors are ignored by the caller if it
    /// prefers (best-effort).
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Config::path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_text())
    }

    fn to_text(&self) -> String {
        format!(
            "# Termleaf configuration\n\
             live_composition = {}\n\
             focus_mode = {}\n\
             sound = {}\n\
             backspace_sound = {}\n\
             return_sound = {}\n\
             sound_profile = {}\n\
             big_font = {}\n\
             font_size = {}\n\
             line_spacing = {}\n\
             page_width = {}\n\
             theme = {}\n\
             language = {}\n\
             show_welcome = {}\n\
             autosave_secs = {}\n",
            self.live_composition,
            self.focus_mode,
            self.sound,
            self.backspace_sound,
            self.return_sound,
            self.sound_profile,
            self.big_font,
            self.font_size,
            self.line_spacing,
            self.page_width,
            self.theme,
            self.language,
            self.show_welcome,
            self.autosave_secs,
        )
    }

    pub fn font_inc(&mut self) {
        self.font_size = (self.font_size + 1).min(MAX_FONT);
    }

    pub fn font_dec(&mut self) {
        self.font_size = self.font_size.saturating_sub(1).max(MIN_FONT);
    }

    pub fn cycle_line_spacing(&mut self) {
        self.line_spacing = if self.line_spacing >= MAX_LINE_SPACING {
            MIN_LINE_SPACING
        } else {
            self.line_spacing + 1
        };
    }

    pub fn set_language(&mut self, language: &str) {
        self.language = match language {
            "ko" => "ko",
            "ja" => "ja",
            _ => "en",
        }
        .to_string();
    }

    pub fn cycle_sound_profile(&mut self) {
        self.sound_profile = match self.sound_profile.as_str() {
            "classic" => "deep",
            "deep" => "soft",
            _ => "classic",
        }
        .to_string();
    }

    pub fn previous_sound_profile(&mut self) {
        self.sound_profile = match self.sound_profile.as_str() {
            "classic" => "soft",
            "soft" => "deep",
            _ => "classic",
        }
        .to_string();
    }
}

fn parse_bool(value: &str, fallback: bool) -> bool {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "on" | "yes" => true,
        "false" | "0" | "off" | "no" => false,
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_parser_accepts_common_spellings_and_preserves_fallback() {
        for value in ["true", "TRUE", "1", "on", "yes"] {
            assert!(parse_bool(value, false));
        }
        for value in ["false", "FALSE", "0", "off", "no"] {
            assert!(!parse_bool(value, true));
        }
        assert!(parse_bool("invalid", true));
        assert!(!parse_bool("invalid", false));
    }

    #[test]
    fn font_size_stays_within_supported_bounds() {
        let mut cfg = Config {
            font_size: MAX_FONT,
            ..Config::default()
        };
        cfg.font_inc();
        assert_eq!(cfg.font_size, MAX_FONT);

        cfg.font_size = MIN_FONT;
        cfg.font_dec();
        assert_eq!(cfg.font_size, MIN_FONT);
    }

    #[test]
    fn line_spacing_defaults_to_relaxed_and_cycles_all_levels() {
        let mut cfg = Config::default();
        assert_eq!(cfg.line_spacing, 2);
        cfg.cycle_line_spacing();
        assert_eq!(cfg.line_spacing, 3);
        cfg.cycle_line_spacing();
        assert_eq!(cfg.line_spacing, 1);
        cfg.cycle_line_spacing();
        assert_eq!(cfg.line_spacing, 2);

        assert_eq!(Config::from_text("line_spacing = 0").line_spacing, 1);
        assert_eq!(Config::from_text("line_spacing = 9").line_spacing, 3);
    }

    #[test]
    fn interface_language_defaults_to_english_and_supports_installed_locales() {
        let mut cfg = Config::default();
        assert_eq!(cfg.language, "en");
        cfg.set_language("ko");
        assert_eq!(cfg.language, "ko");
        cfg.set_language("ja");
        assert_eq!(cfg.language, "ja");
        cfg.set_language("unsupported");
        assert_eq!(cfg.language, "en");
    }

    #[test]
    fn sound_profiles_cycle_and_backspace_sound_is_independent() {
        let mut cfg = Config::default();
        assert!(cfg.backspace_sound);
        assert!(cfg.return_sound);
        assert_eq!(cfg.sound_profile, "classic");
        cfg.cycle_sound_profile();
        assert_eq!(cfg.sound_profile, "deep");
        cfg.cycle_sound_profile();
        assert_eq!(cfg.sound_profile, "soft");
        cfg.cycle_sound_profile();
        assert_eq!(cfg.sound_profile, "classic");
    }

    #[test]
    fn persisted_text_restores_the_last_interface_and_sound_settings() {
        let cfg = Config {
            live_composition: true,
            focus_mode: true,
            sound: true,
            backspace_sound: false,
            return_sound: false,
            sound_profile: "soft".to_string(),
            big_font: false,
            font_size: 3,
            line_spacing: 3,
            page_width: true,
            theme: "xt".to_string(),
            language: "ko".to_string(),
            show_welcome: false,
            autosave_secs: 12,
        };

        let restored = Config::from_text(&cfg.to_text());
        assert!(restored.live_composition);
        assert!(restored.focus_mode);
        assert!(!restored.backspace_sound);
        assert!(!restored.return_sound);
        assert_eq!(restored.sound_profile, "soft");
        assert!(!restored.big_font);
        assert_eq!(restored.font_size, 3);
        assert_eq!(restored.line_spacing, 3);
        assert!(restored.page_width);
        assert_eq!(restored.theme, "xt");
        assert_eq!(restored.language, "ko");
        assert!(!restored.show_welcome);
        assert_eq!(restored.autosave_secs, 12);
    }

    #[test]
    fn legacy_hangul_toggle_does_not_override_the_new_os_ime_default() {
        let restored = Config::from_text("hangul_mode = true\n");
        assert!(!restored.live_composition);
    }
}
