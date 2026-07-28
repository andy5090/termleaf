//! User-configurable settings, persisted as a tiny `key = value` file.
//!
//! We intentionally avoid a serialization crate to keep the dependency tree
//! to just `crossterm`.

use std::fs;
use std::path::PathBuf;

pub const MIN_FONT: u16 = 1;
pub const MAX_FONT: u16 = 6;

/// Runtime configuration for the app.
#[derive(Debug, Clone)]
pub struct Config {
    /// Whether letter keys compose Hangul (`true`) or type Latin (`false`).
    pub hangul_mode: bool,
    /// Focus mode hides the status bar and other chrome.
    pub focus_mode: bool,
    /// Play a soft "clack" (terminal bell) on each keystroke.
    pub sound: bool,
    /// Show the big-pixel focus zone.
    pub big_font: bool,
    /// Pixel scale for the big-font renderer (`MIN_FONT..=MAX_FONT`).
    pub font_size: u16,
    /// Theme name (resolved by `ui::themes`).
    pub theme: String,
    /// Autosave interval in seconds; `0` disables autosave.
    pub autosave_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hangul_mode: false,
            focus_mode: false,
            sound: true,
            big_font: true,
            font_size: 2,
            theme: "paper".to_string(),
            autosave_secs: 30,
        }
    }
}

impl Config {
    /// Path to the config file (`$HOME/.config/tadak/config`).
    pub fn path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("tadak")
                .join("config"),
        )
    }

    /// Load config, falling back to defaults for anything missing or invalid.
    pub fn load() -> Self {
        let mut cfg = Config::default();
        let Some(path) = Config::path() else {
            return cfg;
        };
        let Ok(text) = fs::read_to_string(path) else {
            return cfg;
        };
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
                "hangul_mode" => cfg.hangul_mode = parse_bool(value, cfg.hangul_mode),
                "focus_mode" => cfg.focus_mode = parse_bool(value, cfg.focus_mode),
                "sound" => cfg.sound = parse_bool(value, cfg.sound),
                "big_font" => cfg.big_font = parse_bool(value, cfg.big_font),
                "font_size" => {
                    if let Ok(n) = value.parse::<u16>() {
                        cfg.font_size = n.clamp(MIN_FONT, MAX_FONT);
                    }
                }
                "theme" => cfg.theme = value.to_string(),
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
        let body = format!(
            "# Tadak configuration\n\
             hangul_mode = {}\n\
             focus_mode = {}\n\
             sound = {}\n\
             big_font = {}\n\
             font_size = {}\n\
             theme = {}\n\
             autosave_secs = {}\n",
            self.hangul_mode,
            self.focus_mode,
            self.sound,
            self.big_font,
            self.font_size,
            self.theme,
            self.autosave_secs,
        );
        fs::write(path, body)
    }

    pub fn font_inc(&mut self) {
        self.font_size = (self.font_size + 1).min(MAX_FONT);
    }

    pub fn font_dec(&mut self) {
        self.font_size = self.font_size.saturating_sub(1).max(MIN_FONT);
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
}
