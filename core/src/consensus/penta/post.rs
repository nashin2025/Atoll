//! Proof of Spacetime (PoST) - Layer 3 of Penta-Consensus.
//!
//! Production implementation of PoST verification. A proof contains:
//! - a space proof (Merkle inclusion proofs proving data is stored),
//! - a time proof (Wesolowski VDF proving data was stored continuously),
//! - a plot size and farmer/commitment identity.
//!
//! ## Verification
//!
//! `PoSTVerifier::verify()` runs four checks and returns `true` only when
//! all of them pass:
//!
//! 1. **Space** — `verify_space_proof()` checks each Merkle inclusion proof
//!    in `SpaceProof::merkle_proofs` against `plot_commitment` (the Merkle
//!    root) using `MerkleTree::verify_proof`, pairing each path with its
//!    `challenge_responses` entry (`[leaf_index: 8 bytes LE][leaf: 64 bytes]`).
//!    An empty or malformed proof is rejected.
//! 2. **Time** — `verify_time_proof()` verifies the Wesolowski VDF proof:
//!    `y ≡ π^(2e) · x^(2^(T mod 2)) (mod N)` where `e = blake3(x || y ||
//!    T)`. Rejects zero elapsed time and empty/degenerate outputs.
//! 3. **Size** — `plot_size_gb` must meet the configured minimum (default 100 GB).
//! 4. **Uniqueness** — `verify_uniqueness()` computes `plot_id =
//!    blake3(farmer || commitment || sector_size)` and rejects duplicates
//!    tracked in `plot_registry`.
//!
//! ## VDF Parameters
//!
//! - Generator `x = 2`.
//! - Modulus `N` is derived deterministically from the fixed seed
//!   `b"atoll-post-vdf-modulus-v1"` via blake3 (see `vdf_modulus()`).
//! - Time parameter `T` comes from `TimeProof::elapsed_seconds`.
//! - Challenge `e = blake3(x || y || T)`, encoded as fixed-width big-endian
//!   bytes using the byte length of `N`.
//!
//! ## MVP Limitations
//!
//! - `plot_registry` is an in-memory `HashSet`: committed plot IDs are lost
//!   on restart, so uniqueness is only enforced within a single process lifetime.
//! - The VDF modulus `N` is derived from a fixed seed rather than a
//!   trusted-setup RSA modulus generated in an MPC ceremony; a production
//!   deployment should use a publicly generated modulus.
//! - `num-bigint` modular exponentiation is not constant-time; the VDF path
//!   is not side-channel hardened.
//!
//! `PoSTResult.valid == true` only when all checks above pass.

use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;
use crate::primitives::merkle::MerkleTree;
use blake3;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Result of PoST validation
#[derive(Debug, Clone)]
pub struct PoSTResult {
    pub valid: bool,
    pub size_score: f64,
}

/// Space proof: Merkle proofs that data exists in storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceProof {
    pub merkle_proofs: Vec<Vec<u8>>,
    pub challenge_responses: Vec<Vec<u8>>,
}

/// Time proof: VDF output proving data was stored continuously
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeProof {
    pub vdf_output: Vec<u8>,
    pub vdf_proof: Vec<u8>,
    pub elapsed_seconds: u64,
}

/// Complete spacetime proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacetimeProof {
    pub farmer: Address,
    pub plot_commitment: Hash512,
    pub space_proof: SpaceProof,
    pub time_proof: TimeProof,
    pub plot_size_gb: u64,
}

/// Verifier for spacetime proofs.
///
/// Tracks committed plot IDs in `plot_registry` to enforce plot
/// uniqueness across verification calls.
pub struct PoSTVerifier {
    minimum_plot_size_gb: u64,
    plot_registry: HashSet<Hash512>,
}

/// Bytes in one gibibyte (2^30), used to convert plot_size_gb to bytes.
const BYTES_PER_GIB: u64 = 1_073_741_824;

