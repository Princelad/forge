//! Merge visualizer page state.
//!
//! Manages 3-pane conflict resolution state.

use std::collections::HashMap;

use crate::pages::merge_visualizer::MergePaneFocus;

/// State for the Merge Visualizer view.
///
/// Handles navigation between panes (Files, Local, Incoming) and resolution tracking.
#[derive(Debug, Clone, Default)]
pub struct MergeState {
    /// Currently selected file index in the conflicts list.
    pub selected_file_index: usize,
    /// Currently focused pane (Files, Local, or Incoming).
    pub focus: MergePaneFocus,
    /// Scroll offset for the file list.
    pub scroll: usize,
    /// Map of (project_index, file_index) -> accepted pane for resolutions.
    pub resolutions: HashMap<(usize, usize), MergePaneFocus>,
}

impl MergeState {
    /// Creates a new merge state with default values.
    pub fn new() -> Self {
        Self {
            selected_file_index: 0,
            focus: MergePaneFocus::Files,
            scroll: 0,
            resolutions: HashMap::new(),
        }
    }

    /// Navigates to the previous file in the conflicts list.
    ///
    /// Returns `true` if the selection changed.
    pub fn navigate_up(&mut self) -> bool {
        if self.selected_file_index > 0 {
            self.selected_file_index -= 1;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    /// Navigates to the next file in the conflicts list.
    ///
    /// Returns `true` if the selection changed.
    pub fn navigate_down(&mut self, max_items: usize) -> bool {
        let max_index = max_items.saturating_sub(1);
        if self.selected_file_index < max_index {
            self.selected_file_index += 1;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    /// Cycles focus to the next pane (Files → Local → Incoming → Files).
    pub fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    /// Cycles focus to the previous pane (Files → Incoming → Local → Files).
    pub fn focus_prev(&mut self) {
        self.focus = self.focus.prev();
    }

    /// Scrolls up by the specified amount.
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    /// Scrolls down by the specified amount, respecting the maximum.
    pub fn scroll_down(&mut self, amount: usize, max_items: usize, window_size: usize) {
        if max_items > window_size {
            self.scroll = (self.scroll + amount).min(max_items - window_size);
        }
    }

    /// Accepts the current pane's version for the specified project and file.
    ///
    /// # Arguments
    /// * `project_index` - The project index
    ///
    /// # Returns
    /// A status message describing what was accepted, or `None` if Files pane is focused.
    pub fn accept_current_pane(&mut self, project_index: usize) -> Option<&'static str> {
        match self.focus {
            MergePaneFocus::Files => None,
            MergePaneFocus::Local => {
                self.resolutions
                    .insert((project_index, self.selected_file_index), self.focus);
                Some("Accepted local version")
            }
            MergePaneFocus::Incoming => {
                self.resolutions
                    .insert((project_index, self.selected_file_index), self.focus);
                Some("Accepted incoming version")
            }
        }
    }

    /// Gets the accepted resolution for a specific file, if any.
    pub fn get_resolution(
        &self,
        project_index: usize,
        file_index: usize,
    ) -> Option<MergePaneFocus> {
        self.resolutions.get(&(project_index, file_index)).copied()
    }

    /// Clears all resolutions.
    pub fn clear_resolutions(&mut self) {
        self.resolutions.clear();
    }

    /// Ensures the current selection is visible within the scroll window.
    fn ensure_visible(&mut self) {
        const WINDOW_SIZE: usize = 10;
        if self.selected_file_index < self.scroll {
            self.scroll = self.selected_file_index;
        } else if self.selected_file_index >= self.scroll + WINDOW_SIZE {
            self.scroll = self.selected_file_index.saturating_sub(WINDOW_SIZE - 1);
        }
    }

    /// Resets selection to valid range for the given item count.
    pub fn clamp_selection(&mut self, max_items: usize) {
        self.selected_file_index = self.selected_file_index.min(max_items.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default_values() {
        let state = MergeState::new();
        assert_eq!(state.selected_file_index, 0);
        assert_eq!(state.focus, MergePaneFocus::Files);
        assert_eq!(state.scroll, 0);
        assert!(state.resolutions.is_empty());
    }

    #[test]
    fn test_navigate_up() {
        let mut state = MergeState {
            selected_file_index: 3,
            ..Default::default()
        };
        assert!(state.navigate_up());
        assert_eq!(state.selected_file_index, 2);
    }

    #[test]
    fn test_navigate_up_at_top() {
        let mut state = MergeState::new();
        assert!(!state.navigate_up());
        assert_eq!(state.selected_file_index, 0);
    }

    #[test]
    fn test_navigate_down() {
        let mut state = MergeState {
            selected_file_index: 3,
            ..Default::default()
        };
        assert!(state.navigate_down(10));
        assert_eq!(state.selected_file_index, 4);
    }

    #[test]
    fn test_navigate_down_at_bottom() {
        let mut state = MergeState {
            selected_file_index: 9,
            ..Default::default()
        };
        assert!(!state.navigate_down(10));
        assert_eq!(state.selected_file_index, 9);
    }

    #[test]
    fn test_focus_next() {
        let mut state = MergeState::new();

        assert_eq!(state.focus, MergePaneFocus::Files);
        state.focus_next();
        assert_eq!(state.focus, MergePaneFocus::Local);
        state.focus_next();
        assert_eq!(state.focus, MergePaneFocus::Incoming);
        state.focus_next();
        assert_eq!(state.focus, MergePaneFocus::Files);
    }

    #[test]
    fn test_focus_prev() {
        let mut state = MergeState::new();

        assert_eq!(state.focus, MergePaneFocus::Files);
        state.focus_prev();
        assert_eq!(state.focus, MergePaneFocus::Incoming);
        state.focus_prev();
        assert_eq!(state.focus, MergePaneFocus::Local);
        state.focus_prev();
        assert_eq!(state.focus, MergePaneFocus::Files);
    }

    #[test]
    fn test_accept_local() {
        let mut state = MergeState {
            selected_file_index: 2,
            focus: MergePaneFocus::Local,
            ..Default::default()
        };

        let result = state.accept_current_pane(0);
        assert_eq!(result, Some("Accepted local version"));
        assert_eq!(state.get_resolution(0, 2), Some(MergePaneFocus::Local));
    }

    #[test]
    fn test_accept_incoming() {
        let mut state = MergeState {
            selected_file_index: 3,
            focus: MergePaneFocus::Incoming,
            ..Default::default()
        };

        let result = state.accept_current_pane(1);
        assert_eq!(result, Some("Accepted incoming version"));
        assert_eq!(state.get_resolution(1, 3), Some(MergePaneFocus::Incoming));
    }

    #[test]
    fn test_accept_files_pane_does_nothing() {
        let mut state = MergeState::new();

        let result = state.accept_current_pane(0);
        assert!(result.is_none());
        assert!(state.resolutions.is_empty());
    }

    #[test]
    fn test_clear_resolutions() {
        let mut state = MergeState::new();
        state.resolutions.insert((0, 0), MergePaneFocus::Local);
        state.resolutions.insert((0, 1), MergePaneFocus::Incoming);

        state.clear_resolutions();
        assert!(state.resolutions.is_empty());
    }

    #[test]
    fn test_clamp_selection() {
        let mut state = MergeState {
            selected_file_index: 15,
            ..Default::default()
        };
        state.clamp_selection(10);
        assert_eq!(state.selected_file_index, 9);
    }
}
