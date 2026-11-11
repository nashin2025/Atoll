//! Staking state for the Atoll Protocol.
//!
//! Tracks per-address staked ATOLL balances and accrued rewards.
//! State is persisted to `staking.json` so it survives wallet restarts.
//!
//! Rewards accrue at ~8.5% APR on staked balances, calculated on-the-fly
//! based on blocks elapsed since the last reward calculation.

use crate::chain::state::ChainState;
use crate::primitives::address::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Per-address staking information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StakeInfo {
    /// ATOLL amount currently staked (in smallest units).
    pub staked_amount: u128,
    /// Block height at which rewards were last calculated.
    pub last_reward_block: u64,
}

/// Complete staking state: all addresses with their stake info.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StakingState {
    /// Address -> stake info.
    pub stakes: HashMap<Address, StakeInfo>,
    /// Path to the persistence file.
    #[serde(skip)]
    path: Option<std::path::PathBuf>,
}

impl StakingState {
    /// Creates a new empty staking state.
    pub fn new() -> Self {
        Self {
            stakes: HashMap::new(),
            path: None,
        }
    }

    /// Sets the persistence file path.
    pub fn with_path(mut self, path: &Path) -> Self {
        self.path = Some(path.to_path_buf());
        self
    }

    /// Saves the staking state to disk.
    pub fn save_to_file(&self) -> Result<(), String> {
        let path = self
            .path
            .as_deref()
            .ok_or_else(|| "no staking state path set".to_string())?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("staking serialization failed: {e}"))?;
        fs::write(path, &json).map_err(|e| format!("failed to write staking state: {e}"))?;
        Ok(())
    }

    /// Loads the staking state from disk, or returns a fresh state if the file
    /// does not exist.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::new().with_path(path));
        }
        let content =
            fs::read_to_string(path).map_err(|e| format!("failed to read staking state: {e}"))?;
        let mut state: Self = serde_json::from_str(&content)
            .map_err(|e| format!("staking deserialization failed: {e}"))?;
        state.path = Some(path.to_path_buf());
        Ok(state)
    }

    /// Returns the staked amount for an address.
    pub fn staked_amount(&self, address: &Address) -> u128 {
        self.stakes
            .get(address)
            .map(|info| info.staked_amount)
            .unwrap_or(0)
    }

    /// Stakes ATOLL for an address, reducing the ChainState balance and
    /// increasing the staked amount. Returns the new staked amount.
    pub fn stake(
        &mut self,
        address: Address,
        amount: u128,
        chain_state: &mut ChainState,
    ) -> Result<u128, String> {
        let current_balance = chain_state.balance(&address);
        if current_balance < amount {
            return Err(format!(
                "insufficient balance: have {} ATOLL, requested {} ATOLL",
                current_balance, amount
            ));
        }

        chain_state
            .burn_atoll(&address, amount)
            .map_err(|e| format!("failed to burn staked ATOLL: {e}"))?;

        let info = self.stakes.entry(address).or_default();
        info.staked_amount = info
            .staked_amount
            .checked_add(amount)
            .ok_or_else(|| "staked amount overflow".to_string())?;

        Ok(info.staked_amount)
    }

    /// Unstakes ATOLL for an address, increasing the ChainState balance and
    /// decreasing the staked amount. Returns the new staked amount.
    pub fn unstake(
        &mut self,
        address: Address,
        amount: u128,
        chain_state: &mut ChainState,
    ) -> Result<u128, String> {
        let info = self
            .stakes
            .get(&address)
            .ok_or_else(|| "no staking position found for address".to_string())?;

        if info.staked_amount < amount {
            return Err(format!(
                "insufficient staked amount: have {} ATOLL staked, requested {} ATOLL",
                info.staked_amount, amount
            ));
        }

        chain_state
            .mint_atoll(address, amount)
            .map_err(|e| format!("failed to mint unstaked ATOLL: {e}"))?;

        let info = self.stakes.get_mut(&address).unwrap();
        info.staked_amount -= amount;

        Ok(info.staked_amount)
    }

    /// Calculates pending rewards for an address based on blocks elapsed
    /// since the last reward calculation.
    ///
    /// Rewards accrue at ~8.5% APR. With 1-second blocks (~31,536,000
    /// blocks/year), the per-block rate is APR / blocks_per_year.
    pub fn pending_rewards(&self, address: &Address, current_block: u64) -> u128 {
        let info = match self.stakes.get(address) {
            Some(i) => i,
            None => return 0,
        };

        if info.staked_amount == 0 || current_block <= info.last_reward_block {
            return 0;
        }

        const APR: f64 = 0.085; // 8.5%
        const BLOCKS_PER_YEAR: u64 = 31_536_000;

        let blocks_elapsed = current_block - info.last_reward_block;
        let reward =
            (info.staked_amount as f64) * (APR * blocks_elapsed as f64 / BLOCKS_PER_YEAR as f64);

        reward as u128
    }

    /// Records that rewards have been claimed up to the given block.
    pub fn claim_rewards(&mut self, address: &Address, current_block: u64) {
        if let Some(info) = self.stakes.get_mut(address) {
            info.last_reward_block = current_block;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::state::ChainState;

    #[test]
    fn stake_and_unstake() {
        let mut state = StakingState::new();
        let mut chain_state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        chain_state.mint_atoll(addr, 1000).unwrap();

        // Stake 500
        let staked = state.stake(addr, 500, &mut chain_state).unwrap();
        assert_eq!(staked, 500);
        assert_eq!(chain_state.balance(&addr), 500);
        assert_eq!(state.staked_amount(&addr), 500);

        // Unstake 200
        let remaining = state.unstake(addr, 200, &mut chain_state).unwrap();
        assert_eq!(remaining, 300);
        assert_eq!(chain_state.balance(&addr), 700);
        assert_eq!(state.staked_amount(&addr), 300);
    }

    #[test]
    fn unstake_insufficient_staked() {
        let mut state = StakingState::new();
        let mut chain_state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        chain_state.mint_atoll(addr, 1000).unwrap();
        state.stake(addr, 500, &mut chain_state).unwrap();

        let result = state.unstake(addr, 600, &mut chain_state);
        assert!(result.is_err());
    }

    #[test]
    fn stake_insufficient_balance() {
        let mut state = StakingState::new();
        let mut chain_state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        chain_state.mint_atoll(addr, 100).unwrap();
        let result = state.stake(addr, 500, &mut chain_state);
        assert!(result.is_err());
    }

    #[test]
    fn pending_rewards_accrue() {
        let mut state = StakingState::new();
        let addr = Address::from_bytes([1u8; 20]);

        // Stake 1000 at block 0
        state.stakes.insert(
            addr,
            StakeInfo {
                staked_amount: 1000,
                last_reward_block: 0,
            },
        );

        // After 1 year of blocks (~8.5% of 1000 = 85)
        let rewards = state.pending_rewards(&addr, 31_536_000);
        // ~8.5% of 1000 = 85 (approximate due to float math)
        assert!(rewards > 80);
        assert!(rewards < 90);

        // No additional rewards if called again at same block
        let rewards2 = state.pending_rewards(&addr, 31_536_000);
        assert_eq!(rewards, rewards2);
    }

    #[test]
    fn no_rewards_without_stake() {
        let state = StakingState::new();
        let addr = Address::from_bytes([1u8; 20]);
        assert_eq!(state.pending_rewards(&addr, 1000), 0);
    }
}
