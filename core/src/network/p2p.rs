//! P2P connection management for the Atoll Protocol.
//!
//! Phase 1: Plain TCP connections with simple length-prefixed message framing.
//! Phase 2: Encrypted transport using Noise Protocol Framework
//! (`Noise_XX_25519_ChaChaPoly_BLAKE2s`).

use crate::network::node_identity::NodeIdentity;
use crate::network::NetworkMessage;

use blake3::Hasher;
use snow::{Builder, TransportState};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use tokio::task::block_in_place;

use std::io::{self, Read, Write};
use std::net::TcpStream as StdTcpStream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Node identifier (32-byte random ID)
pub type NodeId = [u8; 32];

/// Maximum plaintext bytes carried by a single Noise transport message.
///
/// Snow caps ciphertext at 65535 bytes; ChaChaPoly appends a 16-byte
/// authentication tag, leaving 65519 bytes of plaintext per message.
const MAX_NOISE_PLAINTEXT: usize = 65535 - 16;

/// A Noise-encrypted byte stream over a TCP socket.
///
/// Wraps a [`snow::TransportState`] (which encrypts and decrypts Noise
/// transport messages) together with the underlying TCP socket. Each Noise
/// message is framed on the wire with a 4-byte little-endian length prefix so
/// the receiver can delimit messages on the byte stream.
///
/// [`tokio::io::AsyncRead`] / [`tokio::io::AsyncWrite`] are implemented by
/// delegating to blocking [`std::io::Read`] / [`std::io::Write`] inside
/// [`tokio::task::block_in_place`].
pub struct NoiseStream {
    /// Noise transport state: encrypts outbound and decrypts inbound messages.
    state: TransportState,
    /// Underlying TCP socket, placed in blocking mode for use with block_in_place.
    tcp: StdTcpStream,
    /// Decrypted plaintext read from the wire but not yet consumed by a reader.
    pending: Vec<u8>,
    /// Offset into `pending` already handed out to readers.
    pending_pos: usize,
}

/// Peer connection (encrypted Noise transport for Phase 2)
pub struct PeerConnection {
    pub node_id: NodeId,
    pub address: String,
    pub stream: NoiseStream,
}

impl PeerConnection {
    /// Creates a new encrypted connection to a peer.
    ///
    /// Performs a Noise XX initiator handshake using `identity` as our static
    /// Curve25519 key, then derives the peer's [`NodeId`] from the remote
    /// static public key fingerprint (`blake3(peer_static_pub_key)`) and verifies
    /// it against `expected_node_id` when one is provided.
    pub async fn connect(
        identity: &NodeIdentity,
        address: &str,
        expected_node_id: Option<NodeId>,
    ) -> Result<Self, String> {
        let tcp = TcpStream::connect(address)
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        // tokio sockets are non-blocking; switch to blocking mode so the
        // synchronous Noise handshake (and the blocking IO used by NoiseStream)
        // can run inside `block_in_place`.
        let mut std_tcp = tcp
            .into_std()
            .map_err(|e| format!("Failed to take ownership of socket: {}", e))?;
        std_tcp
            .set_nonblocking(false)
            .map_err(|e| format!("Failed to set socket to blocking mode: {}", e))?;

        // The Noise handshake is synchronous CPU + blocking socket IO, so run
        // it off the tokio reactor.
        let state = block_in_place(|| perform_initiator_handshake(&mut std_tcp, identity))?;

        // Derive the peer's NodeId from their static public key fingerprint.
        let peer_static = state
            .get_remote_static()
            .ok_or_else(|| "Peer did not send a static key".to_string())?;
        let peer_static_array: [u8; 32] = peer_static[..32]
            .try_into()
            .map_err(|_| "Invalid remote static key length".to_string())?;
        let node_id = compute_node_id(&peer_static_array);

        // Verify the peer fingerprint matches the expected node id, if known.
        if let Some(expected) = expected_node_id {
            if node_id != expected {
                return Err(format!(
                    "Peer fingerprint mismatch: expected {:?}, got {:?}",
                    expected, node_id
                ));
            }
        }

        Ok(Self {
            node_id,
            address: address.to_string(),
            stream: NoiseStream {
                state,
                tcp: std_tcp,
                pending: Vec::new(),
                pending_pos: 0,
            },
        })
    }

