//! Shard manager for the Atoll Protocol.
//!
//! Manages shard lifecycle, validator assignments, shard state tracking,
//! and shard creation/deletion for horizontal scaling.

use crate::chain::sharding::ShardId;
use crate::chain::sharding::{CrossShardReceipt, ShardConfig, ShardState, ShardStats};
use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;
use std::collections::{HashMap, HashSet};

/// Shard manager: manages all shards in the Atoll network
pub struct ShardManager {
    pub shards: HashMap<ShardId, ShardState>,
    pub configs: HashMap<ShardId, ShardConfig>,
    pub validator_assignments: HashMap<ShardId, HashSet<Address>>,
    pub next_shard_id: ShardId,
    pub total_shards: u32,
    pub max_shards: u32,
}

impl ShardManager {
    /// Creates a new shard manager with default configuration
    pub fn new() -> Self {
        let mut manager = Self {
            shards: HashMap::new(),
            configs: HashMap::new(),
            validator_assignments: HashMap::new(),
            next_shard_id: 0,
            total_shards: 0,
            max_shards: 1024,
        };

        // Create genesis shard (shard 0)
        manager.create_shard(ShardConfig::default()).unwrap();

        manager
    }

    /// Creates a new shard with the given configuration
    pub fn create_shard(
        &mut self,
        config: ShardConfig,
    ) -> Result<ShardId, String> {
        if self.total_shards >= self.max_shards {
            return Err("Maximum number of shards reached".to_string());
        }

        let shard_id = config.id;
        if shard_id >= self.max_shards as u16 {
            return Err("Shard ID exceeds maximum".to_string());
        }

        if self.shards.contains_key(&shard_id) {
            return Err("Shard already exists".to_string());
        }

        let state = ShardState::new(shard_id);
        self.configs.insert(shard_id, config);
        self.shards.insert(shard_id, state);
        self.validator_assignments.insert(shard_id, HashSet::new());
        self.next_shard_id = (shard_id as u32 + 1) as ShardId;
        self.total_shards += 1;

        Ok(shard_id)
    }

    /// Removes a shard from the network
    pub fn remove_shard(
        &mut self,
        shard_id: ShardId,
    ) -> Result<(), String> {
        if !self.shards.contains_key(&shard_id) {
            return Err("Shard does not exist".to_string());
        }

        // Check if shard has validators
        if let Some(validators) = self.validator_assignments.get(&shard_id) {
            if !validators.is_empty() {
                return Err("Shard has active validators".to_string());
            }
        }

        self.shards.remove(&shard_id);
        self.configs.remove(&shard_id);
        self.validator_assignments.remove(&shard_id);
        self.total_shards -= 1;

        Ok(())
    }

    /// Assigns validators to a shard
    pub fn assign_validators(
        &mut self,
        shard_id: ShardId,
        validators: Vec<Address>,
    ) -> Result<(), String> {
        if !self.shards.contains_key(&shard_id) {
            return Err("Shard does not exist".to_string());
        }

        let config = self.configs.get(&shard_id).unwrap();
        let current_count = self
            .validator_assignments
            .get(&shard_id)
            .map(|v| v.len() as u32)
            .unwrap_or(0);

        if current_count + validators.len() as u32 > config.max_validators {
            return Err("Exceeds maximum validator count for shard".to_string());
        }

        let shard_validators = self.validator_assignments.get_mut(&shard_id).unwrap();
        for v in validators {
            shard_validators.insert(v);
        }

        // Update shard state validator count
        if let Some(state) = self.shards.get_mut(&shard_id) {
            state.validator_count = shard_validators.len() as u32;
        }

        Ok(())
    }

    /// Removes validators from a shard
    pub fn remove_validators(
        &mut self,
        shard_id: ShardId,
        validators: &[Address],
    ) -> Result<(), String> {
        if !self.shards.contains_key(&shard_id) {
            return Err("Shard does not exist".to_string());
        }

        let shard_validators = self.validator_assignments.get_mut(&shard_id).unwrap();
        for v in validators {
            shard_validators.remove(v);
        }

        // Update shard state validator count
        if let Some(state) = self.shards.get_mut(&shard_id) {
            state.validator_count = shard_validators.len() as u32;
        }

        Ok(())
    }

