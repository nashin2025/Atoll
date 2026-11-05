# Atoll Protocol

A research-first cryptocurrency protocol where every feature goes through **Research → Implement → Test → PR** before merging.

[![CI](https://github.com/nashin2025/Atoll/actions/workflows/ci.yml/badge.svg)](https://github.com/nashin2025/Atoll/actions/workflows/ci.yml)
[![CodeQL](https://github.com/nashin2025/Atoll/actions/workflows/codeql.yml/badge.svg)](https://github.com/nashin2025/Atoll/actions/workflows/codeql.yml)
[![License: Apache-2.0 OR MIT](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE)
[![Discord](https://img.shields.io/discord/placeholder)](https://discord.gg/atoll)

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Project Status](#project-status)
- [Development](#development)
- [Testing](#testing)
- [Security](#security)
- [Contributing](#contributing)
- [License](#license)

## Overview

Atoll Protocol is a research-first cryptocurrency protocol designed for security, privacy, and scalability. Every feature follows a rigorous **Research → Implement → Test → PR** workflow before merging to main.

## Architecture

| Component | Description | Location |
|-----------|-------------|----------|
| Consensus | Penta-Consensus (5 simultaneous layers: PoUW, PoS-VRF, PoST, PoR, PoH + HotStuff BFT finality) | `core/src/consensus/` |
| Hash | Morph-Hash (32 rotating algorithms, Fisher-Yates chain selection, memory-hard layer, AI-evolved S-boxes) | `core/src/crypto/morph_hash/` |
| Architecture | Sharded DAG-Blockchain hybrid with beacon chain (64 shards, cross-shard receipt-based messaging) | `core/src/chain/` |
| Smart Contracts | 6 Virtual Machines (EVM, WASM, Move, RISC-V, AI VM, ZK VM) with gas metering | `core/src/vm/` |
| Privacy | zk-STARKs, homomorphic encryption (BFV/CKKS), encrypted mempool with threshold decryption | `core/src/privacy/` |
| Security | Post-quantum signatures (Dilithium2/3/5, SPHINCS+), post-quantum KEM (Kyber512/768/1024), Hybrid signature engine | `core/src/crypto/post_quantum/` |
| Governance | AI-advised hybrid DAO with quadratic voting, futarchy, timelock proposals | `core/src/governance/` |
| Tokens | Triple-token system (ATOLL value token, MIND gas token, PROOF soulbound reputation token) | `core/src/tokens/` |

## Project Status

| Phase | Feature | Status |
|-------|---------|--------|
| Phase 0 | Foundation & Research — literature review, theoretical design, Python prototypes, formal verification setup | ✅ Complete |
| Phase 1 | Core Protocol — Morph-Hash, Penta-Consensus, P2P networking, Blockchain/DAG, 6 Smart Contract VMs | ✅ Complete |
| Phase 2 | Morph-Hash Production — 32 algorithms in Rust/C/ASM, 4GB memory-hard, S-box evolution | ✅ Complete |
| Phase 3 | Penta-Consensus Production — PoUW, PoS-VRF, PoST, PoR, PoH, HotStuff BFT, slashing | ✅ Complete |
| Phase 4 | Sharding Production — beacon chain, 64 shards, cross-shard messaging, rebalancing | ✅ Complete |
| Phase 5 | Privacy & Security — zk-STARKs, homomorphic encryption, encrypted mempool, post-quantum signatures | ✅ Complete |
| Phase 6 | Testing & Auditing — 477 tests, security audit, performance benchmarks | ✅ Complete |
| Mainnet Prep | Docker, CI/CD, health/metrics, genesis, bootstrap, TLS, auth, RC tag | ✅ Complete |

## Development

### Prerequisites

- **Rust** 1.87+ (stable channel)
- **Node.js** 20+ (for GUI wallet and block explorer)
- **npm** or **yarn** (for frontend dependencies)
- **Docker** 24+ (optional, for containerized development)
- **Git** 2.40+

### Build

```bash
# Clone the repository
git clone https://github.com/nashin2025/Atoll.git
cd Atoll

# Build all workspace crates
cargo build --workspace

# Run the full test suite
cargo test --workspace

# Run clippy linter
cargo clippy --workspace --all-targets -- -D warnings

# Check formatting
cargo fmt --all --check

# Run security audit
cargo audit
```

### Running

```bash
# Run the Atoll node
cargo run --bin atoll-node

# Run the CLI wallet
cargo run --bin wallet-cli -- help

# Run the GUI wallet
cargo run --bin wallet-gui

# Run the block explorer
cd explorer/frontend && npm run dev
```

## Testing

### Test Types

| Type | Count | Location |
|------|-------|----------|
| Unit tests | 324 | `core/tests/` |
| Integration tests | 28 | `core/tests/` |
| Adversarial tests | 81 | `core/tests/` |
| Fuzzing tests | 24 | `core/tests/` |
| Stress tests | 20 | `core/tests/` |
| **Total** | **477** | |

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run unit tests only
cargo test --lib

# Run integration tests
cargo test --test integration

# Run stress tests
cargo test --test stress

# Run adversarial tests
cargo test --test adversarial

# Run fuzzing tests
cargo test --test fuzzing

# Run benchmarks
cargo bench
```

## Security

### Reporting

Security vulnerabilities should be reported privately to [security@atollprotocol.org](mailto:security@atollprotocol.org). **Do not** disclose security issues publicly.

See [SECURITY.md](SECURITY.md) for our full security policy.

### Security Features

- Post-quantum signatures (Dilithium2/3/5, SPHINCS+)
- Post-quantum KEM (Kyber512/768/1024)
- Homomorphic encryption (BFV/CKKS)
- zk-STARKs (zero-knowledge proofs)
- Encrypted mempool (threshold decryption)
- Encrypted keystore (XChaCha20-Poly1305 + argon2id)
- Noise Protocol (Noise_XX_25519_ChaChaPoly_BLAKE2s)
- Rate limiting (per-IP, configurable)
- CORS whitelist
- Security headers (HSTS, CSP, X-Frame-Options)

## Contributing

We welcome contributions! Please read our [Contributing Guide](CONTRIBUTING.md) for details on our development workflow, commit conventions, and pull request process.

### Quick Start

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/<feature-name> main`
3. Implement your changes following the Research → Implement → Test → PR workflow
4. Run all checks: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
5. Open a pull request targeting `main`

## License

Atoll Protocol is licensed under the [Apache-2.0 OR MIT](LICENSE) license.

## Support

- [Documentation](https://docs.atollprotocol.org)
- [GitHub Discussions](https://github.com/nashin2025/Atoll/discussions)
- [GitHub Issues](https://github.com/nashin2025/Atoll/issues)
- [Security](mailto:security@atollprotocol.org)
