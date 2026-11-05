//! Shard rebalancer for the Atoll Protocol.
//!
//! Implements dynamic shard split and merge based on load metrics.
//! Shards are split when overloaded and merged when underloaded.

use crate::chain::sharding::cross_shard::CrossShardManager;
use crate::chain::sharding::shard_manager::ShardManager;
use crate::chain::sharding::{CrossShardReceipt, ShardConfig, ShardState, ShardStats};
use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;

/// Shard rebalancer: manages dynamic shard split and merge
pub struct ShardRebalancer {
    pub shard_manager: ShardManager,
    pub cross_shard_manager: CrossShardManager,
    pub rebalance_interval: u64, // Blocks between rebalance checks
    pub last_rebalance_block: u64,
    pub split_count: u64,
    pub merge_count: u64,
}

impl ShardRebalancer {
    /// Creates a new shard rebalancer
    pub fn new() -> Self {
        Self {
            shard_manager: ShardManager::new(),
            cross_shard_manager: CrossShardManager::new(),
            rebalance_interval: 1000, // Check every 1000 blocks
            last_rebalance_block: 0,
            split_count: 0,
            merge_count: 0,
        }
    }

    /// Updates shard load metrics and checks for rebalancing
    pub fn update_shard_load(
        &mut self,
        shard_id: u16,
        block_height: u64,
        load_metric: f64,
        transaction_count: u64,
        gas_used: u64,
        gas_limit: u64,
        state_root: Hash512,
    ) -> Result<(), String> {
        self.shard_manager.update_shard_state(
            shard_id,
            block_height,
            state_root,
            load_metric,
            transaction_count,
            gas_used,
            gas_limit,
        )?;

        // Check if it's time to rebalance
        if block_height >= self.last_rebalance_block + self.rebalance_interval {
            self.rebalance(block_height)?;
            self.last_rebalance_block = block_height;
        }

        Ok(())
    }

    /// Performs rebalancing: splits overloaded shards and merges underloaded ones
    pub fn rebalance(&mut self, current_block: u64) -> Result<(), String> {
        let _avg_load = self.shard_manager.average_load();

        // First, handle overloaded shards (split)
        let overloaded = self.shard_manager.overloaded_shards();
        for shard_id in overloaded {
            self.split_shard(shard_id, current_block)?;
        }

        // Then, handle underloaded shards (merge)
        let underloaded = self.shard_manager.underloaded_shards();
        for shard_id in underloaded {
            self.merge_shard(shard_id, current_block)?;
        }

        Ok(())
    }

    /// Splits an overloaded shard into two
    pub fn split_shard(&mut self, shard_id: u16, current_block: u64) -> Result<(), String> {
        let state = self
            .shard_manager
            .get_shard_state(shard_id)
            .ok_or_else(|| "Shard does not exist".to_string())?;

        let config = self
            .shard_manager
            .get_shard_config(shard_id)
            .ok_or_else(|| "Shard config not found".to_string())?;

        // Only split if validator count supports it
        if state.validator_count < config.min_validators * 2 {
            return Ok(()); // Not enough validators to split
        }

        // Create new shard ID
        let new_shard_id = self.shard_manager.next_shard_id;
        if new_shard_id >= self.shard_manager.max_shards as u16 {
            return Err("Maximum shards reached, cannot split".to_string());
        }

        // Split validators roughly in half
        let validators = self
            .shard_manager
            .get_shard_validators(shard_id)
            .ok_or_else(|| "Shard validators not found".to_string())?;
        let validator_list: Vec<Address> = validators.iter().cloned().collect();
        let half = validator_list.len() / 2;

        let _keep_validators: Vec<Address> = validator_list[..half].to_vec();
        let move_validators: Vec<Address> = validator_list[half..].to_vec();

        // Create new shard
        let new_config = ShardConfig {
            id: new_shard_id,
            min_validators: config.min_validators,
            max_validators: config.max_validators,
            max_load: config.max_load,
            min_load: config.min_load,
        };

        self.shard_manager.create_shard(new_config)?;
        self.shard_manager
            .assign_validators(new_shard_id, move_validators.clone())?;

        // Keep remaining validators on original shard
        self.shard_manager
            .remove_validators(shard_id, &move_validators)?;

        // Split state
        let original_state = self
            .shard_manager
            .get_shard_state(shard_id)
            .unwrap()
            .clone();
        self.shard_manager.update_shard_state(
            shard_id,
            current_block,
            original_state.state_root,
            original_state.load_metric / 2.0, // Reduce load
            original_state.transaction_count / 2,
            original_state.gas_used / 2,
            original_state.gas_limit,
        )?;

        // Initialize new shard state
        self.shard_manager.update_shard_state(
            new_shard_id,
            current_block,
            original_state.state_root,
            original_state.load_metric / 2.0,
            original_state.transaction_count / 2,
            original_state.gas_used / 2,
            original_state.gas_limit,
        )?;

        self.split_count += 1;

        Ok(())
    }

