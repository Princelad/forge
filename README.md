# Forge

**Forge** is a developer-first, terminal-based Git-aware project management system.
It integrates version control context with lightweight task tracking in a single TUI workflow, allowing developers to manage code changes, view diffs, and track project progress without leaving the terminal.

Forge loads your current Git repository automatically and presents:

- **Real file changes** from `git status`
- **Live diff previews** for modified files
- **Current branch** and repository metadata
- **Project board** for task tracking (manual module management)
- **Merge conflict visualization** with resolution tracking
- **Configurable themes** and settings

---

## Project Objective

Forge validates the usability of a **Git-integrated project board and change visualization system** through a **Text User Interface (TUI)**.

Core question:

> _Can a developer manage project context, tasks, and code changes from a single terminal interface without breaking their coding flow?_

---

## Current State

### Git Integration ✅

- Automatic repository discovery on startup
- Real-time file status (`git status`)
- Diff preview generation (`git diff`)
- Branch detection from HEAD
- Repository metadata (path, name, branch)

### Implemented Features

- Terminal-based UI using **Rust + ratatui + git2**
- Top-bar menu navigation with focus tracking
- Real Git repository parsing and status display
- Multi-pane layouts for complex views
- Keyboard-driven interactions (Tab, arrows, Enter, Esc)
- Project search with live filtering (`Ctrl+F`)
- Settings with theme switching (Default/HighContrast)
- Merge resolution tracking with visual indicators
- Commit message input (UI ready, commit execution pending)
- Help overlay with keybindings (`?`)
- Focus-aware status bar with repo/settings badges

### Explicitly Not Yet Implemented

- Staging files and executing Git commits
- Commit history analysis or log viewing
- Branch switching and creation
- Remote operations (push/pull/fetch)
- Automated task inference from commits
- Persistence layer for modules/developers
- AI/ML features

---

## Key Screens / Views

### 1. Dashboard

- Project list (left pane, selectable)
- Project metadata display (right pane):

  - Name, branch, module count, developer count, description

- Selection syncs with menu when tabbing between views

### 2. Project Board

- Kanban-style layout with three columns:

  - **Pending** — Modules not yet started
  - **Current** — In-progress modules with assigned developer and progress %
  - **Completed** — Finished modules

- Module cards display:

  - Module name
  - Owner (resolved developer name or "unassigned")
  - Progress percentage (0–100%)

- Note: Modules/developers are manually managed (not yet auto-populated from Git)

### 3. Changes Page

- Left pane: List of changed files **from Git status** with status badges:

  - `Modified` — Modified files
  - `Added` — New/untracked files
  - `Deleted` — Deleted files

- Right pane: **Real diff preview** from `git diff` for selected file
- Bottom pane: Commit message input
  - Type freely; press Enter to commit (UI ready, execution pending)
  - Status bar shows commit confirmation

### 4. Merge Visualizer

- Three-pane layout:

  - **Left**: List of changed files from Git
  - **Center**: Local version diff preview
  - **Right**: Incoming version diff preview

- Navigate between panes with Left/Right arrows
- Accept resolution with Enter (tracks choice with green border highlight)
- Focused pane highlighted in yellow, accepted pane in green

### 5. Settings

- **Theme**: Default / High Contrast (applies to status bar styling)
- **Notifications**: On / Off (placeholder)
- **Autosync**: On / Off (placeholder)
- Toggle with Enter, reflects immediately in UI

---

## Screenshots

### Dashboard

![Dashboard](screenshots/Board.png)

### Changes

![Changes](screenshots/Changes.png)

### Merge Visualizer

![Merge](screenshots/Merge.png)

---

## Design Philosophy

- **Git-first mindset** (no abstraction hiding Git concepts)
- **Low cognitive overhead**
- **Keyboard-driven workflow**
- **Developer-centric UI**
- **Explicit > implicit behavior** (no hidden automation)
- **Stateful navigation** — Focus indicates where actions apply

The system assists understanding—it does not make decisions for the developer.

---

## Tech Stack

- **Language:** Rust (2024 edition)
- **TUI Framework:** ratatui 0.29.0
- **Terminal Backend:** crossterm 0.29.0
- **Git Library:** git2 0.20.3 (libgit2 bindings)
- **Error Handling:** color-eyre 0.6.3
- **Other Dependencies:** serde 1.0, uuid 1.19
- **State Management:** In-memory with Git-backed data
- **Architecture Style:** Modular, event-driven, focus-based

