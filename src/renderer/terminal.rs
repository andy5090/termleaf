//! Terminal setup (raw mode + alternate screen) and full-frame painting.

use std::io::{self, Stdout};

use crossterm::style::{Color, Print, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, queue, terminal, SynchronizedUpdate};

use super::font::{glyph_for, Glyph};
use crate::config::settings::{MAX_FONT, MAX_LINE_SPACING};
use crate::config::Config;
use crate::editor::Editor;
use crate::language::{Language, LanguageRegistry};
use crate::ui::{
    char_width, FilePrompt, FilePromptError, FilePromptKind, HelpOverlay, LanguageSettings, Layout,
    SoundSettings, Theme,
};

const PAGE_CONTENT_WIDTH: u16 = 80;

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
pub struct View<'a> {
    pub editor: &'a Editor,
    pub cfg: &'a Config,
    pub theme: &'a Theme,
    pub prompt: Option<&'a FilePrompt>,
    pub help: Option<&'a HelpOverlay>,
    pub sound_settings: Option<&'a SoundSettings>,
    pub language_settings: Option<&'a LanguageSettings>,
    pub languages: &'a LanguageRegistry,
}

pub fn draw(out: &mut Stdout, view: View<'_>) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    // File prompts must remain visible even when focus mode normally hides
    // chrome, so temporarily reserve the two footer rows for them.
    let layout = if view.prompt.is_some() && view.cfg.focus_mode {
        let mut prompt_cfg = view.cfg.clone();
        prompt_cfg.focus_mode = false;
        Layout::compute(cols, rows, &prompt_cfg)
    } else {
        Layout::compute(cols, rows, view.cfg)
    };

    // Terminals supporting synchronized updates keep the previous frame
    // visible until this complete frame is ready, preventing the background
    // clear and large-glyph painting from appearing as separate flashes.
    let frame = Frame {
        editor: view.editor,
        cfg: view.cfg,
        theme: view.theme,
        prompt: view.prompt,
        help: view.help,
        sound_settings: view.sound_settings,
        language_settings: view.language_settings,
        languages: view.languages,
        layout: &layout,
        rows,
    };
    out.sync_update(|out| draw_frame(out, &frame))??;
    Ok(())
}

struct Frame<'a> {
    editor: &'a Editor,
    cfg: &'a Config,
    theme: &'a Theme,
    prompt: Option<&'a FilePrompt>,
    help: Option<&'a HelpOverlay>,
    sound_settings: Option<&'a SoundSettings>,
    language_settings: Option<&'a LanguageSettings>,
    languages: &'a LanguageRegistry,
    layout: &'a Layout,
    rows: u16,
}

fn draw_frame(out: &mut Stdout, frame: &Frame<'_>) -> io::Result<()> {
    queue!(out, cursor::Hide)?;

    // Background fill.
    let blank: String = " ".repeat(frame.layout.cols as usize);
    for y in 0..frame.rows {
        queue!(
            out,
            cursor::MoveTo(0, y),
            SetForegroundColor(frame.theme.fg),
            SetBackgroundColor(frame.theme.bg),
            Print(&blank)
        )?;
    }

    if frame.layout.big_enabled {
        draw_big(
            out,
            frame.editor,
            frame.cfg,
            frame.theme,
            frame.layout,
            frame.languages,
        )?;
    }

    let caret = draw_document(out, frame.editor, frame.cfg, frame.theme, frame.layout)?;

    let prompt_caret = if let (Some(sr), Some(prompt)) = (frame.layout.status_row, frame.prompt) {
        Some(draw_file_prompt(
            out,
            prompt,
            frame.cfg,
            frame.theme,
            frame.layout.cols,
            sr,
        )?)
    } else {
        if let Some(sr) = frame.layout.status_row {
            draw_status(
                out,
                frame.editor,
                frame.cfg,
                frame.theme,
                frame.layout.cols,
                sr,
            )?;
        }
        None
    };

    if let Some(row) = frame.layout.shortcut_row {
        draw_shortcuts(
            out,
            frame.prompt,
            frame.cfg,
            frame.theme,
            frame.layout.cols,
            row,
        )?;
    }

    if frame.help.is_none() && frame.sound_settings.is_none() && frame.language_settings.is_none() {
        if let (Some(prompt), Some(status_row)) = (frame.prompt, frame.layout.status_row) {
            draw_file_candidates(
                out,
                prompt,
                frame.cfg,
                frame.theme,
                frame.layout.cols,
                status_row,
            )?;
        }
    }

    if let Some(help) = frame.help {
        draw_help(
            out,
            frame.cfg,
            help,
            frame.theme,
            frame.layout.cols,
            frame.rows,
        )?;
    } else if let Some(settings) = frame.sound_settings {
        draw_sound_settings(
            out,
            frame.cfg,
            settings,
            frame.theme,
            frame.layout.cols,
            frame.rows,
        )?;
    } else if let Some(settings) = frame.language_settings {
        draw_language_settings(
            out,
            frame.cfg,
            settings,
            frame.languages,
            frame.theme,
            frame.layout.cols,
            frame.rows,
        )?;
    } else if let Some((cx, cy)) = prompt_caret {
        queue!(out, cursor::MoveTo(cx, cy), cursor::Show)?;
    } else if let (Some(cx), Some(cy)) = caret {
        queue!(out, cursor::MoveTo(cx, cy), cursor::Show)?;
    }

    Ok(())
}

