# Changelog

## v0.3.1 — 2026-02-13

### Added

- Cherry-pick single-commit flow with conflict routing to Merge view.
- Merge conflict list sourced from real merge state with accepted-pane resolution flow.
- Stash management: list, create with message, apply, pop, and drop.
- Keybindings configuration via TOML, with validation and conflict detection.
- Settings persistence plus notifications/autosync behavior.
- Integration test repo fixtures and core Git ops coverage (status, stage, commit).
- Troubleshooting guides for common Git errors and recovery steps.

### Changed

- Documentation refreshed for remote ops, keybindings, and merge resolution flow.
- Repository clone URL and wiki references updated for accuracy.

### Dependencies

- Bumped `tempfile` to 3.25.0.

## v0.3.0 — 2026-02-08

### Added

- Improved error messages with standardized formatting and added context.
- Terminal resize handling with automatic UI redraw.
- Git repo validation and recovery guidance for corrupted repositories.
- Upstream tracking display with ahead/behind status.
- Remote branch tracking with deletion handling.
- Remote fetch, pull, and push with progress and cancellation.

### Changed

- Git operations optimized for large repositories.
- Cached expensive computations to improve responsiveness.
- Background task handling improvements for long-running operations.
- Page state extracted into dedicated structs for better isolation and testability.
- Documentation refreshed for the v0.3.0 release.

## v0.2.0 — 2026-01-28

### Added

- Remote fetch, pull, and push with progress reporting, cancellation, and credential handling (SSH agent/keys/helpers).
- Background task handling improvements in `async_task` to support remote operations.

### Changed

- App state extracted into dedicated page structs (`DashboardState`, `ChangesState`, `BoardState`, `MergeState`, `ModuleManagerState`, `BranchManagerState`, `CommitHistoryState`) reducing `App` complexity and improving testability.
- Documentation refreshed for remote operations and new state module.

### Testing

- `cargo test` (155 tests) and `cargo clippy -- -D warnings` passing.
- `cargo bench` completed for `git_operations` and `data_operations` (see `target/criterion/report/index.html`). Noted mild regression on `stage_file` (~+1.4%) and improvement on `unstage_file` (~-3.3%).

### Notes

- Remote branch tracking and upstream display remain pending.
- Real remote smoke test not executed in this run (offline environment); run against a staging remote to verify credentials.

## v0.1.0 — 2025-xx-xx

- Initial release.
