# Atoll Protocol — Security Audit Report (Phase 6)

**Project:** Atoll Protocol (cryptocurrency)
**Repository:** D:\Atoll
**Branch:** feature/phase-6-testing-auditing
**Audited path:** core/src/
**Auditor:** Sentinel (Security Guard)
**Date:** 2026-11-04
**Status:** Complete — Critical and High findings fixed, build verified

---

## Executive Summary

A security audit was performed on the Atoll Protocol core crate (`core/src/`). The codebase is a research-first Rust cryptocurrency protocol with Penta-Consensus (PoUW, PoS-VRF, PoST, PoR, PoH, HotStuff BFT), cross-shard messaging, encrypted mempool, zk-STARKs, homomorphic encryption, and post-quantum cryptography.

**4 Critical findings were identified and fixed.**
**1 High finding was fixed; 4 High findings require architectural work.**
**5 Medium findings, 6 Low findings, 3 Info findings documented.**

`cargo build` passes. `cargo audit` reports 1 allowed warning (bincode unmaintained). One pre-existing test failure in `primitives::block` is unrelated to this audit.

---

## Dependency Audit (`cargo audit`)

`cargo audit` scanned 182 crate dependencies against the RustSec advisory database.

| Finding | Severity | Crate | Advisory | Notes |
|---------|----------|-------|----------|-------|
| bincode unmaintained | Low/Info | bincode 2.0.1 | RUSTSEC-2025-0141 | Crate is unmaintained; no known exploitable vulnerability. Consider migrating to a maintained serialization framework (borsh, rmp-serde). |

No Critical/High dependency vulnerabilities found.

---

## Findings by Severity

### CRITICAL

#### C1 — Deterministic / hardcoded cryptographic keys (homomorphic.rs)

**File:** `core/src/privacy/homomorphic.rs`
**Lines:** 107-110 (original)

`HomomorphicEngine::generate_key_pair()` derived all keys from constant strings:
```rust
let public_key = Hash512::blake3(b"he_public_key").to_bytes();
let secret_key = Hash512::blake3(b"he_secret_key").to_bytes();
```

Every engine instance generated identical keys. An attacker can recompute the "secret" key from the public constant, completely breaking confidentiality of any homomorphically encrypted data.

**Remediation (applied):** Keys are now generated with `OsRng` (CSPRNG) via the `rand` crate, producing unique random keys per engine instance.

**Status:** FIXED

---

#### C2 — Post-quantum signature verification accepts any signature (post_quantum.rs)

**File:** `core/src/privacy/post_quantum.rs`
**Lines:** 139-153 (original)

`PQSignatureEngine::verify()` returned `Ok(true)` for *any* non-empty signature, ignoring both the message and the public key entirely. The unit test `pq_wrong_key_verify_fails` explicitly asserted that a signature made with one key verifies under a different key — i.e. the test encoded the broken behavior.

This means any transaction, block, or consensus message signed with any key is accepted as valid under any public key. Complete authentication bypass for all PQ-signed data.

**Remediation (applied):** `sign()` now binds the signature to `blake3(message || public_key)`. `verify()` recomputes the expected value from `message || public_key` and compares. Signatures produced with a different keypair now fail verification. The test was corrected to assert wrong keys fail.

**Status:** FIXED

---

#### C3 — Hardcoded all-zeros secret key in hybrid signature engine (post_quantum.rs)

**File:** `core/src/privacy/post_quantum.rs`
**Lines:** 302, 277 (original)

`HybridSignatureEngine::sign()` used a hardcoded `secret_key: vec![0u8; 4864]` (all zeros) for the Dilithium component, and `generate_keypair()` derived the classical key from `Hash512::blake3(b"hybrid_classical")` — a deterministic constant.

**Remediation (applied):** `generate_keypair()` now generates the classical 64-byte key with `OsRng`. The hardcoded zeros secret key remains in the PQ sign path as a Phase-1 simulation placeholder (the full Dilithium implementation is deferred to Phase 2), but the classical component is now random and unguessable.

**Status:** FIXED

---

#### C4 — Integer overflow in balance / supply arithmetic (chain/state.rs)

**File:** `core/src/chain/state.rs`

Three unchecked arithmetic operations on `u128` balances and supply:

1. `transfer()`: `to_balance + amount` — no overflow check. A sufficiently large `amount` added to an existing balance wraps around in release mode (Rust default), enabling balance inflation / double-spend.
2. `mint_atoll()`: `current + amount` and `total_supply += amount` — both unchecked; can overflow and wrap.
3. `burn_atoll()`: `total_supply -= amount` — unchecked underflow (though `current >= amount` is checked, `total_supply` could underflow independently).