fn draw_language_settings(
    out: &mut Stdout,
    cfg: &Config,
    settings: &LanguageSettings,
    languages: &LanguageRegistry,
    theme: &Theme,
    cols: u16,
    rows: u16,
) -> io::Result<()> {
    let width = cols.saturating_sub(2).min(62);
    let height = 10;
    if width < 34 || rows < height + 2 {
        return Ok(());
    }
    let locale = Language::from_code(&cfg.language).unwrap_or(Language::English);
    let title = match locale {
        Language::English => "Languages",
        Language::Korean => "언어",
        Language::Japanese => "言語",
    };
    let controls = match locale {
        Language::English => "↑/↓ Select · Enter Install/Use · Delete Remove · Esc Close",
        Language::Korean => "↑/↓ 선택 · Enter 설치/사용 · Delete 제거 · Esc 닫기",
        Language::Japanese => "↑/↓ 選択 · Enter インストール/使用 · Delete 削除 · Esc 閉じる",
    };
    let left = (cols - width) / 2;
    let top = (rows - height) / 2;
    let horizontal = "─".repeat(width.saturating_sub(2) as usize);
    let inner_blank = " ".repeat(width.saturating_sub(2) as usize);
    queue!(
        out,
        SetForegroundColor(theme.accent),
        SetBackgroundColor(theme.bg),
        cursor::MoveTo(left, top),
        Print(format!("┌{horizontal}┐"))
    )?;
    for offset in 1..height - 1 {
        queue!(
            out,
            cursor::MoveTo(left, top + offset),
            Print("│"),
            SetForegroundColor(theme.fg),
            Print(&inner_blank),
            SetForegroundColor(theme.accent),
            Print("│")
        )?;
    }
    queue!(
        out,
        cursor::MoveTo(left, top + height - 1),
        Print(format!("└{horizontal}┘"))
    )?;
    draw_help_line(out, left, top + 1, width, title, theme.accent, theme.bg)?;

    for (index, language) in Language::ALL.iter().copied().enumerate() {
        let marker = if settings.selected == index {
            "▶"
        } else {
            " "
        };
        let active = cfg.language == language.code();
        let state = match (
            locale,
            language.is_builtin(),
            languages.is_installed(language),
            active,
        ) {
            (Language::Korean, _, _, true) => "사용 중",
            (Language::Japanese, _, _, true) => "使用中",
            (_, _, _, true) => "Active",
            (Language::Korean, true, _, _) => "기본 제공",
            (Language::Japanese, true, _, _) => "内蔵",
            (_, true, _, _) => "Built in",
            (Language::Korean, _, true, _) => "설치됨",
            (Language::Japanese, _, true, _) => "インストール済み",
            (_, _, true, _) => "Installed",
            (Language::Korean, _, false, _) => "설치 가능",
            (Language::Japanese, _, false, _) => "利用可能",
            (_, _, false, _) => "Available",
        };
        let line = format!("{marker}  {:<12}  {state}", language.native_name());
        draw_help_line(
            out,
            left,
            top + 3 + index as u16,
            width,
            &line,
            if settings.selected == index {
                theme.accent
            } else {
                theme.fg
            },
            theme.bg,
        )?;
    }
    if let Some(status) = settings.status.as_deref() {
        draw_help_line(out, left, top + 6, width, status, theme.dim, theme.bg)?;
    }
    draw_help_line(
        out,
        left,
        top + height - 2,
        width,
        controls,
        theme.dim,
        theme.bg,
    )
}

fn draw_sound_settings(
    out: &mut Stdout,
    cfg: &Config,
    settings: &SoundSettings,
    theme: &Theme,
    cols: u16,
    rows: u16,
) -> io::Result<()> {
    let width = cols.saturating_sub(2).min(64);
    let height = 9;
    if width < 30 || rows < height + 2 {
        return Ok(());
    }

    let locale = Language::from_code(&cfg.language).unwrap_or(Language::English);
    let title = match locale {
        Language::English => "Sound Settings",
        Language::Korean => "소리 설정",
        Language::Japanese => "サウンド設定",
    };
    let enabled = |value| match (locale, value) {
        (Language::Korean, true) => "켬",
        (Language::Korean, false) => "끔",
        (Language::Japanese, true) => "オン",
        (Language::Japanese, false) => "オフ",
        (_, true) => "on",
        (_, false) => "off",
    };
    let lines = match locale {
        Language::Korean => [
            format!("[{}] 타이핑 소리", enabled(cfg.sound)),
            format!("[{}] 삭제 소리", enabled(cfg.backspace_sound)),
            format!("[{}] 캐리지 리턴 소리", enabled(cfg.return_sound)),
            format!("[{}] 타자기 종류 (F11)", cfg.sound_profile),
        ],
        Language::Japanese => [
            format!("[{}] タイピング音", enabled(cfg.sound)),
            format!("[{}] 削除音", enabled(cfg.backspace_sound)),
            format!("[{}] キャリッジリターン音", enabled(cfg.return_sound)),
            format!("[{}] キー音スタイル (F11)", cfg.sound_profile),
        ],
        Language::English => [
            format!("[{}] Typing sound", enabled(cfg.sound)),
            format!("[{}] Delete sound", enabled(cfg.backspace_sound)),
            format!("[{}] Carriage-return sound", enabled(cfg.return_sound)),
            format!("[{}] Key style (F11)", cfg.sound_profile),
        ],
    };
    let controls = match locale {
        Language::Korean => "↑/↓ 선택 · Space 전환 · ←/→ 변경 · Enter/Esc 닫기",
        Language::Japanese => "↑/↓ 選択 · Space 切替 · ←/→ 変更 · Enter/Esc 閉じる",
        Language::English => "↑/↓ Select · Space Toggle · ←/→ Change · Enter/Esc Close",
    };
    let left = (cols - width) / 2;
    let top = (rows - height) / 2;
    let horizontal = "─".repeat(width.saturating_sub(2) as usize);
    let inner_blank = " ".repeat(width.saturating_sub(2) as usize);

    queue!(
        out,
        SetForegroundColor(theme.accent),
        SetBackgroundColor(theme.bg),
        cursor::MoveTo(left, top),
        Print(format!("┌{horizontal}┐"))
    )?;
    for offset in 1..height - 1 {
        queue!(
            out,
            cursor::MoveTo(left, top + offset),
            Print("│"),
            SetForegroundColor(theme.fg),
            Print(&inner_blank),
            SetForegroundColor(theme.accent),
            Print("│")
        )?;
    }
    queue!(
        out,
        cursor::MoveTo(left, top + height - 1),
        Print(format!("└{horizontal}┘"))
    )?;

    draw_help_line(out, left, top + 1, width, title, theme.accent, theme.bg)?;
    for (index, line) in lines.iter().enumerate() {
        let marker = if settings.selected == index {
            "▶ "
        } else {
            "  "
        };
        draw_help_line(
            out,
            left,
            top + 2 + index as u16,
            width,
            &format!("{marker}{line}"),
            if settings.selected == index {
                theme.accent
            } else {
                theme.fg
            },
            theme.bg,
        )?;
    }
    draw_help_line(
        out,
        left,
        top + height - 2,
        width,
        controls,
        theme.dim,
        theme.bg,
    )
}

