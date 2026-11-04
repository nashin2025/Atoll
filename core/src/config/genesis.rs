//! Genesis configuration for the Atoll Protocol.

pub struct GenesisConfig {
    pub chain_id: u64,
    pub timestamp: u64,
    pub difficulty: u128,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            chain_id: crate::CHAIN_ID,
            timestamp: crate::GENESIS_TIMESTAMP,
            difficulty: crate::INITIAL_DIFFICULTY,
        }
    }
}