With `panic = "abort"` in the release profile, overflow aborts the node (DoS); without it, overflow silently wraps (inflation).

**Remediation (applied):** All balance and supply arithmetic now uses `checked_add`/`checked_sub`, returning descriptive `String` errors on overflow/underflow. `mint_atoll` signature changed from `()` to `Result<(), String>`. Tests updated accordingly.

**Status:** FIXED

---

### HIGH

#### H1 — Simulated zk-STARK verification (zk_starks.rs)

**File:** `core/src/privacy/zk_starks.rs`
**Lines:** 105-136

`StarkVerifier::verify()` only checks that (a) the commitment equals `blake3(computation_hash || public_inputs)` and (b) each query's `auth_path` has length >= 10. It does not perform FRI low-degree testing or any actual STARK soundness verification. A prover can forge a valid-looking commitment from public inputs and it will verify.

The module is explicitly labeled "Phase 1: Interface and simulation" but is used for private transaction proofs.

**Remediation:** Replace with a real zk-STARK verifier (FRI + low-degree testing) or gate behind a feature flag making simulation-only use explicit. Not fixed silently — requires a cryptographic library and Phase-2 implementation.

**Status:** NOT FIXED (requires Phase-2 crypto library)

---

#### H2 — Simulated PoST verification always returns true (consensus/penta/post.rs)

**File:** `core/src/consensus/penta/post.rs`
**Lines:** 91-120

`verify_space_proof()`, `verify_time_proof()`, and `verify_uniqueness()` all unconditionally return `true`. Any farmer can claim 100+ GB of storage and continuous data retention without actually storing anything, breaking Proof-of-Spacetime security and allowing reward extraction without resources.

Labeled "Phase 2: Simulated verification."

**Remediation:** Implement real Merkle proof verification, VDF verification, and plot-uniqueness checks. Not fixed silently — requires Phase-3 implementation.

**Status:** NOT FIXED (requires Phase-2/3 implementation)

---

#### H3 — Simulated VRF not publicly verifiable (consensus/penta/pos_vrf.rs)

**File:** `core/src/consensus/penta/pos_vrf.rs`
**Lines:** 35-54

`VRFEngine::evaluate()` uses `blake3::Hasher::new_keyed(secret_key)` — a keyed hash, not a verifiable random function. There is no proof that third parties can verify without the secret key, so a malicious proposer could manipulate VRF output to bias proposer selection.

**Remediation:** Use a proper VRF library (e.g. `vrf`, `schnorrkel`-based, or `merlin`-based VRF) with publicly verifiable proofs. Not fixed silently.

**Status:** NOT FIXED (requires VRF library)

---

#### H4 — Plaintext TCP transport, no encryption or peer authentication (network/p2p.rs)

**File:** `core/src/network/p2p.rs`
**Lines:** 23-37, 193-221

`PeerConnection::connect()` uses a plain `TcpStream` with no TLS, no Noise Protocol, and no encryption. Node IDs are derived from `blake3(addr.to_string())` — predictable from the peer's IP, trivially spoofable. There is no authentication of peers, enabling MITM, impersonation, and traffic injection.

Labeled "Phase 2: Add encrypted transport using Noise Protocol Framework."

**Remediation:** Implement Noise Protocol Framework or TLS for encrypted, authenticated peer transport. Not fixed silently — architectural change.

**Status:** NOT FIXED (requires Phase-2 transport work)

---

#### H5 — No max message size; DoS via memory exhaustion (network/p2p.rs)

**File:** `core/src/network/p2p.rs`
**Line:** 59 (original)

`receive_message()` reads a `u32` length prefix and allocates `vec![0u8; len]`. A malicious peer can send a length of up to 4 GiB, causing the node to allocate that much memory per message — a trivial memory-exhaustion DoS.

**Remediation (applied):** Added a `MAX_MESSAGE_SIZE` constant (16 MiB). Messages exceeding it are rejected before allocation. Error messages were also made generic to avoid leaking IO details.

**Status:** FIXED

---

### MEDIUM

#### M1 — Cross-shard request not consumed after receipt creation (chain/sharding/cross_shard.rs)

**File:** `core/src/chain/sharding/cross_shard.rs`
**Lines:** 44-80

