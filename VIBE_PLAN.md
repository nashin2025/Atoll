# Atoll Protocol - Project Plan

## Goal
Build a research-first cryptocurrency protocol where every feature undergoes thorough research, implementation, testing, and PR review before merging.

## Target User
Cryptocurrency researchers, developers, and users who value formal verification, post-quantum security, and privacy-preserving transactions.

## MVP Scope (Phase 0-1)
- Research documentation and specifications
- Core protocol skeleton in Rust
- Morph-Hash algorithm research and prototype
- Penta-Consensus design documentation
- Project structure and CI/CD foundation

## Tech Stack
- **Core Node**: Rust (memory safety, speed)
- **Consensus**: Rust + formal spec in Coq/TLA+
- **Hash Algorithm**: Rust + C + ASM (inner loops)
- **Networking**: Rust (tokio async runtime)
- **Smart Contract VM**: Rust (WASM) + C++ (EVM)
- **CLI Wallet**: Rust
- **GUI Wallet**: TypeScript + Tauri (Rust)
- **Block Explorer**: TypeScript + React + GraphQL
- **Formal Proofs**: Coq, TLA+, Isabelle

## Tasks (Max 10 for MVP)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Initialize repository and project structure (Atoll naming) | Forge | ✅ Done |
| 2 | Phase 0: Literature review - consensus mechanisms, hash functions, blockchain architectures | Scout | ✅ Done |
| 3 | Phase 0: Theoretical design - Morph-Hash spec, Penta-Consensus formal model | Atlas | ✅ Done |
| 4 | Phase 0: Proof of concept - Python prototypes of core algorithms | Forge | ✅ Done |
| 5 | Phase 0: Academic paper submissions and formal verification setup | Scribe | ✅ Done |
| 6 | Phase 0: Development roadmap finalization | Atlas | ✅ Done |
| 7 | Set up CI/CD pipeline with research → test → PR workflow | Clockwork | ✅ Done |
| 8 | Create .env.example and documentation scaffolding | Scribe | ✅ Done |
| 9 | Security baseline - secret scanning, dependency audit | Sentinel | ✅ Done |
| 10 | Phase 1 kickoff: Core protocol development begins | Forge | ✅ Done |
| 11 | Phase 1: Morph-Hash Rust implementation (algorithm pool, selection chain, memory-hard, S-box, post-quantum) | Forge | ✅ Done |
| 12 | Phase 1: Penta-Consensus Rust implementation (5 layers, composite scoring, HotStuff BFT) | Forge | ✅ Done |
| 13 | Phase 1: P2P networking (encrypted transport, gossip, sync, discovery) | Forge | ✅ Done |
| 14 | Phase 1: Blockchain/DAG layer (blockchain state, DAG, GHOSTDAG fork choice, beacon chain) | Forge | ✅ Done |
| 15 | Phase 1: Smart Contract VMs (EVM, WASM, Move, RISC-V, AI VM, ZK VM) | Forge | next |

## Decisions
- Coin name: **ATOLL** (renamed from Chimera)
- Primary language: Rust
- Consensus: Penta-Consensus (5 layers)
- Hash: Morph-Hash
- License: Apache 2.0 + MIT dual

## Risks
- Massive scope; must break into small verifiable increments
- Custom hash function requires extensive cryptanalysis before production use
- Formal verification is time-consuming but essential for consensus correctness
- Post-quantum cryptography standards are still evolving

## Status
Phase 0: Foundation & Research - COMPLETE
Phase 1: Core Protocol Development - IN PROGRESS (Tasks 10-14 complete, Task 15 next)
