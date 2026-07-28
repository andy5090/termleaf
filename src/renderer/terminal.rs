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
    // Follow a short slice of the active line, including live composition.
    // This gives the focus zone enough context to read like a phrase while
    // keeping the cursor-side text visible on narrow terminals.
    let chars = editor.focus_text(12);
    if chars.is_empty() {
        return Ok(());
    }

    let mut glyphs: Vec<Glyph> = chars.iter().map(|&c| glyph_for(c)).collect();

    // At least one-cell pixels are required. If the whole phrase does not fit
    // even then, discard its oldest characters until the cursor-side portion
    // does. Otherwise choose the largest configured scale that fits both axes.
    while glyphs.len() > 1 && unscaled_width(&glyphs) > layout.cols {
        glyphs.remove(0);
    }
    let glyph_h_max = glyphs.iter().map(|g| g.height as u16).max().unwrap_or(10);
    if layout.big_height * 2 < glyph_h_max || unscaled_width(&glyphs) > layout.cols {
        return Ok(());
    }
    let scale_x = layout.cols / unscaled_width(&glyphs).max(1);
    let scale_y = layout.big_height * 2 / glyph_h_max.max(1);
    let scale = cfg.font_size.max(1).min(scale_x).min(scale_y).max(1);
    let pixel_scale = scale;
    let gap = scale;

    let widths_sum: u16 = glyphs.iter().map(|g| g.width as u16 * pixel_scale).sum();
    let total = widths_sum + gap * (glyphs.len() as u16 - 1);
    let start_x = if total < layout.cols {
        (layout.cols - total) / 2
    } else {
        0
    };

    let block_h = (glyph_h_max * pixel_scale).div_ceil(2);
    let start_y = layout.big_top
        + if layout.big_height > block_h {
            (layout.big_height - block_h) / 2
        } else {
            0
        };

    let mut gx = start_x;
    for g in &glyphs {
        let glyph_y = start_y + ((glyph_h_max - g.height as u16) * pixel_scale).div_ceil(2);
        draw_glyph(out, g, gx, glyph_y, pixel_scale, theme)?;
        gx += g.width as u16 * pixel_scale + gap;
    }
    Ok(())
}

fn unscaled_width(glyphs: &[Glyph]) -> u16 {
    let glyph_width: u16 = glyphs.iter().map(|g| g.width as u16).sum();
    glyph_width.saturating_add(glyphs.len().saturating_sub(1) as u16)
}

fn draw_glyph(
    out: &mut Stdout,
    g: &Glyph,
    ox: u16,
    oy: u16,
    scale: u16,
    theme: &Theme,
) -> io::Result<()> {
    if g.rows.iter().all(|&row| row == 0) {
        return Ok(());
    }

    // A terminal cell is roughly twice as tall as it is wide. Unicode half
    // blocks preserve two vertical bitmap pixels per row, keeping Galmuri's
    // original proportions while leaving room for a short phrase.
    let expanded_height = g.height * scale as usize;
    for expanded_y in (0..expanded_height).step_by(2) {
        let top_y = expanded_y / scale as usize;
        let bottom_y = (expanded_y + 1) / scale as usize;
        let row = oy + expanded_y as u16 / 2;
        for x in 0..g.width {
            let top = g.lit(x, top_y);
            let bottom = expanded_y + 1 < expanded_height && g.lit(x, bottom_y);
            let (symbol, foreground, background) = match (top, bottom) {
                (true, true) => ('█', theme.pixel, theme.bg),
                (true, false) => ('▀', theme.pixel, theme.pixel_off),
                (false, true) => ('▄', theme.pixel, theme.pixel_off),
                (false, false) => ('█', theme.pixel_off, theme.bg),
            };
            let pixels: String = std::iter::repeat_n(symbol, scale as usize).collect();
            queue!(
                out,
                cursor::MoveTo(ox + x as u16 * scale, row),
                SetForegroundColor(foreground),
                SetBackgroundColor(background),
                Print(&pixels)
            )?;
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
    let sound = if cfg.sound { "♪" } else { "무음" };
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
        " {mode} │ {sound} │ {dirty}{name} │ {}단어 {}자 │ 조합 {}/3 ",
        editor.word_count(),
        editor.char_count(),
        stage
    );
    let hint = " F2 한/영  F3 집중  F4 큰글자  F5 소리  F7/8 크기  ^S 저장  ^Q 종료 ";

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
