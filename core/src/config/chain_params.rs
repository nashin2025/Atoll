//! Chain parameters for the Atoll Protocol.

pub struct ChainParams {
    pub halving_interval: u64,
    pub initial_block_reward: u128,
}

impl Default for ChainParams {
    fn default() -> Self {
        Self {
            halving_interval: 210_000,
            initial_block_reward: 50_000_000, // 50 ATOLL with 6 decimals
        }
    }
}