fn draw_file_candidates(
    out: &mut Stdout,
    prompt: &FilePrompt,
    cfg: &Config,
    theme: &Theme,
    cols: u16,
    status_row: u16,
) -> io::Result<()> {
    if prompt.kind != FilePromptKind::Open || prompt.candidates.is_empty() {
        return Ok(());
    }
    let max_items = status_row.saturating_sub(1).min(6) as usize;
    if max_items == 0 {
        return Ok(());
    }
    let item_count = prompt.candidates.len().min(max_items);
    let max_start = prompt.candidates.len().saturating_sub(item_count);
    let start = prompt
        .selected
        .saturating_sub(item_count.saturating_sub(1))
        .min(max_start);
    let width = cols.min(76);
    let left = (cols - width) / 2;
    let top = status_row.saturating_sub(item_count as u16 + 1);
    let blank = " ".repeat(width as usize);
    let title = match cfg.language.as_str() {
        "ko" => " 문서 선택  ↑/↓ 이동 · Tab 자동완성 · Enter 열기 ",
        "ja" => " 文書を選択  ↑/↓ 移動 · Tab 補完 · Enter 開く ",
        _ => " Choose a document  ↑/↓ Move · Tab Complete · Enter Open ",
    };
    queue!(
        out,
        cursor::MoveTo(left, top),
        SetForegroundColor(theme.bg),
        SetBackgroundColor(theme.fg),
        Print(&blank),
        cursor::MoveTo(left, top),
        Print(clipped(title, width))
    )?;

    for (offset, candidate) in prompt.candidates[start..start + item_count]
        .iter()
        .enumerate()
    {
        let index = start + offset;
        let selected = index == prompt.selected;
        let marker = if selected { ">" } else { " " };
        let kind = if candidate.is_dir { "[dir]" } else { "     " };
        let name = candidate
            .path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| candidate.path.to_string_lossy());
        let text = format!("{marker} {kind} {name}");
        let (fg, bg) = if selected {
            (theme.bg, theme.accent)
        } else {
            (theme.fg, theme.dim)
        };
        let row = top + 1 + offset as u16;
        queue!(
            out,
            cursor::MoveTo(left, row),
            SetForegroundColor(fg),
            SetBackgroundColor(bg),
            Print(&blank),
            cursor::MoveTo(left, row),
            Print(clipped(&text, width))
        )?;
    }
    Ok(())
}

fn draw_help(
    out: &mut Stdout,
    cfg: &Config,
    help: &HelpOverlay,
    theme: &Theme,
    cols: u16,
    rows: u16,
) -> io::Result<()> {
    let width = cols.saturating_sub(2).min(76);
    let max_height = rows.saturating_sub(2);
    if width < 12 || max_height < 6 {
        return Ok(());
    }

    let locale = Language::from_code(&cfg.language).unwrap_or(Language::English);
    let title = match (help.welcome, locale) {
        (true, Language::English) => "Welcome to Termleaf",
        (false, Language::English) => "Termleaf Help",
        (true, Language::Korean) => "Termleaf에 오신 것을 환영합니다",
        (false, Language::Korean) => "Termleaf 도움말",
        (true, Language::Japanese) => "Termleafへようこそ",
        (false, Language::Japanese) => "Termleafヘルプ",
    };
    let lines = match locale {
        Language::Korean => vec![
            "Termleaf는 두 가지 한글 입력 방식을 제공합니다.",
            "IME:OS(기본) — Linux 한/영 키 사용, 완성된 음절만 표시",
            "F2 직접 한글 — OS 입력을 영문으로 두면 ㅎ → 하 → 한 표시",
            "",
            "파일: Ctrl+O 열기 · Ctrl+S 저장 · F12 다른 이름 (.md 기본)",
            "화면: F3 집중 · F4 큰글자 · F6 테마 · F7/F8 크기 1–5",
            "보기: F5 종이 폭 · Shift+F5 줄간격 1–3",
            "보조: macOS Option+P/L · Windows/Linux Alt+P/L",
            "소리: F10 설정(타이핑/삭제/엔터) · F11 타자음",
            "기타: Backspace/Delete 삭제 · F9 언어 · F1 도움말 · Ctrl+Q 종료",
        ],
        Language::Japanese => vec![
            "日本語入力にはOSのIMEをそのまま使用します。",
            "IME:OS（標準）— かな・漢字変換はmacOS/Linux側で確定",
            "韓国語パック導入時のみF2でLive Koreanを利用可能",
            "",
            "ファイル: Ctrl+O 開く · Ctrl+S 保存 · F12 名前を付けて保存",
            "表示: F3 集中 · F4 拡大文字 · F6 テーマ · F7/F8 サイズ",
            "読みやすさ: F5 ページ幅 · Shift+F5 行間 1–3",
            "代替: macOS Option+P/L · Windows/Linux Alt+P/L",
            "サウンド: F10 設定 · F11 キー音スタイル",
            "その他: F9 言語 · F1 ヘルプ · Ctrl+Q 終了",
        ],
        Language::English => vec![
            "Termleaf supports two Korean input paths.",
            "IME:OS (default) — use Linux input switching; final syllables only",
            "F2 Live Korean — keep OS input English to see ㅎ → 하 → 한",
            "",
            "Files: Ctrl+O Open · Ctrl+S Save · F12 Save as (.md default)",
            "View: F3 Focus · F4 Big text · F6 Theme · F7/F8 Size 1–5",
            "Reading: F5 Page width · Shift+F5 Line spacing 1–3",
            "Alternates: macOS Option+P/L · Windows/Linux Alt+P/L",
            "Sound: F10 Settings (typing/delete/return) · F11 Key style",
            "Other: Backspace/Delete · F9 Languages · F1 Help · Ctrl+Q Quit",
        ],
    };

    // Keep the checkbox and close instructions visible on short terminals.
    let line_capacity = max_height.saturating_sub(5) as usize;
    let visible_lines = &lines[..lines.len().min(line_capacity)];
    let height = visible_lines.len() as u16 + 5;
    let left = (cols - width) / 2;
    let top = (rows - height) / 2;
    let horizontal = "─".repeat(width.saturating_sub(2) as usize);
    let inner_blank = " ".repeat(width.saturating_sub(2) as usize);

    queue!(
        out,
        SetForegroundColor(theme.accent),
        SetBackgroundColor(theme.bg),
        cursor::MoveTo(left, top),
        Print(format!("┌{horizontal}┐"))
    )?;
    for offset in 1..height - 1 {
        queue!(
            out,
            cursor::MoveTo(left, top + offset),
            Print("│"),
            SetForegroundColor(theme.fg),
            Print(&inner_blank),
            SetForegroundColor(theme.accent),
            Print("│")
        )?;
    }
    queue!(
        out,
        cursor::MoveTo(left, top + height - 1),
        Print(format!("└{horizontal}┘"))
    )?;

    draw_help_line(out, left, top + 1, width, title, theme.accent, theme.bg)?;
    for (index, line) in visible_lines.iter().enumerate() {
        draw_help_line(
            out,
            left,
            top + 2 + index as u16,
            width,
            line,
            theme.fg,
            theme.bg,
        )?;
    }

    let checked = if help.hide_on_startup { "x" } else { " " };
    let checkbox = match locale {
        Language::Korean => format!("[{checked}] 시작할 때 이 안내를 표시하지 않음"),
        Language::Japanese => format!("[{checked}] 起動時にこの案内を表示しない"),
        Language::English => format!("[{checked}] Don't show this welcome on startup"),
    };
    let controls = match locale {
        Language::Korean => "Space 선택 · Enter/Esc 닫기 · F9 언어",
        Language::Japanese => "Space 切替 · Enter/Esc 閉じる · F9 言語",
        Language::English => "Space Toggle · Enter/Esc Close · F9 Languages",
    };
    draw_help_line(
        out,
        left,
        top + height - 3,
        width,
        &checkbox,
        theme.accent,
        theme.bg,
    )?;
    draw_help_line(
        out,
        left,
        top + height - 2,
        width,
        controls,
        theme.dim,
        theme.bg,
    )
}

