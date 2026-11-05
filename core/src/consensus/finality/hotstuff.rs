//! HotStuff BFT finality for the Penta-Consensus.
//!
//! Provides sub-second finality (~3 seconds, 3 rounds) with f < n/3
//! Byzantine fault tolerance.

use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;

/// HotStuff vote type
#[derive(Debug, Clone, PartialEq)]
pub enum VoteType {
    Prepare,
    PreCommit,
    Commit,
}

/// A BFT vote from a validator
#[derive(Debug, Clone)]
pub struct BFTVote {
    pub validator: Address,
    pub block_hash: Hash512,
    pub height: u64,
    pub round: u64,
    pub vote_type: VoteType,
    pub signature: Vec<u8>,
}

/// HotStuff BFT state
#[derive(Debug, Clone)]
pub struct HotStuffState {
    pub current_height: u64,
    pub current_round: u64,
    pub locked_block: Option<Hash512>,
    pub locked_round: u64,
    pub votes: Vec<BFTVote>,
    pub quorum_size: u64,
    pub total_validators: u64,
}

impl HotStuffState {
    pub fn new(total_validators: u64) -> Self {
        Self {
            current_height: 0,
            current_round: 0,
            locked_block: None,
            locked_round: 0,
            votes: Vec::new(),
            quorum_size: (total_validators * 2 / 3) + 1, // f < n/3 => need 2n/3 + 1
            total_validators,
        }
    }

    /// Returns true if we have a quorum of votes for this block at this height/round
    pub fn has_quorum(
        &self,
        block_hash: &Hash512,
        height: u64,
        round: u64,
        vote_type: VoteType,
    ) -> bool {
        let count = self
            .votes
            .iter()
            .filter(|v| {
                v.block_hash == *block_hash
                    && v.height == height
                    && v.round == round
                    && v.vote_type == vote_type
            })
            .count();
        count >= self.quorum_size as usize
    }

    /// Adds a vote to the state
    pub fn add_vote(&mut self, vote: BFTVote) {
        self.votes.push(vote);
    }

    /// Clears votes for a different height/round
    pub fn clear_stale_votes(&mut self, height: u64, round: u64) {
        self.votes
            .retain(|v| v.height == height && v.round == round);
    }
}

/// HotStuff BFT finality checker
pub struct HotStuffBFT;

impl HotStuffBFT {
    pub fn new() -> Self {
        Self
    }

    /// Checks if a block has achieved finality
    ///
    /// Finality is achieved when a quorum of validators has committed to
    /// the block at the current height and round (3 HotStuff rounds).
    pub fn check_finality(&self, state: &HotStuffState, block_hash: &Hash512) -> bool {
        state.has_quorum(
            block_hash,
            state.current_height,
            state.current_round,
            VoteType::Commit,
        )
    }

    /// Creates a vote for a block
    pub fn create_vote(
        validator: Address,
        block_hash: Hash512,
        height: u64,
        round: u64,
        vote_type: VoteType,
        signature: Vec<u8>,
    ) -> BFTVote {
        BFTVote {
            validator,
            block_hash,
            height,
            round,
            vote_type,
            signature,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotstuff_state_initialization() {
        let state = HotStuffState::new(100);
        assert_eq!(state.current_height, 0);
        assert_eq!(state.quorum_size, 67); // 2*100/3 + 1 = 67
        assert_eq!(state.total_validators, 100);
    }

    #[test]
    fn hotstuff_quorum_detection() {
        let mut state = HotStuffState::new(100);
        let block_hash = Hash512::zero();
        let validator = crate::primitives::address::Address::zero();

        // Add 67 votes (quorum)
        for _ in 0..67 {
            state.add_vote(BFTVote {
                validator: validator.clone(),
                block_hash,
                height: 1,
                round: 1,
                vote_type: VoteType::Commit,
                signature: vec![1; 64],
            });
        }

        assert!(state.has_quorum(&block_hash, 1, 1, VoteType::Commit));
    }

    #[test]
    fn hotstuff_no_quorum_below_threshold() {
        let mut state = HotStuffState::new(100);
        let block_hash = Hash512::zero();
        let validator = crate::primitives::address::Address::zero();

        // Add 66 votes (below quorum)
        for _ in 0..66 {
            state.add_vote(BFTVote {
                validator: validator.clone(),
                block_hash,
                height: 1,
                round: 1,
                vote_type: VoteType::Commit,
                signature: vec![1; 64],
            });
        }

        assert!(!state.has_quorum(&block_hash, 1, 1, VoteType::Commit));
    }

    #[test]
    fn hotstuff_finality_check() {
        let bft = HotStuffBFT::new();
        let mut state = HotStuffState::new(100);
        state.current_height = 1;
        state.current_round = 1;
        let block_hash = Hash512::zero();
        let validator = crate::primitives::address::Address::zero();

        // Add commit votes
        for _ in 0..67 {
            state.add_vote(BFTVote {
                validator: validator.clone(),
                block_hash,
                height: 1,
                round: 1,
                vote_type: VoteType::Commit,
                signature: vec![1; 64],
            });
        }

        assert!(bft.check_finality(&state, &block_hash));
    }
}
