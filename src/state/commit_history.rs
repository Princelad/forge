//! Commit history page state.
//!
//! Manages commit history list navigation and display.

use crate::pages::commit_history::CommitInfo;

/// State for the Commit History view.
///
/// Handles commit list navigation and cached commit data.
#[derive(Debug, Clone, Default)]
pub struct CommitHistoryState {
    /// Currently selected commit index.
    pub selected_index: usize,
    /// Scroll offset for commit list.
    pub scroll: usize,
    /// Cached list of commits.
    pub cached_commits: Vec<CommitInfo>,
}

impl CommitHistoryState {
    /// Creates a new commit history state with default values.
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            scroll: 0,
            cached_commits: Vec::new(),
        }
    }

    /// Navigates to the previous commit.
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

    /// Navigates to the next commit.
    ///
    /// Returns `true` if the selection changed.
    pub fn navigate_down(&mut self) -> bool {
        let max_index = self.cached_commits.len().saturating_sub(1);
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
    pub fn scroll_down(&mut self, amount: usize, window_size: usize) {
        let max_items = self.cached_commits.len();
        if max_items > window_size {
            self.scroll = (self.scroll + amount).min(max_items - window_size);
        }
    }

    /// Gets the currently selected commit, if any.
    pub fn selected_commit(&self) -> Option<&CommitInfo> {
        self.cached_commits.get(self.selected_index)
    }

    /// Updates the cached commits and resets selection.
    pub fn update_commits(&mut self, commits: Vec<CommitInfo>) {
        self.cached_commits = commits;
        self.selected_index = 0;
        self.scroll = 0;
    }

    /// Returns the number of cached commits.
    pub fn commit_count(&self) -> usize {
        self.cached_commits.len()
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
            .min(self.cached_commits.len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_commits() -> Vec<CommitInfo> {
        vec![
            CommitInfo {
                hash: "abc123".to_string(),
                author: "Alice".to_string(),
                date: "2026-01-27".to_string(),
                message: "Initial commit".to_string(),
                files_changed: vec!["file1.rs".to_string(), "file2.rs".to_string()],
            },
            CommitInfo {
                hash: "def456".to_string(),
                author: "Bob".to_string(),
                date: "2026-01-26".to_string(),
                message: "Add feature".to_string(),
                files_changed: vec!["src/main.rs".to_string()],
            },
            CommitInfo {
                hash: "ghi789".to_string(),
                author: "Charlie".to_string(),
                date: "2026-01-25".to_string(),
                message: "Fix bug".to_string(),
                files_changed: vec!["src/lib.rs".to_string()],
            },
        ]
    }

    #[test]
    fn test_new_default_values() {
        let state = CommitHistoryState::new();
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.scroll, 0);
        assert!(state.cached_commits.is_empty());
    }

    #[test]
    fn test_navigate_up() {
        let mut state = CommitHistoryState::new();
        state.cached_commits = sample_commits();
        state.selected_index = 2;
        
        assert!(state.navigate_up());
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_navigate_up_at_top() {
        let mut state = CommitHistoryState::new();
        state.cached_commits = sample_commits();
        
        assert!(!state.navigate_up());
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_navigate_down() {
        let mut state = CommitHistoryState::new();
        state.cached_commits = sample_commits();
        
        assert!(state.navigate_down());
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_navigate_down_at_bottom() {
        let mut state = CommitHistoryState::new();
        state.cached_commits = sample_commits();
        state.selected_index = 2;
        
        assert!(!state.navigate_down());
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_scroll_up() {
        let mut state = CommitHistoryState {
            scroll: 5,
            ..Default::default()
        };
        
        state.scroll_up(3);
        assert_eq!(state.scroll, 2);
    }

    #[test]
    fn test_scroll_up_saturates() {
        let mut state = CommitHistoryState {
            scroll: 2,
            ..Default::default()
        };
        
        state.scroll_up(5);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_scroll_down() {
        let mut state = CommitHistoryState::new();
        state.cached_commits = (0..20)
            .map(|i| CommitInfo {
                hash: format!("hash{}", i),
                author: "Author".to_string(),
                date: "2026-01-27".to_string(),
                message: format!("Commit {}", i),
                files_changed: vec![format!("file{}.rs", i)],
            })
            .collect();
        
        state.scroll_down(5, 10);
        assert_eq!(state.scroll, 5);
    }

    #[test]
    fn test_scroll_down_clamps() {
        let mut state = CommitHistoryState::new();
        state.cached_commits = (0..15)
            .map(|i| CommitInfo {
                hash: format!("hash{}", i),
                author: "Author".to_string(),
                date: "2026-01-27".to_string(),
                message: format!("Commit {}", i),
                files_changed: vec![format!("file{}.rs", i)],
            })
            .collect();
        state.scroll = 3;
        
        state.scroll_down(10, 10);
        assert_eq!(state.scroll, 5); // max is 15 - 10 = 5
    }

    #[test]
    fn test_selected_commit() {
        let mut state = CommitHistoryState::new();
        state.cached_commits = sample_commits();
        state.selected_index = 1;
        
        let commit = state.selected_commit().unwrap();
        assert_eq!(commit.hash, "def456");
        assert_eq!(commit.author, "Bob");
    }

    #[test]
    fn test_update_commits() {
        let mut state = CommitHistoryState::new();
        state.selected_index = 5;
        state.scroll = 3;
        
        state.update_commits(sample_commits());
        
        assert_eq!(state.commit_count(), 3);
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_clamp_selection() {
        let mut state = CommitHistoryState::new();
        state.cached_commits = sample_commits();
        state.selected_index = 10;
        
        state.clamp_selection();
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_clamp_selection_empty() {
        let mut state = CommitHistoryState::new();
        state.selected_index = 5;
        
        state.clamp_selection();
        assert_eq!(state.selected_index, 0);
    }
}
