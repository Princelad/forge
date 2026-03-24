//! Core suggestion engine types and trait.
//!
//! This module defines:
//! - [`SuggestionConfig`]: configuration carried in `AppSettings`
//! - [`CommitSuggestion`]: a single candidate commit message
//! - [`SuggestionEngine`]: the trait that all engine implementations satisfy
//! - [`RuleBasedEngine`]: the heuristic implementation

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::context::CommitContext;
use super::rules::{classify_path, detect_commit_type};

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
#[derive(Debug, Default)]
pub struct RuleBasedEngine;

impl RuleBasedEngine {
    /// Creates a new rule-based engine.
    pub fn new() -> Self {
        Self
    }
}

impl SuggestionEngine for RuleBasedEngine {
    fn suggest(&self, context: &CommitContext, config: &SuggestionConfig) -> Vec<CommitSuggestion> {
        if !config.enabled || context.diff_summary.is_empty() {
            return Vec::new();
        }

        let primary_type = detect_commit_type(&context.diff_summary.files_changed);
        let primary_scope = infer_primary_scope(context);
        let issue_key = context.primary_issue_key.as_deref();
        let change_verb = infer_change_verb(context);
        let common_subject = subject_for_type(
            &primary_type,
            primary_scope.as_deref(),
            change_verb,
            issue_key,
            context,
        );

        let mut candidates = vec![CommitSuggestion {
            commit_type: primary_type.clone(),
            scope: primary_scope.clone(),
            message: common_subject,
            confidence: 0.86,
        }];

        candidates.push(CommitSuggestion {
            commit_type: primary_type.clone(),
            scope: None,
            message: subject_for_type(&primary_type, None, change_verb, issue_key, context),
            confidence: 0.78,
        });

        if let Some(secondary_type) = detect_secondary_type(context, &primary_type) {
            candidates.push(CommitSuggestion {
                commit_type: secondary_type.clone(),
                scope: primary_scope.clone(),
                message: subject_for_type(
                    &secondary_type,
                    primary_scope.as_deref(),
                    change_verb,
                    issue_key,
                    context,
                ),
                confidence: 0.68,
            });
        }

        // Add a generic fallback to ensure at least one sensible suggestion.
        candidates.push(CommitSuggestion {
            commit_type: primary_type,
            scope: None,
            message: fallback_subject(change_verb, issue_key),
            confidence: 0.62,
        });

        let mut ranked = rank_and_dedup(candidates, config);
        ranked.truncate(config.max_suggestions.max(1));
        ranked
    }
}

fn infer_primary_scope(context: &CommitContext) -> Option<String> {
    if context.diff_summary.file_details.is_empty() {
        return context.diff_summary.scopes.first().cloned();
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    for detail in &context.diff_summary.file_details {
        if let Some(scope) = &detail.scope {
            *counts.entry(scope.clone()).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(scope, _)| scope)
}

fn infer_change_verb(context: &CommitContext) -> &'static str {
    match (
        context.diff_summary.lines_added,
        context.diff_summary.lines_removed,
    ) {
        (added, 0) if added > 0 => "add",
        (0, removed) if removed > 0 => "remove",
        _ => "update",
    }
}

fn subject_for_type(
    commit_type: &str,
    scope: Option<&str>,
    verb: &str,
    issue_key: Option<&str>,
    context: &CommitContext,
) -> String {
    let target = match (scope, context.diff_summary.files_changed.len()) {
        (Some(value), _) => value.to_string(),
        (None, 1) => "single-file change".to_string(),
        (None, _) => "project changes".to_string(),
    };

    let base = match commit_type {
        "feat" => format!("{verb} {target} functionality"),
        "fix" => format!("{verb} {target} bug fixes"),
        "docs" => format!("{verb} documentation for {target}"),
        "test" => format!("{verb} tests for {target}"),
        "perf" => format!("improve {target} performance"),
        "ci" => format!("{verb} CI workflow for {target}"),
        "build" => format!("{verb} build configuration for {target}"),
        _ => format!("{verb} {target}"),
    };

    if let Some(key) = issue_key {
        format!("{key}: {base}")
    } else {
        base
    }
}

fn fallback_subject(verb: &str, issue_key: Option<&str>) -> String {
    let base = format!("{verb} staged changes");
    if let Some(key) = issue_key {
        format!("{key}: {base}")
    } else {
        base
    }
}

fn detect_secondary_type(context: &CommitContext, primary_type: &str) -> Option<String> {
    for path in &context.diff_summary.files_changed {
        if let Some(commit_type) = classify_path(path) {
            if commit_type != primary_type {
                return Some(commit_type);
            }
        }
    }
    None
}

fn rank_and_dedup(
    candidates: Vec<CommitSuggestion>,
    config: &SuggestionConfig,
) -> Vec<CommitSuggestion> {
    let mut seen = HashSet::new();
    let mut scored = Vec::new();

    for mut suggestion in candidates {
        apply_subject_length_limit(&mut suggestion, config.max_length);
        let fingerprint = suggestion.formatted().to_lowercase();
        if !seen.insert(fingerprint) {
            continue;
        }

        suggestion.confidence = score_candidate(&suggestion);
        scored.push(suggestion);
    }

    scored.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.formatted().cmp(&b.formatted()))
    });

    scored
}

