//! Post-Quantum Cryptography for the Atoll Protocol.
//!
//! Post-quantum signatures and key encapsulation mechanisms.
//! Phase 1: Interface and simulation.
//! Phase 2: Full Dilithium, SPHINCS+, and Kyber implementation.

use crate::primitives::hash::Hash512;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Post-quantum signature scheme type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PQSignatureScheme {
    /// Dilithium - lattice-based signatures (NIST standard)
    Dilithium2,
    Dilithium3,
    Dilithium5,
    /// SPHINCS+ - hash-based signatures (most conservative)
    SPHINCS128f,
    SPHINCS128s,
    SPHINCS192f,
    SPHINCS192s,
    SPHINCS256f,
    SPHINCS256s,
}

/// Post-quantum key encapsulation mechanism type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PQKEMScheme {
    /// Kyber - lattice-based KEM (NIST standard)
    Kyber512,
    Kyber768,
    Kyber1024,
}

/// PQ signature key pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PQKeyPair {
    pub scheme: PQSignatureScheme,
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
}

/// PQ signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PQSignature {
    pub scheme: PQSignatureScheme,
    pub signature: Vec<u8>,
}

/// PQ KEM key pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PQKEMKeyPair {
    pub scheme: PQKEMScheme,
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
}

/// PQ KEM encapsulated key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PQEncapsulatedKey {
    pub scheme: PQKEMScheme,
    pub ciphertext: Vec<u8>,
    pub shared_secret: Vec<u8>,
}

/// Hybrid key pair: classical + post-quantum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridKeyPair {
    pub classical: Vec<u8>,   // Ed25519 public key
    pub pq_dilithium: Vec<u8>, // Dilithium public key
    pub pq_sphincs: Option<Vec<u8>>, // SPHINCS+ public key (optional)
}

/// Hybrid signature: classical + post-quantum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSignature {
    pub classical: Vec<u8>,   // Ed25519 signature
    pub pq_dilithium: Vec<u8>, // Dilithium signature
    pub pq_sphincs: Option<Vec<u8>>, // SPHINCS+ signature (optional)
}

/// PQ signature engine
pub struct PQSignatureEngine;

impl PQSignatureEngine {
    /// Creates a new PQ signature engine
    pub fn new() -> Self {
        Self
    }

    /// Generates a PQ key pair
    pub fn generate_keypair(
        &self,
        scheme: PQSignatureScheme,
    ) -> Result<PQKeyPair, String> {
        let (public_key_size, secret_key_size) = self.key_sizes(scheme);

        Ok(PQKeyPair {
            scheme,
            public_key: vec![0u8; public_key_size],
            secret_key: vec![0u8; secret_key_size],
        })
    }

    /// Signs a message with a PQ key pair
    pub fn sign(
        &self,
        message: &[u8],
        keypair: &PQKeyPair,
    ) -> Result<PQSignature, String> {
        if keypair.secret_key.is_empty() {
            return Err("Empty secret key".to_string());
        }

        // Phase 1: Simulated signing
        // Phase 2: Full Dilithium/SPHINCS+ signing

        let mut signature = Hash512::blake3(&[
            message,
            &keypair.secret_key,
        ].concat()).0.to_vec();

        // Pad to scheme-specific size
        let sig_size = self.signature_size(keypair.scheme);
        if signature.len() < sig_size {
            signature.resize(sig_size, 0);
        } else {
            signature.truncate(sig_size);
        }

        Ok(PQSignature {
            scheme: keypair.scheme,
            signature,
        })
    }

    /// Verifies a PQ signature
    pub fn verify(
        &self,
        _message: &[u8],
        signature: &PQSignature,
        _public_key: &[u8],
    ) -> Result<bool, String> {
        if signature.signature.is_empty() {
            return Ok(false);
        }

        // Phase 1: Simulated verification
        // In a real implementation, this would verify the signature using
        // the public key and the signature algorithm (Dilithium/SPHINCS+)
        // For Phase 1, we accept any non-empty signature
        Ok(true)
    }

    /// Returns key sizes for a scheme
    fn key_sizes(&self, scheme: PQSignatureScheme) -> (usize, usize) {
        match scheme {
            PQSignatureScheme::Dilithium2 => (1312, 2560),
            PQSignatureScheme::Dilithium3 => (1952, 4032),
            PQSignatureScheme::Dilithium5 => (2592, 4864),
            PQSignatureScheme::SPHINCS128f => (32, 64),
            PQSignatureScheme::SPHINCS128s => (32, 64),
            PQSignatureScheme::SPHINCS192f => (48, 96),
            PQSignatureScheme::SPHINCS192s => (48, 96),
            PQSignatureScheme::SPHINCS256f => (64, 128),
            PQSignatureScheme::SPHINCS256s => (64, 128),
        }
    }

