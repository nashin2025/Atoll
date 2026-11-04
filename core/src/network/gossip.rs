//! Gossip protocol for the Atoll Protocol.
//!
//! Epidemic-style message propagation with fanout-based forwarding.

use crate::network::p2p::NodeId;
use crate::network::NetworkMessage;
use crate::primitives::hash::Hash512;
use std::collections::HashSet;

/// Gossip protocol configuration
#[derive(Debug, Clone)]
pub struct GossipConfig {
    pub fanout: usize,
    pub max_history: usize,
    pub gossip_interval_ms: u64,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            fanout: 8,
            max_history: 10_000,
            gossip_interval_ms: 500,
        }
    }
}

/// Gossip protocol instance
pub struct GossipProtocol {
    config: GossipConfig,
    message_history: HashSet<Hash512>,
    pending_messages: Vec<(NodeId, NetworkMessage)>,
    last_gossip: u64,
}

impl GossipProtocol {
    /// Creates a new gossip protocol instance
    pub fn new(config: GossipConfig) -> Self {
        Self {
            config,
            message_history: HashSet::new(),
            pending_messages: Vec::new(),
            last_gossip: 0,
        }
    }

    /// Broadcasts a message to the network using gossip
    pub fn broadcast(
        &mut self,
        message: &NetworkMessage,
        sender: NodeId,
    ) {
        let message_hash = Self::hash_message(message);

        // Skip if we've already seen this message
        if self.message_history.contains(&message_hash) {
            return;
        }

        // Record the message
        self.message_history.insert(message_hash);

        // Trim history if too large
        if self.message_history.len() > self.config.max_history {
            // Remove oldest 10%
            let to_remove = self.config.max_history / 10;
            let keys: Vec<Hash512> = self.message_history.iter().take(to_remove).cloned().collect();
            for key in keys {
                self.message_history.remove(&key);
            }
        }

        // Add to pending queue for gossip
        self.pending_messages.push((sender, message.clone()));
    }

    /// Selects peers to gossip to (fanout-based)
    pub fn select_gossip_targets(
        &self,
        connected_peers: &[NodeId],
        exclude: NodeId,
    ) -> Vec<NodeId> {
        let mut targets: Vec<NodeId> = connected_peers
            .iter()
            .filter(|id| **id != exclude)
            .cloned()
            .collect();

        // Shuffle targets using simple hash-based ordering
        targets.sort_by(|a, b| {
            let ha = blake3::hash(&a[..]);
            let hb = blake3::hash(&b[..]);
            ha.as_bytes().cmp(hb.as_bytes())
        });

        targets.truncate(self.config.fanout);
        targets
    }

    /// Returns pending messages ready for gossip
    pub fn take_pending(&mut self) -> Vec<(NodeId, NetworkMessage)> {
        std::mem::take(&mut self.pending_messages)
    }

    /// Returns whether it's time to gossip
    pub fn should_gossip(&self, now_ms: u64) -> bool {
        now_ms - self.last_gossip >= self.config.gossip_interval_ms
    }

    /// Updates the last gossip timestamp
    pub fn update_gossip_time(&mut self, now_ms: u64) {
        self.last_gossip = now_ms;
    }

    /// Returns the message history size
    pub fn history_size(&self) -> usize {
        self.message_history.len()
    }

    /// Hashes a message for deduplication
    fn hash_message(message: &NetworkMessage) -> Hash512 {
        let mut serialized = Vec::new();
        match message {
            NetworkMessage::Ping(p) => {
                serialized.push(0);
                serialized.extend_from_slice(&p.nonce.to_le_bytes());
                serialized.extend_from_slice(&p.timestamp.to_le_bytes());
            }
            NetworkMessage::Pong(p) => {
                serialized.push(1);
                serialized.extend_from_slice(&p.nonce.to_le_bytes());
                serialized.extend_from_slice(&p.timestamp.to_le_bytes());
            }
            NetworkMessage::FindNode(f) => {
                serialized.push(2);
                serialized.extend_from_slice(&f.target_node_id);
            }
            NetworkMessage::Neighbors(n) => {
                serialized.push(3);
                serialized.extend_from_slice(&(n.peers.len() as u32).to_le_bytes());
            }
            NetworkMessage::NewBlockHash(h) => {
                serialized.push(11);
                serialized.extend_from_slice(&h.0);
            }
            NetworkMessage::GetBlock(h) => {
                serialized.push(12);
                serialized.extend_from_slice(&h.0);
            }
            _ => {
                serialized.push(255);
                serialized.extend_from_slice(&format!("{:?}", message).into_bytes());
            }
        }
        let mut hasher = blake3::Hasher::new();
        hasher.update(&serialized);
        let mut out = [0u8; 64];
        hasher.finalize_xof().fill(&mut out);
        Hash512(out)
    }
}

impl Default for GossipProtocol {
    fn default() -> Self {
        Self::new(GossipConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::PingMessage;

    #[test]
    fn broadcast_and_dedup() {
        let mut gossip = GossipProtocol::new(GossipConfig::default());
        let message = NetworkMessage::Ping(PingMessage {
            nonce: 1,
            timestamp: 1000,
        });
        let sender = [1u8; 32];

        // First broadcast should be accepted
        let history_before = gossip.history_size();
        gossip.broadcast(&message, sender);
        assert_eq!(gossip.history_size(), history_before + 1);

        // Second broadcast of same message should be deduped
        gossip.broadcast(&message, sender);
        assert_eq!(gossip.history_size(), history_before + 1);
    }

    #[test]
    fn select_gossip_targets_excludes_sender() {
        let gossip = GossipProtocol::new(GossipConfig::default());
        let peers = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let targets = gossip.select_gossip_targets(&peers, [2u8; 32]);

        assert!(!targets.contains(&[2u8; 32]));
        assert!(targets.len() <= 3); // fanout is 8, but we have 3 remaining peers
    }

    #[test]
    fn pending_messages_workflow() {
        let mut gossip = GossipProtocol::new(GossipConfig::default());
        let message = NetworkMessage::Ping(PingMessage {
            nonce: 1,
            timestamp: 1000,
        });
        let sender = [1u8; 32];

        gossip.broadcast(&message, sender);
        assert!(!gossip.take_pending().is_empty());
        assert!(gossip.take_pending().is_empty()); // Already taken
    }
}
