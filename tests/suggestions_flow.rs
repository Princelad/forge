use forge::data::{Change, FileStatus};
use forge::suggestions::{CommitContext, DiffSummary, RuleBasedEngine, SuggestionConfig, SuggestionEngine};

fn staged_change(path: &str, status: FileStatus, diff_preview: &str) -> Change {
    Change {
        path: path.to_string(),
        status,
        diff_preview: diff_preview.to_string(),
        local_preview: None,
        incoming_preview: None,
        staged: true,
    }
}

#[test]
fn suggestion_flow_generates_ranked_unique_messages_from_staged_changes() {
    let changes = vec![
        staged_change("src/main.rs", FileStatus::Modified, "+fn feature() {}"),
        staged_change("src/lib.rs", FileStatus::Modified, "+pub mod feature;"),
        staged_change("README.md", FileStatus::Modified, "+docs update"),
    ];

    let diff_summary = DiffSummary::from_changes(&changes);
    let context = CommitContext::new(diff_summary, Some("feat/proj-42-suggest".to_string()));

    let engine = RuleBasedEngine::new();
    let config = SuggestionConfig {
        enabled: true,
        max_suggestions: 3,
        max_length: 72,
    };

    let suggestions = engine.suggest(&context, &config);

    assert!(!suggestions.is_empty());
    assert!(suggestions.len() <= config.max_suggestions);
    assert_eq!(suggestions[0].commit_type, "feat");
    assert!(suggestions[0].formatted().contains("PROJ-42"));

    for i in 1..suggestions.len() {
        assert!(suggestions[i - 1].confidence >= suggestions[i].confidence);
    }

    let mut seen = std::collections::HashSet::new();
    for suggestion in &suggestions {
        assert!(seen.insert(suggestion.formatted().to_lowercase()));
        assert!(suggestion.formatted().chars().count() <= config.max_length);
    }
}

#[test]
fn suggestion_flow_returns_empty_when_no_staged_changes() {
    let diff_summary = DiffSummary::from_changes(&[]);
    let context = CommitContext::new(diff_summary, Some("feat/proj-7-empty".to_string()));

    let engine = RuleBasedEngine::new();
    let suggestions = engine.suggest(&context, &SuggestionConfig::default());

    assert!(suggestions.is_empty());
}
