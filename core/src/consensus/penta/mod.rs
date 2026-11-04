//! Penta-Consensus: five simultaneous consensus layers with HotStuff BFT finality.
//!
//! Production implementation with all five layers, composite scoring,
//! slashing conditions, and HotStuff BFT finality.

use crate::consensus::finality::hotstuff::{BFTVote, HotStuffBFT, HotStuffState, VoteType};
use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;
use serde::{Deserialize, Serialize};

pub mod poh;
pub mod por;
pub mod pos_vrf;
pub mod post;
pub mod pouw;

// ---------------------------------------------------------------------------
// Consensus Weights
// ---------------------------------------------------------------------------

/// Weights for each consensus layer in composite scoring
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConsensusWeights {
    pub useful_work: f64, // 0.30
    pub stake_vrf: f64,   // 0.25
    pub spacetime: f64,   // 0.15
    pub reputation: f64,  // 0.15
    pub humanness: f64,   // 0.15
}

impl Default for ConsensusWeights {
    fn default() -> Self {
        Self {
            useful_work: 0.30,
            stake_vrf: 0.25,
            spacetime: 0.15,
            reputation: 0.15,
            humanness: 0.15,
        }
    }
}

/// Minimum composite score for a block to be valid
pub const MIN_COMPOSITE_SCORE: f64 = 0.5;

/// Minimum reputation score for a participant to be eligible
pub const MIN_REPUTATION_SCORE: f64 = 0.3;

// ---------------------------------------------------------------------------
// Layer Validation Results
// ---------------------------------------------------------------------------

/// Result of validating a block against all five consensus layers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PentaValidation {
    pub pouw_valid: bool,
    pub pouw_score: f64,
    pub pos_valid: bool,
    pub pos_score: f64,
    pub post_valid: bool,
    pub post_score: f64,
    pub por_valid: bool,
    pub por_score: f64,
    pub poh_valid: bool,
    pub poh_score: f64,
    pub composite_score: f64,
    pub finality_achieved: bool,
    pub all_layers_pass: bool,
}

impl PentaValidation {
    /// Returns true if the block is fully valid (all layers pass + composite >= threshold)
    pub fn is_valid(&self) -> bool {
        self.all_layers_pass && self.composite_score >= MIN_COMPOSITE_SCORE
    }
}

// ---------------------------------------------------------------------------
// Consensus Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusError {
    PoUWFailed,
    PoSFailed,
    PoSTFailed,
    ReputationTooLow,
    HumanVerificationRequired,
    CompositeScoreTooLow,
    FinalityNotAchieved,
    InvalidSlashingEvidence,
    ValidatorNotFound,
}

// ---------------------------------------------------------------------------
// Penta-Consensus Coordinator
// ---------------------------------------------------------------------------

/// Penta-Consensus engine coordinating all five layers
pub struct PentaConsensus {
    weights: ConsensusWeights,
    hotstuff: HotStuffBFT,
}

impl Default for PentaConsensus {
    fn default() -> Self {
        Self::new()
    }
}

impl PentaConsensus {
    /// Creates a new PentaConsensus engine with default weights
    pub fn new() -> Self {
        Self {
            weights: ConsensusWeights::default(),
            hotstuff: HotStuffBFT::new(),
        }
    }

    /// Creates a PentaConsensus engine with custom weights
    pub fn with_weights(weights: ConsensusWeights) -> Self {
        Self {
            weights,
            hotstuff: HotStuffBFT::new(),
        }
    }

    /// Creates a PentaConsensus engine with validator count for HotStuff
    pub fn with_validators(total_validators: u64) -> Self {
        Self {
            weights: ConsensusWeights::default(),
            hotstuff: HotStuffBFT::new(),
        }
    }

    /// Validates a block against all five consensus layers
    pub fn validate_block(
        &self,
        pouw_result: &pouw::PoUWResult,
        pos_result: &pos_vrf::PoSResult,
        post_result: &post::PoSTResult,
        por_result: &por::PoRResult,
        poh_result: &poh::PoHResult,
    ) -> Result<PentaValidation, ConsensusError> {
        // Layer 1: Proof of Useful Work
        if !pouw_result.valid || !pouw_result.difficulty_met {
            return Err(ConsensusError::PoUWFailed);
        }

        // Layer 2: Proof of Stake with VRF
        if !pos_result.valid {
            return Err(ConsensusError::PoSFailed);
        }

        // Layer 3: Proof of Spacetime
        if !post_result.valid {
            return Err(ConsensusError::PoSTFailed);
        }

        // Layer 4: Proof of Reputation
        if !por_result.is_eligible {
            return Err(ConsensusError::ReputationTooLow);
        }

        // Layer 5: Proof of Humanness
        if !poh_result.is_verified {
            return Err(ConsensusError::HumanVerificationRequired);
        }

        // Composite scoring
        let pouw_score = pouw_result.usefulness_score;
        let pos_score = pos_result.combined_score;
        let post_score = post_result.size_score;
        let por_score = por_result.decayed_score;
        let poh_score = if poh_result.is_verified { 1.0 } else { 0.0 };

        let composite = pouw_score * self.weights.useful_work
            + pos_score * self.weights.stake_vrf
            + post_score * self.weights.spacetime
            + por_score * self.weights.reputation
            + poh_score * self.weights.humanness;

        if composite < MIN_COMPOSITE_SCORE {
            return Err(ConsensusError::CompositeScoreTooLow);
        }

        Ok(PentaValidation {
            pouw_valid: true,
            pouw_score,
            pos_valid: true,
            pos_score,
            post_valid: true,
            post_score,
            por_valid: true,
            por_score,
            poh_valid: true,
            poh_score,
            composite_score: composite,
            finality_achieved: false, // Set by HotStuff finality check
            all_layers_pass: true,
        })
    }

