# CONTRIBUTING.md

# Contributing to Atoll Protocol

Thank you for your interest in contributing to Atoll Protocol! This document outlines the process for contributing code, documentation, and other improvements.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Security Policy](#security-policy)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Commit Message Conventions](#commit-message-conventions)
- [Branch Naming](#branch-naming)
- [Testing Expectations](#testing-expectations)
- [Documentation Expectations](#documentation-expectations)
- [Pull Request Process](#pull-request-process)
- [Review Requirements](#review-requirements)
- [Merge Strategy](#merge-strategy)

## Code of Conduct

This project adheres to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Security Policy

Please refer to our [Security Policy](SECURITY.md) for details on how to report security vulnerabilities. **Do not** disclose security issues publicly — use the private reporting channel described in SECURITY.md.

## Getting Started

### Prerequisites

- **Rust** 1.87+ (stable channel)
- **Node.js** 20+ (for GUI wallet and block explorer)
- **npm** or **yarn** (for frontend dependencies)
- **Docker** 24+ (optional, for containerized development)
- **Git** 2.40+

### Getting the Source

```bash
git clone https://github.com/nashin2025/Atoll.git
cd Atoll
```

### Build / Test / Lint

```bash
# Build all workspace crates
cargo build --workspace

# Run the full test suite
cargo test --workspace

# Check formatting
cargo fmt --all --check

# Run clippy linter
cargo clippy --workspace --all-targets -- -D warnings

# Run security audit
cargo audit

# Format code in place
cargo fmt --all
```

## Development Workflow

### 1. Create a Feature Branch

All work must be done on a feature branch, never directly on `main`:

```bash
git checkout -b feat/<feature-name> main
# or
git checkout -b fix/<bug-description> main
```

Branch naming conventions:

| Type | Description |
|------|-------------|
| `feat/<name>` | New features |
| `fix/<name>` | Bug fixes |
| `docs/<name>` | Documentation changes |
| `perf/<name>` | Performance improvements |
| `refactor/<name>` | Code refactoring |
| `test/<name>` | Test additions or improvements |
| `chore/<name>` | Maintenance tasks |
| `security/<name>` | Security fixes |

### 2. Implement Your Changes

Follow the **Research → Implement → Test → PR** workflow:

1. **Research** — Understand the problem, read relevant specs and code
2. **Implement** — Write clean, idiomatic Rust with full error handling
3. **Test** — Add unit, integration, and adversarial tests
4. **PR** — Open a pull request after all checks pass

### 3. Run Pre-Commit Checks

Before committing, ensure:

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## Commit Message Conventions

Atoll Protocol uses [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation changes |
| `style` | Formatting, missing semicolons, etc. (no code change) |
| `refactor` | Code refactoring (no behavior change) |
| `perf` | Performance improvements |
| `test` | Adding or correcting tests |
| `chore` | Maintenance tasks |
| `security` | Security fixes |

### Scope

The scope should be the crate, module, or component affected (e.g., `vm`, `consensus`, `network`, `wallet-cli`, `explorer`).

### Examples

```
feat(vm): add Move VM bytecode validation
fix(consensus): resolve Penta-Consensus slashing edge case
docs(api): document GraphQL endpoint authentication
style(cli): align CLI output formatting
refactor(crypto): unify hash function interfaces
perf(chain): cache MorphHashEngine in Blockchain
test(network): add adversarial gossip test cases
chore(deps): update winterfell to v0.13.1
security(network): encrypt P2P transport with Noise XX
```

## Testing Expectations

- **Unit tests**: All new code must have unit tests in `#[cfg(test)]` modules
- **Integration tests**: Cross-module scenarios in `core/tests/`
- **Adversarial tests**: Invalid inputs, edge cases, malformed data
- **Fuzzing tests**: Property-based testing with `proptest` or `quickcheck`
- **Stress tests**: High-concurrency, high-throughput scenarios
- **Coverage target**: 95%+ line coverage on all changed files

## Documentation Expectations

When making code changes, you must update:

- `README.md` — Project status, architecture, test results
- `VIBE_PLAN.md` — Task tracking and phase status
- `audits/SECURITY_AUDIT_PHASE6.md` — Security findings (if security-related)
- `PERFORMANCE_PHASE6.md` — Performance benchmarks (if performance-related)

If you add new public APIs, document them with `///` doc comments.

## Pull Request Process

1. **Push your feature branch** to GitHub
2. **Open a pull request** targeting `main`
3. **Fill out the PR template** completely
4. **Ensure all CI checks pass**
5. **Request review from CODEOWNERS**
6. **Address review feedback**
7. **Merge after approval**

### PR Template

All PRs must use the [pull request template](.github/PULL_REQUEST_TEMPLATE.md).

### Squash Commits

All PRs are squash-merged into `main`. Ensure your commit history is clean and descriptive before submitting.

## Review Requirements

- **At least one human review** is required for all PRs
- **CODEOWNERS-based review** — file owners are automatically requested
- **All CI checks must pass** before merge
- **No merge conflicts** with `main`
- **Up-to-date branch** — rebase onto latest `main` before merge

## Merge Strategy

- **Squash merge** — All PR commits are squashed into a single commit on `main`
- **No direct pushes** to `main` — all changes must go through PR review
- **Force pushes** are not allowed on `main`
- **No empty commits** on `main`

## License

By contributing to Atoll Protocol, you agree that your contributions will be licensed under the project's [LICENSE](LICENSE).
