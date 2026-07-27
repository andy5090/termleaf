//! Terminal setup (raw mode + alternate screen) and full-frame painting.

use std::io::{self, Stdout, Write};

use crossterm::style::{Print, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, queue, terminal};

use super::font::{glyph_for, Glyph};
use crate::config::Config;
use crate::editor::Editor;
use crate::ui::{char_width, Layout, Theme};

/// RAII guard: enters raw mode + the alternate screen on creation and restores
/// the terminal on drop (including on panic).
pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        crossterm::execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = io::stdout();
        let _ = crossterm::execute!(out, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

/// Paint one full frame.
pub fn draw(out: &mut Stdout, editor: &Editor, cfg: &Config, theme: &Theme) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    let layout = Layout::compute(cols, rows, cfg);

    queue!(out, cursor::Hide)?;

    // Background fill.
    let blank: String = " ".repeat(cols as usize);
    for y in 0..rows {
        queue!(
            out,
            cursor::MoveTo(0, y),
            SetForegroundColor(theme.fg),
            SetBackgroundColor(theme.bg),
            Print(&blank)
        )?;
    }

    if layout.big_enabled {
        draw_big(out, editor, cfg, theme, &layout)?;
    }

    let caret = draw_document(out, editor, theme, &layout)?;

    if let Some(sr) = layout.status_row {
        draw_status(out, editor, cfg, theme, cols, sr)?;
    }

    if let (Some(cx), Some(cy)) = caret {
        queue!(out, cursor::MoveTo(cx, cy), cursor::Show)?;
    }

    out.flush()
}

fn print_chars(
    out: &mut Stdout,
    x: u16,
    y: u16,
    chars: &[char],
    fg: crossterm::style::Color,
    bg: crossterm::style::Color,
) -> io::Result<u16> {
    let s: String = chars.iter().collect();
    queue!(
        out,
        cursor::MoveTo(x, y),
        SetForegroundColor(fg),
        SetBackgroundColor(bg),
        Print(&s)
    )?;
    let mut w = x;
    for &c in chars {
        w = w.saturating_add(char_width(c));
    }
    Ok(w)
}

fn draw_document(
    out: &mut Stdout,
    editor: &Editor,
    theme: &Theme,
    layout: &Layout,
) -> io::Result<(Option<u16>, Option<u16>)> {
    let lines = editor.lines();
    let cur = editor.cursor();
    let composing: Vec<char> = editor.composing().chars().collect();
    let left_margin: u16 = 4;
    let doc_height = layout.doc_height as usize;
    let top = if cur.row >= doc_height {
        cur.row - doc_height + 1
    } else {
        0
    };

    let mut caret = (None, None);
    for i in 0..doc_height {
        let idx = top + i;
        if idx >= lines.len() {
            break;
        }
        let y = layout.doc_top + i as u16;
        if idx == cur.row {
            let line = &lines[idx];
            let split = cur.col.min(line.len());
            let (l, r) = line.split_at(split);
            let mut x = print_chars(out, left_margin, y, l, theme.fg, theme.bg)?;
            x = print_chars(out, x, y, &composing, theme.accent, theme.bg)?;
            caret = (Some(x), Some(y));
            print_chars(out, x, y, r, theme.fg, theme.bg)?;
        } else {
            print_chars(out, left_margin, y, &lines[idx], theme.fg, theme.bg)?;
        }
    }
    Ok(caret)
}

fn draw_big(
    out: &mut Stdout,
    editor: &Editor,
    cfg: &Config,
    theme: &Theme,
    layout: &Layout,
) -> io::Result<()> {
    // Prefer the live composing cluster; otherwise show the last typed char.
    let seq = editor.composer().jamo_sequence();
    let chars: Vec<char> = if !seq.is_empty() {
        seq
    } else {
        let cur = editor.cursor();
        let lines = editor.lines();
        if cur.col > 0 && cur.row < lines.len() {
            vec![lines[cur.row][cur.col - 1]]
        } else {
            Vec::new()
        }
    };
    if chars.is_empty() {
        return Ok(());
    }

    let px_h = cfg.font_size.max(1);
    let px_w = cfg.font_size.max(1) * 2;
    let glyphs: Vec<Glyph> = chars.iter().map(|&c| glyph_for(c)).collect();
    let gap = px_w;

    let widths_sum: u16 = glyphs.iter().map(|g| g.width as u16 * px_w).sum();
    let total = widths_sum + gap * (glyphs.len() as u16 - 1);
    let start_x = if total < layout.cols {
        (layout.cols - total) / 2
    } else {
        0
    };

    let glyph_h_max = glyphs.iter().map(|g| g.height as u16).max().unwrap_or(8);
    let block_h = glyph_h_max * px_h;
    let start_y = layout.big_top
        + if layout.big_height > block_h {
            (layout.big_height - block_h) / 2
        } else {
            0
        };

    let mut gx = start_x;
    for g in &glyphs {
        draw_glyph(out, g, gx, start_y, px_w, px_h, theme)?;
        gx += g.width as u16 * px_w + gap;
    }
    Ok(())
}

fn draw_glyph(
    out: &mut Stdout,
    g: &Glyph,
    ox: u16,
    oy: u16,
    px_w: u16,
    px_h: u16,
    theme: &Theme,
) -> io::Result<()> {
    let block: String = "█".repeat(px_w as usize);
    for y in 0..g.height {
        for x in 0..g.width {
            // Lit pixels use the ink color; unlit pixels get a faint grid so
            // the letter reads as a real pixel display.
            let color = if g.lit(x, y) {
                theme.pixel
            } else {
                theme.pixel_off
            };
            for r in 0..px_h {
                queue!(
                    out,
                    cursor::MoveTo(ox + x as u16 * px_w, oy + y as u16 * px_h + r),
                    SetForegroundColor(color),
                    SetBackgroundColor(theme.bg),
                    Print(&block)
                )?;
            }
        }
    }
    Ok(())
}

fn draw_status(
    out: &mut Stdout,
    editor: &Editor,
    cfg: &Config,
    theme: &Theme,
    cols: u16,
    row: u16,
) -> io::Result<()> {
    let mode = if cfg.hangul_mode { "한" } else { "EN" };
    let dirty = if editor.doc.dirty { "*" } else { "" };
    let name = editor
        .doc
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".to_string());
    let stage = editor.composer().stage();
    let left = format!(
        " {mode} │ {dirty}{name} │ {}단어 {}자 │ 조합 {}/3 ",
        editor.word_count(),
        editor.char_count(),
        stage
    );
    let hint = " F2 한/영  F3 집중  F4 큰글자  F7/8 크기  ^S 저장  ^Q 종료 ";

    let left_w: u16 = left.chars().map(char_width).sum();
    let hint_w: u16 = hint.chars().map(char_width).sum();

    let bar: String = " ".repeat(cols as usize);
    queue!(
        out,
        cursor::MoveTo(0, row),
        SetForegroundColor(theme.fg),
        SetBackgroundColor(theme.dim),
        Print(&bar),
        cursor::MoveTo(0, row),
        Print(&left)
    )?;
    if left_w + hint_w <= cols {
        queue!(out, cursor::MoveTo(cols - hint_w, row), Print(hint))?;
    }
    Ok(())
}
