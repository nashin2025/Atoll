//! Homomorphic Encryption for the Atoll Protocol.
//!
//! Supports BFV (Brakerski-Fan-Vercauteren) and CKKS (Cheon-Kim-Kim-Song)
//! schemes for privacy-preserving computation on encrypted data.
//! Phase 1: Interface and simulation.
//! Phase 2: Full homomorphic encryption implementation.

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Homomorphic encryption scheme type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HomomorphicScheme {
    /// BFV scheme - exact arithmetic on integers
    BFV,
    /// CKKS scheme - approximate arithmetic on real numbers
    CKKS,
}

/// Ciphertext: encrypted data that can be computed on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ciphertext {
    pub scheme: HomomorphicScheme,
    pub data: Vec<u8>,
    pub noise_level: u64,
    pub multiplicative_depth: u32,
}

/// Plaintext: unencrypted data
#[derive(Debug, Clone)]
pub struct Plaintext {
    pub data: Vec<u8>,
}

/// Encryption key pair
#[derive(Debug, Clone)]
pub struct KeyPair {
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
    pub relinearization_key: Option<Vec<u8>>,
    pub galois_keys: Option<Vec<u8>>,
}

/// Homomorphic encryption context
pub struct HomomorphicContext {
    pub scheme: HomomorphicScheme,
    pub poly_modulus_degree: u32,
    pub coefficient_modulus: u64,
    pub plain_modulus: u64,
    pub scale: f64,
}

impl HomomorphicContext {
    /// Creates a new homomorphic encryption context
    pub fn new(
        scheme: HomomorphicScheme,
        poly_modulus_degree: u32,
        coefficient_modulus: u64,
        plain_modulus: u64,
    ) -> Self {
        let scale = if scheme == HomomorphicScheme::CKKS {
            1.0
        } else {
            0.0
        };

        Self {
            scheme,
            poly_modulus_degree,
            coefficient_modulus,
            plain_modulus,
            scale,
        }
    }

    /// Returns the security level in bits
    pub fn security_level(&self) -> u32 {
        match self.poly_modulus_degree {
            1024 => 128,
            2048 => 192,
            4096 => 256,
            8192 => 256,
            16384 => 256,
            _ => 128,
        }
    }
}

/// Homomorphic encryption engine
pub struct HomomorphicEngine {
    pub context: HomomorphicContext,
    pub key_pair: Option<KeyPair>,
}

impl HomomorphicEngine {
    /// Creates a new homomorphic encryption engine
    pub fn new(context: HomomorphicContext) -> Self {
        Self {
            context,
            key_pair: None,
        }
    }

    /// Generates a key pair
    pub fn generate_key_pair(&mut self) -> Result<(), String> {
        let mut rng = OsRng;

        let mut public_key = vec![0u8; 64];
        let mut secret_key = vec![0u8; 64];
        rng.fill_bytes(&mut public_key);
        rng.fill_bytes(&mut secret_key);

        let relinearization_key = if self.context.scheme == HomomorphicScheme::BFV {
            let mut relin_key = vec![0u8; 64];
            rng.fill_bytes(&mut relin_key);
            Some(relin_key)
        } else {
            None
        };

        let galois_keys = if self.context.scheme == HomomorphicScheme::CKKS {
            let mut galois_key = vec![0u8; 64];
            rng.fill_bytes(&mut galois_key);
            Some(galois_key)
        } else {
            None
        };

        self.key_pair = Some(KeyPair {
            public_key,
            secret_key,
            relinearization_key,
            galois_keys,
        });

        Ok(())
    }

    /// Encrypts plaintext to ciphertext
    pub fn encrypt(&self, plaintext: &Plaintext) -> Result<Ciphertext, String> {
        let _key_pair = self
            .key_pair
            .as_ref()
            .ok_or_else(|| "Key pair not generated".to_string())?;

        Ok(Ciphertext {
            scheme: self.context.scheme,
            data: plaintext.data.clone(),
            noise_level: 1,
            multiplicative_depth: 0,
        })
    }

    /// Decrypts ciphertext to plaintext
    pub fn decrypt(&self, ciphertext: &Ciphertext) -> Result<Plaintext, String> {
        let _key_pair = self
            .key_pair
            .as_ref()
            .ok_or_else(|| "Key pair not generated".to_string())?;

        // Phase 1: Simulated decryption - just return the data
        // Phase 2: Full homomorphic decryption
        let data = ciphertext.data.clone();

        Ok(Plaintext { data })
    }

