//! Color themes.

use crossterm::style::Color;

/// A resolved color palette.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Document text.
    pub fg: Color,
    /// Screen background.
    pub bg: Color,
    /// Dimmed chrome (status bar text, hints).
    pub dim: Color,
    /// Accent used for the active composition / cursor.
    pub accent: Color,
    /// Color of "lit" big-font pixels.
    pub pixel: Color,
    /// Color of "unlit" big-font pixels (the faint grid).
    pub pixel_off: Color,
}

impl Theme {
    /// Resolve a theme by name, falling back to `paper`.
    pub fn by_name(name: &str) -> Theme {
        match name {
            "night" => Theme {
                fg: Color::Rgb {
                    r: 242,
                    g: 242,
                    b: 242,
                },
                bg: Color::Rgb { r: 0, g: 0, b: 0 },
                dim: Color::Rgb {
                    r: 92,
                    g: 92,
                    b: 92,
                },
                accent: Color::Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                pixel: Color::Rgb {
                    r: 242,
                    g: 242,
                    b: 242,
                },
                pixel_off: Color::Rgb {
                    r: 28,
                    g: 28,
                    b: 28,
                },
            },
            "xt" => Theme {
                fg: Color::Rgb {
                    r: 76,
                    g: 255,
                    b: 112,
                },
                bg: Color::Rgb { r: 0, g: 5, b: 1 },
                dim: Color::Rgb {
                    r: 12,
                    g: 74,
                    b: 27,
                },
                accent: Color::Rgb {
                    r: 178,
                    g: 255,
                    b: 188,
                },
                pixel: Color::Rgb {
                    r: 76,
                    g: 255,
                    b: 112,
                },
                pixel_off: Color::Rgb { r: 2, g: 28, b: 8 },
            },
            "amber" => Theme {
                fg: Color::Rgb {
                    r: 255,
                    g: 176,
                    b: 0,
                },
                bg: Color::Rgb { r: 20, g: 12, b: 0 },
                dim: Color::Rgb {
                    r: 150,
                    g: 100,
                    b: 0,
                },
                accent: Color::Rgb {
                    r: 255,
                    g: 220,
                    b: 120,
                },
                pixel: Color::Rgb {
                    r: 255,
                    g: 176,
                    b: 0,
                },
                pixel_off: Color::Rgb { r: 55, g: 34, b: 0 },
            },
            // "paper": warm, light, distraction-free
            _ => Theme {
                fg: Color::Rgb {
                    r: 40,
                    g: 38,
                    b: 34,
                },
                bg: Color::Rgb {
                    r: 244,
                    g: 240,
                    b: 232,
                },
                dim: Color::Rgb {
                    r: 150,
                    g: 145,
                    b: 135,
                },
                accent: Color::Rgb {
                    r: 180,
                    g: 70,
                    b: 50,
                },
                pixel: Color::Rgb {
                    r: 40,
                    g: 38,
                    b: 34,
                },
                pixel_off: Color::Rgb {
                    r: 224,
                    g: 219,
                    b: 209,
                },
            },
        }
    }

    /// Cycle to the next theme name.
    pub fn next(name: &str) -> &'static str {
        match name {
            "paper" => "night",
            "night" => "xt",
            "xt" => "amber",
            _ => "paper",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_all_themes_and_falls_back_to_paper() {
        assert_eq!(Theme::next("paper"), "night");
        assert_eq!(Theme::next("night"), "xt");
        assert_eq!(Theme::next("xt"), "amber");
        assert_eq!(Theme::next("amber"), "paper");
        assert_eq!(Theme::next("unknown"), "paper");
        assert_eq!(Theme::by_name("unknown").fg, Theme::by_name("paper").fg);
        assert_ne!(Theme::by_name("xt").fg, Theme::by_name("night").fg);
    }
}
