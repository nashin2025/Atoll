// Example governance proposals for Atoll Protocol
// This file demonstrates how to create and execute governance proposals
// using the GovernanceEngine.

use atoll_core::chain::state::ChainState;
use crate::primitives::address::Address;
use crate::governance::{ProposalType, GovernanceEngine, ProposalState};

/// Example 1: Create and execute a standard proposal
pub fn example_standard_proposal() {
    let mut state = ChainState::new();
    let mut engine = GovernanceEngine::new();

    // Setup: Give proposer ATOLL for deposit
    let proposer = Address::from_bytes([1u8; 20]);
    state.mint_atoll(proposer, 1000 * 10_u128.pow(8)).unwrap();

    // Give voters PROOF for voting weight
    let voter1 = Address::from_bytes([2u8; 20]);
    state.award_proof(voter1, 10000).unwrap(); // weight = 100

    let voter2 = Address::from_bytes([3u8; 20]);
    state.award_proof(voter2, 2500).unwrap(); // weight = 50

    // Create proposal
    let proposal_id = engine.create_proposal(
        &mut state,
        proposer,
        ProposalType::Standard,
        "Reduce block reward from 50 to 45 ATOLL".to_string(),
        0,
    ).unwrap();

    // Vote
    engine.vote(&state, proposal_id, voter1, true, 0).unwrap(); // Yes
    engine.vote(&state, proposal_id, voter2, false, 0).unwrap(); // No

    // Finalize after voting period
    let voting_end = ProposalType::Standard.voting_period_blocks();
    let result = engine.finalize_voting(&mut state, proposal_id, voting_end).unwrap();
    assert_eq!(result, ProposalState::Passed);

    // Execute after timelock
    let timelock_end = voting_end + ProposalType::Standard.timelock_blocks();
    engine.execute_proposal(proposal_id, timelock_end).unwrap();
}

/// Example 2: Constitutional proposal (higher threshold)
pub fn example_constitutional_proposal() {
    let mut state = ChainState::new();
    let mut engine = GovernanceEngine::new();

    let proposer = Address::from_bytes([1u8; 20]);
    state.mint_atoll(proposer, 1000 * 10_u128.pow(8)).unwrap();

    let voter1 = Address::from_bytes([2u8; 20]);
    state.award_proof(voter1, 10000).unwrap(); // weight = 100

    let voter2 = Address::from_bytes([3u8; 20]);
    state.award_proof(voter2, 2500).unwrap(); // weight = 50

    let proposal_id = engine.create_proposal(
        &mut state,
        proposer,
        ProposalType::Constitutional,
        "Change halving interval".to_string(),
        0,
    ).unwrap();

    // Constitutional: 75% threshold needed
    engine.vote(&state, proposal_id, voter1, true, 0).unwrap(); // 100 weight (yes)
    engine.vote(&state, proposal_id, voter2, false, 0).unwrap(); // 50 weight (no)
    // Yes ratio = 100/150 = 66.67%, below 75% threshold -> Rejected

    let voting_end = ProposalType::Constitutional.voting_period_blocks();
    let result = engine.finalize_voting(&mut state, proposal_id, voting_end).unwrap();
    assert_eq!(result, ProposalState::Rejected);
}

/// Example 3: Emergency proposal (fast track)
pub fn example_emergency_proposal() {
    let mut state = ChainState::new();
    let mut engine = GovernanceEngine::new();

    let proposer = Address::from_bytes([1u8; 20]);
    state.mint_atoll(proposer, 1000 * 10_u128.pow(8)).unwrap();

    let voter1 = Address::from_bytes([2u8; 20]);
    state.award_proof(voter1, 10000).unwrap();

    let proposal_id = engine.create_proposal(
        &mut state,
        proposer,
        ProposalType::Emergency,
        "Emergency pause: Contract vulnerability".to_string(),
        0,
    ).unwrap();

    engine.vote(&state, proposal_id, voter1, true, 0).unwrap();

    // Emergency: 1-day voting, 1-day timelock
    let voting_end = ProposalType::Emergency.voting_period_blocks();
    let result = engine.finalize_voting(&mut state, proposal_id, voting_end).unwrap();
    assert_eq!(result, ProposalState::Passed);

    let timelock_end = voting_end + ProposalType::Emergency.timelock_blocks();
    engine.execute_proposal(proposal_id, timelock_end).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_proposal_flow() {
        example_standard_proposal();
    }

    #[test]
    fn test_constitutional_proposal_flow() {
        example_constitutional_proposal();
    }

    #[test]
    fn test_emergency_proposal_flow() {
        example_emergency_proposal();
    }
}
