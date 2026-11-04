//! Network module for the Atoll Protocol.
//!
//! P2P networking, gossip, sync, discovery, and encrypted mempool.
//!
//! Uses tokio for async I/O, curve25519-dalek for Diffie-Hellman key exchange,
//! and chacha20poly1305 for symmetric encryption.

pub mod p2p;
pub mod peer_manager;
pub mod gossip;
pub mod sync;
pub mod discovery;
pub mod encrypted_mempool;

use crate::primitives::hash::Hash512;
use crate::primitives::address::Address;

// ---------------------------------------------------------------------------
// Network Message Types
// ---------------------------------------------------------------------------

/// All network messages in the Atoll Protocol
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    // Discovery
    Ping(PingMessage),
    Pong(PongMessage),
    FindNode(FindNodeMessage),
    Neighbors(NeighborsMessage),

    // Block propagation
    NewBlock(Vec<u8>),           // Serialized block
    NewBlockHash(Hash512),
    GetBlock(Hash512),
    BlockResponse(Vec<u8>),      // Serialized block

    // Transaction propagation
    NewTransaction(Vec<u8>),     // Serialized transaction
    NewTransactionBatch(Vec<Vec<u8>>),

    // Consensus votes
    BFTProposal(BFTVote),
    BFTVote(BFTVote),
    BFTCommit(BFTVote),

    // Sync
    GetHeaders(GetHeadersMessage),
    Headers(Vec<BlockHeader>),
    GetBodies(Vec<Hash512>),
    Bodies(Vec<Vec<u8>>),        // Serialized block bodies
    GetState(StateRequest),
    StateResponse(StateData),

    // Cross-shard
    CrossShardReceipt(Vec<u8>),  // Serialized receipt
    CrossShardRequest(Vec<u8>),  // Serialized request

    // Encrypted mempool
    EncryptedTransaction(EncryptedTx),
    ThresholdDecryptionShare(DecryptionShare),
}

/// Ping message for peer discovery
#[derive(Debug, Clone)]
pub struct PingMessage {
    pub nonce: u64,
    pub timestamp: u64,
}

/// Pong response to ping
#[derive(Debug, Clone)]
pub struct PongMessage {
    pub nonce: u64,
    pub timestamp: u64,
}

/// FindNode request for peer discovery
#[derive(Debug, Clone)]
pub struct FindNodeMessage {
    pub target_node_id: [u8; 32],
}

/// Neighbors response with peer addresses
#[derive(Debug, Clone)]
pub struct NeighborsMessage {
    pub peers: Vec<PeerInfo>,
}

/// Headers request for sync
#[derive(Debug, Clone)]
pub struct GetHeadersMessage {
    pub start_height: u64,
    pub max_count: u64,
}

/// Block header (simplified for network layer)
#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub height: u64,
    pub hash: Hash512,
    pub prev_hash: Hash512,
    pub timestamp: u64,
}

/// State request for sync
#[derive(Debug, Clone)]
pub struct StateRequest {
    pub state_root: Hash512,
}

/// State response for sync
#[derive(Debug, Clone)]
pub struct StateData {
    pub state_root: Hash512,
    pub data: Vec<u8>,
}

/// Encrypted transaction for MEV-resistant mempool
#[derive(Debug, Clone)]
pub struct EncryptedTx {
    pub ciphertext: Vec<u8>,
    pub order_commitment: Hash512,
    pub fee_commitment: Hash512,
    pub submitter_proof: Vec<u8>,
}

/// Threshold decryption share for encrypted mempool
#[derive(Debug, Clone)]
pub struct DecryptionShare {
    pub tx_hash: Hash512,
    pub share: Vec<u8>,
    pub validator: Address,
}

/// BFT vote (simplified for network layer)
#[derive(Debug, Clone)]
pub struct BFTVote {
    pub validator: Address,
    pub block_hash: Hash512,
    pub height: u64,
    pub round: u64,
    pub vote_type: u8, // 0=Prepare, 1=PreCommit, 2=Commit
    pub signature: Vec<u8>,
}

/// Peer information for discovery and connection
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub node_id: [u8; 32],
    pub address: String,
    pub port: u16,
    pub last_seen: u64,
}
