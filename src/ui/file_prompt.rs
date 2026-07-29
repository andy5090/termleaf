//! Small in-app path prompt used by open and save-as commands.

use std::fs;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePromptKind {
    Open,
    SaveAs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePromptError {
    EmptyPath,
    UnsavedChanges,
    OpenFailed(String),
    SaveFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCandidate {
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePrompt {
    pub kind: FilePromptKind,
    pub input: String,
    pub error: Option<FilePromptError>,
    pub candidates: Vec<FileCandidate>,
    pub selected: usize,
}

impl FilePrompt {
    pub fn open(current: Option<&Path>) -> Self {
        let input = directory_prefix(current);
        let mut prompt = Self {
            kind: FilePromptKind::Open,
            input,
            error: None,
            candidates: Vec::new(),
            selected: 0,
        };
        prompt.refresh_candidates();
        prompt
    }

    pub fn save_as(current: Option<&Path>) -> Self {
        let input = current
            .and_then(Path::parent)
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| {
                let mut directory = parent.to_string_lossy().into_owned();
                if !directory.ends_with(MAIN_SEPARATOR) {
                    directory.push(MAIN_SEPARATOR);
                }
                directory
            })
            .unwrap_or_default();
        Self {
            kind: FilePromptKind::SaveAs,
            input,
            error: None,
            candidates: Vec::new(),
            selected: 0,
        }
    }

    pub fn label(&self, korean: bool) -> &'static str {
        match (self.kind, korean) {
            (FilePromptKind::Open, false) => "Open file",
            (FilePromptKind::SaveAs, false) => "Save file",
            (FilePromptKind::Open, true) => "불러올 파일",
            (FilePromptKind::SaveAs, true) => "저장할 파일",
        }
    }

    pub fn push(&mut self, character: char) {
        self.input.push(character);
        self.error = None;
        self.refresh_candidates();
    }

    pub fn pop(&mut self) {
        self.input.pop();
        self.error = None;
        self.refresh_candidates();
    }

    pub fn select_next(&mut self) {
        if !self.candidates.is_empty() {
            self.selected = (self.selected + 1) % self.candidates.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.candidates.is_empty() {
            self.selected = self
                .selected
                .checked_sub(1)
                .unwrap_or(self.candidates.len() - 1);
        }
    }

    /// Put the selected path into the input field. Directories are entered so
    /// their contents immediately replace the candidate list.
    pub fn complete_selected(&mut self) {
        let Some(candidate) = self.candidates.get(self.selected).cloned() else {
            return;
        };
        self.input = display_path(&candidate.path, candidate.is_dir);
        self.error = None;
        self.refresh_candidates();
    }

    /// Resolve Enter in an open prompt. A selected directory navigates rather
    /// than submitting; a selected document or directly typed path is returned.
    pub fn choose_open_target(&mut self) -> Option<PathBuf> {
        if let Some(candidate) = self.candidates.get(self.selected).cloned() {
            if candidate.is_dir {
                self.input = display_path(&candidate.path, true);
                self.error = None;
                self.refresh_candidates();
                return None;
            }
            return Some(candidate.path);
        }
        if self.input.trim().is_empty() {
            return None;
        }
        let typed = PathBuf::from(self.input.trim());
        if typed.is_dir() {
            self.input = display_path(&typed, true);
            self.error = None;
            self.refresh_candidates();
            None
        } else {
            Some(typed)
        }
    }

    /// Resolve a save prompt, applying Markdown as the default format when the
    /// user entered a plain filename without an extension.
    pub fn save_target(&self) -> Option<PathBuf> {
        let input = self.input.trim();
        if input.is_empty() || input.ends_with(MAIN_SEPARATOR) {
            return None;
        }
        let mut path = PathBuf::from(input);
        let hidden_name = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('.'));
        if path.extension().is_none() && !hidden_name {
            path.set_extension("md");
        }
        Some(path)
    }

    fn refresh_candidates(&mut self) {
        if self.kind != FilePromptKind::Open {
            return;
        }
        let (directory, prefix) = split_search_path(&self.input);
        let Ok(entries) = fs::read_dir(&directory) else {
            self.candidates.clear();
            self.selected = 0;
            return;
        };

        let prefix = prefix.to_lowercase();
        let mut candidates: Vec<FileCandidate> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let file_type = entry.file_type().ok()?;
                let name = entry.file_name();
                let name_text = name.to_string_lossy();
                if name_text.starts_with('.') || !name_text.to_lowercase().starts_with(&prefix) {
                    return None;
                }
                let is_dir = file_type.is_dir();
                if !is_dir && !is_document_path(&entry.path()) {
                    return None;
                }
                let path = if directory == Path::new(".") {
                    PathBuf::from(name)
                } else {
                    directory.join(name)
                };
                Some(FileCandidate { path, is_dir })
            })
            .collect();
        candidates.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then_with(|| left.path.cmp(&right.path))
        });
        self.candidates = candidates;
        self.selected = self.selected.min(self.candidates.len().saturating_sub(1));
    }
}

