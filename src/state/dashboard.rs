//! Dashboard page state.
//!
//! Manages project list navigation and filtering.

/// State for the Dashboard view.
///
/// The dashboard displays a list of projects and allows navigation/selection.
#[derive(Debug, Clone, Default)]
pub struct DashboardState {
    /// Currently selected project index in the project list.
    pub selected_index: usize,
    /// Scroll offset for the project list (for windowed display).
    pub scroll: usize,
    /// Pane ratio for dashboard layout (percentage).
    pub pane_ratio: u16,
}

impl DashboardState {
    /// Creates a new dashboard state with default values.
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            scroll: 0,
            pane_ratio: 30,
        }
    }

    /// Navigates to the previous project in the list.
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

    /// Navigates to the next project in the list.
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

    /// Adjusts the pane ratio.
    ///
    /// # Arguments
    /// * `delta` - The amount to adjust (positive widens, negative narrows)
    ///
    /// # Returns
    /// The new ratio value.
    pub fn adjust_pane_ratio(&mut self, delta: i16) -> u16 {
        let new_ratio = (self.pane_ratio as i16 + delta).clamp(10, 90) as u16;
        self.pane_ratio = new_ratio;
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
        let state = DashboardState::new();
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.scroll, 0);
        assert_eq!(state.pane_ratio, 30);
    }

    #[test]
    fn test_navigate_up_at_top() {
        let mut state = DashboardState::new();
        assert!(!state.navigate_up());
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_navigate_up_from_middle() {
        let mut state = DashboardState {
            selected_index: 5,
            ..Default::default()
        };
        assert!(state.navigate_up());
        assert_eq!(state.selected_index, 4);
    }

    #[test]
    fn test_navigate_down_at_bottom() {
        let mut state = DashboardState {
            selected_index: 9,
            ..Default::default()
        };
        assert!(!state.navigate_down(10));
        assert_eq!(state.selected_index, 9);
    }

    #[test]
    fn test_navigate_down_from_middle() {
        let mut state = DashboardState {
            selected_index: 5,
            ..Default::default()
        };
        assert!(state.navigate_down(10));
        assert_eq!(state.selected_index, 6);
    }

    #[test]
    fn test_scroll_up() {
        let mut state = DashboardState {
            scroll: 5,
            ..Default::default()
        };
        state.scroll_up(3);
        assert_eq!(state.scroll, 2);
    }

    #[test]
    fn test_scroll_up_saturates() {
        let mut state = DashboardState {
            scroll: 2,
            ..Default::default()
        };
        state.scroll_up(5);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_scroll_down() {
        let mut state = DashboardState::new();
        state.scroll_down(3, 20, 10);
        assert_eq!(state.scroll, 3);
    }

    #[test]
    fn test_scroll_down_clamps_to_max() {
        let mut state = DashboardState {
            scroll: 8,
            ..Default::default()
        };
        state.scroll_down(5, 15, 10);
        assert_eq!(state.scroll, 5); // max is 15 - 10 = 5
    }

    #[test]
    fn test_adjust_pane_ratio_widen() {
        let mut state = DashboardState {
            pane_ratio: 30,
            ..Default::default()
        };
        let new_ratio = state.adjust_pane_ratio(10);
        assert_eq!(new_ratio, 40);
        assert_eq!(state.pane_ratio, 40);
    }

    #[test]
    fn test_adjust_pane_ratio_narrow() {
        let mut state = DashboardState {
            pane_ratio: 30,
            ..Default::default()
        };
        let new_ratio = state.adjust_pane_ratio(-10);
        assert_eq!(new_ratio, 20);
        assert_eq!(state.pane_ratio, 20);
    }

    #[test]
    fn test_adjust_pane_ratio_clamps_min() {
        let mut state = DashboardState {
            pane_ratio: 15,
            ..Default::default()
        };
        let new_ratio = state.adjust_pane_ratio(-10);
        assert_eq!(new_ratio, 10);
    }

    #[test]
    fn test_adjust_pane_ratio_clamps_max() {
        let mut state = DashboardState {
            pane_ratio: 85,
            ..Default::default()
        };
        let new_ratio = state.adjust_pane_ratio(10);
        assert_eq!(new_ratio, 90);
    }

    #[test]
    fn test_clamp_selection() {
        let mut state = DashboardState {
            selected_index: 10,
            ..Default::default()
        };
        state.clamp_selection(5);
        assert_eq!(state.selected_index, 4);
    }

    #[test]
    fn test_clamp_selection_empty_list() {
        let mut state = DashboardState {
            selected_index: 5,
            ..Default::default()
        };
        state.clamp_selection(0);
        assert_eq!(state.selected_index, 0);
    }
}