    /// Validates a block with HotStuff BFT finality check
    pub fn validate_block_with_finality(
        &self,
        pouw_result: &pouw::PoUWResult,
        pos_result: &pos_vrf::PoSResult,
        post_result: &post::PoSTResult,
        por_result: &por::PoRResult,
        poh_result: &poh::PoHResult,
        hotstuff_state: &HotStuffState,
        block_hash: &Hash512,
    ) -> Result<PentaValidation, ConsensusError> {
        let mut validation =
            self.validate_block(pouw_result, pos_result, post_result, por_result, poh_result)?;

        // Check HotStuff BFT finality
        let finality = self.hotstuff.check_finality(hotstuff_state, block_hash);
        validation.finality_achieved = finality;

        if !finality {
            return Err(ConsensusError::FinalityNotAchieved);
        }

        Ok(validation)
    }

    /// Returns the consensus weights
    pub fn weights(&self) -> ConsensusWeights {
        self.weights
    }

    /// Returns the HotStuff BFT instance
    pub fn hotstuff(&self) -> &HotStuffBFT {
        &self.hotstuff
    }

    /// Returns mutable reference to HotStuff BFT
    pub fn hotstuff_mut(&mut self) -> &mut HotStuffBFT {
        &mut self.hotstuff
    }
}

// ---------------------------------------------------------------------------
// Slashing
// ---------------------------------------------------------------------------

/// Slashing action for a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashAction {
    pub validator: Address,
    pub penalty_percent: u8,
    pub ban_epochs: u64,
    pub burn: bool,
}

/// Evidence for slashing a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlashingEvidence {
    /// Validator voted for two different blocks at the same height
    DoubleVote {
        validator: Address,
        vote1_hash: Hash512,
        vote2_hash: Hash512,
    },
    /// Validator proposed two different blocks at the same slot
    Equivocation {
        validator: Address,
        block1_hash: Hash512,
        block2_hash: Hash512,
    },
    /// Validator missed too many slots
    Downtime {
        validator: Address,
        missed_slots: u64,
    },
    /// Validator submitted invalid useful work
    InvalidWork { validator: Address },
}

