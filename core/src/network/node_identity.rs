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
//
//! The secret key is persisted to disk in plaintext by default.
//! Encryption at rest (passphrase-derived key wrapping with XChaCha20-Poly1305)
//! is available via [`NodeIdentity::save_with_passphrase`] /
//! [`NodeIdentity::load_with_passphrase`].

use std::fs;
use std::io;
use std::path::Path;

use aead::Aead;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use blake3::Hasher;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use hybrid_array::Array;
use rand::{rngs::OsRng, RngCore};
use snow::Builder;
use zeroize::Zeroize;

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
#[derive(Clone)]
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
    ///
    /// # Deprecation
    ///
    /// Plaintext persistence exposes the secret key on disk. Use
    /// [`NodeIdentity::save_with_passphrase`] instead, which encrypts at rest
    /// with a passphrase-derived XChaCha20-Poly1305 key.
    #[deprecated(
        since = "1.0.0",
        note = "plaintext save exposes the secret key; use save_with_passphrase instead"
    )]
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut buf = [0u8; IDENTITY_FILE_LEN];
        buf[..SECRET_KEY_LEN].copy_from_slice(&self.secret_key);
        buf[SECRET_KEY_LEN..].copy_from_slice(&self.public_key);
        fs::write(path, &buf)
    }

    /// Loads a node identity from disk at `path`.
    ///
    /// Expects a 64-byte plaintext file written by [`NodeIdentity::save`].
    /// If the file does not exist, a new random identity is generated and
    /// persisted.
    ///
    /// The file format is auto-detected: a 64-byte file is treated as plaintext,
    /// while a file starting with the encrypted format version byte (`0x01`)
    /// requires a passphrase and must be loaded with [`NodeIdentity::load_with_passphrase`].
    /// If the file exists but is neither, an error is returned.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Self::generate().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("key generation failed: {e}"))
            });
        }

        let bytes = fs::read(path)?;

        // Plaintext file: exactly 64 bytes.
        if bytes.len() == IDENTITY_FILE_LEN {
            return Self::load_plaintext(&bytes);
        }

        // Encrypted format detected ([0x01 || salt(16) || nonce(24) || ciphertext]):
        // a passphrase is required to decrypt it.
        if bytes.first() == Some(&ENCRYPTED_FORMAT_VERSION) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "node identity file is encrypted; passphrase required. Use load_with_passphrase",
            ));
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "node identity file at {} contains {} bytes, expected {} (plaintext)",
                path.display(),
                bytes.len(),
                IDENTITY_FILE_LEN
            ),
        ))
    }

    /// Persists the static keypair to disk at `path`, encrypted at rest with
    /// a passphrase-derived key.
    ///
    /// Uses argon2id to derive a 256-bit key from `passphrase`, then XChaCha20-Poly1305
    /// to authenticate and encrypt `[secret_key(32) || public_key(32)]`.
    /// File format: `[0x01 || salt(16) || nonce(24) || ciphertext]` where ciphertext
    /// is the XChaCha20-Poly1305 encryption of the 64-byte keypair.
    ///
    /// The passphrase is zeroized after key derivation so it does not linger in memory.
    ///
    /// # Errors
    ///
    /// Returns an error if key derivation, encryption, or file writing fails.
    pub fn save_with_passphrase(&self, path: impl AsRef<Path>, passphrase: &str) -> io::Result<()> {
        let path = path.as_ref();

        // Derive a 256-bit encryption key from the passphrase using argon2id.
        let mut salt = [0u8; SALT_LEN];
        OsRng.fill_bytes(&mut salt);
        let mut key_bytes = [0u8; KEY_LEN];
        Self::derive_key(passphrase, &salt, &mut key_bytes)?;

        // Plaintext: [secret_key(32) || public_key(32)]
        let mut plaintext = [0u8; IDENTITY_FILE_LEN];
        plaintext[..SECRET_KEY_LEN].copy_from_slice(&self.secret_key);
        plaintext[SECRET_KEY_LEN..].copy_from_slice(&self.public_key);

        // Random nonce for XChaCha20-Poly1305.
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);

        let cipher = XChaCha20Poly1305::new_from_slice(&key_bytes).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("cipher creation failed: {e}"))
        })?;
        let nonce = Array::from(nonce_bytes);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_slice())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("encryption failed: {e}")))?;
        key_bytes.zeroize();

        // File format: [0x01 || salt(16) || nonce(24) || ciphertext]
        let mut buf = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + ciphertext.len());
        buf.push(ENCRYPTED_FORMAT_VERSION);
        buf.extend_from_slice(&salt);
        buf.extend_from_slice(&nonce_bytes);
        buf.extend_from_slice(&ciphertext);
        fs::write(path, buf)
    }

    /// Derives a 256-bit argon2id key from `passphrase` and `salt` into `out`.
    ///
    /// `passphrase` is zeroized after derivation.
    fn derive_key(
        passphrase: &str,
        salt: &[u8; SALT_LEN],
        out: &mut [u8; KEY_LEN],
    ) -> io::Result<()> {
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(ARGON2_MEM_COST, ARGON2_TIME_COST, ARGON2_LANES, None).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("argon2 params failed: {e}"))
            })?,
        );
        let mut passphrase_bytes: Vec<u8> = passphrase.as_bytes().to_vec();
        let result = argon2.hash_password_into(&passphrase_bytes, salt, out);
        passphrase_bytes.zeroize();
        result.map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("argon2 key derivation failed: {e}"),
            )
        })
    }

    /// Loads a node identity from disk at `path`, decrypting with `passphrase`
    /// if the file is in encrypted format.
    ///
    /// Supports both plaintext (64 bytes) and encrypted format
    /// (`[0x01 || salt(16) || nonce(24) || ciphertext]`). For plaintext files,
    /// `passphrase` is ignored.
    ///
    /// Encryption is authenticated (XChaCha20-Poly1305), so a wrong passphrase or
    /// corrupted file produces an error rather than garbage data.
    ///
    /// The passphrase is zeroized after key derivation so it does not linger in memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, is corrupted, or the passphrase
    /// is wrong (decryption/authentication failure).
    pub fn load_with_passphrase(path: impl AsRef<Path>, passphrase: &str) -> io::Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;

        if bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "node identity file is empty",
            ));
        }

        // Plaintext: exactly 64 bytes (passphrase ignored for backward compatibility).
        if bytes.len() == IDENTITY_FILE_LEN {
            return Self::load_plaintext(&bytes);
        }

        // Encrypted: [0x01 || salt(16) || nonce(24) || ciphertext]
        if bytes.first() != Some(&ENCRYPTED_FORMAT_VERSION) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unknown node identity file format",
            ));
        }

        Self::load_encrypted(&bytes, passphrase)
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
    ///
    /// Format: [version(1) || salt(16) || nonce(24) || ciphertext]
    fn load_encrypted(bytes: &[u8], passphrase: &str) -> io::Result<Self> {
        const HEADER_LEN: usize = 1 + SALT_LEN + NONCE_LEN; // 41
        if bytes.len() < HEADER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file too short for encrypted header",
            ));
        }

        let salt = &bytes[1..1 + SALT_LEN];
        let nonce_bytes: [u8; NONCE_LEN] = bytes[1 + SALT_LEN..HEADER_LEN]
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid nonce length"))?;
        let ciphertext = &bytes[HEADER_LEN..];

        // Derive key from passphrase.
        let mut key_bytes = [0u8; KEY_LEN];
        Self::derive_key(
            passphrase,
            salt.try_into()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid salt length"))?,
            &mut key_bytes,
        )?;

        let cipher = XChaCha20Poly1305::new_from_slice(&key_bytes).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("cipher creation failed: {e}"))
        })?;
        let nonce = Array::from(nonce_bytes);
        let plaintext = cipher.decrypt(&nonce, ciphertext).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("decryption failed (wrong passphrase or corrupted file): {e}"),
            )
        })?;
        key_bytes.zeroize();

        if plaintext.len() != IDENTITY_FILE_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "decrypted plaintext contains {} bytes, expected {}",
                    plaintext.len(),
                    IDENTITY_FILE_LEN
                ),
            ));
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
    #[allow(deprecated)]
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
    fn wrong_passphrase_returns_error() {
        let path = temp_path("wrong-pass");
        let _ = fs::remove_file(&path);

        let original = NodeIdentity::generate().expect("failed to generate identity");
        original
            .save_with_passphrase(&path, "correct-password")
            .expect("failed to save encrypted identity");

        // XChaCha20-Poly1305 is authenticated: a wrong passphrase must fail.
        let wrong = NodeIdentity::load_with_passphrase(&path, "wrong-password");
        assert!(
            wrong.is_err(),
            "wrong passphrase should fail decryption, not produce garbage"
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_rejects_encrypted_file_without_passphrase() {
        let path = temp_path("encrypted-no-pass");
        let _ = fs::remove_file(&path);

        let original = NodeIdentity::generate().expect("failed to generate identity");
        original
            .save_with_passphrase(&path, "some-password")
            .expect("failed to save encrypted identity");

        // `load` takes no passphrase and must reject the encrypted file.
        let result = NodeIdentity::load(&path);
        assert!(
            result.is_err(),
            "loading an encrypted file without a passphrase should fail"
        );

        // `load_with_passphrase` with the right passphrase still works.
        let loaded = NodeIdentity::load_with_passphrase(&path, "some-password")
            .expect("failed to load encrypted identity");
        assert_eq!(loaded.node_id(), original.node_id());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn encrypted_file_has_expected_format() {
        let path = temp_path("format");
        let _ = fs::remove_file(&path);

        let original = NodeIdentity::generate().expect("failed to generate identity");
        original
            .save_with_passphrase(&path, "test-pass")
            .expect("failed to save encrypted identity");

        let bytes = fs::read(&path).expect("failed to read encrypted file");
        assert_eq!(
            bytes[0], ENCRYPTED_FORMAT_VERSION,
            "file should start with version byte"
        );
        // [0x01 || salt(16) || nonce(24) || ciphertext(64+16 tag)]
        assert_eq!(
            bytes.len(),
            1 + SALT_LEN + NONCE_LEN + IDENTITY_FILE_LEN + 16
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    #[allow(deprecated)]
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