---

## Project Structure

```
forge/
├── Cargo.toml
├── LICENSE
├── README.md
├── board.tldr
├── screenshots/
│   ├── Board.png
│   ├── Changes.png
│   └── Merge.png
└── src/
    ├── main.rs              # App entrypoint, state, event loop
    ├── git.rs               # Git repository integration (git2 wrapper)
    ├── key_handler.rs       # Keyboard input → actions
    ├── screen.rs            # Screen layout & view routing
    ├── data.rs              # Data models (Project, Module, Change, Developer)
    └── pages/
        ├── mod.rs
        ├── main_menu.rs         # Top-bar menu navigation
        ├── dashboard.rs         # Project list view with search
        ├── changes.rs           # Git file changes & commit input
        ├── merge_visualizer.rs  # Three-pane merge view with resolution
        ├── project_board.rs     # Kanban board
        ├── settings.rs          # Settings with live toggles
        └── help.rs              # Help overlay
```

---

## Navigation Model

### Focus & State

The app tracks **two focus modes**:

1. **Menu Focus**

   - Top-bar menu is active (highlighted in yellow/bold)
   - Tab/Up/Down cycle or navigate menu items
   - Enter selects an item and switches view
   - Status bar shows "Focus: Menu"

2. **View Focus** (default starting state)
   - Content area (current view) is active
   - Tab cycles between views (Dashboard → Changes → Merge → Board → Settings)
   - Up/Down navigate within the current view (project list, files, etc.)
   - Esc returns focus to menu
   - Menu selection auto-syncs to current view when cycling

### Keyboard Bindings

| Key            | Action                                                    |
| -------------- | --------------------------------------------------------- |
| `Tab`          | Cycle menu items (Menu) OR cycle views (View)             |
| `Up` / `k`     | Navigate up in menu or view                               |
| `Down` / `j`   | Navigate down in menu or view                             |
| `Left` / `h`   | Navigate left (Board columns, Merge panes)                |
| `Right` / `l`  | Navigate right (Board columns, Merge panes)               |
| `Enter`        | Select menu item / commit / toggle setting / accept merge |
| `Esc`          | Back to menu / exit search / close help                   |
| `q` / `Ctrl+C` | Quit immediately from any view                            |
| `?`            | Toggle help overlay                                       |
| `Ctrl+F`       | Toggle project search (Dashboard only)                    |
| `PageUp/Down`  | Scroll long lists                                         |
| Text keys      | Type commit message (Changes) or search query (Dashboard) |
| `Backspace`    | Delete character in text input fields                     |

### Interaction Flow
```
[Menu Focus - "Dashboard" highlighted]
↓ (Enter)
[View Focus - Dashboard view active, menu shows "Dashboard"]
↓ (Tab)
[View Focus - Changes view active, menu shows "Changes"]
↓ (Tab again)
[View Focus - Merge view active, menu shows "Merge"]
↓ (Esc)
[Menu Focus - still on "Merge" in menu, can navigate with arrows]
↓ (from Menu, Esc or q)
[Exit]
```
---

## Data Source

Forge automatically discovers and loads your **current Git repository** on startup:

- **Repository Discovery**: Uses `git2` to find the nearest `.git` folder
- **Project Creation**: Generates a project from:
  - Repository name (from folder name)
  - Current branch (from HEAD)
  - File changes (from `git status`)
  - Diff previews (from `git diff`)
- **Fallback**: If no repo found, starts with empty project list

**Modules and Developers** are currently **manual placeholders** (not auto-populated from Git history).

---

## Status

✅ **Git Integration Active**

### Implemented

- [x] Automatic Git repository discovery
- [x] Real-time file status from `git status`
- [x] Live diff preview generation from `git diff`
- [x] Branch detection and display
- [x] Top-bar menu navigation with focus tracking
- [x] View switching with Tab
- [x] Dashboard with project selection and search (`Ctrl+F`)
- [x] Changes page with real Git file list & diff preview
- [x] Kanban-style project board (manual modules)
- [x] Merge visualizer with resolution tracking
- [x] Settings with live theme/notification toggles
- [x] Help overlay (`?`)
- [x] Status bar with focus/repo/settings badges
- [x] Commit message input (UI ready)
- [x] Merge resolution state tracking
- [x] Theme switching (Default/HighContrast)
- [x] Search with match count display
- [x] Module owner name resolution on board

