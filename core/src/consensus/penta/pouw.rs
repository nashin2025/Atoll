//! Proof of Useful Work (PoUW) - Layer 1 of Penta-Consensus.
//!
//! Miners solve computationally useful problems instead of wasting energy
//! on traditional PoW. Problems include protein folding, climate simulation,
//! prime search, ML training, molecular docking, and math proof verification.

use crate::primitives::hash::Hash512;

/// Types of useful work problems that can be registered
#[derive(Debug, Clone)]
pub enum UsefulWorkProblem {
    /// Protein structure prediction
    ProteinFolding {
        sequence: String,
        target_energy: f64,
        verification_hash: Hash512,
    },
    /// Climate model computation
    ClimateSimulation {
        parameters: String,
        timesteps: u64,
        verification_hash: Hash512,
    },
    /// Prime number discovery
    PrimeSearch {
        candidate_start: u128,
        candidate_end: u128,
    },
    /// Machine learning training
    MLTraining {
        model_hash: Hash512,
        dataset_hash: Hash512,
        gradient_commitment: Hash512,
        loss_improvement: f64,
    },
    /// Drug discovery molecular docking
    MolecularDocking {
        protein_target: String,
        ligand: String,
        binding_affinity: f64,
    },
    /// Mathematical proof verification
    MathProof {
        theorem_hash: Hash512,
        proof_steps: Vec<String>,
    },
    /// Fallback: traditional PoW when no useful work available
    TraditionalPoW {
        nonce: u64,
        morph_hash_result: Hash512,
    },
}

/// Result of PoUW validation
#[derive(Debug, Clone)]
pub struct PoUWResult {
    pub valid: bool,
    pub usefulness_score: f64,
    pub difficulty_met: bool,
}

/// Registry of useful work problems submitted by research institutions
pub struct ProblemRegistry {
    problems: Vec<(UsefulWorkProblem, String)>, // (problem, submitter_address)
}

impl Default for ProblemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProblemRegistry {
    pub fn new() -> Self {
        Self { problems: Vec::new() }
    }

    pub fn register(
        &mut self,
        problem: UsefulWorkProblem,
        submitter: &str,
    ) -> usize {
        self.problems.push((problem, submitter.to_string()));
        self.problems.len() - 1
    }

    pub fn get_problem(&self, index: usize) -> Option<&UsefulWorkProblem> {
        self.problems.get(index).map(|(p, _)| p)
    }

    pub fn problem_count(&self) -> usize {
        self.problems.len()
    }
}

/// Verification engine for useful work proofs
pub struct VerificationEngine;

impl VerificationEngine {
    pub fn new() -> Self {
        Self
    }

    /// Verifies that a useful work proof is valid
    pub fn verify(&self, _problem: &UsefulWorkProblem, proof_data: &[u8]) -> bool {
        // Phase 1: Basic validation - check that proof data is non-empty
        // Phase 2: Full verification with zk-SNARKs and problem-specific checks
        !proof_data.is_empty()
    }

    /// Scores the usefulness of a work proof (0.0 - 1.0)
    pub fn score_usefulness(&self, _problem: &UsefulWorkProblem, quality: f64) -> f64 {
        quality.clamp(0.0, 1.0)
    }
}

/// Result of solving useful work
#[derive(Debug, Clone)]
pub struct WorkSolution {
    pub problem_index: usize,
    pub solution_data: Vec<u8>,
    pub quality_score: f64,
}
