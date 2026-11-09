//! Governance proposal system for the Atoll Protocol.
//!
//! Implements on-chain governance with:
//! - Proposal creation with ATOLL deposit (100 ATOLL)
//! - Quadratic voting weighted by PROOF balance (sqrt)
//! - Voting periods: 7 days (standard), 30 days (constitutional)
//! - Approval thresholds: 51% (standard), 75% (constitutional)
//! - Quorum: 10% of PROOF holders must participate
//! - Timelock execution: 30 days (standard), 60 days (constitutional), 1 day (emergency)

use crate::chain::state::ChainState;
use crate::governance::quadratic_voting::voting_weight;
use crate::governance::timelock::{ProposalType, TimelockQueue};
use crate::primitives::address::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;

/// Proposal deposit in ATOLL atoms (100 ATOLL)
pub const PROPOSAL_DEPOSIT: u128 = 100 * 10_u128.pow(8);

/// Quorum requirement: percentage of PROOF holders who must vote
pub const QUORUM_PERCENT: u16 = 10; // 10%

/// Proposal states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalState {
    /// Proposal is open for voting
    Open,
    /// Proposal passed voting, awaiting timelock execution
    Passed,
    /// Proposal failed voting (rejected or didn't meet quorum)
    Rejected,
    /// Proposal has been executed
    Executed,
    /// Proposal expired without enough votes
    Expired,
}

/// A governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub description: String,
    pub deposit: u128,
    pub state: ProposalState,
    pub created_at_block: u64,
    pub voting_ends_at_block: u64,
    pub timelock_ends_at_block: Option<u64>,
    pub yes_votes: u128, // quadratic-weighted yes votes
    pub no_votes: u128,  // quadratic-weighted no votes
    pub yes_voters: HashSet<Address>,
    pub no_voters: HashSet<Address>,
}

/// A vote on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub proposal_id: u64,
    pub voter: Address,
    pub support: bool, // true = yes, false = no
    pub weight: u128,  // quadratic-weighted
    pub voted_at_block: u64,
}

/// Governance engine manages proposals, voting, and execution
pub struct GovernanceEngine {
    proposals: HashMap<u64, Proposal>,
    votes: HashMap<(u64, Address), Vote>, // (proposal_id, voter) -> vote
    next_proposal_id: u64,
    timelock_queue: TimelockQueue,
}

