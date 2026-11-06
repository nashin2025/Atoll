//! Signature types for the Atoll Protocol.
//!
//! Supports hybrid signatures: Ed25519 (classical, fast) + Dilithium
//! (post-quantum). Both must verify for a hybrid signature to be valid.

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serializer};
use std::fmt;

/// Hybrid signature combining classical and post-quantum algorithms.
///
/// Both the Ed25519 and Dilithium signatures must verify for the
/// hybrid signature to be considered valid. This provides defense
/// in depth against both classical and quantum attacks.
#[derive(Clone, PartialEq, Eq)]
pub struct Signature {
    /// Classical Ed25519 signature (64 bytes, fast verification)
    pub ed25519: [u8; 64],
    /// Post-quantum Dilithium signature (Level 5, ~2500-4600 bytes)
    pub dilithium: Vec<u8>,
}

impl serde::Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Signature", 2)?;
        state.serialize_field("ed25519", &hex::encode(&self.ed25519))?;
        state.serialize_field("dilithium", &hex::encode(&self.dilithium))?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SignatureHelper {
            ed25519: String,
            dilithium: String,
        }

        let helper = SignatureHelper::deserialize(deserializer)?;
        let ed25519_bytes = hex::decode(&helper.ed25519).map_err(de::Error::custom)?;
        let byte_len = ed25519_bytes.len();
        let ed25519: [u8; 64] = ed25519_bytes
            .try_into()
            .map_err(|_| de::Error::invalid_length(byte_len, &"expected 64 bytes"))?;
        let dilithium = hex::decode(&helper.dilithium).map_err(de::Error::custom)?;

        Ok(Signature { ed25519, dilithium })
    }
}

impl Signature {
    /// Creates a new hybrid signature.
    pub fn new(ed25519: [u8; 64], dilithium: Vec<u8>) -> Self {
        Self { ed25519, dilithium }
    }

    /// Returns the length of the Dilithium signature in bytes.
    pub fn dilithium_len(&self) -> usize {
        self.dilithium.len()
    }

    /// Returns the total signature size in bytes.
    pub fn total_size(&self) -> usize {
        64 + self.dilithium.len()
    }
}

impl Default for Signature {
    fn default() -> Self {
        Self {
            ed25519: [0u8; 64],
            dilithium: Vec::new(),
        }
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signature")
            .field("ed25519", &hex::encode(self.ed25519))
            .field("dilithium_len", &self.dilithium.len())
            .field("total_size", &self.total_size())
            .finish()
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Signature(ed25519={}, dilithium={} bytes)",
            hex::encode(&self.ed25519[..8]),
            self.dilithium.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_signature_is_empty() {
        let sig = Signature::default();
        assert_eq!(sig.ed25519, [0u8; 64]);
        assert!(sig.dilithium.is_empty());
    }

    #[test]
    fn new_signature_stores_values() {
        let ed25519 = [1u8; 64];
        let dilithium = vec![2u8; 2000];
        let sig = Signature::new(ed25519, dilithium.clone());
        assert_eq!(sig.ed25519, [1u8; 64]);
        assert_eq!(sig.dilithium, dilithium);
    }

    #[test]
    fn total_size_calculation() {
        let sig = Signature::new([0u8; 64], vec![0u8; 100]);
        assert_eq!(sig.total_size(), 164);
    }
}
