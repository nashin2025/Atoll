# Winterfell AIR Design for Atoll's Computation Model

**Status:** Draft (MVP)  
**Task:** zk-STARKs production plan, Task 2 — Research & document Winterfell AIR design  
**Crate:** `atoll-core`, module `core/src/privacy/zk_starks.rs`  
**Dependency:** `winterfell = "0.13"` (gated behind `production-zk` feature)

---

## 1. Goal

Define a concrete Algebraic Intermediate Representation (AIR) for Atoll's computation model that can be implemented with Winterfell (`winterfell 0.13`). This document captures the arithmetization decisions so that Tasks 3–5 (implement `StarkProver`/`StarkVerifier` backed by Winterfell, wire them into the existing `StarkProof`/`StarkQuery` API, add tests) have a fixed target to build against.

The design is deliberately **pragmatic for MVP**: we start with the simplest provable computation that is still meaningful to Atoll, then iterate toward richer computations (hash preimage, balance transfer, state transition).

---

## 2. What computation is being proved?

### MVP (this task)

**Correct execution of an iterated state-transition function.**

Given a public starting value `start` and a public number of steps `n`, prove that repeatedly applying a transition function `F` yields a public result `result`:

```
s_0 = start
s_{i+1} = F(s_i)   for i = 0 .. n-1
s_n = result
```

For the MVP instantiation we mirror Winterfell's documented hello-world:

```
F(s) = s^3 + c        (c is a public constant)
```

This is a degree-3 transition, single-column trace, two boundary assertions. It is the smallest non-trivial Winterfell AIR and directly generalizes to:

- **Hash-chain preimage:** `F(s) = H(s)` → prove knowledge of `x` such that `H^n(x) = y`.
- **Balance transfer:** `F` updates an account balance column → prove a state transition executed correctly.

### Why not transaction execution / full state transition for MVP?

Atoll's `Transaction` and `ChainState` (see `core/src/primitives/transaction.rs`, `core/src/chain/state.rs`) involve u128 balances, addresses, signatures, and variable-length data. Arithmetizing those over a prime field requires range checks, signature verification, and memory-like constraints — far beyond an MVP AIR. We defer those to post-MVP iterations and start with the iterated function above.

---

## 3. Arithmetization into Winterfell trace columns

### Base field

