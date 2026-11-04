//! Transaction structure for the Atoll Protocol.

use crate::primitives::address::Address;
use crate::primitives::signature::Signature;

/// Transaction structure
#[derive(Debug, Clone)]
pub struct Transaction {
    pub version: u8,
    pub from: Address,
    pub to: Option<Address>,
    pub value: u128,
    pub data: Vec<u8>,
    pub signature: Signature,
}

impl Transaction {
    /// Serializes the transaction to bytes for hashing
    pub fn serialize_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.push(self.version);
        buf.extend_from_slice(&self.from.0);
        match &self.to {
            Some(to) => {
                buf.push(1);
                buf.extend_from_slice(&to.0);
            }
            None => {
                buf.push(0);
            }
        }
        buf.extend_from_slice(&self.value.to_le_bytes());
        buf.extend_from_slice(&(self.data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.data);
        buf.extend_from_slice(&self.signature.ed25519);
        buf.extend_from_slice(&(self.signature.dilithium.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.signature.dilithium);

        buf
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            version: 1,
            from: Address::zero(),
            to: None,
            value: 0,
            data: Vec::new(),
            signature: Signature::default(),
        }
    }
}