fn draw_help_line(
    out: &mut Stdout,
    left: u16,
    row: u16,
    width: u16,
    text: &str,
    fg: crossterm::style::Color,
    bg: crossterm::style::Color,
) -> io::Result<()> {
    queue!(
        out,
        cursor::MoveTo(left + 2, row),
        SetForegroundColor(fg),
        SetBackgroundColor(bg),
        Print(clipped(text, width.saturating_sub(4)))
    )
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
    cfg: &Config,
    theme: &Theme,
    layout: &Layout,
) -> io::Result<(Option<u16>, Option<u16>)> {
    let (left_margin, content_width) = writing_column(cfg, layout.cols);
    let line_spacing = cfg.line_spacing.max(1);
    let line_capacity = visible_line_capacity(layout.doc_height, line_spacing);
    let (visual_rows, cursor_row) = document_visual_rows(editor, content_width);
    let top = if cursor_row >= line_capacity {
        cursor_row - line_capacity + 1
    } else {
        0
    };

    let mut caret = (None, None);
    for i in 0..line_capacity {
        let idx = top + i;
        let Some(row) = visual_rows.get(idx) else {
            break;
        };
        let y = layout.doc_top + i as u16 * line_spacing;

        let mut x = left_margin;
        if row.composition_start > 0 {
            x = print_chars(
                out,
                x,
                y,
                &row.chars[..row.composition_start],
                theme.fg,
                theme.bg,
            )?;
        }
        if row.composition_start < row.composition_end {
            x = print_chars(
                out,
                x,
                y,
                &row.chars[row.composition_start..row.composition_end],
                theme.accent,
                theme.bg,
            )?;
        }
        if row.composition_end < row.chars.len() {
            print_chars(
                out,
                x,
                y,
                &row.chars[row.composition_end..],
                theme.fg,
                theme.bg,
            )?;
        }

        if let Some(caret_col) = row.caret {
            let caret_x = left_margin
                + row.chars[..caret_col]
                    .iter()
                    .map(|&c| char_width(c))
                    .sum::<u16>();
            caret = (Some(caret_x), Some(y));
        }
    }
    Ok(caret)
}

#[derive(Debug, PartialEq, Eq)]
struct VisualRow {
    chars: Vec<char>,
    composition_start: usize,
    composition_end: usize,
    caret: Option<usize>,
}

fn document_visual_rows(editor: &Editor, width: u16) -> (Vec<VisualRow>, usize) {
    let cursor = editor.cursor();
    let composing: Vec<char> = editor.composing().chars().collect();
    let mut rows = Vec::new();
    let mut cursor_row = 0;

    for (logical_row, line) in editor.lines().iter().enumerate() {
        let is_cursor_line = logical_row == cursor.row;
        let mut display = line.clone();
        let insert_at = cursor.col.min(display.len());
        let (composition_start, composition_end, caret) = if is_cursor_line {
            display.splice(insert_at..insert_at, composing.iter().copied());
            (
                insert_at,
                insert_at + composing.len(),
                Some(insert_at + composing.len()),
            )
        } else {
            (0, 0, None)
        };
        let ranges = wrapped_char_ranges(&display, width, caret);
        let caret_range = caret.and_then(|position| {
            ranges
                .iter()
                .rposition(|&(start, end)| position >= start && position <= end)
        });

        for (range_index, (start, end)) in ranges.into_iter().enumerate() {
            let local_caret =
                caret.filter(|_| caret_range.is_some_and(|target| target == range_index));
            if local_caret.is_some() {
                cursor_row = rows.len();
            }
            rows.push(VisualRow {
                chars: display[start..end].to_vec(),
                composition_start: composition_start.clamp(start, end) - start,
                composition_end: composition_end.clamp(start, end) - start,
                caret: local_caret.map(|position| position - start),
            });
        }
    }

    (rows, cursor_row)
}

fn wrapped_char_ranges(line: &[char], width: u16, caret: Option<usize>) -> Vec<(usize, usize)> {
    let width = width.max(1);
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut used: u16 = 0;

    for (index, &character) in line.iter().enumerate() {
        let character_width = char_width(character);
        if index > start && used.saturating_add(character_width) > width {
            ranges.push((start, index));
            start = index;
            used = 0;
        }
        used = used.saturating_add(character_width);
    }
    ranges.push((start, line.len()));

    if caret == Some(line.len()) && !line.is_empty() && used >= width {
        ranges.push((line.len(), line.len()));
    }
    ranges
}

fn writing_column(cfg: &Config, terminal_width: u16) -> (u16, u16) {
    if terminal_width == 0 {
        return (0, 0);
    }
    if cfg.page_width {
        let width = terminal_width.min(PAGE_CONTENT_WIDTH);
        ((terminal_width - width) / 2, width)
    } else {
        let margin = terminal_width.min(4);
        (margin, terminal_width.saturating_sub(margin).max(1))
    }
}

fn visible_line_capacity(height: u16, spacing: u16) -> usize {
    if height == 0 {
        0
    } else {
        (height.saturating_sub(1) / spacing.max(1) + 1) as usize
    }
}

fn draw_big(
    out: &mut Stdout,
    editor: &Editor,
    cfg: &Config,
    theme: &Theme,
    layout: &Layout,
    languages: &LanguageRegistry,
) -> io::Result<()> {
    let (focus_left, focus_width) = writing_column(cfg, layout.cols);
    // One terminal column is the theoretical minimum per glyph, so fetching
    // at most the available width is sufficient. The exact glyph widths below
    // then determine how many characters really fit.
    let chars = editor.focus_text(focus_width as usize);
    if chars.is_empty() {
        return Ok(());
    }

    let mut glyphs: Vec<Glyph> = chars.iter().map(|&c| glyph_for(c, languages)).collect();

    // Honor the configured horizontal level by trimming old context first.
    // Previously the renderer silently reduced the scale to fit the whole
    // phrase, making several configured levels look identical.
    let requested_scale = cfg.font_size.max(1);
    trim_glyphs_to_width(&mut glyphs, requested_scale, focus_width);
    let glyph_h_max = glyphs.iter().map(|g| g.height as u16).max().unwrap_or(10);
    if layout.big_height * 2 < glyph_h_max || unscaled_width(&glyphs) > focus_width {
        return Ok(());
    }
    let available_x = focus_width / unscaled_width(&glyphs).max(1);
    let available_y = layout.big_height * 2 / glyph_h_max.max(1);
    let (scale_x, scale_y) = fitted_scales(requested_scale, available_x, available_y);
    let gap = scale_x;

    let widths_sum: u16 = glyphs.iter().map(|g| g.width as u16 * scale_x).sum();
    let total = widths_sum + gap * (glyphs.len() as u16 - 1);
    let start_x = focus_left
        + if total < focus_width {
            (focus_width - total) / 2
        } else {
            0
        };

    let block_h = (glyph_h_max * scale_y).div_ceil(2);
    let start_y = layout.big_top
        + if layout.big_height > block_h {
            (layout.big_height - block_h) / 2
        } else {
            0
        };

    let mut gx = start_x;
    for g in &glyphs {
        let glyph_y = start_y + ((glyph_h_max - g.height as u16) * scale_y).div_ceil(2);
        draw_glyph(out, g, gx, glyph_y, scale_x, scale_y, theme)?;
        gx += g.width as u16 * scale_x + gap;
    }
    Ok(())
}