    /// Merges an underloaded shard with another shard
    pub fn merge_shard(&mut self, shard_id: u16, current_block: u64) -> Result<(), String> {
        // Find another shard to merge with (preferably nearby ID)
        let target_shard_id = if shard_id > 0 {
            shard_id - 1
        } else {
            shard_id + 1
        };

        // Check if target shard exists
        if !self.shard_manager.shards.contains_key(&target_shard_id) {
            return Ok(()); // No target to merge with
        }

        // Move all validators from source to target
        let validators = self
            .shard_manager
            .get_shard_validators(shard_id)
            .ok_or_else(|| "Shard validators not found".to_string())?;
        let validator_list: Vec<Address> = validators.iter().cloned().collect();

        if !validator_list.is_empty() {
            self.shard_manager
                .assign_validators(target_shard_id, validator_list.clone())?;
            self.shard_manager
                .remove_validators(shard_id, &validator_list)?;
        }

        // Merge state
        let source_state = self
            .shard_manager
            .get_shard_state(shard_id)
            .unwrap()
            .clone();
        let target_state = self
            .shard_manager
            .get_shard_state(target_shard_id)
            .unwrap()
            .clone();

        self.shard_manager.update_shard_state(
            target_shard_id,
            current_block,
            target_state.state_root,
            (target_state.load_metric + source_state.load_metric) / 2.0,
            target_state.transaction_count + source_state.transaction_count,
            target_state.gas_used + source_state.gas_used,
            target_state.gas_limit,
        )?;

        // Remove source shard
        self.shard_manager.remove_shard(shard_id)?;

        self.merge_count += 1;

        Ok(())
    }

    /// Returns the rebalance interval in blocks
    pub fn rebalance_interval(&self) -> u64 {
        self.rebalance_interval
    }

    /// Sets the rebalance interval
    pub fn set_rebalance_interval(&mut self, blocks: u64) {
        self.rebalance_interval = blocks;
    }

    /// Returns total split count
    pub fn split_count(&self) -> u64 {
        self.split_count
    }

    /// Returns total merge count
    pub fn merge_count(&self) -> u64 {
        self.merge_count
    }

    /// Returns the shard manager
    pub fn shard_manager(&self) -> &ShardManager {
        &self.shard_manager
    }

    /// Returns mutable reference to shard manager
    pub fn shard_manager_mut(&mut self) -> &mut ShardManager {
        &mut self.shard_manager
    }

    /// Returns the cross-shard manager
    pub fn cross_shard_manager(&self) -> &CrossShardManager {
        &self.cross_shard_manager
    }

    /// Returns mutable reference to cross-shard manager
    pub fn cross_shard_manager_mut(&mut self) -> &mut CrossShardManager {
        &mut self.cross_shard_manager
    }
}

