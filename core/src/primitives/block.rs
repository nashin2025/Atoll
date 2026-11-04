//! Block structure for the Atoll Protocol.

use crate::primitives::hash::Hash512;

pub struct BlockHeader {
    pub version: u32,
    pub height: u64,
    pub shard_id: u16,
    pub timestamp: u64,
    pub prev_hash: Hash512,
    pub state_root: Hash512,
    pub transaction_root: Hash512,
    pub proposer: crate::primitives::address::Address,
    pub difficulty: u128,
}

pub struct BlockBody {
    pub transactions: Vec<crate::primitives::transaction::Transaction>,
}

pub struct Block {
    pub header: BlockHeader,
    pub body: BlockBody,
}
