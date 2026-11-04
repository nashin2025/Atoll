//! Node identity for the Atoll Protocol encrypted transport.
//!
//! Each node holds a Curve25519 static keypair. The node's network
//! fingerprint (`node_id`) is `blake3(static public key)`.
//!
//! ## snow API adaptation
//!
//! snow 0.10 does not expose a `StaticKey` type. Keypairs are created
//! through [`snow::Builder::generate_keypair`], which returns a
//! [`snow::Keypair`] with raw 32-byte private and public keys. This module
//! wraps that API instead.
//!
//! ## Limitation (MVP)
//!
//! The secret key is persisted to disk in plaintext. Encryption at rest
//! (password-derived key wrapping, keychain integration, etc.) is deferred
//! to a later phase of the H4 encrypted transport work.

use std::fs;
use std::io;
use std::path::Path;

use blake3::Hasher;
use snow::Builder;

/// On-disk identity file: 32-byte secret key followed by 32-byte public key.
const IDENTITY_FILE_LEN: usize = 64;
/// Length of a Curve25519 secret key in bytes.
const SECRET_KEY_LEN: usize = 32;

/// Node identity: a Curve25519 static keypair plus its BLAKE3 fingerprint.
pub struct NodeIdentity {
    /// The node's static Curve25519 secret key.
    secret_key: [u8; 32],
    /// The node's static Curve25519 public key.
    public_key: [u8; 32],
    /// BLAKE3 fingerprint of the public key — the node's network ID.
    node_id: [u8; 32],
}

impl NodeIdentity {
    /// Generates a new random Curve25519 keypair and derives the node ID.
    ///
    /// Uses snow's default crypto resolver (Curve25519 DH, getrandom RNG).
    pub fn generate() -> Result<Self, snow::Error> {
        let params: snow::params::NoiseParams =
            "Noise_XX_25519_ChaChaPoly_BLAKE2s".parse()?;
        let keypair = Builder::new(params).generate_keypair()?;

        let secret_key: [u8; SECRET_KEY_LEN] = keypair.private[..SECRET_KEY_LEN].try_into().unwrap();
        let public_key: [u8; SECRET_KEY_LEN] = keypair.public[..SECRET_KEY_LEN].try_into().unwrap();
        let node_id = Self::compute_node_id(&public_key);

        Ok(Self {
            secret_key,
            public_key,
            node_id,
        })
    }

    /// Returns the node's BLAKE3 fingerprint over the static public key.
    pub fn node_id(&self) -> [u8; 32] {
        self.node_id
    }

    /// Returns the node's static Curve25519 public key.
    pub fn public_key(&self) -> [u8; 32] {
        self.public_key
    }

    /// Returns the node's static Curve25519 secret key.
    pub fn secret_key(&self) -> [u8; 32] {
        self.secret_key
    }

    /// Computes `blake3(public_key)` as `[u8; 32]`.
    fn compute_node_id(public_key: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(public_key);
        let output = hasher.finalize();
        let bytes = output.as_bytes();
        let mut node_id = [0u8; 32];
        node_id.copy_from_slice(&bytes[..32]);
        node_id
    }

    /// Persists the static keypair to disk at `path` as 64 raw bytes:
    /// `[32-byte secret key || 32-byte public key]`.
    ///
    /// ## Limitation (MVP)
    ///
    /// The key is written in plaintext with no encryption at rest.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut buf = [0u8; IDENTITY_FILE_LEN];
        buf[..SECRET_KEY_LEN].copy_from_slice(&self.secret_key);
        buf[SECRET_KEY_LEN..].copy_from_slice(&self.public_key);
        fs::write(path, &buf)
    }

    /// Loads a node identity from disk at `path`.
    ///
    /// Expects a 64-byte file written by [`NodeIdentity::save`].
    /// If the file does not exist, a new random identity is generated and
    /// persisted.
    /// If the file exists but does not contain exactly 64 bytes, an error
    /// is returned.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Self::generate().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("key generation failed: {e}"))
            });
        }

        let bytes = fs::read(path)?;
        if bytes.len() != IDENTITY_FILE_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "node identity file at {:?} contains {} bytes, expected {}",
                    path,
                    bytes.len(),
                    IDENTITY_FILE_LEN
                ),
            ));
        }

        let mut secret_key = [0u8; SECRET_KEY_LEN];
        let mut public_key = [0u8; SECRET_KEY_LEN];
        secret_key.copy_from_slice(&bytes[..SECRET_KEY_LEN]);
        public_key.copy_from_slice(&bytes[SECRET_KEY_LEN..]);
        let node_id = Self::compute_node_id(&public_key);

        Ok(Self {
            secret_key,
            public_key,
            node_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_path(name: &str) -> std::path::PathBuf {
        env::temp_dir().join(format!("atoll_node_identity_test_{}", name))
    }

    #[test]
    fn generate_derives_node_id_from_public_key() {
        let identity = NodeIdentity::generate().expect("failed to generate identity");
        let expected = {
            let mut hasher = Hasher::new();
            hasher.update(&identity.public_key);
            let output = hasher.finalize();
            let bytes = output.as_bytes();
            let mut id = [0u8; 32];
            id.copy_from_slice(&bytes[..32]);
            id
        };
        assert_eq!(identity.node_id(), expected);
    }

    #[test]
    fn generate_produces_distinct_ids() {
        let a = NodeIdentity::generate().expect("failed to generate identity");
        let b = NodeIdentity::generate().expect("failed to generate identity");
        assert_ne!(a.node_id(), b.node_id());
    }

    #[test]
    fn save_load_roundtrip_preserves_identity() {
        let path = temp_path("roundtrip");
        let _ = fs::remove_file(&path);

        let original = NodeIdentity::generate().expect("failed to generate identity");
        original.save(&path).expect("failed to save identity");

        let loaded = NodeIdentity::load(&path).expect("failed to load identity");
        assert_eq!(loaded.node_id(), original.node_id());
        assert_eq!(loaded.public_key(), original.public_key());
        assert_eq!(loaded.secret_key(), original.secret_key());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_generates_new_identity() {
        let path = temp_path("missing");
        let _ = fs::remove_file(&path);

        let loaded = NodeIdentity::load(&path).expect("failed to generate on missing file");
        assert_ne!(loaded.node_id(), [0u8; 32]);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_invalid_file_returns_error() {
        let path = temp_path("invalid");
        let _ = fs::remove_file(&path);
        fs::write(&path, b"not-a-valid-key").expect("failed to write invalid key file");

        let result = NodeIdentity::load(&path);
        assert!(result.is_err(), "loading an invalid key file should fail");

        let _ = fs::remove_file(&path);
    }
}
