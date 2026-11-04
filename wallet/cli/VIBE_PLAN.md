# CLI Wallet — Atoll Protocol (VIBE_PLAN.md)

## Goal
Build the first user-facing component for the Atoll Protocol: a command-line wallet that lets users generate/import keys, query balances, send transactions, and view history. The protocol core is complete; users currently have no way to interact with it.

## Target User
Atoll Protocol users and developers who need to hold ATOLL, send transactions, and inspect on-chain state from the command line.

## MVP Scope
- `wallet new` — generate Ed25519 key pair, save to keystore file
- `wallet import <private-key-hex>` — import an existing key
- `wallet balance <address>` — query ATOLL balance for an address
- `wallet send <from> <to> <amount> <token>` — build, sign, and submit a transaction
- `wallet history <address>` — view transaction history for an address

Deferred (post-MVP): Dilithium/post-quantum signing, MIND/PROOF per-address token balances, encrypted keystore, RPC submission to a live node, hardware wallet support.

## Tech Stack
- Language: Rust (edition 2021)
- CLI parsing: clap v4 (derive)
- Crypto: `ed25519` crate for key generation + signing (core's signature modules are placeholders)
- Serialization: serde + serde_json for keystore
- Encoding: hex
- Core dependency: atoll-core (path) for `Address`, `Transaction`, `Signature`, `ChainState`, `Blockchain`
- Keystore location: `~/.atoll/keystore.json` (configurable via `--keystore` flag)

## Approach
The wallet is a new `wallet-cli` crate added to the workspace (`wallet/cli/`). It depends on `atoll-core` for core types.

Because `core/src/crypto/signatures/*` and `core/src/tokens/*` are placeholder modules (no real key generation, signing, or per-address multi-token balances), the wallet must bring its own Ed25519 implementation for MVP. The `Signature` struct (ed25519 + dilithium) is populated with a real ed25519 signature and an empty dilithium field, to be replaced when post-quantum signing is wired in.

Because all API modules (RPC/REST/GraphQL/WebSocket) are placeholders, there is no running node to query. MVP balance/history/send operate against a locally loaded `ChainState` (offline mode) and/or output the signed transaction hex for manual submission. Node RPC integration is deferred.

Address derivation: hash of the Ed25519 public key, truncated to 20 bytes, matching the existing `Address` type.

## Task Breakdown (max 10)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Create `wallet-cli` crate (`wallet/cli/Cargo.toml`), add to workspace `Cargo.toml`, add dependencies (clap, ed25519, serde_json, hex, atoll-core) | Forge | ✅ Complete |
| 2 | Implement keystore: load/save JSON key file at `~/.atoll/keystore.json`, key entry with address + hex private key | Forge | ✅ Complete |
| 3 | Implement `wallet new`: generate Ed25519 keypair, derive Address, write to keystore, print address | Forge | ✅ Complete |
| 4 | Implement `wallet import <hex>`: parse hex private key, derive Address, write to keystore | Forge | ✅ Complete |
| 5 | Implement `wallet balance <address>`: query ATOLL balance from local ChainState (offline) with `--node` RPC flag stubbed | Forge | ✅ Complete |
| 6 | Implement `wallet send <from> <to> <amount> <token>`: build Transaction, sign with local key, output signed tx hex; `--node` submission stubbed | Forge | ✅ Complete |
| 7 | Implement `wallet history <address>`: list transactions from local tx log or `--node` (stubbed) | Forge | ✅ Complete |
| 8 | Wire clap command tree, shared error handling, and `--keystore`/`--node` global flags | Forge | ✅ Complete |
| 9 | Unit tests: key roundtrip (new→import→address), tx signing + `serialize_bytes` determinism, address derivation | Forge | ✅ Complete |
| 10 | `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`; update workspace; document usage in README | Forge | ✅ Complete |

## Dependencies (new crates for wallet-cli)
- `ed25519 = "2"` — real Ed25519 keypair generation and signing (core has none)
- `serde_json = "1"` — keystore persistence
- `clap = { version = "4", features = ["derive"] }` — CLI parsing
- `hex = "0.4"` — hex encode/decode
- `rand = "0.8"` — random key generation
- `atoll-core = { path = "../../core" }` — core types
- `serde` with `derive` — already in core

## Risks
1. **Placeholder signature/token modules** — `core/src/crypto/signatures/*` and `core/src/tokens/*` are placeholders. No real Ed25519 signing or per-address MIND/PROOF balances exist. Mitigation: wallet brings its own `ed25519` crate; MVP `send` limited to ATOLL; dilithium field left empty (replaced post-MVP).
2. **No live node API** — RPC/REST/GraphQL/WebSocket are placeholders. `balance`/`send`/`history` cannot query or submit to a running node in MVP. Mitigation: offline `ChainState` queries + signed tx hex output; RPC integration deferred.
3. **Block insertion requires full consensus** — `Blockchain::insert_block` needs `PentaValidation` (PoUW/PoS/PoST/PoR/PoH). Wallet cannot append blocks directly. Mitigation: signed tx output for later submission via node RPC.
4. **Keystore security** — plaintext JSON keystore is insecure. Mitigation: document risk; encryption is post-MVP.
5. **Address derivation ambiguity** — core's `Address` has no `from_public_key` method. Wallet must define derivation (hash of pubkey → 20 bytes) consistently.

## Timeline
Estimated complexity: Medium (~3 business days) per plan-feature sizing (M = 3 days), plus 1 day for testing/docs. Target: one work week end-to-end for MVP.

| Phase | Work | Estimate |
|---|---|---|
| Scaffold + keystore | Tasks 1–2 | 1 day |
| Key commands (new/import) | Tasks 3–4 | 1 day |
| Chain commands (balance/send/history) | Tasks 5–7 | 1.5 days |
| CLI wiring + tests | Tasks 8–9 | 1 day |
| Verification + docs | Task 10 | 0.5 day |

## Next Step
Run `/build-feature` against the implementation issue once created from this plan, then `/review-code`.
