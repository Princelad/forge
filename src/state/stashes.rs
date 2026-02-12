//! Stashes page state.
//!
//! Manages stash list navigation and cached data.

use crate::pages::stashes::StashInfo;

/// State for the Stashes view.
#[derive(Debug, Clone, Default)]
pub struct StashesState {
    /// Currently selected stash index.
    pub selected_index: usize,
    /// Scroll offset for stash list.
    pub scroll: usize,
    /// Cached list of stashes.
    pub cached_stashes: Vec<StashInfo>,
}

impl StashesState {
    /// Creates a new stashes state with default values.
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            scroll: 0,
            cached_stashes: Vec::new(),
        }
    }

    /// Navigates to the previous stash.
    ///
    /// Returns `true` if the selection changed.
    pub fn navigate_up(&mut self) -> bool {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    /// Navigates to the next stash.
    ///
    /// Returns `true` if the selection changed.
    pub fn navigate_down(&mut self) -> bool {
        let max_index = self.cached_stashes.len().saturating_sub(1);
        if self.selected_index < max_index {
            self.selected_index += 1;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    /// Updates the cached stashes and resets selection.
    pub fn update_stashes(&mut self, stashes: Vec<StashInfo>) {
        self.cached_stashes = stashes;
        self.selected_index = 0;
        self.scroll = 0;
    }

    /// Ensures the current selection is visible within the scroll window.
    fn ensure_visible(&mut self) {
        const WINDOW_SIZE: usize = 10;
        if self.selected_index < self.scroll {
            self.scroll = self.selected_index;
        } else if self.selected_index >= self.scroll + WINDOW_SIZE {
            self.scroll = self.selected_index.saturating_sub(WINDOW_SIZE - 1);
        }
    }

    /// Resets selection to valid range.
    pub fn clamp_selection(&mut self) {
        self.selected_index = self
            .selected_index
            .min(self.cached_stashes.len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_stashes() -> Vec<StashInfo> {
        vec![
            StashInfo {
                index: 0,
                name: "WIP on main: add feature".to_string(),
                oid: "abc123".to_string(),
            },
            StashInfo {
                index: 1,
                name: "WIP on main: refactor".to_string(),
                oid: "def456".to_string(),
            },
        ]
    }

    #[test]
    fn test_new_default_values() {
        let state = StashesState::new();
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.scroll, 0);
        assert!(state.cached_stashes.is_empty());
    }

    #[test]
    fn test_navigate_up() {
        let mut state = StashesState::new();
        state.cached_stashes = sample_stashes();
        state.selected_index = 1;

        assert!(state.navigate_up());
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_navigate_down() {
        let mut state = StashesState::new();
        state.cached_stashes = sample_stashes();

        assert!(state.navigate_down());
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_update_stashes_resets_selection() {
        let mut state = StashesState::new();
        state.selected_index = 1;
        state.scroll = 3;

        state.update_stashes(sample_stashes());
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.scroll, 0);
        assert_eq!(state.cached_stashes.len(), 2);
    }
}