    /// Sends a network message with length prefix.
    ///
    /// Framing (4-byte little-endian length + payload) and the
    /// [`NetworkMessage`] byte format are unchanged from Phase 1; each
    /// `write_all` call is carried as one or more Noise transport messages.
    pub async fn send_message(&mut self, message: &NetworkMessage) -> Result<(), String> {
        let payload = Self::serialize_message(message)?;

        // Send length-prefixed payload
        let len = (payload.len() as u32).to_le_bytes();
        AsyncWriteExt::write_all(&mut self.stream, &len)
            .await
            .map_err(|_| "Send length failed".to_string())?;
        AsyncWriteExt::write_all(&mut self.stream, &payload)
            .await
            .map_err(|_| "Send data failed".to_string())?;

        Ok(())
    }

    /// Receives a network message with length prefix.
    pub async fn receive_message(&mut self) -> Result<NetworkMessage, String> {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        AsyncReadExt::read_exact(&mut self.stream, &mut len_bytes)
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
        AsyncReadExt::read_exact(&mut self.stream, &mut payload)
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

/// Perform the synchronous Noise XX initiator handshake over a blocking TCP socket.
///
/// Returns the transport-mode [`snow::TransportState`] once the handshake
/// completes. The handshake exchanges:
///
///   1. `-> e`        (our ephemeral public key)
///   2. `<- e, ee, s, es` (their ephemeral + their static key, encrypted)
///   3. `-> s, se`    (our static key, encrypted)
fn perform_initiator_handshake(
    tcp: &mut StdTcpStream,
    identity: &NodeIdentity,
) -> Result<TransportState, String> {
    let params: snow::params::NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s"
        .parse()
        .map_err(|e| format!("Invalid noise params: {e}"))?;

    let secret_key = identity.secret_key();
    let mut hs = Builder::new(params)
        .local_private_key(&secret_key)
        .map_err(|e| format!("Failed to set local static key: {e}"))?
        .build_initiator()
        .map_err(|e| format!("Failed to build initiator handshake state: {e}"))?;

    let mut msg_buf = [0u8; 65535];
    let mut resp_buf = [0u8; 65535];

    // 1. -> e (initiator's first message: empty payload, ephemeral key only)
    let len = hs
        .write_message(&[], &mut msg_buf)
        .map_err(|e| format!("Handshake write (msg 1) failed: {e}"))?;
    send_frame(tcp, &msg_buf[..len]).map_err(|e| format!("Handshake send (msg 1) failed: {e}"))?;

    // 2. <- e, ee, s, es (responder's response; authenticates their static key)
    let resp = recv_frame(tcp, &mut resp_buf)
        .map_err(|e| format!("Handshake recv (msg 2) failed: {e}"))?;
    hs.read_message(resp, &mut msg_buf)
        .map_err(|e| format!("Handshake read (msg 2) failed: {e}"))?;

    // 3. -> s, se (our static key, encrypted)
    let len = hs
        .write_message(&[], &mut msg_buf)
        .map_err(|e| format!("Handshake write (msg 3) failed: {e}"))?;
    send_frame(tcp, &msg_buf[..len]).map_err(|e| format!("Handshake send (msg 3) failed: {e}"))?;

    hs.into_transport_mode()
        .map_err(|e| format!("Handshake finalize (into transport mode) failed: {e}"))
}

/// Send a single Noise message framed with a 4-byte little-endian length prefix.
fn send_frame(tcp: &mut StdTcpStream, data: &[u8]) -> io::Result<()> {
    let len = (data.len() as u32).to_le_bytes();
    tcp.write_all(&len)?;
    tcp.write_all(data)?;
    Ok(())
}

/// Receive a single Noise message framed with a 4-byte little-endian length prefix.
fn recv_frame<'a>(tcp: &mut StdTcpStream, buf: &'a mut [u8]) -> io::Result<&'a [u8]> {
    let mut len_bytes = [0u8; 4];
    tcp.read_exact(&mut len_bytes)?;
    let n = u32::from_le_bytes(len_bytes) as usize;
    if n > buf.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Noise frame too large",
        ));
    }
    tcp.read_exact(&mut buf[..n])?;
    Ok(&buf[..n])
}

/// Compute `blake3(public_key)` as `[u8; 32]` — the node's network fingerprint.
fn compute_node_id(public_key: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(public_key);
    let output = hasher.finalize();
    let bytes = output.as_bytes();
    let mut node_id = [0u8; 32];
    node_id.copy_from_slice(&bytes[..32]);
    node_id
}

