# Atoll Protocol — Contributing Guide

Thank you for contributing to Atoll Protocol! This guide covers how to set up the development environment, run tests, and submit pull requests.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Submitting a Pull Request](#submitting-a-pull-request)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Security](#security)

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). By participating, you agree to uphold this code. Please report unacceptable behavior to the project maintainers.

## Getting Started

### Prerequisites

- **Rust** (stable toolchain) — [rustup.rs](https://rustup.rs/)
- **Node.js** v20+ — for GUI wallet and block explorer frontend
- **Git** — for version control
- **Tauri CLI** (optional, for GUI development) — `cargo install tauri-cli`

### Clone the Repository

```bash
git clone https://github.com/nashin2025/Atoll.git
cd Atoll
```

### Build the Core Protocol

```bash
cd core
cargo build --release
cargo test --lib
```

### Build the GUI Wallet

```bash
cd wallet/gui
cargo build --release
cargo test
npm install
npm run build
```

### Build the Block Explorer

```bash
cd explorer/frontend
npm install
npm run build
```

## Development Workflow

Atoll Protocol follows a **fork-and-pull** model with trunk-based development:

1. **Fork** the repository to your GitHub account
2. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/<your-feature-name>
   ```
3. **Implement your changes** following the coding standards below
4. **Run tests** and verify everything passes:
   ```bash
   cargo test --workspace
   npm test  # for GUI and explorer
   ```
5. **Format your code**:
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   ```
6. **Commit** using conventional commits:
   ```bash
   git commit -m "feat(gui): add new feature"
   ```
7. **Push** and open a pull request against `main`

## Submitting a Pull Request

1. Push your branch to your fork
2. Open a PR against `main` in the Atoll repository
3. Use the [PR template](.github/PULL_REQUEST_TEMPLATE.md) — fill out all sections
4. Ensure all CI checks pass (format, lint, tests, security scans)
5. Address any review feedback
6. A maintainer will merge your PR after approval

### PR Requirements

- All tests must pass (`cargo test --workspace`)
- Code must be formatted (`cargo fmt --all --check` passes)
- Clippy must pass with no warnings (`cargo clippy --workspace --all-targets -- -D warnings`)
- No hardcoded secrets or credentials
- New features must include tests
- Documentation must be updated for user-facing changes

## Coding Standards

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `clippy::all` lints (pedantic is allowed but not required)
- All public items must have doc comments (`///`)
- Error types must implement `std::error::Error` and be serializable for IPC
- Avoid `unwrap()` in production code — use proper error handling

### TypeScript / React

- Use TypeScript strict mode
- Follow the existing component patterns in `src/components/ui.tsx`
- All user-facing strings must be externalized for future i18n
- Components must be accessible (ARIA labels, keyboard navigation)

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(scope): description
fix(scope): description
docs(scope): description
style(scope): description
refactor(scope): description
test(scope): description
chore(scope): description
```

Examples:
- `feat(gui): add embedded node support`
- `fix(core): resolve genesis hash mismatch`
- `docs(core): document tokenomics model`

## Testing

### Rust

```bash
# Run all library unit tests
cargo test --lib

# Run all tests including integration
cargo test --workspace

# Run tests for a specific crate
cargo test -p atoll-core
cargo test -p wallet-cli
cargo test -p atoll-gui
```

### Frontend (Vitest)

```bash
cd wallet/gui
npm test

cd explorer/frontend
npm test
```

### CI

All tests run automatically on push and PR via [`.github/workflows/ci.yml`](.github/workflows/ci.yml):

- Format check (`cargo fmt --check`)
- Clippy lint (`cargo clippy -- -D warnings`)
- Unit tests (`cargo test --lib`)
- Integration tests (`cargo test --test integration`)
- Security scans (`cargo audit`, `cargo deny`, TruffleHog, detect-secrets)
- Build (`cargo build --release --locked`)
- Frontend tests and build

## Security

### Reporting Vulnerabilities

See [SECURITY.md](SECURITY.md) for how to report security vulnerabilities.

### Security Guidelines

- **Never** hardcode secrets, API keys, or credentials
- All user inputs must be validated and sanitized
- Error messages must not leak internal details
- Use parameterized queries for all database operations
- Authentication and authorization checks on all protected endpoints

## Getting Help

- Open a [discussion](https://github.com/nashin2025/Atoll/discussions) for questions
- Open an [issue](https://github.com/nashin2025/Atoll/issues) for bugs or feature requests
- Contact the maintainer: @nashin2025

## License

This project is licensed under the terms in [LICENSE](LICENSE).
