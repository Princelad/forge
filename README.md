# Forge

**A developer-first, terminal-based Git-aware project management system.**

Current release: v0.3.1

Manage Git repositories, view diffs, track tasks, and collaborate—all from your terminal without leaving your coding flow.

![Changes View](screenshots/Changes.png)

## Quick Start

```bash
cargo build --release
./target/release/forge
```

## Key Features

- 🔀 **Git Integration** — Real-time file status and diff preview
- 🌐 **Remote Operations** — Fetch, pull, and push with progress + cancellation
- 🧰 **Stash & Cherry-pick** — Manage stashes and cherry-pick commits
- 📋 **Project Board** — Kanban task tracking (Pending → Current → Completed)
- 🔗 **Branch Management** — Create, switch, and delete branches
- 📝 **Commit History** — Browse and inspect commits
- 💡 **Commit Suggestions** — Ranked conventional-commit suggestions with quick apply (`1-3`)
- 👥 **Team Management** — Track modules and developers
- 🔀 **Merge Visualization** — Side-by-side local/incoming previews with accept markers
- ⚙️ **Customizable** — Theme switching, notifications, autosync, and keybindings
- ⌨️ **Keyboard-Driven** — Fully navigable without mouse

## Documentation

📚 **[Complete Documentation →](https://github.com/Princelad/forge/wiki)**

- **[Getting Started](https://github.com/Princelad/forge/wiki/Getting-Started)** — Installation and first run
- **[Features](https://github.com/Princelad/forge/wiki/Features)** — Complete feature overview
- **[Keyboard Shortcuts](https://github.com/Princelad/forge/wiki/Keyboard-Shortcuts)** — All keybindings
- **[Architecture](https://github.com/Princelad/forge/wiki/Architecture)** — System design
- **[Development](https://github.com/Princelad/forge/wiki/Development)** — Contributing guide
- **[API Reference](https://github.com/Princelad/forge/wiki/API-Reference)** — Core types and functions
- **[Workflows](https://github.com/Princelad/forge/wiki/Workflows)** — User workflows and interaction patterns
- **[Performance](https://github.com/Princelad/forge/wiki/Performance)** — Benchmarks and optimization
- **[Roadmap](https://github.com/Princelad/forge/wiki/Roadmap)** — Future plans
- **[FAQ](https://github.com/Princelad/forge/wiki/FAQ)** — Common questions

## Requirements

- **Rust 1.70+** ([install here](https://rustup.rs/))
- **Git 2.0+**
- **Terminal** with 256-color support

## Build & Run

```bash
# Clone the repository
git clone https://github.com/Princelad/forge.git
cd forge

# Build for production
cargo build --release

# Run in a Git repository
cd /path/to/your/repo
/path/to/forge/target/release/forge
```

## Install

### Linux/macOS (cargo install)

```bash
# From crates.io (when published)
cargo install forge

# From GitHub source
cargo install --git https://github.com/Princelad/forge.git --locked
```

### Linux/macOS (binary download)

```bash
# Example for Linux x86_64
curl -L -o forge.tar.gz \
	https://github.com/Princelad/forge/releases/download/v0.3.1/forge-linux-x86_64.tar.gz
tar -xzf forge.tar.gz
sudo install -m 0755 forge /usr/local/bin/forge

# Example for macOS arm64
curl -L -o forge.tar.gz \
	https://github.com/Princelad/forge/releases/download/v0.3.1/forge-macos-arm64.tar.gz
tar -xzf forge.tar.gz
sudo install -m 0755 forge /usr/local/bin/forge
```

## Shell Completion

Forge supports shell completion script generation via:

```bash
forge --print-completion bash
forge --print-completion zsh
forge --print-completion fish
```

Install examples:

```bash
# Bash
forge --print-completion bash > ~/.local/share/bash-completion/completions/forge

# Zsh
mkdir -p ~/.zsh/completions
forge --print-completion zsh > ~/.zsh/completions/_forge

# Fish
mkdir -p ~/.config/fish/completions
forge --print-completion fish > ~/.config/fish/completions/forge.fish
```

## Usage

1. **Navigate** with Tab and Arrow keys
2. **Stage files** with Space
3. **Commit** with Enter
4. **Apply suggestion** with `1`, `2`, or `3` in Changes view (then edit if needed)
5. **View help** with `?`
6. **Fetch** with `f` (Dashboard or Changes view)
7. **Push** with `p` (Changes view)
8. **Pull** with Ctrl+L (Changes view)
9. **Quit** with Ctrl+C

See the **[Keyboard Shortcuts](https://github.com/Princelad/forge/wiki/Keyboard-Shortcuts)** page for complete reference.

## 5-Minute Quickstart Workflow

This walkthrough covers: init, stage, commit, branch, and sync.

```bash
# 1) Start in your repository
cd /path/to/your/repo
forge

# 2) Make a change in another terminal
echo "# notes" >> NOTES.md

# 3) Stage and commit in Forge
# - Tab to Changes view
# - Use Up/Down to select file
# - Press Space to stage
# - Press 1/2/3 to apply a suggestion (optional)
# - Press Enter to type/edit commit message
# - Press Enter again to commit

# 4) Branch workflow in Forge
# - Tab to Branch Manager
# - Press n to create branch (example: feat/quickstart)
# - Press Enter to switch to selected branch

# 5) Sync workflow in Forge
# - Tab to Changes view
# - Press f to fetch
# - Press Ctrl+l to pull
# - Press p to push
```

Expected result: your change is committed on a feature branch and synced with the selected remote.

## Troubleshooting Decision Tree

Start here when an action fails:

1. Is the error about authentication (`auth`, `permission denied`, `publickey`)?
	- Verify remote URL and credentials.
	- Re-run with valid SSH key or HTTPS token.
2. Is the error about merge conflicts?
	- Open Merge view in Forge.
	- Resolve per-file by choosing Local/Incoming and commit resolution.
3. Are you in detached HEAD state?
	- Create or switch to a normal branch in Branch Manager.
	- Retry commit/push after switching.
4. Is there a lock file error (`index.lock` or refs lock)?
	- Ensure no other Git process is running.
	- Remove stale lock file only after confirming no active Git command.
5. Is the remote operation failing repeatedly?
	- Press `?` for key hints and verify selected remote in Settings.
	- Run fetch first, then pull/push.

If still blocked, capture the exact status bar message and open an issue.

## Commit Suggestions

Forge includes a rule-based commit suggestion engine in Changes view.

Behavior:

1. Suggestions are generated only from staged changes.
2. Suggestions are ranked by confidence.
3. Press `1-3` to apply a suggestion, then edit manually before commit.
4. Branch names with issue keys can influence suggestion formatting.

Limits:

1. Up to `max_suggestions` entries (validated range: 1-5).
2. Message length capped by `max_length` (validated range: 20-200).
3. No suggestions are shown when no staged files exist.

Config knobs:

1. `settings.suggestions.enabled`
2. `settings.suggestions.max_suggestions`
3. `settings.suggestions.max_length`

These are available in Settings and persisted in `.forge/settings.json`.

## Keymap Overrides

Forge writes a default keymap profile to `.forge/keybindings.default.toml` on startup (if missing).

To customize keybindings:

1. Copy `.forge/keybindings.default.toml` to `.forge/keybindings.toml`
2. Edit only the action bindings you want to override
3. Restart Forge and check startup diagnostics in the status bar if the schema is invalid

Example:

```toml
[bindings]
next_view = "Tab"
navigate_up = "Up"
navigate_down = "Down"
search = "Ctrl+f"
toggle_staging = "Space"
```

## Contributing

Contributions are welcome! See **[CONTRIBUTING.md](CONTRIBUTING.md)** for quick start, or the **[Development](https://github.com/Princelad/forge/wiki/Development)** wiki for comprehensive guidelines.

## License

GPL-3.0-only

---

**Need help?** Check the **[FAQ](https://github.com/Princelad/forge/wiki/FAQ)** or open an issue on GitHub.
