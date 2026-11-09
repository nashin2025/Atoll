//! Blockchain state management for the Atoll Protocol.
//!
//! Maintains the chain of blocks, handles block validation and insertion,
//! and tracks the current tip and finalized height.

use crate::chain::genesis::{GenesisConfig, GenesisLoader};
use crate::consensus::penta::{poh, por, pos_vrf, post, pouw, PentaConsensus, PentaValidation};
use crate::crypto::morph_hash::MorphHashEngine;
use crate::primitives::address::Address;
use crate::primitives::block::Block;
use crate::primitives::hash::Hash512;
use std::collections::HashMap;

/// Blockchain state: tracks blocks, state root, and tip
pub struct Blockchain {
    /// Blocks stored by hash
    blocks: HashMap<Hash512, Block>,
    /// Blocks stored by height
    blocks_by_height: HashMap<u64, Hash512>,
    /// Current tip hash
    tip_hash: Hash512,
    /// Current tip height
    tip_height: u64,
    /// Finalized height (blocks up to this are final)
    finalized_height: u64,
    /// State root at each height
    state_roots: HashMap<u64, Hash512>,
    /// Cached Morph-Hash engine with reused 4KB memory buffer
    morph_hash_engine: MorphHashEngine,
}

impl Blockchain {
    /// Creates a new blockchain with genesis block built from `config`.
    pub fn new(config: &GenesisConfig) -> Self {
        let block = GenesisLoader::build_genesis_block(config);
        let genesis_hash = GenesisLoader::genesis_hash(config);

        let mut blocks = HashMap::new();
        let mut blocks_by_height = HashMap::new();
        blocks.insert(genesis_hash, block.clone());
        blocks_by_height.insert(0, genesis_hash);

        Self {
            blocks,
            blocks_by_height,
            tip_hash: genesis_hash,
            tip_height: 0,
            finalized_height: 0,
            state_roots: HashMap::new(),
            morph_hash_engine: MorphHashEngine::new(),
        }
    }

    /// Returns the current tip hash
    pub fn tip_hash(&self) -> Hash512 {
        self.tip_hash
    }

    /// Returns the current tip height
    pub fn tip_height(&self) -> u64 {
        self.tip_height
    }

    /// Returns the finalized height
    pub fn finalized_height(&self) -> u64 {
        self.finalized_height
    }

    /// Returns a block by hash
    pub fn get_block(&self, hash: &Hash512) -> Option<&Block> {
        self.blocks.get(hash)
    }

    /// Returns a block by height
    pub fn get_block_by_height(&self, height: u64) -> Option<&Block> {
        self.blocks_by_height
            .get(&height)
            .and_then(|h| self.blocks.get(h))
    }

    /// Inserts a new block into the chain
    pub fn insert_block(
        &mut self,
        block: Block,
        validation: &PentaValidation,
    ) -> Result<Hash512, String> {
        // Verify the block's previous hash matches our tip
        if block.header.prev_hash != self.tip_hash {
            return Err("Block prev_hash does not match tip".to_string());
        }

        // Verify composite score meets threshold
        if !validation.is_valid() {
            return Err("Block composite score below threshold".to_string());
        }

        // Compute block hash
        let block_hash = self.morph_hash_engine.morph_hash(
            &block.serialize_bytes(),
            &block.header.prev_hash,
            block.header.height,
        );

        // Store the block
        self.blocks.insert(block_hash, block.clone());
        self.blocks_by_height
            .insert(block.header.height, block_hash);

        // Update tip
        self.tip_hash = block_hash;
        self.tip_height = block.header.height;

        // Store state root
        self.state_roots
            .insert(block.header.height, block.header.state_root);

        Ok(block_hash)
    }

    /// Finalizes blocks up to the given height
    pub fn finalize(&mut self, height: u64) {
        if height > self.finalized_height {
            self.finalized_height = height;
        }
    }

    /// Returns the state root at a given height
    pub fn state_root_at(&self, height: u64) -> Option<&Hash512> {
        self.state_roots.get(&height)
    }

    /// Returns the number of blocks in the chain
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Returns the proposer of the block at a given height
    pub fn proposer_at(&self, height: u64) -> Option<&Address> {
        self.get_block_by_height(height).map(|b| &b.header.proposer)
    }