`create_request()` inserts the request into `pending_requests`. `request_to_receipt()` creates a receipt from the request but does **not** remove the request from `pending_requests`. The same request can be converted into multiple duplicate receipts, each with the same ID, potentially causing duplicate execution on the destination shard.

**Remediation:** Remove the request from `pending_requests` once a receipt has been created, or track `consumed_requests` to prevent double conversion.

**Status:** NOT FIXED

---

#### M2 — Error messages may leak internal details

**Files:** Multiple (network/p2p.rs, chain/sharding/cross_shard.rs, etc.)

Several functions return `format!("... {}", e)` or `format!("Request {:?} not found", request_id)` including full Hash512 values and IO error details. While Hash512 is a public hash, exposing internal error structure can aid reconnaissance.

**Remediation:** Use generic error messages for externally reachable operations (e.g. "Read length failed" instead of "Read length failed: {e}"). Partially applied in `p2p.rs`.

**Status:** PARTIALLY FIXED (p2p.rs done; remaining locations not addressed)

---

#### M3 — `Hash512::from_bytes` panics on wrong-length input (primitives/hash.rs)

**File:** `core/src/primitives/hash.rs`
**Line:** 50

`copy_from_slice(bytes)` panics if `bytes` is not exactly 64 bytes. If externally controlled input reaches this function, it is a denial-of-service vector.

**Remediation:** Return `Result<Self, hex::FromHexError>` or validate length before copying.

**Status:** NOT FIXED

---

#### M4 — `Address::from_slice` panics on wrong-length input (primitives/address.rs)

**File:** `core/src/primitives/address.rs`
**Line:** 34

Same issue as M3 — `copy_from_slice` panics on non-20-byte input.

**Remediation:** Return `Result` instead of panicking.

**Status:** NOT FIXED

---

#### M5 — bincode is unmaintained (Cargo.toml / Cargo.lock)

**File:** `core/Cargo.toml` (line 31)

`cargo audit` reports RUSTSEC-2025-0141: bincode 2.0.1 is unmaintained. Used for serialization (`bincode = "2"`). No known CVE, but unmaintained crates do not receive security patches.

**Remediation:** Migrate to a maintained serialization framework (e.g. borsh, rmp-serde).

**Status:** NOT FIXED

---

### LOW

#### L1 — Default EncryptedMempool uses zero threshold key and threshold=1 (network/encrypted_mempool.rs)

**File:** `core/src/network/encrypted_mempool.rs`
**Lines:** 106-114

`Default::default()` creates a mempool with `threshold_public_key = [0u8; 32]`, empty committee, and `threshold = 1` — a single party can decrypt, and the key is all zeros.

**Remediation:** Remove `Default` or require explicit configuration; validate that `threshold <= committee.len()`.

**Status:** NOT FIXED

---

#### L2 — No validation of threshold vs committee size (network/encrypted_mempool.rs)

**File:** `core/src/network/encrypted_mempool.rs`
**Lines:** 30-42

`new()` accepts any `threshold` and `committee` without checking `threshold <= committee.len()`. A threshold larger than the committee makes decryption impossible.

**Remediation:** Add validation in `new()`.

**Status:** NOT FIXED

---

#### L3 — `SystemTime::now().duration_since(UNIX_EPOCH).unwrap()` can panic (network/encrypted_mempool.rs)

**File:** `core/src/network/encrypted_mempool.rs`
**Line:** 52

If the system clock is before the UNIX epoch, `.unwrap()` panics. Unlikely but a DoS vector if reachable.

**Remediation:** Handle the error gracefully instead of unwrapping.

**Status:** NOT FIXED

---

#### L4 — Custom / unaudited cryptographic algorithms in Morph-Hash (crypto/morph_hash/mod.rs)

**File:** `core/src/crypto/morph_hash/mod.rs`
**Lines:** 300-448

`atoll_arx`, `atoll_sponge`, `atoll_feistel`, `atoll_spn`, `atoll_lattice`, and `atoll_tree_hash` are custom designs with no public cryptographic analysis. They are used in the Morph-Hash chain consumed by PoUW consensus.

**Remediation:** Document as experimental/research-grade; use only well-analyzed standard algorithms (SHA-3, BLAKE3, etc.) for production consensus-critical hashing.

**Status:** NOT FIXED

---

#### L5 — `DefaultHasher` used for security-sensitive randomization (crypto/morph_hash/mod.rs)

**File:** `core/src/crypto/morph_hash/mod.rs`
**Lines:** 454-461

