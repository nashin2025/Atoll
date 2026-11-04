//! Network parameters for the Atoll Protocol.

pub struct NetworkParams {
    pub target_block_time: u64,
    pub max_block_size: usize,
    pub max_tx_per_block: usize,
}

impl Default for NetworkParams {
    fn default() -> Self {
        Self {
            target_block_time: crate::TARGET_BLOCK_TIME,
            max_block_size: 2 * 1024 * 1024, // 2 MB
            max_tx_per_block: 10_000,
        }
    }
}
