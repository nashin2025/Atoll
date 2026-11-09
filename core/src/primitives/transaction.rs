//! Transaction structure for the Atoll Protocol.

use crate::primitives::address::Address;
use crate::primitives::signature::Signature;
use serde::{Deserialize, Serialize};

/// Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_construction_with_recipient() {
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        let tx = Transaction {
            version: 1,
            from,
            to: Some(to),
            value: 1_000_000_000,
            data: vec![0xde, 0xad, 0xbe, 0xef],
            signature: Signature::new([0u8; 64], vec![1u8; 2000]),
        };
        assert_eq!(tx.from, from);
        assert_eq!(tx.to, Some(to));
        assert_eq!(tx.value, 1_000_000_000);
        assert_eq!(tx.data, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn transaction_without_recipient() {
        let tx = Transaction {
            version: 1,
            from: Address::zero(),
            to: None,
            value: 0,
            data: Vec::new(),
            signature: Signature::default(),
        };
        assert_eq!(tx.to, None);
        assert_eq!(tx.value, 0);
        assert!(tx.data.is_empty());
    }

    #[test]
    fn transaction_serialize_bytes_is_deterministic() {
        let tx = Transaction {
            version: 1,
            from: Address::from_bytes([5u8; 20]),
            to: Some(Address::from_bytes([6u8; 20])),
            value: 42,
            data: vec![1, 2, 3],
            signature: Signature::new([7u8; 64], vec![8u8; 100]),
        };
        let bytes1 = tx.serialize_bytes();
        let bytes2 = tx.serialize_bytes();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn transaction_serialize_bytes_layout() {
        let tx = Transaction {
            version: 1,
            from: Address::from_bytes([1u8; 20]),
            to: None,
            value: 0,
            data: Vec::new(),
            signature: Signature::default(),
        };
        let bytes = tx.serialize_bytes();
        assert_eq!(bytes[0], 1); // version
        assert_eq!(&bytes[1..21], &Address::from_bytes([1u8; 20]).0);
        assert_eq!(bytes[21], 0); // no recipient
    }

    #[test]
    fn transaction_serialize_bytes_with_recipient() {
        let tx = Transaction {
            version: 1,
            from: Address::zero(),
            to: Some(Address::from_bytes([9u8; 20])),
            value: 100,
            data: vec![],
            signature: Signature::default(),
        };
        let bytes = tx.serialize_bytes();
        assert_eq!(bytes[21], 1); // has recipient
        assert_eq!(&bytes[22..42], &Address::from_bytes([9u8; 20]).0);
    }

    #[test]
    fn transaction_large_data() {
        let large_data = vec![0u8; 10_000];
        let tx = Transaction {
            version: 1,
            from: Address::zero(),
            to: None,
            value: 0,
            data: large_data.clone(),
            signature: Signature::default(),
        };
        let bytes = tx.serialize_bytes();
        let data_len = u32::from_le_bytes(bytes[38..42].try_into().unwrap()) as usize;
        assert_eq!(data_len, 10_000);
    }

    #[test]
    fn default_transaction_values() {
        let tx = Transaction::default();
        assert_eq!(tx.version, 1);
        assert_eq!(tx.from, Address::zero());
        assert_eq!(tx.to, None);
        assert_eq!(tx.value, 0);
        assert!(tx.data.is_empty());
        assert_eq!(tx.signature.ed25519, [0u8; 64]);
        assert!(tx.signature.dilithium.is_empty());
    }
}
