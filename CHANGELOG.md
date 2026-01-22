# Changelog

All notable changes to Forge are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Removed

- Deleted `src/render_context.rs` - 280 lines of unused infrastructure never integrated
- Removed unused `Screen` delegation methods: `handle_key_action`, `get_selected_menu_item`, `get_menu_items_count`, `get_selected_menu_item_by_index`
- Removed `PAGE_SIZE` constant from `main.rs` - marked as dead code
- Removed `KeyHandler::is_module_manager_context()` - always returned `false`, never used
- Removed `MainMenu.selected_option` field - App maintains this state independently
- Removed `pub mod render_context` declaration from `main.rs`

### Added

- Created `src/ui_utils.rs` with common UI utilities:
  - `create_list_state()` - standardizes ListState creation with bounds checking
  - `focused_block()` - creates blocks with conditional focus styling
  - `render_input_form()` - renders common input forms
  - `auto_scroll()` - auto-scrolls views to keep selected items visible

### Changed

- Refactored all page modules to use `ui_utils` helpers:
  - `dashboard.rs`: uses `create_list_state()`
  - `changes.rs`: uses `create_list_state()`
  - `branch_manager.rs`: uses `create_list_state()`, `render_input_form()`
  - `commit_history.rs`: uses `create_list_state()`
  - `module_manager.rs`: uses `create_list_state()` (2×), `render_input_form()` (2×)
  - `project_board.rs`: uses `create_list_state()` (3×), `focused_block()` (3×)
- Eliminated 21 instances of duplicate UI patterns across 6 page modules
- Reduced code by ~80 lines through deduplication
- Standardized render interfaces with params structs:
  - Added `ChangesParams` to `changes.rs`
  - Added `CommitHistoryParams` to `commit_history.rs`
  - Added `ProjectBoardParams` to `project_board.rs`
  - Added `SettingsParams` to `settings.rs`
- All page modules now consistently use params structs for rendering
- Updated `screen.rs` to use params structs for all page render calls
- Created `src/status_symbols.rs` module with standardized status indicators:
  - Constants: `SUCCESS`, `ERROR`, `PROGRESS`, `INFO`
  - Helper functions: `success()`, `error()`, `progress()`, `info()`
- Updated all status messages throughout `main.rs` to use status_symbols helpers
- Added `adjust_pane_ratio()` helper to `ui_utils.rs`
- Consolidated pane ratio adjustment logic in `key_handler.rs` using helper function
- Reduced 60+ lines of duplicate pane adjustment code to 8 lines

### Fixed

- Simplified navigation bounds checking throughout `key_handler.rs`:
  - Replaced all `if index > 0 { index - 1 } else { index }` patterns with `saturating_sub()`
  - Replaced all `if index < max { index + 1 } else { index }` patterns with `.min(max)`
  - Reduced ~100 lines of conditional navigation code
  - Applied to all views: Dashboard, Changes, CommitHistory, BranchManager, MergeVisualizer, ModuleManager, Settings
- Added `safe_decrement()` helper to `ui_utils.rs` for future use
- Removed unnecessary conditional returns in navigation handlers by using saturating arithmetic

### Changed

- **BREAKING**: Renamed `FakeStore` to `Store` across entire codebase
  - The "Fake" prefix was misleading - this is production-ready JSON persistence
  - Updated all references in source files, tests, and benchmarks
  - All tests passing (33/33)
  - Breaking change for external API consumers
- Improved benchmark error handling in `benches/git_operations.rs`
  - Added error tracking with `Cell<u32>` for thread-safe counting
  - Replaced silent `.ok()` calls with `is_err()` checks and error counting
  - Added warning messages to stderr when errors occur during benchmarking
  - Prevents masking performance regressions caused by errors
  - Zero performance impact on successful operations
- Optimized `.clone()` usage to reduce unnecessary allocations
  - `src/git.rs`: Use `as_ref()` instead of cloning `Option<String>` values for diff previews
  - `src/main.rs`: Removed duplicate `workdir.clone()` and `repo_name.clone()` calls
  - Moved ownership where possible instead of cloning
  - All tests passing - no functional changes

