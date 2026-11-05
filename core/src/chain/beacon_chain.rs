//! Beacon chain for shard coordination in the Atoll Protocol.
//!
//! Coordinates shards, manages validator assignments, and provides
//! cross-links between shards and the beacon chain.

use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;
use std::collections::{HashMap, HashSet};

/// Beacon block: coordinates shards and finalizes checkpoints
#[derive(Debug, Clone)]
pub struct BeaconBlock {
    pub height: u64,
    pub timestamp: u64,
    pub shard_block_roots: HashMap<u16, Hash512>, // shard_id -> block root
    pub validator_set: Vec<Address>,
    pub randao_reveal: Hash512, // Randomness for VRF
    pub finalized_checkpoint: Option<Hash512>,
    pub parent_hash: Hash512,
}

/// Beacon chain: manages shards and validator registry
pub struct BeaconChain {
    pub blocks: Vec<BeaconBlock>,
    pub validator_registry: ValidatorRegistry,
    pub shard_assignments: HashMap<u16, HashSet<Address>>, // shard_id -> validators
    pub crosslinks: Vec<Crosslink>,
    pub finalized_checkpoint: Hash512,
    pub current_height: u64,
}

/// Crosslink: links a shard block to the beacon chain
#[derive(Debug, Clone)]
pub struct Crosslink {
    pub shard_id: u16,
    pub shard_block_root: Hash512,
    pub beacon_block_height: u64,
    pub slot: u64,
}

/// Validator registry for the beacon chain
pub struct ValidatorRegistry {
    pub validators: HashMap<Address, ValidatorInfo>,
    pub total_stake: u128,
    pub active_validators: HashSet<Address>,
}

/// Validator information
#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    pub address: Address,
    pub stake: u128,
    pub activation_epoch: u64,
    pub exit_epoch: Option<u64>,
    pub slashed: bool,
    pub exit_initiated: bool,
}

impl BeaconChain {
    /// Creates a new beacon chain with genesis block
    pub fn new() -> Self {
        let genesis = BeaconBlock {
            height: 0,
            timestamp: 0,
            shard_block_roots: HashMap::new(),
            validator_set: Vec::new(),
            randao_reveal: Hash512::zero(),
            finalized_checkpoint: None,
            parent_hash: Hash512::zero(),
        };

        Self {
            blocks: vec![genesis],
            validator_registry: ValidatorRegistry::new(),
            shard_assignments: HashMap::new(),
            crosslinks: Vec::new(),
            finalized_checkpoint: Hash512::zero(),
            current_height: 0,
        }
    }

    /// Adds a new beacon block
    pub fn add_block(&mut self, block: BeaconBlock) {
        self.current_height = block.height;
        self.finalized_checkpoint = block
            .finalized_checkpoint
            .unwrap_or(self.finalized_checkpoint);
        self.blocks.push(block);
    }

    /// Registers a validator
    pub fn register_validator(&mut self, address: Address, stake: u128, activation_epoch: u64) {
        self.validator_registry.validators.insert(
            address.clone(),
            ValidatorInfo {
                address,
                stake,
                activation_epoch,
                exit_epoch: None,
                slashed: false,
                exit_initiated: false,
            },
        );
        self.validator_registry.total_stake += stake;
        self.validator_registry.active_validators.insert(address);
    }

    /// Exits a validator
    pub fn exit_validator(&mut self, address: &Address, exit_epoch: u64) {
        if let Some(info) = self.validator_registry.validators.get_mut(address) {
            info.exit_epoch = Some(exit_epoch);
            info.exit_initiated = true;
        }
        self.validator_registry.active_validators.remove(address);
    }

    /// Slashes a validator
    pub fn slash_validator(&mut self, address: &Address, penalty_percent: u8) {
        if let Some(info) = self.validator_registry.validators.get_mut(address) {
            info.slashed = true;
            // Reduce stake by penalty
            let penalty = (info.stake as f64 * penalty_percent as f64 / 100.0) as u128;
            info.stake = info.stake.saturating_sub(penalty);
            self.validator_registry.total_stake =
                self.validator_registry.total_stake.saturating_sub(penalty);
        }
    }

    /// Assigns validators to shards
    pub fn assign_validators_to_shard(&mut self, shard_id: u16, validators: Vec<Address>) {
        let shard_validators = self
            .shard_assignments
            .entry(shard_id)
            .or_insert_with(HashSet::new);
        for v in validators {
            shard_validators.insert(v);
        }
    }