    /// Updates shard state after block processing
    pub fn update_shard_state(
        &mut self,
        shard_id: ShardId,
        block_height: u64,
        state_root: Hash512,
        load_metric: f64,
        transaction_count: u64,
        gas_used: u64,
        gas_limit: u64,
    ) -> Result<(), String> {
        if let Some(state) = self.shards.get_mut(&shard_id) {
            state.block_height = block_height;
            state.state_root = state_root;
            state.load_metric = load_metric;
            state.transaction_count = transaction_count;
            state.gas_used = gas_used;
            state.gas_limit = gas_limit;
            Ok(())
        } else {
            Err("Shard does not exist".to_string())
        }
    }

    /// Returns shard state if it exists
    pub fn get_shard_state(
        &self,
        shard_id: ShardId,
    ) -> Option<&ShardState> {
        self.shards.get(&shard_id)
    }

    /// Returns shard configuration if it exists
    pub fn get_shard_config(
        &self,
        shard_id: ShardId,
    ) -> Option<&ShardConfig> {
        self.configs.get(&shard_id)
    }

    /// Returns validators assigned to a shard
    pub fn get_shard_validators(
        &self,
        shard_id: ShardId,
    ) -> Option<&HashSet<Address>> {
        self.validator_assignments.get(&shard_id)
    }