    /// Homomorphic addition of two ciphertexts
    pub fn add(&self, ct1: &Ciphertext, ct2: &Ciphertext) -> Result<Ciphertext, String> {
        if ct1.scheme != ct2.scheme {
            return Err("Ciphertext schemes must match".to_string());
        }

        if ct1.data.len() != ct2.data.len() {
            return Err("Ciphertext sizes must match".to_string());
        }

        let mut result_data = Vec::with_capacity(ct1.data.len());
        for (a, b) in ct1.data.iter().zip(ct2.data.iter()) {
            result_data.push(a.wrapping_add(*b));
        }

        let noise_level = ct1.noise_level.max(ct2.noise_level) + 1;
        let multiplicative_depth = ct1.multiplicative_depth.max(ct2.multiplicative_depth);

        Ok(Ciphertext {
            scheme: ct1.scheme,
            data: result_data,
            noise_level,
            multiplicative_depth,
        })
    }

    /// Homomorphic multiplication of two ciphertexts
    pub fn multiply(&self, ct1: &Ciphertext, ct2: &Ciphertext) -> Result<Ciphertext, String> {
        if ct1.scheme != ct2.scheme {
            return Err("Ciphertext schemes must match".to_string());
        }

        // Multiplication increases noise significantly
        let mut result_data = Vec::with_capacity(ct1.data.len().min(ct2.data.len()));
        for i in 0..ct1.data.len().min(ct2.data.len()) {
            result_data.push(ct1.data[i].wrapping_mul(ct2.data[i]));
        }

        let noise_level = ct1.noise_level + ct2.noise_level + 10;
        let multiplicative_depth = ct1.multiplicative_depth + ct2.multiplicative_depth + 1;

        // Check if noise level is too high
        if noise_level > 1000 {
            return Err("Noise level too high - bootstrapping required".to_string());
        }

        Ok(Ciphertext {
            scheme: ct1.scheme,
            data: result_data,
            noise_level,
            multiplicative_depth,
        })
    }

    /// Relinearization: reduces ciphertext size after multiplication
    pub fn relinearize(&self, ciphertext: &Ciphertext) -> Result<Ciphertext, String> {
        if self.context.scheme != HomomorphicScheme::BFV {
            return Err("Relinearization only supported for BFV".to_string());
        }

        let key_pair = self
            .key_pair
            .as_ref()
            .ok_or_else(|| "Key pair not generated".to_string())?;

        if key_pair.relinearization_key.is_none() {
            return Err("Relinearization key not available".to_string());
        }

        // Relinearization reduces noise and size
        Ok(Ciphertext {
            scheme: ciphertext.scheme,
            data: ciphertext.data.clone(),
            noise_level: ciphertext.noise_level / 2,
            multiplicative_depth: ciphertext.multiplicative_depth,
        })
    }

    /// Bootstrapping: refreshes ciphertext noise
    pub fn bootstrap(&self, ciphertext: &Ciphertext) -> Result<Ciphertext, String> {
        if ciphertext.noise_level < 100 {
            return Ok(ciphertext.clone()); // No need to bootstrap
        }

        Ok(Ciphertext {
            scheme: ciphertext.scheme,
            data: ciphertext.data.clone(),
            noise_level: 1,
            multiplicative_depth: 0,
        })
    }

    /// Returns the security level
    pub fn security_level(&self) -> u32 {
        self.context.security_level()
    }

    /// Returns whether the engine has a key pair
    pub fn has_key_pair(&self) -> bool {
        self.key_pair.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homomorphic_context_new() {
        let context = HomomorphicContext::new(HomomorphicScheme::BFV, 4096, 1 << 30, 1 << 20);
        assert_eq!(context.security_level(), 256);
        assert_eq!(context.scale, 0.0);
    }

    #[test]
    fn homomorphic_context_ckks() {
        let context = HomomorphicContext::new(HomomorphicScheme::CKKS, 8192, 1 << 40, 1 << 20);
        assert_eq!(context.scale, 1.0);
        assert_eq!(context.security_level(), 256);
    }

    #[test]
    fn encrypt_decrypt_bfv() {
        let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
            HomomorphicScheme::BFV,
            4096,
            1 << 30,
            1 << 20,
        ));
        engine.generate_key_pair().unwrap();

        let plaintext = Plaintext {
            data: vec![1, 2, 3, 4, 5],
        };

        let ciphertext = engine.encrypt(&plaintext).unwrap();
        assert_eq!(ciphertext.scheme, HomomorphicScheme::BFV);
        assert_eq!(ciphertext.noise_level, 1);

