//! Block structure for the Atoll Protocol.
//!
//! Defines Block, BlockHeader, and BlockBody with serialization support.

use crate::primitives::hash::Hash512;
use crate::primitives::address::Address;

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
