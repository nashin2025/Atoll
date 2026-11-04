//! Keystore persistence for the CLI wallet.
//!
//! Keys are stored as an encrypted JSON envelope at ~/.atoll/keystore.json:
//! `{"salt": "<hex>", "nonce": "<hex>", "ciphertext": "<hex>"}` where the
//! ciphertext is the XChaCha20-Poly1305 AEAD encryption of the plaintext
//! JSON map of address (hex) -> secret key (hex), keyed with an argon2id
//! derivation of the user passphrase.
//!
//! The in-memory representation is `HashMap<Address, Vec<u8>>`, faithful to
//! the wallet plan.

use atoll_core::primitives::address::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::crypto::{decrypt_keystore, encrypt_keystore, NONCE_LEN, SALT_LEN};
use crate::WalletError;

/// On-disk encrypted keystore envelope.
#[derive(Debug, Deserialize, Serialize)]
struct Envelope {
    salt: String,
    nonce: String,
    ciphertext: String,
}

/// On-disk keystore: maps addresses to Ed25519 secret key bytes.
pub struct Keystore {
    path: PathBuf,
    pub keypairs: HashMap<Address, Vec<u8>>,
}

impl Keystore {
    /// Opens (or creates) the keystore at `path`, decrypting with `passphrase`.
    ///
    /// A missing file yields an empty keystore. An unencrypted (legacy)
    /// plaintext keystore is rejected with [`WalletError::PlaintextKeystore`]
    /// — there is no silent plaintext fallback.
    pub fn load(path: PathBuf, passphrase: &str) -> Result<Self, WalletError> {
        let path = expand_home(path);
        if !path.exists() {
            return Ok(Self {
                path,
                keypairs: HashMap::new(),
            });
        }
        let content = fs::read_to_string(&path)?;

        // Try the encrypted envelope format first.
        if let Ok(envelope) = serde_json::from_str::<Envelope>(&content) {
            let keypairs = decrypt_envelope(&envelope, passphrase).map_err(WalletError::Crypto)?;
            return Ok(Self { path, keypairs });
        }

        // Reject legacy plaintext keystores explicitly; never silently read them.
        if serde_json::from_str::<HashMap<String, String>>(&content).is_ok() {
            return Err(WalletError::PlaintextKeystore(format!(
                "keystore at {path:?} is stored unencrypted; re-encrypt it with `wallet new --passphrase <passphrase>` (no silent plaintext fallback)"
            )));
        }

        // Unrecognizable / corrupt content.
        Err(WalletError::Json(
            serde_json::from_str::<Envelope>(&content).unwrap_err(),
        ))
    }

    /// Persists the keystore to disk, encrypted with `passphrase`, with
    /// 0600 file permissions (owner read/write only).
    pub fn save(&self, passphrase: &str) -> Result<(), WalletError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let map: HashMap<String, String> = self
            .keypairs
            .iter()
            .map(|(address, secret_key)| (address.to_hex(), hex::encode(secret_key)))
            .collect();
        let plaintext = serde_json::to_string_pretty(&map)?;
        let encrypted =
            encrypt_keystore(plaintext.as_bytes(), passphrase).map_err(WalletError::Crypto)?;
        let envelope = Envelope {
            salt: hex::encode(&encrypted[..SALT_LEN]),
            nonce: hex::encode(&encrypted[SALT_LEN..SALT_LEN + NONCE_LEN]),
            ciphertext: hex::encode(&encrypted[SALT_LEN + NONCE_LEN..]),
        };
        let json = serde_json::to_string_pretty(&envelope)?;
        fs::write(&self.path, json)?;
        set_restrictive_permissions(&self.path)?;
        Ok(())
    }

    /// Adds a key pair to the keystore.
    pub fn add(&mut self, address: Address, secret_key: Vec<u8>) {
        self.keypairs.insert(address, secret_key);
    }

    /// Returns the secret key bytes for `address`, if stored.
    pub fn get(&self, address: &Address) -> Option<Vec<u8>> {
        self.keypairs.get(address).cloned()
    }
}

