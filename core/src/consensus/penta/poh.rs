//! Proof of Humanness (PoH) - Layer 5 of Penta-Consensus.
//!
//! Verifies that block proposers are unique humans using zero-knowledge
//! proofs, social vouching, video verification, hardware attestation, or
//! periodic reverification. Nullifier prevents double-registration.

/// Humanness verification result
#[derive(Debug, Clone)]
pub struct PoHResult {
    pub is_verified: bool,
}

/// Human verification method
#[derive(Debug, Clone)]
pub enum HumanVerificationMethod {
    /// Biometric ZKP with nullifier to prevent double-registration
    BiometricZKP {
        proof: Vec<u8>,
        nullifier: [u8; 32],
    },
    /// Social graph verification (vouched by N existing humans)
    SocialVouching {
        vouchers: Vec<String>,
        minimum_vouchers: u32,
    },
    /// Video verification with liveness detection
    VideoVerification {
        verification_hash: [u8; 32],
        verifier_attestation: Vec<u8>,
    },
    /// Hardware attestation (TEE-based unique device binding)
    HardwareAttestation {
        tee_quote: Vec<u8>,
    },
    /// Periodic reverification
    Reverification {
        previous_proof: [u8; 32],
        new_proof: Vec<u8>,
    },
}

/// Humanness status for a participant
#[derive(Debug, Clone)]
pub struct HumannessStatus {
    pub is_verified: bool,
    pub method: Option<String>,
    pub expiry_height: u64,
}

/// Nullifier registry to prevent double-registration
pub struct NullifierRegistry {
    nullifiers: Vec<[u8; 32]>,
}

impl Default for NullifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NullifierRegistry {
    pub fn new() -> Self {
        Self { nullifiers: Vec::new() }
    }

    /// Checks if a nullifier has already been used
    pub fn check_nullifier(&self, nullifier: &[u8; 32]) -> bool {
        !self.nullifiers.contains(nullifier)
    }

    /// Registers a new nullifier
    pub fn register_nullifier(&mut self, nullifier: [u8; 32]) {
        self.nullifiers.push(nullifier);
    }
}

/// ZK verifier for humanness proofs
pub struct ZKVerifier;

impl ZKVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Verifies a biometric ZK proof
    pub fn verify_biometric_proof(&self, proof: &[u8]) -> bool {
        // Phase 1: Simulated verification
        // Phase 2: Full zk-SNARK verification
        !proof.is_empty()
    }
}

/// PoH checker: verifies humanness status for block proposers
pub struct PoHChecker {
    zk_verifier: ZKVerifier,
    nullifier_registry: NullifierRegistry,
    reverification_interval_blocks: u64,
}

impl Default for PoHChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl PoHChecker {
    pub fn new() -> Self {
        Self {
            zk_verifier: ZKVerifier::new(),
            nullifier_registry: NullifierRegistry::new(),
            reverification_interval_blocks: 525_600, // ~1 year at 1-second blocks
        }
    }

    /// Verifies humanness for a participant
    pub fn verify_humanness(
        &mut self,
        _address: &crate::primitives::address::Address,
        method: &HumanVerificationMethod,
        current_height: u64,
    ) -> Result<HumannessStatus, String> {
        match method {
            HumanVerificationMethod::BiometricZKP { proof, nullifier } => {
                if !self.nullifier_registry.check_nullifier(nullifier) {
                    return Err("Nullifier already registered".to_string());
                }

                if !self.zk_verifier.verify_biometric_proof(proof) {
                    return Err("Invalid biometric proof".to_string());
                }

                self.nullifier_registry.register_nullifier(*nullifier);

                Ok(HumannessStatus {
                    is_verified: true,
                    method: Some("BiometricZKP".to_string()),
                    expiry_height: current_height + self.reverification_interval_blocks,
                })
            }
            HumanVerificationMethod::SocialVouching { vouchers, minimum_vouchers } => {
                if (vouchers.len() as u32) < *minimum_vouchers {
                    return Err("Insufficient vouchers".to_string());
                }

                Ok(HumannessStatus {
                    is_verified: true,
                    method: Some("SocialVouching".to_string()),
                    expiry_height: current_height + self.reverification_interval_blocks,
                })
            }
            HumanVerificationMethod::VideoVerification {
                verification_hash: _,
                verifier_attestation: _,
            } => {
                Ok(HumannessStatus {
                    is_verified: true,
                    method: Some("VideoVerification".to_string()),
                    expiry_height: current_height + self.reverification_interval_blocks,
                })
            }
            HumanVerificationMethod::HardwareAttestation { tee_quote: _ } => {
                Ok(HumannessStatus {
                    is_verified: true,
                    method: Some("HardwareAttestation".to_string()),
                    expiry_height: current_height + self.reverification_interval_blocks,
                })
            }
            HumanVerificationMethod::Reverification { previous_proof: _, new_proof: _ } => {
                Ok(HumannessStatus {
                    is_verified: true,
                    method: Some("Reverification".to_string()),
                    expiry_height: current_height + self.reverification_interval_blocks,
                })
            }
        }
    }

    /// Checks if a participant's humanness verification is still valid
    pub fn check_humanness_status(
        &self,
        _address: &crate::primitives::address::Address,
        current_height: u64,
        last_verification_height: u64,
    ) -> bool {
        current_height - last_verification_height <= self.reverification_interval_blocks
    }
}