### In Progress / Not Yet Implemented

- [ ] Git commit execution (staging + committing files)
- [ ] Branch switching and creation
- [ ] Commit history viewing
- [ ] Remote operations (push/pull/fetch)
- [ ] Multi-repo support / repo picker
- [ ] Auto-population of modules from Git data
- [ ] Persistence for manual modules/developers
- [ ] Advanced merge conflict resolution
- [ ] AI/ML inference features

---

## Running the Prototype

### Requirements

- Rust 1.70+
- A terminal with UTF-8 support
- A Git repository (Forge auto-discovers from current directory)

### Build & Run

```bash
cd forge
cargo build --release
cargo run
```

Or run from any Git repository:

```bash
cd /path/to/your/git/project
/path/to/forge/target/release/forge
```

### Demo Flow

1. **Start** — Forge discovers your Git repo and loads file changes
2. **Dashboard** — View shows current repository with real branch/path
3. Press `Ctrl+F` → Search projects (type to filter, Esc to exit)
4. Press `Tab` → Switch to **Changes** view with real Git status
5. **Navigate** files with Up/Down → See live diff preview on right
6. Type a commit message in the bottom pane
7. Press `Enter` → Commit prepared (execution pending implementation)
8. Press `Tab` → **Merge Visualizer** shows files with diff previews
9. Navigate panes with `Left`/`Right`, accept with `Enter`
10. Press `Tab` → **Project Board** shows manual modules (if any)
11. Press `Tab` → **Settings** to toggle theme/notifications
12. Press `?` → Toggle help overlay
13. Press `Esc` → Return to menu focus
14. Press `q` or `Esc` from menu → Quit

---

## Architecture Notes

### App State (`src/main.rs`)

```rust
pub struct App {
    current_view: AppMode,               // Which view is visible
    focus: Focus,                        // Menu or View
    menu_selected_index: usize,          // Which menu item is highlighted
    store: FakeStore,                    // Project data (Git-backed)
    git_client: Option<GitClient>,       // Git repository handle
    git_workdir: Option<PathBuf>,        // Repo path
    settings: AppSettings,               // Theme/notifications/autosync
    merge_resolutions: HashMap<...>,     // Accepted merge decisions
    status_message: String,              // Bottom bar text
    search_active: bool,                 // Search mode flag
    // ... view-specific state (selections, scrolls, commit msg, etc.)
}
```

### Event Handling

1. `KeyHandler` reads terminal events and maps to `KeyAction` enum
2. `App::handle_action()` updates state based on focus & current view
3. `App::render()` passes state to `Screen`
4. `Screen` routes render calls to the appropriate page component
5. Each page is a **stateless render function** consuming read-only state

### Focus-Based Behavior

- Menu has different keybindings than views
- Tab behavior changes based on focus
- Menu selection index syncs with current view when tabbing in View focus
- Esc returns to menu or exits search/help

---

## Roadmap & Future Extensions

### Near-Term (Core Git Operations)

- [ ] Execute Git commits (staging + commit with message)
- [ ] Branch listing and switching
- [ ] Commit history view with log navigation
- [ ] Remote operations (fetch/pull/push)
- [ ] Stash management
- [ ] Repo picker / multi-repo support

### Mid-Term (Automation & Intelligence)

- [ ] Auto-populate modules from commit history
- [ ] Commit-to-task inference
- [ ] Progress tracking from Git activity
- [ ] Developer assignment from Git authors
- [ ] Semantic change analysis
- [ ] Conflict detection and smart merge suggestions

### Long-Term (Advanced Features)

- [ ] Plugin-based Git provider support
- [ ] Persistent configuration and project metadata
- [ ] AI-assisted commit message generation
- [ ] Code review integration
- [ ] CI/CD pipeline status display
- [ ] Collaborative/multiplayer features

---

## Intended Audience

- Developers interested in TUI/CLI design
- Project management practitioners
- Open-source contributors
- System design reviewers

---

## License

MIT

---
