//! Commit context aggregation.
//!
//! `CommitContext` bundles all inputs to the suggestion engine:
//! the staged diff summary and the current Git branch name.

use crate::suggestions::diff::DiffSummary;

/// All contextual information available to the suggestion engine.
///
/// Constructed from staged `Change` slices and the optional branch name
/// before being passed to [`super::engine::SuggestionEngine::suggest`].
#[derive(Debug, Clone)]
pub struct CommitContext {
    /// Summary of staged file changes.
    pub diff_summary: DiffSummary,
    /// Current branch name, if known (e.g. `"feat/PROJ-42-add-login"`).
    pub branch_name: Option<String>,
}

impl CommitContext {
    /// Creates a new context from a diff summary and optional branch name.
    pub fn new(diff_summary: DiffSummary, branch_name: Option<String>) -> Self {
        Self {
            diff_summary,
            branch_name,
        }
    }

    /// Returns `true` if there are no staged files and no branch name.
    pub fn is_empty(&self) -> bool {
        self.diff_summary.is_empty() && self.branch_name.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_with_branch_is_not_empty() {
        let ctx = CommitContext::new(DiffSummary::empty(), Some("main".to_string()));
        assert!(!ctx.is_empty());
    }

    #[test]
    fn test_context_with_no_data_is_empty() {
        let ctx = CommitContext::new(DiffSummary::empty(), None);
        assert!(ctx.is_empty());
    }

    #[test]
    fn test_context_stores_branch_name() {
        let ctx = CommitContext::new(DiffSummary::empty(), Some("feat/login".to_string()));
        assert_eq!(ctx.branch_name.as_deref(), Some("feat/login"));
    }
}
