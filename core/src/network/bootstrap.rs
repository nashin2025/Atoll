//! Bootstrap nodes and peer discovery for the Atoll Protocol.
//!
//! Provides a hardcoded seed node list and a [`BootstrapClient`] that connects
//! to seed nodes, exchanges `FindNode`/`Neighbors` discovery messages over the
//! Noise transport, and integrates the discovered peers with
//! [`DiscoveryProtocol`] and [`PeerManager`].

use crate::network::discovery::DiscoveryProtocol;
use crate::network::node_identity::NodeIdentity;
use crate::network::p2p::{NodeId, PeerConnection};
use crate::network::peer_manager::PeerManager;
use crate::network::{FindNodeMessage, NeighborsMessage, NetworkMessage, PeerInfo};
use std::time::Duration;
use tokio::time::timeout;

/// A bootstrap (seed) node used for initial peer discovery.
#[derive(Debug, Clone)]
pub struct BootstrapNode {
    /// Socket address of the seed node, e.g. `"127.0.0.1:30333"`.
    pub address: &'static str,
    /// Expected node ID (BLAKE3 fingerprint of the seed's static public key).
    pub node_id: NodeId,
}

/// Hardcoded mainnet bootstrap node list.
///
/// Operators maintain this list. Replace the example addresses below with the
/// operator-maintained mainnet seed list before launch.
pub const BOOTSTRAP_NODES: &[BootstrapNode] = &[
    BootstrapNode {
        address: "127.0.0.1:30333",
        node_id: [0u8; 32],
    },
    BootstrapNode {
        address: "127.0.0.1:30334",
        node_id: [1u8; 32],
    },
];

/// Client that bootstraps peer discovery by connecting to seed nodes.
pub struct BootstrapClient {
    identity: NodeIdentity,
    connect_timeout: Duration,
}

impl BootstrapClient {
    /// Creates a new bootstrap client for the local node identity.
    pub fn new(identity: NodeIdentity) -> Self {
        Self {
            identity,
            connect_timeout: Duration::from_secs(10),
        }
    }

    /// Connects to every configured bootstrap node, exchanges a
    /// `FindNode`/`Neighbors` discovery round, and returns the discovered
    /// peers.
    ///
    /// Failures to connect to an individual seed node are logged and skipped;
    /// a single unreachable seed does not abort bootstrap.
    pub async fn fetch_peers(&self) -> Vec<PeerInfo> {
        let mut peers = Vec::new();
        for seed in BOOTSTRAP_NODES.iter() {
            match self.fetch_from_seed(seed).await {
                Ok(mut discovered) => peers.append(&mut discovered),
                Err(e) => {
                    tracing::warn!(
                        address = %seed.address,
                        "bootstrap seed unreachable: {e}"
                    );
                }
            }
        }
        peers
    }

    /// Connects to a single seed node and collects its advertised neighbors.
    async fn fetch_from_seed(&self, seed: &BootstrapNode) -> Result<Vec<PeerInfo>, String> {
        let mut connection = timeout(
            self.connect_timeout,
            PeerConnection::connect(&self.identity, &seed.address, Some(seed.node_id)),
        )
        .await
        .map_err(|_| "connection timed out".to_string())?
        .map_err(|e| format!("handshake failed: {e}"))?;

        // Ask the seed for peers closest to our own node id.
        let our_node_id = self.identity.node_id();
        connection
            .send_message(&NetworkMessage::FindNode(FindNodeMessage {
                target_node_id: our_node_id,
            }))
            .await
            .map_err(|e| format!("send FindNode failed: {e}"))?;

        let response = timeout(self.connect_timeout, connection.receive_message())
            .await
            .map_err(|_| "response timed out".to_string())?
            .map_err(|e| format!("receive failed: {e}"))?;

        match response {
            NetworkMessage::Neighbors(NeighborsMessage { peers }) => Ok(peers),
            other => Err("unexpected response from seed".to_string()),
        }
    }

    /// Registers all discovered peers with both [`DiscoveryProtocol`] and
    /// [`PeerManager`].
    pub fn register_discovered(
        peers: &[PeerInfo],
        discovery: &mut DiscoveryProtocol,
        peer_manager: &mut PeerManager,
    ) {
        for info in peers {
            discovery.register_peer(info.clone());
            let _ = peer_manager.add_peer(info.node_id, info.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::node_identity::NodeIdentity;
    use crate::network::p2p::PeerListener;

    #[test]
    fn bootstrap_nodes_are_non_empty() {
        assert!(!BOOTSTRAP_NODES.is_empty());
        for seed in BOOTSTRAP_NODES.iter() {
            assert!(!seed.address.is_empty());
        }
    }

    #[test]
    fn bootstrap_client_construction() {
        let identity = NodeIdentity::generate().expect("failed to generate identity");
        let client =
            BootstrapClient::new(NodeIdentity::generate().expect("failed to generate identity"));
        let _ = (client, identity);
    }

    #[test]
    fn register_discovered_adds_peers_to_both() {
        let info = PeerInfo {
            node_id: [4u8; 32],
            address: "127.0.0.1:40000".to_string(),
            port: 40000,
            last_seen: 1000,
        };

        let mut discovery = DiscoveryProtocol::new([0u8; 32], 8);
        let mut peer_manager = PeerManager::new(256);

        BootstrapClient::register_discovered(&[info.clone()], &mut discovery, &mut peer_manager);

        assert_eq!(discovery.peer_count(), 1);
        assert_eq!(peer_manager.connected_count(), 1);
        assert_eq!(discovery.all_peers()[0].node_id, info.node_id);
        assert!(peer_manager.get_peer(&info.node_id).is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fetch_from_seed_exchanges_find_node_neighbors() {
        let bootstrap_identity =
            NodeIdentity::generate().expect("failed to generate bootstrap identity");
        let bootstrap_node_id = bootstrap_identity.node_id();

        let client_identity = NodeIdentity::generate().expect("failed to generate client identity");
        let client = BootstrapClient::new(client_identity);

        let mut listener = PeerListener::bind("127.0.0.1:0", bootstrap_identity)
            .await
            .expect("failed to bind bootstrap listener");
        let addr = listener.local_addr().expect("failed to get local address");

        let address: &'static str = Box::leak(addr.to_string().into_boxed_str());
        let seed = BootstrapNode {
            address,
            node_id: bootstrap_node_id,
        };

        // The bootstrap side accepts the connection and answers a FindNode
        // with a Neighbors message.
        let accepted = tokio::spawn(async move {
            let mut connection = listener.accept().await.expect("bootstrap accept failed");
            let message = connection
                .receive_message()
                .await
                .expect("bootstrap receive failed");
            match message {
                NetworkMessage::FindNode(_) => {
                    let peers = vec![PeerInfo {
                        node_id: [9u8; 32],
                        address: "127.0.0.1:50000".to_string(),
                        port: 50000,
                        last_seen: 0,
                    }];
                    connection
                        .send_message(&NetworkMessage::Neighbors(NeighborsMessage { peers }))
                        .await
                        .expect("bootstrap send failed");
                }
                other => panic!("unexpected message from client: {other:?}"),
            }
        });

        let peers = client
            .fetch_from_seed(&seed)
            .await
            .expect("fetch from seed should succeed");

        accepted.await.expect("bootstrap task panicked");

        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, [9u8; 32]);
        assert_eq!(peers[0].port, 50000);
    }
}
