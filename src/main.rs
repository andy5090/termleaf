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
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use audio::SoundPlayer;
use config::Config;
use editor::Editor;
use input::{map_key, Action};
use renderer::{draw, TerminalGuard};
use ui::{FilePrompt, FilePromptError, FilePromptKind, HelpOverlay, SoundSettings, Theme};

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
    let mut ui = UiState {
        file_prompt: None,
        help: cfg.show_welcome.then(HelpOverlay::welcome),
        sound_settings: None,
    };
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            draw(
                &mut stdout,
                &editor,
                &cfg,
                &theme,
                ui.file_prompt.as_ref(),
                ui.help.as_ref(),
                ui.sound_settings.as_ref(),
            )?;
            needs_redraw = false;
        }

        // Wake at least twice a second for autosave, but do not repaint an
        // unchanged frame. This avoids the previous idle-screen flicker.
        if event::poll(Duration::from_millis(500))? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    if ui.help.is_some() {
                        if handle_help_key(key, &mut cfg, &mut ui.help) {
                            break;
                        }
                        needs_redraw = true;
                    } else if ui.sound_settings.is_some() {
                        if handle_sound_settings_key(key, &mut cfg, &sound, &mut ui.sound_settings)
                        {
                            break;
                        }
                        needs_redraw = true;
                    } else if ui.file_prompt.is_some() {
                        if key.code == KeyCode::F(1) {
                            ui.help = Some(HelpOverlay::help(cfg.show_welcome));
                            needs_redraw = true;
                            continue;
                        }
                        if key.code == KeyCode::F(9) {
                            cfg.toggle_language();
                        } else if handle_file_prompt(key, &mut editor, &mut ui.file_prompt) {
                            break;
                        }
                        needs_redraw = true;
                    } else {
                        let action = map_key(key, cfg.live_composition);
                        if matches!(action, Action::Quit) {
                            break;
                        }
                        needs_redraw = !matches!(action, Action::Ignore);
                        apply(&mut editor, &mut cfg, &mut theme, &sound, &mut ui, action);
                    }
                }
                Event::Resize(_, _) => needs_redraw = true,
                _ => {}
            }
        }

        needs_redraw |= maybe_autosave(&mut editor, &cfg, &mut last_autosave);
    }

    // Best-effort: flush pending composition, save config.
    editor.flush();
    let _ = cfg.save();
    Ok(())
}

struct UiState {
    file_prompt: Option<FilePrompt>,
    help: Option<HelpOverlay>,
    sound_settings: Option<SoundSettings>,
}

fn apply(
    editor: &mut Editor,
    cfg: &mut Config,
    theme: &mut Theme,
    sound: &SoundPlayer,
    ui: &mut UiState,
    action: Action,
) {
    match action {
        Action::InsertChar(c) => {
            editor.insert_char(c);
            key_clack(sound, cfg);
        }
        Action::Jamo(j) => {
            editor.input_jamo(j);
            key_clack(sound, cfg);
        }
        Action::Backspace => {
            if editor.backspace() && cfg.sound && cfg.backspace_sound {
                sound.play_backspace();
            }
        }
        Action::Delete => {
            if editor.delete_forward() && cfg.sound && cfg.backspace_sound {
                sound.play_backspace();
            }
        }
        Action::Newline => {
            editor.newline();
            if cfg.sound && cfg.return_sound {
                sound.play_return();
            }
        }
        Action::Left => editor.move_left(),
        Action::Right => editor.move_right(),
        Action::Up => editor.move_up(),
        Action::Down => editor.move_down(),
        Action::Home => editor.move_home(),
        Action::End => editor.move_end(),
        Action::ShowHelp => ui.help = Some(HelpOverlay::help(cfg.show_welcome)),
        Action::ToggleLiveComposition => {
            editor.flush();
            cfg.live_composition = !cfg.live_composition;
        }
        Action::ToggleFocus => cfg.focus_mode = !cfg.focus_mode,
        Action::ToggleBigFont => cfg.big_font = !cfg.big_font,
        Action::ToggleSound => {
            cfg.sound = !cfg.sound;
            if cfg.sound {
                sound.play_key(&cfg.sound_profile);
            }
        }
        Action::ToggleTheme => {
            cfg.theme = Theme::next(&cfg.theme).to_string();
            *theme = Theme::by_name(&cfg.theme);
        }
        Action::ToggleLanguage => cfg.toggle_language(),
        Action::ShowSoundSettings => ui.sound_settings = Some(SoundSettings::new()),
        Action::CycleSoundProfile => {
            cfg.cycle_sound_profile();
            if cfg.sound {
                sound.play_key(&cfg.sound_profile);
            }
        }
        Action::FontInc => cfg.font_inc(),
        Action::FontDec => cfg.font_dec(),
        Action::Open => ui.file_prompt = Some(FilePrompt::open(editor.doc.path.as_deref())),
        Action::Save => {
            if editor.doc.path.is_some() {
                let _ = editor.save();
            } else {
                ui.file_prompt = Some(FilePrompt::save_as(None));
            }
        }
        Action::SaveAs => {
            ui.file_prompt = Some(FilePrompt::save_as(editor.doc.path.as_deref()));
        }
        Action::Quit | Action::Ignore => {}
    }
}