impl PentaConsensus {
    /// Checks slashing conditions and returns the appropriate action
    pub fn check_slashing(&self, evidence: &SlashingEvidence) -> Option<SlashAction> {
        match evidence {
            SlashingEvidence::DoubleVote { validator, .. } => Some(SlashAction {
                validator: validator.clone(),
                penalty_percent: 33,
                ban_epochs: 36_500, // ~100 years
                burn: true,
            }),
            SlashingEvidence::Equivocation { validator, .. } => Some(SlashAction {
                validator: validator.clone(),
                penalty_percent: 100,
                ban_epochs: u64::MAX, // Permanent ban
                burn: true,
            }),
            SlashingEvidence::Downtime {
                validator,
                missed_slots,
            } if *missed_slots > 1000 => Some(SlashAction {
                validator: validator.clone(),
                penalty_percent: 1,
                ban_epochs: 0,
                burn: false,
            }),
            SlashingEvidence::Downtime { .. } => None,
            SlashingEvidence::InvalidWork { validator } => Some(SlashAction {
                validator: validator.clone(),
                penalty_percent: 50,
                ban_epochs: 365,
                burn: true,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::finality::hotstuff::VoteType;

    fn valid_pouw() -> pouw::PoUWResult {
        pouw::PoUWResult {
            valid: true,
            usefulness_score: 0.8,
            difficulty_met: true,
        }
    }

    fn valid_pos() -> pos_vrf::PoSResult {
        pos_vrf::PoSResult {
            valid: true,
            combined_score: 0.9,
        }
    }

    fn valid_post() -> post::PoSTResult {
        post::PoSTResult {
            valid: true,
            size_score: 1.0,
        }
    }

    fn valid_por() -> por::PoRResult {
        por::PoRResult {
            raw_score: 0.8,
            decayed_score: 0.72,
            is_eligible: true,
        }
    }

    fn valid_poh() -> poh::PoHResult {
        poh::PoHResult { is_verified: true }
    }

    #[test]
    fn valid_block_passes_all_layers() {
        let consensus = PentaConsensus::new();
        let result = consensus.validate_block(
            &valid_pouw(),
            &valid_pos(),
            &valid_post(),
            &valid_por(),
            &valid_poh(),
        );
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.is_valid());
        assert!(v.all_layers_pass);
        assert!(v.composite_score >= MIN_COMPOSITE_SCORE);
        println!("Composite score: {:.3}", v.composite_score);
    }

    #[test]
    fn invalid_pouw_fails() {
        let consensus = PentaConsensus::new();
        let bad_pouw = pouw::PoUWResult {
            valid: false,
            usefulness_score: 0.8,
            difficulty_met: true,
        };
        let result = consensus.validate_block(
            &bad_pouw,
            &valid_pos(),
            &valid_post(),
            &valid_por(),
            &valid_poh(),
        );
        assert_eq!(result, Err(ConsensusError::PoUWFailed));
    }

    #[test]
    fn low_reputation_fails() {
        let consensus = PentaConsensus::new();
        let bad_por = por::PoRResult {
            raw_score: 0.1,
            decayed_score: 0.09,
            is_eligible: false,
        };
        let result = consensus.validate_block(
            &valid_pouw(),
            &valid_pos(),
            &valid_post(),
            &bad_por,
            &valid_poh(),
        );
        assert_eq!(result, Err(ConsensusError::ReputationTooLow));
    }

    #[test]
    fn not_human_verified_fails() {
        let consensus = PentaConsensus::new();
        let bad_poh = poh::PoHResult { is_verified: false };
        let result = consensus.validate_block(
            &valid_pouw(),
            &valid_pos(),
            &valid_post(),
            &valid_por(),
            &bad_poh,
        );
        assert_eq!(result, Err(ConsensusError::HumanVerificationRequired));
    }

    #[test]
    fn composite_score_calculation() {
        let consensus = PentaConsensus::new();
        let v = consensus
            .validate_block(
                &valid_pouw(),
                &valid_pos(),
                &valid_post(),
                &valid_por(),
                &valid_poh(),
            )
            .unwrap();

        // Expected: 0.8*0.30 + 0.9*0.25 + 1.0*0.15 + 0.72*0.15 + 1.0*0.15
        let expected = 0.8 * 0.30 + 0.9 * 0.25 + 1.0 * 0.15 + 0.72 * 0.15 + 1.0 * 0.15;
        assert!((v.composite_score - expected).abs() < 0.001);
    }

    #[test]
    fn slashing_double_vote() {
        let consensus = PentaConsensus::new();
        let evidence = SlashingEvidence::DoubleVote {
            validator: Address::zero(),
            vote1_hash: Hash512::zero(),
            vote2_hash: Hash512::zero(),
        };
        let action = consensus.check_slashing(&evidence).unwrap();
        assert_eq!(action.penalty_percent, 33);
        assert!(action.burn);
    }

    #[test]
    fn slashing_equivocation() {
        let consensus = PentaConsensus::new();
        let evidence = SlashingEvidence::Equivocation {
            validator: Address::zero(),
            block1_hash: Hash512::zero(),
            block2_hash: Hash512::zero(),
        };
        let action = consensus.check_slashing(&evidence).unwrap();
        assert_eq!(action.penalty_percent, 100);
        assert!(action.burn);
        assert_eq!(action.ban_epochs, u64::MAX);
    }

    #[test]
    fn slashing_downtime_below_threshold() {
        let consensus = PentaConsensus::new();
        let evidence = SlashingEvidence::Downtime {
            validator: Address::zero(),
            missed_slots: 500,
        };
        assert!(consensus.check_slashing(&evidence).is_none());
    }

    #[test]
    fn weights_sum_to_one() {
        let w = ConsensusWeights::default();
        let total = w.useful_work + w.stake_vrf + w.spacetime + w.reputation + w.humanness;
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn validate_block_with_finality() {
        let consensus = PentaConsensus::new();
        let mut hotstuff_state = HotStuffState::new(100);
        hotstuff_state.current_height = 1;
        hotstuff_state.current_round = 1;

        // Add commit votes for quorum
        let block_hash = Hash512::zero();
        let validator = Address::zero();
        for _ in 0..67 {
            hotstuff_state.add_vote(BFTVote {
                validator: validator.clone(),
                block_hash,
                height: 1,
                round: 1,
                vote_type: VoteType::Commit,
                signature: vec![1; 64],
            });
        }

        let result = consensus.validate_block_with_finality(
            &valid_pouw(),
            &valid_pos(),
            &valid_post(),
            &valid_por(),
            &valid_poh(),
            &hotstuff_state,
            &block_hash,
        );
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.finality_achieved);
        assert!(v.is_valid());
    }

    #[test]
    fn validate_block_without_finality_fails() {
        let consensus = PentaConsensus::new();
        let hotstuff_state = HotStuffState::new(100); // No votes, no quorum

        let block_hash = Hash512::zero();
        let result = consensus.validate_block_with_finality(
            &valid_pouw(),
            &valid_pos(),
            &valid_post(),
            &valid_por(),
            &valid_poh(),
            &hotstuff_state,
            &block_hash,
        );
        assert_eq!(result, Err(ConsensusError::FinalityNotAchieved));
    }
}
