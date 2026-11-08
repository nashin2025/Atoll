//! ATOLL token implementation.
//!
//! Implements block reward distribution (6-way split) and genesis allocation.

use crate::chain::state::ChainState;
use crate::primitives::address::Address;
use crate::tokens::emission::block_reward_at_height;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Recipients of block rewards with their percentage shares (Era 0).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RewardRecipient {
    pub name: &'static str,
    pub share_basis_points: u16,
}

/// Default Era 0 reward distribution (sums to 10000 basis points = 100%).
pub const DEFAULT_REWARD_DISTRIBUTION: [RewardRecipient; 6] = [
    RewardRecipient {
        name: "block_proposer",
        share_basis_points: 3000, // 30%
    },
    RewardRecipient {
        name: "pouw_miner",
        share_basis_points: 2500, // 25%
    },
    RewardRecipient {
        name: "post_farmers",
        share_basis_points: 1500, // 15%
    },
    RewardRecipient {
        name: "treasury",
        share_basis_points: 1500, // 15%
    },
    RewardRecipient {
        name: "staking_rewards",
        share_basis_points: 1000, // 10%
    },
    RewardRecipient {
        name: "public_goods",
        share_basis_points: 500, // 5%
    },
];

/// Genesis allocation entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAllocationEntry {
    pub address: Address,
    pub atoll: u128,
}

/// Total genesis ATOLL (2.1M = 10% of 21M supply)
pub fn genesis_atoll_total() -> u128 {
    default_genesis_allocation()
        .iter()
        .map(|e| e.atoll)
        .sum()
}

/// Returns the default genesis allocation.
pub fn default_genesis_allocation() -> Vec<GenesisAllocationEntry> {
    let atoll_unit = 10_u128.pow(8);

    let treasury_addr = Address::from_bytes([1u8; 20]);
    let team_addr = Address::from_bytes([2u8; 20]);
    let research_addr = Address::from_bytes([3u8; 20]);
    let bounty_addr = Address::from_bytes([4u8; 20]);
    let airdrop_addr = Address::from_bytes([5u8; 20]);

    vec![
        GenesisAllocationEntry {
            address: treasury_addr,
            atoll: 840_000 * atoll_unit,
        },
        GenesisAllocationEntry {
            address: team_addr,
            atoll: 630_000 * atoll_unit,
        },
        GenesisAllocationEntry {
            address: research_addr,
            atoll: 315_000 * atoll_unit,
        },
        GenesisAllocationEntry {
            address: bounty_addr,
            atoll: 210_000 * atoll_unit,
        },
        GenesisAllocationEntry {
            address: airdrop_addr,
            atoll: 105_000 * atoll_unit,
        },
    ]
}

/// Distributes block rewards to recipients.
pub fn distribute_block_rewards(
    total_reward: u128,
    distribution: &[RewardRecipient],
) -> HashMap<&'static str, u128> {
    let mut rewards = HashMap::new();
    for recipient in distribution {
        let amount = (total_reward as u128)
            .checked_mul(recipient.share_basis_points as u128)
            .and_then(|v| v.checked_div(10_000))
            .unwrap_or(0);
        rewards.insert(recipient.name, amount);
    }
    rewards
}

/// Applies block rewards to ChainState.
pub fn apply_block_rewards(
    state: &mut ChainState,
    block_height: u64,
    proposer: Address,
    pouw_miner: Option<Address>,
    post_farmers: &[Address],
) -> Result<(), String> {
    let reward = block_reward_at_height(block_height);
    let distribution = distribute_block_rewards(reward, &DEFAULT_REWARD_DISTRIBUTION);

        // Pay block proposer (30%)
    if let Some(amount) = distribution.get("block_proposer") {
        if *amount > 0 {
            state.mint_atoll(proposer, *amount)?;
        }
    }

    // Pay PoUW miner (25%)
    if let Some(amount) = distribution.get("pouw_miner") {
        if *amount > 0 {
            if let Some(miner) = pouw_miner {
                state.mint_atoll(miner, *amount)?;
            }
        }
    }

    // Pay PoST farmers (15%, split evenly)
    if let Some(amount) = distribution.get("post_farmers") {
        if *amount > 0 && !post_farmers.is_empty() {
            let per_farmer = amount.checked_div(post_farmers.len() as u128).unwrap_or(0);
            for farmer in post_farmers {
                state.mint_atoll(*farmer, per_farmer)?;
            }
        }
    }

    // Treasury, staking rewards, public goods are minted to zero address
    // (removed from circulation but tracked in total supply)
    let zero_address = Address::zero();
    for recipient in ["treasury", "staking_rewards", "public_goods"] {
        if let Some(amount) = distribution.get(recipient) {
            if *amount > 0 {
                state.mint_atoll(zero_address, *amount)?;
            }
        }
    }

    Ok(())
}

/// Applies genesis allocation to ChainState.
pub fn apply_genesis_allocation(
    state: &mut ChainState,
    allocations: &[GenesisAllocationEntry],
) -> Result<(), String> {
    for entry in allocations {
        state.mint_atoll(entry.address, entry.atoll)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::state::ChainState;
    use crate::primitives::address::Address;

    #[test]
    fn reward_distribution_sums_to_total() {
        let total = 1000;
        let rewards = distribute_block_rewards(total, &DEFAULT_REWARD_DISTRIBUTION);
        let sum: u128 = rewards.values().sum();
        assert!(sum <= total);
        assert!(sum > total - 10);
    }

    #[test]
    fn reward_distribution_proposer_share() {
        let total = 10_000 * 10_u128.pow(8);
        let rewards = distribute_block_rewards(total, &DEFAULT_REWARD_DISTRIBUTION);
        let proposer = rewards.get("block_proposer").copied().unwrap_or(0);
        assert_eq!(proposer, total * 3000 / 10_000); // 30%
    }

    #[test]
    fn apply_block_rewards_test() {
        let mut state = ChainState::new();
        let proposer = Address::from_bytes([1u8; 20]);
        let pouw_miner = Some(Address::from_bytes([2u8; 20]));
        let post_farmers = vec![
            Address::from_bytes([3u8; 20]),
            Address::from_bytes([4u8; 20]),
        ];

        apply_block_rewards(&mut state, 0, proposer, pouw_miner, &post_farmers).unwrap();

        // Proposer gets 30% of 50 ATOLL = 15 ATOLL
        assert_eq!(state.balance(&proposer), 15 * 10_u128.pow(8));

        // PoUW miner gets 25% = 12.5 ATOLL
        let miner = Address::from_bytes([2u8; 20]);
        assert_eq!(state.balance(&miner), 1_250_000_000);

        // PoST farmers split 15% = 7.5 ATOLL, each 3.75 ATOLL
        for farmer in &post_farmers {
            assert_eq!(state.balance(farmer), 375_000_000);
        }

        assert_eq!(state.total_supply(), 50 * 10_u128.pow(8));
    }

    #[test]
    fn genesis_allocation_total() {
        assert_eq!(genesis_atoll_total(), 2_100_000 * 10_u128.pow(8));
    }

    #[test]
    fn apply_genesis_allocation_test() {
        let mut state = ChainState::new();
        let allocations = default_genesis_allocation();

        apply_genesis_allocation(&mut state, &allocations).unwrap();

        assert_eq!(state.total_supply(), genesis_atoll_total());

        let treasury_addr = Address::from_bytes([1u8; 20]);
        assert_eq!(state.balance(&treasury_addr), 840_000 * 10_u128.pow(8));
    }
}