impl NoiseStream {
    /// Read and decrypt one Noise message from the wire into fresh pending storage.
    fn read_one_message(&mut self) -> io::Result<Vec<u8>> {
        let mut len_bytes = [0u8; 4];
        self.tcp.read_exact(&mut len_bytes)?;
        let enc_len = u32::from_le_bytes(len_bytes) as usize;
        if enc_len > 65535 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Noise frame too large",
            ));
        }
        let mut enc = vec![0u8; enc_len];
        self.tcp.read_exact(&mut enc)?;
        let mut plain = vec![0u8; enc_len];
        let n = self.state.read_message(&enc, &mut plain).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Noise decrypt failed: {e}"),
            )
        })?;
        plain.truncate(n);
        Ok(plain)
    }
}

impl Read for NoiseStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // Serve any decrypted plaintext already buffered.
        if self.pending_pos < self.pending.len() {
            let available = &self.pending[self.pending_pos..];
            let n = available.len().min(buf.len());
            buf[..n].copy_from_slice(&available[..n]);
            self.pending_pos += n;
            return Ok(n);
        }

        // Pending exhausted: read and decrypt the next Noise message.
        loop {
            self.pending = self.read_one_message()?;
            self.pending_pos = 0;
            if !self.pending.is_empty() {
                break;
            }
            // A zero-plaintext Noise message carries no data; read the next one.
        }

        let available = &self.pending[self.pending_pos..];
        let n = available.len().min(buf.len());
        buf[..n].copy_from_slice(&available[..n]);
        self.pending_pos += n;
        Ok(n)
    }
}

impl Write for NoiseStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // One `write` call carries exactly one Noise message; chunk large writes
        // so each ciphertext stays within the Noise maximum message size.
        let chunk = &buf[..buf.len().min(MAX_NOISE_PLAINTEXT)];
        let mut enc = vec![0u8; chunk.len() + 16];
        let n = self.state.write_message(chunk, &mut enc).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Noise encrypt failed: {e}"))
        })?;
        enc.truncate(n);

        // Frame: [u32 LE ciphertext length][ciphertext bytes]
        let mut frame = vec![0u8; 4 + enc.len()];
        frame[..4].copy_from_slice(&(enc.len() as u32).to_le_bytes());
        frame[4..].copy_from_slice(&enc);
        self.tcp.write_all(&frame)?;

        Ok(chunk.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // The TCP socket has no application-level buffering to flush here.
        Ok(())
    }
}

impl AsyncRead for NoiseStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let remaining = buf.remaining();
        if remaining == 0 {
            return Poll::Ready(Ok(()));
        }
        // Read into a stack buffer owned by the blocking closure. Note that the
        // closure must return the buffer: `[u8; 8192]` is `Copy`, so a `move`
        // closure would copy `tmp` and the decrypted bytes would be written to
        // the copy while the outer `tmp` (used by `put_slice` below) stays zeroed.
        let want = remaining.min(8192);
        let (n, mut tmp) = block_in_place(move || {
            let mut tmp = [0u8; 8192];
            let n = std::io::Read::read(this, &mut tmp[..want])?;
            Ok::<(usize, [u8; 8192]), io::Error>((n, tmp))
        })?;
        let n = n.min(remaining);
        buf.put_slice(&tmp[..n]);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for NoiseStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let n = block_in_place(move || std::io::Write::write(this, buf))?;
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Listener for incoming peer connections
pub struct PeerListener {
    pub listener: tokio::net::TcpListener,
    /// Local node identity: static Noise keypair used for the responder handshake.
    pub identity: NodeIdentity,
}

impl PeerListener {
    /// Creates a new listener on the given address
    pub async fn bind(address: &str, identity: NodeIdentity) -> Result<Self, String> {
        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|e| format!("Bind failed: {}", e))?;

