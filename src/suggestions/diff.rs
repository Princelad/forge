//! Diff summary extraction for the suggestion engine.
//!
//! `DiffSummary` is a reduced view of staged changes:
//! changed file paths, inferred scopes, change-type counts,
//! and aggregate line additions/removals parsed from `diff_preview`.

use std::collections::BTreeSet;

use crate::data::{Change, FileStatus};

/// Count of staged files by high-level change type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChangeTypeCounts {
    /// Number of staged files with `Added` status.
    pub added: usize,
    /// Number of staged files with `Modified` status.
    pub modified: usize,
    /// Number of staged files with `Deleted` status.
    pub deleted: usize,
}

/// Per-file staged diff details.
#[derive(Debug, Clone, PartialEq)]
pub struct FileDiffDetail {
    /// Repository-relative file path.
    pub path: String,
    /// Optional scope inferred from path/filename.
    pub scope: Option<String>,
    /// Coarse change type for the file.
    pub change_type: FileStatus,
    /// Added lines parsed from the patch preview.
    pub lines_added: usize,
    /// Removed lines parsed from the patch preview.
    pub lines_removed: usize,
}

/// A compact summary of staged changes used as engine input.
#[derive(Debug, Clone, Default)]
pub struct DiffSummary {
    /// Paths of all staged files.
    pub files_changed: Vec<String>,
    /// Distinct inferred scopes from staged file paths.
    pub scopes: Vec<String>,
    /// Total added lines across all staged files.
    pub lines_added: usize,
    /// Total removed lines across all staged files.
    pub lines_removed: usize,
    /// Count of staged file statuses by type.
    pub change_types: ChangeTypeCounts,
    /// Per-file details used by later ranking/rule steps.
    pub file_details: Vec<FileDiffDetail>,
}

impl DiffSummary {
    /// Creates an empty diff summary.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builds a summary from a slice of changes.
    pub fn from_changes(changes: &[Change]) -> Self {
        let staged_changes: Vec<&Change> = changes.iter().filter(|change| change.staged).collect();

        let mut files_changed = Vec::with_capacity(staged_changes.len());
        let mut scopes = BTreeSet::new();
        let mut lines_added = 0;
        let mut lines_removed = 0;
        let mut change_types = ChangeTypeCounts::default();
        let mut file_details = Vec::with_capacity(staged_changes.len());

        for change in staged_changes {
            let (file_added, file_removed) = parse_patch_line_counts(&change.diff_preview);
            let scope = infer_scope(&change.path);

            files_changed.push(change.path.clone());
            if let Some(scope_name) = &scope {
                scopes.insert(scope_name.clone());
            }

            lines_added += file_added;
            lines_removed += file_removed;

            match change.status {
                FileStatus::Added => change_types.added += 1,
                FileStatus::Modified => change_types.modified += 1,
                FileStatus::Deleted => change_types.deleted += 1,
            }

            file_details.push(FileDiffDetail {
                path: change.path.clone(),
                scope,
                change_type: change.status,
                lines_added: file_added,
                lines_removed: file_removed,
            });
        }

        Self {
            files_changed,
            scopes: scopes.into_iter().collect(),
            lines_added,
            lines_removed,
            change_types,
            file_details,
        }
    }

    /// Returns `true` if no staged files are present.
    pub fn is_empty(&self) -> bool {
        self.files_changed.is_empty()
    }
}

fn parse_patch_line_counts(diff_preview: &str) -> (usize, usize) {
    if diff_preview.is_empty()
        || diff_preview == "(no diff)"
        || diff_preview == "(diff not loaded)"
        || diff_preview.starts_with("Binary files")
    {
        return (0, 0);
    }

    let mut added = 0;
    let mut removed = 0;

    for line in diff_preview.lines() {
        if line.starts_with("+++")
            || line.starts_with("---")
            || line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("@@")
            || line.starts_with("new file mode")
            || line.starts_with("deleted file mode")
            || line.starts_with("similarity index")
            || line.starts_with("rename from")
            || line.starts_with("rename to")
        {
            continue;
        }

        if line.starts_with('+') {
            added += 1;
        } else if line.starts_with('-') {
            removed += 1;
        }
    }

    (added, removed)
}

