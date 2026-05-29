# Contributing to aicite

Thank you for your interest in contributing! This guide covers everything you need to get started.

## Prerequisites

- **Rust toolchain** (stable). Install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Git** for version control

## Dev Environment Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/risaavedraf/aicite.git
   cd aicite
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run the tests to verify everything works:
   ```bash
   cargo test
   ```

## Development Workflow

### Running Tests

```bash
cargo test
```

### Linting

```bash
cargo clippy -- -D warnings
```

### Formatting

```bash
cargo fmt
cargo fmt --check   # verify without modifying
```

### Full CI Check

Run all checks before submitting a PR:

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

## Pull Request Process

1. **Fork** the repository and create a branch from `main`.
2. **Make your changes** in small, focused commits.
3. **Write or update tests** for any new functionality.
4. **Run the full CI check** locally before pushing.
5. **Open a PR** with a clear description of what changed and why.
6. **Respond to review feedback** promptly.

### Code Style

- Follow standard Rust conventions and idioms.
- Run `cargo fmt` before committing.
- Fix all `cargo clippy` warnings.
- Use meaningful variable and function names.
- Add doc comments for public APIs.
- Keep functions small and focused.

### Commit Messages

- Use clear, descriptive commit messages.
- Prefix with a short tag when relevant: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`.

## Reporting Issues

- Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md) for bugs.
- Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md) for new ideas.
- Include reproduction steps, expected behavior, and your environment details.
- Search existing issues before opening a new one.

## Security

For security vulnerabilities, please see [SECURITY.md](SECURITY.md). Do **not** open a public issue for security reports.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
