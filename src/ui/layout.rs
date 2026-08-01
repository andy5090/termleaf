//! Computes how the terminal is divided into regions.

use crate::config::Config;

/// Resolved screen regions for the current frame. All values are in terminal
/// cells; rows are 0-indexed from the top.
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub cols: u16,
    /// Whether the big-font focus zone is shown this frame.
    pub big_enabled: bool,
    /// Top row and height of the big-font zone.
    pub big_top: u16,
    pub big_height: u16,
    /// Top row and height of the document area.
    pub doc_top: u16,
    pub doc_height: u16,
    /// Row of the status bar, if visible.
    pub status_row: Option<u16>,
    /// Row of the persistent shortcut guide, if visible.
    pub shortcut_row: Option<u16>,
}

impl Layout {
    pub fn compute(cols: u16, rows: u16, cfg: &Config) -> Layout {
        let status_visible = !cfg.focus_mode;
        let status_reserve: u16 = if status_visible {
            if rows >= 3 {
                2
            } else {
                1
            }
        } else {
            0
        };
        let avail = rows.saturating_sub(status_reserve).max(1);

        let mut big_enabled = false;
        let mut big_height = 0u16;
        if cfg.big_font {
            let px_h = cfg.font_size;
            // Galmuri's 10 bitmap rows are packed two per terminal row.
            let desired = 5 * px_h + 2;
            // Reserve only the minimum useful document height. Taller
            // terminals can show all five levels proportionally; standard
            // terminals use adaptive horizontal scaling for levels 4–5.
            let cap = avail.saturating_sub(3);
            let h = desired.min(cap);
            // Only show it if it fits without starving the document.
            if h >= 5 && avail.saturating_sub(h) >= 3 {
                big_enabled = true;
                big_height = h;
            }
        }

        // Without the big-font zone, leave one quiet row above the document
        // when doing so still preserves at least two editable rows.
        let top_padding = u16::from(!cfg.big_font && avail >= 3);
        let doc_top = big_height + top_padding;
        let doc_height = avail
            .saturating_sub(big_height)
            .saturating_sub(top_padding)
            .max(1);
        let status_row = if status_visible {
            Some(rows.saturating_sub(status_reserve))
        } else {
            None
        };
        let shortcut_row = if status_reserve == 2 {
            Some(rows - 1)
        } else {
            None
        };

        Layout {
            cols,
            big_enabled,
            big_top: 0,
            big_height,
            doc_top,
            doc_height,
            status_row,
            shortcut_row,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::{MAX_FONT, MIN_FONT};

    #[test]
    fn normal_layout_reserves_status_and_document_space() {
        let cfg = Config::default();
        let layout = Layout::compute(80, 24, &cfg);

        assert_eq!(layout.cols, 80);
        assert_eq!(layout.status_row, Some(22));
        assert_eq!(layout.shortcut_row, Some(23));
        assert!(layout.big_enabled);
        assert!(layout.doc_height >= 3);
        assert_eq!(layout.big_height + layout.doc_height, 22);
    }

    #[test]
    fn compact_or_focus_layout_degrades_safely() {
        let mut cfg = Config::default();
        let compact = Layout::compute(20, 4, &cfg);
        assert!(!compact.big_enabled);
        assert_eq!(compact.doc_height, 2);
        assert_eq!(compact.shortcut_row, Some(3));

        cfg.focus_mode = true;
        let focused = Layout::compute(20, 4, &cfg);
        assert_eq!(focused.status_row, None);
        assert_eq!(focused.shortcut_row, None);
        assert_eq!(focused.doc_height, 4);
    }

    #[test]
    fn disabled_big_font_adds_top_padding_without_starving_compact_layouts() {
        let mut cfg = Config {
            big_font: false,
            ..Config::default()
        };

        let normal = Layout::compute(80, 24, &cfg);
        assert_eq!(normal.doc_top, 1);
        assert_eq!(normal.doc_height, 21);

        cfg.focus_mode = true;
        let focused = Layout::compute(80, 24, &cfg);
        assert_eq!(focused.doc_top, 1);
        assert_eq!(focused.doc_height, 23);

        let compact = Layout::compute(20, 4, &cfg);
        assert_eq!(compact.doc_top, 1);
        assert_eq!(compact.doc_height, 3);

        cfg.focus_mode = false;
        let constrained = Layout::compute(20, 4, &cfg);
        assert_eq!(constrained.doc_top, 0);
        assert_eq!(constrained.doc_height, 2);
    }

    #[test]
    fn big_zone_uses_available_height_without_starving_the_document() {
        let mut cfg = Config::default();
        let mut heights = Vec::new();
        for size in MIN_FONT..=MAX_FONT {
            cfg.font_size = size;
            heights.push(Layout::compute(80, 24, &cfg).big_height);
        }
        assert_eq!(heights, [7, 12, 17, 19, 19]);
        assert!(heights.windows(2).all(|pair| pair[0] <= pair[1]));
    }
}