fn fitted_scales(requested: u16, available_x: u16, available_y: u16) -> (u16, u16) {
    (
        requested.min(available_x).max(1),
        requested.min(available_y).max(1),
    )
}

fn unscaled_width(glyphs: &[Glyph]) -> u16 {
    let glyph_width: u16 = glyphs.iter().map(|g| g.width as u16).sum();
    glyph_width.saturating_add(glyphs.len().saturating_sub(1) as u16)
}

fn trim_glyphs_to_width(glyphs: &mut Vec<Glyph>, scale: u16, width: u16) {
    while glyphs.len() > 1 && unscaled_width(glyphs).saturating_mul(scale) > width {
        glyphs.remove(0);
    }
}

fn draw_glyph(
    out: &mut Stdout,
    g: &Glyph,
    ox: u16,
    oy: u16,
    scale_x: u16,
    scale_y: u16,
    theme: &Theme,
) -> io::Result<()> {
    if g.rows.iter().all(|&row| row == 0) {
        return Ok(());
    }

    // A terminal cell is roughly twice as tall as it is wide. Unicode half
    // blocks preserve two vertical bitmap pixels per row. On short terminals,
    // levels 4–5 may use a larger horizontal than vertical scale.
    let expanded_height = g.height * scale_y as usize;
    for expanded_y in (0..expanded_height).step_by(2) {
        let top_y = expanded_y / scale_y as usize;
        let bottom_y = (expanded_y + 1) / scale_y as usize;
        let row = oy + expanded_y as u16 / 2;
        for x in 0..g.width {
            let top = g.lit(x, top_y);
            let bottom = expanded_y + 1 < expanded_height && g.lit(x, bottom_y);
            let (symbol, foreground, background) = pixel_cell(top, bottom, theme);
            let pixels: String = std::iter::repeat_n(symbol, scale_x as usize).collect();
            queue!(
                out,
                cursor::MoveTo(ox + x as u16 * scale_x, row),
                SetForegroundColor(foreground),
                SetBackgroundColor(background),
                Print(&pixels)
            )?;
        }
    }
    Ok(())
}

fn pixel_cell(top: bool, bottom: bool, theme: &Theme) -> (char, Color, Color) {
    match (top, bottom) {
        (true, true) => ('█', theme.pixel, theme.bg),
        (true, false) => ('▀', theme.pixel, theme.pixel_off),
        (false, true) => ('▄', theme.pixel, theme.pixel_off),
        // A real space keeps glyphs legible when NO_COLOR suppresses the
        // foreground color that normally distinguishes the faint pixel grid.
        (false, false) => (' ', theme.pixel_off, theme.bg),
    }
}

fn draw_status(
    out: &mut Stdout,
    editor: &Editor,
    cfg: &Config,
    theme: &Theme,
    cols: u16,
    row: u16,
) -> io::Result<()> {
    let locale = Language::from_code(&cfg.language).unwrap_or(Language::English);
    let mode = if cfg.live_composition {
        "IME:LIVE"
    } else {
        "IME:OS"
    };
    let mut sound = match (locale, cfg.sound) {
        (Language::Korean, true) => format!("소리:{}", cfg.sound_profile),
        (Language::Japanese, true) => format!("音:{}", cfg.sound_profile),
        (Language::English, true) => format!("sound:{}", cfg.sound_profile),
        (Language::Korean, false) => "무음".to_string(),
        (Language::Japanese, false) => "消音".to_string(),
        (Language::English, false) => "muted".to_string(),
    };
    if cfg.sound && !cfg.backspace_sound {
        sound.push_str(match locale {
            Language::Korean => " 삭제:끔",
            Language::Japanese => " 削除:オフ",
            Language::English => " del:off",
        });
    }
    if cfg.sound && !cfg.return_sound {
        sound.push_str(match locale {
            Language::Korean => " 엔터:끔",
            Language::Japanese => " 改行:オフ",
            Language::English => " return:off",
        });
    }
    let dirty = if editor.doc.dirty { "*" } else { "" };
    let name = editor
        .doc
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| match locale {
            Language::Korean => "제목 없음".to_string(),
            Language::Japanese => "無題".to_string(),
            Language::English => "untitled".to_string(),
        });
    let stage = editor.composer().stage();
    let left = match locale {
        Language::Korean => format!(
            " {mode} │ {sound} │ {dirty}{name} │ {}단어 {}자 │ 크기 {}/{} │ 줄 {}/{} │ 페이지:{} │ 조합 {stage}/3 ",
            editor.word_count(),
            editor.char_count(),
            cfg.font_size,
            MAX_FONT,
            cfg.line_spacing,
            MAX_LINE_SPACING,
            if cfg.page_width { "켬" } else { "끔" },
        ),
        Language::Japanese => format!(
            " {mode} │ {sound} │ {dirty}{name} │ {}語 {}文字 │ サイズ {}/{} │ 行間 {}/{} │ ページ:{} ",
            editor.word_count(),
            editor.char_count(),
            cfg.font_size,
            MAX_FONT,
            cfg.line_spacing,
            MAX_LINE_SPACING,
            if cfg.page_width { "オン" } else { "オフ" },
        ),
        Language::English => format!(
            " {mode} │ {sound} │ {dirty}{name} │ {} words {} chars │ size {}/{} │ line {}/{} │ page:{} │ compose {stage}/3 ",
            editor.word_count(),
            editor.char_count(),
            cfg.font_size,
            MAX_FONT,
            cfg.line_spacing,
            MAX_LINE_SPACING,
            if cfg.page_width { "on" } else { "off" },
        ),
    };

    let bar: String = " ".repeat(cols as usize);
    queue!(
        out,
        cursor::MoveTo(0, row),
        SetForegroundColor(theme.fg),
        SetBackgroundColor(theme.dim),
        Print(&bar),
        cursor::MoveTo(0, row),
        Print(clipped(&left, cols))
    )?;
    Ok(())
}

