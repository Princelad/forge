//! Diff summary extraction for the suggestion engine.
//!
//! `DiffSummary` is a reduced view of staged changes:
//! file paths and line counts. Phase 2 will implement full
//! diff parsing from `change.diff_preview`.

use crate::data::Change;

/// A compact summary of staged changes used as engine input.
#[derive(Debug, Clone, Default)]
pub struct DiffSummary {
    /// Paths of all staged files.
    pub files_changed: Vec<String>,
    /// Total added lines across all staged files.
    pub lines_added: usize,
    /// Total removed lines across all staged files.
    pub lines_removed: usize,
}

impl DiffSummary {
    /// Creates an empty diff summary.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builds a summary from a slice of changes.
    ///
    /// Only staged files are included. Line counts are left at zero
    /// until Phase 2 implements `diff_preview` parsing.
    pub fn from_changes(changes: &[Change]) -> Self {
        let files_changed = changes
            .iter()
            .filter(|c| c.staged)
            .map(|c| c.path.clone())
            .collect();

        Self {
            files_changed,
            lines_added: 0,
            lines_removed: 0,
        }
    }

    /// Returns `true` if no staged files are present.
    pub fn is_empty(&self) -> bool {
        self.files_changed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Change, FileStatus};

    fn make_change(path: &str, staged: bool) -> Change {
        Change {
            path: path.to_string(),
            status: FileStatus::Modified,
            diff_preview: String::new(),
            local_preview: None,
            incoming_preview: None,
            staged,
        }
    }

    #[test]
    fn test_diff_summary_default_is_empty() {
        let summary = DiffSummary::default();
        assert!(summary.is_empty());
        assert_eq!(summary.lines_added, 0);
        assert_eq!(summary.lines_removed, 0);
    }

    #[test]
    fn test_from_changes_filters_staged_only() {
        let changes = vec![
            make_change("src/main.rs", true),
            make_change("README.md", false),
        ];
        let summary = DiffSummary::from_changes(&changes);
        assert_eq!(summary.files_changed.len(), 1);
        assert_eq!(summary.files_changed[0], "src/main.rs");
    }

    #[test]
    fn test_from_changes_empty_slice() {
        let summary = DiffSummary::from_changes(&[]);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_from_changes_no_staged_files() {
        let changes = vec![make_change("foo.rs", false)];
        let summary = DiffSummary::from_changes(&changes);
        assert!(summary.is_empty());
    }
}