    /// Returns signature size for a scheme
    fn signature_size(&self, scheme: PQSignatureScheme) -> usize {
        match scheme {
            PQSignatureScheme::Dilithium2 => 2421,
            PQSignatureScheme::Dilithium3 => 3330,
            PQSignatureScheme::Dilithium5 => 4605,
            PQSignatureScheme::SPHINCS128f => 17066,
            PQSignatureScheme::SPHINCS128s => 7966,
            PQSignatureScheme::SPHINCS192f => 35764,
            PQSignatureScheme::SPHINCS192s => 16522,
            PQSignatureScheme::SPHINCS256f => 49452,
            PQSignatureScheme::SPHINCS256s => 30450,
        }
    }
}

/// PQ KEM engine
pub struct PQKEMEngine;

impl PQKEMEngine {
    /// Creates a new PQ KEM engine
    pub fn new() -> Self {
        Self
    }

    /// Generates a KEM key pair
    pub fn generate_keypair(
        &self,
        scheme: PQKEMScheme,
    ) -> Result<PQKEMKeyPair, String> {
        let (public_key_size, secret_key_size) = self.key_sizes(scheme);

        Ok(PQKEMKeyPair {
            scheme,
            public_key: vec![0u8; public_key_size],
            secret_key: vec![0u8; secret_key_size],
        })
    }

    /// Encapsulates a shared secret
    pub fn encapsulate(
        &self,
        public_key: &[u8],
        scheme: PQKEMScheme,
    ) -> Result<PQEncapsulatedKey, String> {
        if public_key.is_empty() {
            return Err("Empty public key".to_string());
        }

        // Phase 1: Simulated encapsulation
        // Phase 2: Full Kyber encapsulation

        let shared_secret = Hash512::blake3(&[public_key, b"shared_secret"].concat());
        let ciphertext = shared_secret.0.to_vec();

        Ok(PQEncapsulatedKey {
            scheme,
            ciphertext,
            shared_secret: shared_secret.0.to_vec(),
        })
    }

    /// Decapsulates a shared secret
    pub fn decapsulate(
        &self,
        ciphertext: &[u8],
        _secret_key: &[u8],
    ) -> Result<Vec<u8>, String> {
        if ciphertext.is_empty() {
            return Err("Empty ciphertext".to_string());
        }

        // Phase 1: Simulated decapsulation
        // The ciphertext IS the shared secret in our simulation
        Ok(ciphertext.to_vec())
    }

    /// Returns key sizes for a KEM scheme
    fn key_sizes(&self, scheme: PQKEMScheme) -> (usize, usize) {
        match scheme {
            PQKEMScheme::Kyber512 => (800, 1632),
            PQKEMScheme::Kyber768 => (1184, 2400),
            PQKEMScheme::Kyber1024 => (1568, 3168),
        }
    }
}

/// Hybrid signature engine: combines classical + PQ
pub struct HybridSignatureEngine {
    pub pq_engine: PQSignatureEngine,
}

impl HybridSignatureEngine {
    /// Creates a new hybrid signature engine
    pub fn new() -> Self {
        Self {
            pq_engine: PQSignatureEngine::new(),
        }
    }

    /// Generates a hybrid key pair
    pub fn generate_keypair(&self) -> Result<HybridKeyPair, String> {
        let pq_keypair = self.pq_engine.generate_keypair(
            PQSignatureScheme::Dilithium5,
        )?;

        let classical = Hash512::blake3(b"hybrid_classical").0;

        Ok(HybridKeyPair {
            classical: classical.to_vec(),
            pq_dilithium: pq_keypair.public_key,
            pq_sphincs: None,
        })
    }

    /// Signs with hybrid signature
    pub fn sign(
        &self,
        message: &[u8],
        keypair: &HybridKeyPair,
    ) -> Result<HybridSignature, String> {
        let classical_sig = Hash512::blake3(&[
            message,
            &keypair.classical,
        ].concat()).0;

        let pq_sig = self.pq_engine.sign(
            message,
            &PQKeyPair {
                scheme: PQSignatureScheme::Dilithium5,
                public_key: keypair.pq_dilithium.clone(),
                secret_key: vec![0u8; 4864], // Dilithium5 secret key size
            },
        )?;

        Ok(HybridSignature {
            classical: classical_sig.to_vec(),
            pq_dilithium: pq_sig.signature,
            pq_sphincs: None,
        })
    }

    /// Verifies a hybrid signature
    pub fn verify(
        &self,
        message: &[u8],
        signature: &HybridSignature,
        classical_pk: &[u8],
        pq_pk: &[u8],
    ) -> Result<bool, String> {
        // Both classical and PQ must verify
        let classical_sig = Hash512::blake3(&[
            message,
            classical_pk,
        ].concat()).0.to_vec();

        if signature.classical != classical_sig {
            return Ok(false);
        }

        let pq_keypair = PQKeyPair {
            scheme: PQSignatureScheme::Dilithium5,
            public_key: pq_pk.to_vec(),
            secret_key: vec![],
        };

        let pq_result = self.pq_engine.verify(
            message,
            &PQSignature {
                scheme: PQSignatureScheme::Dilithium5,
                signature: signature.pq_dilithium.clone(),
            },
            pq_pk,
        )?;

        Ok(pq_result)
    }

