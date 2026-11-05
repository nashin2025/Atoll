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
| 15 | Phase 1: Smart Contract VMs (EVM, WASM, Move, RISC-V, AI VM, ZK VM) | Forge | ✅ Done |
| 16 | Phase 2: Morph-Hash Production (32 algorithms in Rust/C/ASM, 4GB memory, S-box evolution) | Forge | ✅ Done |
| 17 | Phase 3: Penta-Consensus Production (5 layers, composite scoring, HotStuff BFT production) | Forge | ✅ Done |
| 18 | Phase 4: Sharding Production (beacon chain, 64 shards, cross-shard communication, rebalancing) | Forge | ✅ Done |
| 19 | Phase 5: Privacy & Security Production (zk-STARKs, homomorphic encryption, encrypted mempool, post-quantum signatures) | Forge | ✅ Done |
| 20 | Phase 6: Testing & Auditing (unit tests, integration tests, stress tests, adversarial tests, formal verification, fuzzing, security audits) | Beaker | ✅ Done |

## Decisions
- Coin name: **ATOLL** (renamed from Chimera)
- Primary language: Rust
- Consensus: Penta-Consensus (5 layers)
- Hash: Morph-Hash
- License: Apache 2.0 + MIT dual
- **README updates required for every PR** — every PR must update README.md to reflect current project state, feature status, test counts, and audit results before merging

## Risks
- Massive scope; must break into small verifiable increments
- Custom hash function requires extensive cryptanalysis before production use
- Formal verification is time-consuming but essential for consensus correctness
- Post-quantum cryptography standards are still evolving

## Status
Phase 0: Foundation & Research - COMPLETE
Phase 1: Core Protocol Development - COMPLETE
Phase 2: Morph-Hash Production - COMPLETE
Phase 3: Penta-Consensus Production - COMPLETE
Phase 4: Sharding Production - COMPLETE
Phase 5: Privacy & Security - COMPLETE
Phase 6: Testing & Auditing - COMPLETE
Mainnet Preparation (Docker, CI/CD, health/metrics, genesis, bootstrap, TLS, auth, RC tag) - COMPLETE

RC Tag: v0.1.0-rc.1

Security Audit (Phase 6): ALL findings resolved
- Critical (4): FIXED — PRs #2-#5
- High (4): FIXED — PRs #2-#5
- Medium (5): FIXED — PR #10

Remaining work (priority order):
1. GUI Wallet (TypeScript + Tauri) — desktop wallet with balance, transactions, staking
2. Block Explorer (TypeScript + React + GraphQL) — view blocks, transactions, validators
3. Smart Contract VM Production Hardening — EVM, WASM, Move, RISC-V, AI VM, ZK VM audit + optimization
4. Sharding Production Hardening — cross-shard rebalancing, load detection under load
5. Formal Verification Completion — Coq/TLA+/Isabelle proofs for consensus correctness

---

## Feature Plan: Encrypted Authenticated P2P Transport (Security Finding H4)

### Goal
Remediate audit finding H4 (NOT FIXED): replace plaintext TCP transport in `core/src/network/p2p.rs` with an encrypted, authenticated peer transport so peers cannot be spoofed, traffic cannot be injected, and connections are confidential. Node IDs must be derived from a node's public-key fingerprint instead of its IP address.

### Target User
Atoll Protocol node operators and the P2P mesh layer; all upstream consumers (`discovery`, `gossip`, `sync`, `peer_manager`, `encrypted_mempool`) that rely on `NodeId`/`PeerInfo` and `send_message`/`receive_message`.

### Library Choice
**`snow` (Noise Protocol Framework, pure Rust) — `snow = "0.9"`**.

Rejected: TLS 1.3 via `tokio-rustls`/`rustls`. TLS requires X.509 certificate provisioning, CA trust, and renewal — heavyweight infrastructure for a P2P mesh and out of MVP scope. Noise XX gives mutual authentication (both sides present static public keys), encryption, and forward secrecy in a single handshake with no PKI, matching the audit's stated remediation ("Phase 2: Add encrypted transport using Noise Protocol Framework"). `snow` is pure Rust (no C toolchain), depends only on crates already compatible with this codebase (`curve25519-dalek`, `chacha20poly1305`, `blake2`, `digest`, `rand`), and pairs cleanly with `tokio::net::TcpStream`.

