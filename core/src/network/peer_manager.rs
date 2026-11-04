//! Peer manager for the Atoll Protocol.
//!
//! Tracks connected peers, their status, and connection metadata.

use crate::network::p2p::NodeId;
use crate::network::PeerInfo;
use std::collections::HashMap;
use std::time::SystemTime;

/// Peer connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerStatus {
    Connected,
    Disconnected,
    Banned,
}

/// Peer metadata
#[derive(Debug, Clone)]
pub struct PeerMeta {
    pub info: PeerInfo,
    pub status: PeerStatus,
    pub connection_count: u32,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub last_ping: Option<u64>,
    pub last_pong: Option<u64>,
}

/// Peer manager: tracks all connected peers
pub struct PeerManager {
    peers: HashMap<NodeId, PeerMeta>,
    max_peers: usize,
    banned_peers: HashMap<NodeId, u64>, // node_id -> ban_until_timestamp
}

impl PeerManager {
    /// Creates a new peer manager
    pub fn new(max_peers: usize) -> Self {
        Self {
            peers: HashMap::new(),
            max_peers,
            banned_peers: HashMap::new(),
        }
    }

    /// Adds a new peer
    pub fn add_peer(&mut self, node_id: NodeId, info: PeerInfo) -> bool {
        if self.banned_peers.contains_key(&node_id) {
            return false;
        }

        if self.peers.len() >= self.max_peers {
            return false;
        }

        let _now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.peers.insert(
            node_id,
            PeerMeta {
                info,
                status: PeerStatus::Connected,
                connection_count: 1,
                messages_sent: 0,
                messages_received: 0,
                last_ping: None,
                last_pong: None,
            },
        );

        true
    }

    /// Removes a peer
    pub fn remove_peer(&mut self, node_id: &NodeId) {
        self.peers.remove(node_id);
    }

    /// Marks a peer as disconnected
    pub fn disconnect_peer(&mut self, node_id: &NodeId) {
        if let Some(meta) = self.peers.get_mut(node_id) {
            meta.status = PeerStatus::Disconnected;
        }
    }

    /// Bans a peer for a given duration (seconds)
    pub fn ban_peer(&mut self, node_id: NodeId, duration_secs: u64) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.banned_peers.insert(node_id, now + duration_secs);
        self.peers.remove(&node_id);
    }

    /// Checks if a peer is banned
    pub fn is_banned(&self, node_id: &NodeId) -> bool {
        if let Some(ban_until) = self.banned_peers.get(node_id) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            now < *ban_until
        } else {
            false
        }
    }

    /// Returns the number of connected peers
    pub fn connected_count(&self) -> usize {
        self.peers.values().filter(|m| m.status == PeerStatus::Connected).count()
    }

    /// Returns the number of banned peers
    pub fn banned_count(&self) -> usize {
        self.banned_peers.len()
    }

    /// Returns peer info for a node ID
    pub fn get_peer(&self, node_id: &NodeId) -> Option<&PeerMeta> {
        self.peers.get(node_id)
    }

    /// Returns all connected peer IDs
    pub fn connected_peers(&self) -> Vec<NodeId> {
        self.peers
            .iter()
            .filter(|(_, m)| m.status == PeerStatus::Connected)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Records a message sent to a peer
    pub fn record_message_sent(&mut self, node_id: &NodeId) {
        if let Some(meta) = self.peers.get_mut(node_id) {
            meta.messages_sent += 1;
        }
    }

    /// Records a message received from a peer
    pub fn record_message_received(&mut self, node_id: &NodeId) {
        if let Some(meta) = self.peers.get_mut(node_id) {
            meta.messages_received += 1;
        }
    }

    /// Updates ping/pong timestamps
    pub fn update_ping_pong(&mut self, node_id: &NodeId, ping: u64, pong: Option<u64>) {
        if let Some(meta) = self.peers.get_mut(node_id) {
            meta.last_ping = Some(ping);
            meta.last_pong = pong;
        }
    }

    /// Returns the max peers setting
    pub fn max_peers(&self) -> usize {
        self.max_peers
    }
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove_peer() {
        let mut pm = PeerManager::new(10);
        let node_id = [1u8; 32];
        let info = PeerInfo {
            node_id,
            address: "127.0.0.1".to_string(),
            port: 30333,
            last_seen: 0,
        };

        assert!(pm.add_peer(node_id, info.clone()));
        assert_eq!(pm.connected_count(), 1);
        assert!(pm.get_peer(&node_id).is_some());

        pm.remove_peer(&node_id);
        assert_eq!(pm.connected_count(), 0);
        assert!(pm.get_peer(&node_id).is_none());
    }

    #[test]
    fn ban_peer_prevents_reconnection() {
        let mut pm = PeerManager::new(10);
        let node_id = [2u8; 32];
        let info = PeerInfo {
            node_id,
            address: "127.0.0.1".to_string(),
            port: 30333,
            last_seen: 0,
        };

        assert!(pm.add_peer(node_id, info.clone()));
        pm.ban_peer(node_id, 3600);
        assert!(pm.is_banned(&node_id));
        assert!(!pm.add_peer(node_id, info)); // Should fail - banned
    }

    #[test]
    fn max_peers_limit() {
        let mut pm = PeerManager::new(2);
        for i in 0..3 {
            let node_id = [i as u8; 32];
            let info = PeerInfo {
                node_id,
                address: format!("127.0.0.1:{}", 30000 + i),
                port: 30000 + i as u16,
                last_seen: 0,
            };
            let result = pm.add_peer(node_id, info);
            if i < 2 {
                assert!(result, "Peer {} should be added", i);
            } else {
                assert!(!result, "Peer {} should be rejected (max reached)", i);
            }
        }
    }
}