fn directory_prefix(current: Option<&Path>) -> String {
    current
        .and_then(Path::parent)
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| display_path(parent, true))
        .unwrap_or_default()
}

fn split_search_path(input: &str) -> (PathBuf, String) {
    if input.is_empty() {
        return (PathBuf::from("."), String::new());
    }
    let path = Path::new(input);
    if input.ends_with(MAIN_SEPARATOR) {
        return (path.to_path_buf(), String::new());
    }
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let prefix = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    (directory, prefix)
}

fn display_path(path: &Path, directory: bool) -> String {
    let mut text = path.to_string_lossy().into_owned();
    if directory && !text.ends_with(MAIN_SEPARATOR) {
        text.push(MAIN_SEPARATOR);
    }
    text
}

fn is_document_path(path: &Path) -> bool {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|extension| {
            matches!(
                extension.as_str(),
                "txt" | "md" | "markdown" | "mdown" | "rst" | "adoc" | "asciidoc" | "org" | "tex"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("tadak-picker-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn first_save_starts_with_an_empty_filename() {
        let prompt = FilePrompt::save_as(None);
        assert_eq!(prompt.kind, FilePromptKind::SaveAs);
        assert!(prompt.input.is_empty());
        assert_eq!(prompt.label(false), "Save file");
        assert_eq!(prompt.label(true), "저장할 파일");
    }

    #[test]
    fn save_as_prefills_only_the_current_directory() {
        let prompt = FilePrompt::save_as(Some(Path::new("notes/today.txt")));
        assert_eq!(prompt.input, format!("notes{MAIN_SEPARATOR}"));

        let prompt = FilePrompt::save_as(Some(Path::new("today.txt")));
        assert!(prompt.input.is_empty());
    }

    #[test]
    fn save_defaults_to_markdown_but_preserves_explicit_extensions() {
        let mut prompt = FilePrompt::save_as(None);
        prompt.input = "memo".to_string();
        assert_eq!(prompt.save_target(), Some(PathBuf::from("memo.md")));

        prompt.input = "memo.txt".to_string();
        assert_eq!(prompt.save_target(), Some(PathBuf::from("memo.txt")));

        prompt.input = ".notes".to_string();
        assert_eq!(prompt.save_target(), Some(PathBuf::from(".notes")));
    }

    #[test]
    fn open_lists_document_files_and_directories_but_not_other_assets() {
        let root = temporary_directory();
        fs::create_dir_all(root.join("drafts")).unwrap();
        fs::write(root.join("note.txt"), "text").unwrap();
        fs::write(root.join("readme.md"), "markdown").unwrap();
        fs::write(root.join("image.png"), "image").unwrap();
        fs::write(root.join(".hidden.txt"), "hidden").unwrap();
        let current = root.join("current.txt");

        let prompt = FilePrompt::open(Some(&current));
        let names: Vec<String> = prompt
            .candidates
            .iter()
            .map(|candidate| {
                candidate
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();

        assert_eq!(names, ["drafts", "note.txt", "readme.md"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn arrows_completion_and_directory_navigation_update_the_selection() {
        let root = temporary_directory();
        fs::create_dir_all(root.join("drafts")).unwrap();
        fs::write(root.join("drafts").join("nested.md"), "nested").unwrap();
        fs::write(root.join("alpha.txt"), "alpha").unwrap();
        let current = root.join("current.txt");
        let mut prompt = FilePrompt::open(Some(&current));

        assert!(prompt.candidates[0].is_dir);
        assert!(prompt.choose_open_target().is_none());
        assert!(prompt
            .input
            .ends_with(format!("drafts{MAIN_SEPARATOR}").as_str()));
        assert_eq!(prompt.candidates.len(), 1);
        assert_eq!(
            prompt.choose_open_target().unwrap(),
            root.join("drafts").join("nested.md")
        );

        prompt = FilePrompt::open(Some(&current));
        prompt.select_next();
        prompt.complete_selected();
        assert!(prompt.input.ends_with("alpha.txt"));

        fs::remove_dir_all(root).unwrap();
    }
}
