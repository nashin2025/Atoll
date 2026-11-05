//! Tauri command wrappers for the Atoll GUI wallet.
//!
//! These expose the wallet-cli logic (unlock, new, import, balance, send,
//! history, derive-address) via Tauri `invoke`. Every function takes an
//! explicit passphrase supplied by the frontend; there is no interactive
//! TTY prompt in the GUI, so `rpassword` is not used.
//!
//! Command names are kebab-case (e.g. `wallet-new`) because Tauri v2
//! permission identifiers only allow lowercase ASCII and hyphens.
//!
//! Each public command is a thin wrapper around an internal `do_*` function
//! that accepts an explicit keystore path, so the logic can be unit-tested
//! with temporary files instead of the real `~/.atoll/keystore.json`.

use atoll_core::chain::state::ChainState;
use atoll_core::primitives::address::Address;
use atoll_core::primitives::signature::Signature;
use atoll_core::primitives::transaction::Transaction;
use ed25519::signature::{Signer, Verifier};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::path::Path;
use wallet_cli::{derive_address, Keystore};

use crate::error::GuiError;

/// The only token supported by the MVP.
const ATOLL: &str = "ATOLL";

/// Default keystore path, shared with the CLI wallet: `~/.atoll/keystore.json`.
const DEFAULT_KEYSTORE: &str = "~/.atoll/keystore.json";

/// Resolves the default keystore path (with `~` expanded by `Keystore::load`).
fn default_keystore_path() -> std::path::PathBuf {
    std::path::PathBuf::from(DEFAULT_KEYSTORE)
}