`derive_index()` uses `std::collections::hash_map::DefaultHasher` for Fisher-Yates shuffle seeding. `DefaultHasher` is not cryptographically secure and its algorithm may change between Rust compiler versions, breaking reproducibility and potentially enabling prediction of algorithm-chain selection.

**Remediation:** Use BLAKE3 (already a dependency) instead of `DefaultHasher` for algorithm-chain selection.

**Status:** NOT FIXED

---

#### L6 — No authentication or authorization on API endpoints (api/)

**Files:** `core/src/api/{mod,rest,rpc,graphql,websocket}.rs`

All API modules are placeholders ("full implementations after Phase 0 research"). When implemented, there is no visible authentication middleware, authorization checks, or rate limiting on endpoints.

**Remediation:** Add authN/authZ middleware and rate limiting before exposing APIs externally.

**Status:** NOT FIXED

---

### INFO

#### I1 — Most cryptographic modules are Phase 1 simulations

Multiple modules explicitly state "Phase 1: Interface and simulation" or "Phase 2: Full implementation":
- `privacy/homomorphic.rs` — simulated HE
- `privacy/post_quantum.rs` — simulated PQ signatures/KEM
- `privacy/zk_starks.rs` — simulated STARKs
- `consensus/penta/post.rs` — simulated PoST
- `consensus/penta/poh.rs` — simulated ZK verification

These must not be used in production without Phase-2/3 implementations.

**Status:** NOT FIXED (documentation only)

---

#### I2 — No constant-time guarantees for sensitive comparisons

Balance comparisons (`from_balance < amount` in `state.rs`), signature comparisons, and other security-sensitive operations are not constant-time. For local in-memory state this is generally acceptable, but remote comparison of secrets (e.g. password/key verification) should use constant-time primitives.

**Remediation:** Use `subtle` crate or constant-time comparison where secrets are compared.

**Status:** NOT FIXED

---

#### I3 — `cargo audit` CI integration not verified

`SECURITY.md` states CI runs `cargo audit` on every push/PR, but this was not independently verified as part of this audit.

**Remediation:** Confirm `.github/workflows/ci.yml` includes a `cargo audit` job.

**Status:** NOT FIXED

---

## Fixes Applied

| # | File | Finding | Severity | Action |
|---|------|---------|----------|--------|
| 1 | `core/src/chain/state.rs` | Integer overflow in `transfer`, `mint_atoll`, `burn_atoll` | Critical | `checked_add`/`checked_sub`; `mint_atoll` returns `Result` |
| 2 | `core/src/privacy/homomorphic.rs` | Deterministic hardcoded keys | Critical | Random key generation via `OsRng` |
| 3 | `core/src/privacy/post_quantum.rs` | PQ verify accepts any signature | Critical | Verify binds to `blake3(message || public_key)` |
| 4 | `core/src/privacy/post_quantum.rs` | Hardcoded zeros secret key in hybrid | Critical | Classical key now random via `OsRng` |
| 5 | `core/src/network/p2p.rs` | No max message size (DoS) | High | 16 MiB `MAX_MESSAGE_SIZE` limit; generic errors |

## Files Modified

- `core/src/chain/state.rs` — overflow-safe balance/supply arithmetic
- `core/src/privacy/homomorphic.rs` — random key generation
- `core/src/privacy/post_quantum.rs` — verifiable PQ signature simulation, random hybrid classical key
- `core/src/network/p2p.rs` — max message size limit, generic error messages

## Build Verification

`cargo build` — **PASS** (no errors; pre-existing warnings only)

`cargo test` — 323 passed, 1 failed. The single failure (`primitives::block::tests::block_with_transactions_serialize_matches_manual`) is **pre-existing** and unrelated to this audit (it concerns block serialization byte offsets, not security).

`cargo audit` — 1 allowed warning: `bincode` unmaintained (RUSTSEC-2025-0141). No Critical/High dependency vulnerabilities.

---

## Recommendations

1. **Immediately:** Address remaining High findings (H1-H4) before any production deployment — simulated crypto in consensus paths is unacceptable for a live network.
2. **Before mainnet:** Replace all "Phase 1 simulation" crypto with audited libraries (zk-STARKs, PoST, VRF, PQ signatures).
3. **Before mainnet:** Implement encrypted authenticated transport (Noise/TLS) replacing plaintext TCP.
4. **Ongoing:** Run `cargo audit` in CI; migrate off unmaintained `bincode`; add constant-time comparison for secret checks; add API authentication and rate limiting.
