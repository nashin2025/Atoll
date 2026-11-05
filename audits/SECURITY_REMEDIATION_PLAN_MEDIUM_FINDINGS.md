# Atoll Protocol — Medium-Priority Security Findings Remediation Plan

**Project:** Atoll Protocol (cryptocurrency)
**Repository:** D:\Atoll
**Branch:** `main`
**Source audit:** `audits/SECURITY_AUDIT_PHASE6.md` (Phase 6, complete; Critical & High fixed; Medium NOT FIXED)
**Owner:** Atlas (Planner)
**Status:** Plan only — no implementation code written yet

---

## Scope

Remediate the five **Medium** findings documented in the Phase 6 security audit:

| ID | Finding | File | Status |
|----|---------|------|--------|
| M1 | Cross-shard request not consumed after receipt creation | `core/src/chain/sharding/cross_shard.rs` | NOT FIXED |
| M2 | Error messages may leak internal details | `cross_shard.rs`, `network/p2p.rs`, + others | PARTIALLY FIXED (p2p.rs only) |
| M3 | `Hash512::from_bytes` panics on wrong-length input | `core/src/primitives/hash.rs` | NOT FIXED |
| M4 | `Address::from_slice` panics on wrong-length input | `core/src/primitives/address.rs` | NOT FIXED |
| M5 | Unmaintained `bincode` crate (RUSTSEC-2025-0141) | `core/Cargo.toml` | NOT FIXED |

---

## Approach

Each finding is treated as a self-contained, low-risk remediation:

1. **M1 — Cross-shard receipt consumption.** In `request_to_receipt()`, remove the request from `pending_requests` once a receipt is created (or track consumed request IDs in a new `consumed_requests` map). This guarantees one request → one receipt → at most one execution on the destination shard. Add a regression test asserting a second `request_to_receipt()` call for the same request ID fails.

2. **M2 — Error message leakage.** Replace externally-visible error strings that embed internal values (Hash512 via `{:?}`, IO errors via `{e}`, message lengths) with generic messages that carry no reconstructable detail. Audit the files the audit names plus a broader sweep of `chain/`, `network/`, `vm/`, and `privacy/` for remaining leakage in externally reachable operations.

3. **M3/M4 — Panic-on-bad-input.** Change `Hash512::from_bytes` and `Address::from_slice` to return `Result<Self, String>` (validating length before `copy_from_slice`) instead of panicking. Update all callers (tests in `primitives/block.rs`, `primitives/address.rs`, and `Address::from_public_key` in `address.rs`) to handle the `Result`. Add regression tests asserting invalid-length input returns an error rather than panicking.

4. **M5 — bincode migration.** `bincode = "2"` is declared in `core/Cargo.toml` but **unused** in any `.rs` file across the repository (zero references found). Remediation is to drop the dependency from `Cargo.toml` and regenerate `Cargo.lock`, eliminating the RUSTSEC-2025-0141 warning from `cargo audit`. No code migration is needed because nothing imports `bincode`.

Verification: `cargo build`, `cargo test`, and `cargo audit` must all pass with the bincode warning gone. Existing tests that assert on specific error strings or on `pending_request_count` after receipt creation will be updated as part of the affected tasks.

---

## Task Breakdown (max 10)

| # | Task | Finding | File(s) | Depends on | Est. |
|---|------|---------|---------|------------|------|
| 1 | M1: Consume/remove request from `pending_requests` in `request_to_receipt()`; add consumed-request tracking or map removal; add double-conversion regression test | M1 | `core/src/chain/sharding/cross_shard.rs` | — | 0.5 d |
| 2 | M2a: Genericize error messages in `cross_shard.rs` (lines 90, 125, 129, 136) — stop leaking Hash512 via `{:?}` | M2 | `core/src/chain/sharding/cross_shard.rs` | — | 0.25 d |
| 3 | M2b: Genericize error messages in `network/p2p.rs` — replace remaining `{e}` IO-detail leaks (lines 72, 79, 82, 151, 331, handshake paths) and message-length leak | M2 | `core/src/network/p2p.rs` | — | 0.5 d |
| 4 | M2c: Broader error-leakage audit — scan `chain/sharding/shard_manager.rs`, `rebalancer.rs`, `chain/dag.rs`, `network/bootstrap.rs`, `network/node_identity.rs`, `privacy/zk_starks.rs` for externally reachable operations leaking internal details; fix as needed | M2 | Multiple | 2, 3 | 0.75 d |
| 5 | M3: Change `Hash512::from_bytes` to return `Result<Self, String>`; validate length before `copy_from_slice` | M3 | `core/src/primitives/hash.rs` | — | 0.5 d |
| 6 | M4: Change `Address::from_slice` to return `Result<Self, String>`; validate length before `copy_from_slice` | M4 | `core/src/primitives/address.rs` | — | 0.5 d |
| 7 | M5: Remove unused `bincode = "2"` from `core/Cargo.toml`; regenerate `Cargo.lock`; confirm `cargo audit` bincode warning is gone | M5 | `core/Cargo.toml`, `Cargo.lock` | — | 0.25 d |
| 8 | Update all callers of changed `from_bytes`/`from_slice` signatures (tests in `primitives/block.rs`, `primitives/address.rs`, `Address::from_public_key`); add regression tests for invalid-length input returning errors | M3, M4 | Multiple | 5, 6 | 0.5 d |
| 9 | Full verification: `cargo build`, `cargo test`, `cargo audit`; confirm no regressions and bincode warning eliminated | All | — | 1–8 | 0.5 d |
| 10 | Update `audits/SECURITY_AUDIT_PHASE6.md` status: mark M1–M5 FIXED; note remediation summary | All | `audits/SECURITY_AUDIT_PHASE6.md` | 9 | 0.25 d |

