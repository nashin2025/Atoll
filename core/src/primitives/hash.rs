//! Hash primitives for the Atoll Protocol.
//!
//! Implements Hash512 (512-bit hash) using BLAKE3 as the default
//! hash function. This is a placeholder for Morph-Hash which will
//! be implemented in Phase 2.

use std::fmt;

/// 512-bit hash value used throughout the Atoll Protocol.
///
/// Hash512 is the primary hash type for blocks, transactions,
/// addresses, and state commitments. It uses BLAKE3 as the
/// default hash function in Phase 1, transitioning to Morph-Hash
/// in Phase 2.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash512(pub [u8; 64]);

impl Hash512 {
    /// Returns a zero hash (all bytes are 0).
    pub fn zero() -> Self {
        Self([0u8; 64])
    }

    /// Creates a Hash512 from a byte slice.
    ///
    /// # Panics
    /// Panics if `bytes` is not exactly 64 bytes long.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut result = [0u8; 64];
        result.copy_from_slice(bytes);
        Self(result)
    }

    /// Creates a Hash512 from a 64-byte array.
    pub const fn from_array(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// Returns the inner byte array.
    pub const fn to_array(&self) -> [u8; 64] {
        self.0
    }

    /// Returns the hash as a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Returns the hash as a hexadecimal string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Creates a Hash512 from a hexadecimal string.
    ///
    /// # Errors
    /// Returns an error if the string is not valid hex or not 128 characters.
    pub fn from_hex(hex: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(hex)?;
        if bytes.len() != 64 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut result = [0u8; 64];
        result.copy_from_slice(&bytes);
        Ok(Self(result))
    }

    /// Computes the BLAKE3 hash of the input data (64 bytes via XOF).
    pub fn blake3(input: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(input);
        let mut out = [0u8; 64];
        hasher.finalize_xof().fill(&mut out);
        Self(out)
    }

    /// Computes the BLAKE3 hash with keyed mode (64 bytes via XOF).
    pub fn blake3_keyed(key: &[u8; 32], input: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new_keyed(key);
        hasher.update(input);
        let mut out = [0u8; 64];
        hasher.finalize_xof().fill(&mut out);
        Self(out)
    }
}

impl Default for Hash512 {
    fn default() -> Self {
        Self::zero()
    }
}

impl AsRef<[u8]> for Hash512 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Hash512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash512({})", self.to_hex())
    }
}

impl fmt::Display for Hash512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u8; 64]> for Hash512 {
    fn from(bytes: [u8; 64]) -> Self {
        Self::from_array(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_hash_is_all_zeros() {
        let h = Hash512::zero();
        assert_eq!(h.0, [0u8; 64]);
    }

    #[test]
    fn from_and_to_bytes_roundtrip() {
        let data: [u8; 64] = [42u8; 64];
        let h = Hash512::from_array(data);
        assert_eq!(h.to_array(), data);
    }

    #[test]
    fn blake3_is_deterministic() {
        let input = b"ATOLL Protocol";
        let h1 = Hash512::blake3(input);
        let h2 = Hash512::blake3(input);
        assert_eq!(h1, h2);
    }

    #[test]
    fn blake3_avalanche_effect() {
        let h1 = Hash512::blake3(b"hello");
        let h2 = Hash512::blake3(b"hellp");
        let diff_bits: u32 = h1
            .0
            .iter()
            .zip(h2.0.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        assert!(
            diff_bits > 16,
            "Avalanche effect: expected >16 bit flips, got {}",
            diff_bits
        );
    }

    #[test]
    fn hex_roundtrip() {
        let original = Hash512::blake3(b"test");
        let hex = original.to_hex();
        let restored = Hash512::from_hex(&hex).unwrap();
        assert_eq!(original, restored);
    }
}
