//! Proof of Reputation (PoR) - Layer 4 of Penta-Consensus.
//!
//! Reputation score based on uptime, block validity, transaction honesty,
//! useful work contribution, governance participation, endorsements, and age.
//! Score decays over time.

use crate::primitives::address::Address;

/// Weights for reputation scoring components
#[derive(Debug, Clone, Copy)]
pub struct ReputationWeights {
    pub uptime: f64,                    // 0.20
    pub block_validity_rate: f64,       // 0.25
    pub transaction_honesty: f64,       // 0.15
    pub useful_work_contribution: f64,  // 0.15
    pub governance_participation: f64,  // 0.10
    pub community_endorsements: f64,    // 0.10
    pub age_bonus: f64,                 // 0.05
}

impl Default for ReputationWeights {
    fn default() -> Self {
        Self {
            uptime: 0.20,
            block_validity_rate: 0.25,
            transaction_honesty: 0.15,
            useful_work_contribution: 0.15,
            governance_participation: 0.10,
            community_endorsements: 0.10,
            age_bonus: 0.05,
        }
    }
}

/// Reputation score result
#[derive(Debug, Clone)]
pub struct PoRResult {
    pub raw_score: f64,
    pub decayed_score: f64,
    pub is_eligible: bool,
}

/// Participant history for reputation calculation
#[derive(Debug, Clone)]
pub struct ParticipantHistory {
    pub online_blocks: u64,
    pub total_blocks: u64,
    pub proposed_blocks: u64,
    pub valid_blocks: u64,
    pub total_transactions: u64,
    pub reverted_transactions: u64,
    pub useful_work_completed: u64,
    pub governance_votes: u64,
    pub eligible_votes: u64,
    pub endorsements: Vec<crate::primitives::address::Address>,
    pub first_seen_height: u64,
}

impl Default for ParticipantHistory {
    fn default() -> Self {
        Self {
            online_blocks: 0,
            total_blocks: 1, // Avoid division by zero
            proposed_blocks: 0,
            valid_blocks: 0,
            total_transactions: 0,
            reverted_transactions: 0,
            useful_work_completed: 0,
            governance_votes: 0,
            eligible_votes: 0,
            endorsements: Vec::new(),
            first_seen_height: 0,
        }
    }
}

/// Reputation calculator
pub struct ReputationCalculator {
    weights: ReputationWeights,
    decay_rate: f64,        // 0.10 per year
    minimum_score: f64,     // 0.3
    history_depth: u64,
}

impl Default for ReputationCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl ReputationCalculator {
    pub fn new() -> Self {
        Self {
            weights: ReputationWeights::default(),
            decay_rate: 0.10,
            minimum_score: 0.3,
            history_depth: 10_000,
        }
    }

    /// Calculates reputation score for a participant
    pub fn calculate(
        &self,
        _participant: &crate::primitives::address::Address,
        history: &ParticipantHistory,
        current_height: u64,
    ) -> PoRResult {
        let w = self.weights;

        // Uptime score (0-1)
        let uptime = if history.total_blocks > 0 {
            history.online_blocks as f64 / history.total_blocks as f64
        } else {
            0.5
        };

        // Block validity rate (0-1)
        let validity = if history.proposed_blocks > 0 {
            history.valid_blocks as f64 / history.proposed_blocks as f64
        } else {
            0.5
        };

        // Transaction honesty (0-1)
        let tx_honesty = if history.total_transactions > 0 {
            1.0 - (history.reverted_transactions as f64 / history.total_transactions as f64)
        } else {
            0.5
        };

        // Useful work contribution (0-1, normalized)
        let work_score = (history.useful_work_completed as f64 / 100.0).min(1.0);

        // Governance participation (0-1)
        let gov_score = if history.eligible_votes > 0 {
            (history.governance_votes as f64 / history.eligible_votes as f64).min(1.0)
        } else {
            0.0
        };

        // Community endorsements (0-1, normalized)
        let endorsement_score = (history.endorsements.len() as f64 / 10.0).min(1.0);

        // Age bonus (logarithmic, capped at 1.0)
        let age_blocks = current_height.saturating_sub(history.first_seen_height);
        let age_score = ((age_blocks as f64).ln() + 1.0).min(20.0) / 20.0;

        let raw = uptime * w.uptime
            + validity * w.block_validity_rate
            + tx_honesty * w.transaction_honesty
            + work_score * w.useful_work_contribution
            + gov_score * w.governance_participation
            + endorsement_score * w.community_endorsements
            + age_score * w.age_bonus;

        // Apply time decay
        let decayed = raw * (-self.decay_rate * age_blocks as f64).exp().max(0.5);

        PoRResult {
            raw_score: raw,
            decayed_score: decayed,
            is_eligible: decayed >= self.minimum_score,
        }
    }

    /// Returns the minimum reputation score for eligibility
    pub fn minimum_score(&self) -> f64 {
        self.minimum_score
    }
}

/// Reputation proof for block inclusion
#[derive(Debug, Clone)]
pub struct ReputationProof {
    pub address: Address,
    pub score: f64,
    pub block_height: u64,
}