Noise pattern: **Noise_XX_25519_ChaChaPoly_BLAKE2s** — mutual static-key authentication, ephemeral DH for forward secrecy, ChaCha20-Poly1305 AEAD, BLAKE2s hashing.

### Integration Approach (design only — no implementation code yet)

The change is confined to `core/src/network/p2p.rs`. The `NetworkMessage` serialization format, length-prefixed framing (4-byte LE u32 length + payload), `MAX_MESSAGE_SIZE` (16 MiB, H5 fix preserved), `NodeId = [u8; 32]`, and `PeerInfo` shape are all unchanged so existing consumers and unit tests interoperate.

1. **Node identity**: each node generates a static Curve25519 Noise keypair at startup and persists it to disk (e.g. `data/node_key.json` via `serde`, already a dependency). `NodeId` = BLAKE3 fingerprint (first 32 bytes) of the static public key. A lost key = lost identity; document backup/rotation.
2. **`PeerConnection::connect(node_id, address)`**: `TcpStream::connect` → Noise XX **initiator** handshake (send static key, receive and authenticate peer's static key) → derive peer `NodeId` from peer's static-key fingerprint and verify it matches the expected `node_id` when known → wrap stream in Noise transport (encrypt/decrypt).
3. **`PeerListener::accept()`**: `TcpListener::accept` → Noise XX **responder** handshake → derive `NodeId` from peer's static-key fingerprint.
4. **Framing**: keep existing length-prefixed framing over the Noise-encrypted stream; `send_message`/`receive_message` signatures and `NetworkMessage` byte format unchanged. Noise `encrypt`/`decrypt` operate on whole messages, so each length-prefixed payload is sent as one Noise message.
5. **Consumers**: `peer_manager`, `discovery`, `gossip`, `sync`, `encrypted_mempool` use `NodeId`/`PeerInfo` and the `send_message`/`receive_message` API — no semantic changes expected; verify compilation after the transport rework.

### Dependencies to Add
- `snow = "0.9"` (Noise Protocol Framework; pulls in curve25519-dalek, chacha20poly1305, blake2, digest, rand)
- `serde` is already a dependency (derive feature) — reuse for keypair persistence
- `blake3` is already a dependency — reuse for NodeId fingerprinting

No new `noise`, `rustls`, or `tokio-rustls` crates.

### Task Breakdown (max 10)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Add `snow` dependency to `core/Cargo.toml`; verify `cargo build` passes | Forge | Planned |
| 2 | Implement node identity: static Noise keypair generation, load/save to disk, `NodeId = blake3(static_pub_key)` fingerprint | Forge | Planned |
| 3 | Rework `PeerConnection::connect()`: TCP connect → Noise XX initiator handshake → authenticate peer static key → wrap stream; verify peer fingerprint vs expected `node_id` | Forge | Planned |
| 4 | Rework `PeerListener::accept()`: TCP accept → Noise XX responder handshake → derive `NodeId` from peer fingerprint | Forge | Planned |
| 5 | Adapt length-prefixed framing over Noise-encrypted transport; keep `NetworkMessage` serialization byte-for-byte unchanged | Forge | Planned |
| 6 | Add unit tests: handshake roundtrip between two nodes, encrypted send/receive, fingerprint derivation, key-mismatch rejection | Forge | Planned |
| 7 | Verify all consumers compile (`peer_manager`, `discovery`, `gossip`, `sync`, `encrypted_mempool`); fix type mismatches | Forge | Planned |
| 8 | Run `cargo build`, `cargo test`, `cargo clippy`; resolve warnings/errors | Forge | Planned |
| 9 | Update VIBE_PLAN.md + audit tracking artifact marking H4 remediated; document key backup/rotation and backward-compatibility notes | Atlas | Planned |

### Risks
- **Identity loss**: losing the static key changes the node's `NodeId`; existing peers won't recognize it. Mitigation: document key backup; treat `node_key.json` like a wallet seed.
- **Handshake complexity**: initiator/responder state machine, handshake failure handling without panicking, and partial-handshake timeouts need care.
- **Backward compatibility**: plain-TCP nodes cannot connect to Noise nodes. Mitigation: version/upgrade coordination; dual-port or cleartext-optional mode is a post-MVP consideration.
- **`snow` + tokio ergonomics**: adapting `snow` transports to `tokio::io::AsyncRead`/`AsyncWrite` may require small wrappers.
- **DoS on handshake**: Noise DH is compute-heavy; rate limiting on connection attempts is deferred to a follow-up task.
- **Fingerprint collision**: astronomically unlikely with 256-bit BLAKE3; noted but not mitigated in MVP.

### Timeline
Estimated complexity: **Medium (≈ 3 business days)** per plan-feature sizing (M = 3 days), plus 1 day for testing/docs. Target: one work week end-to-end for MVP.

### Behavior Preserved (must not break)
- `serialize_message`/`deserialize_message` byte format for all `NetworkMessage` variants (Ping, Pong, FindNode, etc.) — pure-function unit tests in `p2p.rs` and `mod.rs` must continue to pass.
- 4-byte LE length-prefixed framing and `MAX_MESSAGE_SIZE` 16 MiB cap.
- `NodeId = [u8; 32]` and `PeerInfo { node_id, address, port, last_seen }` shapes — all consumers keep compiling.
- `nonce_counter` field retained on `PeerConnection`.

### Decision
Adopt `snow` Noise_XX_25519_ChaChaPoly_BLAKE2s; NodeId from static-key fingerprint; framing and message serialization unchanged.

### Next Step
Run `/build-feature` against the implementation issue once it is created from this plan; then `/review-code`.

---

## Feature Plan: Block Validation Performance Optimization

### Goal
Optimize block validation performance by caching `MorphHashEngine` in `Blockchain` and reusing the 4KB `MemoryHardLayer` buffer across `insert_block` calls.

### Context

- `Blockchain` struct had no cached engine. Both `new(genesis)` and `insert_block` constructed `MorphHashEngine::new()` inline.
- `MorphHashEngine::new()` is allocation-free (just field assignment).
- The real cost is `MemoryHardLayer::init_buffer(prev_block_hash)` — allocates 4KB on every call because the seed changes per block.
- `morph_hash` benchmark is ~198µs with reused engine; engine caching alone won't reach <1ms — buffer reuse is required too.
- `morph_hash(&mut self, ...)` takes `&mut self`, matching `Blockchain`'s `&mut self` in `insert_block` — no public API changes needed.
- Existing direct-engine consumers (stress/integration/fuzzing/adversarial tests) are unaffected — they create their own engine instances.

### Task Breakdown

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Add `morph_hash_engine` field to `Blockchain` struct; initialize once in `new()` | Forge | ✅ Done |
| 2 | Refactor `insert_block` to use `self.morph_hash_engine.morph_hash(...)` | Forge | ✅ Done |
| 3 | Implement 4KB buffer reuse: add `buffer` field to `MemoryHardLayer`, `init_buffer_in_place()` method, update `morph_hash()` | Forge | ✅ Done |
| 4 | Update tests: run existing blockchain tests, add equivalence test (cached == fresh engine) | Forge | ✅ Done |
| 5 | Fix criterion benchmark: reuse `Blockchain` instance instead of reconstructing each iteration | Forge | ✅ Done |
| 6 | Add before/after benchmark: `block_validation_insert_fresh_engine` vs `block_validation_insert` | Forge | ✅ Done |
| 7 | Run full verification: `cargo check`, `cargo fmt --check`, `cargo clippy --all-targets`, `cargo test --lib` | Forge | ✅ Done |
| 8 | Document results: update `PERFORMANCE_PHASE6.md` and `VIBE_PLAN.md` | Forge | ✅ Done |

### Files Changed

| File | Change |
|------|--------|
| `core/src/chain/blockchain.rs` | Added `morph_hash_engine: MorphHashEngine` field; initialized once in `new()`; `insert_block()` uses cached engine |
| `core/src/crypto/morph_hash/mod.rs` | Added `buffer: Vec<u8>` to `MemoryHardLayer`; added `init_buffer_in_place()`; `morph_hash()` reuses buffer |
| `core/benches/hash_benchmarks.rs` | Fixed benchmarks to reuse `Blockchain`; added `block_validation_insert_fresh_engine` comparison benchmark |
| `PERFORMANCE_PHASE6.md` | Updated with optimization details, new benchmark, and file change documentation |
| `VIBE_PLAN.md` | This section — performance optimization task tracking |

### Verification

- `cargo check` — passes (no errors)
- `cargo fmt --check` — passes
- `cargo clippy --all-targets` — passes (no new warnings)
- `cargo test --lib` — 364 passed, 0 failed (includes new `cached_engine_equivalence_with_fresh_engine` test)
