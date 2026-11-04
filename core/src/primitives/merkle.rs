//! Merkle tree implementation for the Atoll Protocol.
//!
//! Binary Merkle tree over `Hash512` leaves using BLAKE3 as the
//! internal node hash function. Internal nodes are
//! `Hash512::blake3(left || right)` where `left` and `right` are the
//! 64-byte encodings of the child hashes. Levels with an odd number
//! of nodes are padded by duplicating the last node.

use crate::primitives::hash::Hash512;

/// Binary Merkle tree built from `Hash512` leaf hashes.
///
/// Stores every level of the tree (padded leaves at index 0, root at
/// the last index) so inclusion proofs can be derived efficiently.
pub struct MerkleTree {
    levels: Vec<Vec<Hash512>>,
    leaf_count: usize,
    root: Hash512,
}

impl MerkleTree {
    /// Builds a Merkle tree from a slice of leaf hashes.
    ///
    /// - Empty input yields a zero root.
    /// - A single leaf has root equal to that leaf.
    /// - Odd levels are right-padded by duplicating the last node.
    pub fn build(leaves: &[Hash512]) -> Self {
        let leaf_count = leaves.len();

        if leaf_count == 0 {
            return Self {
                levels: Vec::new(),
                leaf_count: 0,
                root: Hash512::zero(),
            };
        }

        let mut levels = Vec::new();
        let mut current = leaves.to_vec();

        loop {
            if current.len() == 1 {
                levels.push(current.clone());
                break;
            }

            if current.len() % 2 == 1 {
                let last = *current.last().unwrap();
                current.push(last);
            }
            levels.push(current.clone());

            let mut next = Vec::with_capacity(current.len() / 2);
            for pair in current.chunks_exact(2) {
                let mut input = [0u8; 128];
                input[..64].copy_from_slice(&pair[0].0);
                input[64..].copy_from_slice(&pair[1].0);
                next.push(Hash512::blake3(&input));
            }
            current = next;
        }

        let root = levels.last().unwrap()[0];
        Self {
            levels,
            leaf_count,
            root,
        }
    }

    /// Returns the root hash of the tree.
    pub fn root(&self) -> Hash512 {
        self.root
    }

    /// Returns the Merkle inclusion proof for the leaf at `index`.
    ///
    /// The proof is the list of sibling hashes from the leaf up to the
    /// root. Returns `None` if `index` is out of bounds.
    pub fn prove(&self, index: usize) -> Option<Vec<Hash512>> {
        if index >= self.leaf_count {
            return None;
        }

        let mut path = Vec::with_capacity(self.levels.len());
        let mut idx = index;
        for level in &self.levels {
            if level.len() == 1 {
                break;
            }
            let sibling = if idx % 2 == 0 {
                level[idx + 1]
            } else {
                level[idx - 1]
            };
            path.push(sibling);
            idx /= 2;
        }

        Some(path)
    }

    /// Verifies a Merkle inclusion proof.
    ///
    /// Re-hashes the path from `leaf` up to the root and compares the
    /// result to `root`. Returns `false` if `index` is out of bounds
    /// for the proof depth or if the recomputed root does not match.
    pub fn verify_proof(root: Hash512, leaf: Hash512, index: usize, path: &[Hash512]) -> bool {
        if path.is_empty() {
            return leaf == root;
        }

        let max_index = 1usize.checked_shl(path.len() as u32).unwrap_or(usize::MAX);
        if index >= max_index {
            return false;
        }

        let mut current = leaf;
        let mut idx = index;
        for sibling in path {
            let mut input = [0u8; 128];
            if idx % 2 == 0 {
                input[..64].copy_from_slice(&current.0);
                input[64..].copy_from_slice(&sibling.0);
            } else {
                input[..64].copy_from_slice(&sibling.0);
                input[64..].copy_from_slice(&current.0);
            }
            current = Hash512::blake3(&input);
            idx /= 2;
        }

        current == root
    }