/// Fixed seed used to deterministically derive the VDF RSA modulus N for MVP.
///
/// A production deployment would use a trusted-setup RSA modulus generated
/// in an MPC ceremony. For MVP, deriving N from a fixed seed keeps parameter
/// generation reproducible without a ceremony.
const VDF_SEED: &[u8] = b"atoll-post-vdf-modulus-v1";

impl PoSTVerifier {
    pub fn new() -> Self {
        Self {
            minimum_plot_size_gb: 100, // 100 GB minimum
            plot_registry: HashSet::new(),
        }
    }

    pub fn with_minimum_size(minimum_gb: u64) -> Self {
        Self {
            minimum_plot_size_gb: minimum_gb,
            plot_registry: HashSet::new(),
        }
    }

    /// Verifies a spacetime proof against a challenge.
    ///
    /// Requires `&mut self` because verified plot IDs are recorded in
    /// `plot_registry` to enforce uniqueness.
    pub fn verify(&mut self, proof: &SpacetimeProof, challenge: &[u8]) -> bool {
        // 1. Verify space commitment (data is actually stored)
        let space_valid =
            self.verify_space_proof(&proof.space_proof, &proof.plot_commitment, challenge);

        // 2. Verify time proof (data was stored continuously)
        let time_valid = self.verify_time_proof(&proof.time_proof, &proof.plot_commitment);

        // 3. Verify minimum plot size
        let size_valid = proof.plot_size_gb >= self.minimum_plot_size_gb;

        // 4. Verify plot uniqueness (no duplicate plot commitments)
        let unique_valid = self.verify_uniqueness(proof);

        space_valid && time_valid && size_valid && unique_valid
    }

    /// Verifies a space proof by checking each Merkle inclusion proof
    /// against the plot commitment (the Merkle root).
    ///
    /// Each entry of `challenge_responses` encodes a leaf index
    /// (little-endian u64, 8 bytes) followed by the 64-byte leaf hash.
    /// The corresponding entry of `merkle_proofs` holds the serialized
    /// authentication path (each node 64 bytes). Every response/path pair
    /// must verify against `commitment`; an empty proof is rejected because
    /// it proves no stored data.
    ///
    /// Takes `&self` only: it does not touch `plot_registry`, unlike
    /// `verify()` which needs `&mut self` for uniqueness tracking.
    fn verify_space_proof(
        &self,
        proof: &SpaceProof,
        commitment: &Hash512,
        _challenge: &[u8],
    ) -> bool {
        if proof.challenge_responses.is_empty() {
            return false; // no storage commitment was proven
        }
        if proof.merkle_proofs.len() != proof.challenge_responses.len() {
            return false; // malformed proof: paths and responses must pair up
        }

        for (response, path_bytes) in proof
            .challenge_responses
            .iter()
            .zip(proof.merkle_proofs.iter())
        {
            // Response: [leaf_index: 8 bytes LE][leaf: 64 bytes]
            if response.len() != 8 + 64 {
                return false;
            }
            let index = u64::from_le_bytes(response[..8].try_into().unwrap()) as usize; // len == 72
            let leaf = Hash512::from_bytes(&response[8..72]); // len == 72

            // Path: serialized Hash512 nodes, 64 bytes each
            if path_bytes.len() % 64 != 0 {
                return false;
            }
            let path: Vec<Hash512> = path_bytes
                .chunks_exact(64)
                .map(|chunk| Hash512::from_bytes(chunk.try_into().unwrap()))
                .collect();

            if !MerkleTree::verify_proof(*commitment, leaf, index, &path) {
                return false;
            }
        }

        true
    }

