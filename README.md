# Forge

**Forge** is a developer-first, terminal-based prototype for a Git-aware project management system.
It explores how version control context and lightweight task tracking can coexist in a single workflow, without replacing Git or enforcing heavyweight project management processes.

This prototype focuses on **user experience, navigation, and interaction flow**, not real repository analysis or automation.

---

## Project Objective

The goal of Forge is to validate the usability of a **Git-integrated project board and change visualization system** through a **Text User Interface (TUI)**.

This prototype answers one core question:

> _Can a developer manage project context, tasks, and code changes from a single terminal interface without breaking their coding flow?_

---

## Scope of This Prototype

### Included

- Terminal-based UI using **Rust + ratatui**
- Stateful menu and view navigation with focus tracking
- Mock data for:

  - Projects
  - Modules / tasks with ownership
  - Assigned developers
  - File changes (with status: modified, added, deleted)
  - Progress simulation

- Keyboard-driven interactions (Tab, arrows, Enter, Esc)
- Multi-pane layouts for complex views
- Commit message input and progress simulation

### Explicitly Excluded

- Real Git repository parsing
- Commit history analysis
- LOC-based metrics
- Automation or AI
- Networking or persistence

This is **not** a production system—it is a **proof-of-concept UI and interaction prototype**.

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
  - Owner (assigned developer)
  - Progress bar (0–100%)

### 3. Changes Page

- Left pane: List of changed files with status badges:

  - `[M]` — Modified
  - `[A]` — Added
  - `[D]` — Deleted

- Right pane: Diff preview (mock text) for selected file
- Bottom pane: Commit message input
  - Type freely; press Enter to simulate commit
  - Commit bumps progress on current module by 5%
  - Status bar shows "Committed: [message]"

### 4. Merge Visualizer

- Three-pane layout:

  - **Left**: List of changed files
  - **Center**: Local file version (mock code)
  - **Right**: Incoming file version (mock code)

- Visual concept proof for merge conflict presentation

### 5. Settings

- Placeholder view with mock configuration options

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
- **TUI Framework:** ratatui 0.29
- **Terminal Backend:** crossterm 0.28
- **State Management:** In-memory (mock state only)
- **Architecture Style:** Modular, event-driven, focus-based

---

## Project Structure

```
forge/
├── src/
│   ├── main.rs           # App state, event loop, navigation logic
│   ├── key_handler.rs    # Keyboard input → actions
│   ├── screen.rs         # Screen layout & view routing
│   ├── data.rs           # Mock data store (projects, modules, developers)
│   ├── pages/
│   │   ├── mod.rs
│   │   ├── main_menu.rs  # Left sidebar menu with focus indicator
│   │   ├── dashboard.rs  # Project list view
│   │   ├── changes.rs    # File changes & commit input
│   │   ├── merge_visualizer.rs  # Three-pane merge view
│   │   ├── project_board.rs     # Kanban board
│   │   └── settings.rs          # Settings placeholder
├── Cargo.toml
└── README.md
```

---

## Navigation Model

### Focus & State

The app tracks **two focus modes**:

1. **Menu Focus** (starting state)

   - Left pane (main menu) is active
   - Tab/Up/Down cycle or navigate menu items
   - Enter selects an item and switches view
   - Border changes to green to indicate focus

2. **View Focus**
   - Right pane (current view) is active
   - Tab cycles between views
   - Up/Down navigate within the current view (project list, files, etc.)
   - Esc returns focus to menu
   - Menu selection auto-syncs to current view

### Keyboard Bindings

| Key            | Action                                                       |
| -------------- | ------------------------------------------------------------ |
| `Tab`          | Cycle menu items (Menu) OR cycle views (View)                |
| `Up` / `k`     | Navigate up in menu or view                                  |
| `Down` / `j`   | Navigate down in menu or view                                |
| `Left` / `h`   | Navigate left (reserved for future use)                      |
| `Right` / `l`  | Navigate right (reserved for future use)                     |
| `Enter`        | Select menu item and open view; OR commit message on Changes |
| `Esc`          | Back to menu (or exit from menu)                             |
| `q` / `Ctrl+C` | Quit immediately from any view                               |
| Text keys      | Type commit message (on Changes view only)                   |
| `Backspace`    | Delete character (on Changes view only)                      |

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

## Mock Data

The app includes a `FakeStore` with:

- **2 Projects:**

  - Forge (mock Git-aware project manager)
  - Atlas (internal tooling demos)

- **Per Project:**
  - 3 mock Modules (Auth, Dashboard, Merge UI)
  - Status: Pending, Current, Completed
  - Assigned developers: Alice, Bob, Carol
  - Progress scores: 10–100%
  - Changed files: src/lib.rs, README.md, atlas/core.rs, etc.

All data is immutable except for mock progress bumps on commit.

---

## Status

✅ **Prototype Ready**

### Implemented

- [x] Menu-based navigation with focus tracking
- [x] View switching with Tab
- [x] Back navigation (Esc) with history
- [x] Mock data store (projects, modules, developers, changes)
- [x] Dashboard with project selection
- [x] Changes page with file list & diff preview
- [x] Kanban-style project board
- [x] Merge visualizer (concept proof)
- [x] Settings placeholder
- [x] Bottom status bar with shortcuts & status messages
- [x] Commit message input & mock progress simulation
- [x] Menu selection sync with current view

### Not Implemented (Future Work)

- Real Git integration
- Diff parsing
- SCM hooks
- Persistence
- AI inference
- Toast notifications / ephemeral messages
- Module reassignment (keyboard interaction)
- Scrolling within long lists

---

## Running the Prototype

### Requirements

- Rust 1.70+
- A terminal with UTF-8 support

### Build & Run

```bash
cd forge
cargo run
```

### Demo Flow

1. Start — Menu is focused on "Dashboard"
2. Press `Enter` → Dashboard view opens
3. Press `Down` → Scroll project list (if multiple projects)
4. Press `Tab` → Switch to "Changes" view; menu syncs
5. Type a commit message
6. Press `Enter` → Commit simulated; progress bumped
7. Press `Tab` again → "Project Board" view
8. Observe progress in Current modules
9. Press `Esc` → Return to menu
10. Press `q` or `Esc` again → Quit

---

## Architecture Notes

### App State (`src/main.rs`)

```rust
pub struct App {
    current_view: AppMode,        // Which view is visible
    prev_view: AppMode,           // For back navigation
    focus: Focus,                 // Menu or View
    menu_selected_index: usize,   // Which menu item is highlighted
    store: FakeStore,             // Mock data
    status_message: String,       // Bottom bar text
    // ... view-specific state (selected projects, commit msg, etc.)
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
- Esc always returns to menu (unless already in menu, then exits)

---

## Intended Audience

- Developers interested in TUI/CLI design
- Project management practitioners
- Open-source contributors
- System design reviewers

---

## Future Extensions (Out of Scope for Prototype)

- Git repository introspection
- Commit-to-module inference
- Semantic change analysis
- Progress suggestion engine
- Plugin-based Git provider support
- Real persistence layer
- Multiplayer/collaborative features

---

## License

MIT

---
