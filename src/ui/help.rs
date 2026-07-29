//! State for the startup guide and the reusable F1 help overlay.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpOverlay {
    /// Uses a welcome title when opened automatically on startup.
    pub welcome: bool,
    /// Checkbox value edited with Space and persisted when the overlay closes.
    pub hide_on_startup: bool,
}

impl HelpOverlay {
    pub fn welcome() -> Self {
        Self {
            welcome: true,
            hide_on_startup: false,
        }
    }

    pub fn help(show_welcome: bool) -> Self {
        Self {
            welcome: false,
            hide_on_startup: !show_welcome,
        }
    }

    pub fn toggle_startup_visibility(&mut self) {
        self.hide_on_startup = !self.hide_on_startup;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_is_visible_again_until_the_checkbox_is_selected() {
        let mut overlay = HelpOverlay::welcome();
        assert!(!overlay.hide_on_startup);
        overlay.toggle_startup_visibility();
        assert!(overlay.hide_on_startup);
    }

    #[test]
    fn f1_help_reflects_the_saved_startup_preference() {
        assert!(!HelpOverlay::help(true).hide_on_startup);
        assert!(HelpOverlay::help(false).hide_on_startup);
    }
}
