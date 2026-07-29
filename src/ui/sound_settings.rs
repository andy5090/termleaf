//! State for the modal sound-settings panel.

const OPTION_COUNT: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SoundSettings {
    pub selected: usize,
}

impl SoundSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1) % OPTION_COUNT;
    }

    pub fn select_previous(&mut self) {
        self.selected = (self.selected + OPTION_COUNT - 1) % OPTION_COUNT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_wraps_in_both_directions() {
        let mut settings = SoundSettings::new();
        settings.select_previous();
        assert_eq!(settings.selected, 3);
        settings.select_next();
        assert_eq!(settings.selected, 0);
    }
}
