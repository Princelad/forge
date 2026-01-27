//! Branch manager page state.
//!
//! Manages branch list navigation, creation, and operations.

use crate::pages::branch_manager::{BranchInfo, BranchManagerMode};

/// State for the Branch Manager view.
///
/// Handles branch list navigation, creation mode, and branch operations.
#[derive(Debug, Clone, Default)]
pub struct BranchManagerState {
    /// Current mode (list or create).
    pub mode: BranchManagerMode,
    /// Currently selected branch index.
    pub selected_index: usize,
    /// Input buffer for new branch name.
    pub input_buffer: String,
    /// Scroll offset for branch list.
    pub scroll: usize,
    /// Cached list of branches.
    pub cached_branches: Vec<BranchInfo>,
}

impl BranchManagerState {
    /// Creates a new branch manager state with default values.
    pub fn new() -> Self {
        Self {
            mode: BranchManagerMode::List,
            selected_index: 0,
            input_buffer: String::new(),
            scroll: 0,
            cached_branches: Vec::new(),
        }
    }

    /// Returns `true` if in branch creation mode.
    pub fn is_create_mode(&self) -> bool {
        matches!(self.mode, BranchManagerMode::CreateBranch)
    }

    /// Enters branch creation mode.
    pub fn enter_create_mode(&mut self) {
        self.mode = BranchManagerMode::CreateBranch;
        self.input_buffer.clear();
    }

    /// Exits branch creation mode back to list view.
    pub fn exit_create_mode(&mut self) {
        self.mode = BranchManagerMode::List;
        self.input_buffer.clear();
    }

    /// Navigates to the previous branch.
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

    /// Navigates to the next branch.
    ///
    /// Returns `true` if the selection changed.
    pub fn navigate_down(&mut self) -> bool {
        let max_index = self.cached_branches.len().saturating_sub(1);
        if self.selected_index < max_index {
            self.selected_index += 1;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    /// Appends a character to the input buffer.
    pub fn append_input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    /// Removes the last character from the input buffer.
    ///
    /// Returns `true` if a character was removed.
    pub fn pop_input_char(&mut self) -> bool {
        self.input_buffer.pop().is_some()
    }

    /// Clears the input buffer.
    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
    }

    /// Returns `true` if the input buffer is empty or whitespace-only.
    pub fn is_input_empty(&self) -> bool {
        self.input_buffer.trim().is_empty()
    }

    /// Gets the trimmed input value.
    pub fn get_input_value(&self) -> &str {
        self.input_buffer.trim()
    }

    /// Gets the currently selected branch, if any.
    pub fn selected_branch(&self) -> Option<&BranchInfo> {
        self.cached_branches.get(self.selected_index)
    }

    /// Gets the name of the currently selected branch, if any.
    pub fn selected_branch_name(&self) -> Option<&str> {
        self.selected_branch().map(|b| b.name.as_str())
    }

    /// Returns `true` if the currently selected branch is the current branch.
    pub fn is_selected_current(&self) -> bool {
        self.selected_branch().is_some_and(|b| b.is_current)
    }

    /// Updates the cached branches and resets selection.
    pub fn update_branches(&mut self, branches: Vec<BranchInfo>) {
        self.cached_branches = branches;
        self.selected_index = 0;
        self.scroll = 0;
    }

    /// Returns the number of cached branches.
    pub fn branch_count(&self) -> usize {
        self.cached_branches.len()
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
            .min(self.cached_branches.len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_branches() -> Vec<BranchInfo> {
        vec![
            BranchInfo {
                name: "main".to_string(),
                is_current: true,
                is_remote: false,
            },
            BranchInfo {
                name: "develop".to_string(),
                is_current: false,
                is_remote: false,
            },
            BranchInfo {
                name: "feature/test".to_string(),
                is_current: false,
                is_remote: false,
            },
        ]
    }

    #[test]
    fn test_new_default_values() {
        let state = BranchManagerState::new();
        assert!(matches!(state.mode, BranchManagerMode::List));
        assert_eq!(state.selected_index, 0);
        assert!(state.input_buffer.is_empty());
        assert!(state.cached_branches.is_empty());
    }

    #[test]
    fn test_create_mode_toggle() {
        let mut state = BranchManagerState::new();
        
        assert!(!state.is_create_mode());
        state.enter_create_mode();
        assert!(state.is_create_mode());
        state.exit_create_mode();
        assert!(!state.is_create_mode());
    }

    #[test]
    fn test_enter_create_mode_clears_input() {
        let mut state = BranchManagerState::new();
        state.input_buffer = "existing".to_string();
        
        state.enter_create_mode();
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn test_navigate_up() {
        let mut state = BranchManagerState::new();
        state.cached_branches = sample_branches();
        state.selected_index = 2;
        
        assert!(state.navigate_up());
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_navigate_up_at_top() {
        let mut state = BranchManagerState::new();
        state.cached_branches = sample_branches();
        
        assert!(!state.navigate_up());
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_navigate_down() {
        let mut state = BranchManagerState::new();
        state.cached_branches = sample_branches();
        
        assert!(state.navigate_down());
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_navigate_down_at_bottom() {
        let mut state = BranchManagerState::new();
        state.cached_branches = sample_branches();
        state.selected_index = 2;
        
        assert!(!state.navigate_down());
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_input_operations() {
        let mut state = BranchManagerState::new();
        
        assert!(state.is_input_empty());
        
        state.append_input_char('f');
        state.append_input_char('o');
        state.append_input_char('o');
        assert_eq!(state.input_buffer, "foo");
        assert!(!state.is_input_empty());
        
        assert!(state.pop_input_char());
        assert_eq!(state.input_buffer, "fo");
        
        state.clear_input();
        assert!(state.is_input_empty());
    }

    #[test]
    fn test_selected_branch() {
        let mut state = BranchManagerState::new();
        state.cached_branches = sample_branches();
        state.selected_index = 1;
        
        let branch = state.selected_branch().unwrap();
        assert_eq!(branch.name, "develop");
        assert_eq!(state.selected_branch_name(), Some("develop"));
    }

    #[test]
    fn test_is_selected_current() {
        let mut state = BranchManagerState::new();
        state.cached_branches = sample_branches();
        
        assert!(state.is_selected_current()); // index 0 is main (current)
        
        state.selected_index = 1;
        assert!(!state.is_selected_current()); // develop is not current
    }

    #[test]
    fn test_update_branches() {
        let mut state = BranchManagerState::new();
        state.selected_index = 5;
        state.scroll = 3;
        
        state.update_branches(sample_branches());
        
        assert_eq!(state.branch_count(), 3);
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_clamp_selection() {
        let mut state = BranchManagerState::new();
        state.cached_branches = sample_branches();
        state.selected_index = 10;
        
        state.clamp_selection();
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_clamp_selection_empty() {
        let mut state = BranchManagerState::new();
        state.selected_index = 5;
        
        state.clamp_selection();
        assert_eq!(state.selected_index, 0);
    }
}
