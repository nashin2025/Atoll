//! ATOLL emission schedule.
//!
//! Implements the halving schedule: 50 ATOLL per block, halving every
//! 210,000 blocks (~1 year at 1-second blocks), with a 21M supply cap.

/// Maximum ATOLL supply (21 million atoms, where 1 ATOLL = 10^8 atoms)
pub const MAX_ATOLL_SUPPLY: u128 = 21_000_000 * 10_u128.pow(8);

/// Initial block reward in Era 0 (50 ATOLL)
pub const INITIAL_BLOCK_REWARD: u128 = 50 * 10_u128.pow(8);

/// Halving interval in blocks (~1 year at 1-second blocks)
pub const HALVING_INTERVAL: u64 = 210_000;

/// Number of halvings after which reward reaches zero
pub const MAX_ERAS: u64 = 64;

/// Computes the ATOLL block reward for a given block height.
///
/// The reward starts at 50 ATOLL and halves every 210,000 blocks.
/// Returns 0 once the reward would be less than 1 atom (10^-8 ATOLL).
pub fn block_reward_at_height(height: u64) -> u128 {
    let era = height / HALVING_INTERVAL;
    if era >= MAX_ERAS {
        return 0;
    }
    INITIAL_BLOCK_REWARD >> era
}

/// Returns the era number for a given block height.
pub fn era_at_height(height: u64) -> u64 {
    height / HALVING_INTERVAL
}

/// Returns the block at which the given era starts.
pub fn era_start_block(era: u64) -> u64 {
    era * HALVING_INTERVAL
}

/// Returns the number of blocks remaining in the current era.
pub fn blocks_remaining_in_era(height: u64) -> u64 {
    let era = era_at_height(height);
    era_start_block(era + 1) - height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_reward_era_0() {
        assert_eq!(block_reward_at_height(0), INITIAL_BLOCK_REWARD);
        assert_eq!(block_reward_at_height(100_000), INITIAL_BLOCK_REWARD);
        assert_eq!(block_reward_at_height(209_999), INITIAL_BLOCK_REWARD);
    }

    #[test]
    fn block_reward_halving() {
        assert_eq!(block_reward_at_height(210_000), INITIAL_BLOCK_REWARD / 2);
        assert_eq!(block_reward_at_height(420_000), INITIAL_BLOCK_REWARD / 4);
        assert_eq!(block_reward_at_height(630_000), INITIAL_BLOCK_REWARD / 8);
    }

    #[test]
    fn block_reward_eventually_zero() {
        assert_eq!(block_reward_at_height(HALVING_INTERVAL * MAX_ERAS), 0);
    }

    #[test]
    fn test_era_at_height() {
        assert_eq!(era_at_height(0), 0);
        assert_eq!(era_at_height(209_999), 0);
        assert_eq!(era_at_height(210_000), 1);
        assert_eq!(era_at_height(419_999), 1);
        assert_eq!(era_at_height(420_000), 2);
    }

    #[test]
    fn test_blocks_remaining_in_era() {
        assert_eq!(blocks_remaining_in_era(0), HALVING_INTERVAL);
        assert_eq!(blocks_remaining_in_era(100_000), 110_000);
        assert_eq!(blocks_remaining_in_era(209_999), 1);
        assert_eq!(blocks_remaining_in_era(210_000), HALVING_INTERVAL);
    }
}
