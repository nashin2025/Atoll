//! PROOF token implementation.
//!
//! PROOF is a soulbound reputation token. It is non-transferable, awarded
//! for various contributions, and decays at 10% per year (continuously
//! compounded).

use crate::chain::state::ChainState;
use crate::primitives::address::Address;

/// PROOF awarded for producing a valid block
pub const PROOF_VALID_BLOCK: u128 = 10;
/// PROOF awarded for solving useful work
pub const PROOF_USEFUL_WORK: u128 = 50;
/// PROOF awarded for governance vote
pub const PROOF_GOVERNANCE_VOTE: u128 = 5;
/// PROOF awarded per block for storage contribution
pub const PROOF_STORAGE_PER_BLOCK: u128 = 3;
/// PROOF awarded for humanity verification (one-time)
pub const PROOF_HUMANITY_VERIFIED: u128 = 100;
/// PROOF awarded for bug bounty
pub const PROOF_BUG_BOUNTY: u128 = 1_000;
/// PROOF awarded for community contribution
pub const PROOF_COMMUNITY: u128 = 20;

/// Blocks per year at 1-second block time (~31.5M)
const BLOCKS_PER_YEAR: u64 = 31_536_000;
/// PROOF decay rate: 10% per year
const DECAY_RATE: f64 = 0.10;

/// Awards PROOF tokens to an address (soulbound - cannot be transferred).
pub fn award_proof(state: &mut ChainState, address: Address, amount: u128) -> Result<(), String> {
    state.award_proof(address, amount)
}

/// Applies PROOF decay (10% per year, continuously compounded).
///
/// Should be called once per block to apply continuous decay.
pub fn decay_proof(state: &mut ChainState, current_block: u64) {
    state.decay_proof(current_block);
}

/// Returns the PROOF balance of an address
pub fn proof_balance(state: &ChainState, address: &Address) -> u128 {
    state.proof_balance(address)
}

/// Awards PROOF for producing a valid block
pub fn award_proof_valid_block(state: &mut ChainState, address: Address) -> Result<(), String> {
    award_proof(state, address, PROOF_VALID_BLOCK)
}

/// Awards PROOF for solving useful work
pub fn award_proof_useful_work(
    state: &mut ChainState,
    address: Address,
) -> Result<(), String> {
    award_proof(state, address, PROOF_USEFUL_WORK)
}

/// Awards PROOF for governance vote
pub fn award_proof_governance(
    state: &mut ChainState,
    address: Address,
) -> Result<(), String> {
    award_proof(state, address, PROOF_GOVERNANCE_VOTE)
}

/// Awards PROOF for storage contribution (per block)
pub fn award_proof_storage(
    state: &mut ChainState,
    address: Address,
) -> Result<(), String> {
    award_proof(state, address, PROOF_STORAGE_PER_BLOCK)
}

/// Awards PROOF for humanity verification (one-time)
pub fn award_proof_humanity(
    state: &mut ChainState,
    address: Address,
) -> Result<(), String> {
    award_proof(state, address, PROOF_HUMANITY_VERIFIED)
}

/// Awards PROOF for bug bounty
pub fn award_proof_bug_bounty(
    state: &mut ChainState,
    address: Address,
) -> Result<(), String> {
    award_proof(state, address, PROOF_BUG_BOUNTY)
}

/// Awards PROOF for community contribution
pub fn award_proof_community(
    state: &mut ChainState,
    address: Address,
) -> Result<(), String> {
    award_proof(state, address, PROOF_COMMUNITY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::state::ChainState;
    use crate::primitives::address::Address;

    #[test]
    fn proof_award_and_decay() {
        let mut state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        award_proof(&mut state, addr, 1000).unwrap();
        assert_eq!(proof_balance(&state, &addr), 1000);

        // No decay if no blocks elapsed
        decay_proof(&mut state, 0);
        assert_eq!(proof_balance(&state, &addr), 1000);

        // Decay after ~1 year
        decay_proof(&mut state, BLOCKS_PER_YEAR);
        let decayed = proof_balance(&state, &addr);
        assert!(decayed < 1000);
        assert!(decayed > 900);
    }

    #[test]
    fn proof_is_soulbound() {
        let mut state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        award_proof(&mut state, addr, 500).unwrap();
        assert_eq!(proof_balance(&state, &addr), 500);

        // PROOF can only be awarded, not transferred - no transfer method exists
        decay_proof(&mut state, BLOCKS_PER_YEAR);
        assert!(proof_balance(&state, &addr) < 500);
    }

    #[test]
    fn proof_award_types() {
        let mut state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        award_proof_valid_block(&mut state, addr).unwrap();
        assert_eq!(proof_balance(&state, &addr), PROOF_VALID_BLOCK);

        award_proof_useful_work(&mut state, addr).unwrap();
        assert_eq!(
            proof_balance(&state, &addr),
            PROOF_VALID_BLOCK + PROOF_USEFUL_WORK
        );

        award_proof_governance(&mut state, addr).unwrap();
        assert_eq!(
            proof_balance(&state, &addr),
            PROOF_VALID_BLOCK + PROOF_USEFUL_WORK + PROOF_GOVERNANCE_VOTE
        );
    }
}
