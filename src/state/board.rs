//! Project board (Kanban) page state.
//!
//! Manages Kanban board column and item navigation.

use crate::data::ModuleStatus;

/// State for the Project Board view (Kanban board).
///
/// Handles navigation between columns (Pending, Current, Completed) and items.
#[derive(Debug, Clone, Default)]
pub struct BoardState {
    /// Currently selected column (0=Pending, 1=Current, 2=Completed).
    pub selected_column: usize,
    /// Currently selected item within the column.
    pub selected_item: usize,
}

impl BoardState {
    /// Creates a new board state with default values.
    ///
    /// Defaults to Current column (index 1) for typical workflow.
    pub fn new() -> Self {
        Self {
            selected_column: 1, // Start in "Current" column
            selected_item: 0,
        }
    }

    /// Navigates to the previous item in the current column.
    ///
    /// Wraps to the last item if at the top.
    pub fn navigate_up(&mut self, column_len: usize) {
        if column_len == 0 {
            self.selected_item = 0;
        } else if self.selected_item > 0 {
            self.selected_item -= 1;
        } else {
            self.selected_item = column_len - 1;
        }
    }

    /// Navigates to the next item in the current column.
    ///
    /// Does not wrap at the bottom.
    pub fn navigate_down(&mut self, column_len: usize) {
        if column_len == 0 {
            self.selected_item = 0;
        } else if self.selected_item < column_len.saturating_sub(1) {
            self.selected_item += 1;
        }
    }

    /// Navigates to the previous column.
    ///
    /// Wraps from Pending (0) to Completed (2).
    /// Clamps the item selection to the new column's length.
    pub fn navigate_left(&mut self, new_column_len: usize) {
        if self.selected_column == 0 {
            self.selected_column = 2;
        } else {
            self.selected_column -= 1;
        }
        self.clamp_item_to_column(new_column_len);
    }

    /// Navigates to the next column.
    ///
    /// Wraps from Completed (2) to Pending (0).
    /// Clamps the item selection to the new column's length.
    pub fn navigate_right(&mut self, new_column_len: usize) {
        self.selected_column = (self.selected_column + 1) % 3;
        self.clamp_item_to_column(new_column_len);
    }

    /// Returns the `ModuleStatus` corresponding to the current column.
    pub fn current_status(&self) -> ModuleStatus {
        match self.selected_column {
            0 => ModuleStatus::Pending,
            1 => ModuleStatus::Current,
            _ => ModuleStatus::Completed,
        }
    }

    /// Returns the name of the current column.
    pub fn current_column_name(&self) -> &'static str {
        match self.selected_column {
            0 => "Pending",
            1 => "Current",
            _ => "Completed",
        }
    }

    /// Clamps the item selection to the new column's length.
    fn clamp_item_to_column(&mut self, column_len: usize) {
        self.selected_item = if column_len == 0 {
            0
        } else {
            self.selected_item.min(column_len - 1)
        };
    }

    /// Resets item selection to valid range for the given column length.
    pub fn clamp_selection(&mut self, column_len: usize) {
        self.clamp_item_to_column(column_len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default_values() {
        let state = BoardState::new();
        assert_eq!(state.selected_column, 1); // Current column
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn test_navigate_up_in_column() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 3,
        };
        state.navigate_up(5);
        assert_eq!(state.selected_item, 2);
    }

    #[test]
    fn test_navigate_up_wraps_to_bottom() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 0,
        };
        state.navigate_up(5);
        assert_eq!(state.selected_item, 4);
    }

    #[test]
    fn test_navigate_up_empty_column() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 0,
        };
        state.navigate_up(0);
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn test_navigate_down_in_column() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 2,
        };
        state.navigate_down(5);
        assert_eq!(state.selected_item, 3);
    }

    #[test]
    fn test_navigate_down_at_bottom() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 4,
        };
        state.navigate_down(5);
        assert_eq!(state.selected_item, 4); // Stays at bottom
    }

    #[test]
    fn test_navigate_down_empty_column() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 0,
        };
        state.navigate_down(0);
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn test_navigate_left() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 3,
        };
        state.navigate_left(2);
        assert_eq!(state.selected_column, 0);
        assert_eq!(state.selected_item, 1); // Clamped to new column length
    }

    #[test]
    fn test_navigate_left_wraps() {
        let mut state = BoardState {
            selected_column: 0,
            selected_item: 0,
        };
        state.navigate_left(5);
        assert_eq!(state.selected_column, 2);
    }

    #[test]
    fn test_navigate_right() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 3,
        };
        state.navigate_right(2);
        assert_eq!(state.selected_column, 2);
        assert_eq!(state.selected_item, 1); // Clamped to new column length
    }

    #[test]
    fn test_navigate_right_wraps() {
        let mut state = BoardState {
            selected_column: 2,
            selected_item: 0,
        };
        state.navigate_right(5);
        assert_eq!(state.selected_column, 0);
    }

    #[test]
    fn test_current_status() {
        let mut state = BoardState::new();

        state.selected_column = 0;
        assert_eq!(state.current_status(), ModuleStatus::Pending);

        state.selected_column = 1;
        assert_eq!(state.current_status(), ModuleStatus::Current);

        state.selected_column = 2;
        assert_eq!(state.current_status(), ModuleStatus::Completed);
    }

    #[test]
    fn test_current_column_name() {
        let mut state = BoardState::new();

        state.selected_column = 0;
        assert_eq!(state.current_column_name(), "Pending");

        state.selected_column = 1;
        assert_eq!(state.current_column_name(), "Current");

        state.selected_column = 2;
        assert_eq!(state.current_column_name(), "Completed");
    }

    #[test]
    fn test_clamp_selection() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 10,
        };
        state.clamp_selection(3);
        assert_eq!(state.selected_item, 2);
    }

    #[test]
    fn test_clamp_selection_empty() {
        let mut state = BoardState {
            selected_column: 1,
            selected_item: 5,
        };
        state.clamp_selection(0);
        assert_eq!(state.selected_item, 0);
    }
}