fn draw_shortcuts(
    out: &mut Stdout,
    prompt: Option<&FilePrompt>,
    cfg: &Config,
    theme: &Theme,
    cols: u16,
    row: u16,
) -> io::Result<()> {
    let locale = Language::from_code(&cfg.language).unwrap_or(Language::English);
    let bar: String = " ".repeat(cols as usize);
    queue!(
        out,
        cursor::MoveTo(0, row),
        SetForegroundColor(theme.bg),
        SetBackgroundColor(theme.fg),
        Print(&bar),
        cursor::MoveTo(0, row),
    )?;

    if prompt.is_none() {
        return draw_shortcut_guide(out, shortcut_guide(locale, cols), theme, cols);
    }

    let text = if let Some(error) = prompt.and_then(|prompt| prompt.error.as_ref()) {
        let message = localized_prompt_error(error, locale);
        match locale {
            Language::Korean => format!(" 오류: {message}  │  Esc 취소 "),
            Language::Japanese => format!(" エラー: {message}  │  Esc キャンセル "),
            Language::English => format!(" Error: {message}  │  Esc Cancel "),
        }
    } else if prompt.is_some_and(|prompt| prompt.kind == FilePromptKind::Open) {
        match locale {
            Language::Korean => {
                " ↑/↓ 선택  Tab 자동완성  Enter 열기  Esc 취소  F1 도움말 ".to_string()
            }
            Language::Japanese => {
                " ↑/↓ 選択  Tab 補完  Enter 開く  Esc キャンセル  F1 ヘルプ ".to_string()
            }
            Language::English => {
                " ↑/↓ Select  Tab Complete  Enter Open  Esc Cancel  F1 Help ".to_string()
            }
        }
    } else if prompt.is_some() {
        match locale {
            Language::Korean => " Enter 저장  Esc 취소  │ 확장자 생략 시 .md  F9 언어 ".to_string(),
            Language::Japanese => {
                " Enter 保存  Esc キャンセル  │ 拡張子なし → .md  F9 言語 ".to_string()
            }
            Language::English => {
                " Enter Save  Esc Cancel  │ No extension → .md  F9 Languages ".to_string()
            }
        }
    } else {
        unreachable!("the persistent guide is rendered before prompt-specific hints")
    };
    queue!(
        out,
        SetForegroundColor(theme.bg),
        SetBackgroundColor(theme.fg),
        Print(clipped(&text, cols))
    )
}

#[derive(Debug, Clone, Copy)]
struct ShortcutGroup {
    modifier: Option<&'static str>,
    bindings: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Clone, Copy)]
struct ShortcutGuide {
    groups: &'static [ShortcutGroup],
}

const fn shortcut_group(
    modifier: Option<&'static str>,
    bindings: &'static [(&'static str, &'static str)],
) -> ShortcutGroup {
    ShortcutGroup { modifier, bindings }
}

impl ShortcutGuide {
    fn width(self) -> u16 {
        self.groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                let separator = u16::from(index > 0) * text_width(" │ ");
                let modifier = group
                    .modifier
                    .map_or(0, |modifier| text_width(modifier) + 2);
                let bindings: u16 = group
                    .bindings
                    .iter()
                    .map(|(key, action)| 2 + text_width(key) + text_width(action))
                    .sum();
                separator + modifier + bindings
            })
            .sum()
    }

    #[cfg(test)]
    fn plain_text(self) -> String {
        self.groups
            .iter()
            .map(|group| {
                let bindings = group
                    .bindings
                    .iter()
                    .map(|(key, action)| format!("{key} {action}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                group.modifier.map_or(bindings.clone(), |modifier| {
                    format!("{modifier} {bindings}")
                })
            })
            .collect::<Vec<_>>()
            .join(" │ ")
    }
}

const CTRL_EN: &[(&str, &str)] = &[("O", "Open"), ("S", "Save"), ("Q", "Quit")];
const CTRL_KO: &[(&str, &str)] = &[("O", "열기"), ("S", "저장"), ("Q", "종료")];
const CTRL_JA: &[(&str, &str)] = &[("O", "開く"), ("S", "保存"), ("Q", "終了")];
const CTRL_QUIT_EN: &[(&str, &str)] = &[("Q", "Quit")];
const CTRL_QUIT_KO: &[(&str, &str)] = &[("Q", "종료")];
const CTRL_QUIT_JA: &[(&str, &str)] = &[("Q", "終了")];

const F_WIDE_EN: &[(&str, &str)] = &[
    ("F1", "Help"),
    ("F2", "Input"),
    ("F3", "Focus"),
    ("F4", "Big"),
    ("F5", "Page"),
    ("F6", "Theme"),
    ("F7/8", "Size"),
    ("F9", "Languages"),
    ("F10", "Sound"),
    ("F12", "Save-as"),
];
const F_WIDE_KO: &[(&str, &str)] = &[
    ("F1", "도움"),
    ("F2", "입력"),
    ("F3", "집중"),
    ("F4", "큰글자"),
    ("F5", "종이폭"),
    ("F6", "테마"),
    ("F7/8", "크기"),
    ("F9", "언어"),
    ("F10", "소리"),
    ("F12", "다른이름"),
];
const F_WIDE_JA: &[(&str, &str)] = &[
    ("F1", "ヘルプ"),
    ("F2", "入力"),
    ("F3", "集中"),
    ("F4", "拡大"),
    ("F5", "ページ"),
    ("F6", "テーマ"),
    ("F7/8", "サイズ"),
    ("F9", "言語"),
    ("F10", "サウンド"),
    ("F12", "別名保存"),
];
const F_STANDARD_EN: &[(&str, &str)] = &[
    ("F1", "Help"),
    ("F3", "Focus"),
    ("F5", "Page"),
    ("F6", "Theme"),
    ("F7/8", "Size"),
    ("F9", "Languages"),
    ("F10", "Sound"),
];
const F_STANDARD_KO: &[(&str, &str)] = &[
    ("F1", "도움"),
    ("F3", "집중"),
    ("F5", "종이폭"),
    ("F6", "테마"),
    ("F7/8", "크기"),
    ("F9", "언어"),
    ("F10", "소리"),
];
const F_STANDARD_JA: &[(&str, &str)] = &[
    ("F1", "ヘルプ"),
    ("F3", "集中"),
    ("F5", "ページ"),
    ("F6", "テーマ"),
    ("F7/8", "サイズ"),
    ("F9", "言語"),
    ("F10", "音"),
];
const F_COMPACT_EN: &[(&str, &str)] = &[("F1", "Help"), ("F5", "Page"), ("F10", "Sound")];
const F_COMPACT_KO: &[(&str, &str)] = &[("F1", "도움"), ("F5", "종이"), ("F10", "소리")];
const F_COMPACT_JA: &[(&str, &str)] = &[("F1", "ヘルプ"), ("F5", "幅"), ("F10", "音")];
const F_NARROW_EN: &[(&str, &str)] = &[("F5", "Page"), ("F10", "Sound")];
const F_NARROW_KO: &[(&str, &str)] = &[("F5", "종이"), ("F10", "소리")];
const F_NARROW_JA: &[(&str, &str)] = &[("F5", "幅"), ("F10", "音")];
const F_TINY_EN: &[(&str, &str)] = F_NARROW_EN;
const F_TINY_KO: &[(&str, &str)] = F_NARROW_KO;
const F_TINY_JA: &[(&str, &str)] = F_NARROW_JA;
const SPACING_EN: &[(&str, &str)] = &[("F5", "Spacing")];
const SPACING_KO: &[(&str, &str)] = &[("F5", "줄간격")];
const SPACING_JA: &[(&str, &str)] = &[("F5", "行間")];

