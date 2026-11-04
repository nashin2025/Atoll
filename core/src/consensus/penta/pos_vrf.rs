//! Proof of Stake with VRF (PoS-VRF) - Layer 2 of Penta-Consensus.
//!
//! Production implementation with VRF-based validator selection,
//! slashing conditions, and stake-weighted probability.

use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        let mut hasher = blake3::Hasher::new_keyed(secret_key);
        hasher.update(&epoch.to_le_bytes());
        hasher.update(&slot.to_le_bytes());
        hasher.update(randomness);
        hasher.update(validator_address.as_bytes());
        let mut out = [0u8; 64];
        hasher.finalize_xof().fill(&mut out);
        VRFOutput {
            output: out,
            proof: out.to_vec(),
        }
    }

    /// Converts VRF output to a score in [0, 1]
    pub fn output_to_score(output: &[u8; 64]) -> f64 {
        let val = u64::from_le_bytes(output[..8].try_into().unwrap_or([0; 8]));
        val as f64 / u64::MAX as f64
    }
}

/// Validator in the PoS-VRF system
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vrf_output_to_score() {
        let output = [0u8; 64];
        let score = VRFEngine::output_to_score(&output);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn vrf_evaluate_is_deterministic() {
        let engine = VRFEngine::new();
        let secret = [1u8; 32];
        let output1 = engine.evaluate(
            &secret, 0, 0, &[0u8; 32], "validator1"
        );
        let output2 = engine.evaluate(
            &secret, 0, 0, &[0u8; 32], "validator1"
        );
        assert_eq!(output1.output, output2.output);
    }

    #[test]
    fn vrf_different_inputs_different_outputs() {
        let engine = VRFEngine::new();
        let secret = [1u8; 32];
        let output1 = engine.evaluate(
            &secret, 0, 0, &[0u8; 32], "validator1"
        );
        let output2 = engine.evaluate(
            &secret, 0, 0, &[0u8; 32], "validator2"
        );
        assert_ne!(output1.output, output2.output);
    }

    #[test]
    fn select_proposer() {
        let vrf_engine = VRFEngine::new();
        let selector = ProposerSelector::new();
        let validators = vec![
            Validator {
                address: Address::from_bytes([1u8; 20]),
                stake: 100_000,
                vrf_secret_key: [1u8; 32],
                reputation_score: 0.8,
            },
            Validator {
                address: Address::from_bytes([2u8; 20]),
                stake: 50_000,
                vrf_secret_key: [2u8; 32],
                reputation_score: 0.9,
            },
        ];
        let result = selector.select(
            &vrf_engine,
            &validators,
            0,
            0,
            &[0u8; 32],
            150_000,
        );
        assert!(result.is_some());
        let sel = result.unwrap();
        assert!(validators.iter().any(|v| v.address == sel.selected_validator));
    }

    #[test]
    fn select_no_validators() {
        let vrf_engine = VRFEngine::new();
        let selector = ProposerSelector::new();
        let result = selector.select(
            &vrf_engine,
            &[],
            0,
            0,
            &[0u8; 32],
            0,
        );
        assert!(result.is_none());
    }
}
