//! Proof of Stake with VRF (PoS-VRF) - Layer 2 of Penta-Consensus.
//!
//! Validators are selected using a Verifiable Random Function (VRF) weighted
//! by stake and reputation. Slashing conditions penalize double-voting and
//! equivocation.

use crate::primitives::address::Address;

/// Result of PoS-VRF validation
#[derive(Debug, Clone)]
pub struct PoSResult {
    pub valid: bool,
    pub combined_score: f64,
}

/// VRF proof output
#[derive(Debug, Clone)]
pub struct VRFOutput {
    pub output: [u8; 64],
    pub proof: Vec<u8>,
}

/// VRF engine for validator selection
#[derive(Debug, Clone)]
pub struct VRFEngine;

impl VRFEngine {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates the VRF for a validator
    pub fn evaluate(
        &self,
        secret_key: &[u8; 32],
        epoch: u64,
        slot: u64,
        randomness: &[u8],
        validator_address: &str,
    ) -> VRFOutput {
        // Phase 1: Simulated VRF using BLAKE3 keyed hashing
        // Phase 2: Real VRF implementation (e.g., EC-VRF)
        use blake3::Hasher;
        let mut hasher = Hasher::new_keyed(secret_key);
        hasher.update(&epoch.to_le_bytes());
        hasher.update(&slot.to_le_bytes());
        hasher.update(randomness);
        hasher.update(validator_address.as_bytes());
        let result = hasher.finalize();
        let bytes = result.as_bytes();
        let mut output = [0u8; 64];
        output.copy_from_slice(bytes);
        VRFOutput {
            output,
            proof: bytes.to_vec(),
        }
    }

    /// Converts VRF output to a score in [0, 1]
    pub fn output_to_score(output: &[u8; 64]) -> f64 {
        // Use first 8 bytes as a fraction of u64::MAX
        let val = u64::from_le_bytes(output[..8].try_into().unwrap_or([0; 8]));
        val as f64 / u64::MAX as f64
    }
}

/// Validator in the PoS-VRF system
#[derive(Debug, Clone)]
pub struct Validator {
    pub address: Address,
    pub stake: u128,
    pub vrf_secret_key: [u8; 32],
    pub reputation_score: f64,
}

/// PoS-VRF selection result
#[derive(Debug, Clone)]
pub struct SelectionResult {
    pub selected_validator: Address,
    pub score: f64,
    pub vrf_output: VRFOutput,
}

/// Selects a block proposer using VRF weighted by stake and reputation
pub struct ProposerSelector;

impl ProposerSelector {
    pub fn new() -> Self {
        Self
    }

    /// Selects the proposer with the highest VRF-weighted score
    pub fn select(
        &self,
        vrf_engine: &VRFEngine,
        validators: &[Validator],
        epoch: u64,
        slot: u64,
        randomness: &[u8],
        total_stake: u128,
    ) -> Option<SelectionResult> {
        if validators.is_empty() {
            return None;
        }

        let mut best: Option<SelectionResult> = None;

        for validator in validators {
            let vrf_output = vrf_engine.evaluate(
                &validator.vrf_secret_key,
                epoch,
                slot,
                randomness,
                &validator.address.to_hex(),
            );
            let vrf_score = VRFEngine::output_to_score(&vrf_output.output);

            let stake_weight = if total_stake > 0 {
                validator.stake as f64 / total_stake as f64
            } else {
                0.0
            };

            let score = vrf_score * stake_weight * validator.reputation_score;

            match &best {
                None => {
                    best = Some(SelectionResult {
                        selected_validator: validator.address.clone(),
                        score,
                        vrf_output,
                    });
                }
                Some(current) if score > current.score => {
                    best = Some(SelectionResult {
                        selected_validator: validator.address.clone(),
                        score,
                        vrf_output,
                    });
                }
                _ => {}
            }
        }

        best
    }
}
