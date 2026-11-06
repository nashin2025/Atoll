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

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use blake3::Hasher;
use rand::{rngs::OsRng, RngCore};
use snow::Builder;

/// On-disk identity file: 64 raw bytes plaintext, or encrypted format:
/// `[0x01 || salt(16) || nonce(24) || ciphertext]` where ciphertext is
/// XChaCha20-Poly1305 encryption of `[secret_key(32) || public_key(32)]`
/// using an argon2id-derived key.
const IDENTITY_FILE_LEN: usize = 64;
const ENCRYPTED_FORMAT_VERSION: u8 = 0x01;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;
const ARGON2_TIME_COST: u32 = 3;
const ARGON2_MEM_COST: u32 = 65536; // 64 MiB
const ARGON2_LANES: u32 = 4;
const SECRET_KEY_LEN: usize = 32;
const PUBLIC_KEY_LEN: usize = 32;

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
        let params: snow::params::NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s".parse()?;
        let keypair = Builder::new(params).generate_keypair()?;

        let secret_key: [u8; SECRET_KEY_LEN] =
            keypair.private[..SECRET_KEY_LEN].try_into().unwrap();
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

    /// Persists the static keypair to disk at `path`, encrypted at rest with
    /// a passphrase-derived key.
    ///
    /// Uses argon2id to derive a 256-bit key from `passphrase`, then XORs
    /// `[secret_key || public_key]` with a blake3-derived key stream.
    /// File format: `[0x01 || salt(b64) || \n || nonce(24) || ciphertext]`.
    ///
    /// # Errors
    ///
    /// Returns an error if key derivation, encryption, or file writing fails.
    pub fn save_with_passphrase(&self, path: impl AsRef<Path>, passphrase: &str) -> io::Result<()> {
        let path = path.as_ref();

        // Derive encryption key from passphrase using argon2id
        let salt = SaltString::generate(&mut OsRng);
        let mut key_bytes = [0u8; KEY_LEN];
        Argon2::default()
            .hash_password_into(
                passphrase.as_bytes(),
                salt.as_str().as_bytes(),
                &mut key_bytes,
            )
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("argon2 key derivation failed: {e}"),
                )
            })?;

        // Build plaintext: [secret_key(32) || public_key(32)]
        let mut plaintext = [0u8; IDENTITY_FILE_LEN];
        plaintext[..SECRET_KEY_LEN].copy_from_slice(&self.secret_key);
        plaintext[SECRET_KEY_LEN..].copy_from_slice(&self.public_key);

        // Generate key stream using blake3: blake3(key || counter)[..64]
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let mut ciphertext = [0u8; IDENTITY_FILE_LEN];
        for (i, chunk) in plaintext.chunks_exact(32).enumerate() {
            let mut counter = [0u8; 8];
            counter[..8].copy_from_slice(&(i as u64).to_le_bytes());
            let mut hasher = Hasher::new_keyed(&key_bytes);
            hasher.update(&nonce_bytes);
            hasher.update(&counter);
            let keystream = hasher.finalize().as_bytes().to_vec();
            for (j, byte) in chunk.iter().enumerate() {
                ciphertext[i * 32 + j] = byte ^ keystream[j];
            }
        }

        // Write: [version(1) || salt(b64) || \n || nonce(24) || ciphertext]
        let mut buf = Vec::with_capacity(1 + salt.len() + 1 + NONCE_LEN + IDENTITY_FILE_LEN);
        buf.push(ENCRYPTED_FORMAT_VERSION);
        buf.extend_from_slice(salt.as_str().as_bytes());
        buf.push(b'\n');
        buf.extend_from_slice(&nonce_bytes);
        buf.extend_from_slice(&ciphertext);
        fs::write(path, buf)
    }

    /// Loads a node identity from disk at `path`, decrypting with `passphrase`
    /// if the file is in encrypted format.
    ///
    /// Supports both plaintext (64 bytes) and encrypted format
    /// (`[0x01 || salt(b64) || \n || nonce(24) || ciphertext]`). For plaintext files,
    /// `passphrase` is ignored.
    ///
    /// Note: MVP encryption provides confidentiality but not authentication.
    /// A wrong passphrase will produce garbage data rather than an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or is corrupted.
    pub fn load_with_passphrase(path: impl AsRef<Path>, passphrase: &str) -> io::Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;

        if bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "node identity file is empty",
            ));
        }

        // Detect format: first byte 0x01 = encrypted, otherwise plaintext
        if bytes[0] == ENCRYPTED_FORMAT_VERSION {
            Self::load_encrypted(&bytes, passphrase)
        } else {
            Self::load_plaintext(&bytes)
        }
    }

    /// Loads a plaintext node identity from raw bytes.
    fn load_plaintext(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != IDENTITY_FILE_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "node identity file contains {} bytes, expected {}",
                    bytes.len(),
                    IDENTITY_FILE_LEN
                ),
            ));
        }

        let mut secret_key = [0u8; SECRET_KEY_LEN];
        let mut public_key = [0u8; PUBLIC_KEY_LEN];
        secret_key.copy_from_slice(&bytes[..SECRET_KEY_LEN]);
        public_key.copy_from_slice(&bytes[SECRET_KEY_LEN..]);
        let node_id = Self::compute_node_id(&public_key);

        Ok(Self {
            secret_key,
            public_key,
            node_id,
        })
    }

    /// Loads an encrypted node identity from raw bytes.
    fn load_encrypted(bytes: &[u8], passphrase: &str) -> io::Result<Self> {
        // Format: [version(1) || salt(b64) || \n || nonce(24) || ciphertext]
        let newline_pos = bytes.iter().position(|&b| b == b'\n').ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "missing salt/nonce separator")
        })?;

        if newline_pos < 2 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "salt too short"));
        }

        let salt_str = std::str::from_utf8(&bytes[1..newline_pos]).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid salt encoding: {e}"),
            )
        })?;
        let salt = SaltString::from_b64(salt_str).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("invalid salt: {e}"))
        })?;

        let nonce_start = newline_pos + 1;
        if bytes.len() < nonce_start + NONCE_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file too short for nonce",
            ));
        }

        let nonce_bytes: [u8; NONCE_LEN] =
            bytes[nonce_start..nonce_start + NONCE_LEN]
                .try_into()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid nonce length"))?;
        let ciphertext = &bytes[nonce_start + NONCE_LEN..];

        if ciphertext.len() != IDENTITY_FILE_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "ciphertext contains {} bytes, expected {}",
                    ciphertext.len(),
                    IDENTITY_FILE_LEN
                ),
            ));
        }

        // Derive key from passphrase
        let mut key_bytes = [0u8; KEY_LEN];
        Argon2::default()
            .hash_password_into(
                passphrase.as_bytes(),
                salt.as_str().as_bytes(),
                &mut key_bytes,
            )
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("argon2 key derivation failed: {e}"),
                )
            })?;

        // Decrypt: XOR with blake3 key stream (same as encryption)
        let mut plaintext = [0u8; IDENTITY_FILE_LEN];
        for (i, chunk) in ciphertext.chunks_exact(32).enumerate() {
            let mut counter = [0u8; 8];
            counter[..8].copy_from_slice(&(i as u64).to_le_bytes());
            let mut hasher = Hasher::new_keyed(&key_bytes);
            hasher.update(&nonce_bytes);
            hasher.update(&counter);
            let keystream = hasher.finalize().as_bytes().to_vec();
            for (j, byte) in chunk.iter().enumerate() {
                plaintext[i * 32 + j] = byte ^ keystream[j];
            }
        }

        let mut secret_key = [0u8; SECRET_KEY_LEN];
        let mut public_key = [0u8; PUBLIC_KEY_LEN];
        secret_key.copy_from_slice(&plaintext[..SECRET_KEY_LEN]);
        public_key.copy_from_slice(&plaintext[SECRET_KEY_LEN..]);
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

    #[test]
    fn save_load_with_passphrase_roundtrip_preserves_identity() {
        let path = temp_path("encrypted");
        let _ = fs::remove_file(&path);

        let original = NodeIdentity::generate().expect("failed to generate identity");
        original
            .save_with_passphrase(&path, "correct-horse-battery-staple")
            .expect("failed to save encrypted identity");

        let loaded = NodeIdentity::load_with_passphrase(&path, "correct-horse-battery-staple")
            .expect("failed to load encrypted identity");
        assert_eq!(loaded.node_id(), original.node_id());
        assert_eq!(loaded.public_key(), original.public_key());
        assert_eq!(loaded.secret_key(), original.secret_key());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn wrong_passphrase_produces_different_data() {
        let path = temp_path("wrong-pass");
        let _ = fs::remove_file(&path);

        let original = NodeIdentity::generate().expect("failed to generate identity");
        original
            .save_with_passphrase(&path, "correct-password")
            .expect("failed to save encrypted identity");

        // Wrong passphrase produces garbage (no authentication in MVP encryption)
        let wrong = NodeIdentity::load_with_passphrase(&path, "wrong-password");
        assert!(
            wrong.is_ok(),
            "wrong passphrase should produce garbage, not error"
        );
        let wrong_identity = wrong.unwrap();
        assert_ne!(
            wrong_identity.node_id(),
            original.node_id(),
            "wrong passphrase should produce different data"
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_with_passphrase_handles_plaintext_file() {
        let path = temp_path("plaintext-via-passphrase");
        let _ = fs::remove_file(&path);

        let original = NodeIdentity::generate().expect("failed to generate identity");
        original
            .save(&path)
            .expect("failed to save plaintext identity");

        // load_with_passphrase should handle plaintext files too (ignoring passphrase)
        let loaded = NodeIdentity::load_with_passphrase(&path, "any-passphrase")
            .expect("failed to load plaintext identity with passphrase");
        assert_eq!(loaded.node_id(), original.node_id());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn encrypted_file_detects_format_version() {
        let path = temp_path("format-detect");
        let _ = fs::remove_file(&path);

        let original = NodeIdentity::generate().expect("failed to generate identity");
        original
            .save_with_passphrase(&path, "test-pass")
            .expect("failed to save encrypted identity");

        // Verify the file starts with the encrypted format version byte
        let bytes = fs::read(&path).expect("failed to read encrypted file");
        assert_eq!(
            bytes[0], ENCRYPTED_FORMAT_VERSION,
            "file should start with encrypted format version"
        );

        let _ = fs::remove_file(&path);
    }
}
