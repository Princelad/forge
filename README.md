# Forge

**A developer-first, terminal-based Git-aware project management system.**

Manage Git repositories, view diffs, track tasks, and collaborate—all from your terminal without leaving your coding flow.

![Changes View](screenshots/Changes.png)

## Quick Start

```bash
cargo build --release
./target/release/forge
```

## Key Features

- 🔀 **Git Integration** — Real-time file status and diff preview
- 📋 **Project Board** — Kanban task tracking (Pending → Current → Completed)
- 🔗 **Branch Management** — Create, switch, and delete branches
- 📝 **Commit History** — Browse and inspect commits
- 👥 **Team Management** — Track modules and developers
- 🔀 **Merge Visualization** — Side-by-side conflict resolution
- ⚙️ **Customizable** — Theme switching and settings
- ⌨️ **Keyboard-Driven** — Fully navigable without mouse

## Documentation

📚 **[Complete Documentation →](docs/wiki/Home.md)**

- **[Getting Started](docs/wiki/Getting-Started.md)** — Installation and first run
- **[Features](docs/wiki/Features.md)** — Complete feature overview
- **[Keyboard Shortcuts](docs/wiki/Keyboard-Shortcuts.md)** — All keybindings
- **[Architecture](docs/wiki/Architecture.md)** — System design
- **[Development](docs/wiki/Development.md)** — Contributing guide
- **[API Reference](docs/wiki/API-Reference.md)** — Core types and functions
- **[Workflows](docs/wiki/Workflows.md)** — User workflows and interaction patterns
- **[Performance](docs/wiki/Performance.md)** — Benchmarks and optimization
- **[Roadmap](docs/wiki/Roadmap.md)** — Future plans
- **[FAQ](docs/wiki/FAQ.md)** — Common questions

## Requirements

- **Rust 1.70+** ([install here](https://rustup.rs/))
- **Git 2.0+**
- **Terminal** with 256-color support

## Build & Run

```bash
# Clone the repository
git clone https://github.com/yourusername/forge.git
cd forge

# Build for production
cargo build --release

# Run in a Git repository
cd /path/to/your/repo
/path/to/forge/target/release/forge
```

## Usage

1. **Navigate** with Tab and Arrow keys
2. **Stage files** with Space
3. **Commit** with Enter
4. **View help** with `?`
5. **Quit** with Ctrl+C

See the **[Keyboard Shortcuts](docs/wiki/Keyboard-Shortcuts.md)** page for complete reference.

## Contributing

Contributions are welcome! See **[CONTRIBUTING.md](CONTRIBUTING.md)** for quick start, or the **[Development](docs/wiki/Development.md)** wiki for comprehensive guidelines.

## License

GPL-3.0-only

---

**Need help?** Check the **[FAQ](docs/wiki/FAQ.md)** or open an issue on GitHub.
