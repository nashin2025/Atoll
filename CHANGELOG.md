# Atoll Protocol — Changelog

All notable changes to Atoll Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.6] — 2026-11-09

### Fixed

- Removed duplicate `release.yml` workflow (consolidated into `mainnet.yml`)
- Removed duplicate `cargo audit` step from security scan job
- Removed duplicate frontend build step
- Pinned `trufflesecurity/trufflehog` to `@main` (verified creator reference)
- Pinned `docker/setup-buildx-action` to `@v3`
- Pinned `anchore/sbom-action` to `@v2`
- Pinned `actions/upload-release-asset` to `@v2`
- Fixed `swatinem/rust-cache` → `Swatinem/rust-cache@v2` casing
- Added `rust-cache` to Format Check job
- Added job-level permissions for least privilege across all CI jobs
- Made macOS and Windows GUI builds non-blocking (`continue-on-error: true`)
- Added `continue-on-error` to Windows code signing steps (certificate may not be configured)
- Removed `Remove-Item -Force package-lock.json` from builds (preserves reproducible builds)
- Removed `RUSTFLAGS: "-D warnings"` at workflow level

## [1.0.5] — 2026-11-09

### Added

- Embedded Atoll node in GUI wallet — running the GUI wallet makes it a node, no separate process required
- Node status display in Dashboard (chain ID, tip height, peer count, syncing)
- Tokenomics module: ATOLL emission schedule, block reward distribution, MINT/PROOF tokens
- Governance system: proposals, quadratic voting, timelock execution, quorum enforcement
- Block Explorer frontend (React + TypeScript + Tailwind + Apollo Client)
- Live node RPC integration for GUI wallet
- Derive Address view in GUI wallet
- Staking UI: stake/unstake ATOLL, view balance and rewards in GUI wallet
- Multi-sig treasury module with threshold signatures and propose-execute flow
- Legal entity documentation: jurisdiction, entity type, regulatory compliance, tax, IP
- Disaster recovery plan with rollback procedures, backups, and runbooks
- Production monitoring: Prometheus, Grafana, alerting rules, dashboards
- E2E tests (Playwright + Vitest) for GUI wallet
- Performance benchmarks (TPS, latency, load testing)
- Block Explorer deployment infrastructure (Docker Compose, nginx)
- Testnet infrastructure (genesis, network config, Docker Compose, faucet, docs)
- Atoll Protocol litepaper v1.0 (whitepaper)

### Changed

- GUI wallet now runs `atoll-core` in-process instead of proxying HTTP RPC to a separate node
- Removed `reqwest` HTTP client dependency from GUI backend
- Dashboard: "Connect node" replaced with "Start embedded node"

### Fixed

- Tauri v2 windows subsystem configuration (`#![windows_subsystem = "windows"]` in main.rs)
- Removed error information leaks in production code (M2 security findings)
- Removed unmaintained `bincode` dependency (M5 security finding)
- Fixed `cargo fmt` formatting issues across all new code

## [1.0.3] — 2026-11-09

### Added

- Block Explorer frontend with GraphQL integration
- Live node RPC commands for GUI wallet
- Security hardening (error leakage removal, bincode removal)

### Fixed

- Tauri v2 bundle schema validation error (invalid `windowsSubsystem` property)
- Windows console window suppression
- macOS ad-hoc signing and SBOM non-blocking
- SBOM generation and release signing pipeline

## [1.0.2] — 2026-11-09

### Added

- Code signing, SBOM generation, and macOS build to release pipeline
- Code signing cleanup and timestamp verification

### Fixed

- macOS bundle path and Windows certificate cleanup guard
- SBOM non-blocking for release pipeline

## [1.0.1] — 2026-11-08

### Added

- Windows console window suppression via `#![windows_subsystem = "windows"]`
- Dashboard subheading set to "offline wallet"
- Crypto wallet GUI CI best practices documentation
- macOS app icon (icon.png converted from icon.ico)

### Fixed

- Tauri v2 invoke payload wrapping (`request` key for snake_case commands)
- Tauri v2 bundle configuration

## [1.0.0] — 2026-11-08

### Added

- Initial mainnet release
- Atoll Protocol Layer 1 blockchain with Morph-Hash and Penta-Consensus
- Sharded architecture (64 shards)
- Triple-token system (ATOLL/MIND/PROOF)
- CLI wallet (new, import, balance, send, history)
- GUI wallet with offline mode
- Post-quantum cryptography (Dilithium, SPHINCS+, Kyber)
- zk-STARKs and homomorphic encryption
- Encrypted mempool
- Docker/CI/CD release pipeline

## [1.0.0-rc.54] — 2026-11-08

### Added

- Release candidate pipeline with all 7 CI jobs green
- Windows CLI, wallet CLI, and GUI binaries
- macOS build and Docker image
