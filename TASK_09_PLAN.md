# Task 9 Plan — Cross-VM Fuzzing & Determinism Testing

**Owner:** Atlas (Planner)  
**Project:** Atoll Protocol (`D:\Atoll`)  
**Branch:** `feature/vm-hardening-fuzzing`  
**Status:** Draft — awaiting approval  
**Depends on:** Tasks 3-8 (per-VM hardening) complete  
**Estimated size:** M (3 business days)

---

## 1. Goal

Verify that all six VMs (EVM, WASM, Move, RISC-V, AI VM, ZK VM) produce deterministic output for identical input, remain panic-free under adversarial/random input, handle errors gracefully, and respect gas limits. Also verify cross-VM consistency where a logical operation can be expressed in multiple VMs.

This is Track C (Validation) in the VIBE_PLAN.md hardening plan.

---

## 2. Current State (reference)

### VM `execute()` signatures (from `core/src/vm/`)

| VM | Method | Signature |
|----|--------|-----------|
| EVM | `execute` | `(&mut self, bytecode: &[u8], input: &[u8]) -> Result<ExecutionResult, VMError>` |
| WASM | `execute` | `(&mut self, wasm_bytes: &[u8], function: &str, args: &[Vec<u8>]) -> Result<ExecutionResult, VMError>` |
| Move | `execute` | `(&mut self, bytecode: &[u8], function: &str, args: &[Vec<u8>]) -> Result<ExecutionResult, VMError>` |
| RISC-V | `execute` | `(&mut self, binary: &[u8], entry_point: u64, args: &[Vec<u8>]) -> Result<ExecutionResult, VMError>` |
| AI VM | `deploy_model` | `(&mut self, model_bytes: &[u8]) -> Result<ExecutionResult, VMError>` |
| AI VM | `execute_inference` | `(&mut self, model_hash: &Hash512, input_data: &[u8]) -> Result<ExecutionResult, VMError>` |
| ZK VM | `deploy_circuit` | `(&mut self, circuit_bytes: &[u8]) -> Result<ExecutionResult, VMError>` |
| ZK VM | `execute` | `(&mut self, circuit_id: &Hash512, public_inputs: &[Vec<u8>], proof: &[u8]) -> Result<ExecutionResult, VMError>` |

### Shared types
- `ExecutionResult::{Success { gas_used, return_value, logs }, Revert { gas_used, reason }, Error { gas_used, message }}`
- `VMError::{OutOfGas, InvalidOpcode, StackOverflow, StackUnderflow, InvalidJump, InvalidMemoryAccess, InvalidInstruction, UnsupportedOperation, Panic}`
- `ExecutionContext` — caller, origin, gas_price, gas_limit, block_number, block_timestamp, chain_id, difficulty, is_static
- `VMRouter::route(data: &[u8]) -> Result<VMType, String>` and `VMRouter::execute(context, bytecode) -> ExecutionResult`
- `execute_safe(f) -> ExecutionResult` — catches panics, returns `ExecutionResult::Error`
- `GasMeter` — `charge(gas) -> Result<(), VMError>`, `remaining()`, `gas_used`

### Existing test patterns
- Test files live in `core/tests/`: `fuzzing.rs`, `stress.rs`, `adversarial.rs`, `integration.rs`
- Tests reference the crate as `atoll_core::vm::...` (external crate path, not `crate::`)
- Simple for-loop fuzzing is used today; `proptest` and `quickcheck` are dev-dependencies but not yet used
- VM adversarial tests exist in `adversarial.rs` covering happy-path errors per VM

