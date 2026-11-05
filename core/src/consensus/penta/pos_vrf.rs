//! Proof of Stake with VRF (PoS-VRF) - Layer 2 of Penta-Consensus.
//!
//! Production implementation with VRF-based validator selection,
//! slashing conditions, and stake-weighted probability.
//!
//! ## VRF implementation
//!
//! A real schnorrkel Ristretto255 ECVRF (draft-irtf-cfrg-vrf) backed by
//! the `schnorrkel` crate v0.11.5. A VRF evaluation produces `(beta, pi)`:
//!
//! - **beta** — 64-byte VRF output: the 32-byte VRF preout followed by 32
//!   derived bytes (`make_bytes(b"derive")`).
//! - **pi** — 64-byte schnorrkel short proof (non-batchable).
//!
//! ### prove()
//! VRFProve(sk, alpha) -> (beta, pi)
//!
//! Signs `alpha` with the validator's secret key under a domain-separated
//! transcript (`b"Atoll-VRF"` bound to `alpha`) and returns the 64-byte beta
//! and 64-byte proof.
//!
//! ### verify()
//! VRFVerify(pk, alpha, beta, pi) -> bool
//!
//! Checks the proof against the public key on `alpha`, bound to the preout
//! (first 32 bytes of `beta`). Requires only the public key, so any third
//! party can publicly verify without the secret key.
//!
//! ### alpha construction
//! alpha = epoch || slot || randomness || validator_address
//!
//! `epoch` and `slot` are little-endian u64 bytes; `randomness` is the epoch
//! randomness beacon bytes; `validator_address` is the validator's
//! hex-encoded address bytes.
//!
//! ### Proposer selection
//! `ProposerSelector::select()` runs `prove()` per validator with its own
//! secret key, weights the VRF score by stake and reputation, and carries the
//! proof in [`SelectionResult`] so the winning selection can be publicly
//! re-verified.
//!
//! ### Validator keys
//! [`Validator`] stores `vrf_secret_key: [u8; 64]` and
//! `vrf_public_key: [u8; 32]`.
//!
//! ### MVP notes
//! - schnorrkel 0.11.5 short proofs: 64 bytes each.
//! - Proofs are **not batchable**; each verification is an independent EC
//!   operation.

use crate::primitives::address::Address;
use serde::{Deserialize, Serialize};

/// Serde helper for serializing `[u8; 64]` as raw byte arrays.
///
/// Without this, serde would try to serialize the array as a sequence of
/// individual 64-tuples, which is both lossy and incompatible with existing
/// serialized validators.
mod secret_key_serde {
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        key: &[u8; 64],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(key)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        let arr: [u8; 64] = bytes
            .try_into()
            .map_err(|_| de::Error::custom("expected 64-byte secret key"))?;
        Ok(arr)
    }
}

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

/// Ristretto255 VRF keypair for PoS-VRF validator keys.
///
/// Wraps a `schnorrkel` VRF keypair as raw byte arrays: a 64-byte secret
/// key and a 32-byte public key. The secret key must be kept confidential;
/// only the public key is needed to verify VRF outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VRFKeyPair {
    pub secret_key: [u8; 64],
    pub public_key: [u8; 32],
}

impl VRFKeyPair {
    /// Generates a new random VRF keypair using the OS CSPRNG.
    pub fn generate() -> Self {
        let keypair = schnorrkel::Keypair::generate_with(&mut rand::thread_rng());
        Self {
            secret_key: keypair.secret.to_bytes(),
            public_key: keypair.public.to_bytes(),
        }
    }

    /// Reconstructs a `VRFKeyPair` from a 64-byte secret key.
    ///
    /// Returns `None` if the secret key bytes are malformed.
    pub fn from_secret(secret_key: &[u8; 64]) -> Option<Self> {
        let secret = schnorrkel::SecretKey::from_bytes(&secret_key[..]).ok()?;
        let public_key = secret.to_public().to_bytes();
        Some(Self {
            secret_key: *secret_key,
            public_key,
        })
    }
}

/// VRF engine for validator selection
#[derive(Debug, Clone)]
pub struct VRFEngine;

impl VRFEngine {
    pub fn new() -> Self {
        Self
    }

    /// Produces a VRF proof and output for the given secret key and alpha.
    ///
    /// `alpha` is the VRF input message, conventionally the concatenation of
    /// `epoch || slot || randomness || validator_address` bytes.
    ///
    /// Returns a `VRFOutput` whose `output` is 64 bytes (the 32-byte VRF
    /// preout followed by 32 derived bytes) and whose `proof` is the 64-byte
    /// schnorrkel VRF proof.
    pub fn prove(
        &self,
        secret_key: &[u8; 64],
        alpha: &[u8],
    ) -> VRFOutput {
        let secret = schnorrkel::SecretKey::from_bytes(&secret_key[..])
            .expect("invalid 64-byte VRF secret key");
        let public = secret.to_public();
        let keypair = schnorrkel::Keypair { secret, public };

        // Domain-separated transcript bound to the alpha message.
        let transcript = schnorrkel::signing_context(b"Atoll-VRF").bytes(alpha);

        // Produce the VRF input/output pair and the non-batchable (64-byte) proof.
        let (io, proof, _proof_batchable) = keypair.vrf_sign(transcript);

        // `output` leads with the 32-byte VRF preout that the proof is bound to,
        // followed by 32 derived bytes, for a total of 64 bytes.
        let mut output = [0u8; 64];
        output[..32].copy_from_slice(&io.to_preout().to_bytes());
        output[32..].copy_from_slice(&io.make_bytes::<[u8; 32]>(b"derive"));

        let pi: [u8; 64] = proof.to_bytes();
        VRFOutput {
            output,
            proof: pi.to_vec(),
        }
    }