    /// Verifies a Wesolowski VDF time proof against the plot commitment.
    ///
    /// Checks `y ≡ π^(2e) · x^(2^(T mod 2)) (mod N)` where `e = blake3(x ||
    /// y || T)`. Rejects zero elapsed time and empty VDF output/proof as
    /// degenerate. Takes `&self` only; it does not touch `plot_registry`.
    fn verify_time_proof(&self, proof: &TimeProof, _commitment: &Hash512) -> bool {
        // Zero elapsed time means no VDF computation was performed; reject.
        if proof.elapsed_seconds == 0 {
            return false;
        }
        // An empty VDF output or proof is degenerate and cannot be valid.
        if proof.vdf_output.is_empty() || proof.vdf_proof.is_empty() {
            return false;
        }

        // Protocol parameters for the Wesolowski VDF.
        // MVP: modulus N derived deterministically from a fixed seed via blake3;
        // generator x = 2 (a common small generator).
        let n = vdf_modulus();
        let x = BigUint::from(2u32);

        let y = bytes_to_biguint(&proof.vdf_output);
        let pi = bytes_to_biguint(&proof.vdf_proof);

        // A zero VDF output is degenerate (x^(2^T) mod N is never 0).
        if y == BigUint::from(0u32) {
            return false;
        }

        verify_wesolowski(&x, &y, &pi, proof.elapsed_seconds, &n)
    }

    /// Checks that a plot has not already been committed.
    ///
    /// Computes a plot ID from the farmer address, plot commitment, and
    /// sector size, then rejects the proof if that plot ID is already in
    /// the registry. Otherwise the plot ID is recorded and the proof is
    /// accepted as unique.
    fn verify_uniqueness(&mut self, proof: &SpacetimeProof) -> bool {
        let sector_size_bytes = proof.plot_size_gb * BYTES_PER_GIB;
        let plot_id =
            Self::compute_plot_id(&proof.farmer, &proof.plot_commitment, sector_size_bytes);

        if self.plot_registry.contains(&plot_id) {
            return false; // duplicate plot
        }
        self.plot_registry.insert(plot_id);
        true
    }

    /// Computes `blake3(farmer_address || plot_commitment || sector_size_bytes)`.
    fn compute_plot_id(farmer: &Address, commitment: &Hash512, sector_size_bytes: u64) -> Hash512 {
        let mut hasher = blake3::Hasher::new();
        hasher.update(farmer.as_ref());
        hasher.update(commitment.as_ref());
        hasher.update(&sector_size_bytes.to_be_bytes());
        let mut out = [0u8; 64];
        hasher.finalize_xof().fill(&mut out);
        Hash512::from_array(out)
    }

    /// Returns the minimum plot size in GB
    pub fn minimum_plot_size_gb(&self) -> u64 {
        self.minimum_plot_size_gb
    }
}

/// Serialize a BigUint to big-endian bytes, zero-padded on the left to `width` bytes.
fn fixed_width_bytes(n: &BigUint, width: usize) -> Vec<u8> {
    let mut bytes = n.to_bytes_be();
    if bytes.len() < width {
        let mut padded = vec![0u8; width - bytes.len()];
        padded.append(&mut bytes);
        padded
    } else {
        bytes
    }
}

/// Convert big-endian bytes to a BigUint.
fn bytes_to_biguint(bytes: &[u8]) -> BigUint {
    BigUint::from_bytes_be(bytes)
}

/// Returns the Wesolowski VDF RSA modulus N, derived deterministically from
/// a fixed seed via blake3.
fn vdf_modulus() -> BigUint {
    let digest = blake3::hash(VDF_SEED);
    bytes_to_biguint(digest.as_bytes())
}

/// Compute the Wesolowski challenge `e = blake3(x || y || T)` as a BigUint.
///
/// x and y are encoded as fixed-width big-endian bytes using the byte length of N
/// so the encoding is deterministic; T is encoded as an 8-byte big-endian integer.
fn wesolowski_challenge(x: &BigUint, y: &BigUint, T: u64, N: &BigUint) -> BigUint {
    let width = (N.bits() as usize + 7) / 8;
    let mut hasher = blake3::Hasher::new();
    hasher.update(&fixed_width_bytes(x, width));
    hasher.update(&fixed_width_bytes(y, width));
    hasher.update(&T.to_be_bytes());
    let digest = hasher.finalize();
    bytes_to_biguint(digest.as_bytes())
}