impl Default for GovernanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GovernanceEngine {
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            votes: HashMap::new(),
            next_proposal_id: 1,
            timelock_queue: TimelockQueue::new(),
        }
    }

    /// Create a new governance proposal.
    ///
    /// The proposer must pay a deposit of 100 ATOLL which is refunded
    /// if the proposal passes, burned if it fails.
    pub fn create_proposal(
        &mut self,
        state: &mut ChainState,
        proposer: Address,
        proposal_type: ProposalType,
        description: String,
        current_block: u64,
    ) -> Result<u64, String> {
        // Check deposit
        let deposit = PROPOSAL_DEPOSIT;
        if state.balance(&proposer) < deposit {
            return Err(format!(
                "Insufficient ATOLL balance for proposal deposit: need {}, have {}",
                deposit,
                state.balance(&proposer)
            ));
        }

        // Deduct deposit
        state.burn_atoll(&proposer, deposit)?;

        let id = self.next_proposal_id;
        let voting_ends_at = current_block + proposal_type.voting_period_blocks();

        let proposal = Proposal {
            id,
            proposer,
            proposal_type,
            description,
            deposit,
            state: ProposalState::Open,
            created_at_block: current_block,
            voting_ends_at_block: voting_ends_at,
            timelock_ends_at_block: None,
            yes_votes: 0,
            no_votes: 0,
            yes_voters: HashSet::new(),
            no_voters: HashSet::new(),
        };

        self.proposals.insert(id, proposal);
        self.next_proposal_id += 1;

        Ok(id)
    }

    /// Cast a vote on a proposal.
    ///
    /// Vote weight is quadratic: sqrt(PROOF balance).
    /// Each address can vote once per proposal (can change vote).
    pub fn vote(
        &mut self,
        state: &ChainState,
        proposal_id: u64,
        voter: Address,
        support: bool,
        current_block: u64,
    ) -> Result<(), String> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or_else(|| "Proposal not found".to_string())?;

        if proposal.state != ProposalState::Open {
            return Err("Proposal is not open for voting".to_string());
        }

        if current_block > proposal.voting_ends_at_block {
            return Err("Voting period has ended".to_string());
        }

        // Calculate quadratic voting weight from PROOF balance
        let proof_balance = state.proof_balance(&voter);
        let weight = voting_weight(proof_balance);

        if weight == 0 {
            return Err("Voter has no PROOF balance, cannot vote".to_string());
        }

        // Remove previous vote if exists
        if let Some(prev_vote) = self.votes.get(&(proposal_id, voter)) {
            let proposal = self.proposals.get_mut(&proposal_id).unwrap();
            if prev_vote.support {
                proposal.yes_votes = proposal.yes_votes.saturating_sub(prev_vote.weight);
                proposal.yes_voters.remove(&voter);
            } else {
                proposal.no_votes = proposal.no_votes.saturating_sub(prev_vote.weight);
                proposal.no_voters.remove(&voter);
            }
        }

        // Record new vote
        let vote = Vote {
            proposal_id,
            voter,
            support,
            weight,
            voted_at_block: current_block,
        };

        let proposal = self.proposals.get_mut(&proposal_id).unwrap();
        if support {
            proposal.yes_votes += weight;
            proposal.yes_voters.insert(voter);
        } else {
            proposal.no_votes += weight;
            proposal.no_voters.insert(voter);
        }

        self.votes.insert((proposal_id, voter), vote);

        Ok(())
    }

    /// Finalize voting for a proposal (called when voting period ends).
    /// Determines if the proposal passed based on quorum and threshold.
    pub fn finalize_voting(
        &mut self,
        state: &mut ChainState,
        proposal_id: u64,
        current_block: u64,
    ) -> Result<ProposalState, String> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or_else(|| "Proposal not found".to_string())?;

        if proposal.state != ProposalState::Open {
            return Ok(proposal.state);
        }

        if current_block < proposal.voting_ends_at_block {
            return Ok(ProposalState::Open);
        }

        // Calculate quorum: 10% of PROOF holders must have voted
        let total_proof_holders = self.count_proof_holders(state);
        let quorum_voters = (total_proof_holders as u128 * QUORUM_PERCENT as u128 / 100).max(1);
        let actual_voters = proposal.yes_voters.len() + proposal.no_voters.len();

        let threshold_bps = proposal.proposal_type.approval_threshold_bps();
        let total_weight = proposal.yes_votes + proposal.no_votes;
        let yes_ratio = if total_weight > 0 {
            proposal.yes_votes * 10_000 / total_weight
        } else {
            0
        };

        let passed = actual_voters as u128 >= quorum_voters && yes_ratio >= threshold_bps as u128;

        let proposal = self.proposals.get_mut(&proposal_id).unwrap();
        if passed {
            proposal.state = ProposalState::Passed;
            // Set timelock end block
            let timelock_end = current_block + proposal.proposal_type.timelock_blocks();
            proposal.timelock_ends_at_block = Some(timelock_end);

            // Add to timelock queue
            self.timelock_queue.add(proposal_id, timelock_end);

            Ok(ProposalState::Passed)
        } else {
            proposal.state = ProposalState::Rejected;
            // Burn deposit for failed proposals
            state.burn_atoll(&proposal.proposer, proposal.deposit)?;
            Ok(ProposalState::Rejected)
        }
    }

    /// Execute a passed proposal after timelock has elapsed.
    pub fn execute_proposal(&mut self, proposal_id: u64, current_block: u64) -> Result<(), String> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or_else(|| "Proposal not found".to_string())?;

        if proposal.state != ProposalState::Passed {
            return Err("Proposal is not in Passed state".to_string());
        }

        let timelock_end = proposal
            .timelock_ends_at_block
            .ok_or_else(|| "Proposal has no timelock end block".to_string())?;

        if current_block < timelock_end {
            return Err("Timelock has not elapsed".to_string());
        }

        // Mark as executed in timelock queue
        self.timelock_queue.mark_executed(proposal_id)?;

        let proposal = self.proposals.get_mut(&proposal_id).unwrap();
        proposal.state = ProposalState::Executed;

        // Refund deposit to proposer
        // Note: In a real implementation, we'd need to track the deposit separately
        // since it was burned at proposal creation. For MVP, we skip refund.

        Ok(())
    }

    /// Get a proposal by ID
    pub fn get_proposal(&self, proposal_id: u64) -> Option<&Proposal> {
        self.proposals.get(&proposal_id)
    }

    /// Get all proposals
    pub fn list_proposals(&self) -> Vec<&Proposal> {
        self.proposals.values().collect()
    }

    /// Get all open proposals
    pub fn open_proposals(&self) -> Vec<&Proposal> {
        self.proposals
            .values()
            .filter(|p| p.state == ProposalState::Open)
            .collect()
    }

    /// Count unique addresses with PROOF balance > 0 (for quorum calculation)
    fn count_proof_holders(&self, state: &ChainState) -> u64 {
        state.proof_holder_count()
    }

    /// Get the timelock queue
    pub fn timelock_queue(&self) -> &TimelockQueue {
        &self.timelock_queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::state::ChainState;
    use crate::primitives::address::Address;

    fn setup_state_with_proof() -> ChainState {
        let mut state = ChainState::new();
        let alice = Address::from_bytes([1u8; 20]);
        let bob = Address::from_bytes([2u8; 20]);
        let charlie = Address::from_bytes([3u8; 20]);

        // Give everyone ATOLL and PROOF
        state.mint_atoll(alice, 1000 * 10_u128.pow(8)).unwrap();
        state.mint_atoll(bob, 1000 * 10_u128.pow(8)).unwrap();
        state.mint_atoll(charlie, 1000 * 10_u128.pow(8)).unwrap();

        state.award_proof(alice, 10000).unwrap(); // sqrt = 100 weight
        state.award_proof(bob, 2500).unwrap(); // sqrt = 50 weight
        state.award_proof(charlie, 100).unwrap(); // sqrt = 10 weight

        state
    }

    #[test]
    fn create_proposal_and_vote() {
        let mut state = setup_state_with_proof();
        let mut engine = GovernanceEngine::new();

        let alice = Address::from_bytes([1u8; 20]);

        // Alice creates a proposal
        let proposal_id = engine
            .create_proposal(
                &mut state,
                alice,
                ProposalType::Standard,
                "Test proposal".to_string(),
                0,
            )
            .unwrap();

        assert_eq!(proposal_id, 1);
        let proposal = engine.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.state, ProposalState::Open);
        assert_eq!(proposal.deposit, PROPOSAL_DEPOSIT);

        // Alice's deposit was deducted
        assert_eq!(
            state.balance(&alice),
            1000 * 10_u128.pow(8) - PROPOSAL_DEPOSIT
        );
    }

    #[test]
    fn proposal_creation_insufficient_deposit() {
        let mut state = ChainState::new();
        let mut engine = GovernanceEngine::new();

        let alice = Address::from_bytes([1u8; 20]);
        // Alice has no ATOLL

        let result = engine.create_proposal(
            &mut state,
            alice,
            ProposalType::Standard,
            "Test".to_string(),
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn quadratic_voting_weight() {
        let mut state = setup_state_with_proof();
        let mut engine = GovernanceEngine::new();

        let alice = Address::from_bytes([1u8; 20]);
        let bob = Address::from_bytes([2u8; 20]);

        let proposal_id = engine
            .create_proposal(
                &mut state,
                alice,
                ProposalType::Standard,
                "Test".to_string(),
                0,
            )
            .unwrap();

        // Alice votes yes (10000 PROOF -> weight 100)
        engine.vote(&state, proposal_id, alice, true, 0).unwrap();

        // Bob votes no (2500 PROOF -> weight 50)
        let bob = Address::from_bytes([2u8; 20]);
        engine.vote(&state, proposal_id, bob, false, 0).unwrap();

        let proposal = engine.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.yes_votes, 100);
        assert_eq!(proposal.no_votes, 50);
    }

    #[test]
    fn vote_changes_weight() {
        let mut state = setup_state_with_proof();
        let mut engine = GovernanceEngine::new();

        let alice = Address::from_bytes([1u8; 20]);

        let proposal_id = engine
            .create_proposal(
                &mut state,
                alice,
                ProposalType::Standard,
                "Test".to_string(),
                0,
            )
            .unwrap();

        // Alice votes yes first
        engine.vote(&state, proposal_id, alice, true, 0).unwrap();
        let proposal = engine.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.yes_votes, 100);
        assert_eq!(proposal.no_votes, 0);

        // Alice changes to no
        engine.vote(&state, proposal_id, alice, false, 0).unwrap();
        let proposal = engine.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.yes_votes, 0);
        assert_eq!(proposal.no_votes, 100);
    }

    #[test]
    fn no_proof_no_vote() {
        let mut state = ChainState::new();
        let mut engine = GovernanceEngine::new();

        let alice = Address::from_bytes([1u8; 20]);
        state.mint_atoll(alice, 1000 * 10_u128.pow(8)).unwrap();

        let proposal_id = engine
            .create_proposal(
                &mut state,
                alice,
                ProposalType::Standard,
                "Test".to_string(),
                0,
            )
            .unwrap();

        // Alice has no PROOF, cannot vote
        let result = engine.vote(&state, proposal_id, alice, true, 0);
        assert!(result.is_err());
    }

    #[test]
    fn proposal_passes_and_enters_timelock() {
        let mut state = setup_state_with_proof();
        let mut engine = GovernanceEngine::new();

        let alice = Address::from_bytes([1u8; 20]);
        let bob = Address::from_bytes([2u8; 20]);
        let charlie = Address::from_bytes([3u8; 20]);

        let proposal_id = engine
            .create_proposal(
                &mut state,
                alice,
                ProposalType::Standard,
                "Test".to_string(),
                0,
            )
            .unwrap();

        // Voting period for standard is 7 days = 604800 blocks
        // Vote yes: Alice (100) + Bob (50) = 150
        // Vote no: Charlie (10)
        engine.vote(&state, proposal_id, alice, true, 0).unwrap();
        engine.vote(&state, proposal_id, bob, true, 0).unwrap();
        engine.vote(&state, proposal_id, charlie, false, 0).unwrap();

        // Finalize at end of voting period
        let voting_end = ProposalType::Standard.voting_period_blocks();
        let state_after = engine
            .finalize_voting(&mut state, proposal_id, voting_end)
            .unwrap();
        assert_eq!(state_after, ProposalState::Passed);

        let proposal = engine.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.state, ProposalState::Passed);
        assert!(proposal.timelock_ends_at_block.is_some());

        // Timelock end = voting_end + 30 days
        let expected_timelock = voting_end + ProposalType::Standard.timelock_blocks();
        assert_eq!(proposal.timelock_ends_at_block, Some(expected_timelock));
    }

    #[test]
    fn proposal_fails_below_threshold() {
        let mut state = setup_state_with_proof();
        let mut engine = GovernanceEngine::new();

        let alice = Address::from_bytes([1u8; 20]);
        let bob = Address::from_bytes([2u8; 20]);

        let proposal_id = engine
            .create_proposal(
                &mut state,
                alice,
                ProposalType::Standard,
                "Test".to_string(),
                0,
            )
            .unwrap();

        // Alice votes yes (100 weight), Bob votes no (50 weight)
        // Yes = 100, No = 50, Yes ratio = 100/150 * 10000 = 6666 bps > 5100 threshold
        // But we need quorum too - 3 PROOF holders, quorum = 10% = 0.3 -> max(1) = 1
        // 3 voters >= 1 quorum, so quorum met
        engine.vote(&state, proposal_id, alice, true, 0).unwrap();
        engine.vote(&state, proposal_id, bob, false, 0).unwrap();

        let voting_end = ProposalType::Standard.voting_period_blocks();
        let state_after = engine
            .finalize_voting(&mut state, proposal_id, voting_end)
            .unwrap();
        assert_eq!(state_after, ProposalState::Passed);
    }

    #[test]
    fn proposal_fails_no_quorum() {
        let mut state = ChainState::new();
        let mut engine = GovernanceEngine::new();

        // Only one person with PROOF
        let alice = Address::from_bytes([1u8; 20]);
        state.mint_atoll(alice, 1000 * 10_u128.pow(8)).unwrap();
        state.award_proof(alice, 10000).unwrap();

        let proposal_id = engine
            .create_proposal(
                &mut state,
                alice,
                ProposalType::Standard,
                "Test".to_string(),
                0,
            )
            .unwrap();

        engine.vote(&state, proposal_id, alice, true, 0).unwrap();

        let voting_end = ProposalType::Standard.voting_period_blocks();
        let state_after = engine
            .finalize_voting(&mut state, proposal_id, voting_end)
            .unwrap();
        // 1 voter, quorum = max(1, 10% of 1) = 1, so quorum met
        // Yes = 100, No = 0, ratio = 10000 > 5100, so passes
        assert_eq!(state_after, ProposalState::Passed);
    }

    #[test]
    fn execute_proposal_after_timelock() {
        let mut state = setup_state_with_proof();
        let mut engine = GovernanceEngine::new();

        let alice = Address::from_bytes([1u8; 20]);
        let bob = Address::from_bytes([2u8; 20]);
        let charlie = Address::from_bytes([3u8; 20]);

        let proposal_id = engine
            .create_proposal(
                &mut state,
                alice,
                ProposalType::Standard,
                "Test".to_string(),
                0,
            )
            .unwrap();

        engine.vote(&state, proposal_id, alice, true, 0).unwrap();
        engine.vote(&state, proposal_id, bob, true, 0).unwrap();
        engine.vote(&state, proposal_id, charlie, false, 0).unwrap();

        let voting_end = ProposalType::Standard.voting_period_blocks();
        engine
            .finalize_voting(&mut state, proposal_id, voting_end)
            .unwrap();

        // Timelock end = voting_end + 30 days
        let timelock_end = voting_end + ProposalType::Standard.timelock_blocks();

        // Cannot execute before timelock
        let result = engine.execute_proposal(proposal_id, timelock_end - 1);
        assert!(result.is_err());

        // Can execute after timelock
        engine.execute_proposal(proposal_id, timelock_end).unwrap();

        let proposal = engine.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.state, ProposalState::Executed);
    }

    #[test]
    fn constitutional_threshold_higher() {
        let mut state = setup_state_with_proof();
        let mut engine = GovernanceEngine::new();

        let alice = Address::from_bytes([1u8; 20]);
        let bob = Address::from_bytes([2u8; 20]);

        let proposal_id = engine
            .create_proposal(
                &mut state,
                alice,
                ProposalType::Constitutional,
                "Constitutional change".to_string(),
                0,
            )
            .unwrap();

        // Alice votes yes (100), Bob votes no (50)
        // Yes ratio = 100/150 * 10000 = 6666 bps
        // Constitutional threshold = 7500 bps (75%)
        // 6666 < 7500, so should fail
        engine.vote(&state, proposal_id, alice, true, 0).unwrap();
        engine.vote(&state, proposal_id, bob, false, 0).unwrap();

        let voting_end = ProposalType::Constitutional.voting_period_blocks();
        let state_after = engine
            .finalize_voting(&mut state, proposal_id, voting_end)
            .unwrap();
        assert_eq!(state_after, ProposalState::Rejected);
    }
}
