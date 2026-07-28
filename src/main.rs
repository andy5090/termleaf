//! Tadak (타닥) — a distraction-free terminal writing app.
//!
//! Big pixel fonts, a mechanical-typewriter feel, and live Hangul jamo-by-jamo
//! composition. Rendering is ANSI-only (via crossterm) for broad terminal
//! compatibility.

mod audio;
mod config;
mod editor;
mod input;
mod renderer;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};

use audio::SoundPlayer;
use config::Config;
use editor::Editor;
use input::{map_key, Action};
use renderer::{draw, TerminalGuard};
use ui::Theme;

fn main() -> io::Result<()> {
    let mut cfg = Config::load();
    let path = std::env::args().nth(1);

    let mut editor = match &path {
        Some(p) => Editor::open(p)?,
        None => Editor::new(),
    };

    // The guard restores the terminal on drop (including on panic/error).
    let _guard = TerminalGuard::enter()?;
    let mut stdout = io::stdout();
    let mut theme = Theme::by_name(&cfg.theme);
    let mut last_autosave = Instant::now();
    let sound = SoundPlayer::new();

    loop {
        draw(&mut stdout, &editor, &cfg, &theme)?;

        // Wake at least twice a second so autosave can fire while idle.
        if event::poll(Duration::from_millis(500))? {
            let ev = event::read()?;
            if let Event::Key(key) = ev {
                if key.kind != KeyEventKind::Release {
                    let action = map_key(key, cfg.hangul_mode);
                    if matches!(action, Action::Quit) {
                        break;
                    }
                    apply(&mut editor, &mut cfg, &mut theme, &sound, action);
                }
            }
        }

        maybe_autosave(&mut editor, &cfg, &mut last_autosave);
    }

    // Best-effort: flush pending composition, save config.
    editor.flush();
    let _ = cfg.save();
    Ok(())
}

fn apply(
    editor: &mut Editor,
    cfg: &mut Config,
    theme: &mut Theme,
    sound: &SoundPlayer,
    action: Action,
) {
    match action {
        Action::InsertChar(c) => {
            editor.insert_char(c);
            clack(sound, cfg);
        }
        Action::Jamo(j) => {
            editor.input_jamo(j);
            clack(sound, cfg);
        }
        Action::Backspace => {
            editor.backspace();
            clack(sound, cfg);
        }
        Action::Newline => {
            editor.newline();
            clack(sound, cfg);
        }
        Action::Left => editor.move_left(),
        Action::Right => editor.move_right(),
        Action::Up => editor.move_up(),
        Action::Down => editor.move_down(),
        Action::Home => editor.move_home(),
        Action::End => editor.move_end(),
        Action::ToggleHangul => {
            editor.flush();
            cfg.hangul_mode = !cfg.hangul_mode;
        }
        Action::ToggleFocus => cfg.focus_mode = !cfg.focus_mode,
        Action::ToggleBigFont => cfg.big_font = !cfg.big_font,
        Action::ToggleSound => {
            cfg.sound = !cfg.sound;
            if cfg.sound {
                sound.play();
            }
        }
        Action::ToggleTheme => {
            cfg.theme = Theme::next(&cfg.theme).to_string();
            *theme = Theme::by_name(&cfg.theme);
        }
        Action::FontInc => cfg.font_inc(),
        Action::FontDec => cfg.font_dec(),
        Action::Save => {
            let _ = editor.save();
        }
        Action::Quit | Action::Ignore => {}
    }
}

/// Queue a soft system sound without blocking input or rendering.
fn clack(sound: &SoundPlayer, cfg: &Config) {
    if cfg.sound {
        sound.play();
    }
}

fn maybe_autosave(editor: &mut Editor, cfg: &Config, last: &mut Instant) {
    if cfg.autosave_secs == 0 || !editor.doc.dirty || editor.doc.path.is_none() {
        return;
    }
    if last.elapsed().as_secs() >= cfg.autosave_secs {
        let _ = editor.save();
        *last = Instant::now();
    }
}