impl Default for ShardRebalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebalancer_creation() {
        let rebalancer = ShardRebalancer::new();
        assert_eq!(rebalancer.rebalance_interval(), 1000);
        assert_eq!(rebalancer.split_count(), 0);
        assert_eq!(rebalancer.merge_count(), 0);
        assert_eq!(rebalancer.shard_manager().shard_count(), 1);
    }

    #[test]
    fn update_shard_load_triggers_rebalance() {
        let mut rebalancer = ShardRebalancer::new();
        rebalancer.set_rebalance_interval(100);

        // Update with high load to trigger split
        let result = rebalancer.update_shard_load(
            0,
            100,
            2.0, // High load
            1000,
            100,
            100,
            Hash512::zero(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn split_shard() {
        let mut rebalancer = ShardRebalancer::new();

        // Assign many validators to shard 0
        let validators: Vec<Address> = (0..200)
            .map(|i| Address::from_bytes([i as u8; 20]))
            .collect();
        rebalancer
            .shard_manager_mut()
            .assign_validators(0, validators)
            .unwrap();

        // Set high load
        rebalancer
            .shard_manager_mut()
            .update_shard_state(0, 100, Hash512::zero(), 2.0, 1000, 100, 100)
            .unwrap();

        // Split should succeed
        let result = rebalancer.split_shard(0, 100);
        assert!(result.is_ok());
        assert_eq!(rebalancer.split_count(), 1);
        assert_eq!(rebalancer.shard_manager().shard_count(), 2);
    }

    #[test]
    fn merge_shard() {
        let mut rebalancer = ShardRebalancer::new();

        // Create a second shard
        rebalancer
            .shard_manager_mut()
            .create_shard(crate::chain::sharding::ShardConfig {
                id: 1,
                ..Default::default()
            })
            .unwrap();

        // Assign validators to shard 1
        let validators: Vec<Address> = (0..50)
            .map(|i| Address::from_bytes([10u8 + i as u8; 20]))
            .collect();
        rebalancer
            .shard_manager_mut()
            .assign_validators(1, validators)
            .unwrap();

        // Set low load on shard 1
        rebalancer
            .shard_manager_mut()
            .update_shard_state(1, 100, Hash512::zero(), 0.1, 10, 10, 100)
            .unwrap();

        // Merge shard 1 into shard 0
        let result = rebalancer.merge_shard(1, 100);
        assert!(result.is_ok());
        assert_eq!(rebalancer.merge_count(), 1);
        assert_eq!(rebalancer.shard_manager().shard_count(), 1);
    }

    #[test]
    fn rebalance_triggers_on_interval() {
        let mut rebalancer = ShardRebalancer::new();
        rebalancer.set_rebalance_interval(100);

        // First update at block 100 should trigger rebalance
        let result = rebalancer.update_shard_load(0, 100, 1.0, 100, 100, 100, Hash512::zero());
        assert!(result.is_ok());
        assert_eq!(rebalancer.last_rebalance_block, 100);

        // Update at block 150 should NOT trigger rebalance (interval not reached)
        let result = rebalancer.update_shard_load(0, 150, 1.0, 100, 100, 100, Hash512::zero());
        assert!(result.is_ok());
        assert_eq!(rebalancer.last_rebalance_block, 100);

        // Update at block 200 should trigger rebalance
        let result = rebalancer.update_shard_load(0, 200, 1.0, 100, 100, 100, Hash512::zero());
        assert!(result.is_ok());
        assert_eq!(rebalancer.last_rebalance_block, 200);
    }

    #[test]
    fn cross_shard_integration() {
        let mut rebalancer = ShardRebalancer::new();

        // Create a cross-shard request
        let request_id = rebalancer.cross_shard_manager_mut().create_request(
            0,
            1,
            Address::from_bytes([1u8; 20]),
            Address::from_bytes([2u8; 20]),
            50_000_000,
            vec![],
            100,
        );

        assert_eq!(rebalancer.cross_shard_manager().total_cross_shard_tx(), 1);

        // Convert to receipt and execute
        let receipt_id = rebalancer
            .cross_shard_manager_mut()
            .request_to_receipt(&request_id)
            .unwrap()
            .id;

        rebalancer
            .cross_shard_manager_mut()
            .execute_receipt(&receipt_id, 150)
            .unwrap();

        assert!(rebalancer
            .cross_shard_manager()
            .is_receipt_executed(&receipt_id));
    }
}
