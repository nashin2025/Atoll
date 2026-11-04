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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_tree_new_has_zero_root() {
        let tree = MerkleTree::new();
        assert_eq!(tree.root, crate::primitives::hash::Hash512::zero());
    }

    #[test]
    fn merkle_tree_root_is_512_bits() {
        let tree = MerkleTree::new();
        assert_eq!(tree.root.to_array().len(), 64);
    }
}
