//! Commit type detection rules.
//!
//! A [`CommitTypeRule`] maps a file pattern (extension or path prefix)
//! to a conventional commit type.
//!
//! Phase 1: skeleton and initial registry stubs.
//! Phase 2: expand rule set and implement priority ordering.

/// A rule that maps a file pattern to a conventional commit type.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitTypeRule {
    /// File extension to match (without the leading dot, e.g. `"md"`).
    pub extension: Option<String>,
    /// Path prefix to match (e.g. `"docs/"`, `"tests/"`).
    pub path_prefix: Option<String>,
    /// The conventional commit type emitted when this rule matches.
    pub commit_type: String,
}

impl CommitTypeRule {
    /// Creates a rule that matches by file extension.
    pub fn by_extension(ext: &str, commit_type: &str) -> Self {
        Self {
            extension: Some(ext.to_string()),
            path_prefix: None,
            commit_type: commit_type.to_string(),
        }
    }

    /// Creates a rule that matches by path prefix.
    pub fn by_prefix(prefix: &str, commit_type: &str) -> Self {
        Self {
            extension: None,
            path_prefix: Some(prefix.to_string()),
            commit_type: commit_type.to_string(),
        }
    }

    /// Returns `true` if this rule matches the given file path.
    pub fn matches(&self, path: &str) -> bool {
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
///
/// Phase 2 will expand this with `.rs`, `.toml`, CI config, and lock file patterns.
pub fn default_rules() -> Vec<CommitTypeRule> {
    vec![
        CommitTypeRule::by_extension("md", "docs"),
        CommitTypeRule::by_extension("txt", "docs"),
        CommitTypeRule::by_prefix("docs/", "docs"),
        CommitTypeRule::by_prefix("tests/", "test"),
        CommitTypeRule::by_prefix("benches/", "perf"),
    ]
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
}