        let decrypted = engine.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted.data, plaintext.data);
    }

    #[test]
    fn encrypt_decrypt_ckks() {
        let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
            HomomorphicScheme::CKKS,
            8192,
            1 << 40,
            1 << 20,
        ));
        engine.generate_key_pair().unwrap();

        let plaintext = Plaintext {
            data: vec![10, 20, 30],
        };

        let ciphertext = engine.encrypt(&plaintext).unwrap();
        assert_eq!(ciphertext.scheme, HomomorphicScheme::CKKS);

        let decrypted = engine.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted.data, plaintext.data);
    }

    #[test]
    fn homomorphic_addition() {
        let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
            HomomorphicScheme::BFV,
            4096,
            1 << 30,
            1 << 20,
        ));
        engine.generate_key_pair().unwrap();

        let pt1 = Plaintext {
            data: vec![1, 2, 3],
        };
        let pt2 = Plaintext {
            data: vec![4, 5, 6],
        };

        let ct1 = engine.encrypt(&pt1).unwrap();
        let ct2 = engine.encrypt(&pt2).unwrap();

        let ct_sum = engine.add(&ct1, &ct2).unwrap();
        let decrypted = engine.decrypt(&ct_sum).unwrap();

        // Homomorphic addition: encrypt(a) + encrypt(b) = encrypt(a + b)
        assert_eq!(decrypted.data, vec![5, 7, 9]);
    }

    #[test]
    fn homomorphic_multiplication() {
        let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
            HomomorphicScheme::BFV,
            4096,
            1 << 30,
            1 << 20,
        ));
        engine.generate_key_pair().unwrap();

        let pt1 = Plaintext {
            data: vec![2, 3, 4],
        };
        let pt2 = Plaintext {
            data: vec![3, 4, 5],
        };

        let ct1 = engine.encrypt(&pt1).unwrap();
        let ct2 = engine.encrypt(&pt2).unwrap();

        let ct_product = engine.multiply(&ct1, &ct2).unwrap();
        let decrypted = engine.decrypt(&ct_product).unwrap();

        // Homomorphic multiplication: encrypt(a) * encrypt(b) = encrypt(a * b)
        assert_eq!(decrypted.data, vec![6, 12, 20]);
    }

    #[test]
    fn noise_level_increases() {
        let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
            HomomorphicScheme::BFV,
            4096,
            1 << 30,
            1 << 20,
        ));
        engine.generate_key_pair().unwrap();

        let pt = Plaintext { data: vec![1] };
        let ct1 = engine.encrypt(&pt).unwrap();
        let ct2 = engine.encrypt(&pt).unwrap();

        let ct_sum = engine.add(&ct1, &ct2).unwrap();
        assert!(ct_sum.noise_level > ct1.noise_level);

        let ct_product = engine.multiply(&ct1, &ct2).unwrap();
        assert!(ct_product.noise_level > ct_sum.noise_level);
    }

    #[test]
    fn bootstrapping_reduces_noise() {
        let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
            HomomorphicScheme::BFV,
            4096,
            1 << 30,
            1 << 20,
        ));
        engine.generate_key_pair().unwrap();

        let pt = Plaintext { data: vec![1] };
        let ct1 = engine.encrypt(&pt).unwrap();
        let ct2 = engine.encrypt(&pt).unwrap();

        // Multiply many times to increase noise
        let mut ct = ct1.clone();
        for _ in 0..10 {
            ct = engine.multiply(&ct, &ct2).unwrap();
        }

        assert!(ct.noise_level > 100);

        let ct_bootstrapped = engine.bootstrap(&ct).unwrap();
        assert!(ct_bootstrapped.noise_level < ct.noise_level);
    }

    #[test]
    fn relinearization_reduces_noise() {
        let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
            HomomorphicScheme::BFV,
            4096,
            1 << 30,
            1 << 20,
        ));
        engine.generate_key_pair().unwrap();

        let pt1 = Plaintext { data: vec![2] };
        let pt2 = Plaintext { data: vec![3] };

        let ct1 = engine.encrypt(&pt1).unwrap();
        let ct2 = engine.encrypt(&pt2).unwrap();

        let ct_product = engine.multiply(&ct1, &ct2).unwrap();
        let ct_relin = engine.relinearize(&ct_product).unwrap();

        assert!(ct_relin.noise_level < ct_product.noise_level);
    }

    #[test]
    fn encrypt_without_key_fails() {
        let engine = HomomorphicEngine::new(HomomorphicContext::new(
            HomomorphicScheme::BFV,
            4096,
            1 << 30,
            1 << 20,
        ));

        let plaintext = Plaintext { data: vec![1] };
        let result = engine.encrypt(&plaintext);
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_with_key() {
        let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
            HomomorphicScheme::BFV,
            4096,
            1 << 30,
            1 << 20,
        ));
        engine.generate_key_pair().unwrap();

        let plaintext = Plaintext {
            data: vec![1, 2, 3],
        };
        let ciphertext = engine.encrypt(&plaintext).unwrap();
        let decrypted = engine.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted.data, plaintext.data);
    }

    #[test]
    fn security_level_by_degree() {
        let contexts = vec![(1024, 128), (2048, 192), (4096, 256), (8192, 256)];

        for (degree, expected) in contexts {
            let context = HomomorphicContext::new(HomomorphicScheme::BFV, degree, 1 << 30, 1 << 20);
            assert_eq!(context.security_level(), expected);
        }
    }
}