fn infer_scope(path: &str) -> Option<String> {
    let first_component = path.split('/').next().unwrap_or_default();
    if first_component.is_empty() {
        return None;
    }

    if path.contains('/') {
        return Some(first_component.to_string());
    }

    let file_stem = path.rsplit_once('.').map_or(path, |(stem, _)| stem);
    if file_stem.is_empty() {
        return None;
    }

    let prefix = file_stem
        .split(['_', '-'])
        .next()
        .unwrap_or(file_stem)
        .trim();

    if prefix.is_empty() {
        None
    } else {
        Some(prefix.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(summary.change_types, ChangeTypeCounts::default());
        assert!(summary.scopes.is_empty());
        assert!(summary.file_details.is_empty());
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
        assert_eq!(summary.scopes, vec!["src"]);
        assert_eq!(summary.change_types.modified, 1);
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

    #[test]
    fn test_from_changes_parses_added_removed_lines() {
        let changes = vec![Change {
            path: "src/lib.rs".to_string(),
            status: FileStatus::Modified,
            diff_preview: "diff --git a/src/lib.rs b/src/lib.rs\nindex 123..456 100644\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,2 +1,3 @@\n-pub fn old() {}\n+pub fn new() {}\n+pub fn helper() {}\n unchanged"
                .to_string(),
            local_preview: None,
            incoming_preview: None,
            staged: true,
        }];

        let summary = DiffSummary::from_changes(&changes);
        assert_eq!(summary.lines_added, 2);
        assert_eq!(summary.lines_removed, 1);
        assert_eq!(summary.file_details[0].lines_added, 2);
        assert_eq!(summary.file_details[0].lines_removed, 1);
    }

    #[test]
    fn test_from_changes_extracts_change_type_counts() {
        let changes = vec![
            Change {
                path: "src/new.rs".to_string(),
                status: FileStatus::Added,
                diff_preview: "+fn new_file() {}".to_string(),
                local_preview: None,
                incoming_preview: None,
                staged: true,
            },
            Change {
                path: "src/existing.rs".to_string(),
                status: FileStatus::Modified,
                diff_preview: "-old\n+new".to_string(),
                local_preview: None,
                incoming_preview: None,
                staged: true,
            },
            Change {
                path: "src/obsolete.rs".to_string(),
                status: FileStatus::Deleted,
                diff_preview: "-dead_code".to_string(),
                local_preview: None,
                incoming_preview: None,
                staged: true,
            },
        ];

        let summary = DiffSummary::from_changes(&changes);
        assert_eq!(summary.change_types.added, 1);
        assert_eq!(summary.change_types.modified, 1);
        assert_eq!(summary.change_types.deleted, 1);
    }

    #[test]
    fn test_infer_scope_from_path_and_filename_prefix() {
        let changes = vec![
            Change {
                path: "src/suggestions/engine.rs".to_string(),
                status: FileStatus::Modified,
                diff_preview: "+change".to_string(),
                local_preview: None,
                incoming_preview: None,
                staged: true,
            },
            Change {
                path: "auth_login.rs".to_string(),
                status: FileStatus::Modified,
                diff_preview: "+change".to_string(),
                local_preview: None,
                incoming_preview: None,
                staged: true,
            },
        ];

        let summary = DiffSummary::from_changes(&changes);
        assert!(summary.scopes.contains(&"src".to_string()));
        assert!(summary.scopes.contains(&"auth".to_string()));
    }

    #[test]
    fn test_ignores_non_patch_previews_for_line_counts() {
        let changes = vec![
            Change {
                path: "assets/logo.png".to_string(),
                status: FileStatus::Added,
                diff_preview: "Binary files a/assets/logo.png and b/assets/logo.png differ"
                    .to_string(),
                local_preview: None,
                incoming_preview: None,
                staged: true,
            },
            Change {
                path: "src/main.rs".to_string(),
                status: FileStatus::Modified,
                diff_preview: "(diff not loaded)".to_string(),
                local_preview: None,
                incoming_preview: None,
                staged: true,
            },
        ];

        let summary = DiffSummary::from_changes(&changes);
        assert_eq!(summary.lines_added, 0);
        assert_eq!(summary.lines_removed, 0);
    }
}
