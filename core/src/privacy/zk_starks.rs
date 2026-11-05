//! zk-STARKs (Zero-Knowledge Scalable Transparent Arguments of Knowledge)
//! for the Atoll Protocol.
//!
//! Transparent, post-quantum zero-knowledge proofs for private transactions.
//! Phase 1: Interface and simulation.
//! Phase 2: Full STARK prover and verifier implementation.

use crate::primitives::hash::Hash512;
use serde::{Deserialize, Serialize};

/// A zk-STARK proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarkProof {
    /// Proof commitment
    pub commitment: Hash512,
    /// Proof queries and responses
    pub queries: Vec<StarkQuery>,
    /// Proof of work (for soundness)
    pub proof_of_work: Vec<u8>,
}

/// A single query in a STARK proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarkQuery {
    /// Query position
    pub position: u64,
    /// Query value
    pub value: Vec<u8>,
    /// Authentication path
    pub auth_path: Vec<Hash512>,
}

/// STARK prover: generates zero-knowledge proofs
pub struct StarkProver {
    /// Security parameter (number of queries)
    pub security_parameter: u32,
    /// Field size (bits)
    pub field_size: u32,
}

impl StarkProver {
    /// Creates a new STARK prover
    pub fn new(
        security_parameter: u32,
        field_size: u32,
    ) -> Self {
        Self {
            security_parameter,
            field_size,
        }
    }

    /// Generates a STARK proof for a computation
    pub fn prove(
        &self,
        computation_hash: Hash512,
        inputs: &[Vec<u8>],
        outputs: &[Vec<u8>],
    ) -> Result<StarkProof, String> {
        if inputs.is_empty() {
            return Err("No inputs provided".to_string());
        }
        if outputs.is_empty() {
            return Err("No outputs provided".to_string());
        }

        #[cfg(feature = "production-zk")]
        return self.prove_winterfell(computation_hash, inputs, outputs);

        #[cfg(not(feature = "production-zk"))]
        {
            // Phase 1: Simulated proof generation
            // Phase 2: Full STARK prover with FRI low-degree testing

            let mut queries = Vec::new();
            for i in 0..self.security_parameter {
                let idx = (i as usize) % inputs.len();
                queries.push(StarkQuery {
                    position: i as u64,
                    value: inputs[idx].clone(),
                    auth_path: vec![Hash512::zero(); 10],
                });
            }

            let commitment =
                Hash512::blake3(&[computation_hash.as_ref(), &inputs.concat()].concat());

            Ok(StarkProof {
                commitment,
                queries,
                proof_of_work: vec![0u8; 32],
            })
        }
    }

    /// Winterfell-backed STARK proof generation with FRI low-degree testing.
    ///
    /// Available only when the `production-zk` feature is enabled. Proves correct
    /// execution of the iterated state-transition function `F(s) = s^3 + c` over
    /// the f128 base field, as documented in `zk_starks_air_design.md`.
    #[cfg(feature = "production-zk")]
    fn prove_winterfell(
        &self,
        computation_hash: Hash512,
        inputs: &[Vec<u8>],
        _outputs: &[Vec<u8>],
    ) -> Result<StarkProof, String> {
        let _ = self;
        winterfell_backend::prove_iteration(computation_hash, inputs, _outputs)
    }

    /// Returns the security parameter
    pub fn security_parameter(&self) -> u32 {
        self.security_parameter
    }
}

/// STARK verifier: verifies zero-knowledge proofs
pub struct StarkVerifier;

impl StarkVerifier {
    /// Creates a new STARK verifier
    pub fn new() -> Self {
        Self
    }

