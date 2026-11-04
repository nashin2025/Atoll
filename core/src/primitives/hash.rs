//! Hash primitives for the Atoll Protocol.
//!
//! Placeholder for Morph-Hash implementation.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash512(pub [u8; 64]);

impl Default for Hash512 {
    fn default() -> Self {
        Self([0u8; 64])
    }
}

impl Hash512 {
    pub fn zero() -> Self {
        Self([0u8; 64])
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut result = [0u8; 64];
        let len = bytes.len().min(64);
        result[..len].copy_from_slice(&bytes[..len]);
        Self(result)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl AsRef<[u8]> for Hash512 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
