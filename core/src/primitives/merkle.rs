//! Merkle tree implementation for the Atoll Protocol.

pub struct MerkleTree {
    pub root: crate::primitives::hash::Hash512,
}

impl MerkleTree {
    pub fn new() -> Self {
        Self {
            root: crate::primitives::hash::Hash512::zero(),
        }
    }
}
