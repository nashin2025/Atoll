//! Proof of Reputation (PoR) - Layer 4 of Penta-Consensus.
//!
//! Production implementation with reputation scoring, time decay,
//! and eligibility tracking.

use crate::primitives::address::Address;
use serde::{Deserialize, Serialize};

/// Weights for reputation scoring components
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReputationWeights {
    pub uptime: f64,                   // 0.20
    pub block_validity_rate: f64,      // 0.25
    pub transaction_honesty: f64,      // 0.15
    pub useful_work_contribution: f64, // 0.15
    pub governance_participation: f64, // 0.10
    pub community_endorsements: f64,   // 0.10
    pub age_bonus: f64,                // 0.05
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub endorsements: Vec<Address>,
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
    decay_rate: f64,    // 0.10 per year
    minimum_score: f64, // 0.3
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
        _participant: &Address,
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
        let age_score = ((age_blocks as f64 + 1.0).ln()).min(20.0) / 20.0;

        let raw = uptime * w.uptime
            + validity * w.block_validity_rate
            + tx_honesty * w.transaction_honesty
            + work_score * w.useful_work_contribution
            + gov_score * w.governance_participation
            + endorsement_score * w.community_endorsements
            + age_score * w.age_bonus;

        // Apply time decay (10% per year, blocks per year ~31,536,000)
        let blocks_per_year = 31_536_000u64;
        let years = age_blocks as f64 / blocks_per_year as f64;
        let decay_factor = (-self.decay_rate * years).exp().max(0.5);
        let decayed = raw * decay_factor;

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

    /// Returns the decay rate
    pub fn decay_rate(&self) -> f64 {
        self.decay_rate
    }
}

/// Reputation proof for block inclusion
#[derive(Debug, Clone)]
pub struct ReputationProof {
    pub address: Address,
    pub score: f64,
    pub block_height: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_full_reputation() {
        let calc = ReputationCalculator::new();
        let history = ParticipantHistory {
            online_blocks: 1000,
            total_blocks: 1000,
            proposed_blocks: 100,
            valid_blocks: 99,
            total_transactions: 1000,
            reverted_transactions: 5,
            useful_work_completed: 50,
            governance_votes: 50,
            eligible_votes: 50,
            endorsements: vec![Address::zero(); 10],
            first_seen_height: 1000,
        };
        let result = calc.calculate(&Address::zero(), &history, 2000);
        assert!(result.is_eligible);
        assert!(result.raw_score > 0.7);
        assert!(result.decayed_score > 0.6);
    }

    #[test]
    fn calculate_low_reputation() {
        let calc = ReputationCalculator::new();
        let history = ParticipantHistory {
            online_blocks: 100,
            total_blocks: 1000,
            proposed_blocks: 10,
            valid_blocks: 5,
            total_transactions: 100,
            reverted_transactions: 90,
            useful_work_completed: 0,
            governance_votes: 0,
            eligible_votes: 100,
            endorsements: vec![],
            first_seen_height: 19000,
        };
        let result = calc.calculate(&Address::zero(), &history, 20000);
        assert!(!result.is_eligible);
    }

    #[test]
    fn new_participant_neutral() {
        let calc = ReputationCalculator::new();
        let history = ParticipantHistory::default();
        let result = calc.calculate(&Address::zero(), &history, 0);
        // Should not panic and should produce a valid result
        assert!(result.decayed_score >= 0.0);
        assert!(result.decayed_score <= 1.0);
    }

    #[test]
    fn weights_sum_to_one() {
        let w = ReputationWeights::default();
        let total = w.uptime
            + w.block_validity_rate
            + w.transaction_honesty
            + w.useful_work_contribution
            + w.governance_participation
            + w.community_endorsements
            + w.age_bonus;
        assert!((total - 1.0).abs() < 0.001);
    }
}
