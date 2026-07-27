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
}

impl Layout {
    pub fn compute(cols: u16, rows: u16, cfg: &Config) -> Layout {
        let status_visible = !cfg.focus_mode;
        let status_reserve: u16 = if status_visible { 1 } else { 0 };
        let avail = rows.saturating_sub(status_reserve).max(1);

        let mut big_enabled = false;
        let mut big_height = 0u16;
        if cfg.big_font {
            let px_h = cfg.font_size;
            // 8 rows of pixels (jamo cell) plus a little padding.
            let desired = 8 * px_h + 2;
            let cap = avail / 2;
            let h = desired.min(cap);
            // Only show it if it fits without starving the document.
            if h >= 6 && avail.saturating_sub(h) >= 3 {
                big_enabled = true;
                big_height = h;
            }
        }

        let doc_top = big_height;
        let doc_height = avail.saturating_sub(big_height).max(1);
        let status_row = if status_visible { Some(rows - 1) } else { None };

        Layout {
            cols,
            big_enabled,
            big_top: 0,
            big_height,
            doc_top,
            doc_height,
            status_row,
        }
    }
}
