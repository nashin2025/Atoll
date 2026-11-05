//! Genesis block finalization for the Atoll Protocol.
//!
//! Defines [`GenesisConfig`] (chain id, genesis timestamp, initial difficulty,
//! initial token distribution) and [`GenesisLoader`], which loads a config from
//! a JSON file or generates a default, builds the genesis block, and validates
//! its hash.

use crate::chain::state::ChainState;
use crate::crypto::morph_hash::MorphHashEngine;
use crate::primitives::address::Address;
use crate::primitives::block::{Block, BlockBody, BlockHeader};
use crate::primitives::hash::Hash512;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Serde helpers
// ---------------------------------------------------------------------------

/// Serializes `HashMap<Address, u128>` using hex-encoded address strings as
/// JSON object keys, since `Address` itself serializes as a byte array which
/// is not a valid JSON key.
mod address_map {
    use super::Address;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(map: &HashMap<Address, u128>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: HashMap<String, u128> = map
            .iter()
            .map(|(addr, amt)| (addr.to_hex(), *amt))
            .collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<Address, u128>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: HashMap<String, u128> = HashMap::deserialize(deserializer)?;
        let mut map = HashMap::with_capacity(string_map.len());
        for (hex, amt) in &string_map {
            let addr = Address::from_hex(hex).map_err(|e| de::Error::custom(e.to_string()))?;
            map.insert(addr, *amt);
        }
        Ok(map)
    }
}

/// Configuration for the genesis block.
///
/// The genesis block is immutable after mainnet launch: any change to
/// `chain_id`, `genesis_timestamp`, `initial_difficulty`, or
/// `initial_distribution` changes the genesis hash and forks the chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisConfig {
    /// Chain identifier.
    pub chain_id: u64,
    /// Genesis block timestamp (Unix seconds).
    pub genesis_timestamp: u64,
    /// Initial proof-of-work difficulty.
    pub initial_difficulty: u64,
    /// Initial token distribution: address -> ATOLL balance (smallest unit).
    #[serde(with = "address_map")]
    pub initial_distribution: HashMap<Address, u128>,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            chain_id: crate::CHAIN_ID,
            genesis_timestamp: crate::GENESIS_TIMESTAMP,
            initial_difficulty: crate::INITIAL_DIFFICULTY as u64,
            initial_distribution: HashMap::new(),
        }
    }
}

/// Loads and validates genesis configuration.
pub struct GenesisLoader;

impl GenesisLoader {
    /// Loads a [`GenesisConfig`] from a JSON file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or does not contain valid
    /// JSON matching [`GenesisConfig`].
    pub fn load(path: &Path) -> Result<GenesisConfig, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: GenesisConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Returns the default [`GenesisConfig`] for the protocol.
    pub fn default_config() -> GenesisConfig {
        GenesisConfig::default()
    }

    /// Builds the genesis [`Block`] from a [`GenesisConfig`].
    pub fn build_genesis_block(config: &GenesisConfig) -> Block {
        let header = BlockHeader {
            version: 1,
            height: 0,
            shard_id: 0,
            timestamp: config.genesis_timestamp,
            prev_hash: Hash512::zero(),
            state_root: Hash512::zero(),
            transaction_root: Hash512::zero(),
            proposer: Address::zero(),
            difficulty: config.initial_difficulty as u128,
        };
        Block {
            header,
            body: BlockBody {
                transactions: Vec::new(),
            },
        }
    }

    /// Computes the genesis block hash for a [`GenesisConfig`].
    ///
    /// The hash is the Morph-Hash of the serialized genesis block with a zero
    /// previous hash at height 0, matching [`Blockchain::new`].
    pub fn genesis_hash(config: &GenesisConfig) -> Hash512 {
        let block = Self::build_genesis_block(config);
        let mut engine = MorphHashEngine::new();
        engine.morph_hash(&block.serialize_bytes(), &Hash512::zero(), 0)
    }

