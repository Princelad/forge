//! Commit context aggregation.
//!
//! `CommitContext` bundles all inputs to the suggestion engine:
//! the staged diff summary and branch-derived metadata.

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::suggestions::diff::DiffSummary;
use regex::Regex;

static ISSUE_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b([a-z][a-z0-9]{1,9}-\d+)\b").expect("issue key regex"));

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
    /// Distinct issue keys extracted from the branch name (e.g. `"PROJ-42"`).
    pub issue_keys: Vec<String>,
    /// First extracted issue key for convenience in suggestion templates.
    pub primary_issue_key: Option<String>,
}

impl CommitContext {
    /// Creates a new context from a diff summary and optional branch name.
    pub fn new(diff_summary: DiffSummary, branch_name: Option<String>) -> Self {
        let issue_keys = branch_name
            .as_deref()
            .map_or_else(Vec::new, Self::extract_issue_keys);
        let primary_issue_key = issue_keys.first().cloned();

        Self {
            diff_summary,
            branch_name,
            issue_keys,
            primary_issue_key,
        }
    }

    /// Extracts normalized issue keys from a branch name.
    ///
    /// Supported pattern: `<PROJECT>-<NUMBER>` where `<PROJECT>` is 2-10
    /// alphanumeric chars starting with a letter (case-insensitive).
    pub fn extract_issue_keys(branch_name: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut keys = Vec::new();

        for cap in ISSUE_KEY_RE.captures_iter(branch_name) {
            let Some(raw_key) = cap.get(1) else {
                continue;
            };
            let normalized = raw_key.as_str().to_uppercase();
            if seen.insert(normalized.clone()) {
                keys.push(normalized);
            }
        }

        keys
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

    #[test]
    fn test_context_extracts_issue_key_from_branch() {
        let ctx = CommitContext::new(
            DiffSummary::empty(),
            Some("feat/proj-42-add-login".to_string()),
        );
        assert_eq!(ctx.issue_keys, vec!["PROJ-42".to_string()]);
        assert_eq!(ctx.primary_issue_key.as_deref(), Some("PROJ-42"));
    }

    #[test]
    fn test_context_extracts_multiple_distinct_issue_keys() {
        let ctx = CommitContext::new(
            DiffSummary::empty(),
            Some("chore/abc-1-def-2-abc-1-cleanup".to_string()),
        );
        assert_eq!(
            ctx.issue_keys,
            vec!["ABC-1".to_string(), "DEF-2".to_string()]
        );
        assert_eq!(ctx.primary_issue_key.as_deref(), Some("ABC-1"));
    }

    #[test]
    fn test_context_handles_branch_without_issue_key() {
        let ctx = CommitContext::new(DiffSummary::empty(), Some("feat/refactor-auth".to_string()));
        assert!(ctx.issue_keys.is_empty());
        assert!(ctx.primary_issue_key.is_none());
    }

    #[test]
    fn test_extract_issue_keys_empty_for_main_branch() {
        let keys = CommitContext::extract_issue_keys("main");
        assert!(keys.is_empty());
    }
}
