# Atoll Protocol - Smart Contract VM Production Hardening Plan

**Owner:** Atlas (Planner)  
**Project:** Atoll Protocol (`D:\Atoll`)  
**Branch:** `main`  
**Status:** Draft — awaiting approval  
**Feature:** Smart Contract VM Production Hardening (EVM, WASM, Move, RISC-V, AI VM, ZK VM)

---

## 1. Goal

Hardening all six Smart Contract VMs (EVM, WASM, Move, RISC-V, AI VM, ZK VM) from MVP-level simulated execution to production-ready implementations suitable for mainnet. Each VM must be secure, deterministic, accurately gas-metered, performant, fuzz-tested, and panic-free.

---

## 2. Current State Assessment

All six VMs live under `core/src/vm/` with a shared router (`mod.rs`) and gas meter (`GasMeter`). Each VM executor is a "Phase 1" stub:

| VM | File | Current State |
|----|------|---------------|
| EVM | `core/src/vm/evm/mod.rs` | ~20 opcodes interpreted; unknown opcodes silently skipped; no stack/memory; no precompiles |
| WASM | `core/src/vm/wasm/mod.rs` | Validates `\0asm` magic + version 1 only; simulated execution (fixed return string); no runtime |
| Move | `core/src/vm/move_vm/mod.rs` | Validates non-empty bytecode only; simulated execution; no bytecode parser |
| RISC-V | `core/src/vm/riscv/mod.rs` | Validates ELF magic only; simulated execution; no instruction decoder |
| AI VM | `core/src/vm/ai_vm/mod.rs` | Basic ONNX size/version check; simulated inference; simulated proof verification |
| ZK VM | `core/src/vm/zk_vm/mod.rs` | Validates proof/circuit_id non-empty; simulated verification (always true) |

**Shared infrastructure gaps:**
- `VMRouter::route()` uses only the first byte — no magic-byte validation beyond prefix
- `GasMeter` charges per-opcode/module but has no shared cost table, no refunds, no cross-VM calibration
- `ExecutionContext` fields (block_number, block_timestamp, difficulty) are not validated
- No sandbox/escap prevention in any VM
- No determinism guarantees — "simulated" execution returns fixed data, not real compute
- No VM-specific fuzzing beyond basic adversarial cases in `core/tests/adversarial.rs`
- No benchmarking or profiling for any VM
- Error handling returns `String` errors; no panic boundaries

**Existing tests:** `core/tests/fuzzing.rs`, `core/tests/stress.rs`, `core/tests/integration.rs`, `core/tests/adversarial.rs` — VM coverage is limited to happy-path and basic error cases.

---

## 3. Approach

Hardening proceeds in two tracks:

**Track A — Shared foundation (Tasks 1-2):** VM router validation, unified gas metering, panic-safe execution wrapper, ExecutionContext validation. These are prerequisites for all per-VM work.

**Track B — Per-VM hardening (Tasks 3-8):** Each VM gets sandboxing, input validation, bounds checking, real execution (where applicable), deterministic output, accurate gas metering, and error handling without panics.

**Track C — Validation (Tasks 9-10):** Cross-VM fuzzing, determinism testing, benchmarking, profiling, and integration testing.

Design principles (Ponytail/YAGNI):
- Reuse existing `GasMeter`, `ExecutionContext`, `ExecutionResult` types — extend, don't replace
- Reuse `proptest` and `quickcheck` (already dev-dependencies) for fuzzing
- Reuse `criterion` (already dev-dependency) for benchmarking
- `winterfell` (zk-STARKs) is already an optional dependency — gate ZK VM behind `production-zk` feature
- No new dependencies beyond what's already in `Cargo.toml` unless justified
- One task per VM to keep diffs reviewable

---

## 4. Task Breakdown (max 10)

