//! GHOSTDAG fork choice rule for the Atoll Protocol.
//!
//! Placeholder - full implementation after Phase 0 research.

use crate::primitives::hash::Hash512;
use std::collections::HashMap;

/// Fork choice rule state placeholder
#[derive(Debug, Clone, Default)]
pub struct ForkChoice {
    pub blocks: HashMap<Hash512, u64>, // block_hash -> height
    pub tip: Option<Hash512>,
}

impl ForkChoice {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            tip: None,
        }
    }

    pub fn insert_block(&mut self, hash: Hash512, height: u64) {
        self.blocks.insert(hash, height);
        if self.tip.is_none() || height > self.blocks.get(&self.tip.unwrap()).copied().unwrap_or(0)
        {
            self.tip = Some(hash);
        }
    }

    pub fn tip(&self) -> Option<&Hash512> {
        self.tip.as_ref()
    }

    pub fn block_height(&self, hash: &Hash512) -> Option<u64> {
        self.blocks.get(hash).copied()
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fork_choice_new() {
        let fc = ForkChoice::new();
        assert!(fc.tip().is_none());
        assert_eq!(fc.block_count(), 0);
    }

    #[test]
    fn fork_choice_insert_block_updates_tip() {
        let mut fc = ForkChoice::new();
        let h1 = Hash512::from_bytes(&[1u8; 64]);
        let h2 = Hash512::from_bytes(&[2u8; 64]);

        fc.insert_block(h1, 10);
        assert_eq!(fc.tip(), Some(&h1));
        assert_eq!(fc.block_height(&h1), Some(10));
        assert_eq!(fc.block_count(), 1);

        fc.insert_block(h2, 5);
        assert_eq!(fc.tip(), Some(&h1)); // h1 still has higher height

        fc.insert_block(h2, 20);
        assert_eq!(fc.tip(), Some(&h2)); // h2 now has higher height
    }

    #[test]
    fn fork_choice_block_not_found() {
        let fc = ForkChoice::new();
        assert_eq!(fc.block_height(&Hash512::zero()), None);
    }
}