fn score_candidate(suggestion: &CommitSuggestion) -> f32 {
    let mut score = 0.45_f32;
    if suggestion.scope.is_some() {
        score += 0.12_f32;
    }
    if suggestion.message.contains(':') {
        score += 0.08_f32;
    }
    if matches!(suggestion.commit_type.as_str(), "feat" | "fix") {
        score += 0.12_f32;
    }
    if suggestion.message.len() <= 60 {
        score += 0.08_f32;
    }
    score.min(0.98_f32)
}

fn apply_subject_length_limit(suggestion: &mut CommitSuggestion, max_length: usize) {
    if max_length == 0 {
        suggestion.message.clear();
        return;
    }

    let prefix = match &suggestion.scope {
        Some(scope) => format!("{}({scope}): ", suggestion.commit_type),
        None => format!("{}: ", suggestion.commit_type),
    };
    let prefix_len = prefix.chars().count();

    if prefix_len >= max_length {
        suggestion.message.clear();
        return;
    }

    let allowed = max_length - prefix_len;
    if suggestion.message.chars().count() <= allowed {
        return;
    }

    suggestion.message = suggestion.message.chars().take(allowed).collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::FileStatus;
    use crate::suggestions::diff::{ChangeTypeCounts, DiffSummary, FileDiffDetail};

    fn make_context(paths: Vec<&str>, branch: Option<&str>) -> CommitContext {
        let file_details = paths
            .iter()
            .map(|path| FileDiffDetail {
                path: (*path).to_string(),
                scope: path.split('/').next().map(ToString::to_string),
                change_type: FileStatus::Modified,
                lines_added: 3,
                lines_removed: 1,
                normalized_preview: "+example".to_string(),
            })
            .collect::<Vec<_>>();

        let summary = DiffSummary {
            files_changed: paths.iter().map(|value| (*value).to_string()).collect(),
            scopes: vec!["src".to_string()],
            lines_added: 10,
            lines_removed: 2,
            change_types: ChangeTypeCounts {
                added: 0,
                modified: paths.len(),
                deleted: 0,
            },
            file_details,
        };

        CommitContext::new(summary, branch.map(ToString::to_string))
    }

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

    #[test]
    fn test_rule_based_engine_generates_ranked_suggestions() {
        let engine = RuleBasedEngine::new();
        let config = SuggestionConfig::default();
        let context = make_context(vec!["src/main.rs", "src/lib.rs"], Some("feat/proj-12-api"));

        let suggestions = engine.suggest(&context, &config);
        assert!(!suggestions.is_empty());
        assert!(suggestions.len() <= config.max_suggestions);
        assert_eq!(suggestions[0].commit_type, "feat");
        assert!(suggestions[0].formatted().contains("PROJ-12"));
        for i in 1..suggestions.len() {
            assert!(suggestions[i - 1].confidence >= suggestions[i].confidence);
        }
    }

    #[test]
    fn test_rule_based_engine_deduplicates_formatted_suggestions() {
        let engine = RuleBasedEngine::new();
        let config = SuggestionConfig {
            max_suggestions: 6,
            ..SuggestionConfig::default()
        };
        let context = make_context(vec!["docs/guide.md"], Some("docs/proj-1-readme"));

        let suggestions = engine.suggest(&context, &config);
        let mut seen = HashSet::new();
        for suggestion in suggestions {
            assert!(seen.insert(suggestion.formatted()));
        }
    }

    #[test]
    fn test_rule_based_engine_respects_max_length() {
        let engine = RuleBasedEngine::new();
        let config = SuggestionConfig {
            max_suggestions: 3,
            max_length: 40,
            enabled: true,
        };
        let context = make_context(
            vec!["src/main.rs", "src/feature/very_long_name.rs"],
            Some("feat/proj-999-super-long-feature-name"),
        );

        let suggestions = engine.suggest(&context, &config);
        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .all(|suggestion| suggestion.formatted().chars().count() <= 40));
    }

    #[test]
    fn test_rule_based_engine_returns_empty_when_disabled() {
        let engine = RuleBasedEngine::new();
        let config = SuggestionConfig {
            enabled: false,
            ..SuggestionConfig::default()
        };
        let context = make_context(vec!["src/main.rs"], Some("feat/proj-1"));

        let suggestions = engine.suggest(&context, &config);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_rule_based_engine_respects_max_suggestions() {
        let engine = RuleBasedEngine::new();
        let config = SuggestionConfig {
            max_suggestions: 1,
            ..SuggestionConfig::default()
        };
        let context = make_context(vec!["src/main.rs", "docs/readme.md"], Some("feat/proj-7"));

        let suggestions = engine.suggest(&context, &config);
        assert_eq!(suggestions.len(), 1);
    }

    #[test]
    fn test_rule_based_engine_outputs_unique_formatted_messages() {
        let engine = RuleBasedEngine::new();
        let config = SuggestionConfig {
            max_suggestions: 5,
            ..SuggestionConfig::default()
        };
        let context = make_context(vec!["src/main.rs"], Some("feat/proj-8"));

        let suggestions = engine.suggest(&context, &config);
        let mut seen = HashSet::new();
        for suggestion in suggestions {
            assert!(seen.insert(suggestion.formatted().to_lowercase()));
        }
    }
}
