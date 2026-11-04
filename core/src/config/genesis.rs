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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_config_default_values() {
        let config = GenesisConfig::default();
        assert_eq!(config.chain_id, crate::CHAIN_ID);
        assert_eq!(config.timestamp, crate::GENESIS_TIMESTAMP);
        assert_eq!(config.difficulty, crate::INITIAL_DIFFICULTY);
    }
}