Winterfell's `Air::BaseField` must implement `StarkField + ExtensibleField<2> + ExtensibleField<3>`. Winterfell's `f128` field (a 128-bit prime field, used in the crate's own docs example) satisfies this. We select **`f128`** as the MVP base field.

> Decision needed for Task 3: confirm `f128` is available/exposed via `winterfell::math::fields::f128::BaseElement` in v0.13, or select an alternative exposed field.

### Public inputs

The prover and verifier must agree on the computation instance. Public inputs are passed to `Air::new()` via `PublicInputs` and to the verifier separately.

```rust
pub struct IterationPublicInputs {
    pub start: BaseElement,   // s_0, pinned at step 0
    pub result: BaseElement,  // s_n, pinned at last step
    pub n: usize,             // number of steps (encoded in trace length)
    pub c: BaseElement,       // constant added each step
}
```

`PublicInputs` must implement `ToElements<BaseField>` (Winterfell requirement) so it can be serialized into field elements for the proof's public-input commitment.

### Trace columns

| Column | Name | Meaning |
|--------|------|---------|
| 0 | `state` | `s_i` at step `i` |

**Trace width:** 1 column (MVP).  
**Trace length:** power of two, `>= 8`, equal to `n` (number of steps). Configurable; MVP default `4096` (2^12) for fast local verification, production `2^20` (1,048,576).

Each row `i` of the trace holds `s_i`. The trace is built by iterating `F` from `start`.

---

## 4. Trace length and number of columns

| Parameter | MVP value | Notes |
|-----------|-----------|-------|
| Trace width (columns) | 1 | Single `state` column |
| Trace length (rows) | 2^12 = 4096 (default) | Must be power of two, >= 8. Equals `n` steps. |
| Trace polynomial degree | `trace_length - 1` | Winterfell invariant |
| Transition constraints | 1 | `s_{i+1} - (s_i^3 + c) = 0` |
| Transition constraint degree | 3 | `s_i^3` dominates; `s_i` and `c` are degree 1 |
| Boundary assertions | 2 | `state[0] = start`, `state[last] = result` |
| Periodic columns | 0 | Not needed for MVP |

---

## 5. Transition constraints

Winterfell evaluates transition constraints over an `EvaluationFrame<E>` containing two consecutive rows (`current` = `s_i`, `next` = `s_{i+1}`).

**Constraint (single, degree 3):**

```
result[0] = frame.next()[0] - (frame.current()[0]^3 + c)
```

This evaluates to zero iff `s_{i+1} = s_i^3 + c`. For any invalid transition, the result is non-zero.

`TransitionConstraintDegree::new(3)` is passed to `AirContext::new()`.

> Note on degrees: Winterfell requires the declared degree to match the actual constraint degree in debug builds; a mismatch in release builds produces an invalid proof the verifier will reject. We declare degree 3 to be safe.

---

## 6. Boundary constraints (assertions)

Boundary constraints tie the execution trace to the public inputs. We use two **single assertions**:

| Assertion | Column | Step | Value | Meaning |
|-----------|--------|------|-------|---------|
| `Assertion::single(0, 0, start)` | 0 | 0 | `start` | Initial state pinned |
| `Assertion::single(0, last_step, result)` | 0 | `n - 1` | `result` | Final state pinned |

`last_step = trace_length - 1`.

These are returned from `Air::get_assertions()`. `num_assertions = 2` is passed to `AirContext::new()`.

---

## 7. Proof options (security parameters)

Modeled on Winterfell's docs example (~96-bit conjectured security):

| Parameter | Value |
|-----------|-------|
| Number of queries | 32 |
| Blowup factor | 8 |
| Grinding factor | 0 |
| Field extension | `FieldExtension::None` |
| FRI folding factor | 8 |
| FRI max remainder degree | 31 |
| Constraint batching | `BatchingMethod::Linear` |
| DEEP batching | `BatchingMethod::Linear` |
| Min conjectured security (verifier) | 95 bits |

These live in `ProofOptions` and are stored in the `AirContext`. The `StarkProver`/`StarkVerifier` wrapper (Task 3) should expose configurable options with these as defaults.

---

## 8. Winterfell types to implement (Task 3 reference)

The `Air` trait (winter-air 0.13.1) requires:

| Associated type / method | MVP choice |
|--------------------------|------------|
| `type BaseField` | `BaseElement` (f128) |
| `type PublicInputs` | `IterationPublicInputs` (impl `ToElements`) |
| `fn new(trace_info, pub_inputs, options)` | Build `AirContext` with 1 constraint (degree 3), 2 assertions |
| `fn context(&self)` | `&AirContext<BaseElement>` |
| `fn evaluate_transition<E, ...>(&self, frame, periodic_values, result)` | `result[0] = next - (current^3 + c)` |
| `fn get_assertions(&self)` | `vec![single(0,0,start), single(0,last,result)]` |

The `Prover` trait (winter-prover 0.13.1) requires many associated types (`HashFn`, `VC`, `RandomCoin`, `TraceLde`, `ConstraintCommitment`, `ConstraintEvaluator`, …). For MVP we use Winterfell's defaults:

- `HashFn = Blake3_256<BaseElement>` — matches Atoll's existing BLAKE3 usage (`Hash512::blake3`)
- `VC = MerkleTree<Blake3_256<BaseElement>>`
- `RandomCoin = DefaultRandomCoin<Blake3_256<BaseElement>>`
- `Trace = TraceTable<BaseElement>`
- `TraceLde = DefaultTraceLde<E, HashFn, VC>`
- `ConstraintCommitment = DefaultConstraintCommitment<E, HashFn, VC>`
- `ConstraintEvaluator = DefaultConstraintEvaluator<'a, Air, E>`

`get_pub_inputs(trace)` reads `trace.get(0, 0)` and `trace.get(0, last_step)`.

---

## 9. How this maps to the existing Atoll API

The current `zk_starks.rs` (Phase 1, simulated) exposes:

```rust
pub struct StarkProver { security_parameter, field_size }
pub struct StarkVerifier
pub fn prove(&self, computation_hash: Hash512, inputs: &[Vec<u8>], outputs: &[Vec<u8>]) -> Result<StarkProof, String>
pub fn verify(&self, proof, computation_hash, public_inputs) -> Result<bool, String>
pub struct StarkProof { commitment, queries, proof_of_work }
pub struct StarkQuery { position, value, auth_path }
```

**Mapping decision (Task 3):** Winterfell produces a `winterfell::Proof` (a rich, self-describing proof with commitments, queries, and FRI layers), not the minimal `StarkProof` above. Two options:

- **Option A (recommended MVP):** Keep the existing `StarkProof`/`StarkQuery` facade as a thin wrapper. Serialize the Winterfell `Proof` into `commitment` (root commitment hash) and `queries` (sampled query responses + auth paths). `StarkProof.proof_of_work` holds the serialized Winterfell proof bytes or a hash thereof. This preserves the public API while swapping the backend.
- **Option B:** Replace `StarkProof` with Winterfell's `Proof` directly. Breaking change to the privacy API; only justified if callers need full Winterfell interoperability.

We recommend **Option A** for MVP to avoid breaking the existing API surface and tests.

`field_size` on `StarkProver` maps to the base field size (128 bits for f128). `security_parameter` maps roughly to the number of queries / conjectured security target.

---

## 10. API decisions needed for Tasks 3–5

The following decisions must be made before implementing the Winterfell-backed prover/verifier:

1. **Base field:** Confirm `f128` (`winterfell::math::fields::f128::BaseElement`) is usable as `Air::BaseField` in v0.13 with the `std` feature. Select alternative if not exposed.
2. **Proof facade:** Option A (wrap Winterfell `Proof` inside existing `StarkProof`) vs Option B (expose Winterfell `Proof`). Recommend Option A.
3. **Public input encoding:** How `computation_hash: Hash512` and `inputs: &[Vec<u8>]` from the current API map to `IterationPublicInputs` (start, result, c). Need a deterministic encoding from bytes → field elements.
4. **Trace length derivation:** Derive `n` (and thus trace length) from inputs, or require it as an explicit parameter. Recommend explicit `steps` parameter for MVP.
5. **Feature gating:** Winterfell is behind `production-zk`. The Winterfell-backed implementation should be `#[cfg(feature = "production-zk")]`, with the simulated Phase 1 prover as the default (non-feature) fallback so `cargo test` runs without Winterfell.
6. **Hash function:** Use `Blake3_256` to stay consistent with Atoll's `Hash512::blake3`. Confirm `winterfell::crypto::hashers::Blake3_256` exists in v0.13.
7. **Security defaults:** Expose `ProofOptions` with the values in §8 as defaults; allow caller override.

---

## 11. Future extensions (post-MVP)

| Extension | Computation | Trace columns | Constraint degree | Notes |
|-----------|-------------|---------------|-------------------|-------|
| Hash preimage | `s_{i+1} = H(s_i)` | 1+ | high (ARX over field) | Requires arithmetizing BLAKE3 compression; complex |
| Balance transfer | `balance' = balance - amount` | 2+ (balance, amount) | 1-2 | Needs range checks for non-negative balances |
| State transition | Full VM step | many | varies | Post-MVP; likely needs Randomized AIR / memory |

Randomized AIR (auxiliary trace segments) is available in Winterfell for range checks / memory / permutation checks but is out of scope for MVP.

---

## 12. Open questions

1. Is `winterfell::math::fields::f128::BaseElement` the correct field path in v0.13, and does it satisfy `StarkField + ExtensibleField<2> + ExtensibleField<3>`?
2. Does `winterfell::crypto::hashers::Blake3_256` exist in v0.13, or is the hasher named differently?
3. Should trace length be fixed at compile time, or passed as a runtime parameter to `StarkProver::prove`?
4. Option A vs Option B for the `StarkProof` facade — confirm no caller depends on the exact `StarkProof` shape.

---

## 13. Summary for Tasks 3–5

- **Computation proved:** Iterated function `s_{i+1} = s_i^3 + c` for `n` steps, with `s_0 = start` and `s_n = result` public.
- **Arithmetization:** 1 trace column (`state`), trace length = `n` (power of two), field `f128`.
- **Transition constraints:** 1 constraint, degree 3: `next - (current^3 + c) = 0`.
- **Boundary constraints:** 2 single assertions pinning `state[0] = start` and `state[last] = result`.
- **Proof options:** 32 queries, blowup 8, Blake3_256 hash, ~96-bit security.
- **Primary API decision:** Wrap Winterfell `Proof` inside existing `StarkProof` (Option A), gated behind `production-zk`, with simulated prover as default fallback.
