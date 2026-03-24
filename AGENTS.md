# AGENTS.md - Forge Development Guide

AI agent guidance for the Forge project (terminal-based Git-aware project management in Rust).

## Build, Test, and Lint Commands

```bash
# Build
cargo build                    # Debug
cargo build --release          # Release

# Test
cargo test                     # All tests
cargo test -- --nocapture     # With output
cargo test test_name          # Single unit test
cargo test --test git_ops status_reports_untracked_file  # Single integration test
cargo test --lib              # Unit tests only
cargo test --test git_ops     # Integration tests only
cargo test stash              # Tests matching pattern

# Code quality
cargo fmt                     # Format (required)
cargo fmt --check             # Check without modifying
cargo clippy -- -W clippy::pedantic  # Lint (required)
cargo fmt && cargo clippy -- -W clippy::pedantic && cargo test  # All checks
```

---

## Code Style Guidelines

### Formatting and Naming
- Use rustfmt defaults (`cargo fmt` before committing)
- `snake_case` for functions/variables/modules
- `PascalCase` for types, enums, structs
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines)

### Imports
Group in order: std → external crates → local modules. Use explicit paths:

```rust
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use color_eyre::eyre::Result;
use git2::Repository;
use crate::data::{Change, FileStatus};
use crate::git::GitClient;
```

### Error Handling
- Use `Result<T, E>` for recoverable errors
- **Avoid `unwrap()`/`expect()` in production** - use `?` or `.unwrap_or_else()`
- This project uses `color_eyre` for error reporting
- **UI should never panic** - display errors in status bar
- Wrap errors: `.wrap_err("context message")`

```rust
// Good: proper error handling
fn discover(path: &Path) -> Result<GitClient> {
    let repo = Repository::discover(path).wrap_err("failed to discover repository")?;
    Ok(GitClient { repo })
}

// Good: graceful fallback
let branch = client.head_branch().unwrap_or_else(|| "main".to_string());

// Bad: unwrap in production
let client = GitClient::discover(path).unwrap();
```

### Types and Collections
- Prefer iterators over manual loops
- Use `Vec`, `HashMap`, `HashSet` appropriately
- Use `&str` over `String` for parameters when borrowing
- Clone sparingly - prefer `&T` or `Arc<T>` over clone

```rust
// Good: iterator-based
let added: Vec<_> = changes.iter().filter(|c| c.staged).collect();

// Good: borrowing
fn process_change(change: &Change) -> String {
    format!("{}: {:?}", change.path, change.status)
}
```

### Safety and Performance
- Avoid `unsafe` unless necessary; document invariants
- Use `Arc<Mutex<T>>` for shared mutable state
- Atomic types for flags: `AtomicBool`, `AtomicUsize`
- Use `saturating_add/sub` for bounded counters

### Documentation
- Public API needs doc comments (`///`)
- Use `//!` for module-level docs
- No comments for obvious code

---

## Testing Guidelines

### Unit Tests
- In same file with `#[cfg(test)]` module
- Descriptive names: `test_<operation>_<expected>`
- Use `tempfile` crate for temp repos

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_spawn_operation() {
        let temp = TempDir::new().expect("create temp dir");
        let mut tm = TaskManager::new();
        tm.spawn_operation(temp.path().to_path_buf(), GitOperation::Fetch("origin".into()));
        assert!(tm.has_pending());
    }
}
```

### Integration Tests
- In `tests/` directory
- Use `tests/common/mod.rs` fixtures (`RepoFixture`)

```rust
mod common;
use common::RepoFixture;

#[test]
fn status_reports_untracked_file() {
    let fixture = RepoFixture::new().expect("fixture init failed");
    fixture.write_file("foo.txt", "hello").expect("write failed");
    let client = GitClient::discover(fixture.path()).expect("discover failed");
    let changes = client.list_changes_summary().expect("list failed");
    assert!(!changes.is_empty());
}
```

---

## Project Structure

```
src/
├── main.rs           # Entry, TUI event loop
├── lib.rs           # Exports for testing
├── git.rs           # Git operations
├── data.rs          # Models (Change, Project, Module, Store)
├── key_handler.rs   # Input handling
├── screen.rs        # Rendering
├── async_task.rs    # Background Git ops (fetch/push/pull)
├── status_symbols.rs # Git status symbols
├── ui_utils.rs      # UI utilities
├── pages/           # TUI page components
└── state/           # App state

tests/
├── git_ops.rs       # Integration tests
└── common/mod.rs    # Test fixtures (RepoFixture)

benches/
├── git_operations.rs
└── data_operations.rs
```

---

## Key Patterns

### Git Client
```rust
use crate::git::GitClient;
let client = GitClient::discover(path).wrap_err("not a git repo")?;
let changes = client.list_changes_summary()?;
client.stage_file("file.txt")?;
client.commit_all("message")?;
```

### Async Task (channel-based, not async/await)
```rust
use crate::async_task::{TaskManager, GitOperation};
let mut tm = TaskManager::new();
tm.spawn_operation(path, GitOperation::Fetch("origin".into()));
if let Some(result) = tm.try_recv() { /* handle */ }
```

### Data Persistence (`.forge/` directory)
```rust
use crate::data::Store;
let mut store = Store::new();
store.load_from_json(workdir)?;
store.save_to_json(workdir)?;
```

---

## Dependencies

- Add with `cargo add <crate>` (not manual edit)
- Pin versions in `Cargo.toml`
- Prefer well-maintained, minimal dependencies
- No heavy new deps without discussion

---

## Before Committing

1. `cargo fmt`
2. `cargo clippy -- -W clippy::pedantic`
3. `cargo test`

Follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.

Update `PROGRESS.md` to track sprint progress.

---

## References

- [Development Wiki](https://github.com/Princelad/forge/wiki/Development)
- [Architecture Wiki](https://github.com/Princelad/forge/wiki/Architecture)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines)
- `.github/instructions/forge.instructions.md`