| # | Task | Description | Depends On | Est. Size |
|---|------|-------------|------------|-----------|
| 1 | **VM Router & ExecutionContext Hardening** | Validate bytecode beyond first byte (magic bytes), reject ambiguous prefixes, validate ExecutionContext fields (chain_id, gas limits), add panic-safe `execute_safe()` wrapper that catches panics and returns `ExecutionResult::Error` | — | S (2 days) |
| 2 | **Unified Gas Metering Standard** | Shared gas cost table across all 6 VMs, per-VM calibration factors, gas refund mechanism, `charge_with_refund()`, cross-VM consistency tests | Task 1 | S (2 days) |
| 3 | **EVM Production Hardening** | Stack + memory model with bounds checking, complete opcode set (no silent skips), precompile interface, sandbox memory limits, determinism via fixed-point arithmetic | Task 1, Task 2 | M (3 days) |
| 4 | **WASM Production Hardening** | Integrate wasmtime/wasmer runtime (or minimal interpreter), linear memory sandbox, import/function validation, execution time limits, deterministic floating-point handling | Task 1, Task 2 | M (3 days) |
| 5 | **Move VM Production Hardening** | Move bytecode parser, resource-oriented type validation, module safety checks, borrow checker simulation, deterministic storage access | Task 1, Task 2 | M (3 days) |
| 6 | **RISC-V VM Production Hardening** | RV64IMC instruction decoder, memory sandboxing with page permissions, privileged mode restrictions, syscall whitelist, deterministic execution | Task 1, Task 2 | M (3 days) |
| 7 | **AI VM Production Hardening** | Deterministic ONNX inference (fixed-point or quantized), model hash commitment, input sanitization, proof generation/verification interface, inference cost model | Task 1, Task 2 | M (3 days) |
| 8 | **ZK VM Production Hardening** | Integrate `winterfell` zk-STARKs (behind `production-zk` feature), circuit compilation/validation, proof verification, trusted setup ceremony interface, recursive proof support | Task 1, Task 2 | L (5 days) |
| 9 | **Cross-VM Fuzzing & Determinism Testing** | Property-based fuzzing for each VM (`proptest`/`quickcheck`), cross-node determinism tests (same input → same output across VM instances), adversarial input suites, panic-recovery tests | Tasks 3-8 | M (3 days) |
| 10 | **VM Performance Benchmarking & Profiling** | Criterion benchmarks for each VM, gas cost calibration against real execution, profiling for hot paths, optimization targets (gas per op, execution time), regression thresholds | Tasks 3-8 | M (3 days) |

**Total estimated effort: ~29 business days (~6 weeks)**

---

## 5. Dependencies

### Internal dependencies
- `core/src/vm/mod.rs` — `GasMeter`, `ExecutionContext`, `ExecutionResult`, `VMRouter`, `VMType` (extended, not replaced)
- `core/src/primitives/hash.rs` — `Hash512` (used for state roots, model hashes, circuit IDs)
- `core/src/primitives/address.rs` — `Address` (used for caller/origin in context)
- `core/Cargo.toml` — existing dev-deps: `proptest`, `quickcheck`, `criterion`; optional `winterfell` for ZK VM

### External dependencies to add
| Crate | Purpose | Task |
|-------|---------|------|
| `wasmtime` | WASM runtime with sandboxing | Task 4 |
| `move-bytes` / `move-ir` | Move bytecode parsing (if available) or minimal parser | Task 5 |

> If `move-bytes` is not suitable, implement a minimal Move bytecode parser inline (no new dependency).

### No new dependencies for: EVM (interpreter already inline), RISC-V (decoder inline), AI VM (deterministic math inline), ZK VM (uses existing `winterfell`), fuzzing (uses existing `proptest`/`quickcheck`), benchmarking (uses existing `criterion`).

---

