//! Keystore persistence for the CLI wallet.
//!
//! Keys are stored as a JSON map of address (hex) -> secret key (hex) at
//! ~/.atoll/keystore.json. The in-memory representation is
//! `HashMap<Address, Vec<u8>>`, faithful to the wallet plan.

use atoll_core::primitives::address::Address;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// On-disk keystore: maps addresses to Ed25519 secret key bytes.
pub struct Keystore {
    path: PathBuf,
    pub keypairs: HashMap<Address, Vec<u8>>,
}

impl Keystore {
    /// Opens (or creates) the keystore at `path`.
    pub fn load(path: PathBuf) -> Result<Self, crate::WalletError> {
        let path = expand_home(path);
        if !path.exists() {
            return Ok(Self {
                path,
                keypairs: HashMap::new(),
            });
        }
        let content = fs::read_to_string(&path)?;
        let map: HashMap<String, String> = serde_json::from_str(&content)?;
        let mut keypairs = HashMap::new();
        for (address_hex, secret_hex) in map {
            let address = Address::from_hex(&address_hex).map_err(|e| {
                crate::WalletError::Address(format!("invalid address {address_hex}: {e}"))
            })?;
            let secret_key = hex::decode(&secret_hex).map_err(crate::WalletError::Hex)?;
            keypairs.insert(address, secret_key);
        }
        Ok(Self { path, keypairs })
    }

    /// Persists the keystore to disk.
    pub fn save(&self) -> Result<(), crate::WalletError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let map: HashMap<String, String> = self
            .keypairs
            .iter()
            .map(|(address, secret_key)| (address.to_hex(), hex::encode(secret_key)))
            .collect();
        let json = serde_json::to_string_pretty(&map)?;
        fs::write(&self.path, json)?;
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

        let mut keystore = Keystore::load(path.clone()).unwrap();
        keystore.add(address, secret_key.clone());
        keystore.save().unwrap();

        let loaded = Keystore::load(path.clone()).unwrap();
        assert_eq!(loaded.get(&address), Some(secret_key));
        assert!(loaded.get(&Address::from_bytes([2u8; 20])).is_none());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_empty_keystore() {
        let path = temp_path("missing");
        let _ = fs::remove_file(&path);
        let keystore = Keystore::load(path).unwrap();
        assert!(keystore.keypairs.is_empty());
    }
}