        Ok(Self { listener, identity })
    }

    /// Accepts an incoming connection
    ///
    /// Performs a TCP accept, then runs the synchronous Noise XX responder
    /// handshake inside [`tokio::task::block_in_place`]. Derives the peer's
    /// [`NodeId`] from the remote static public key fingerprint
    /// (`blake3(peer_static_pub_key)`) and wraps the socket in a [`NoiseStream`].
    pub async fn accept(&mut self) -> Result<PeerConnection, String> {
        let (tcp, addr) = self
            .listener
            .accept()
            .await
            .map_err(|e| format!("Accept failed: {}", e))?;

        // tokio sockets are non-blocking; switch to blocking mode so the
        // synchronous Noise handshake (and the blocking IO used by NoiseStream)
        // can run inside `block_in_place`.
        let mut std_tcp = tcp
            .into_std()
            .map_err(|e| format!("Failed to take ownership of socket: {}", e))?;
        std_tcp
            .set_nonblocking(false)
            .map_err(|e| format!("Failed to set socket to blocking mode: {}", e))?;

        let address = addr.to_string();

        // The Noise handshake is synchronous CPU + blocking socket IO, so run
        // it off the tokio reactor.
        let state = block_in_place(|| perform_responder_handshake(&mut std_tcp, &self.identity))?;

        // Derive the peer's NodeId from their static public key fingerprint.
        let peer_static = state
            .get_remote_static()
            .ok_or_else(|| "Peer did not send a static key".to_string())?;
        let peer_static_array: [u8; 32] = peer_static[..32]
            .try_into()
            .map_err(|_| "Invalid remote static key length".to_string())?;
        let node_id = compute_node_id(&peer_static_array);

        Ok(PeerConnection {
            node_id,
            address,
            stream: NoiseStream {
                state,
                tcp: std_tcp,
                pending: Vec::new(),
                pending_pos: 0,
            },
        })
    }

    /// Returns the listener's local address
    pub fn local_addr(&self) -> Result<std::net::SocketAddr, String> {
        self.listener
            .local_addr()
            .map_err(|e| format!("Local addr failed: {}", e))
    }
}