## 6. Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Determinism**: floating-point operations (WASM f32/f64, AI VM inference) produce different results across platforms/architectures | High | Use fixed-point arithmetic or deterministic soft-float; document non-deterministic ops as unsupported |
| **Sandbox escape**: malicious bytecode accessing host memory or executing host syscalls | Critical | WASM linear memory sandbox; RISC-V page-permission model; EVM memory bounds; syscall whitelist |
| **Gas undercharging**: VM operations cost less gas than actual compute, enabling DoS | High | Calibrate gas costs against benchmarked execution time; add safety margin (20%) |
| **ZK VM feature gate**: `winterfell` is optional (`production-zk`) — ZK VM hardening may not compile without it | Medium | Gate all ZK VM production code behind `#[cfg(feature = "production-zk")]`; provide stub for default build |
| **Scope creep**: building full EVM/WASM/Move/RISC-V spec compliance | High | Each VM task targets "production-hardened MVP" — safe execution, not full spec compatibility. Full opcode/instruction set can be post-MVP. |
| **Test maintenance**: fuzzing suites need ongoing upkeep as VMs evolve | Medium | Keep fuzzers focused on invariants (no panic, gas never exceeds limit, deterministic output) rather than behavioral correctness |

---

## 7. Timeline

| Week | Focus |
|------|-------|
| Week 1 | Tasks 1-2: VM router hardening, unified gas metering |
| Week 2 | Tasks 3-4: EVM hardening, WASM hardening |
| Week 3 | Tasks 5-6: Move VM hardening, RISC-V hardening |
| Week 4 | Tasks 7-8: AI VM hardening, ZK VM hardening |
| Week 5 | Tasks 9-10: Fuzzing/determinism testing, benchmarking/profiling |
| Week 6 | Integration, bug fixes, documentation, review |

**Target:** Production-hardened VMs ready for mainnet by end of Week 6.

---

## 8. Definition of Done (exit criteria)

- [ ] `cargo build --release --locked` passes (with and without `production-zk` feature)
- [ ] `cargo test` passes — all existing VM tests + new hardening tests
- [ ] `cargo clippy --all-targets -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] No VM panics on any adversarial input (fuzzing suites pass)
- [ ] Same bytecode → same `state_root` across 100 VM instances (determinism)
- [ ] Gas used never exceeds `gas_limit` for any input (gas safety)
- [ ] Each VM has criterion benchmarks with documented gas-per-op costs
- [ ] `ExecutionContext` fields validated; invalid contexts rejected before execution
- [ ] ZK VM compiles and passes tests with `cargo test --features production-zk`
- [ ] README.md updated to reflect VM hardening status

---

## 9. Next Step

Review and approve this plan, then run `/build-feature <issue-number>` for full-stack implementation, followed by `/review-code`. Do not write implementation code until the plan is approved.

---

## 10. Per-VM Security Requirements (reference for implementation)

### EVM
- Stack depth limit (1024), memory expansion bounds, PC bounds checking
- No unknown opcode skipping — return error on undefined opcodes
- Precompiles (ecRecover, SHA256, RIPEMD160, identity, modexp) with fixed gas costs
- Deterministic: no floating-point, fixed-point arithmetic only

### WASM
- Linear memory with max size (e.g., 32 pages), import validation (only allowed host functions)
- No arbitrary syscalls, no FFI escape
- Deterministic floating-point: use soft-float or document non-deterministic ops

### Move VM
- Resource type validation (no duplication, no dropping resources)
- Module bytecode validation (magic, version, type signatures)
- Borrow checker: no dangling references, no double-borrow mutable

### RISC-V
- RV64IMC instruction set only, no privileged instructions (CSR access restricted)
- Memory sandbox: code segment read-only, data segment no-execute
- Syscall whitelist: only `read`, `write`, `exit` allowed

### AI VM
- ONNX model size limit (already 100 MB), input tensor bounds
- Deterministic inference: quantized or fixed-point, no random initialization
- Model hash commitment: deployed model hash stored, inference requires matching hash

### ZK VM
- Circuit ID must match proof commitment
- Proof verification before state transition
- `winterfell`-based zk-STARKs behind `production-zk` feature
- No unverified proof execution
