//! Tokens module for the Atoll Protocol.
//!
//! Triple-token system: ATOLL (value), MIND (gas), PROOF (reputation).

pub mod chimera_token;
pub mod emission;
pub mod mind_token;
pub mod proof_token;

pub use chimera_token::{
    apply_block_rewards, apply_genesis_allocation, default_genesis_allocation,
    distribute_block_rewards, genesis_atoll_total, GenesisAllocationEntry, RewardRecipient,
    DEFAULT_REWARD_DISTRIBUTION,
};
pub use emission::{
    block_reward_at_height, blocks_remaining_in_era, era_at_height, era_start_block,
    MAX_ATOLL_SUPPLY,
};
pub use mind_token::{burn_mind_fees, mint_mind, MIND_MINT_PER_BLOCK};
pub use proof_token::{
    award_proof, award_proof_community, award_proof_governance, award_proof_humanity,
    award_proof_storage, award_proof_useful_work, decay_proof, proof_balance, PROOF_BUG_BOUNTY,
    PROOF_COMMUNITY, PROOF_GOVERNANCE_VOTE, PROOF_HUMANITY_VERIFIED, PROOF_STORAGE_PER_BLOCK,
    PROOF_USEFUL_WORK, PROOF_VALID_BLOCK,
};