### What Task 9 does NOT do
- Does not write VM implementation code (that's Tasks 3-8)
- Does not change `GasMeter`, `ExecutionContext`, or `VMRouter`
- Does not add new dependencies (uses existing `proptest`, `quickcheck`, `rand`)

---

## 3. Approach

### 3.1 Determinism testing (per VM)

For each VM, run the same input through two fresh executor instances and assert:
- `return_value` bytes are identical
- `gas_used` is identical
- `ExecutionResult` variant is identical (Success/Revert/Error)
- Repeat across N different inputs (property-based with `proptest`)

Also run the same input 100 times through a single executor (re-created each iteration) and verify the 100th output equals the 1st.

### 3.2 Cross-VM fuzzing (per VM)

Generate random bytecode/input for each VM type using `proptest` strategies. For each generated input:
- Create a fresh executor
- Call `execute()` (or `deploy_model`/`deploy_circuit` for AI/ZK)
- Assert the result does **not** panic — use `execute_safe` wrapper or `std::panic::catch_unwind` around the call
- Assert `gas_used <= gas_limit` (gas never exceeds the budget)
- Assert errors return `Err(VMError)` or `ExecutionResult::Error`, never a panic

### 3.3 Determinism across VMs (where applicable)

For simple, expressible logical operations (e.g., add two small integers), encode the same operation in EVM bytecode, WASM module, Move bytecode, and RISC-V binary. Execute each and verify the observable result (return value or final register/stack state) is equivalent. This is a best-effort check — not all VMs can express the same operations.

### 3.4 End-to-end router fuzzing

Feed random bytecode prefixes (0x00–0x05 + random payload) through `VMRouter::execute()`. Verify:
- No panics
- Routing returns a valid `VMType` or an error (never a crash)
- Execution result is well-formed

### 3.5 Property invariants (checked in every fuzz test)

1. **No panic**: `catch_unwind` never triggers
2. **Gas safety**: `gas_used <= gas_limit` always
3. **Determinism**: same input → same `return_value` and `gas_used`
4. **Error grace**: malformed input returns `Err` or `ExecutionResult::Error`, never unwinds

---

## 4. Task Breakdown (max 8)

| # | Task | Description | Depends On | Est. Size |
|---|------|-------------|------------|-----------|
| 9.1 | **Shared test harness & determinism helpers** | Create `core/tests/vm_fuzzing.rs` with shared helpers: `assert_deterministic()`, `assert_no_panic()`, `assert_gas_safe()`, `fuzz_executor()` wrapper using `catch_unwind`/`execute_safe`, proptest strategies for random `Vec<u8>` generation | Tasks 3-8 | S (1 day) |
| 9.2 | **EVM fuzzing & determinism tests** | proptest strategies for random EVM bytecode (valid opcodes + random bytes), determinism tests (same bytecode → same output/gas), panic-recovery tests on malformed bytecode, out-of-gas verification | 9.1 | S (0.5 day) |
| 9.3 | **WASM fuzzing & determinism tests** | Random WASM byte fuzzing (valid header + random sections), determinism tests, import rejection verification, fuel/out-of-gas tests | 9.1 | S (0.5 day) |
| 9.4 | **Move VM fuzzing & determinism tests** | Random Move bytecode fuzzing (magic + version + random counts/body), determinism tests, invalid magic/version rejection | 9.1 | S (0.5 day) |
| 9.5 | **RISC-V fuzzing & determinism tests** | Random ELF64 binary fuzzing (valid header + random bytes), determinism tests, invalid magic/class/machine rejection, entry-point bounds | 9.1 | S (0.5 day) |
| 9.6 | **AI VM & ZK VM fuzzing & determinism tests** | Random model/circuit byte fuzzing, deploy+execute determinism, size-limit enforcement, duplicate deployment rejection, proof verification determinism | 9.1 | S (0.5 day) |
| 9.7 | **Cross-VM consistency & router fuzzing** | Same logical op (integer add) across EVM/WASM/Move/RISC-V, `VMRouter::execute()` fuzzing with random prefixes, cross-VM gas consistency | 9.2-9.6 | S (0.5 day) |
| 9.8 | **Test suite integration & verification** | Add `vm_fuzzing.rs` to test module, run `cargo test` for all VM tests, verify `cargo clippy` and `cargo fmt` pass, document fuzzing coverage in VIBE_PLAN.md | 9.1-9.7 | S (0.5 day) |

**Total: ~3 business days (M)**

---

## 5. Dependencies

### Internal
- **Tasks 3-8** must be complete — all VMs must have production-hardened `execute()` implementations with `GasMeter`, `ExecutionContext`, and `VMError` integration
- `execute_safe()` wrapper (Task 1) — used for panic boundary testing
- `VMRouter::route()` and `VMRouter::execute()` — used for end-to-end router fuzzing
- `Hash512::blake3()` — used for AI VM model hashing and ZK VM circuit hashing in tests

### External (already in `core/Cargo.toml` dev-dependencies)
- `proptest = "1.0"` — property-based test strategies
- `quickcheck = "1.0"` — alternative property-based testing (available if needed)
- `rand = "0.8"` — random byte generation (already a main dependency)
- `wasmtime = "15"` — WASM runtime (already a dependency)

### No new dependencies required

---

## 6. Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Test execution time** — fuzzing many iterations across 6 VMs can be slow, especially WASM compilation via wasmtime | Medium | Limit proptest case counts (e.g., 100-500 cases per test), use small gas limits, run fuzzing tests in `cargo test --test vm_fuzzing` separately from unit tests |
| **ZK VM behind `production-zk` feature** — ZK VM tests may not compile without `winterfell` | Medium | Gate ZK VM fuzz tests behind `#[cfg(feature = "production-zk")]`; provide a non-feature test stub that verifies the stub compiles |
| **Cross-VM equivalence is limited** — different VMs have different execution models; "same logical op" may not be expressible identically | Low | Limit cross-VM tests to simple integer arithmetic; document which VM pairs are comparable |
| **proptest strategies generating inputs that exhaust gas** — random bytecode may trigger out-of-gas, which is expected behavior | Low | Treat `OutOfGas` as a valid (non-panic) outcome; assert gas_used <= gas_limit, not success |
| **Panic in `catch_unwind` with non-unwind-safe types** — some VM state may not implement `AssertUnwindSafe` safely | Low | Wrap executor creation + execute in `execute_safe()` which already uses `AssertUnwindSafe`; test through `VMRouter::execute()` where possible |

---

## 7. Timeline

| Day | Focus |
|-----|-------|
| Day 1 | Task 9.1: Shared test harness + determinism helpers; Task 9.2: EVM fuzzing & determinism |
| Day 2 | Task 9.3: WASM fuzzing & determinism; Task 9.4: Move VM fuzzing & determinism; Task 9.5: RISC-V fuzzing & determinism |
| Day 3 | Task 9.6: AI VM & ZK VM fuzzing & determinism; Task 9.7: Cross-VM consistency & router fuzzing; Task 9.8: Integration, clippy, fmt, full test run |

**Target:** All Task 9 tests pass, `cargo test --test vm_fuzzing` green, `cargo clippy --all-targets -D warnings` clean, `cargo fmt --check` clean.

---

## 8. Definition of Done

- [ ] `core/tests/vm_fuzzing.rs` exists with per-VM fuzzing and determinism tests
- [ ] Each VM has determinism tests: same input → same `return_value` and `gas_used` across fresh executor instances
- [ ] Each VM has fuzzing tests: random input never panics, gas never exceeds limit, errors returned gracefully
- [ ] Cross-VM consistency test: integer add produces equivalent results across EVM/WASM/Move/RISC-V
- [ ] `VMRouter::execute()` fuzzing: random prefixes routed without panic
- [ ] `cargo test --test vm_fuzzing` passes
- [ ] `cargo clippy --all-targets -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] ZK VM tests compile both with and without `--features production-zk`
- [ ] Plan documented; ready for `/build-feature` implementation

---

## 9. Next Step

Review and approve this plan, then run `/build-feature <issue-number>` for implementation. Do not write implementation code until the plan is approved.