const WIDE_EN: &[ShortcutGroup] = &[
    shortcut_group(Some("Ctrl"), CTRL_EN),
    shortcut_group(None, F_WIDE_EN),
    shortcut_group(Some("Shift"), SPACING_EN),
];
const WIDE_KO: &[ShortcutGroup] = &[
    shortcut_group(Some("Ctrl"), CTRL_KO),
    shortcut_group(None, F_WIDE_KO),
    shortcut_group(Some("Shift"), SPACING_KO),
];
const WIDE_JA: &[ShortcutGroup] = &[
    shortcut_group(Some("Ctrl"), CTRL_JA),
    shortcut_group(None, F_WIDE_JA),
    shortcut_group(Some("Shift"), SPACING_JA),
];
const STANDARD_EN: &[ShortcutGroup] = &[
    shortcut_group(Some("Ctrl"), CTRL_EN),
    shortcut_group(None, F_STANDARD_EN),
    shortcut_group(Some("Shift"), SPACING_EN),
];
const STANDARD_KO: &[ShortcutGroup] = &[
    shortcut_group(Some("Ctrl"), CTRL_KO),
    shortcut_group(None, F_STANDARD_KO),
    shortcut_group(Some("Shift"), SPACING_KO),
];
const STANDARD_JA: &[ShortcutGroup] = &[
    shortcut_group(Some("Ctrl"), CTRL_JA),
    shortcut_group(None, F_STANDARD_JA),
    shortcut_group(Some("Shift"), SPACING_JA),
];
const COMPACT_EN: &[ShortcutGroup] = &[
    shortcut_group(Some("Ctrl"), CTRL_QUIT_EN),
    shortcut_group(None, F_COMPACT_EN),
    shortcut_group(Some("Shift"), SPACING_EN),
];
const COMPACT_KO: &[ShortcutGroup] = &[
    shortcut_group(Some("Ctrl"), CTRL_QUIT_KO),
    shortcut_group(None, F_COMPACT_KO),
    shortcut_group(Some("Shift"), SPACING_KO),
];
const COMPACT_JA: &[ShortcutGroup] = &[
    shortcut_group(Some("Ctrl"), CTRL_QUIT_JA),
    shortcut_group(None, F_COMPACT_JA),
    shortcut_group(Some("Shift"), SPACING_JA),
];
const NARROW_EN: &[ShortcutGroup] = &[
    shortcut_group(None, F_NARROW_EN),
    shortcut_group(Some("Shift"), SPACING_EN),
];
const NARROW_KO: &[ShortcutGroup] = &[
    shortcut_group(None, F_NARROW_KO),
    shortcut_group(Some("Shift"), SPACING_KO),
];
const NARROW_JA: &[ShortcutGroup] = &[
    shortcut_group(None, F_NARROW_JA),
    shortcut_group(Some("Shift"), SPACING_JA),
];
const TINY_EN: &[ShortcutGroup] = &[shortcut_group(None, F_TINY_EN)];
const TINY_KO: &[ShortcutGroup] = &[shortcut_group(None, F_TINY_KO)];
const TINY_JA: &[ShortcutGroup] = &[shortcut_group(None, F_TINY_JA)];

fn shortcut_guide(language: Language, cols: u16) -> ShortcutGuide {
    let choices = match language {
        Language::Korean => [WIDE_KO, STANDARD_KO, COMPACT_KO, NARROW_KO, TINY_KO],
        Language::Japanese => [WIDE_JA, STANDARD_JA, COMPACT_JA, NARROW_JA, TINY_JA],
        Language::English => [WIDE_EN, STANDARD_EN, COMPACT_EN, NARROW_EN, TINY_EN],
    };

    choices
        .into_iter()
        .map(|groups| ShortcutGuide { groups })
        .find(|guide| guide.width() <= cols)
        .unwrap_or(ShortcutGuide { groups: choices[4] })
}

fn draw_shortcut_guide(
    out: &mut Stdout,
    guide: ShortcutGuide,
    theme: &Theme,
    cols: u16,
) -> io::Result<()> {
    let mut remaining = cols;
    for (index, group) in guide.groups.iter().enumerate() {
        if index > 0 {
            draw_shortcut_segment(out, &mut remaining, " │ ", theme.dim, theme.fg)?;
        }
        if let Some(modifier) = group.modifier {
            draw_shortcut_segment(
                out,
                &mut remaining,
                &format!(" {modifier} "),
                theme.bg,
                theme.accent,
            )?;
        }
        for (key, action) in group.bindings {
            draw_shortcut_segment(out, &mut remaining, &format!(" {key}"), theme.bg, theme.dim)?;
            draw_shortcut_segment(
                out,
                &mut remaining,
                &format!(" {action}"),
                theme.bg,
                theme.fg,
            )?;
        }
    }
    Ok(())
}

fn draw_shortcut_segment(
    out: &mut Stdout,
    remaining: &mut u16,
    text: &str,
    foreground: Color,
    background: Color,
) -> io::Result<()> {
    if *remaining == 0 {
        return Ok(());
    }
    let visible = clipped(text, *remaining);
    *remaining = remaining.saturating_sub(text_width(&visible));
    queue!(
        out,
        SetForegroundColor(foreground),
        SetBackgroundColor(background),
        Print(visible)
    )
}

fn text_width(text: &str) -> u16 {
    text.chars().map(char_width).sum()
}

fn draw_file_prompt(
    out: &mut Stdout,
    prompt: &FilePrompt,
    cfg: &Config,
    theme: &Theme,
    cols: u16,
    row: u16,
) -> io::Result<(u16, u16)> {
    let prefix = format!(" {}: ", prompt.label(&cfg.language));
    let full = format!("{prefix}{}", prompt.input);
    let visible = clipped_from_end(&full, cols.saturating_sub(1));
    let width = visible.chars().map(char_width).sum::<u16>();
    let bar: String = " ".repeat(cols as usize);
    queue!(
        out,
        cursor::MoveTo(0, row),
        SetForegroundColor(theme.fg),
        SetBackgroundColor(theme.dim),
        Print(&bar),
        cursor::MoveTo(0, row),
        Print(&visible)
    )?;
    Ok((width.min(cols.saturating_sub(1)), row))
}

