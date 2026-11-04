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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_message_ping_variant() {
        let msg = NetworkMessage::Ping(PingMessage {
            nonce: 42,
            timestamp: 1000,
        });
        assert!(matches!(msg, NetworkMessage::Ping(_)));
    }

    #[test]
    fn network_message_pong_variant() {
        let msg = NetworkMessage::Pong(PongMessage {
            nonce: 99,
            timestamp: 2000,
        });
        assert!(matches!(msg, NetworkMessage::Pong(_)));
    }

    #[test]
    fn network_message_find_node_variant() {
        let msg = NetworkMessage::FindNode(FindNodeMessage {
            target_node_id: [7u8; 32],
        });
        assert!(matches!(msg, NetworkMessage::FindNode(_)));
    }

    #[test]
    fn network_message_neighbors_variant() {
        let msg = NetworkMessage::Neighbors(NeighborsMessage {
            peers: vec![PeerInfo {
                node_id: [1u8; 32],
                address: "127.0.0.1".to_string(),
                port: 30333,
                last_seen: 0,
            }],
        });
        assert!(matches!(msg, NetworkMessage::Neighbors(_)));
    }

    #[test]
    fn network_message_new_block_variant() {
        let msg = NetworkMessage::NewBlock(vec![1, 2, 3]);
        assert!(matches!(msg, NetworkMessage::NewBlock(_)));
    }

    #[test]
    fn network_message_new_block_hash_variant() {
        let msg = NetworkMessage::NewBlockHash(Hash512::zero());
        assert!(matches!(msg, NetworkMessage::NewBlockHash(_)));
    }

    #[test]
    fn network_message_get_block_variant() {
        let msg = NetworkMessage::GetBlock(Hash512::from_bytes(&[5u8; 64]));
        assert!(matches!(msg, NetworkMessage::GetBlock(_)));
    }

    #[test]
    fn network_message_block_response_variant() {
        let msg = NetworkMessage::BlockResponse(vec![0, 1]);
        assert!(matches!(msg, NetworkMessage::BlockResponse(_)));
    }

    #[test]
    fn network_message_new_transaction_variant() {
        let msg = NetworkMessage::NewTransaction(vec![9]);
        assert!(matches!(msg, NetworkMessage::NewTransaction(_)));
    }

    #[test]
    fn network_message_new_transaction_batch_variant() {
        let msg = NetworkMessage::NewTransactionBatch(vec![vec![1], vec![2]]);
        assert!(matches!(msg, NetworkMessage::NewTransactionBatch(_)));
    }

    #[test]
    fn network_message_bft_proposal_variant() {
        let msg = NetworkMessage::BFTProposal(crate::network::BFTVote {
            validator: Address::zero(),
            block_hash: Hash512::zero(),
            height: 1,
            round: 1,
            vote_type: 0,
            signature: vec![],
        });
        assert!(matches!(msg, NetworkMessage::BFTProposal(_)));
    }

    #[test]
    fn network_message_bft_vote_variant() {
        let msg = NetworkMessage::BFTVote(crate::network::BFTVote {
            validator: Address::zero(),
            block_hash: Hash512::zero(),
            height: 1,
            round: 1,
            vote_type: 1,
            signature: vec![],
        });
        assert!(matches!(msg, NetworkMessage::BFTVote(_)));
    }

    #[test]
    fn network_message_bft_commit_variant() {
        let msg = NetworkMessage::BFTCommit(crate::network::BFTVote {
            validator: Address::zero(),
            block_hash: Hash512::zero(),
            height: 1,
            round: 1,
            vote_type: 2,
            signature: vec![],
        });
        assert!(matches!(msg, NetworkMessage::BFTCommit(_)));
    }

    #[test]
    fn network_message_get_headers_variant() {
        let msg = NetworkMessage::GetHeaders(GetHeadersMessage {
            start_height: 10,
            max_count: 100,
        });
        assert!(matches!(msg, NetworkMessage::GetHeaders(_)));
    }

    #[test]
    fn network_message_headers_variant() {
        let msg = NetworkMessage::Headers(vec![BlockHeader {
            height: 1,
            hash: Hash512::zero(),
            prev_hash: Hash512::zero(),
            timestamp: 1000,
        }]);
        assert!(matches!(msg, NetworkMessage::Headers(_)));
    }

    #[test]
    fn network_message_get_bodies_variant() {
        let msg = NetworkMessage::GetBodies(vec![Hash512::zero()]);
        assert!(matches!(msg, NetworkMessage::GetBodies(_)));
    }

    #[test]
    fn network_message_bodies_variant() {
        let msg = NetworkMessage::Bodies(vec![vec![1], vec![2]]);
        assert!(matches!(msg, NetworkMessage::Bodies(_)));
    }

    #[test]
    fn network_message_get_state_variant() {
        let msg = NetworkMessage::GetState(StateRequest {
            state_root: Hash512::zero(),
        });
        assert!(matches!(msg, NetworkMessage::GetState(_)));
    }

    #[test]
    fn network_message_state_response_variant() {
        let msg = NetworkMessage::StateResponse(StateData {
            state_root: Hash512::zero(),
            data: vec![],
        });
        assert!(matches!(msg, NetworkMessage::StateResponse(_)));
    }

    #[test]
    fn network_message_cross_shard_receipt_variant() {
        let msg = NetworkMessage::CrossShardReceipt(vec![1]);
        assert!(matches!(msg, NetworkMessage::CrossShardReceipt(_)));
    }

    #[test]
    fn network_message_cross_shard_request_variant() {
        let msg = NetworkMessage::CrossShardRequest(vec![2]);
        assert!(matches!(msg, NetworkMessage::CrossShardRequest(_)));
    }

    #[test]
    fn network_message_encrypted_transaction_variant() {
        let msg = NetworkMessage::EncryptedTransaction(crate::network::EncryptedTx {
            ciphertext: vec![3],
            order_commitment: Hash512::zero(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
        });
        assert!(matches!(msg, NetworkMessage::EncryptedTransaction(_)));
    }

    #[test]
    fn network_message_threshold_decryption_share_variant() {
        let msg = NetworkMessage::ThresholdDecryptionShare(crate::network::DecryptionShare {
            tx_hash: Hash512::zero(),
            share: vec![4],
            validator: Address::zero(),
        });
        assert!(matches!(msg, NetworkMessage::ThresholdDecryptionShare(_)));
    }

    #[test]
    fn ping_message_construction() {
        let ping = PingMessage {
            nonce: 1,
            timestamp: 2,
        };
        assert_eq!(ping.nonce, 1);
        assert_eq!(ping.timestamp, 2);
    }

    #[test]
    fn pong_message_construction() {
        let pong = PongMessage {
            nonce: 1,
            timestamp: 2,
        };
        assert_eq!(pong.nonce, 1);
        assert_eq!(pong.timestamp, 2);
    }

    #[test]
    fn find_node_message_construction() {
        let find = FindNodeMessage {
            target_node_id: [3u8; 32],
        };
        assert_eq!(find.target_node_id, [3u8; 32]);
    }

    #[test]
    fn neighbors_message_construction() {
        let neighbors = NeighborsMessage {
            peers: vec![],
        };
        assert!(neighbors.peers.is_empty());
    }

    #[test]
    fn get_headers_message_construction() {
        let req = GetHeadersMessage {
            start_height: 0,
            max_count: 50,
        };
        assert_eq!(req.start_height, 0);
        assert_eq!(req.max_count, 50);
    }

    #[test]
    fn network_block_header_construction() {
        let header = BlockHeader {
            height: 100,
            hash: Hash512::from_bytes(&[1u8; 64]),
            prev_hash: Hash512::from_bytes(&[2u8; 64]),
            timestamp: 1798761600,
        };
        assert_eq!(header.height, 100);
        assert_eq!(header.timestamp, 1798761600);
    }

    #[test]
    fn state_request_construction() {
        let req = StateRequest {
            state_root: Hash512::from_bytes(&[3u8; 64]),
        };
        assert_eq!(req.state_root.to_array().len(), 64);
    }

    #[test]
    fn state_data_construction() {
        let data = StateData {
            state_root: Hash512::zero(),
            data: vec![1, 2, 3],
        };
        assert_eq!(data.data, vec![1, 2, 3]);
    }

    #[test]
    fn encrypted_tx_construction() {
        let tx = EncryptedTx {
            ciphertext: vec![1, 2, 3],
            order_commitment: Hash512::zero(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
        };
        assert_eq!(tx.ciphertext, vec![1, 2, 3]);
    }

    #[test]
    fn decryption_share_construction() {
        let share = DecryptionShare {
            tx_hash: Hash512::zero(),
            share: vec![1, 2, 3],
            validator: Address::from_bytes([1u8; 20]),
        };
        assert_eq!(share.share, vec![1, 2, 3]);
    }

    #[test]
    fn peer_info_construction() {
        let peer = PeerInfo {
            node_id: [1u8; 32],
            address: "127.0.0.1:30333".to_string(),
            port: 30333,
            last_seen: 1000,
        };
        assert_eq!(peer.port, 30333);
        assert_eq!(peer.last_seen, 1000);
    }

    #[test]
    fn bft_vote_construction() {
        let vote = BFTVote {
            validator: Address::from_bytes([1u8; 20]),
            block_hash: Hash512::zero(),
            height: 1,
            round: 1,
            vote_type: 2,
            signature: vec![1; 64],
        };
        assert_eq!(vote.vote_type, 2);
        assert_eq!(vote.signature.len(), 64);
    }
}