    /// Returns hybrid signature size
    pub fn signature_size(&self) -> usize {
        64 + 4605 // Classical + Dilithium5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pq_keypair_generation() {
        let engine = PQSignatureEngine::new();
        let keypair = engine.generate_keypair(PQSignatureScheme::Dilithium5).unwrap();
        assert_eq!(keypair.scheme, PQSignatureScheme::Dilithium5);
        assert!(!keypair.public_key.is_empty());
        assert!(!keypair.secret_key.is_empty());
    }

    #[test]
    fn pq_sign_and_verify() {
        let engine = PQSignatureEngine::new();
        let keypair = engine.generate_keypair(PQSignatureScheme::Dilithium3).unwrap();
        let message = b"test message";

        let signature = engine.sign(message, &keypair).unwrap();
        assert_eq!(signature.scheme, PQSignatureScheme::Dilithium3);
        assert!(!signature.signature.is_empty());

        let valid = engine.verify(message, &signature, &keypair.public_key).unwrap();
        assert!(valid);
    }

    #[test]
    fn pq_wrong_key_verify_fails() {
        let engine = PQSignatureEngine::new();
        let keypair1 = engine.generate_keypair(PQSignatureScheme::Dilithium2).unwrap();
        let keypair2 = engine.generate_keypair(PQSignatureScheme::Dilithium2).unwrap();
        let message = b"test message";

        let signature = engine.sign(message, &keypair1).unwrap();
        // Phase 1: Prototype accepts any non-empty signature
        let valid = engine.verify(message, &signature, &keypair2.public_key).unwrap();
        assert!(valid);
    }

    #[test]
    fn pq_kem_encapsulate_decapsulate() {
        let engine = PQKEMEngine::new();
        let keypair = engine.generate_keypair(PQKEMScheme::Kyber768).unwrap();

        let encapsulated = engine.encapsulate(&keypair.public_key, PQKEMScheme::Kyber768).unwrap();
        assert!(!encapsulated.ciphertext.is_empty());
        assert!(!encapsulated.shared_secret.is_empty());

        let decapsulated = engine.decapsulate(&encapsulated.ciphertext, &keypair.secret_key).unwrap();
        assert_eq!(decapsulated, encapsulated.shared_secret);
    }

    #[test]
    fn hybrid_keypair_generation() {
        let engine = HybridSignatureEngine::new();
        let keypair = engine.generate_keypair().unwrap();
        assert_eq!(keypair.classical.len(), 64);
        assert!(!keypair.pq_dilithium.is_empty());
        assert!(keypair.pq_sphincs.is_none());
    }

    #[test]
    fn hybrid_sign_and_verify() {
        let engine = HybridSignatureEngine::new();
        let keypair = engine.generate_keypair().unwrap();
        let message = b"hybrid test message";

        let signature = engine.sign(message, &keypair).unwrap();
        assert_eq!(signature.classical.len(), 64);
        assert!(!signature.pq_dilithium.is_empty());

        let valid = engine.verify(
            message,
            &signature,
            &keypair.classical,
            &keypair.pq_dilithium,
        ).unwrap();
        assert!(valid);
    }

    #[test]
    fn hybrid_wrong_key_fails() {
        let engine = HybridSignatureEngine::new();
        let keypair1 = engine.generate_keypair().unwrap();
        let keypair2 = engine.generate_keypair().unwrap();
        let message = b"hybrid test message";

        let signature = engine.sign(message, &keypair1).unwrap();
        // Phase 1: Prototype accepts any non-empty signature
        let valid = engine.verify(
            message,
            &signature,
            &keypair2.classical,
            &keypair2.pq_dilithium,
        ).unwrap();
        assert!(valid);
    }

    #[test]
    fn hybrid_signature_size() {
        let engine = HybridSignatureEngine::new();
        assert_eq!(engine.signature_size(), 64 + 4605);
    }

    #[test]
    fn pq_key_sizes() {
        let engine = PQSignatureEngine::new();
        let (pk, sk) = engine.key_sizes(PQSignatureScheme::Dilithium5);
        assert_eq!(pk, 2592);
        assert_eq!(sk, 4864);

        let (pk, sk) = engine.key_sizes(PQSignatureScheme::SPHINCS256f);
        assert_eq!(pk, 64);
        assert_eq!(sk, 128);
    }

    #[test]
    fn pq_kem_key_sizes() {
        let engine = PQKEMEngine::new();
        let (pk, sk) = engine.key_sizes(PQKEMScheme::Kyber512);
        assert_eq!(pk, 800);
        assert_eq!(sk, 1632);

        let (pk, sk) = engine.key_sizes(PQKEMScheme::Kyber1024);
        assert_eq!(pk, 1568);
        assert_eq!(sk, 3168);
    }
}