/// Decrypts an envelope and decodes the plaintext keystore map.
fn decrypt_envelope(
    envelope: &Envelope,
    passphrase: &str,
) -> Result<HashMap<Address, Vec<u8>>, String> {
    let salt = hex::decode(&envelope.salt).map_err(|e| format!("invalid salt hex: {e}"))?;
    let nonce = hex::decode(&envelope.nonce).map_err(|e| format!("invalid nonce hex: {e}"))?;
    let ciphertext =
        hex::decode(&envelope.ciphertext).map_err(|e| format!("invalid ciphertext hex: {e}"))?;
    if salt.len() != SALT_LEN {
        return Err(format!(
            "invalid salt length: {} bytes (expected {SALT_LEN})",
            salt.len()
        ));
    }
    if nonce.len() != NONCE_LEN {
        return Err(format!(
            "invalid nonce length: {} bytes (expected {NONCE_LEN})",
            nonce.len()
        ));
    }
    let mut combined = Vec::with_capacity(salt.len() + nonce.len() + ciphertext.len());
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&ciphertext);
    let plaintext = decrypt_keystore(&combined, passphrase)?;
    let map: HashMap<String, String> = serde_json::from_str(
        std::str::from_utf8(&plaintext)
            .map_err(|e| format!("decrypted plaintext is not valid UTF-8: {e}"))?,
    )
    .map_err(|e| format!("decrypted plaintext is not valid JSON: {e}"))?;
    let mut keypairs = HashMap::new();
    for (address_hex, secret_hex) in map {
        let address = Address::from_hex(&address_hex)
            .map_err(|e| format!("invalid address {address_hex}: {e}"))?;
        let secret_key =
            hex::decode(&secret_hex).map_err(|e| format!("invalid secret key hex: {e}"))?;
        keypairs.insert(address, secret_key);
    }
    Ok(keypairs)
}

/// Sets 0600 (owner read/write only) permissions on `path`. No-op on
/// non-Unix platforms where this concept does not apply.
fn set_restrictive_permissions(path: &Path) -> Result<(), WalletError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Expands a leading `~` to the user's home directory.
fn expand_home(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy().to_string();
    if let Some(home) = std::env::home_dir() {
        if s == "~" {
            return home;
        }
        if let Some(rest) = s.strip_prefix("~/") {
            return home.join(rest);
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("atoll-wallet-test-{name}"))
    }

    #[test]
    fn keystore_roundtrip_preserves_keys() {
        let path = temp_path("roundtrip");
        let _ = fs::remove_file(&path);
        let address = Address::from_bytes([1u8; 20]);
        let secret_key = vec![42u8; 32];
        let passphrase = "test-passphrase";

        let mut keystore = Keystore::load(path.clone(), passphrase).unwrap();
        keystore.add(address, secret_key.clone());
        keystore.save(passphrase).unwrap();

        let loaded = Keystore::load(path.clone(), passphrase).unwrap();
        assert_eq!(loaded.get(&address), Some(secret_key));
        assert!(loaded.get(&Address::from_bytes([2u8; 20])).is_none());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_empty_keystore() {
        let path = temp_path("missing");
        let _ = fs::remove_file(&path);
        let keystore = Keystore::load(path, "test-passphrase").unwrap();
        assert!(keystore.keypairs.is_empty());
    }

    #[test]
    fn wrong_passphrase_is_rejected() {
        let path = temp_path("wrong-passphrase");
        let _ = fs::remove_file(&path);
        let address = Address::from_bytes([1u8; 20]);
        let mut keystore = Keystore::load(path.clone(), "right").unwrap();
        keystore.add(address, vec![42u8; 32]);
        keystore.save("right").unwrap();

        let result = Keystore::load(path.clone(), "wrong");
        assert!(result.is_err(), "wrong passphrase must fail");
        assert!(
            matches!(result, Err(WalletError::Crypto(_))),
            "wrong passphrase must surface a crypto error"
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn plaintext_keystore_is_rejected() {
        let path = temp_path("plaintext");
        let _ = fs::remove_file(&path);
        let map: HashMap<String, String> = HashMap::from([(
            "0x0102030405060708090a0b0c0d0e0f1011121314".to_string(),
            "deadbeef".to_string(),
        )]);
        let json = serde_json::to_string(&map).unwrap();
        fs::write(&path, json).unwrap();

        let result = Keystore::load(path.clone(), "any-passphrase");
        assert!(
            matches!(result, Err(WalletError::PlaintextKeystore(_))),
            "plaintext keystore must be rejected with a clear error"
        );

        let _ = fs::remove_file(&path);
    }

    #[cfg(unix)]
    #[test]
    fn saved_keystore_has_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let path = temp_path("permissions");
        let _ = fs::remove_file(&path);
        let address = Address::from_bytes([1u8; 20]);
        let mut keystore = Keystore::load(path.clone(), "test-passphrase").unwrap();
        keystore.add(address, vec![42u8; 32]);
        keystore.save("test-passphrase").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "keystore file must be 0600");

        let _ = fs::remove_file(&path);
    }
}