### Added

- **Documentation**: Comprehensive edge case documentation for corrupted repository handling
  - Added module-level documentation in `src/git.rs` explaining error handling strategy
  - Documented 5 common corruption scenarios: missing HEAD, corrupted index, missing objects, invalid refs, locked index
  - Added method-level documentation for `discover()`, `head_branch()`, `list_changes()`, `commit_all()`
  - Created detailed FAQ entry in `docs/wiki/FAQ.md` with recovery procedures
  - Documented recommended improvements and prevention strategies
  - All current behavior and limitations are now explicitly documented
- **Documentation**: Commit message shortcut architecture analysis and design rationale
  - Evaluated Phase 6c's `commit_message_empty` conditional shortcut pattern
  - Documented pros/cons of current design vs explicit input mode state machine
  - Added comprehensive comparison in `src/key_handler.rs` ActionContext documentation
  - **Recommendation**: Keep current design - intuitive UX, only 2 conflicting shortcuts
  - Future alternatives documented: multi-key chords, explicit focus, modal editing
  - Design decision preserved for future maintainers (see key_handler.rs lines 90-155)
- Remote operations (fetch, pull, push) support
- AI/ML features planning
- Multi-repository support planning

---

## January 20, 2026 - Comprehensive Code Quality & Testing

### Added

#### Phase 4: Test Suite Foundation

- Created `src/lib.rs` to enable library testing of core modules
- Added dev dependency `tempfile = "3.8.1"` for temp directory test fixtures
- Updated `Cargo.toml` to define both lib and bin targets
- **23 unit tests** covering:
  - **Git Integration Tests** (7 tests in `src/git.rs`):
    - `test_gitclient_discover_valid_repo` — Repository discovery succeeds for valid repos
    - `test_gitclient_discover_invalid_path` — Discovery fails for non-existent paths
    - `test_head_branch_on_empty_repo` — Empty repo HEAD handling
    - `test_list_changes_empty_repo` — Empty repo has no changes
    - `test_list_changes_with_untracked_file` — Untracked files are detected and marked as Added
    - `test_stage_file` — Files can be staged successfully
    - `test_commit_data_type_alias` — Type alias compilation verification
    - `test_full_git_workflow` — Integration test covering create/commit/detect workflow
  - **Data Model Tests** (15 tests in `src/data.rs`):
    - Enum and struct creation tests
    - FakeStore CRUD operations (add/delete developers, modules)
    - Progress tracking with saturation math
    - Auto-population from Git with duplicate prevention
  - **UI Event Tests** (1 test in `src/key_handler.rs`):
    - `test_maps_basic_keys` — Core keybindings map correctly
- ✅ All 23 tests pass with 100% pass rate
- Test execution time: < 100ms for full suite

#### Phase 5: Performance Profiling & Benchmarks

- Created `benches/` directory with criterion-based benchmarks
- Added `criterion = { version = "0.5", features = ["html_reports"] }` dev dependency
- Configured `[[bench]]` declarations in `Cargo.toml`
- **11 comprehensive benchmarks** across two suites:
  - **Git Operations** (7 benchmarks in `benches/git_operations.rs`):
    - `discover_repo` — 9.65 ms (I/O bottleneck)
    - `head_branch` — 47.18 µs
    - `list_changes` — 549 µs (10 files) to 2.79 ms (50 files)
    - `get_commit_history` — 333 µs (10 commits) to 1.70 ms (50 commits)
    - `list_branches_local` — 11.48 µs
    - `list_branches_remote` — 11.43 µs
    - `stage_file` — 26.39 µs
    - `unstage_file` — 10.62 µs (fastest Git operation)
  - **Data Operations** (4 benchmarks in `benches/data_operations.rs`):
    - `bump_progress` — 120-200 µs
    - `add_developer` — 149-204 ns (fastest overall)
    - `delete_developer` — 91 ns
    - `auto_populate_developers` — 233 ns (10) to 840 µs (1000 committers)
- Collected performance baselines with statistical analysis
- HTML reports generated at `target/criterion/report/index.html`

#### Phase 6: Remote Operations

