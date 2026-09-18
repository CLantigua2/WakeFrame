// Minimal tray state model used by tests and future tray icon rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayColor {
    Blue,
    Gray,
}

pub struct TrayState {
    pub enabled: bool,
}

impl TrayState {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn color(&self) -> TrayColor {
        if self.enabled {
            TrayColor::Blue
        } else {
            TrayColor::Gray
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TrayColor, TrayState};

    #[test]
    fn tray_state_toggle_changes_value() {
        let mut state = TrayState::new(true);
        state.toggle();
        assert!(!state.enabled);
    }

    #[test]
    fn tray_state_color_matches_enabled_state() {
        let enabled = TrayState::new(true);
        let disabled = TrayState::new(false);

        assert_eq!(enabled.color(), TrayColor::Blue);
        assert_eq!(disabled.color(), TrayColor::Gray);
    }
}
