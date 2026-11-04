//! Sharding module for the Atoll Protocol.
//!
//! Implements horizontal scaling through shard management, cross-shard
//! communication, and dynamic shard rebalancing.
//!
//! Phase 4: Production implementation with 64 initial shards, receipt-based
//! cross-shard messaging, and load-based dynamic rebalancing.

pub mod cross_shard;
pub mod rebalancer;
pub mod shard_manager;

use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Shard identifier
pub type ShardId = u16;

/// Shard configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShardConfig {
    pub id: ShardId,
    pub min_validators: u32,
    pub max_validators: u32,
    pub max_load: f64,
    pub min_load: f64,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            id: 0,
            min_validators: 100,
            max_validators: 1000,
            max_load: 1.5, // 150% of average triggers split
            min_load: 0.3, // 30% of average triggers merge
        }
    }
}

/// Shard state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardState {
    pub id: ShardId,
    pub block_height: u64,
    pub state_root: Hash512,
    pub validator_count: u32,
    pub load_metric: f64,
    pub transaction_count: u64,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub is_active: bool,
    pub last_crosslink: u64,
}

impl ShardState {
    pub fn new(id: ShardId) -> Self {
        Self {
            id,
            block_height: 0,
            state_root: Hash512::zero(),
            validator_count: 0,
            load_metric: 0.0,
            transaction_count: 0,
            gas_used: 0,
            gas_limit: 30_000_000,
            is_active: true,
            last_crosslink: 0,
        }
    }
}

/// Cross-shard message receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossShardReceipt {
    pub id: Hash512,
    pub source_shard: ShardId,
    pub destination_shard: ShardId,
    pub sender: Address,
    pub recipient: Address,
    pub amount: u128,
    pub data: Vec<u8>,
    pub executed: bool,
    pub executed_at: Option<u64>,
    pub block_height: u64,
}

/// Cross-shard message request (before execution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossShardRequest {
    pub id: Hash512,
    pub source_shard: ShardId,
    pub destination_shard: ShardId,
    pub sender: Address,
    pub recipient: Address,
    pub amount: u128,
    pub data: Vec<u8>,
    pub block_height: u64,
}

/// Shard statistics for monitoring
#[derive(Debug, Clone)]
pub struct ShardStats {
    pub id: ShardId,
    pub tps: f64,
    pub avg_gas_used: u64,
    pub validator_count: u32,
    pub load_metric: f64,
    pub cross_shard_tx_count: u64,
    pub last_updated: u64,
}

impl Default for ShardStats {
    fn default() -> Self {
        Self {
            id: 0,
            tps: 0.0,
            avg_gas_used: 0,
            validator_count: 0,
            load_metric: 0.0,
            cross_shard_tx_count: 0,
            last_updated: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shard_config_default() {
        let config = ShardConfig::default();
        assert_eq!(config.min_validators, 100);
        assert_eq!(config.max_validators, 1000);
        assert_eq!(config.max_load, 1.5);
        assert_eq!(config.min_load, 0.3);
    }

    #[test]
    fn shard_state_new() {
        let state = ShardState::new(42);
        assert_eq!(state.id, 42);
        assert_eq!(state.block_height, 0);
        assert_eq!(state.state_root, Hash512::zero());
        assert!(state.is_active);
    }

    #[test]
    fn cross_shard_receipt_creation() {
        let receipt = CrossShardReceipt {
            id: Hash512::zero(),
            source_shard: 0,
            destination_shard: 1,
            sender: Address::zero(),
            recipient: Address::from_bytes([1u8; 20]),
            amount: 100_000_000,
            data: vec![1, 2, 3],
            executed: false,
            executed_at: None,
            block_height: 1000,
        };
        assert_eq!(receipt.source_shard, 0);
        assert_eq!(receipt.destination_shard, 1);
        assert_eq!(receipt.amount, 100_000_000);
        assert!(!receipt.executed);
    }

    #[test]
    fn cross_shard_request_creation() {
        let request = CrossShardRequest {
            id: Hash512::from_bytes(&[7u8; 64]),
            source_shard: 5,
            destination_shard: 10,
            sender: Address::from_bytes([2u8; 20]),
            recipient: Address::from_bytes([3u8; 20]),
            amount: 50_000_000,
            data: vec![0xde, 0xad, 0xbe, 0xef],
            block_height: 5000,
        };
        assert_eq!(request.source_shard, 5);
        assert_eq!(request.destination_shard, 10);
        assert_eq!(request.amount, 50_000_000);
    }

    #[test]
    fn shard_stats_default() {
        let stats = ShardStats::default();
        assert_eq!(stats.id, 0);
        assert_eq!(stats.tps, 0.0);
        assert_eq!(stats.validator_count, 0);
    }
}
