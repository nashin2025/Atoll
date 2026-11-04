//! Address type for the Atoll Protocol.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address(pub [u8; 20]);

impl Address {
    pub fn zero() -> Self {
        Self([0u8; 20])
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::zero()
    }
}