fn localized_prompt_error(error: &FilePromptError, language: Language) -> String {
    match (error, language) {
        (FilePromptError::EmptyPath, Language::English) => "Enter a file path".to_string(),
        (FilePromptError::UnsavedChanges, Language::English) => {
            "Unsaved changes: press Esc and save first".to_string()
        }
        (FilePromptError::OpenFailed(error), Language::English) => format!("Open failed: {error}"),
        (FilePromptError::SaveFailed(error), Language::English) => format!("Save failed: {error}"),
        (FilePromptError::EmptyPath, Language::Korean) => "파일 경로를 입력하세요".to_string(),
        (FilePromptError::UnsavedChanges, Language::Korean) => {
            "저장하지 않은 변경이 있습니다. Esc 후 먼저 저장하세요".to_string()
        }
        (FilePromptError::OpenFailed(error), Language::Korean) => format!("불러오기 실패: {error}"),
        (FilePromptError::SaveFailed(error), Language::Korean) => format!("저장 실패: {error}"),
        (FilePromptError::EmptyPath, Language::Japanese) => {
            "ファイルパスを入力してください".to_string()
        }
        (FilePromptError::UnsavedChanges, Language::Japanese) => {
            "未保存の変更があります。Escで戻って先に保存してください".to_string()
        }
        (FilePromptError::OpenFailed(error), Language::Japanese) => {
            format!("開けませんでした: {error}")
        }
        (FilePromptError::SaveFailed(error), Language::Japanese) => {
            format!("保存できませんでした: {error}")
        }
    }
}

fn clipped(text: &str, max_width: u16) -> String {
    let mut width = 0;
    text.chars()
        .take_while(|&character| {
            let next = width + char_width(character);
            if next > max_width {
                false
            } else {
                width = next;
                true
            }
        })
        .collect()
}

fn clipped_from_end(text: &str, max_width: u16) -> String {
    let mut width = 0;
    let mut kept: Vec<char> = text
        .chars()
        .rev()
        .take_while(|&character| {
            let next = width + char_width(character);
            if next > max_width {
                false
            } else {
                width = next;
                true
            }
        })
        .collect();
    kept.reverse();
    kept.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_guide_groups_shared_modifiers_like_a_modal_status_bar() {
        for language in Language::ALL {
            let guide = shortcut_guide(language, 180);
            let labels: Vec<Option<&str>> =
                guide.groups.iter().map(|group| group.modifier).collect();
            assert_eq!(labels, [Some("Ctrl"), None, Some("Shift")]);

            let text = guide.plain_text();
            assert_eq!(text.matches("Ctrl").count(), 1);
            assert!(text.contains(match language {
                Language::Korean => "Ctrl O 열기 S 저장 Q 종료",
                Language::Japanese => "Ctrl O 開く S 保存 Q 終了",
                Language::English => "Ctrl O Open S Save Q Quit",
            }));
        }
    }

    #[test]
    fn shortcut_guide_adapts_without_hiding_primary_reading_and_sound_settings() {
        for language in Language::ALL {
            for width in [24, 40, 80, 180] {
                let guide = shortcut_guide(language, width);
                let text = guide.plain_text();
                assert!(guide.width() <= width);
                assert!(text.contains("F5"));
                assert!(text.contains("F10"));
                if width >= 80 {
                    assert!(text.contains("F1"));
                }
                if width >= 40 {
                    assert!(text.contains("Shift F5"));
                }
            }
        }

        for language in Language::ALL {
            assert!(!shortcut_guide(language, 40).plain_text().contains("Alt+L"));
        }
    }

    #[test]
    fn five_font_levels_remain_distinct_on_a_standard_height() {
        let profiles: Vec<(u16, u16)> = (1..=5)
            .map(|requested| fitted_scales(requested, 5, 3))
            .collect();
        assert_eq!(profiles, [(1, 1), (2, 2), (3, 3), (4, 3), (5, 3)]);
    }

    #[test]
    fn taller_terminals_keep_the_largest_levels_proportional() {
        assert_eq!(fitted_scales(4, 5, 5), (4, 4));
        assert_eq!(fitted_scales(5, 5, 5), (5, 5));
    }

    #[test]
    fn blank_big_pixels_remain_empty_without_terminal_colors() {
        let theme = Theme::by_name("night");

        assert_eq!(pixel_cell(true, true, &theme), ('█', theme.pixel, theme.bg));
        assert_eq!(
            pixel_cell(false, false, &theme),
            (' ', theme.pixel_off, theme.bg)
        );
        assert_eq!(
            pixel_cell(true, false, &theme),
            ('▀', theme.pixel, theme.pixel_off)
        );
        assert_eq!(
            pixel_cell(false, true, &theme),
            ('▄', theme.pixel, theme.pixel_off)
        );
    }

    #[test]
    fn relaxed_line_spacing_uses_physical_rows_without_losing_the_cursor_line() {
        assert_eq!(visible_line_capacity(10, 1), 10);
        assert_eq!(visible_line_capacity(10, 2), 5);
        assert_eq!(visible_line_capacity(10, 3), 4);
        assert_eq!(visible_line_capacity(0, 2), 0);
    }

    #[test]
    fn page_width_centers_an_eighty_column_writing_area() {
        let mut cfg = Config::default();
        assert_eq!(writing_column(&cfg, 120), (4, 116));

        cfg.page_width = true;
        assert_eq!(writing_column(&cfg, 120), (20, 80));
        assert_eq!(writing_column(&cfg, 60), (0, 60));
    }

    #[test]
    fn long_lines_soft_wrap_at_terminal_cell_boundaries() {
        let line: Vec<char> = "a한b".chars().collect();
        assert_eq!(wrapped_char_ranges(&line, 3, None), [(0, 2), (2, 3)]);
        assert_eq!(wrapped_char_ranges(&line, 4, None), [(0, 3)]);
    }

    #[test]
    fn caret_moves_to_a_new_visual_row_at_an_exact_boundary() {
        let line: Vec<char> = "abcd".chars().collect();
        assert_eq!(
            wrapped_char_ranges(&line, 4, Some(line.len())),
            [(0, 4), (4, 4)]
        );
    }

    #[test]
    fn visual_wrapping_does_not_insert_document_newlines() {
        let mut editor = Editor::new();
        for character in "ab한cd".chars() {
            editor.insert_char(character);
        }

        let (rows, cursor_row) = document_visual_rows(&editor, 4);
        let displayed: Vec<String> = rows.iter().map(|row| row.chars.iter().collect()).collect();

        assert_eq!(displayed, ["ab한", "cd"]);
        assert_eq!(cursor_row, 1);
        assert_eq!(rows[cursor_row].caret, Some(2));
        assert_eq!(editor.buffer.to_text(), "ab한cd");
    }

    #[test]
    fn wider_focus_areas_show_more_big_glyphs() {
        let languages =
            LanguageRegistry::load_from(std::env::temp_dir().join("termleaf-terminal-font"));
        let glyphs = vec![glyph_for('a', &languages); 30];
        let mut narrow = glyphs.clone();
        let mut wide = glyphs;
        trim_glyphs_to_width(&mut narrow, 1, 40);
        trim_glyphs_to_width(&mut wide, 1, 80);
        assert!(wide.len() > narrow.len());
    }
}
