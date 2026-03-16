//! Core suggestion engine types and trait.
//!
//! This module defines:
//! - [`SuggestionConfig`]: configuration carried in `AppSettings`
//! - [`CommitSuggestion`]: a single candidate commit message
//! - [`SuggestionEngine`]: the trait that all engine implementations satisfy
//! - [`RuleBasedEngine`]: the heuristic implementation (logic in Phase 2)

use serde::{Deserialize, Serialize};

use super::context::CommitContext;

/// Configuration for the commit message suggestion engine.
///
/// Stored in `AppSettings` with `#[serde(default)]` so that existing
/// config files without this field continue to load without error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionConfig {
    /// Whether the suggestion engine is enabled.
    pub enabled: bool,
    /// Maximum number of suggestions to generate per analysis.
    pub max_suggestions: usize,
    /// Maximum character length for a generated commit subject line.
    pub max_length: usize,
}

impl Default for SuggestionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_suggestions: 3,
            max_length: 72,
        }
    }
}

/// A single candidate commit message produced by the suggestion engine.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitSuggestion {
    /// Conventional commit type, e.g. `"feat"`, `"fix"`, `"docs"`.
    pub commit_type: String,
    /// Optional scope derived from directory names or file prefixes.
    pub scope: Option<String>,
    /// Human-readable commit subject (excluding the type/scope prefix).
    pub message: String,
    /// Engine confidence in this suggestion, in the range `0.0..=1.0`.
    pub confidence: f32,
}

impl CommitSuggestion {
    /// Formats the suggestion as a conventional commit subject line.
    ///
    /// # Examples
    ///
    /// ```
    /// // "feat(auth): add login endpoint"
    /// // "fix: correct null check"
    /// ```
    pub fn formatted(&self) -> String {
        match &self.scope {
            Some(scope) => format!("{}({}): {}", self.commit_type, scope, self.message),
            None => format!("{}: {}", self.commit_type, self.message),
        }
    }
}

/// Interface for commit message suggestion engines.
///
/// Implementing this trait enables unit testing with mock engines
/// and future swapping between heuristic and ML backends.
pub trait SuggestionEngine {
    /// Analyses the given context and returns ranked commit message candidates.
    ///
    /// Returns an empty `Vec` when there is insufficient context to suggest.
    fn suggest(&self, context: &CommitContext, config: &SuggestionConfig) -> Vec<CommitSuggestion>;
}

/// Rule-based heuristic implementation of [`SuggestionEngine`].
///
/// Phase 2 will implement diff analysis and rule matching.
/// Phase 1 provides only the scaffolding.
#[derive(Debug, Default)]
pub struct RuleBasedEngine;

impl RuleBasedEngine {
    /// Creates a new rule-based engine.
    pub fn new() -> Self {
        Self
    }
}

impl SuggestionEngine for RuleBasedEngine {
    fn suggest(
        &self,
        _context: &CommitContext,
        _config: &SuggestionConfig,
    ) -> Vec<CommitSuggestion> {
        // Phase 2: implement rule matching against context.diff_summary and context.branch_name
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suggestions::diff::DiffSummary;

    #[test]
    fn test_commit_suggestion_formatted_with_scope() {
        let s = CommitSuggestion {
            commit_type: "feat".to_string(),
            scope: Some("auth".to_string()),
            message: "add login".to_string(),
            confidence: 0.9,
        };
        assert_eq!(s.formatted(), "feat(auth): add login");
    }

    #[test]
    fn test_commit_suggestion_formatted_without_scope() {
        let s = CommitSuggestion {
            commit_type: "fix".to_string(),
            scope: None,
            message: "correct null check".to_string(),
            confidence: 0.7,
        };
        assert_eq!(s.formatted(), "fix: correct null check");
    }

    #[test]
    fn test_suggestion_config_defaults() {
        let config = SuggestionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_suggestions, 3);
        assert_eq!(config.max_length, 72);
    }

    #[test]
    fn test_suggestion_config_serde_roundtrip() {
        let config = SuggestionConfig::default();
        let json = serde_json::to_string(&config).expect("serialize");
        let restored: SuggestionConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.enabled, config.enabled);
        assert_eq!(restored.max_suggestions, config.max_suggestions);
        assert_eq!(restored.max_length, config.max_length);
    }

    #[test]
    fn test_rule_based_engine_returns_empty_for_empty_context() {
        let engine = RuleBasedEngine::new();
        let context = CommitContext::new(DiffSummary::empty(), None);
        let suggestions = engine.suggest(&context, &SuggestionConfig::default());
        assert!(suggestions.is_empty());
    }
}
