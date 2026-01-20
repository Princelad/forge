# Contributing to Forge

Thank you for your interest in contributing to Forge! We welcome contributions from the community.

## Quick Links

For comprehensive development information, please see the **[Development Wiki](docs/wiki/Development.md)**.

The Development wiki covers:

- **Setting up your development environment**
- **Code quality standards** (formatting, linting, error handling, performance, testing)
- **Project structure** and architecture
- **Adding new screens** (step-by-step guide)
- **Testing** (unit, integration, benchmarks)
- **Git workflow** and commit message format
- **Common development tasks**
- **Debugging tips**
- **Performance considerations**

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/forge.git
   cd forge
   ```
3. **Install Rust** (1.70+): https://rustup.rs/
4. **Build the project**:
   ```bash
   cargo build
   ```
5. **Run tests** to ensure everything works:
   ```bash
   cargo test --lib
   cargo clippy
   cargo fmt --check
   ```

## Development Workflow

1. **Create a feature branch**:

   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**:
   - Follow the code quality standards in the [Development wiki](docs/wiki/Development.md)
   - Add tests for new functionality
   - Run `cargo clippy` to check for warnings
   - Run `cargo fmt` to format your code

3. **Test your changes**:

   ```bash
   cargo test --lib
   cargo build --release
   ```

4. **Commit your changes**:

   ```bash
   git add .
   git commit -m "feat: add new feature"
   ```

   Follow [Conventional Commits](https://www.conventionalcommits.org/) format:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `refactor:` for code refactoring
   - `test:` for adding tests
   - `chore:` for maintenance tasks

5. **Push to your fork**:

   ```bash
   git push origin feature/your-feature-name
   ```

6. **Open a Pull Request** on GitHub

## Code Quality Standards

Before submitting a pull request:

- ✅ **Run `cargo clippy`** and fix all warnings
- ✅ **Run `cargo fmt`** to format code
- ✅ **Run `cargo test --lib`** and ensure all tests pass
- ✅ **Add tests** for new functionality
- ✅ **Update documentation** if needed (README, wiki, doc comments)

## Need Help?

- **[Development Wiki](docs/wiki/Development.md)** — Comprehensive development guide
- **[Architecture Wiki](docs/wiki/Architecture.md)** — System design and structure
- **[FAQ Wiki](docs/wiki/FAQ.md)** — Common questions
- **GitHub Issues** — Ask questions or report bugs

## Code of Conduct

Be respectful and constructive in all interactions. We aim to foster an inclusive and welcoming community.

## License

By contributing to Forge, you agree that your contributions will be licensed under the GPL-3.0-only license.

---

Thank you for contributing to Forge! 🚀
