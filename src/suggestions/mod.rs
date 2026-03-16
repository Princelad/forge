//! Commit message suggestion engine.
//!
//! Provides a rule-based heuristic engine that analyses staged changes
//! and produces ranked conventional-commit message candidates.
//!
//! # Architecture
//!
//! ```text
//! suggestions
//! ├── engine   - CommitSuggestion, SuggestionConfig, SuggestionEngine trait, RuleBasedEngine
//! ├── diff     - DiffSummary: staged file list extracted from Change slices
//! ├── context  - CommitContext: assembles diff + branch name for the engine
//! └── rules    - CommitTypeRule registry (file pattern → commit type)
//! ```
//!
//! Phase 2 will fill in rule matching and scoring logic in `engine` and `rules`.

pub mod context;
pub mod diff;
pub mod engine;
pub mod rules;

pub use context::CommitContext;
pub use diff::DiffSummary;
pub use engine::{CommitSuggestion, RuleBasedEngine, SuggestionConfig, SuggestionEngine};
pub use rules::CommitTypeRule;
