//! P2P connection management for the Atoll Protocol.
//!
//! Phase 1: Plain TCP connections with simple length-prefixed message framing.
//! Phase 2: Add encrypted transport using Noise Protocol Framework.

use crate::network::NetworkMessage;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Node identifier (32-byte random ID)
pub type NodeId = [u8; 32];

/// Peer connection (plain TCP for Phase 1)
pub struct PeerConnection {
    pub node_id: NodeId,
    pub address: String,
    pub stream: TcpStream,
    pub nonce_counter: u64,
}

impl PeerConnection {
    /// Creates a new connection to a peer
    pub async fn connect(node_id: NodeId, address: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(address)
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        Ok(Self {
            node_id,
            address: address.to_string(),
            stream,
            nonce_counter: 0,
        })
    }

    /// Sends a network message with length prefix
    pub async fn send_message(&mut self, message: &NetworkMessage) -> Result<(), String> {
        let payload = Self::serialize_message(message)?;

        // Send length-prefixed payload
        let len = (payload.len() as u32).to_le_bytes();
        self.stream
            .write_all(&len)
            .await
            .map_err(|_| "Send length failed".to_string())?;
        self.stream
            .write_all(&payload)
            .await
            .map_err(|_| "Send data failed".to_string())?;

        Ok(())
    }

    /// Receives a network message with length prefix
    pub async fn receive_message(&mut self) -> Result<NetworkMessage, String> {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        self.stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(|_e| "Read length failed".to_string())?;
        let len = u32::from_le_bytes(len_bytes) as usize;

        // Reject unreasonably large payloads to prevent memory exhaustion
        const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16 MB
        if len > MAX_MESSAGE_SIZE {
            return Err(format!("Message too large: {} bytes", len));
        }

        // Read payload
        let mut payload = vec![0u8; len];
        self.stream
            .read_exact(&mut payload)
            .await
            .map_err(|_e| "Read data failed".to_string())?;

        Self::deserialize_message(&payload)
    }

    /// Returns the peer's address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Returns the peer's node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Serializes a NetworkMessage to bytes
    fn serialize_message(message: &NetworkMessage) -> Result<Vec<u8>, String> {
        let mut buf = Vec::new();

        match message {
            NetworkMessage::Ping(p) => {
                buf.push(0);
                buf.extend_from_slice(&p.nonce.to_le_bytes());
                buf.extend_from_slice(&p.timestamp.to_le_bytes());
            }
            NetworkMessage::Pong(p) => {
                buf.push(1);
                buf.extend_from_slice(&p.nonce.to_le_bytes());
                buf.extend_from_slice(&p.timestamp.to_le_bytes());
            }
            NetworkMessage::FindNode(f) => {
                buf.push(2);
                buf.extend_from_slice(&f.target_node_id);
            }
            NetworkMessage::Neighbors(n) => {
                buf.push(3);
                buf.extend_from_slice(&(n.peers.len() as u32).to_le_bytes());
                for peer in &n.peers {
                    buf.extend_from_slice(&peer.node_id);
                    let addr_bytes = peer.address.as_bytes();
                    buf.extend_from_slice(&(addr_bytes.len() as u16).to_le_bytes());
                    buf.extend_from_slice(addr_bytes);
                    buf.extend_from_slice(&peer.port.to_le_bytes());
                    buf.extend_from_slice(&peer.last_seen.to_le_bytes());
                }
            }
            NetworkMessage::NewBlock(data) => {
                buf.push(10);
                buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buf.extend_from_slice(data);
            }
            NetworkMessage::NewBlockHash(h) => {
                buf.push(11);
                buf.extend_from_slice(&h.0);
            }
            NetworkMessage::GetBlock(h) => {
                buf.push(12);
                buf.extend_from_slice(&h.0);
            }
            NetworkMessage::BlockResponse(data) => {
                buf.push(13);
                buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buf.extend_from_slice(data);
            }
            NetworkMessage::NewTransaction(data) => {
                buf.push(20);
                buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buf.extend_from_slice(data);
            }
            NetworkMessage::NewTransactionBatch(batch) => {
                buf.push(21);
                buf.extend_from_slice(&(batch.len() as u32).to_le_bytes());
                for tx in batch {
                    buf.extend_from_slice(&(tx.len() as u32).to_le_bytes());
                    buf.extend_from_slice(tx);
                }
            }
            _ => return Err("Unsupported message type for serialization".to_string()),
        }

        Ok(buf)
    }

