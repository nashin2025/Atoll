//! Peer discovery protocol for the Atoll Protocol.
//!
//! Kademlia-style DHT for finding peers on the network.

use crate::network::p2p::NodeId;
use crate::network::{FindNodeMessage, NeighborsMessage, PeerInfo, PingMessage, PongMessage};
use std::collections::HashMap;
use std::time::SystemTime;

/// Discovery protocol instance
pub struct DiscoveryProtocol {
    node_id: NodeId,
    known_peers: HashMap<NodeId, PeerInfo>,
    bucket_size: usize,
}

impl DiscoveryProtocol {
    /// Creates a new discovery protocol instance
    pub fn new(node_id: NodeId, bucket_size: usize) -> Self {
        Self {
            node_id,
            known_peers: HashMap::new(),
            bucket_size,
        }
    }

    /// Registers a known peer
    pub fn register_peer(&mut self, info: PeerInfo) {
        self.known_peers.insert(info.node_id, info);
    }

    /// Returns the number of known peers
    pub fn peer_count(&self) -> usize {
        self.known_peers.len()
    }

    /// Returns our node ID
    pub fn our_node_id(&self) -> NodeId {
        self.node_id
    }

    /// Creates a ping message
    pub fn create_ping(&self) -> PingMessage {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        PingMessage {
            nonce: now,
            timestamp: now,
        }
    }

    /// Handles an incoming ping and returns a pong
    pub fn handle_ping(&self, ping: PingMessage) -> PongMessage {
        PongMessage {
            nonce: ping.nonce,
            timestamp: ping.timestamp,
        }
    }

    /// Handles an incoming pong
    pub fn handle_pong(&self, _pong: PongMessage) {
        // Phase 1: No-op
        // Phase 2: Update RTT statistics
    }

    /// Creates a FindNode request for a target node ID
    pub fn create_find_node(&self, target: NodeId) -> FindNodeMessage {
        FindNodeMessage {
            target_node_id: target,
        }
    }

    /// Handles a FindNode request and returns closest peers
    pub fn handle_find_node(
        &self,
        request: &FindNodeMessage,
    ) -> NeighborsMessage {
        let mut peers: Vec<_> = self.known_peers.values().cloned().collect();

        // Sort by XOR distance to target
        peers.sort_by(|a, b| {
            let da = xor_distance(&a.node_id, &request.target_node_id);
            let db = xor_distance(&b.node_id, &request.target_node_id);
            da.cmp(&db)
        });

        peers.truncate(self.bucket_size);

        NeighborsMessage { peers }
    }

    /// Returns all known peers
    pub fn all_peers(&self) -> Vec<PeerInfo> {
        self.known_peers.values().cloned().collect()
    }
}

/// XOR distance between two node IDs (Kademlia distance metric)
fn xor_distance(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut result = [0u8; 32];
    for i in 0..32 {
        result[i] = a[i] ^ b[i];
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_find_peer() {
        let mut discovery = DiscoveryProtocol::new([1u8; 32], 8);
        let info = PeerInfo {
            node_id: [2u8; 32],
            address: "127.0.0.1".to_string(),
            port: 30333,
            last_seen: 0,
        };

        discovery.register_peer(info.clone());
        assert_eq!(discovery.peer_count(), 1);

        let neighbors = discovery.handle_find_node(&FindNodeMessage {
            target_node_id: [2u8; 32],
        });
        assert_eq!(neighbors.peers.len(), 1);
        assert_eq!(neighbors.peers[0].node_id, [2u8; 32]);
    }

    #[test]
    fn ping_pong_roundtrip() {
        let discovery = DiscoveryProtocol::new([1u8; 32], 8);
        let ping = discovery.create_ping();
        let pong = discovery.handle_ping(ping.clone());
        assert_eq!(pong.nonce, ping.nonce);
    }

    #[test]
    fn bucket_size_limits_neighbors() {
        let mut discovery = DiscoveryProtocol::new([1u8; 32], 2);
        for i in 0..5 {
            let info = PeerInfo {
                node_id: [i as u8; 32],
                address: format!("127.0.0.1:{}", 30000 + i),
                port: 30000 + i as u16,
                last_seen: 0,
            };
            discovery.register_peer(info);
        }

        let neighbors = discovery.handle_find_node(&FindNodeMessage {
            target_node_id: [0u8; 32],
        });
        assert!(neighbors.peers.len() <= 2);
    }
}