- **Phase 6c**: Fixed UX issue where users couldn't type 'f' or 'p' characters in commit messages
  - Applied fix using `commit_message_empty` context field
  - Fetch/push shortcuts only trigger when NOT typing commit message
  - Allows natural workflows like typing "fix bug" without losing characters
- **Phase 6b**: Integrated fetch and push into UI
  - Updated help page to document 'f' (fetch) and 'p' (push) keybindings
  - Added remote operation hints to Changes view title
  - Added remote operation hints to Dashboard title
  - Status messages display operation results with visual feedback
- Frontend fully integrated with status bar feedback and context-aware keybindings

### Changed

#### Phase 1: Automatic & Manual Fixes

- Applied **17 automatic clippy fixes** across 14 files
- Applied **4 manual flatten optimizations** in `git.rs` (iterator patterns simplified)
- Created module-level `CommitData` type alias to reduce type complexity
- **Warnings Reduction**: 26 → 9 warnings

#### Phase 2: Parameter Struct Refactoring

- Refactored **5 page components** to use parameter structs:
  - `src/pages/dashboard.rs` — 9 args → 1 param struct (`DashboardParams`)
  - `src/pages/branch_manager.rs` — 7 args → 1 param struct (`BranchManagerParams`)
  - `src/pages/merge_visualizer.rs` — 7 args → 1 param struct (`MergeVisualizerParams`)
  - `src/pages/module_manager.rs` — 8-9 args → 2 param structs (`ModuleManagerParams`, `ModuleListParams`)
  - `src/screen.rs` — Updated to construct and pass parameter structs to each page
- **Warnings Reduction**: 9 → 1 warning (96% reduction from original 26)
- Improved code maintainability and self-documentation

#### Phase 3: Infrastructure & Foundation

- Created `src/render_context.rs` — Framework for future screen-level parameter consolidation
- Implemented builder pattern in `RenderContext` for fluent API

### Fixed

- **UX Issue**: Users can now type 'f' and 'p' characters in commit messages without triggering fetch/push shortcuts
- Various clippy warnings related to code quality and idioms

### Performance

- Quantified I/O bottleneck: Repository discovery at 9.65ms dominates workflow
- Confirmed linear scaling for file status (O(n)) and commit history (O(n) up to limit)
- Data operations confirmed negligible (nanosecond range)
- Typical workflow (discover → list → stage → commit) ≈ 13-15ms

### Documentation

- Updated README with UX note explaining fetch/push behavior
- All changes tracked in progress log and recent changes sections

---

## Progress Log (Historical Entries)

### 2026-01-20 (Late Evening) — Phase 6c: Key Handling Review & UX Fix

**Author**: GitHub Copilot

Reviewed key handling implementation for fetch/push features. Identified and fixed critical UX issue: users couldn't type 'f' or 'p' characters in commit messages (shortcuts were intercepted unconditionally). Applied fix using `commit_message_empty` context field: fetch/push shortcuts only trigger when NOT typing commit message. This allows natural workflows like typing "fix bug" without losing the 'f' character. Verified fix: 24 tests passing, clean compile. Updated README with UX note explaining behavior.

**Status**: Remote operations feature now complete and fully usable  
**Next**: Implement git pull

---

### 2026-01-20 (Evening) — Phase 6b: Remote Operations Frontend Complete

**Author**: GitHub Copilot

Integrated fetch and push into UI with proper keybinding hints. Updated help page to document 'f' (fetch) and 'p' (push) keybindings with description. Added remote operation hints to Changes view title: "Space: stage/unstage | f: fetch | p: push". Added remote operation hints to Dashboard title: "(Ctrl+F: search, f: fetch)". Status messages display operation results: "✓ Fetched N objects from origin" or "✗ Push failed: <error>". Frontend fully integrated with status bar feedback and context-aware keybindings.

**Status**: 24 tests passing, release build clean, frontend production-ready  
**Next**: Implement git pull

---

## See Also

- **[Roadmap](Roadmap.md)** — Future planned features
- **[Development](Development.md)** — Contributing guide
- **[Getting Started](Getting-Started.md)** — Installation and setup