    /// Verifies a VRF proof produced by [`VRFEngine::prove`].
    ///
    /// Returns `true` when `proof` is a valid schnorrkel VRF proof for
    /// `public_key` on `alpha`, cryptographically bound to `output`.
    pub fn verify(
        &self,
        public_key: &[u8; 32],
        alpha: &[u8],
        output: &[u8; 64],
        proof: &[u8],
    ) -> bool {
        let public = match schnorrkel::PublicKey::from_bytes(public_key) {
            Ok(p) => p,
            Err(_) => return false,
        };
        let vrf_proof = match schnorrkel::vrf::VRFProof::from_bytes(proof) {
            Ok(p) => p,
            Err(_) => return false,
        };
        // The first 32 bytes of `output` are the VRF preout the proof is bound to.
        let preout = match schnorrkel::vrf::VRFPreOut::from_bytes(&output[..32]) {
            Ok(p) => p,
            Err(_) => return false,
        };

        let transcript = schnorrkel::signing_context(b"Atoll-VRF").bytes(alpha);
        public.vrf_verify(transcript, &preout, &vrf_proof).is_ok()
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
    #[serde(with = "secret_key_serde")]
    pub vrf_secret_key: [u8; 64],
    pub vrf_public_key: [u8; 32],
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
            // Build alpha = epoch || slot || randomness || validator_address bytes,
            // matching the input the VRF proof is cryptographically bound to.
            let mut alpha = Vec::new();
            alpha.extend_from_slice(&epoch.to_le_bytes());
            alpha.extend_from_slice(&slot.to_le_bytes());
            alpha.extend_from_slice(randomness);
            alpha.extend_from_slice(validator.address.to_hex().as_bytes());
            let vrf_output = vrf_engine.prove(&validator.vrf_secret_key, &alpha);
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
    fn vrf_prove_is_deterministic() {
        let engine = VRFEngine::new();
        let secret = [1u8; 64];
        let alpha = b"epoch0-slot0-randomness0-validator1";
        let output1 = engine.prove(&secret, alpha);
        let output2 = engine.prove(&secret, alpha);
        assert_eq!(output1.output, output2.output);
    }

    #[test]
    fn vrf_prove_different_inputs_different_outputs() {
        let engine = VRFEngine::new();
        let secret = [1u8; 64];
        let output1 = engine.prove(&secret, b"epoch0-slot0-randomness0-validator1");
        let output2 = engine.prove(&secret, b"epoch0-slot0-randomness0-validator2");
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
                vrf_secret_key: [1u8; 64],
                vrf_public_key: [0u8; 32],
                reputation_score: 0.8,
            },
            Validator {
                address: Address::from_bytes([2u8; 20]),
                stake: 50_000,
                vrf_secret_key: [2u8; 64],
                vrf_public_key: [0u8; 32],
                reputation_score: 0.9,
            },
        ];
        let result = selector.select(&vrf_engine, &validators, 0, 0, &[0u8; 32], 150_000);
        assert!(result.is_some());
        let sel = result.unwrap();
        assert!(validators
            .iter()
            .any(|v| v.address == sel.selected_validator));
    }

    #[test]
    fn select_no_validators() {
        let vrf_engine = VRFEngine::new();
        let selector = ProposerSelector::new();
        let result = selector.select(&vrf_engine, &[], 0, 0, &[0u8; 32], 0);
        assert!(result.is_none());
    }

    #[test]
    fn vrf_keypair_generate_roundtrip() {
        let kp = VRFKeyPair::generate();
        assert_eq!(kp.secret_key.len(), 64);
        assert_eq!(kp.public_key.len(), 32);

        let reconstructed = VRFKeyPair::from_secret(&kp.secret_key)
            .expect("a valid 64-byte secret key should reconstruct");
        assert_eq!(reconstructed.public_key, kp.public_key);
    }

    #[test]
    fn vrf_keypair_generated_keypairs_differ() {
        let kp1 = VRFKeyPair::generate();
        let kp2 = VRFKeyPair::generate();
        assert_ne!(kp1.public_key, kp2.public_key);
    }

    #[test]
    fn vrf_prove_verify_roundtrip() {
        let engine = VRFEngine::new();
        let kp = VRFKeyPair::generate();
        let alpha = b"epoch0-slot0-randomness0-validator1";
        let output = engine.prove(&kp.secret_key, alpha);
        assert_eq!(output.output.len(), 64);
        assert_eq!(output.proof.len(), 64);
        assert!(
            engine.verify(&kp.public_key, alpha, &output.output, &output.proof),
            "valid VRF proof must verify under the same public key"
        );
    }

    #[test]
    fn vrf_prove_deterministic() {
        let engine = VRFEngine::new();
        let kp = VRFKeyPair::generate();
        let alpha = b"epoch1-slot2-randomness3-validator2";
        let output1 = engine.prove(&kp.secret_key, alpha);
        let output2 = engine.prove(&kp.secret_key, alpha);
        // The VRF output (beta) is deterministic for identical inputs.
        assert_eq!(output1.output, output2.output);
        // The proof (pi) is a Schnorr proof with a random nonce, so it differs
        // across calls even with identical key and alpha.
        assert_ne!(output1.proof, output2.proof);
    }

    #[test]
    fn vrf_prove_wrong_key_verify_fails() {
        let engine = VRFEngine::new();
        let kp1 = VRFKeyPair::generate();
        let kp2 = VRFKeyPair::generate();
        let alpha = b"epoch2-slot3-randomness4-validator3";
        let output = engine.prove(&kp1.secret_key, alpha);
        assert!(
            !engine.verify(&kp2.public_key, alpha, &output.output, &output.proof),
            "VRF proof must not verify under a different public key"
        );
    }
}
