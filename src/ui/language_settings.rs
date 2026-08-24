//! State for the install/select/remove language panel.

use crate::language::Language;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSettings {
    pub selected: usize,
    pub status: Option<String>,
}

impl LanguageSettings {
    pub fn new(active: &str) -> Self {
        let selected = Language::ALL
            .iter()
            .position(|language| language.code() == active)
            .unwrap_or(0);
        Self {
            selected,
            status: None,
        }
    }

    pub fn selected_language(&self) -> Language {
        Language::ALL[self.selected]
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1) % Language::ALL.len();
        self.status = None;
    }

    pub fn select_previous(&mut self) {
        self.selected = (self.selected + Language::ALL.len() - 1) % Language::ALL.len();
        self.status = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_on_active_language_and_wraps() {
        let mut settings = LanguageSettings::new("ja");
        assert_eq!(settings.selected_language(), Language::Japanese);
        settings.select_next();
        assert_eq!(settings.selected_language(), Language::English);
        settings.select_previous();
        assert_eq!(settings.selected_language(), Language::Japanese);
    }
}
