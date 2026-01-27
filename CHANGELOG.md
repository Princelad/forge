# Changelog

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