    /// Deserializes bytes to a NetworkMessage
    fn deserialize_message(data: &[u8]) -> Result<NetworkMessage, String> {
        if data.is_empty() {
            return Err("Empty message".to_string());
        }

        let msg_type = data[0];
        let payload = &data[1..];

        match msg_type {
            0 => {
                if payload.len() < 16 {
                    return Err("Ping message too short".to_string());
                }
                Ok(NetworkMessage::Ping(crate::network::PingMessage {
                    nonce: u64::from_le_bytes(payload[..8].try_into().unwrap()),
                    timestamp: u64::from_le_bytes(payload[8..16].try_into().unwrap()),
                }))
            }
            1 => {
                if payload.len() < 16 {
                    return Err("Pong message too short".to_string());
                }
                Ok(NetworkMessage::Pong(crate::network::PongMessage {
                    nonce: u64::from_le_bytes(payload[..8].try_into().unwrap()),
                    timestamp: u64::from_le_bytes(payload[8..16].try_into().unwrap()),
                }))
            }
            2 => {
                if payload.len() < 32 {
                    return Err("FindNode message too short".to_string());
                }
                let mut target = [0u8; 32];
                target.copy_from_slice(&payload[..32]);
                Ok(NetworkMessage::FindNode(crate::network::FindNodeMessage {
                    target_node_id: target,
                }))
            }
            _ => Err(format!("Unknown message type: {}", msg_type)),
        }
    }
}

/// Listener for incoming peer connections
pub struct PeerListener {
    pub listener: tokio::net::TcpListener,
}

impl PeerListener {
    /// Creates a new listener on the given address
    pub async fn bind(address: &str) -> Result<Self, String> {
        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|e| format!("Bind failed: {}", e))?;

        Ok(Self { listener })
    }

    /// Accepts an incoming connection
    pub async fn accept(&mut self) -> Result<PeerConnection, String> {
        let (stream, addr) = self
            .listener
            .accept()
            .await
            .map_err(|e| format!("Accept failed: {}", e))?;

        // Derive node ID from address hash (simplified for Phase 1)
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(addr.to_string().as_bytes());
        let hash = hasher.finalize();
        let bytes = hash.as_bytes();
        let mut node_id = [0u8; 32];
        node_id.copy_from_slice(&bytes[..32]);

        Ok(PeerConnection {
            node_id,
            address: addr.to_string(),
            stream,
            nonce_counter: 0,
        })
    }

    /// Returns the listener's local address
    pub fn local_addr(&self) -> Result<std::net::SocketAddr, String> {
        self.listener
            .local_addr()
            .map_err(|e| format!("Local addr failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_serialize_deserialize() {
        let ping = crate::network::PingMessage {
            nonce: 42,
            timestamp: 1000,
        };
        let msg = NetworkMessage::Ping(ping.clone());
        let serialized = PeerConnection::serialize_message(&msg).unwrap();
        let deserialized = PeerConnection::deserialize_message(&serialized).unwrap();
        match deserialized {
            NetworkMessage::Ping(p) => {
                assert_eq!(p.nonce, 42);
                assert_eq!(p.timestamp, 1000);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn find_node_serialize_deserialize() {
        let target = [5u8; 32];
        let msg = NetworkMessage::FindNode(crate::network::FindNodeMessage {
            target_node_id: target,
        });
        let serialized = PeerConnection::serialize_message(&msg).unwrap();
        let deserialized = PeerConnection::deserialize_message(&serialized).unwrap();
        match deserialized {
            NetworkMessage::FindNode(f) => {
                assert_eq!(f.target_node_id, target);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn pong_serialize_deserialize() {
        let pong = crate::network::PongMessage {
            nonce: 99,
            timestamp: 2000,
        };
        let msg = NetworkMessage::Pong(pong.clone());
        let serialized = PeerConnection::serialize_message(&msg).unwrap();
        let deserialized = PeerConnection::deserialize_message(&serialized).unwrap();
        match deserialized {
            NetworkMessage::Pong(p) => {
                assert_eq!(p.nonce, 99);
                assert_eq!(p.timestamp, 2000);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
