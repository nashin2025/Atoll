//! Block structure for the Atoll Protocol.
//!
//! Defines Block, BlockHeader, and BlockBody with serialization support.

use crate::primitives::hash::Hash512;
use crate::primitives::address::Address;
use crate::primitives::transaction::Transaction;
use crate::primitives::signature::Signature;

/// Block header containing metadata about a block
#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub version: u32,
    pub height: u64,
    pub shard_id: u16,
    pub timestamp: u64,
    pub prev_hash: Hash512,
    pub state_root: Hash512,
    pub transaction_root: Hash512,
    pub proposer: Address,
    pub difficulty: u128,
}

/// Block body containing transactions
#[derive(Debug, Clone)]
pub struct BlockBody {
    pub transactions: Vec<crate::primitives::transaction::Transaction>,
}

/// Complete block with header and body
#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub body: BlockBody,
}

impl Block {
    /// Serializes the block to bytes for hashing
    pub fn serialize_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Header
        buf.extend_from_slice(&self.header.version.to_le_bytes());
        buf.extend_from_slice(&self.header.height.to_le_bytes());
        buf.extend_from_slice(&self.header.shard_id.to_le_bytes());
        buf.extend_from_slice(&self.header.timestamp.to_le_bytes());
        buf.extend_from_slice(&self.header.prev_hash.0);
        buf.extend_from_slice(&self.header.state_root.0);
        buf.extend_from_slice(&self.header.transaction_root.0);
        buf.extend_from_slice(&self.header.proposer.0);
        buf.extend_from_slice(&self.header.difficulty.to_le_bytes());

        // Body
        buf.extend_from_slice(&(self.body.transactions.len() as u32).to_le_bytes());
        for tx in &self.body.transactions {
            let tx_bytes = tx.serialize_bytes();
            buf.extend_from_slice(&(tx_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(&tx_bytes);
        }

        buf
    }
}

impl Default for Block {
    fn default() -> Self {
        Self {
            header: BlockHeader {
                version: 1,
                height: 0,
                shard_id: 0,
                timestamp: 0,
                prev_hash: Hash512::zero(),
                state_root: Hash512::zero(),
                transaction_root: Hash512::zero(),
                proposer: Address::zero(),
                difficulty: 1,
            },
            body: BlockBody {
                transactions: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::signature::Signature;
    use crate::primitives::transaction::Transaction;

    #[test]
    fn block_header_construction() {
        let header = BlockHeader {
            version: 1,
            height: 42,
            shard_id: 1,
            timestamp: 1798761600,
            prev_hash: Hash512::from_bytes(&[1u8; 64]),
            state_root: Hash512::from_bytes(&[2u8; 64]),
            transaction_root: Hash512::from_bytes(&[3u8; 64]),
            proposer: Address::from_bytes([4u8; 20]),
            difficulty: 1 << 20,
        };
        assert_eq!(header.height, 42);
        assert_eq!(header.shard_id, 1);
        assert_eq!(header.version, 1);
        assert_eq!(header.difficulty, 1 << 20);
    }

    #[test]
    fn block_body_holds_transactions() {
        let tx = Transaction {
            version: 1,
            from: Address::from_bytes([1u8; 20]),
            to: Some(Address::from_bytes([2u8; 20])),
            value: 100,
            data: vec![1, 2, 3],
            signature: Signature::default(),
        };
        let body = BlockBody {
            transactions: vec![tx.clone()],
        };
        assert_eq!(body.transactions.len(), 1);
        assert_eq!(body.transactions[0].value, 100);
    }

    #[test]
    fn block_serialize_bytes_is_deterministic() {
        let block = Block::default();
        let bytes1 = block.serialize_bytes();
        let bytes2 = block.serialize_bytes();
        assert_eq!(bytes1, bytes2);
        assert!(!bytes1.is_empty());
    }

    #[test]
    fn block_serialize_bytes_includes_transaction_count() {
        let tx1 = Transaction {
            version: 1,
            from: Address::from_bytes([1u8; 20]),
            to: None,
            value: 50,
            data: vec![],
            signature: Signature::default(),
        };
        let tx2 = Transaction {
            version: 1,
            from: Address::from_bytes([2u8; 20]),
            to: Some(Address::from_bytes([3u8; 20])),
            value: 75,
            data: vec![9],
            signature: Signature::default(),
        };
        let block = Block {
            header: BlockHeader {
                version: 1,
                height: 0,
                shard_id: 0,
                timestamp: 0,
                prev_hash: Hash512::zero(),
                state_root: Hash512::zero(),
                transaction_root: Hash512::zero(),
                proposer: Address::zero(),
                difficulty: 1,
            },
            body: BlockBody {
                transactions: vec![tx1, tx2],
            },
        };
        let bytes = block.serialize_bytes();
        // Header: version(4) + height(8) + shard_id(2) + timestamp(8) + prev_hash(64)
        // + state_root(64) + transaction_root(64) + proposer(20) + difficulty(16) = 250
        // Transaction count at offset 250
        let count = u32::from_le_bytes(bytes[250..254].try_into().unwrap());
        assert_eq!(count, 2);
    }

    #[test]
    fn default_block_has_genesis_values() {
        let block = Block::default();
        assert_eq!(block.header.height, 0);
        assert_eq!(block.header.prev_hash, Hash512::zero());
        assert_eq!(block.header.state_root, Hash512::zero());
        assert_eq!(block.header.transaction_root, Hash512::zero());
        assert_eq!(block.header.proposer, Address::zero());
        assert!(block.body.transactions.is_empty());
    }

    #[test]
    fn block_with_transactions_serializes_with_count() {
        let tx1 = Transaction {
            version: 1,
            from: Address::from_bytes([1u8; 20]),
            to: None,
            value: 50,
            data: vec![],
            signature: Signature::default(),
        };
        let tx2 = Transaction {
            version: 1,
            from: Address::from_bytes([2u8; 20]),
            to: Some(Address::from_bytes([3u8; 20])),
            value: 75,
            data: vec![9],
            signature: Signature::default(),
        };
        let block = Block {
            header: BlockHeader {
                version: 1,
                height: 0,
                shard_id: 0,
                timestamp: 0,
                prev_hash: Hash512::zero(),
                state_root: Hash512::zero(),
                transaction_root: Hash512::zero(),
                proposer: Address::zero(),
                difficulty: 1,
            },
            body: BlockBody {
                transactions: vec![tx1, tx2],
            },
        };
        let bytes = block.serialize_bytes();
        // Header is 250 bytes, then 4-byte transaction count at offset 250
        let count = u32::from_le_bytes(bytes[250..254].try_into().unwrap());
        assert_eq!(count, 2);
        // Block with transactions should be larger than empty block
        let empty_block = Block::default();
        assert!(bytes.len() > empty_block.serialize_bytes().len());
    }
}