**Total estimated effort:** ~4–5 business days (contained, well-understood fixes; no new dependencies or architectural changes).

---

## Dependencies

- Tasks 5 (M3) and 6 (M4) are independent of each other and of M1/M2/M5.
- Task 4 (M2c broader audit) depends on 2 and 3 being done first so the pattern of genericization is established before auditing remaining files.
- Task 8 depends on 5 and 6 (signature changes require caller updates).
- Task 9 depends on 1–8 (full verification after all fixes).
- Task 10 depends on 9 (status update after verification).

No cross-cutting architectural changes, no new crates, no database/schema changes.

---

## Risks

| Risk | Description | Mitigation |
|------|-------------|------------|
| **Breaking pub API change (M3/M4)** | `Hash512::from_bytes` and `Address::from_slice` are `pub`. Changing to `Result` is a breaking signature change; any missed caller fails compilation. | Grep for all callers (done: `block.rs` tests, `address.rs` `from_public_key`, tests in `address.rs`). Update all callers in task 8; `cargo build` will catch any miss. |
| **Test assertions on error strings (M2)** | Existing tests may assert on specific error message text (e.g., `contains("fingerprint mismatch")` in `p2p.rs`). Genericizing messages can break them. | Review each affected test when genericizing; update assertions to check error category rather than exact text where needed. |
| **Test assertion on `pending_request_count` (M1)** | `create_request` test asserts `pending_request_count() == 1` after creation. If a test converts that request to a receipt and then checks the count, the count semantics change. | Update `request_to_receipt` test to assert the request is consumed (count drops to 0 or a consumed set grows). Add explicit double-conversion test. |
| **bincode removal breaks hidden dependents (M5)** | `bincode` is declared but unused in `.rs` files; however `Cargo.lock` lists it. Removing it could theoretically affect a transitive build script or proc-macro not caught by source grep. | `cargo build` and `cargo test` after removal will confirm no breakage; `Cargo.lock` regenerated. |
| **Over-scoped M2 audit** | "etc." in the audit is vague; the broader sweep could expand beyond reasonable effort. | Scope M2c to the six named files plus `chain/`, `network/`, `vm/` top-level error returns; time-box to 0.75 day. Defer anything not externally reachable. |

---

## Timeline

| Phase | Tasks | Target |
|-------|-------|--------|
| Receipt consumption (M1) | Task 1 | Day 1 |
| Error-leakage fixes (M2a, M2b) | Tasks 2, 3 | Day 1–2 |
| Broader error-leakage audit (M2c) | Task 4 | Day 2 |
| Panic-on-input fixes (M3, M4) | Tasks 5, 6 | Day 2–3 |
| bincode removal (M5) | Task 7 | Day 3 |
| Caller updates + regression tests | Task 8 | Day 3 |
| Full verification | Task 9 | Day 4 |
| Audit status update | Task 10 | Day 4–5 |

**Target completion:** Within 5 business days of plan approval. All five Medium findings are independent, contained fixes with no external dependencies or coordination required.

---

## Deliverables (upon implementation)

- Modified source files for M1–M5 (no new files except possibly test additions).
- Updated `core/Cargo.toml` and regenerated `Cargo.lock` (bincode removed).
- Regression tests: double-consumption of cross-shard requests; invalid-length `from_bytes`/`from_slice` return errors.
- Updated `audits/SECURITY_AUDIT_PHASE6.md` marking M1–M5 as FIXED.
- Verification output: `cargo build` PASS, `cargo test` PASS, `cargo audit` with no bincode warning.

**Next step after approval:** Run `/build-feature` or implement tasks 1–10 directly on a feature branch off `main`, then `/review-code`.
