//! Health monitoring for the Atoll Protocol node.
//!
//! Tracks chain sync status, peer count, tip height, and finalized height
//! to power the `/health` endpoint.

use crate::chain::blockchain::Blockchain;
use crate::network::peer_manager::PeerManager;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::RwLock;

/// Snapshot of node health at a point in time.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub chain_id: u64,
    pub block_height: u64,
    pub syncing: bool,
    pub peers: usize,
}

/// Monitors node health by reading chain and network state.
pub struct HealthMonitor {
    blockchain: Arc<RwLock<Blockchain>>,
    peer_manager: Arc<RwLock<PeerManager>>,
    /// Syncing flag — `AtomicBool` so it can be updated through shared
    /// references held behind an `Arc`.
    syncing: AtomicBool,
    version: String,
}

impl HealthMonitor {
    /// Creates a new health monitor.
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        peer_manager: Arc<RwLock<PeerManager>>,
    ) -> Self {
        Self {
            blockchain,
            peer_manager,
            syncing: AtomicBool::new(false),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Sets the syncing flag (e.g. when the sync module is catching up).
    pub fn set_syncing(&self, syncing: bool) {
        self.syncing.store(syncing, Ordering::SeqCst);
    }

    /// Returns whether the node is currently syncing.
    pub fn is_syncing(&self) -> bool {
        self.syncing.load(Ordering::SeqCst)
    }

    /// Captures a fresh health snapshot.
    pub fn check(&self) -> HealthStatus {
        let blockchain = self.blockchain.read().expect("blockchain lock poisoned");
        let peer_manager = self
            .peer_manager
            .read()
            .expect("peer manager lock poisoned");

        let syncing = self.syncing.load(Ordering::SeqCst);
        let status = if syncing { "syncing" } else { "healthy" };

        HealthStatus {
            status: status.to_string(),
            version: self.version.clone(),
            chain_id: crate::CHAIN_ID,
            block_height: blockchain.tip_height(),
            syncing,
            peers: peer_manager.connected_count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::state::ChainState;
    use std::sync::RwLock;

    #[test]
    fn health_status_defaults() {
        let blockchain = Arc::new(RwLock::new(Blockchain::default()));
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(256)));
        let monitor = HealthMonitor::new(Arc::clone(&blockchain), Arc::clone(&peer_manager));

        let status = monitor.check();
        assert_eq!(status.status, "healthy");
        assert!(!status.syncing);
        assert_eq!(status.chain_id, crate::CHAIN_ID);
        assert_eq!(status.block_height, 0);
        assert_eq!(status.peers, 0);
        assert!(!status.version.is_empty());
    }

    #[test]
    fn health_status_syncing() {
        let blockchain = Arc::new(RwLock::new(Blockchain::default()));
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(256)));
        let mut monitor = HealthMonitor::new(Arc::clone(&blockchain), Arc::clone(&peer_manager));
        monitor.set_syncing(true);

        let status = monitor.check();
        assert_eq!(status.status, "syncing");
        assert!(status.syncing);
    }

    #[test]
    fn health_status_peer_count() {
        let blockchain = Arc::new(RwLock::new(Blockchain::default()));
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(256)));
        let monitor = HealthMonitor::new(Arc::clone(&blockchain), Arc::clone(&peer_manager));

        peer_manager.write().expect("lock").add_peer(
            [1u8; 32],
            crate::network::PeerInfo {
                node_id: [1u8; 32],
                address: "127.0.0.1".to_string(),
                port: 30333,
                last_seen: 0,
            },
        );

        let status = monitor.check();
        assert_eq!(status.peers, 1);
    }

    #[test]
    fn health_status_chain_state_not_required() {
        // Health status only needs blockchain + peer manager; ChainState is
        // held in AppState for future endpoints but not read by health.
        let _state = ChainState::new();
        assert_eq!(_state.total_supply(), 0);
    }
}
