//! Changes page state.
//!
//! Manages Git staging interface and commit message input.

/// State for the Changes view (Git staging/commit interface).
///
/// Handles file selection, staging status, and commit message composition.
#[derive(Debug, Clone, Default)]
pub struct ChangesState {
    /// Currently selected file index in the changes list.
    pub selected_index: usize,
    /// Scroll offset for the changes list.
    pub scroll: usize,
    /// Commit message being composed.
    pub commit_message: String,
    /// Pane ratio for changes list (percentage).
    pub changes_pane_ratio: u16,
    /// Pane ratio for commit message area (percentage).
    pub commit_pane_ratio: u16,
}

impl ChangesState {
    /// Creates a new changes state with default values.
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            scroll: 0,
            commit_message: String::new(),
            changes_pane_ratio: 35,
            commit_pane_ratio: 50,
        }
    }

    /// Navigates to the previous file in the changes list.
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

    /// Navigates to the next file in the changes list.
    ///
    /// Returns `true` if the selection changed.
    pub fn navigate_down(&mut self, max_items: usize) -> bool {
        let max_index = max_items.saturating_sub(1);
        if self.selected_index < max_index {
            self.selected_index += 1;
            self.ensure_visible();
            true
        } else {
            false
        }
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

    /// Appends a character to the commit message.
    pub fn append_commit_char(&mut self, c: char) {
        self.commit_message.push(c);
    }

    /// Removes the last character from the commit message.
    ///
    /// Returns `true` if a character was removed.
    pub fn pop_commit_char(&mut self) -> bool {
        self.commit_message.pop().is_some()
    }

    /// Clears the commit message.
    pub fn clear_commit_message(&mut self) {
        self.commit_message.clear();
    }

    /// Returns `true` if the commit message is empty or whitespace-only.
    pub fn is_commit_message_empty(&self) -> bool {
        self.commit_message.trim().is_empty()
    }

    /// Adjusts the changes pane ratio.
    ///
    /// # Returns
    /// The new ratio value.
    pub fn adjust_changes_pane_ratio(&mut self, delta: i16) -> u16 {
        let new_ratio = (self.changes_pane_ratio as i16 + delta).clamp(10, 90) as u16;
        self.changes_pane_ratio = new_ratio;
        new_ratio
    }

    /// Adjusts the commit pane ratio.
    ///
    /// # Returns
    /// The new ratio value.
    pub fn adjust_commit_pane_ratio(&mut self, delta: i16) -> u16 {
        let new_ratio = (self.commit_pane_ratio as i16 + delta).clamp(10, 90) as u16;
        self.commit_pane_ratio = new_ratio;
        new_ratio
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

    /// Resets selection to valid range for the given item count.
    pub fn clamp_selection(&mut self, max_items: usize) {
        self.selected_index = self.selected_index.min(max_items.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default_values() {
        let state = ChangesState::new();
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.scroll, 0);
        assert!(state.commit_message.is_empty());
        assert_eq!(state.changes_pane_ratio, 35);
        assert_eq!(state.commit_pane_ratio, 50);
    }

    #[test]
    fn test_navigate_up() {
        let mut state = ChangesState {
            selected_index: 3,
            ..Default::default()
        };
        assert!(state.navigate_up());
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_navigate_up_at_top() {
        let mut state = ChangesState::new();
        assert!(!state.navigate_up());
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_navigate_down() {
        let mut state = ChangesState {
            selected_index: 3,
            ..Default::default()
        };
        assert!(state.navigate_down(10));
        assert_eq!(state.selected_index, 4);
    }

    #[test]
    fn test_navigate_down_at_bottom() {
        let mut state = ChangesState {
            selected_index: 9,
            ..Default::default()
        };
        assert!(!state.navigate_down(10));
        assert_eq!(state.selected_index, 9);
    }

    #[test]
    fn test_commit_message_operations() {
        let mut state = ChangesState::new();

        // Initially empty
        assert!(state.is_commit_message_empty());

        // Append characters
        state.append_commit_char('H');
        state.append_commit_char('i');
        assert_eq!(state.commit_message, "Hi");
        assert!(!state.is_commit_message_empty());

        // Pop character
        assert!(state.pop_commit_char());
        assert_eq!(state.commit_message, "H");

        // Clear
        state.clear_commit_message();
        assert!(state.is_commit_message_empty());
    }

    #[test]
    fn test_whitespace_only_is_empty() {
        let mut state = ChangesState::new();
        state.commit_message = "   \t\n  ".to_string();
        assert!(state.is_commit_message_empty());
    }

    #[test]
    fn test_adjust_changes_pane_ratio() {
        let mut state = ChangesState::new();

        let ratio = state.adjust_changes_pane_ratio(10);
        assert_eq!(ratio, 45);

        let ratio = state.adjust_changes_pane_ratio(-20);
        assert_eq!(ratio, 25);
    }

    #[test]
    fn test_adjust_commit_pane_ratio() {
        let mut state = ChangesState::new();

        let ratio = state.adjust_commit_pane_ratio(10);
        assert_eq!(ratio, 60);

        let ratio = state.adjust_commit_pane_ratio(-30);
        assert_eq!(ratio, 30);
    }

    #[test]
    fn test_pane_ratio_clamps() {
        let mut state = ChangesState {
            changes_pane_ratio: 15,
            commit_pane_ratio: 85,
            ..Default::default()
        };

        assert_eq!(state.adjust_changes_pane_ratio(-10), 10);
        assert_eq!(state.adjust_commit_pane_ratio(10), 90);
    }

    #[test]
    fn test_clamp_selection() {
        let mut state = ChangesState {
            selected_index: 15,
            ..Default::default()
        };
        state.clamp_selection(10);
        assert_eq!(state.selected_index, 9);
    }
}