/// Perform the synchronous Noise XX responder handshake over a blocking TCP socket.
///
/// Returns the transport-mode [`snow::TransportState`] once the handshake
/// completes. The handshake exchanges:
///
///   1. `<- e`        (initiator's ephemeral public key)
///   2. `-> e, ee, s, es` (our ephemeral + our static key, encrypted)
///   3. `<- s, se`    (initiator's static key, encrypted)
fn perform_responder_handshake(
    tcp: &mut StdTcpStream,
    identity: &NodeIdentity,
) -> Result<TransportState, String> {
    let params: snow::params::NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s"
        .parse()
        .map_err(|e| format!("Invalid noise params: {e}"))?;

    let secret_key = identity.secret_key();
    let mut hs = Builder::new(params)
        .local_private_key(&secret_key)
        .map_err(|e| format!("Failed to set local static key: {e}"))?
        .build_responder()
        .map_err(|e| format!("Failed to build responder handshake state: {e}"))?;

    let mut msg_buf = [0u8; 65535];
    let mut resp_buf = [0u8; 65535];

    // 1. <- e (initiator's first message: ephemeral key only)
    let init_msg =
        recv_frame(tcp, &mut msg_buf).map_err(|e| format!("Handshake recv (msg 1) failed: {e}"))?;
    hs.read_message(init_msg, &mut resp_buf)
        .map_err(|e| format!("Handshake read (msg 1) failed: {e}"))?;

    // 2. -> e, ee, s, es (our response; authenticates our static key)
    let len = hs
        .write_message(&[], &mut msg_buf)
        .map_err(|e| format!("Handshake write (msg 2) failed: {e}"))?;
    send_frame(tcp, &msg_buf[..len]).map_err(|e| format!("Handshake send (msg 2) failed: {e}"))?;

    // 3. <- s, se (initiator's third message; authenticates their static key)
    let resp = recv_frame(tcp, &mut resp_buf)
        .map_err(|e| format!("Handshake recv (msg 3) failed: {e}"))?;
    hs.read_message(resp, &mut msg_buf)
        .map_err(|e| format!("Handshake read (msg 3) failed: {e}"))?;

    hs.into_transport_mode()
        .map_err(|e| format!("Handshake finalize (into transport mode) failed: {e}"))
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

    #[test]
    fn test_node_identity_generate() {
        let identity = NodeIdentity::generate().expect("failed to generate identity");
        assert_ne!(identity.node_id(), [0u8; 32], "node_id must be non-zero");
        assert_ne!(
            identity.public_key(),
            [0u8; 32],
            "public key must be non-zero"
        );
        assert_ne!(
            identity.secret_key(),
            [0u8; 32],
            "secret key must be non-zero"
        );

        // node_id must equal blake3(public_key)
        let mut hasher = Hasher::new();
        hasher.update(&identity.public_key());
        let output = hasher.finalize();
        let bytes = output.as_bytes();
        let mut expected = [0u8; 32];
        expected.copy_from_slice(&bytes[..32]);
        assert_eq!(identity.node_id(), expected);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_noise_handshake_roundtrip() {
        let initiator_identity =
            NodeIdentity::generate().expect("failed to generate initiator identity");
        let responder_identity =
            NodeIdentity::generate().expect("failed to generate responder identity");

        let responder_node_id = responder_identity.node_id();
        let initiator_node_id = initiator_identity.node_id();
        let responder_pub_key = responder_identity.public_key();
        let initiator_pub_key = initiator_identity.public_key();

        // Responder binds a listener on an ephemeral port and awaits a connection.
        let mut listener = PeerListener::bind("127.0.0.1:0", responder_identity)
            .await
            .expect("failed to bind listener");
        let addr = listener.local_addr().expect("failed to get local address");

        let accept_handle = tokio::spawn(async move { listener.accept().await });

        // Initiator connects to the responder, asserting its expected node_id.
        let initiator = PeerConnection::connect(
            &initiator_identity,
            &addr.to_string(),
            Some(responder_node_id),
        )
        .await
        .expect("initiator handshake failed");

        let responder = accept_handle
            .await
            .expect("accept task panicked")
            .expect("responder handshake failed");

        // Both sides derive the same node_id from the remote static key.
        assert_eq!(initiator.node_id(), responder_node_id);
        assert_eq!(responder.node_id(), initiator_node_id);

        // The derived node_id matches blake3(peer_static_pub_key) on both sides.
        assert_eq!(responder.node_id(), compute_node_id(&initiator_pub_key));
        assert_eq!(initiator.node_id(), compute_node_id(&responder_pub_key));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_noise_encrypt_decrypt_roundtrip() {
        let initiator_identity =
            NodeIdentity::generate().expect("failed to generate initiator identity");
        let responder_identity =
            NodeIdentity::generate().expect("failed to generate responder identity");

        let responder_node_id = responder_identity.node_id();

        let mut listener = PeerListener::bind("127.0.0.1:0", responder_identity)
            .await
            .expect("failed to bind listener");
        let addr = listener.local_addr().expect("failed to get local address");

        let accept_handle = tokio::spawn(async move { listener.accept().await });

        let mut initiator = PeerConnection::connect(
            &initiator_identity,
            &addr.to_string(),
            Some(responder_node_id),
        )
        .await
        .expect("initiator handshake failed");

        let mut responder = accept_handle
            .await
            .expect("accept task panicked")
            .expect("responder handshake failed");

        // Initiator sends a Ping through the encrypted Noise stream.
        let ping = NetworkMessage::Ping(crate::network::PingMessage {
            nonce: 7,
            timestamp: 12345,
        });
        initiator
            .send_message(&ping)
            .await
            .expect("failed to send ping");

        let received = responder
            .receive_message()
            .await
            .expect("failed to receive ping");
        match received {
            NetworkMessage::Ping(p) => {
                assert_eq!(p.nonce, 7);
                assert_eq!(p.timestamp, 12345);
            }
            _ => panic!("unexpected message type: {received:?}"),
        }

        // Responder replies with a Pong, verifying bidirectional encryption.
        let pong = NetworkMessage::Pong(crate::network::PongMessage {
            nonce: 8,
            timestamp: 999,
        });
        responder
            .send_message(&pong)
            .await
            .expect("failed to send pong");

        let received_pong = initiator
            .receive_message()
            .await
            .expect("failed to receive pong");
        match received_pong {
            NetworkMessage::Pong(p) => {
                assert_eq!(p.nonce, 8);
                assert_eq!(p.timestamp, 999);
            }
            _ => panic!("unexpected message type: {received_pong:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fingerprint_mismatch_rejected() {
        let initiator_identity =
            NodeIdentity::generate().expect("failed to generate initiator identity");
        let responder_identity =
            NodeIdentity::generate().expect("failed to generate responder identity");
        let wrong_identity = NodeIdentity::generate().expect("failed to generate wrong identity");
        let wrong_node_id = wrong_identity.node_id();

        let mut listener = PeerListener::bind("127.0.0.1:0", responder_identity)
            .await
            .expect("failed to bind listener");
        let addr = listener.local_addr().expect("failed to get local address");

        let accept_handle = tokio::spawn(async move { listener.accept().await });

        let result =
            PeerConnection::connect(&initiator_identity, &addr.to_string(), Some(wrong_node_id))
                .await;

        assert!(
            result.is_err(),
            "connect with a wrong expected_node_id should fail"
        );
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected an error"),
        };
        assert!(
            err.contains("fingerprint mismatch"),
            "expected fingerprint mismatch error, got: {err}"
        );

        // The responder completes the handshake: the fingerprint check is
        // initiator-side only.
        let _responder = accept_handle
            .await
            .expect("accept task panicked")
            .expect("responder handshake failed");
    }
}