    /// Creates an empty Merkle tree with a zero root.
    #[deprecated(note = "use MerkleTree::build instead")]
    pub fn new() -> Self {
        Self {
            levels: Vec::new(),
            leaf_count: 0,
            root: Hash512::zero(),
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    fn leaf(byte: u8) -> Hash512 {
        let mut arr = [0u8; 64];
        arr[0] = byte;
        Hash512::from_array(arr)
    }

    #[test]
    fn new_has_zero_root() {
        let tree = MerkleTree::new();
        assert_eq!(tree.root(), Hash512::zero());
    }

    #[test]
    fn empty_build_has_zero_root() {
        let tree = MerkleTree::build(&[]);
        assert_eq!(tree.root(), Hash512::zero());
    }

    #[test]
    fn single_leaf_root_is_the_leaf() {
        let leaf = leaf(1);
        let tree = MerkleTree::build(&[leaf]);
        assert_eq!(tree.root(), leaf);
    }

    #[test]
    fn build_computes_root_for_four_leaves() {
        let leaves: Vec<Hash512> = (0..4).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);

        let mut input = [0u8; 128];
        let l0l1 = Hash512::blake3(&{
            let mut i = [0u8; 128];
            i[..64].copy_from_slice(&leaves[0].0);
            i[64..].copy_from_slice(&leaves[1].0);
            i
        });
        let l2l3 = Hash512::blake3(&{
            let mut i = [0u8; 128];
            i[..64].copy_from_slice(&leaves[2].0);
            i[64..].copy_from_slice(&leaves[3].0);
            i
        });
        input[..64].copy_from_slice(&l0l1.0);
        input[64..].copy_from_slice(&l2l3.0);
        let expected = Hash512::blake3(&input);

        assert_eq!(tree.root(), expected);
    }

    #[test]
    fn odd_leaf_count_pads_last_node() {
        let leaves: Vec<Hash512> = (0..3).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);

        let l2l2 = Hash512::blake3(&{
            let mut input = [0u8; 128];
            input[..64].copy_from_slice(&leaves[2].0);
            input[64..].copy_from_slice(&leaves[2].0);
            input
        });
        let l0l1 = Hash512::blake3(&{
            let mut input = [0u8; 128];
            input[..64].copy_from_slice(&leaves[0].0);
            input[64..].copy_from_slice(&leaves[1].0);
            input
        });
        let mut input = [0u8; 128];
        input[..64].copy_from_slice(&l0l1.0);
        input[64..].copy_from_slice(&l2l2.0);
        let expected = Hash512::blake3(&input);

        assert_eq!(tree.root(), expected);
    }

    #[test]
    fn prove_and_verify_roundtrip() {
        let leaves: Vec<Hash512> = (0..8).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();

        for i in 0..leaves.len() {
            let proof = tree.prove(i).unwrap();
            assert!(
                MerkleTree::verify_proof(root, leaves[i], i, &proof),
                "proof verification failed for leaf {}",
                i
            );
        }
    }

    #[test]
    fn prove_out_of_bounds_is_none() {
        let tree = MerkleTree::build(&[leaf(1), leaf(2)]);
        assert!(tree.prove(2).is_none());
        assert!(tree.prove(usize::MAX).is_none());
    }

    #[test]
    fn verify_proof_wrong_leaf_is_false() {
        let leaves: Vec<Hash512> = (0..4).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let proof = tree.prove(0).unwrap();
        assert!(!MerkleTree::verify_proof(tree.root(), leaf(99), 0, &proof));
    }

    #[test]
    fn verify_proof_wrong_root_is_false() {
        let leaves: Vec<Hash512> = (0..4).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let proof = tree.prove(0).unwrap();
        assert!(!MerkleTree::verify_proof(Hash512::zero(), leaves[0], 0, &proof));
    }

    #[test]
    fn verify_proof_index_out_of_bounds_is_false() {
        let leaves: Vec<Hash512> = (0..2).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let proof = tree.prove(0).unwrap();
        assert!(!MerkleTree::verify_proof(tree.root(), leaves[0], 2, &proof));
    }

    #[test]
    fn verify_proof_empty_path_matches_leaf() {
        let leaf = leaf(7);
        assert!(MerkleTree::verify_proof(leaf, leaf, 0, &[]));
        assert!(!MerkleTree::verify_proof(Hash512::zero(), leaf, 0, &[]));
    }
}