    /// Verifies a STARK proof
    pub fn verify(
        &self,
        proof: &StarkProof,
        computation_hash: Hash512,
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, String> {
        #[cfg(feature = "production-zk")]
        return winterfell_backend::verify_winterfell(
            &proof.proof_of_work,
            computation_hash,
            public_inputs,
        );

        #[cfg(not(feature = "production-zk"))]
        {
            if proof.queries.is_empty() {
                return Ok(false);
            }

            // Phase 1: Simulated verification
            // Phase 2: Full STARK verifier with FRI low-degree testing

            // Verify commitment matches computation hash
            let expected_commitment =
                Hash512::blake3(&[computation_hash.as_ref(), &public_inputs.concat()].concat());

            if proof.commitment != expected_commitment {
                return Ok(false);
            }

            // Verify each query has a valid auth path
            for query in &proof.queries {
                if query.auth_path.len() < 10 {
                    return Ok(false);
                }
            }

            Ok(true)
        }
    }

    /// Returns the verification key size in bytes
    pub fn verification_key_size(&self) -> usize {
        32
    }
}

/// STARK proof verification result
#[derive(Debug, Clone)]
pub struct StarkVerificationResult {
    pub valid: bool,
    pub computation_hash: Hash512,
    pub queries_checked: u32,
    pub gas_used: u64,
}

impl StarkVerificationResult {
    pub fn new(
        valid: bool,
        computation_hash: Hash512,
        queries_checked: u32,
    ) -> Self {
        Self {
            valid,
            computation_hash,
            queries_checked,
            gas_used: queries_checked as u64 * 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stark_prover_new() {
        let prover = StarkProver::new(40, 256);
        assert_eq!(prover.security_parameter(), 40);
    }

    #[test]
    #[cfg(not(feature = "production-zk"))]
    fn stark_prove_and_verify() {
        // Simulated prover: proof carries one query per security parameter.
        let prover = StarkProver::new(40, 256);
        let verifier = StarkVerifier::new();

        let computation_hash = Hash512::blake3(b"test_computation");
        let inputs = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let outputs = vec![vec![7, 8, 9]];

        let proof = prover.prove(computation_hash, &inputs, &outputs).unwrap();
        assert!(!proof.queries.is_empty());
        assert_eq!(proof.queries.len(), 40);

        let result = verifier.verify(&proof, computation_hash, &inputs);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn stark_invalid_proof() {
        let verifier = StarkVerifier::new();
        let proof = StarkProof {
            commitment: Hash512::zero(),
            queries: vec![],
            proof_of_work: vec![],
        };

        let result = verifier.verify(&proof, Hash512::zero(), &[]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    #[cfg(feature = "production-zk")]
    fn stark_winterfell_verify_roundtrip() {
        use super::winterfell_backend::{prove_iteration, verify_winterfell};

        let computation_hash = Hash512::blake3(b"roundtrip_computation");
        let inputs = vec![vec![1, 2, 3]];
        let outputs = vec![vec![9]];

        // A valid Winterfell proof (with FRI low-degree testing) verifies.
        let proof = prove_iteration(computation_hash, &inputs, &outputs).unwrap();
        let result = verify_winterfell(&proof.proof_of_work, computation_hash, &inputs);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Tampered proof bytes are rejected (FRI low-degree test fails).
        let mut tampered = proof.proof_of_work.clone();
        if tampered.len() > 10 {
            let idx = tampered.len() / 2;
            tampered[idx] ^= 0xFF;
        }
        let result = verify_winterfell(&tampered, computation_hash, &inputs);
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Wrong public inputs are rejected.
        let result = verify_winterfell(&proof.proof_of_work, computation_hash, &[vec![9, 9, 9]]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    #[cfg(feature = "production-zk")]
    fn stark_winterfell_prove_and_verify() {
        // Production prover: proof wraps a real Winterfell proof in proof_of_work.
        let prover = StarkProver::new(40, 256);
        let verifier = StarkVerifier::new();

        let computation_hash = Hash512::blake3(b"production_prove_and_verify");
        let inputs = vec![vec![1, 2, 3]];
        let outputs = vec![vec![9]];

        let proof = prover.prove(computation_hash, &inputs, &outputs).unwrap();
        assert!(!proof.proof_of_work.is_empty());

        let result = verifier.verify(&proof, computation_hash, &inputs);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    #[cfg(feature = "production-zk")]
    fn stark_winterfell_forged_proof_rejected() {
        // Tampered proof bytes must be rejected by the FRI low-degree test.
        let prover = StarkProver::new(40, 256);
        let verifier = StarkVerifier::new();

        let computation_hash = Hash512::blake3(b"production_forged_proof");
        let inputs = vec![vec![1, 2, 3]];
        let outputs = vec![vec![9]];

        let proof = prover.prove(computation_hash, &inputs, &outputs).unwrap();

        let mut forged = proof.clone();
        if !forged.proof_of_work.is_empty() {
            let idx = forged.proof_of_work.len() / 2;
            forged.proof_of_work[idx] ^= 0xFF;
        }

        let result = verifier.verify(&forged, computation_hash, &inputs);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    #[cfg(feature = "production-zk")]
    fn stark_winterfell_wrong_inputs_rejected() {
        // Wrong public inputs must be rejected: the recomputed final state
        // no longer matches the proof's boundary assertion.
        let prover = StarkProver::new(40, 256);
        let verifier = StarkVerifier::new();

        let computation_hash = Hash512::blake3(b"production_wrong_inputs");
        let inputs = vec![vec![1, 2, 3]];
        let outputs = vec![vec![9]];

        let proof = prover.prove(computation_hash, &inputs, &outputs).unwrap();

        let result = verifier.verify(&proof, computation_hash, &[vec![9, 9, 9]]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    #[cfg(not(feature = "production-zk"))]
    fn stark_prover_different_security_params() {
        let prover_low = StarkProver::new(10, 128);
        let prover_high = StarkProver::new(100, 256);

        let computation_hash = Hash512::zero();
        let inputs = vec![vec![1]];

        let proof_low = prover_low
            .prove(computation_hash, &inputs, &vec![vec![2]])
            .unwrap();
        let proof_high = prover_high
            .prove(computation_hash, &inputs, &vec![vec![2]])
            .unwrap();

        assert_eq!(proof_low.queries.len(), 10);
        assert_eq!(proof_high.queries.len(), 100);
    }

    #[test]
    fn stark_verification_result() {
        let result = StarkVerificationResult::new(true, Hash512::zero(), 40);
        assert!(result.valid);
        assert_eq!(result.queries_checked, 40);
        assert_eq!(result.gas_used, 40000);
    }
}

/// Winterfell-backed STARK prover implementation.
///
/// This module is only compiled when the `production-zk` feature is enabled.
/// It implements the Iteration AIR from `zk_starks_air_design.md` using
/// Winterfell's prover with BLAKE3-256 hashing over the f128 base field.
#[cfg(feature = "production-zk")]
mod winterfell_backend {
    use super::{Hash512, StarkProof};
    use winterfell::crypto::{hashers::Blake3_256, DefaultRandomCoin, MerkleTree};
    use winterfell::math::{fields::f128::BaseElement, FieldElement, ToElements};
    use winterfell::matrix::ColMatrix;
    use winterfell::{
        verify, AcceptableOptions, Air, AirContext, Assertion, BatchingMethod, CompositionPoly,
        CompositionPolyTrace, DefaultConstraintCommitment, DefaultConstraintEvaluator,
        DefaultTraceLde, EvaluationFrame, FieldExtension, PartitionOptions, Proof, ProofOptions,
        Prover, StarkDomain, Trace, TraceInfo, TracePolyTable, TraceTable,
        TransitionConstraintDegree,
    };
    use winterfell::{AuxRandElements, ConstraintCompositionCoefficients};

    // Trace length: power of two, >= 8. MVP default 2^12 = 4096. Shared by the
    // prover and verifier so both derive the same public inputs.
    const TRACE_LENGTH: usize = 4096;

    // ---------------------------------------------------------------------------
    // Public inputs for the iterated computation
    // ---------------------------------------------------------------------------

    /// Public inputs for the iterated state-transition computation.
    ///
    /// `start` is the initial state `s_0`, `result` is the final state `s_n`,
    /// `c` is the public constant added each step, and `n` is the number of steps
    /// (encoded in the trace length).
    #[derive(Debug, Clone)]
    pub struct IterationPublicInputs {
        pub start: BaseElement,
        pub result: BaseElement,
        pub c: BaseElement,
        pub n: usize,
    }

    impl ToElements<BaseElement> for IterationPublicInputs {
        fn to_elements(&self) -> Vec<BaseElement> {
            vec![self.start, self.result, self.c]
        }
    }

    // ---------------------------------------------------------------------------
    // AIR: Iterated state-transition F(s) = s^3 + c
    // ---------------------------------------------------------------------------

    /// Algebraic intermediate representation for the iterated computation.
    ///
    /// Proves that `s_{i+1} = s_i^3 + c` for `n` steps, with `s_0 = start` and
    /// `s_n = result` pinned as boundary assertions.
    pub struct IterationAir {
        context: AirContext<BaseElement>,
        start: BaseElement,
        result: BaseElement,
        c: BaseElement,
    }

    impl Air for IterationAir {
        type BaseField = BaseElement;
        type PublicInputs = IterationPublicInputs;

        fn new(
            trace_info: TraceInfo,
            pub_inputs: IterationPublicInputs,
            options: ProofOptions,
        ) -> Self {
            assert_eq!(1, trace_info.width());

            // Single transition constraint of degree 3: next - (current^3 + c) = 0
            let degrees = vec![TransitionConstraintDegree::new(3)];

            // Two boundary assertions: state[0] = start, state[last] = result
            let num_assertions = 2;

            IterationAir {
                context: AirContext::new(trace_info, degrees, num_assertions, options),
                start: pub_inputs.start,
                result: pub_inputs.result,
                c: pub_inputs.c,
            }
        }

        fn context(&self) -> &AirContext<Self::BaseField> {
            &self.context
        }

        fn evaluate_transition<E: FieldElement + From<Self::BaseField>>(
            &self,
            frame: &EvaluationFrame<E>,
            _periodic_values: &[E],
            result: &mut [E],
        ) {
            let current_state = frame.current()[0];
            let c = E::from(self.c);
            let next_state = current_state.exp(3u32.into()) + c;
            result[0] = frame.next()[0] - next_state;
        }

        fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
            let last_step = self.trace_length() - 1;
            vec![
                Assertion::single(0, 0, self.start),
                Assertion::single(0, last_step, self.result),
            ]
        }
    }

    // ---------------------------------------------------------------------------
    // Prover: Winterfell Prover trait implementation
    // ---------------------------------------------------------------------------

    /// Winterfell prover for the Iteration AIR.
    pub struct IterationProver {
        options: ProofOptions,
        c: BaseElement,
    }

    impl IterationProver {
        pub fn new(
            options: ProofOptions,
            c: BaseElement,
        ) -> Self {
            Self { options, c }
        }
    }

    impl Prover for IterationProver {
        type BaseField = BaseElement;
        type Air = IterationAir;
        type Trace = TraceTable<Self::BaseField>;
        type HashFn = Blake3_256<Self::BaseField>;
        type VC = MerkleTree<Self::HashFn>;
        type RandomCoin = DefaultRandomCoin<Self::HashFn>;
        type TraceLde<E: FieldElement<BaseField = Self::BaseField>> =
            DefaultTraceLde<E, Self::HashFn, Self::VC>;
        type ConstraintCommitment<E: FieldElement<BaseField = Self::BaseField>> =
            DefaultConstraintCommitment<E, Self::HashFn, Self::VC>;
        type ConstraintEvaluator<'a, E: FieldElement<BaseField = Self::BaseField>> =
            DefaultConstraintEvaluator<'a, Self::Air, E>;

        fn get_pub_inputs(
            &self,
            trace: &Self::Trace,
        ) -> IterationPublicInputs {
            let last_step = trace.length() - 1;
            IterationPublicInputs {
                start: trace.get(0, 0),
                result: trace.get(0, last_step),
                c: self.c,
                n: trace.length(),
            }
        }

        fn options(&self) -> &ProofOptions {
            &self.options
        }

        fn new_trace_lde<E: FieldElement<BaseField = Self::BaseField>>(
            &self,
            trace_info: &TraceInfo,
            main_trace: &ColMatrix<Self::BaseField>,
            domain: &StarkDomain<Self::BaseField>,
            partition_option: PartitionOptions,
        ) -> (Self::TraceLde<E>, TracePolyTable<E>) {
            DefaultTraceLde::new(trace_info, main_trace, domain, partition_option)
        }

        fn build_constraint_commitment<E: FieldElement<BaseField = Self::BaseField>>(
            &self,
            composition_poly_trace: CompositionPolyTrace<E>,
            num_constraint_composition_columns: usize,
            domain: &StarkDomain<Self::BaseField>,
            partition_options: PartitionOptions,
        ) -> (Self::ConstraintCommitment<E>, CompositionPoly<E>) {
            DefaultConstraintCommitment::new(
                composition_poly_trace,
                num_constraint_composition_columns,
                domain,
                partition_options,
            )
        }

        fn new_evaluator<'a, E: FieldElement<BaseField = Self::BaseField>>(
            &self,
            air: &'a Self::Air,
            aux_rand_elements: Option<AuxRandElements<E>>,
            composition_coefficients: ConstraintCompositionCoefficients<E>,
        ) -> Self::ConstraintEvaluator<'a, E> {
            DefaultConstraintEvaluator::new(air, aux_rand_elements, composition_coefficients)
        }
    }

    // ---------------------------------------------------------------------------
    // Entry point: prove an iteration computation with Winterfell
    // ---------------------------------------------------------------------------

    /// Generates a Winterfell STARK proof for the iterated computation.
    ///
    /// `start` is derived from `computation_hash` (BLAKE3 of the hash, truncated
    /// to 16 bytes and interpreted as a little-endian u128). `c` is derived from
    /// the first input entry. The result `s_n` is computed by iterating
    /// `F(s) = s^3 + c` from `start` for `n` steps.
    ///
    /// The returned `StarkProof` wraps the serialized Winterfell `Proof` in
    /// `proof_of_work`, and sets `commitment` to BLAKE3 of the proof bytes.
    /// `queries` is left empty for MVP (Winterfell handles Merkle auth paths
    /// internally).
    pub fn prove_iteration(
        computation_hash: Hash512,
        inputs: &[Vec<u8>],
        _outputs: &[Vec<u8>],
    ) -> Result<StarkProof, String> {
        // Derive `start` from `computation_hash`.
        let hash_input = Hash512::blake3(computation_hash.as_ref());
        let hash_array = hash_input.to_array();
        let mut start_bytes = [0u8; 16];
        start_bytes.copy_from_slice(&hash_array[..16]);
        let start = BaseElement::new(u128::from_le_bytes(start_bytes));

        // Derive `c` from the first input entry.
        let c_hash = Hash512::blake3(&inputs[0]);
        let c_array = c_hash.to_array();
        let mut c_bytes = [0u8; 16];
        c_bytes.copy_from_slice(&c_array[..16]);
        let c = BaseElement::new(u128::from_le_bytes(c_bytes));

        // Build the execution trace by iterating F(s) = s^3 + c from `start`.
        let mut trace = TraceTable::new(1, TRACE_LENGTH);
        trace.fill(
            |state| {
                state[0] = start;
            },
            |_, state| {
                state[0] = state[0].exp(3u32.into()) + c;
            },
        );

        // Proof options matching the design doc (§8): ~96-bit conjectured security.
        let options = ProofOptions::new(
            32, // num_queries
            8,  // blowup_factor
            0,  // grinding_factor
            FieldExtension::None,
            8,  // fri_folding_factor
            31, // fri_remainder_max_degree
            BatchingMethod::Linear,
            BatchingMethod::Linear,
        );

        let prover = IterationProver::new(options, c);

        let proof = prover
            .prove(trace)
            .map_err(|_| "Winterfell proof generation failed".to_string())?;

        // Serialize the Winterfell proof; this becomes the proof_of_work.
        let proof_bytes = proof.to_bytes();

        // Commitment is BLAKE3 of the serialized proof (which includes all
        // Merkle commitments made during the protocol commit phase).
        let commitment = Hash512::blake3(&proof_bytes);

        Ok(StarkProof {
            commitment,
            queries: vec![],
            proof_of_work: proof_bytes,
        })
    }

    /// Verifies a Winterfell STARK proof with FRI low-degree testing.
    ///
    /// Deserializes the Winterfell `Proof` from `proof_bytes`, reconstructs the
    /// `IterationPublicInputs` deterministically from `computation_hash` and
    /// `public_inputs` (same encoding as [`prove_iteration`]), and runs
    /// Winterfell's verifier. Returns `Ok(true)` if the proof is valid —
    /// including the FRI low-degree test, which rejects forged proofs — and
    /// `Ok(false)` otherwise.
    pub fn verify_winterfell(
        proof_bytes: &[u8],
        computation_hash: Hash512,
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, String> {
        // Empty proof bytes or missing public inputs cannot yield a valid proof.
        if proof_bytes.is_empty() || public_inputs.is_empty() {
            return Ok(false);
        }

        // Deserialize the Winterfell proof. Malformed bytes are treated as an
        // invalid proof rather than propagated as an error.
        let proof = match Proof::from_bytes(proof_bytes) {
            Ok(proof) => proof,
            Err(_) => return Ok(false),
        };

        // Reconstruct public inputs using the same encoding as the prover.
        let pub_inputs = reconstruct_public_inputs(computation_hash, public_inputs);

        // Verify with FRI low-degree testing. Only proofs meeting the minimum
        // conjectured security level (~96-bit for the prover's options) are
        // accepted; any verification failure means the proof is invalid.
        let min_opts = AcceptableOptions::MinConjecturedSecurity(95);
        match verify::<
            IterationAir,
            Blake3_256<BaseElement>,
            DefaultRandomCoin<Blake3_256<BaseElement>>,
            MerkleTree<Blake3_256<BaseElement>>,
        >(proof, pub_inputs, &min_opts)
        {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Reconstructs `IterationPublicInputs` deterministically from
    /// `computation_hash` and `public_inputs`, using the same encoding as
    /// [`prove_iteration`].
    ///
    /// `start` is derived from `computation_hash`; `c` is derived from the
    /// first public input entry; `result` is obtained by iterating
    /// `F(s) = s^3 + c` from `start` for `TRACE_LENGTH` steps; `n` is the
    /// trace length. The verifier must recompute `result` because the proof's
    /// boundary assertions pin the final trace state to it.
    fn reconstruct_public_inputs(
        computation_hash: Hash512,
        public_inputs: &[Vec<u8>],
    ) -> IterationPublicInputs {
        // Derive `start` from `computation_hash`.
        let hash_input = Hash512::blake3(computation_hash.as_ref());
        let hash_array = hash_input.to_array();
        let mut start_bytes = [0u8; 16];
        start_bytes.copy_from_slice(&hash_array[..16]);
        let start = BaseElement::new(u128::from_le_bytes(start_bytes));

        // Derive `c` from the first public input entry.
        let c_hash = Hash512::blake3(&public_inputs[0]);
        let c_array = c_hash.to_array();
        let mut c_bytes = [0u8; 16];
        c_bytes.copy_from_slice(&c_array[..16]);
        let c = BaseElement::new(u128::from_le_bytes(c_bytes));

        // Iterate F(s) = s^3 + c from `start` to recover the final state.
        let mut result = start;
        for _ in 1..TRACE_LENGTH {
            result = result.exp(3u32.into()) + c;
        }

        IterationPublicInputs {
            start,
            result,
            c,
            n: TRACE_LENGTH,
        }
    }
}