/// Handle the F10 sound-settings panel. Returns `true` only for an explicit
/// application quit.
fn handle_sound_settings_key(
    key: KeyEvent,
    cfg: &mut Config,
    sound: &SoundPlayer,
    settings: &mut Option<SoundSettings>,
) -> bool {
    let quit = matches!(
        key.code,
        KeyCode::Char(c)
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(c.to_ascii_lowercase(), 'q' | 'c')
    );
    if quit {
        return true;
    }

    let Some(active) = settings.as_mut() else {
        return false;
    };
    match key.code {
        KeyCode::Up => active.select_previous(),
        KeyCode::Down => active.select_next(),
        KeyCode::F(5) => toggle_sound_option(cfg, sound, 0, true),
        KeyCode::F(11) => toggle_sound_option(cfg, sound, 3, true),
        KeyCode::Char(' ') | KeyCode::Right => {
            toggle_sound_option(cfg, sound, active.selected, true);
        }
        KeyCode::Left => toggle_sound_option(cfg, sound, active.selected, false),
        KeyCode::F(9) => cfg.toggle_language(),
        KeyCode::Enter | KeyCode::Esc | KeyCode::F(10) => *settings = None,
        _ => {}
    }
    false
}

fn toggle_sound_option(cfg: &mut Config, sound: &SoundPlayer, selected: usize, forward: bool) {
    match selected {
        0 => {
            cfg.sound = !cfg.sound;
            if cfg.sound {
                sound.play_key(&cfg.sound_profile);
            }
        }
        1 => {
            cfg.backspace_sound = !cfg.backspace_sound;
            if cfg.sound && cfg.backspace_sound {
                sound.play_backspace();
            }
        }
        2 => {
            cfg.return_sound = !cfg.return_sound;
            if cfg.sound && cfg.return_sound {
                sound.play_return();
            }
        }
        3 => {
            if forward {
                cfg.cycle_sound_profile();
            } else {
                cfg.previous_sound_profile();
            }
            if cfg.sound {
                sound.play_key(&cfg.sound_profile);
            }
        }
        _ => {}
    }
}

/// Handle the modal welcome/F1 help controls. Returns `true` when the user
/// explicitly quits from the overlay.
fn handle_help_key(key: KeyEvent, cfg: &mut Config, help: &mut Option<HelpOverlay>) -> bool {
    let quit = matches!(
        key.code,
        KeyCode::Char(c)
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(c.to_ascii_lowercase(), 'q' | 'c')
    );
    if quit {
        if let Some(overlay) = help {
            cfg.show_welcome = !overlay.hide_on_startup;
        }
        return true;
    }

    match key.code {
        KeyCode::Char(' ') => {
            if let Some(overlay) = help {
                overlay.toggle_startup_visibility();
            }
        }
        KeyCode::F(9) => cfg.toggle_language(),
        KeyCode::Enter | KeyCode::Esc | KeyCode::F(1) => {
            if let Some(overlay) = help.take() {
                cfg.show_welcome = !overlay.hide_on_startup;
            }
        }
        _ => {}
    }
    false
}

