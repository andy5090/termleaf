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
                    r: 220,
                    g: 220,
                    b: 230,
                },
                bg: Color::Rgb {
                    r: 18,
                    g: 18,
                    b: 24,
                },
                dim: Color::Rgb {
                    r: 110,
                    g: 110,
                    b: 130,
                },
                accent: Color::Rgb {
                    r: 120,
                    g: 200,
                    b: 255,
                },
                pixel: Color::Rgb {
                    r: 235,
                    g: 235,
                    b: 245,
                },
                pixel_off: Color::Rgb {
                    r: 40,
                    g: 40,
                    b: 52,
                },
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
            "night" => "amber",
            _ => "paper",
        }
    }
}
