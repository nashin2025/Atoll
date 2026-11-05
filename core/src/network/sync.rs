//! Block sync engine for the Atoll Protocol.
//!
//! Handles block/header exchange, state sync, and chain synchronization.

use crate::network::{BlockHeader, GetHeadersMessage};
use crate::primitives::hash::Hash512;
use std::collections::HashMap;

/// Sync state for a peer
#[derive(Debug, Clone)]
pub struct PeerSyncState {
    pub node_id: crate::network::p2p::NodeId,
    pub current_height: u64,
    pub common_height: u64,
    pub syncing: bool,
}

/// Sync manager: coordinates block/header exchange with peers
pub struct SyncManager {
    peer_states: HashMap<crate::network::p2p::NodeId, PeerSyncState>,
    max_sync_peers: usize,
    header_cache: HashMap<Hash512, BlockHeader>,
}

impl SyncManager {
    /// Creates a new sync manager
    pub fn new(max_sync_peers: usize) -> Self {
        Self {
            peer_states: HashMap::new(),
            max_sync_peers,
            header_cache: HashMap::new(),
        }
    }

    /// Updates a peer's sync state
    pub fn update_peer_height(&mut self, node_id: crate::network::p2p::NodeId, height: u64) {
        self.peer_states
            .entry(node_id)
            .or_insert(PeerSyncState {
                node_id,
                current_height: height,
                common_height: 0,
                syncing: false,
            })
            .current_height = height;
    }

    /// Returns peers that are behind our height (need blocks from us)
    pub fn peers_needing_blocks(&self, our_height: u64) -> Vec<crate::network::p2p::NodeId> {
        self.peer_states
            .iter()
            .filter(|(_, state)| state.current_height < our_height)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns peers that are ahead of us (we need blocks from them)
    pub fn peers_ahead_of_us(&self, our_height: u64) -> Vec<crate::network::p2p::NodeId> {
        self.peer_states
            .iter()
            .filter(|(_, state)| state.current_height > our_height)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Creates a GetHeaders request for syncing from a peer
    pub fn create_get_headers_request(
        &self,
        start_height: u64,
        max_count: u64,
    ) -> GetHeadersMessage {
        GetHeadersMessage {
            start_height,
            max_count,
        }
    }

    /// Caches a block header
    pub fn cache_header(&mut self, header: BlockHeader) {
        self.header_cache.insert(header.hash, header);
    }

    /// Returns a cached header if available
    pub fn get_cached_header(&self, hash: &Hash512) -> Option<&BlockHeader> {
        self.header_cache.get(hash)
    }

    /// Returns the number of tracked peers
    pub fn tracked_peer_count(&self) -> usize {
        self.peer_states.len()
    }

    /// Marks a peer as syncing
    pub fn set_syncing(&mut self, node_id: crate::network::p2p::NodeId, syncing: bool) {
        if let Some(state) = self.peer_states.get_mut(&node_id) {
            state.syncing = syncing;
        }
    }
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_and_query_peer_height() {
        let mut sm = SyncManager::new(10);
        let node_id = [1u8; 32];

        sm.update_peer_height(node_id, 100);
        assert_eq!(sm.tracked_peer_count(), 1);

        let ahead = sm.peers_ahead_of_us(50);
        assert!(ahead.contains(&node_id));

        let behind = sm.peers_needing_blocks(200);
        assert!(behind.contains(&node_id));
    }

    #[test]
    fn header_cache() {
        let mut sm = SyncManager::new(10);
        let header = BlockHeader {
            height: 1,
            hash: Hash512::zero(),
            prev_hash: Hash512::zero(),
            timestamp: 1000,
        };

        sm.cache_header(header.clone());
        let cached = sm.get_cached_header(&Hash512::zero());
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().height, 1);
    }
}
