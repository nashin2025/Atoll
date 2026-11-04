//! Address type for the Atoll Protocol.
//!
//! Addresses are 20-byte identifiers derived from public keys.
//! They use Base58Check encoding for human-readable representation.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Address type for the Atoll Protocol.
///
/// Addresses are 20-byte identifiers, similar to Ethereum addresses.
/// They are derived from the hash of a public key and use Base58Check
/// encoding for human-readable representation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub [u8; 20]);

impl Address {
    /// Returns a zero address (all bytes are 0).
    pub fn zero() -> Self {
        Self([0u8; 20])
    }

    /// Creates an Address from a 20-byte array.
    pub const fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Creates an Address from a byte slice.
    ///
    /// # Panics
    /// Panics if `bytes` is not exactly 20 bytes long.
    pub fn from_slice(bytes: &[u8]) -> Self {
        let mut result = [0u8; 20];
        result.copy_from_slice(bytes);
        Self(result)
    }

    /// Returns the inner byte array.
    pub const fn to_bytes(&self) -> [u8; 20] {
        self.0
    }

    /// Returns the address as a byte vector.
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Returns the address as a hexadecimal string with "0x" prefix.
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }

    /// Creates an Address from a hexadecimal string (with or without "0x" prefix).
    ///
    /// # Errors
    /// Returns an error if the string is not valid hex or not 40 characters (without prefix).
    pub fn from_hex(hex_str: &str) -> Result<Self, hex::FromHexError> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != 20 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut result = [0u8; 20];
        result.copy_from_slice(&bytes);
        Ok(Self(result))
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::zero()
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self.to_hex())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u8; 20]> for Address {
    fn from(bytes: [u8; 20]) -> Self {
        Self::from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_address_is_all_zeros() {
        let addr = Address::zero();
        assert_eq!(addr.0, [0u8; 20]);
    }

    #[test]
    fn from_and_to_bytes_roundtrip() {
        let data: [u8; 20] = [42u8; 20];
        let addr = Address::from_bytes(data);
        assert_eq!(addr.to_bytes(), data);
    }

    #[test]
    fn hex_roundtrip() {
        let original = Address::from_bytes([1u8; 20]);
        let hex = original.to_hex();
        let restored = Address::from_hex(&hex).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn hex_with_0x_prefix() {
        let addr = Address::from_hex("0x0102030405060708090a0b0c0d0e0f1011121314").unwrap();
        assert_eq!(addr.0[0], 0x01);
        assert_eq!(addr.0[19], 0x14);
    }

    #[test]
    fn from_slice_roundtrip() {
        let data = [7u8; 20];
        let addr = Address::from_slice(&data);
        assert_eq!(addr.to_bytes(), data);
    }
}
