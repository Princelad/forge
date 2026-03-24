//! Commit type detection rules.
//!
//! A [`CommitTypeRule`] maps a file pattern (extension or path prefix)
//! to a conventional commit type.
//!
//! Phase 2: expanded rule set with deterministic priority ordering.

use std::collections::HashMap;

/// A rule that maps a file pattern to a conventional commit type.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitTypeRule {
    /// File extension to match (without the leading dot, e.g. `"md"`).
    pub extension: Option<String>,
    /// Path prefix to match (e.g. `"docs/"`, `"tests/"`).
    pub path_prefix: Option<String>,
    /// Exact path to match (e.g. `"Cargo.toml"`).
    pub exact_path: Option<String>,
    /// The conventional commit type emitted when this rule matches.
    pub commit_type: String,
}

impl CommitTypeRule {
    /// Creates a rule that matches by file extension.
    pub fn by_extension(ext: &str, commit_type: &str) -> Self {
        Self {
            extension: Some(ext.to_string()),
            path_prefix: None,
            exact_path: None,
            commit_type: commit_type.to_string(),
        }
    }

    /// Creates a rule that matches by path prefix.
    pub fn by_prefix(prefix: &str, commit_type: &str) -> Self {
        Self {
            extension: None,
            path_prefix: Some(prefix.to_string()),
            exact_path: None,
            commit_type: commit_type.to_string(),
        }
    }

    /// Creates a rule that matches an exact path.
    pub fn by_exact_path(path: &str, commit_type: &str) -> Self {
        Self {
            extension: None,
            path_prefix: None,
            exact_path: Some(path.to_string()),
            commit_type: commit_type.to_string(),
        }
    }

    /// Returns `true` if this rule matches the given file path.
    pub fn matches(&self, path: &str) -> bool {
        if let Some(exact) = &self.exact_path {
            if path == exact {
                return true;
            }
        }
        if let Some(ext) = &self.extension {
            if path.ends_with(&format!(".{ext}")) {
                return true;
            }
        }
        if let Some(prefix) = &self.path_prefix {
            if path.starts_with(prefix.as_str()) {
                return true;
            }
        }
        false
    }
}

/// Returns the default rule registry for a software project.
pub fn default_rules() -> Vec<CommitTypeRule> {
    vec![
        // Most specific high-signal rules first.
        CommitTypeRule::by_prefix(".github/workflows/", "ci"),
        CommitTypeRule::by_exact_path("Cargo.lock", "build"),
        CommitTypeRule::by_exact_path("Cargo.toml", "build"),
        CommitTypeRule::by_prefix("docs/", "docs"),
        CommitTypeRule::by_prefix("tests/", "test"),
        CommitTypeRule::by_prefix("benches/", "perf"),
        CommitTypeRule::by_extension("md", "docs"),
        CommitTypeRule::by_extension("txt", "docs"),
        CommitTypeRule::by_extension("toml", "build"),
        CommitTypeRule::by_extension("yml", "ci"),
        CommitTypeRule::by_extension("yaml", "ci"),
        CommitTypeRule::by_extension("rs", "feat"),
    ]
}

/// Classifies a path into a commit type using ordered default rules.
///
/// Returns `None` when no rule matches.
pub fn classify_path(path: &str) -> Option<String> {
    for rule in default_rules() {
        if rule.matches(path) {
            return Some(rule.commit_type);
        }
    }
    None
}

/// Determines the dominant commit type for a set of changed paths.
///
/// Applies ordered rules to each path, votes by commit type frequency,
/// and returns a deterministic winner. Falls back to `"chore"`
/// when no files match a rule.
pub fn detect_commit_type(paths: &[String]) -> String {
    if paths.is_empty() {
        return "chore".to_string();
    }

    let rules = default_rules();
    let mut counts: HashMap<String, usize> = HashMap::new();

    for path in paths {
        if let Some(commit_type) = classify_path_with_rules(path, &rules) {
            *counts.entry(commit_type).or_insert(0) += 1;
        }
    }

    if counts.is_empty() {
        return "chore".to_string();
    }

    let mut winner = "chore".to_string();
    let mut best_count = 0;
    let mut best_priority = usize::MAX;

    for (idx, rule) in rules.iter().enumerate() {
        let count = counts.get(&rule.commit_type).copied().unwrap_or(0);
        if count == 0 {
            continue;
        }
        if count > best_count || (count == best_count && idx < best_priority) {
            best_count = count;
            best_priority = idx;
            winner = rule.commit_type.clone();
        }
    }

    winner
}

fn classify_path_with_rules(path: &str, rules: &[CommitTypeRule]) -> Option<String> {
    for rule in rules {
        if rule.matches(path) {
            return Some(rule.commit_type.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_rule_matches() {
        let rule = CommitTypeRule::by_extension("md", "docs");
        assert!(rule.matches("README.md"));
        assert!(!rule.matches("main.rs"));
    }

    #[test]
    fn test_prefix_rule_matches() {
        let rule = CommitTypeRule::by_prefix("docs/", "docs");
        assert!(rule.matches("docs/guide.txt"));
        assert!(!rule.matches("src/guide.txt"));
    }

    #[test]
    fn test_default_rules_are_non_empty() {
        assert!(!default_rules().is_empty());
    }

    #[test]
    fn test_docs_rule_applies_to_md_files() {
        let rules = default_rules();
        assert!(rules.iter().any(|r| r.matches("README.md")));
    }

    #[test]
    fn test_exact_path_rule_matches_only_exact_path() {
        let rule = CommitTypeRule::by_exact_path("Cargo.toml", "build");
        assert!(rule.matches("Cargo.toml"));
        assert!(!rule.matches("workspace/Cargo.toml"));
    }

    #[test]
    fn test_classify_path_prefers_prefix_specificity() {
        assert_eq!(classify_path("tests/git_ops.rs"), Some("test".to_string()));
        assert_eq!(classify_path("benches/speed.rs"), Some("perf".to_string()));
        assert_eq!(classify_path("src/main.rs"), Some("feat".to_string()));
    }

    #[test]
    fn test_detect_commit_type_returns_majority_type() {
        let paths = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "README.md".to_string(),
        ];
        assert_eq!(detect_commit_type(&paths), "feat");
    }

    #[test]
    fn test_detect_commit_type_falls_back_to_chore() {
        let paths = vec!["assets/logo.png".to_string(), "scripts/run.sh".to_string()];
        assert_eq!(detect_commit_type(&paths), "chore");
    }

    #[test]
    fn test_detect_commit_type_uses_priority_on_tie() {
        let paths = vec!["docs/guide.md".to_string(), "src/main.rs".to_string()];
        // docs and feat both score 1; docs wins by rule priority.
        assert_eq!(detect_commit_type(&paths), "docs");
    }

    #[test]
    fn test_classify_path_exact_match_beats_extension() {
        // Cargo.toml matches exact-path build rule before toml extension rule.
        assert_eq!(classify_path("Cargo.toml"), Some("build".to_string()));
    }

    #[test]
    fn test_detect_commit_type_empty_paths_defaults_to_chore() {
        let paths: Vec<String> = Vec::new();
        assert_eq!(detect_commit_type(&paths), "chore");
    }
}