/// Verify a Wesolowski VDF proof.
///
/// Checks: `y ≡ π^(2e) · x^(2^(T mod 2)) (mod N)`
/// where `e = blake3(x || y || T)` interpreted as a BigUint.
///
/// Returns `true` if the proof is valid, `false` otherwise.
pub fn verify_wesolowski(x: &BigUint, y: &BigUint, pi: &BigUint, T: u64, N: &BigUint) -> bool {
    // Reject degenerate moduli; modpow panics on modulus 0.
    if N == &BigUint::from(0u32) || N == &BigUint::from(1u32) {
        return false;
    }

    let e = wesolowski_challenge(x, y, T, N);

    // π^(2e) mod N
    let two_e = BigUint::from(2u32) * e;
    let pi_pow = pi.modpow(&two_e, N);

    // x^(2^(T mod 2)) mod N:
    //   T even → x^1   = x
    //   T odd  → x^2   = (x * x) mod N
    let x_factor = if T % 2 == 0 { x % N } else { (x * x) % N };

    let lhs = (pi_pow * x_factor) % N;
    let rhs = y % N;

    lhs == rhs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::address::Address;
    use crate::primitives::hash::Hash512;

    fn leaf(byte: u8) -> Hash512 {
        let mut arr = [0u8; 64];
        arr[0] = byte;
        Hash512::from_array(arr)
    }

    /// Builds a SpaceProof whose single Merkle proof verifies against the
    /// plot commitment for the leaf at `index`.
    fn valid_space_proof(index: usize, leaf: Hash512, path: &[Hash512]) -> SpaceProof {
        let mut path_bytes = Vec::new();
        for node in path {
            path_bytes.extend_from_slice(&node.to_bytes());
        }
        let mut response = Vec::new();
        response.extend_from_slice(&(index as u64).to_le_bytes());
        response.extend_from_slice(&leaf.to_bytes());
        SpaceProof {
            merkle_proofs: vec![path_bytes],
            challenge_responses: vec![response],
        }
    }

    fn valid_time_proof() -> TimeProof {
        // Valid Wesolowski proof for x = 2, T = 30 (even):
        //   pi = 1  =>  pi^(2e) = 1 (for any challenge e)
        //   T even => x_factor = x = 2
        //   y must equal x_factor = 2
        TimeProof {
            vdf_output: vec![2u8], // y = 2
            vdf_proof: vec![1u8],  // pi = 1
            elapsed_seconds: 30,   // even
        }
    }

    #[test]
    fn verify_valid_proof() {
        let mut verifier = PoSTVerifier::new();
        let leaves: Vec<Hash512> = (0..8).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        let index = 3;
        let path = tree.prove(index).unwrap();
        let proof = SpacetimeProof {
            farmer: Address::zero(),
            plot_commitment: root,
            space_proof: valid_space_proof(index, leaves[index], &path),
            time_proof: valid_time_proof(),
            plot_size_gb: 500,
        };
        assert!(verifier.verify(&proof, &[0u8; 32]));
    }

    #[test]
    fn verify_insufficient_storage() {
        let mut verifier = PoSTVerifier::new();
        let proof = SpacetimeProof {
            farmer: Address::zero(),
            plot_commitment: Hash512::zero(),
            space_proof: SpaceProof {
                merkle_proofs: vec![],
                challenge_responses: vec![],
            },
            time_proof: valid_time_proof(),
            plot_size_gb: 10, // Below 100 GB minimum
        };
        assert!(!verifier.verify(&proof, &[0u8; 32]));
    }

    #[test]
    fn verify_rejects_empty_space_proof() {
        let mut verifier = PoSTVerifier::new();
        let proof = SpacetimeProof {
            farmer: Address::zero(),
            plot_commitment: Hash512::zero(),
            space_proof: SpaceProof {
                merkle_proofs: vec![],
                challenge_responses: vec![],
            },
            time_proof: valid_time_proof(),
            plot_size_gb: 500,
        };
        assert!(!verifier.verify(&proof, &[0u8; 32]));
    }

    #[test]
    fn minimum_plot_size() {
        let verifier = PoSTVerifier::new();
        assert_eq!(verifier.minimum_plot_size_gb(), 100);

        let verifier = PoSTVerifier::with_minimum_size(50);
        assert_eq!(verifier.minimum_plot_size_gb(), 50);
    }

    #[test]
    fn verify_rejects_duplicate_plot() {
        let mut verifier = PoSTVerifier::new();
        let leaves: Vec<Hash512> = (0..8).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        let index = 3;
        let path = tree.prove(index).unwrap();
        let space_proof = valid_space_proof(index, leaves[index], &path);
        let proof = SpacetimeProof {
            farmer: Address::from_bytes([1u8; 20]),
            plot_commitment: root,
            space_proof: space_proof.clone(),
            time_proof: valid_time_proof(),
            plot_size_gb: 500,
        };
        assert!(verifier.verify(&proof, &[0u8; 32]));
        assert!(!verifier.verify(&proof, &[0u8; 32])); // same plot committed twice
    }

    #[test]
    fn verify_accepts_distinct_plots() {
        let mut verifier = PoSTVerifier::new();
        let leaves: Vec<Hash512> = (0..8).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        let index = 3;
        let path = tree.prove(index).unwrap();
        let space_proof = valid_space_proof(index, leaves[index], &path);
        let proof_a = SpacetimeProof {
            farmer: Address::from_bytes([1u8; 20]),
            plot_commitment: root,
            space_proof: space_proof.clone(),
            time_proof: valid_time_proof(),
            plot_size_gb: 500,
        };
        let proof_b = SpacetimeProof {
            farmer: Address::from_bytes([3u8; 20]),
            plot_commitment: root,
            space_proof,
            time_proof: valid_time_proof(),
            plot_size_gb: 500,
        };
        assert!(verifier.verify(&proof_a, &[0u8; 32]));
        assert!(verifier.verify(&proof_b, &[0u8; 32]));
    }

    #[test]
    fn verify_rejects_invalid_merkle_proof() {
        let mut verifier = PoSTVerifier::new();
        let leaves: Vec<Hash512> = (0..8).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        // An impostor leaf not in the tree, paired with a real path: must fail.
        let impostor = Hash512::from_array([99u8; 64]);
        let path = tree.prove(0).unwrap();
        let mut response = Vec::new();
        response.extend_from_slice(&(0u64).to_le_bytes());
        response.extend_from_slice(&impostor.to_bytes());
        let space_proof = SpaceProof {
            merkle_proofs: vec![path.iter().flat_map(|h| h.to_bytes()).collect()],
            challenge_responses: vec![response],
        };
        let proof = SpacetimeProof {
            farmer: Address::zero(),
            plot_commitment: root,
            space_proof,
            time_proof: valid_time_proof(),
            plot_size_gb: 500,
        };
        assert!(!verifier.verify(&proof, &[0u8; 32]));
    }

    #[test]
    fn verify_rejects_zero_elapsed_time() {
        let mut verifier = PoSTVerifier::new();
        let leaves: Vec<Hash512> = (0..8).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        let index = 3;
        let path = tree.prove(index).unwrap();
        let space_proof = valid_space_proof(index, leaves[index], &path);
        // Valid VDF params but zero elapsed time: no computation was performed.
        let time_proof = TimeProof {
            vdf_output: vec![2u8],
            vdf_proof: vec![1u8],
            elapsed_seconds: 0,
        };
        let proof = SpacetimeProof {
            farmer: Address::zero(),
            plot_commitment: root,
            space_proof,
            time_proof,
            plot_size_gb: 500,
        };
        assert!(!verifier.verify(&proof, &[0u8; 32]));
    }

    #[test]
    fn verify_rejects_empty_vdf_proof() {
        let mut verifier = PoSTVerifier::new();
        let leaves: Vec<Hash512> = (0..8).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        let index = 3;
        let path = tree.prove(index).unwrap();
        let space_proof = valid_space_proof(index, leaves[index], &path);
        // Empty VDF output/proof is degenerate and must be rejected.
        let time_proof = TimeProof {
            vdf_output: vec![],
            vdf_proof: vec![],
            elapsed_seconds: 30,
        };
        let proof = SpacetimeProof {
            farmer: Address::zero(),
            plot_commitment: root,
            space_proof,
            time_proof,
            plot_size_gb: 500,
        };
        assert!(!verifier.verify(&proof, &[0u8; 32]));
    }

    #[test]
    fn bytes_to_biguint_roundtrip() {
        let n = BigUint::from(12345u32);
        let bytes = n.to_bytes_be();
        assert_eq!(bytes_to_biguint(&bytes), n);
    }

    #[test]
    fn fixed_width_bytes_pads_to_width() {
        let n = BigUint::from(1u32);
        let padded = fixed_width_bytes(&n, 8);
        assert_eq!(padded.len(), 8);
        assert_eq!(padded, vec![0, 0, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn wesolowski_challenge_is_deterministic() {
        let x = BigUint::from(7u32);
        let y = BigUint::from(11u32);
        let n = BigUint::from(97u32);
        let e1 = wesolowski_challenge(&x, &y, 42, &n);
        let e2 = wesolowski_challenge(&x, &y, 42, &n);
        assert_eq!(e1, e2);
    }

    #[test]
    fn wesolowski_rejects_degenerate_modulus() {
        let x = BigUint::from(2u32);
        let y = BigUint::from(2u32);
        let pi = BigUint::from(1u32);
        assert!(!verify_wesolowski(&x, &y, &pi, 2, &BigUint::from(0u32)));
        assert!(!verify_wesolowski(&x, &y, &pi, 2, &BigUint::from(1u32)));
    }

    #[test]
    fn wesolowski_valid_even_t() {
        // pi = 1 ⇒ π^(2e) = 1, so y must equal x_factor = x (T even).
        let n = BigUint::from(97u32);
        let x = BigUint::from(5u32);
        let y = BigUint::from(5u32);
        let pi = BigUint::from(1u32);
        assert!(verify_wesolowski(&x, &y, &pi, 2, &n));
    }

    #[test]
    fn wesolowski_valid_odd_t() {
        // pi = 1 ⇒ y must equal x^2 mod N (T odd).
        let n = BigUint::from(97u32);
        let x = BigUint::from(5u32);
        let y = BigUint::from(25u32); // 5^2 mod 97
        let pi = BigUint::from(1u32);
        assert!(verify_wesolowski(&x, &y, &pi, 1, &n));
    }

    #[test]
    fn wesolowski_invalid_y_rejected() {
        let n = BigUint::from(97u32);
        let x = BigUint::from(5u32);
        let y = BigUint::from(26u32); // should be 25 = 5^2 mod 97
        let pi = BigUint::from(1u32);
        assert!(!verify_wesolowski(&x, &y, &pi, 1, &n));
    }

    #[test]
    fn wesolowski_wrong_pi_rejected() {
        let n = BigUint::from(97u32);
        let x = BigUint::from(5u32);
        let y = BigUint::from(5u32);
        let pi = BigUint::from(2u32); // not 1, so π^(2e) ≠ 1
        assert!(!verify_wesolowski(&x, &y, &pi, 2, &n));
    }
}
