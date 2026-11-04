//! Signature schemes for the Atoll Protocol.
//!
//! Hybrid signatures: Ed25519 (classical) + Dilithium (post-quantum)
//! + SPHINCS+ (hash-based backup).

pub mod dilithium;
pub mod ed25519;
pub mod sphincs;
pub mod threshold;

// Placeholder - full implementations after Phase 0 research