/// Queue a printing-key strike without blocking input or rendering.
fn key_clack(sound: &SoundPlayer, cfg: &Config) {
    if cfg.sound {
        sound.play_key(&cfg.sound_profile);
    }
}

enum PromptCommand {
    None,
    Cancel,
    Submit(FilePromptKind, PathBuf),
    Quit,
}

/// Handle raw path entry while an open/save prompt owns the keyboard. Returns
/// `true` only when the user explicitly quits from inside the prompt.
fn handle_file_prompt(key: KeyEvent, editor: &mut Editor, prompt: &mut Option<FilePrompt>) -> bool {
    let command = {
        let Some(active) = prompt.as_mut() else {
            return false;
        };
        match key.code {
            KeyCode::Char(c)
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(c.to_ascii_lowercase(), 'q' | 'c') =>
            {
                PromptCommand::Quit
            }
            KeyCode::Esc => PromptCommand::Cancel,
            KeyCode::Enter if active.kind == FilePromptKind::Open => {
                match active.choose_open_target() {
                    Some(path) => PromptCommand::Submit(FilePromptKind::Open, path),
                    None if active.input.trim().is_empty() && active.candidates.is_empty() => {
                        active.error = Some(FilePromptError::EmptyPath);
                        PromptCommand::None
                    }
                    None => PromptCommand::None,
                }
            }
            KeyCode::Enter if active.kind == FilePromptKind::SaveAs => match active.save_target() {
                Some(path) => PromptCommand::Submit(FilePromptKind::SaveAs, path),
                None => {
                    active.error = Some(FilePromptError::EmptyPath);
                    PromptCommand::None
                }
            },
            KeyCode::Up => {
                active.select_previous();
                active.error = None;
                PromptCommand::None
            }
            KeyCode::Down => {
                active.select_next();
                active.error = None;
                PromptCommand::None
            }
            KeyCode::Tab => {
                active.complete_selected();
                PromptCommand::None
            }
            KeyCode::Backspace => {
                active.pop();
                PromptCommand::None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                active.push(c);
                PromptCommand::None
            }
            _ => PromptCommand::None,
        }
    };

    match command {
        PromptCommand::None => false,
        PromptCommand::Cancel => {
            *prompt = None;
            false
        }
        PromptCommand::Quit => true,
        PromptCommand::Submit(FilePromptKind::Open, _path) if editor.doc.dirty => {
            if let Some(active) = prompt {
                active.error = Some(FilePromptError::UnsavedChanges);
            }
            false
        }
        PromptCommand::Submit(FilePromptKind::Open, path) => {
            match Editor::open(&path) {
                Ok(opened) => {
                    *editor = opened;
                    *prompt = None;
                }
                Err(error) => {
                    if let Some(active) = prompt {
                        active.error = Some(FilePromptError::OpenFailed(error.to_string()));
                    }
                }
            }
            false
        }
        PromptCommand::Submit(FilePromptKind::SaveAs, path) => {
            match editor.save_as(&path) {
                Ok(_) => *prompt = None,
                Err(error) => {
                    if let Some(active) = prompt {
                        active.error = Some(FilePromptError::SaveFailed(error.to_string()));
                    }
                }
            }
            false
        }
    }
}