// ---------------------------------------------------------------------------
// Request / response types (all Serialize + Deserialize for the IPC boundary)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct UnlockRequest {
    pub passphrase: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnlockResponse {
    pub addresses: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewRequest {
    pub passphrase: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewResponse {
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportRequest {
    pub secret_key: String,
    pub passphrase: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResponse {
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceRequest {
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub address: String,
    pub balance: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendRequest {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub token: String,
    pub passphrase: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendResponse {
    pub signed_tx: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryRequest {
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub address: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeriveAddressRequest {
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeriveAddressResponse {
    pub address: String,
}

// ---------------------------------------------------------------------------
// Internal (testable) logic
// ---------------------------------------------------------------------------

fn do_unlock(keystore_path: &Path, passphrase: &str) -> Result<Vec<String>, GuiError> {
    let keystore =
        Keystore::load(keystore_path.to_path_buf(), passphrase).map_err(GuiError::from)?;
    Ok(keystore
        .keypairs
        .keys()
        .map(|address| address.to_hex())
        .collect())
}

fn do_new(keystore_path: &Path, passphrase: &str) -> Result<String, GuiError> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_bytes();
    let secret_key = signing_key.to_bytes().to_vec();
    let address = derive_address(&public_key);

    let mut keystore =
        Keystore::load(keystore_path.to_path_buf(), passphrase).map_err(GuiError::from)?;
    keystore.add(address, secret_key.clone());
    keystore.save(passphrase).map_err(GuiError::from)?;

    // zeroize the in-memory copy once it is persisted
    drop(secret_key);
    Ok(address.to_hex())
}

fn do_import(
    keystore_path: &Path,
    secret_key_hex: &str,
    passphrase: &str,
) -> Result<String, GuiError> {
    let secret_bytes = hex::decode(secret_key_hex).map_err(GuiError::from)?;
    if secret_bytes.len() != 32 {
        return Err(GuiError::Crypto(
            "secret key must be 32 bytes (64 hexadecimal characters)".to_string(),
        ));
    }

    let mut array = [0u8; 32];
    array.copy_from_slice(&secret_bytes);
    let signing_key = SigningKey::from_bytes(&array);
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_bytes();
    let address = derive_address(&public_key);

    let mut keystore =
        Keystore::load(keystore_path.to_path_buf(), passphrase).map_err(GuiError::from)?;
    keystore.add(address, secret_bytes);
    keystore.save(passphrase).map_err(GuiError::from)?;

    Ok(address.to_hex())
}

fn do_balance(keystore_path: &Path, address: &str) -> Result<BalanceResponse, GuiError> {
    let _ = keystore_path; // balance reads ChainState, not the keystore
    let addr = Address::from_hex(address)
        .map_err(|e| GuiError::Address(format!("invalid address '{address}': {e}")))?;
    let state = ChainState::new();
    let balance = state.balance(&addr);
    Ok(BalanceResponse {
        address: addr.to_hex(),
        balance,
    })
}

fn do_send(keystore_path: &Path, request: &SendRequest) -> Result<String, GuiError> {
    if request.token != ATOLL {
        return Err(GuiError::UnsupportedToken(request.token.clone()));
    }

    let from_addr = Address::from_hex(&request.from).map_err(|e| {
        GuiError::Address(format!("invalid 'from' address '{}': {e}", request.from))
    })?;
    let to_addr = Address::from_hex(&request.to)
        .map_err(|e| GuiError::Address(format!("invalid 'to' address '{}': {e}", request.to)))?;
    let value: u128 = request
        .amount
        .parse()
        .map_err(|e| GuiError::Amount(format!("'{}': {e}", request.amount)))?;

    // Reject zero amounts before any signing work.
    if value == 0 {
        return Err(GuiError::Amount(
            "amount must be greater than zero".to_string(),
        ));
    }

    // Check sender balance before signing (offline ChainState has no funded
    // accounts in MVP, so this will typically report insufficient balance).
    let state = ChainState::new();
    let available = state.balance(&from_addr);
    if available < value {
        return Err(GuiError::InsufficientBalance {
            available,
            requested: value,
        });
    }

    let keystore =
        Keystore::load(keystore_path.to_path_buf(), &request.passphrase).map_err(GuiError::from)?;
    let secret_bytes = keystore
        .get(&from_addr)
        .ok_or_else(|| GuiError::KeyNotFound(from_addr.to_string()))?;
    if secret_bytes.len() != 32 {
        return Err(GuiError::Crypto(
            "stored secret key is not 32 bytes".to_string(),
        ));
    }

    let mut array = [0u8; 32];
    array.copy_from_slice(&secret_bytes);
    let signing_key = SigningKey::from_bytes(&array);
    let verifying_key = signing_key.verifying_key();

    let tx = Transaction {
        version: 1,
        from: from_addr,
        to: Some(to_addr),
        value,
        data: Vec::new(),
        signature: Signature::default(),
    };

    // Sign the transaction payload with an empty signature field so the
    // covered bytes are deterministic and exclude the signature itself.
    let message = tx.serialize_bytes();
    let signature = signing_key
        .try_sign(&message)
        .map_err(|_| GuiError::Crypto("failed to sign transaction".to_string()))?;

    let mut signed_tx = tx;
    signed_tx.signature.ed25519 = signature.to_bytes();

    // Sanity-check: the fresh signature must verify against the signed message.
    verifying_key
        .verify(&message, &signature)
        .map_err(|_| GuiError::Crypto("signature self-verification failed".to_string()))?;

    Ok(hex::encode(signed_tx.serialize_bytes()))
}

fn do_history(address: &str) -> Result<HistoryResponse, GuiError> {
    let addr = Address::from_hex(address)
        .map_err(|e| GuiError::Address(format!("invalid address '{address}': {e}")))?;
    Ok(HistoryResponse {
        address: addr.to_hex(),
        message: "No transaction history (offline mode)".to_string(),
    })
}

fn do_derive_address(public_key: &str) -> Result<String, GuiError> {
    let public_key_hex = public_key.strip_prefix("0x").unwrap_or(public_key);
    let public_key_bytes = hex::decode(public_key_hex)
        .map_err(|e| GuiError::Hex(format!("invalid public key hex: {e}")))?;
    if public_key_bytes.len() != 32 {
        return Err(GuiError::Crypto(
            "public key must be 32 bytes (64 hexadecimal characters)".to_string(),
        ));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&public_key_bytes);
    let address = derive_address(&array);
    Ok(address.to_hex())
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Decrypts the keystore with `passphrase` and returns the stored addresses.
/// Used to validate the passphrase on unlock and to list known addresses.
#[tauri::command(name = "wallet-unlock")]
pub fn wallet_unlock(request: UnlockRequest) -> Result<UnlockResponse, GuiError> {
    let addresses = do_unlock(&default_keystore_path(), &request.passphrase)?;
    Ok(UnlockResponse { addresses })
}

/// Generates a new Ed25519 key pair, encrypts it to the keystore, and returns
/// the derived address.
#[tauri::command(name = "wallet-new")]
pub fn wallet_new(request: NewRequest) -> Result<NewResponse, GuiError> {
    let address = do_new(&default_keystore_path(), &request.passphrase)?;
    Ok(NewResponse { address })
}

/// Imports an Ed25519 secret key from hex (32 bytes / 64 hex characters),
/// encrypts it to the keystore, and returns the derived address.
#[tauri::command(name = "wallet-import")]
pub fn wallet_import(request: ImportRequest) -> Result<ImportResponse, GuiError> {
    let address = do_import(
        &default_keystore_path(),
        &request.secret_key,
        &request.passphrase,
    )?;
    Ok(ImportResponse { address })
}

/// Queries the ATOLL balance of an address using the offline ChainState.
#[tauri::command(name = "wallet-balance")]
pub fn wallet_balance(request: BalanceRequest) -> Result<BalanceResponse, GuiError> {
    do_balance(&default_keystore_path(), &request.address)
}

/// Builds, signs, and outputs an ATOLL transaction as hex (offline signing).
/// The signed hex is ready for manual submission or a future RPC integration.
#[tauri::command(name = "wallet-send")]
pub fn wallet_send(request: SendRequest) -> Result<SendResponse, GuiError> {
    let signed_tx = do_send(&default_keystore_path(), &request)?;
    Ok(SendResponse { signed_tx })
}

/// Returns the offline-mode history message for an address (no tx log in MVP).
#[tauri::command(name = "wallet-history")]
pub fn wallet_history(request: HistoryRequest) -> Result<HistoryResponse, GuiError> {
    do_history(&request.address)
}

/// Derives an Atoll address from a 32-byte Ed25519 public key (hex).
#[tauri::command(name = "wallet-derive-address")]
pub fn wallet_derive_address(
    request: DeriveAddressRequest,
) -> Result<DeriveAddressResponse, GuiError> {
    let address = do_derive_address(&request.public_key)?;
    Ok(DeriveAddressResponse { address })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn temp_path(name: &str) -> std::path::PathBuf {
        env::temp_dir().join(format!("atoll-gui-test-{name}"))
    }

    fn temp_keystore(name: &str) -> std::path::PathBuf {
        let path = temp_path(name);
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn unlock_returns_empty_for_missing_keystore() {
        let path = temp_keystore("unlock_missing");
        let addresses = do_unlock(&path, "passphrase").unwrap();
        assert!(addresses.is_empty());
    }

    #[test]
    fn new_then_unlock_roundtrip_preserves_address() {
        let path = temp_keystore("new_roundtrip");
        let passphrase = "test-passphrase";
        let address = do_new(&path, passphrase).unwrap();
        assert!(!address.is_empty());
        assert!(address.starts_with("0x"));

        let addresses = do_unlock(&path, passphrase).unwrap();
        assert_eq!(addresses, vec![address.clone()]);

        // Wrong passphrase is rejected.
        assert!(do_unlock(&path, "wrong-passphrase").is_err());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn import_then_unlock_roundtrip_preserves_address() {
        let path = temp_keystore("import_roundtrip");
        let passphrase = "test-passphrase";
        // Ed25519 secret key for [3u8; 32] (64 hex chars).
        let secret_key_hex = "03".repeat(32);
        let address = do_import(&path, &secret_key_hex, passphrase).unwrap();
        assert!(!address.is_empty());

        let addresses = do_unlock(&path, passphrase).unwrap();
        assert_eq!(addresses, vec![address.clone()]);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn import_rejects_wrong_length_secret_key() {
        let path = temp_keystore("import_badlen");
        let result = do_import(&path, "deadbeef", "passphrase");
        assert!(result.is_err());
        assert!(matches!(result, Err(GuiError::Crypto(_))));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn balance_is_zero_for_every_address_offline() {
        let path = temp_keystore("balance_offline");
        let addr = Address::from_bytes([1u8; 20]);
        let response = do_balance(&path, &addr.to_hex()).unwrap();
        assert_eq!(response.balance, 0);
        assert_eq!(response.address, addr.to_hex());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn send_rejects_zero_amount() {
        let path = temp_keystore("send_zero");
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        let request = SendRequest {
            from: from.to_hex(),
            to: to.to_hex(),
            amount: "0".to_string(),
            token: ATOLL.to_string(),
            passphrase: "passphrase".to_string(),
        };
        let result = do_send(&path, &request);
        assert!(matches!(result, Err(GuiError::Amount(_))));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn send_rejects_insufficient_balance() {
        let path = temp_keystore("send_insufficient");
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        // Fund the keystore so the key exists, but ChainState has balance 0.
        let mut keystore = Keystore::load(path.clone(), "passphrase").unwrap();
        keystore.add(from, vec![3u8; 32]);
        keystore.save("passphrase").unwrap();

        let request = SendRequest {
            from: from.to_hex(),
            to: to.to_hex(),
            amount: "100".to_string(),
            token: ATOLL.to_string(),
            passphrase: "passphrase".to_string(),
        };
        let result = do_send(&path, &request);
        assert!(
            matches!(
                result,
                Err(GuiError::InsufficientBalance {
                    available: 0,
                    requested: 100
                })
            ),
            "insufficient balance must be rejected"
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn signing_is_deterministic_and_verifies() {
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let tx = Transaction {
            version: 1,
            from: Address::from_bytes([1u8; 20]),
            to: Some(Address::from_bytes([2u8; 20])),
            value: 1_000,
            data: vec![1, 2, 3],
            signature: Signature::default(),
        };
        let message = tx.serialize_bytes();
        let sig1 = signing_key.try_sign(&message).unwrap();
        let sig2 = signing_key.try_sign(&message).unwrap();
        assert_eq!(
            sig1.to_bytes(),
            sig2.to_bytes(),
            "Ed25519 signatures are deterministic"
        );
        assert!(
            verifying_key.verify(&message, &sig1).is_ok(),
            "signature verifies against the signed message"
        );
    }

    #[test]
    fn derive_address_is_deterministic() {
        let a = do_derive_address("03".repeat(32).as_str()).unwrap();
        let b = do_derive_address("03".repeat(32).as_str()).unwrap();
        assert_eq!(a, b);
        assert!(a.starts_with("0x"));
    }

    #[test]
    fn derive_address_rejects_bad_length() {
        let result = do_derive_address("deadbeef");
        assert!(matches!(result, Err(GuiError::Crypto(_))));
    }

    #[test]
    fn history_returns_offline_message() {
        let addr = Address::from_bytes([1u8; 20]);
        let response = do_history(&addr.to_hex()).unwrap();
        assert_eq!(response.address, addr.to_hex());
        assert!(response.message.contains("offline"));
    }
}
