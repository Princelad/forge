# Progress

- Pending:

- AI/ML foundations: commit message suggestions MVP.
  - [x] Inventory current commit flow and integration points.
  - [x] Identify data sources for suggestions (staged diff, status, branch name).
  - [x] Decide on rule-based heuristic engine (no ML for MVP).
  - Phase 1: Core Infrastructure
    - [x] Create `src/suggestions/` module (mod.rs, engine.rs, rules.rs, diff.rs, context.rs)
    - [x] Define `CommitSuggestion` struct (type, scope, message, confidence)
    - [x] Define `SuggestionEngine` trait for testability
    - [x] Extend `AppSettings` with suggestion config (enabled, max_suggestions, max_length)
    - [x] Extend `ChangesState` with suggestions list and selected index
  - Phase 2: Suggestion Engine Implementation
    - [x] Implement diff analysis: extract files, scopes, change types
    - [x] Implement text normalization: strip binaries, limit size, redact secrets
    - [x] Implement branch context: extract issue keys from branch names
    - Implement rule-based type detection (file extension → commit type)
    - Implement suggestion generation with ranking/deduplication
  - Phase 3: UI Integration
    - Update `ChangesPage` to render suggestions panel
    - Add suggestion keybindings (1-3 to accept, or Tab+Enter pattern)
    - Wire suggestion generation to staged changes changes
    - Allow editing after accepting suggestion
  - Phase 4: Configuration & Polish
    - Add settings page options for suggestions
    - Add fallback messaging when no suggestions available
    - Add performance caching
  - Phase 5: Testing & Documentation
    - Add unit tests for rules, diff analysis, engine
    - Add integration test for suggestion flow
    - Update docs and CHANGELOG
  - **Clarifying Questions (need answers before proceeding):**
    - Keybinding preference: number keys (1-3) or cycle+accept (Tab/Enter)?
    - UI placement: above commit message, right panel, or popup overlay?
    - Scope detection: extract from directory names, filename prefixes, or both?
    - Performance: is <100ms for 1K lines a hard requirement?

- Docs: video tutorials.
- Docs: use case examples.
- Docs: architecture deep-dive.


- Completed:

- Testing: expand integration tests and add UI workflow coverage.
- Repo health UX: surface recovery actions inline for common Git failures.
- Remote branches: tracking + switch/manage remote-only branches.
- Upstream tracking: ahead/behind display and sync status.
- Docs audit: fix Features/Architecture inconsistencies and stale dependency versions.
- Remote ops: wire fetch/push keybindings to actions.
- Remote ops: surface remote selection (beyond hardcoded origin).
- Merge: apply accepted pane as real conflict resolution flow.
- Merge: add conflict list from real merge state instead of general changes.
- Settings: persist theme/notifications/autosync to config.
- Settings: implement notifications/autosync behavior (currently placeholders).
- Keybindings: load config file (TOML).
- Keybindings: validate and report errors.
- Keybindings: detect conflicting bindings across actions.
- Stash: list existing stashes.
- Stash: create stash with message.
- Stash: apply and pop stash.
- Stash: drop stash entry.
- Cherry-pick: single commit flow.
- Cherry-pick: conflict handling UX.
- Troubleshooting guide: common Git errors.
- Troubleshooting guide: recovery steps.
- Integration tests: repo fixture helpers.
- Integration tests: core Git ops (status, stage, commit).