    /// Validates that `block` hashes to `expected`.
    ///
    /// # Errors
    ///
    /// Returns an error if the recomputed hash does not match `expected`,
    /// indicating a tampered or corrupted genesis block.
    pub fn validate_genesis_hash(block: &Block, expected: Hash512) -> Result<Hash512, String> {
        let mut engine = MorphHashEngine::new();
        let computed = engine.morph_hash(&block.serialize_bytes(), &Hash512::zero(), 0);
        if computed != expected {
            return Err(format!(
                "genesis hash mismatch: expected {:?}, computed {:?}",
                expected, computed
            ));
        }
        Ok(computed)
    }

    /// Applies the initial token distribution to `chain_state` via [`mint_atoll`].
    ///
    /// [`mint_atoll`]: crate::chain::state::ChainState::mint_atoll
    pub fn apply_distribution(
        config: &GenesisConfig,
        chain_state: &mut ChainState,
    ) -> Result<(), String> {
        for (address, amount) in &config.initial_distribution {
            chain_state.mint_atoll(*address, *amount)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::state::ChainState;

    #[test]
    fn genesis_config_default_values_match_protocol_constants() {
        let config = GenesisConfig::default();
        assert_eq!(config.chain_id, crate::CHAIN_ID);
        assert_eq!(config.genesis_timestamp, crate::GENESIS_TIMESTAMP);
        assert_eq!(config.initial_difficulty, crate::INITIAL_DIFFICULTY as u64);
        assert!(config.initial_distribution.is_empty());
    }

    #[test]
    fn genesis_config_serializes_and_deserializes() {
        let mut distribution = HashMap::new();
        distribution.insert(Address::from_bytes([1u8; 20]), 1_000_000);
        let config = GenesisConfig {
            chain_id: 999,
            genesis_timestamp: 1798761600,
            initial_difficulty: 1 << 20,
            initial_distribution: distribution,
        };

        let json = serde_json::to_string(&config).expect("serialization should succeed");
        let restored: GenesisConfig =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(config, restored);
    }

    #[test]
    fn build_genesis_block_has_expected_fields() {
        let config = GenesisConfig::default();
        let block = GenesisLoader::build_genesis_block(&config);

        assert_eq!(block.header.height, 0);
        assert_eq!(block.header.timestamp, crate::GENESIS_TIMESTAMP);
        assert_eq!(block.header.prev_hash, Hash512::zero());
        assert_eq!(block.header.state_root, Hash512::zero());
        assert_eq!(block.header.transaction_root, Hash512::zero());
        assert_eq!(block.header.proposer, Address::zero());
        assert_eq!(block.header.difficulty, crate::INITIAL_DIFFICULTY as u128);
        assert!(block.body.transactions.is_empty());
    }

    #[test]
    fn genesis_hash_is_deterministic() {
        let config = GenesisConfig::default();
        let hash1 = GenesisLoader::genesis_hash(&config);
        let hash2 = GenesisLoader::genesis_hash(&config);
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, Hash512::zero());
    }

    #[test]
    fn validate_genesis_hash_accepts_matching_hash() {
        let config = GenesisConfig::default();
        let block = GenesisLoader::build_genesis_block(&config);
        let expected = GenesisLoader::genesis_hash(&config);

        assert!(GenesisLoader::validate_genesis_hash(&block, expected).is_ok());
    }

    #[test]
    fn validate_genesis_hash_rejects_mismatched_hash() {
        let config = GenesisConfig::default();
        let block = GenesisLoader::build_genesis_block(&config);

        assert!(GenesisLoader::validate_genesis_hash(&block, Hash512::zero()).is_err());
    }

    #[test]
    fn apply_distribution_mints_to_chain_state() {
        let mut config = GenesisConfig::default();
        let address = Address::from_bytes([7u8; 20]);
        config.initial_distribution.insert(address, 1_000_000);

        let mut chain_state = ChainState::new();
        GenesisLoader::apply_distribution(&config, &mut chain_state)
            .expect("distribution should apply");

        assert_eq!(chain_state.balance(&address), 1_000_000);
        assert_eq!(chain_state.total_supply(), 1_000_000);
    }
}
