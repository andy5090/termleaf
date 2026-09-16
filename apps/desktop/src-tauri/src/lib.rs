use std::{
    path::{Component, Path, PathBuf, MAIN_SEPARATOR},
    sync::mpsc::{self, Sender},
    thread,
};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::Color,
};
use serde::Serialize;
use tauri::State;
use termleaf::{
    audio::SoundPlayer,
    config::{Config, LiveInputMode},
    editor::{buffer::Buffer, state::Cursor, Editor},
    input::{map_key, Action},
    language::{Language, LanguageRegistry},
    renderer::font::glyph_for,
    ui::{FilePrompt, Theme},
};

const FOCUS_CHARACTER_COUNT: usize = 12;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EditorView {
    active_document_id: u64,
    documents: Vec<DocumentView>,
    text: String,
    cursor_utf16: usize,
    path: Option<String>,
    dirty: bool,
    composing: String,
    words: usize,
    characters: usize,
    input_mode: &'static str,
    japanese_katakana: bool,
    candidate: Option<String>,
    config: ConfigView,
    colors: ColorsView,
    glyphs: Vec<GlyphView>,
    languages: Vec<LanguageView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentView {
    id: u64,
    path: Option<String>,
    label: String,
    dirty: bool,
}

struct ParkedDocument {
    id: u64,
    editor: Editor,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigView {
    theme: String,
    font_size: u16,
    line_spacing: u16,
    page_width: bool,
    big_font: bool,
    focus_mode: bool,
    sound: bool,
    sound_profile: String,
    backspace_sound: bool,
    return_sound: bool,
    language: String,
    autosave_secs: u64,
    show_welcome: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ColorsView {
    fg: String,
    bg: String,
    dim: String,
    accent: String,
    pixel: String,
    pixel_off: String,
}

#[derive(Debug, Serialize)]
struct GlyphView {
    width: usize,
    height: usize,
    rows: Vec<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LanguageView {
    code: &'static str,
    name: &'static str,
    installed: bool,
    live_input: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileLocation {
    input: String,
    candidates: Vec<FileCandidateView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileCandidateView {
    path: String,
    name: String,
    is_dir: bool,
}

struct Engine {
    editor: Editor,
    active_document_id: u64,
    next_document_id: u64,
    parked_documents: Vec<ParkedDocument>,
    config: Config,
    languages: LanguageRegistry,
    sound: Option<SoundPlayer>,
    persist_config: bool,
}

impl Engine {
    fn load() -> Self {
        let mut engine = Self {
            editor: Editor::new(),
            active_document_id: 1,
            next_document_id: 2,
            parked_documents: Vec::new(),
            config: Config::load(),
            languages: LanguageRegistry::load(),
            sound: Some(SoundPlayer::new()),
            persist_config: true,
        };
        engine.validate_input_mode();
        engine.load_japanese_conversion();
        engine
    }

    #[cfg(test)]
    fn for_test(languages: LanguageRegistry) -> Self {
        Self {
            editor: Editor::new(),
            active_document_id: 1,
            next_document_id: 2,
            parked_documents: Vec::new(),
            config: Config::default(),
            languages,
            sound: None,
            persist_config: false,
        }
    }

    fn validate_input_mode(&mut self) {
        if self.config.live_composition && !self.languages.is_installed(Language::Korean) {
            self.config.live_composition = false;
        }
        if self.config.live_japanese && !self.languages.supports_live_input(Language::Japanese) {
            self.config.live_japanese = false;
        }
    }

    fn load_japanese_conversion(&mut self) {
        if !self.config.live_japanese {
            return;
        }
        let Some(path) = self
            .languages
            .pack_asset(Language::Japanese, "akaza-default-model")
        else {
            self.editor.clear_japanese_model();
            return;
        };
        if self.editor.load_japanese_model(&path).is_err() {
            self.editor.clear_japanese_model();
        }
    }

    fn save_config(&self) -> Result<(), String> {
        if self.persist_config {
            self.config.save().map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn view(&self) -> EditorView {
        let composing = self.editor.composing();
        let committed_text = self.editor.buffer.to_text();
        let committed_cursor = cursor_character_index(&self.editor);
        let byte_cursor = byte_index_for_character(&committed_text, committed_cursor);
        let mut text = committed_text;
        text.insert_str(byte_cursor, &composing);
        let cursor_utf16 = text[..byte_cursor + composing.len()].encode_utf16().count();
        let theme = Theme::by_name(&self.config.theme);

        EditorView {
            active_document_id: self.active_document_id,
            documents: self.documents(),
            text,
            cursor_utf16,
            path: self
                .editor
                .doc
                .path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            dirty: self.editor.doc.dirty,
            composing,
            words: self.editor.word_count(),
            characters: self.editor.char_count(),
            input_mode: match self.config.live_input_mode() {
                LiveInputMode::Off => "os",
                LiveInputMode::Korean => "ko",
                LiveInputMode::Japanese => "ja",
            },
            japanese_katakana: self.config.japanese_katakana,
            candidate: candidate_status(&self.editor),
            config: ConfigView {
                theme: self.config.theme.clone(),
                font_size: self.config.font_size,
                line_spacing: self.config.line_spacing,
                page_width: self.config.page_width,
                big_font: self.config.big_font,
                focus_mode: self.config.focus_mode,
                sound: self.config.sound,
                sound_profile: self.config.sound_profile.clone(),
                backspace_sound: self.config.backspace_sound,
                return_sound: self.config.return_sound,
                language: self.config.language.clone(),
                autosave_secs: self.config.autosave_secs,
                show_welcome: self.config.show_welcome,
            },
            colors: ColorsView {
                fg: color_hex(theme.fg),
                bg: color_hex(theme.bg),
                dim: color_hex(theme.dim),
                accent: color_hex(theme.accent),
                pixel: color_hex(theme.pixel),
                pixel_off: color_hex(theme.pixel_off),
            },
            glyphs: self
                .editor
                .focus_text(FOCUS_CHARACTER_COUNT)
                .into_iter()
                .map(|character| {
                    let glyph = glyph_for(character, &self.languages);
                    GlyphView {
                        width: glyph.width,
                        height: glyph.height,
                        rows: glyph.rows,
                    }
                })
                .collect(),
            languages: Language::ALL
                .into_iter()
                .map(|language| LanguageView {
                    code: language.code(),
                    name: language.native_name(),
                    installed: self.languages.is_installed(language),
                    live_input: self.languages.supports_live_input(language),
                })
                .collect(),
        }
    }

    fn sync(&mut self, text: String, cursor_utf16: usize) {
        let previous = self.editor.buffer.to_text();
        let previous_lines = previous.matches('\n').count();
        let changed = previous != text || !self.editor.composing().is_empty();

        if !self.editor.composing().is_empty() {
            self.editor.cancel_composition();
        }
        self.editor.buffer = Buffer::from_str(&text);
        self.editor.doc.cursor = cursor_from_utf16(&text, cursor_utf16);
        self.editor.doc.dirty |= changed;

        if changed && self.config.sound {
            if text.chars().count() < previous.chars().count() && self.config.backspace_sound {
                self.play_backspace();
            } else if text.matches('\n').count() > previous_lines && self.config.return_sound {
                self.play_return();
            } else {
                self.play_key();
            }
        }
    }

    fn select(&mut self, cursor_utf16: usize) {
        self.editor.flush();
        let text = self.editor.buffer.to_text();
        self.editor.doc.cursor = cursor_from_utf16(&text, cursor_utf16);
    }

    fn apply_key(&mut self, key: String, ctrl: bool, alt: bool, shift: bool) -> Result<(), String> {
        let Some(code) = key_code(&key, shift) else {
            return Ok(());
        };
        let mut modifiers = KeyModifiers::NONE;
        if ctrl {
            modifiers |= KeyModifiers::CONTROL;
        }
        if alt {
            modifiers |= KeyModifiers::ALT;
        }
        if shift {
            modifiers |= KeyModifiers::SHIFT;
        }
        let action = map_key(
            KeyEvent::new(code, modifiers),
            self.config.live_composition,
            self.config.live_japanese,
        );
        self.apply_editor_action(action)
    }

    fn apply_editor_action(&mut self, action: Action) -> Result<(), String> {
        let mut config_changed = false;
        match action {
            Action::InsertChar(character) => {
                self.editor.insert_char(character);
                self.play_key();
            }
            Action::Jamo(jamo) => {
                self.editor.input_jamo(jamo);
                self.play_key();
            }
            Action::Romaji(character) => {
                self.editor
                    .input_romaji(character, self.config.japanese_katakana);
                self.play_key();
            }
            Action::JapaneseConvertNext => {
                if !self.editor.japanese_convert_next() {
                    self.editor.insert_char(' ');
                }
                self.play_key();
            }
            Action::JapaneseConvertPrev => {
                self.editor.japanese_convert_prev();
            }
            Action::CancelComposition => {
                self.editor.cancel_composition();
            }
            Action::Backspace => {
                if self.editor.backspace() && self.config.sound && self.config.backspace_sound {
                    self.play_backspace();
                }
            }
            Action::Delete => {
                if self.editor.delete_forward() && self.config.sound && self.config.backspace_sound
                {
                    self.play_backspace();
                }
            }
            Action::Newline => {
                self.editor.newline();
                if self.config.sound && self.config.return_sound {
                    self.play_return();
                }
            }
            Action::Left => {
                if !self.editor.japanese_move_segment_left() {
                    self.editor.move_left();
                }
            }
            Action::Right => {
                if !self.editor.japanese_move_segment_right() {
                    self.editor.move_right();
                }
            }
            Action::Up => self.editor.move_up(),
            Action::Down => self.editor.move_down(),
            Action::Home => self.editor.move_home(),
            Action::End => self.editor.move_end(),
            Action::CycleLiveInput => {
                self.cycle_live_input(false);
                config_changed = true;
            }
            Action::CycleLiveInputReverse => {
                self.cycle_live_input(true);
                config_changed = true;
            }
            Action::ToggleJapaneseScript => {
                if self.config.live_japanese {
                    self.editor.flush();
                    self.config.japanese_katakana = !self.config.japanese_katakana;
                    config_changed = true;
                }
            }
            Action::ToggleFocus => {
                self.config.focus_mode = !self.config.focus_mode;
                config_changed = true;
            }
            Action::ToggleBigFont => {
                self.config.big_font = !self.config.big_font;
                config_changed = true;
            }
            Action::ToggleTheme => {
                self.config.theme = Theme::next(&self.config.theme).to_owned();
                config_changed = true;
            }
            Action::CycleSoundProfile => {
                self.config.cycle_sound_profile();
                config_changed = true;
                self.play_key();
            }
            Action::FontInc => {
                self.config.font_inc();
                config_changed = true;
            }
            Action::FontDec => {
                self.config.font_dec();
                config_changed = true;
            }
            Action::CycleLineSpacing => {
                self.config.cycle_line_spacing();
                config_changed = true;
            }
            Action::TogglePageWidth => {
                self.config.page_width = !self.config.page_width;
                config_changed = true;
            }
            Action::Save => {
                self.save(None)?;
            }
            Action::ShowHelp
            | Action::ShowLanguageSettings
            | Action::ShowSoundSettings
            | Action::ToggleTouchMode
            | Action::Open
            | Action::SaveAs
            | Action::Quit
            | Action::Ignore => {}
        }
        if config_changed {
            self.save_config()?;
        }
        Ok(())
    }

    fn action(&mut self, name: &str) -> Result<(), String> {
        match name {
            "cycle-input" => self.cycle_live_input(false),
            "reverse-input" => self.cycle_live_input(true),
            "toggle-script" if self.config.live_japanese => {
                self.editor.flush();
                self.config.japanese_katakana = !self.config.japanese_katakana;
            }
            "toggle-script" => {}
            "toggle-focus" => self.config.focus_mode = !self.config.focus_mode,
            "toggle-big" => self.config.big_font = !self.config.big_font,
            "cycle-theme" => self.config.theme = Theme::next(&self.config.theme).to_owned(),
            "font-inc" => self.config.font_inc(),
            "font-dec" => self.config.font_dec(),
            "cycle-spacing" => self.config.cycle_line_spacing(),
            "toggle-page" => self.config.page_width = !self.config.page_width,
            "toggle-sound" => self.config.sound = !self.config.sound,
            "cycle-sound" => {
                self.config.cycle_sound_profile();
                self.play_key();
            }
            "toggle-backspace-sound" => {
                self.config.backspace_sound = !self.config.backspace_sound;
            }
            "toggle-return-sound" => {
                self.config.return_sound = !self.config.return_sound;
            }
            "toggle-welcome" => self.config.show_welcome = !self.config.show_welcome,
            _ => return Err(format!("Unknown editor action: {name}")),
        }
        self.save_config()
    }

    fn cycle_live_input(&mut self, reverse: bool) {
        let available = Language::ALL
            .into_iter()
            .filter(|language| self.languages.supports_live_input(*language))
            .filter_map(live_input_mode)
            .collect::<Vec<_>>();
        self.editor.flush();
        self.editor.clear_japanese_model();
        if self.config.cycle_live_input(&available, reverse) == LiveInputMode::Japanese {
            self.load_japanese_conversion();
        }
    }

    fn documents(&self) -> Vec<DocumentView> {
        let mut documents = self
            .parked_documents
            .iter()
            .map(|document| document_view(document.id, &document.editor))
            .collect::<Vec<_>>();
        documents.push(document_view(self.active_document_id, &self.editor));
        documents.sort_by_key(|document| document.id);
        documents
    }

    fn activate_new(&mut self, replacement: Editor) {
        self.editor.flush();
        self.editor.clear_japanese_model();
        self.parked_documents.push(ParkedDocument {
            id: self.active_document_id,
            editor: std::mem::replace(&mut self.editor, replacement),
        });
        self.active_document_id = self.next_document_id;
        self.next_document_id += 1;
        self.load_japanese_conversion();
    }

    fn new_document(&mut self) {
        self.activate_new(Editor::new());
    }

    fn switch_document(&mut self, document_id: u64) -> Result<(), String> {
        if document_id == self.active_document_id {
            return Ok(());
        }
        let index = self
            .parked_documents
            .iter()
            .position(|document| document.id == document_id)
            .ok_or_else(|| "This document is no longer open".to_string())?;
        self.editor.flush();
        self.editor.clear_japanese_model();
        let document = &mut self.parked_documents[index];
        std::mem::swap(&mut self.editor, &mut document.editor);
        std::mem::swap(&mut self.active_document_id, &mut document.id);
        self.load_japanese_conversion();
        Ok(())
    }

    fn document_for_path(&self, path: &Path) -> Result<Option<u64>, String> {
        let normalized = normalized_path(path)?;
        for (id, editor) in std::iter::once((self.active_document_id, &self.editor)).chain(
            self.parked_documents
                .iter()
                .map(|document| (document.id, &document.editor)),
        ) {
            if let Some(open_path) = &editor.doc.path {
                if normalized_path(open_path)? == normalized {
                    return Ok(Some(id));
                }
            }
        }
        Ok(None)
    }

    fn open(&mut self, path: &str) -> Result<(), String> {
        let path = expand_path(path)?;
        if let Some(document_id) = self.document_for_path(&path)? {
            return self.switch_document(document_id);
        }
        let replacement = Editor::open(&path).map_err(|error| error.to_string())?;
        self.activate_new(replacement);
        Ok(())
    }

    fn save(&mut self, path: Option<String>) -> Result<(), String> {
        if let Some(path) = path {
            let path = save_target(&path, self.editor.doc.path.as_deref())?;
            if self
                .document_for_path(&path)?
                .is_some_and(|id| id != self.active_document_id)
            {
                return Err("This file is already open in another document".to_string());
            }
            self.editor
                .save_as(path)
                .map_err(|error| error.to_string())?;
        } else {
            if self.editor.doc.path.is_none() {
                return Err("Choose a filename before saving this document".to_string());
            }
            self.editor.save().map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn language(&mut self, code: &str, operation: &str) -> Result<(), String> {
        let language = Language::from_code(code)
            .ok_or_else(|| format!("Unsupported language code: {code}"))?;
        match operation {
            "select" => {
                if !self.languages.is_installed(language) {
                    return Err(format!("{} is not installed", language.native_name()));
                }
                self.config.set_language(language.code());
            }
            "install" => {
                self.languages
                    .install(language)
                    .map_err(|error| error.to_string())?;
            }
            "remove" => {
                if matches!(language, Language::Korean | Language::Japanese) {
                    self.editor.flush();
                }
                self.languages
                    .remove(language)
                    .map_err(|error| error.to_string())?;
                if language == Language::Korean {
                    self.config.live_composition = false;
                }
                if language == Language::Japanese {
                    self.config.live_japanese = false;
                    self.editor.clear_japanese_model();
                }
                if self.config.language == language.code() {
                    self.config.set_language("en");
                }
            }
            _ => return Err(format!("Unknown language operation: {operation}")),
        }
        self.save_config()
    }

    fn play_key(&self) {
        if self.config.sound {
            if let Some(sound) = &self.sound {
                sound.play_key(&self.config.sound_profile);
            }
        }
    }

    fn play_backspace(&self) {
        if let Some(sound) = &self.sound {
            sound.play_backspace();
        }
    }

    fn play_return(&self) {
        if let Some(sound) = &self.sound {
            sound.play_return();
        }
    }
}

enum EditorCommand {
    Snapshot,
    New,
    Switch(u64),
    Sync(String, usize),
    Select(usize),
    Key(String, bool, bool, bool),
    Action(String),
    Open(String),
    Save(Option<String>),
    Language(String, String),
}

enum EditorRequest {
    Edit(EditorCommand, Sender<Result<EditorView, String>>),
    Browse(String, Sender<Result<FileLocation, String>>),
}

/// Tauri state stays thread-safe while the non-`Send` editor and audio engine
/// retain affinity to one session worker for the entire application lifetime.
#[derive(Clone)]
struct EditorState(Sender<EditorRequest>);

impl EditorState {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("termleaf-desktop-editor".into())
            .spawn(move || {
                let mut engine = Engine::load();
                while let Ok(request) = receiver.recv() {
                    match request {
                        EditorRequest::Edit(command, respond) => {
                            let result =
                                apply_command(&mut engine, command).map(|()| engine.view());
                            let _ = respond.send(result);
                        }
                        EditorRequest::Browse(path, respond) => {
                            let _ = respond.send(browse(&path));
                        }
                    }
                }
            })
            .expect("failed to start the Termleaf editor session");
        Self(sender)
    }

    fn edit(&self, command: EditorCommand) -> Result<EditorView, String> {
        let (respond, response) = mpsc::channel();
        self.0
            .send(EditorRequest::Edit(command, respond))
            .map_err(|_| "The editor session has stopped".to_string())?;
        response
            .recv()
            .map_err(|_| "The editor session did not respond".to_string())?
    }

    fn browse(&self, path: String) -> Result<FileLocation, String> {
        let (respond, response) = mpsc::channel();
        self.0
            .send(EditorRequest::Browse(path, respond))
            .map_err(|_| "The editor session has stopped".to_string())?;
        response
            .recv()
            .map_err(|_| "The editor session did not respond".to_string())?
    }
}

fn apply_command(engine: &mut Engine, command: EditorCommand) -> Result<(), String> {
    match command {
        EditorCommand::Snapshot => Ok(()),
        EditorCommand::New => {
            engine.new_document();
            Ok(())
        }
        EditorCommand::Switch(document_id) => engine.switch_document(document_id),
        EditorCommand::Sync(text, cursor) => {
            engine.sync(text, cursor);
            Ok(())
        }
        EditorCommand::Select(cursor) => {
            engine.select(cursor);
            Ok(())
        }
        EditorCommand::Key(key, ctrl, alt, shift) => engine.apply_key(key, ctrl, alt, shift),
        EditorCommand::Action(name) => engine.action(&name),
        EditorCommand::Open(path) => engine.open(&path),
        EditorCommand::Save(path) => engine.save(path),
        EditorCommand::Language(code, operation) => engine.language(&code, &operation),
    }
}

async fn edit(state: State<'_, EditorState>, command: EditorCommand) -> Result<EditorView, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.edit(command))
        .await
        .map_err(|error| format!("The editor request failed: {error}"))?
}

#[tauri::command]
async fn editor_snapshot(state: State<'_, EditorState>) -> Result<EditorView, String> {
    edit(state, EditorCommand::Snapshot).await
}

#[tauri::command]
async fn editor_new(state: State<'_, EditorState>) -> Result<EditorView, String> {
    edit(state, EditorCommand::New).await
}

#[tauri::command]
async fn editor_switch(
    state: State<'_, EditorState>,
    document_id: u64,
) -> Result<EditorView, String> {
    edit(state, EditorCommand::Switch(document_id)).await
}

#[tauri::command]
async fn editor_sync(
    state: State<'_, EditorState>,
    text: String,
    cursor_utf16: usize,
) -> Result<EditorView, String> {
    edit(state, EditorCommand::Sync(text, cursor_utf16)).await
}

#[tauri::command]
async fn editor_select(
    state: State<'_, EditorState>,
    cursor_utf16: usize,
) -> Result<EditorView, String> {
    edit(state, EditorCommand::Select(cursor_utf16)).await
}

#[tauri::command]
async fn editor_key(
    state: State<'_, EditorState>,
    key: String,
    ctrl: bool,
    alt: bool,
    shift: bool,
) -> Result<EditorView, String> {
    edit(state, EditorCommand::Key(key, ctrl, alt, shift)).await
}

#[tauri::command]
async fn editor_action(state: State<'_, EditorState>, name: String) -> Result<EditorView, String> {
    edit(state, EditorCommand::Action(name)).await
}

#[tauri::command]
async fn editor_open(state: State<'_, EditorState>, path: String) -> Result<EditorView, String> {
    edit(state, EditorCommand::Open(path)).await
}

#[tauri::command]
async fn editor_browse(
    state: State<'_, EditorState>,
    path: String,
) -> Result<FileLocation, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.browse(path))
        .await
        .map_err(|error| format!("The file browser request failed: {error}"))?
}

#[tauri::command]
async fn editor_save(
    state: State<'_, EditorState>,
    path: Option<String>,
) -> Result<EditorView, String> {
    edit(state, EditorCommand::Save(path)).await
}

#[tauri::command]
async fn editor_language(
    state: State<'_, EditorState>,
    code: String,
    operation: String,
) -> Result<EditorView, String> {
    edit(state, EditorCommand::Language(code, operation)).await
}

fn document_view(id: u64, editor: &Editor) -> DocumentView {
    let label = editor
        .doc
        .path
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            editor
                .buffer
                .to_text()
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("")
                .trim()
                .chars()
                .take(40)
                .collect()
        });
    DocumentView {
        id,
        path: editor
            .doc
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        label,
        dirty: editor.doc.dirty || !editor.composing().is_empty(),
    }
}

// Canonicalize the existing ancestor so aliases also match before a new file exists.
fn normalized_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    let mut ancestor = absolute.as_path();
    let mut suffix = Vec::new();
    let mut result = loop {
        if let Ok(canonical) = ancestor.canonicalize() {
            break canonical;
        }
        let component = ancestor
            .components()
            .next_back()
            .ok_or_else(|| "The file path cannot be resolved".to_string())?;
        suffix.push(component.as_os_str().to_owned());
        ancestor = ancestor
            .parent()
            .ok_or_else(|| "The file path cannot be resolved".to_string())?;
    };
    for component in suffix.into_iter().rev() {
        match Path::new(&component).components().next() {
            Some(Component::ParentDir) => {
                result.pop();
            }
            Some(Component::CurDir) | None => {}
            _ => result.push(component),
        }
    }
    Ok(result)
}

fn candidate_status(editor: &Editor) -> Option<String> {
    let (candidate, candidates) = editor.japanese_candidate_position()?;
    match editor.japanese_segment_position() {
        Some((segment, segments)) => Some(format!(
            "Candidate {candidate}/{candidates} · segment {segment}/{segments}"
        )),
        None => Some(format!("Candidate {candidate}/{candidates}")),
    }
}

fn cursor_character_index(editor: &Editor) -> usize {
    let Cursor { row, col } = editor.cursor();
    editor
        .lines()
        .iter()
        .take(row)
        .map(|line| line.len() + 1)
        .sum::<usize>()
        + col
}

fn byte_index_for_character(text: &str, character_index: usize) -> usize {
    text.char_indices()
        .nth(character_index)
        .map_or(text.len(), |(byte, _)| byte)
}

fn cursor_from_utf16(text: &str, target: usize) -> Cursor {
    let mut utf16 = 0;
    let mut cursor = Cursor::default();
    for character in text.chars() {
        let next = utf16 + character.len_utf16();
        if next > target {
            break;
        }
        utf16 = next;
        if character == '\n' {
            cursor.row += 1;
            cursor.col = 0;
        } else {
            cursor.col += 1;
        }
    }
    cursor
}

fn key_code(key: &str, shift: bool) -> Option<KeyCode> {
    match key {
        "Backspace" => Some(KeyCode::Backspace),
        "Delete" => Some(KeyCode::Delete),
        "Enter" => Some(KeyCode::Enter),
        "Tab" if shift => Some(KeyCode::BackTab),
        "Tab" => Some(KeyCode::Tab),
        "Escape" | "Esc" => Some(KeyCode::Esc),
        "ArrowLeft" | "Left" => Some(KeyCode::Left),
        "ArrowRight" | "Right" => Some(KeyCode::Right),
        "ArrowUp" | "Up" => Some(KeyCode::Up),
        "ArrowDown" | "Down" => Some(KeyCode::Down),
        "Home" => Some(KeyCode::Home),
        "End" => Some(KeyCode::End),
        " " | "Space" | "Spacebar" => Some(KeyCode::Char(' ')),
        value if value.starts_with('F') => value[1..].parse().ok().map(KeyCode::F),
        value => {
            let mut characters = value.chars();
            let character = characters.next()?;
            characters
                .next()
                .is_none()
                .then_some(KeyCode::Char(character))
        }
    }
}

fn live_input_mode(language: Language) -> Option<LiveInputMode> {
    match language {
        Language::English => None,
        Language::Korean => Some(LiveInputMode::Korean),
        Language::Japanese => Some(LiveInputMode::Japanese),
    }
}

fn expand_path(value: &str) -> Result<PathBuf, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Choose a file path".into());
    }
    if value == "~" || value.starts_with("~/") {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or_else(|| "The home directory is unavailable".to_string())?;
        let suffix = value.strip_prefix("~/").unwrap_or("");
        return Ok(PathBuf::from(home).join(suffix));
    }
    Ok(PathBuf::from(value))
}

fn save_target(value: &str, current: Option<&Path>) -> Result<PathBuf, String> {
    let expanded = expand_path(value)?;
    if expanded.is_dir() {
        return Err("Choose a filename, not a directory".to_string());
    }
    let mut prompt = FilePrompt::save_as(current);
    prompt.input = expanded.to_string_lossy().into_owned();
    prompt
        .save_target()
        .ok_or_else(|| "Choose a filename, not a directory".to_string())
}

fn browse(value: &str) -> Result<FileLocation, String> {
    let mut input = if value.trim().is_empty() {
        String::new()
    } else {
        expand_path(value)?.to_string_lossy().into_owned()
    };
    if Path::new(&input).is_dir() && !input.ends_with(MAIN_SEPARATOR) {
        input.push(MAIN_SEPARATOR);
    }
    let mut prompt = FilePrompt::open(None);
    prompt.set_input(input);
    Ok(FileLocation {
        input: prompt.input,
        candidates: prompt
            .candidates
            .into_iter()
            .map(|candidate| FileCandidateView {
                name: candidate
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| candidate.path.to_string_lossy().into_owned()),
                path: candidate.path.to_string_lossy().into_owned(),
                is_dir: candidate.is_dir,
            })
            .collect(),
    })
}

fn color_hex(color: Color) -> String {
    let (red, green, blue) = match color {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::DarkGrey => (85, 85, 85),
        Color::Red => (255, 85, 85),
        Color::DarkRed => (170, 0, 0),
        Color::Green => (85, 255, 85),
        Color::DarkGreen => (0, 170, 0),
        Color::Yellow => (255, 255, 85),
        Color::DarkYellow => (170, 85, 0),
        Color::Blue => (85, 85, 255),
        Color::DarkBlue => (0, 0, 170),
        Color::Magenta => (255, 85, 255),
        Color::DarkMagenta => (170, 0, 170),
        Color::Cyan => (85, 255, 255),
        Color::DarkCyan => (0, 170, 170),
        Color::White => (255, 255, 255),
        Color::Grey => (170, 170, 170),
        Color::AnsiValue(_) | Color::Reset => (0, 0, 0),
    };
    format!("#{red:02x}{green:02x}{blue:02x}")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(EditorState::new())
        .invoke_handler(tauri::generate_handler![
            editor_snapshot,
            editor_new,
            editor_switch,
            editor_sync,
            editor_select,
            editor_key,
            editor_action,
            editor_open,
            editor_browse,
            editor_save,
            editor_language,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Termleaf");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_engine(name: &str) -> (Engine, tempfile::TempDir) {
        let directory = tempfile::tempdir().expect("temporary desktop data");
        let languages = LanguageRegistry::load_from(directory.path().join(name));
        (Engine::for_test(languages), directory)
    }

    fn root_asset(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join(path)
    }

    #[test]
    fn switching_keeps_unsaved_unicode_and_each_caret() {
        let (mut engine, _directory) = test_engine("documents");
        let first = engine.active_document_id;
        engine.sync("  \n한글 👩‍💻 first".into(), 7);
        let first_view = engine.view();
        engine.new_document();
        let second = engine.active_document_id;
        assert_ne!(first, second);
        assert_eq!(engine.view().text, "");
        engine.sync("日本語 second".into(), 2);
        engine.switch_document(first).unwrap();
        assert_eq!(engine.view().text, first_view.text);
        assert_eq!(engine.view().cursor_utf16, first_view.cursor_utf16);
        assert!(engine.view().dirty);
        assert_eq!(engine.view().documents[0].label, "한글 👩‍💻 first");
        engine.switch_document(second).unwrap();
        assert_eq!(engine.view().text, "日本語 second");
        assert_eq!(engine.view().cursor_utf16, 2);
        assert_eq!(engine.view().documents.len(), 2);
    }

    #[test]
    fn opening_the_same_file_activates_its_unsaved_buffer() {
        let (mut engine, directory) = test_engine("dedupe");
        let path = directory.path().join("draft.md");
        fs::write(&path, "on disk").unwrap();
        engine.open(path.to_str().unwrap()).unwrap();
        let opened_id = engine.active_document_id;
        engine.sync("unsaved 한글".into(), 3);
        engine.new_document();
        let count = engine.view().documents.len();
        let alias = directory.path().join(".").join("draft.md");
        engine.open(alias.to_str().unwrap()).unwrap();
        assert_eq!(engine.active_document_id, opened_id);
        assert_eq!(engine.view().text, "unsaved 한글");
        assert_eq!(engine.view().cursor_utf16, 3);
        assert!(engine.view().dirty);
        assert_eq!(engine.view().documents.len(), count);
        assert_eq!(fs::read_to_string(path).unwrap(), "on disk");
    }

    #[test]
    fn saving_only_changes_the_active_document() {
        let (mut engine, directory) = test_engine("save-active");
        engine.sync("first unsaved".into(), 5);
        let first = engine.active_document_id;
        engine.new_document();
        engine.sync("second saved".into(), 4);
        let second = engine.active_document_id;
        let path = directory.path().join("second.md");
        engine
            .save(Some(path.to_string_lossy().into_owned()))
            .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second saved");
        assert!(!engine.view().dirty);
        assert_eq!(engine.view().documents[1].label, "second.md");
        engine.switch_document(first).unwrap();
        assert_eq!(engine.view().text, "first unsaved");
        assert!(engine.view().dirty);
        assert!(engine.view().path.is_none());
        engine.switch_document(second).unwrap();
        assert_eq!(engine.view().cursor_utf16, 4);
    }

    #[test]
    fn invalid_switch_and_open_preserve_every_document() {
        let (mut engine, directory) = test_engine("invalid-documents");
        engine.sync("first".into(), 1);
        let first = engine.active_document_id;
        engine.new_document();
        engine.sync("second".into(), 2);
        let before = engine.view();
        assert!(engine.switch_document(u64::MAX).is_err());
        assert!(engine.open(directory.path().to_str().unwrap()).is_err());
        assert_eq!(engine.active_document_id, before.active_document_id);
        assert_eq!(engine.view().text, before.text);
        assert_eq!(engine.view().cursor_utf16, before.cursor_utf16);
        assert_eq!(engine.view().documents.len(), 2);
        engine.switch_document(first).unwrap();
        assert_eq!(engine.view().text, "first");
        assert_eq!(engine.view().cursor_utf16, 1);
    }

    #[test]
    fn save_as_cannot_overwrite_another_open_document() {
        let (mut engine, directory) = test_engine("save-collision");
        let path = directory.path().join("first.md");
        fs::write(&path, "first on disk").unwrap();
        engine.open(path.to_str().unwrap()).unwrap();
        let first = engine.active_document_id;
        engine.sync("first unsaved".into(), 5);
        engine.new_document();
        engine.sync("second unsaved".into(), 6);
        let alias = directory.path().join(".").join("first.md");
        assert!(engine
            .save(Some(alias.to_string_lossy().into_owned()))
            .is_err());
        assert_eq!(engine.view().text, "second unsaved");
        assert_eq!(engine.view().cursor_utf16, 6);
        assert!(engine.view().path.is_none());
        assert!(engine.view().dirty);
        assert_eq!(fs::read_to_string(&path).unwrap(), "first on disk");
        engine.switch_document(first).unwrap();
        assert_eq!(engine.view().text, "first unsaved");
    }

    #[test]
    fn switching_commits_live_composition_without_losing_text() {
        let (mut engine, _directory) = test_engine("park-composition");
        engine.config.live_japanese = true;
        engine.apply_key("n".into(), false, false, false).unwrap();
        engine.apply_key("i".into(), false, false, false).unwrap();
        let first = engine.active_document_id;
        assert_eq!(engine.view().composing, "に");
        engine.new_document();
        engine.switch_document(first).unwrap();
        assert_eq!(engine.view().text, "に");
        assert_eq!(engine.view().cursor_utf16, 1);
        assert!(engine.view().composing.is_empty());
        assert!(engine.view().dirty);
    }

    #[test]
    fn nonexistent_path_aliases_match_before_first_save() {
        let (mut engine, directory) = test_engine("new-path-alias");
        let path = directory.path().join("new").join("draft.md");
        engine.open(path.to_str().unwrap()).unwrap();
        let first = engine.active_document_id;
        engine.sync("not yet on disk".into(), 3);
        engine.new_document();
        let alias = directory
            .path()
            .join("new")
            .join("child")
            .join("..")
            .join("draft.md");
        engine.open(alias.to_str().unwrap()).unwrap();
        assert_eq!(engine.active_document_id, first);
        assert_eq!(engine.view().text, "not yet on disk");
        assert_eq!(engine.view().documents.len(), 3);
    }

    #[test]
    fn utf16_selection_round_trips_multiline_unicode() {
        let (mut engine, _directory) = test_engine("utf16");
        let text = "A👩‍💻한\n日本語";
        let target = "A👩‍💻한\n日".encode_utf16().count();
        engine.sync(text.into(), target);

        let view = engine.view();
        assert_eq!(view.text, text);
        assert_eq!(view.cursor_utf16, target);
        assert_eq!(engine.editor.cursor(), Cursor { row: 1, col: 1 });
    }

    #[test]
    fn live_korean_and_japanese_composition_are_returned_at_the_cursor() {
        let (mut engine, _directory) = test_engine("composition");
        engine
            .languages
            .install_from_source(Language::Korean, &root_asset("language-packs/ko"))
            .unwrap();
        engine.config.live_composition = true;
        for key in ["g", "k", "s"] {
            engine.apply_key(key.into(), false, false, false).unwrap();
        }
        let korean = engine.view();
        assert_eq!(korean.text, "한");
        assert_eq!(korean.composing, "한");
        assert_eq!(korean.cursor_utf16, 1);

        engine.editor.cancel_composition();
        engine.config.live_composition = false;
        engine.config.live_japanese = true;
        for key in ["n", "i"] {
            engine.apply_key(key.into(), false, false, false).unwrap();
        }
        let japanese = engine.view();
        assert_eq!(japanese.text, "に");
        assert_eq!(japanese.composing, "に");
        assert_eq!(japanese.cursor_utf16, 1);
    }

    #[test]
    fn shifted_browser_tab_maps_to_previous_japanese_candidate() {
        let code = key_code("Tab", true).expect("Tab should be recognized");
        assert_eq!(code, KeyCode::BackTab);
        assert_eq!(
            map_key(KeyEvent::new(code, KeyModifiers::SHIFT), false, true),
            Action::JapaneseConvertPrev
        );
    }

    #[test]
    fn normal_text_sync_preserves_path_and_marks_only_changes_dirty() {
        let (mut engine, directory) = test_engine("sync");
        let path = directory.path().join("draft.txt");
        fs::write(&path, "before").unwrap();
        engine.open(path.to_str().unwrap()).unwrap();
        engine.select(3);
        assert!(!engine.editor.doc.dirty);

        engine.sync("before".into(), 6);
        assert!(!engine.editor.doc.dirty);
        engine.sync("after 日本語".into(), 5);
        assert!(engine.editor.doc.dirty);
        assert_eq!(engine.editor.doc.path.as_deref(), Some(path.as_path()));
    }

    #[test]
    fn failed_open_and_save_keep_the_previous_document() {
        let (mut engine, directory) = test_engine("failures");
        engine.sync("keep this 日本語".into(), 4);
        let before = engine.view();
        assert!(engine.save(None).is_err());
        assert_eq!(engine.view().text, before.text);
        let unreadable = directory.path().join("directory");
        fs::create_dir(&unreadable).unwrap();

        assert!(engine.open(unreadable.to_str().unwrap()).is_err());
        assert_eq!(engine.view().text, before.text);
        assert_eq!(engine.view().cursor_utf16, before.cursor_utf16);

        assert!(engine
            .save(Some(unreadable.to_string_lossy().into_owned()))
            .is_err());
        assert_eq!(engine.view().text, before.text);
        assert!(engine.editor.doc.dirty);
    }

    #[test]
    fn removing_a_live_language_flushes_pending_composition() {
        let (mut engine, _directory) = test_engine("remove-live");
        engine
            .languages
            .install_from_source(Language::Korean, &root_asset("language-packs/ko"))
            .unwrap();
        engine.config.live_composition = true;
        for key in ["g", "k", "s"] {
            engine.apply_key(key.into(), false, false, false).unwrap();
        }

        engine.language("ko", "remove").unwrap();

        assert_eq!(engine.editor.buffer.to_text(), "한");
        assert!(engine.editor.composing().is_empty());
        assert!(!engine.config.live_composition);
        assert!(!engine.languages.is_installed(Language::Korean));
    }

    #[test]
    fn save_and_browse_reuse_file_prompt_path_rules() {
        let (mut engine, directory) = test_engine("paths");
        engine.sync("plain text".into(), 10);
        let extensionless = directory.path().join("chapter-one");
        engine
            .save(Some(extensionless.to_string_lossy().into_owned()))
            .unwrap();
        let saved = extensionless.with_extension("md");
        assert_eq!(fs::read_to_string(&saved).unwrap(), "plain text");
        assert_eq!(engine.editor.doc.path.as_deref(), Some(saved.as_path()));

        fs::write(directory.path().join("chapter-two.txt"), "two").unwrap();
        fs::write(directory.path().join("image.png"), "ignored").unwrap();
        let nested = directory.path().join("drafts");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("inside.md"), "nested").unwrap();
        let location = browse(&format!("{}/chapter", directory.path().display())).unwrap();
        assert_eq!(location.candidates.len(), 2);
        assert_eq!(location.candidates[0].name, "chapter-one.md");
        assert_eq!(location.candidates[1].name, "chapter-two.txt");

        let nested_location = browse(&nested.to_string_lossy()).unwrap();
        assert!(nested_location.input.ends_with(MAIN_SEPARATOR));
        assert_eq!(nested_location.candidates.len(), 1);
        assert_eq!(nested_location.candidates[0].name, "inside.md");
    }

    #[test]
    fn preference_actions_are_complete_and_do_not_write_user_config_in_tests() {
        let (mut engine, _directory) = test_engine("actions");
        let welcome = engine.config.show_welcome;
        let theme = engine.config.theme.clone();

        engine.action("toggle-welcome").unwrap();
        engine.action("cycle-theme").unwrap();
        engine.action("toggle-focus").unwrap();
        engine.action("toggle-page").unwrap();

        assert_eq!(engine.config.show_welcome, !welcome);
        assert_ne!(engine.config.theme, theme);
        assert!(engine.config.focus_mode);
        assert!(engine.config.page_width);
        assert!(engine.action("not-an-action").is_err());
    }

    #[test]
    fn view_contains_real_focus_glyphs_theme_and_languages() {
        let (mut engine, _directory) = test_engine("view");
        engine.sync("Termleaf".into(), 8);
        let view = engine.view();

        assert_eq!(view.glyphs.len(), 8);
        assert!(view.glyphs.iter().all(|glyph| glyph.height == 10));
        assert_eq!(view.colors.bg, "#f4f0e8");
        assert_eq!(view.languages[0].code, "en");
        assert!(view.languages[0].installed);
    }
}