fn maybe_autosave(editor: &mut Editor, cfg: &Config, last: &mut Instant) -> bool {
    if cfg.autosave_secs == 0 || !editor.doc.dirty || editor.doc.path.is_none() {
        return false;
    }
    if last.elapsed().as_secs() >= cfg.autosave_secs {
        let saved = editor.save().is_ok();
        *last = Instant::now();
        return saved;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("tadak-{name}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn first_save_prompt_writes_the_chosen_filename() {
        let path = unique_path("first-save").join("note.txt");
        let mut editor = Editor::new();
        editor.insert_char('글');
        let mut prompt = Some(FilePrompt {
            kind: FilePromptKind::SaveAs,
            input: path.to_string_lossy().into_owned(),
            error: None,
            candidates: Vec::new(),
            selected: 0,
        });

        assert!(!handle_file_prompt(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut editor,
            &mut prompt
        ));
        assert!(prompt.is_none());
        assert_eq!(editor.doc.path.as_deref(), Some(path.as_path()));
        assert_eq!(fs::read_to_string(&path).unwrap(), "글");

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn first_save_without_an_extension_creates_a_markdown_file() {
        let path_without_extension = unique_path("markdown-default").join("note");
        let expected = path_without_extension.with_extension("md");
        let mut editor = Editor::new();
        editor.insert_char('글');
        let mut prompt = Some(FilePrompt {
            kind: FilePromptKind::SaveAs,
            input: path_without_extension.to_string_lossy().into_owned(),
            error: None,
            candidates: Vec::new(),
            selected: 0,
        });

        handle_file_prompt(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut editor,
            &mut prompt,
        );

        assert!(prompt.is_none());
        assert_eq!(editor.doc.path.as_deref(), Some(expected.as_path()));
        assert_eq!(fs::read_to_string(&expected).unwrap(), "글");

        fs::remove_dir_all(expected.parent().unwrap()).unwrap();
    }

    #[test]
    fn save_as_keeps_the_original_and_switches_to_the_new_filename() {
        let root = unique_path("save-as");
        let original = root.join("original.txt");
        let copy = root.join("copy.txt");
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.save_as(&original).unwrap();
        editor.insert_char('b');

        let mut prompt = Some(FilePrompt::save_as(editor.doc.path.as_deref()));
        prompt
            .as_mut()
            .expect("save-as prompt should exist")
            .input
            .push_str("copy.txt");
        handle_file_prompt(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut editor,
            &mut prompt,
        );

        assert!(prompt.is_none());
        assert_eq!(editor.doc.path.as_deref(), Some(copy.as_path()));
        assert_eq!(fs::read_to_string(&original).unwrap(), "a");
        assert_eq!(fs::read_to_string(&copy).unwrap(), "ab");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn opening_another_file_refuses_to_discard_unsaved_changes() {
        let path = unique_path("open-guard");
        let mut editor = Editor::new();
        editor.insert_char('!');
        let mut prompt = Some(FilePrompt {
            kind: FilePromptKind::Open,
            input: path.to_string_lossy().into_owned(),
            error: None,
            candidates: Vec::new(),
            selected: 0,
        });

        handle_file_prompt(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut editor,
            &mut prompt,
        );

        assert_eq!(editor.buffer.to_text(), "!");
        assert!(prompt
            .as_ref()
            .and_then(|prompt| prompt.error.as_ref())
            .is_some_and(|error| matches!(error, FilePromptError::UnsavedChanges)));
    }

    #[test]
    fn welcome_checkbox_is_persisted_when_the_overlay_closes() {
        let mut cfg = Config::default();
        let mut help = Some(HelpOverlay::welcome());

        handle_help_key(
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
            &mut cfg,
            &mut help,
        );
        handle_help_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut cfg,
            &mut help,
        );

        assert!(help.is_none());
        assert!(!cfg.show_welcome);
    }

    #[test]
    fn help_can_change_the_interface_language_before_closing() {
        let mut cfg = Config::default();
        let mut help = Some(HelpOverlay::help(cfg.show_welcome));

        handle_help_key(
            KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE),
            &mut cfg,
            &mut help,
        );

        assert_eq!(cfg.language, "ko");
        assert!(help.is_some());
    }
}