    /// Returns a block by height along with its hash.
    ///
    /// The hash is the `Hash512` key under which the block is stored in the
    /// internal `blocks` map. This is needed by the GraphQL layer to expose
    /// the block hash on the `Block` type, since the `Block` struct itself
    /// does not store its own hash.
    pub fn get_block_and_hash_by_height(&self, height: u64) -> Option<(Hash512, &Block)> {
        self.blocks_by_height
            .get(&height)
            .and_then(|h| self.blocks.get(h).map(|b| (*h, b)))
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new(&GenesisConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::penta::{poh, por, pos_vrf, post, pouw, PentaValidation};
    use crate::primitives::block::{BlockBody, BlockHeader};

    fn valid_validation() -> PentaValidation {
        PentaValidation {
            pouw_valid: true,
            pouw_score: 0.8,
            pos_valid: true,
            pos_score: 0.9,
            post_valid: true,
            post_score: 1.0,
            por_valid: true,
            por_score: 0.72,
            poh_valid: true,
            poh_score: 1.0,
            composite_score: 0.665,
            finality_achieved: true,
            all_layers_pass: true,
        }
    }

    #[test]
    fn blockchain_initialization() {
        let bc = Blockchain::default();
        assert_eq!(bc.tip_height(), 0);
        assert_eq!(bc.block_count(), 1);
        assert_eq!(bc.finalized_height(), 0);
    }

    #[test]
    fn insert_block_updates_tip() {
        let mut bc = Blockchain::default();
        let consensus = PentaConsensus::new();

        let block = Block {
            header: BlockHeader {
                version: 1,
                height: 1,
                shard_id: 0,
                timestamp: 1000,
                prev_hash: bc.tip_hash(),
                state_root: Hash512::zero(),
                transaction_root: Hash512::zero(),
                proposer: Address::zero(),
                difficulty: 1,
            },
            body: BlockBody {
                transactions: Vec::new(),
            },
        };

        let validation = consensus.validate_block(
            &pouw::PoUWResult {
                valid: true,
                usefulness_score: 0.8,
                difficulty_met: true,
            },
            &pos_vrf::PoSResult {
                valid: true,
                combined_score: 0.9,
            },
            &post::PoSTResult {
                valid: true,
                size_score: 1.0,
            },
            &por::PoRResult {
                raw_score: 0.8,
                decayed_score: 0.72,
                is_eligible: true,
            },
            &poh::PoHResult { is_verified: true },
        );

        assert!(validation.is_ok());
        let block_hash = bc
            .insert_block(block, validation.as_ref().unwrap())
            .unwrap();
        assert_eq!(bc.tip_height(), 1);
        assert_eq!(bc.tip_hash(), block_hash);
        assert_eq!(bc.block_count(), 2);
    }

    #[test]
    fn reject_block_with_wrong_prev_hash() {
        let mut bc = Blockchain::default();
        let consensus = PentaConsensus::new();

        let block = Block {
            header: BlockHeader {
                version: 1,
                height: 1,
                shard_id: 0,
                timestamp: 1000,
                prev_hash: Hash512::from_bytes(&[1u8; 64]).unwrap(), // Wrong prev hash
                state_root: Hash512::zero(),
                transaction_root: Hash512::zero(),
                proposer: Address::zero(),
                difficulty: 1,
            },
            body: BlockBody {
                transactions: Vec::new(),
            },
        };

        let validation = consensus.validate_block(
            &pouw::PoUWResult {
                valid: true,
                usefulness_score: 0.8,
                difficulty_met: true,
            },
            &pos_vrf::PoSResult {
                valid: true,
                combined_score: 0.9,
            },
            &post::PoSTResult {
                valid: true,
                size_score: 1.0,
            },
            &por::PoRResult {
                raw_score: 0.8,
                decayed_score: 0.72,
                is_eligible: true,
            },
            &poh::PoHResult { is_verified: true },
        );

        assert!(validation.is_ok());
        let result = bc.insert_block(block, validation.as_ref().unwrap());
        assert!(result.is_err());
        assert_eq!(bc.tip_height(), 0); // Tip unchanged
    }

    #[test]
    fn finalize_blocks() {
        let mut bc = Blockchain::default();
        bc.finalize(10);
        assert_eq!(bc.finalized_height(), 10);
        bc.finalize(5); // Should not decrease
        assert_eq!(bc.finalized_height(), 10);
    }

    #[test]
    fn state_root_at() {
        let mut bc = Blockchain::default();
        let state_root = Hash512::from_bytes(&[42u8; 64]).unwrap();
        bc.state_roots.insert(0, state_root);
        assert_eq!(bc.state_root_at(0), Some(&state_root));
        assert_eq!(bc.state_root_at(1), None);
    }

    #[test]
    fn cached_engine_equivalence_with_fresh_engine() {
        let bc = Blockchain::default();

        // Replicate the genesis block from Default::default() via GenesisConfig.
        let genesis = GenesisLoader::build_genesis_block(&GenesisConfig::default());

        // Fresh engine computing the same genesis hash
        let mut fresh_engine = MorphHashEngine::new();
        let fresh_hash = fresh_engine.morph_hash(&genesis.serialize_bytes(), &Hash512::zero(), 0);

        // Cached engine (used in Blockchain::new) must match fresh engine
        assert_eq!(bc.tip_hash(), fresh_hash);
    }
}