    /// Returns all active shard IDs
    pub fn active_shard_ids(&self) -> Vec<ShardId> {
        self.shards
            .iter()
            .filter(|(_, s)| s.is_active)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns all shard IDs
    pub fn all_shard_ids(&self) -> Vec<ShardId> {
        self.shards.keys().copied().collect()
    }

    /// Returns total number of shards
    pub fn shard_count(&self) -> u32 {
        self.total_shards
    }

    /// Returns maximum number of shards
    pub fn max_shards(&self) -> u32 {
        self.max_shards
    }

    /// Returns shard statistics for monitoring
    pub fn get_shard_stats(
        &self,
        shard_id: ShardId,
    ) -> Option<ShardStats> {
        let state = self.shards.get(&shard_id)?;
        let validators = self
            .validator_assignments
            .get(&shard_id)
            .map(|v| v.len() as u32)
            .unwrap_or(0);

        Some(ShardStats {
            id: shard_id,
            tps: state.transaction_count as f64 / state.block_height.max(1) as f64,
            avg_gas_used: if state.transaction_count > 0 {
                state.gas_used / state.transaction_count
            } else {
                0
            },
            validator_count: validators,
            load_metric: state.load_metric,
            cross_shard_tx_count: 0, // Tracked separately
            last_updated: state.block_height,
        })
    }

    /// Returns statistics for all shards
    pub fn get_all_shard_stats(&self) -> Vec<ShardStats> {
        self.active_shard_ids()
            .iter()
            .filter_map(|id| self.get_shard_stats(*id))
            .collect()
    }

    /// Returns the average load across all shards
    pub fn average_load(&self) -> f64 {
        if self.shards.is_empty() {
            return 0.0;
        }

        let total_load: f64 = self.shards.values().map(|s| s.load_metric).sum();
        total_load / self.shards.len() as f64
    }

    /// Returns overloaded shards (load > max_load)
    pub fn overloaded_shards(&self) -> Vec<ShardId> {
        let avg_load = self.average_load();
        self.shards
            .iter()
            .filter(|(_, s)| {
                s.load_metric
                    > avg_load * self.configs.get(&s.id).map(|c| c.max_load).unwrap_or(1.5)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns underloaded shards (load < min_load)
    pub fn underloaded_shards(&self) -> Vec<ShardId> {
        let avg_load = self.average_load();
        self.shards
            .iter()
            .filter(|(_, s)| {
                s.load_metric
                    < avg_load * self.configs.get(&s.id).map(|c| c.min_load).unwrap_or(0.3)
            })
            .map(|(id, _)| *id)
            .collect()
    }
}

impl Default for ShardManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shard_manager_creation() {
        let manager = ShardManager::new();
        assert_eq!(manager.shard_count(), 1);
        assert_eq!(manager.total_shards, 1);
        assert!(manager.shards.contains_key(&0));
    }

    #[test]
    fn create_shard() {
        let mut manager = ShardManager::new();
        let config = ShardConfig {
            id: 1,
            min_validators: 50,
            max_validators: 500,
            max_load: 1.5,
            min_load: 0.3,
        };
        let result = manager.create_shard(config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
        assert_eq!(manager.shard_count(), 2);
    }

    #[test]
    fn create_duplicate_shard_fails() {
        let mut manager = ShardManager::new();
        let config = ShardConfig {
            id: 0,
            ..Default::default()
        };
        let result = manager.create_shard(config);
        assert!(result.is_err());
    }

    #[test]
    fn assign_and_remove_validators() {
        let mut manager = ShardManager::new();
        let addr1 = Address::from_bytes([1u8; 20]);
        let addr2 = Address::from_bytes([2u8; 20]);

        manager
            .assign_validators(0, vec![addr1.clone(), addr2.clone()])
            .unwrap();
        let validators = manager.get_shard_validators(0).unwrap();
        assert!(validators.contains(&addr1));
        assert!(validators.contains(&addr2));

        manager.remove_validators(0, &[addr1]).unwrap();
        let validators = manager.get_shard_validators(0).unwrap();
        assert!(!validators.contains(&addr1));
        assert!(validators.contains(&addr2));
    }

    #[test]
    fn update_shard_state() {
        let mut manager = ShardManager::new();
        let result = manager.update_shard_state(
            0,
            100,
            Hash512::from_bytes(&[1u8; 64]).unwrap(),
            0.8,
            500,
            1_000_000,
            30_000_000,
        );
        assert!(result.is_ok());

        let state = manager.get_shard_state(0).unwrap();
        assert_eq!(state.block_height, 100);
        assert_eq!(state.load_metric, 0.8);
        assert_eq!(state.transaction_count, 500);
    }

    #[test]
    fn overloaded_and_underloaded_shards() {
        let mut manager = ShardManager::new();

        // Create additional shards with different loads
        manager
            .create_shard(ShardConfig {
                id: 1,
                ..Default::default()
            })
            .unwrap();
        manager
            .create_shard(ShardConfig {
                id: 2,
                ..Default::default()
            })
            .unwrap();

        // Set different loads
        manager
            .update_shard_state(0, 100, Hash512::zero(), 2.0, 1000, 100, 100)
            .unwrap();
        manager
            .update_shard_state(1, 100, Hash512::zero(), 0.1, 10, 10, 100)
            .unwrap();
        manager
            .update_shard_state(2, 100, Hash512::zero(), 1.0, 100, 100, 100)
            .unwrap();

        let overloaded = manager.overloaded_shards();
        assert!(!overloaded.is_empty());
        assert!(overloaded.contains(&0)); // Load 2.0 > avg * 1.5

        let underloaded = manager.underloaded_shards();
        assert!(!underloaded.is_empty());
        assert!(underloaded.contains(&1)); // Load 0.1 < avg * 0.3
    }

    #[test]
    fn remove_shard() {
        let mut manager = ShardManager::new();
        manager
            .create_shard(ShardConfig {
                id: 1,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(manager.shard_count(), 2);
        assert!(manager.remove_shard(1).is_ok());
        assert_eq!(manager.shard_count(), 1);
        assert!(!manager.shards.contains_key(&1));
    }

    #[test]
    fn max_shards_limit() {
        let mut manager = ShardManager::new();
        manager.max_shards = 2;
        // Already have shard 0, so creating shard 1 should succeed
        let result = manager.create_shard(ShardConfig {
            id: 1,
            ..Default::default()
        });
        assert!(result.is_ok());
        // Creating shard 2 should fail since max_shards = 2
        let result = manager.create_shard(ShardConfig {
            id: 2,
            ..Default::default()
        });
        assert!(result.is_err());
    }
}