    /// Returns validators for a shard
    pub fn shard_validators(&self, shard_id: u16) -> Option<&HashSet<Address>> {
        self.shard_assignments.get(&shard_id)
    }

    /// Creates a crosslink for a shard block
    pub fn create_crosslink(&mut self, shard_id: u16, shard_block_root: Hash512, slot: u64) {
        self.crosslinks.push(Crosslink {
            shard_id,
            shard_block_root,
            beacon_block_height: self.current_height,
            slot,
        });
    }

    /// Returns the latest beacon block
    pub fn latest_block(&self) -> &BeaconBlock {
        self.blocks.last().unwrap()
    }

    /// Returns total validator count
    pub fn validator_count(&self) -> usize {
        self.validator_registry.active_validators.len()
    }
}

impl Default for BeaconChain {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            total_stake: 0,
            active_validators: HashSet::new(),
        }
    }

    /// Returns validator info if exists
    pub fn get_validator(&self, address: &Address) -> Option<&ValidatorInfo> {
        self.validators.get(address)
    }

    /// Checks if a validator is active
    pub fn is_active(&self, address: &Address) -> bool {
        self.active_validators.contains(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beacon_chain_initialization() {
        let bc = BeaconChain::new();
        assert_eq!(bc.current_height, 0);
        assert_eq!(bc.validator_count(), 0);
        assert_eq!(bc.blocks.len(), 1); // Genesis
    }

    #[test]
    fn add_beacon_block() {
        let mut bc = BeaconChain::new();
        let block = BeaconBlock {
            height: 1,
            timestamp: 1000,
            shard_block_roots: HashMap::new(),
            validator_set: vec![Address::zero()],
            randao_reveal: Hash512::zero(),
            finalized_checkpoint: None,
            parent_hash: Hash512::zero(),
        };
        bc.add_block(block);
        assert_eq!(bc.current_height, 1);
        assert_eq!(bc.blocks.len(), 2);
    }

    #[test]
    fn register_and_exit_validator() {
        let mut bc = BeaconChain::new();
        let addr = Address::from_bytes([1u8; 20]);

        bc.register_validator(addr.clone(), 10_000, 0);
        assert_eq!(bc.validator_count(), 1);
        assert!(bc.validator_registry.is_active(&addr));

        bc.exit_validator(&addr, 100);
        assert!(!bc.validator_registry.is_active(&addr));
    }

    #[test]
    fn slash_validator() {
        let mut bc = BeaconChain::new();
        let addr = Address::from_bytes([1u8; 20]);

        bc.register_validator(addr.clone(), 10_000, 0);
        bc.slash_validator(&addr, 33);
        let info = bc.validator_registry.get_validator(&addr).unwrap();
        assert!(info.slashed);
        assert_eq!(info.stake, 10_000 - 3_300);
    }

    #[test]
    fn shard_assignment() {
        let mut bc = BeaconChain::new();
        let addr1 = Address::from_bytes([1u8; 20]);
        let addr2 = Address::from_bytes([2u8; 20]);

        bc.assign_validators_to_shard(0, vec![addr1.clone(), addr2.clone()]);
        let validators = bc.shard_validators(0).unwrap();
        assert!(validators.contains(&addr1));
        assert!(validators.contains(&addr2));
    }

    #[test]
    fn crosslink_creation() {
        let mut bc = BeaconChain::new();
        let root = Hash512::from_bytes(&[42u8; 64]).unwrap();

        bc.create_crosslink(0, root, 5);
        assert_eq!(bc.crosslinks.len(), 1);
        assert_eq!(bc.crosslinks[0].shard_id, 0);
        assert_eq!(bc.crosslinks[0].shard_block_root, root);
        assert_eq!(bc.crosslinks[0].slot, 5);
    }

    #[test]
    fn validator_stake_tracking() {
        let mut bc = BeaconChain::new();
        let addr = Address::from_bytes([1u8; 20]);

        bc.register_validator(addr.clone(), 5_000, 0);
        assert_eq!(bc.validator_registry.total_stake, 5_000);

        bc.register_validator(addr.clone(), 3_000, 0); // Same address, adds stake
        assert_eq!(bc.validator_registry.total_stake, 8_000);
    }
}
