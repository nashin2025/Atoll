//! Privacy & Security module for the Atoll Protocol.
//!
//! Phase 5: Production implementation of privacy-preserving transactions,
//! post-quantum cryptography, and MEV-resistant encrypted mempool.
//!
//! Includes:
//! - zk-STARKs (transparent, post-quantum zero-knowledge proofs)
//! - Homomorphic encryption (BFV, CKKS schemes)
//! - Encrypted mempool with threshold decryption
//! - Privacy levels (public, shielded, private, fully private, selective disclosure)
//! - Post-quantum signatures (Dilithium, SPHINCS+, Kyber key encapsulation)

pub mod encrypted_mempool;
pub mod homomorphic;
pub mod post_quantum;
pub mod zk_starks;
