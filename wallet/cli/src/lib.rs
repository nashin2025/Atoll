//! Atoll Protocol CLI Wallet
//!
//! MVP command-line wallet: generate/import Ed25519 keys, query
//! balances, build and sign ATOLL transactions, and view history.
//! All chain operations run against a local, offline ChainState;
//! signed transactions are emitted as hex for manual submission.

#![warn(clippy::all)]

mod crypto;
mod keystore;

pub use keystore::Keystore;

use atoll_core::chain::state::ChainState;
use atoll_core::primitives::address::Address;
use atoll_core::primitives::signature::Signature;
use atoll_core::primitives::transaction::Transaction;
use clap::Parser;
use ed25519::signature::{Signer, Verifier};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::fmt;
use std::path::{Path, PathBuf};

/// The only token supported by the MVP.
const ATOLL: &str = "ATOLL";

/// Derives an Atoll address from an Ed25519 public key:
/// `Address = BLAKE3(public_key)[..20]`.
pub fn derive_address(public_key: &[u8; 32]) -> Address {
    Address::from_public_key(public_key)
}

/// Wallet errors with user-friendly messages.
#[derive(Debug)]
pub enum WalletError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Hex(hex::FromHexError),
    Crypto(String),
    /// The passphrase could not be resolved (missing flag, env var, or prompt input).
    Passphrase(String),
    /// An existing keystore is stored unencrypted plaintext and must be re-encrypted.
    PlaintextKeystore(String),
    Address(String),
    Amount(String),
    KeyNotFound(Address),
    UnsupportedToken(String),
    InsufficientBalance {
        available: u128,
        requested: u128,
    },
}

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalletError::Io(e) => write!(f, "I/O error: {e}"),
            WalletError::Json(e) => write!(f, "serialization error: {e}"),
            WalletError::Hex(e) => write!(f, "invalid hex: {e}"),
            WalletError::Crypto(message) => write!(f, "crypto error: {message}"),
            WalletError::Passphrase(message) => write!(f, "passphrase error: {message}"),
            WalletError::PlaintextKeystore(message) => {
                write!(f, "unencrypted plaintext keystore: {message}")
            }
            WalletError::Address(message) => write!(f, "invalid address: {message}"),
            WalletError::Amount(message) => write!(f, "invalid amount: {message}"),
            WalletError::KeyNotFound(address) => {
                write!(f, "no key found for address {address} in keystore")
            }
            WalletError::UnsupportedToken(token) => write!(
                f,
                "unsupported token '{token}'; only 'ATOLL' is supported in MVP"
            ),
            WalletError::InsufficientBalance {
                available,
                requested,
            } => write!(
                f,
                "insufficient balance: have {available} ATOLL, requested {requested} ATOLL"
            ),
        }
    }
}

impl std::error::Error for WalletError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WalletError::Io(e) => Some(e),
            WalletError::Json(e) => Some(e),
            WalletError::Hex(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WalletError {
    fn from(e: std::io::Error) -> Self {
        WalletError::Io(e)
    }
}

impl From<serde_json::Error> for WalletError {
    fn from(e: serde_json::Error) -> Self {
        WalletError::Json(e)
    }
}

impl From<hex::FromHexError> for WalletError {
    fn from(e: hex::FromHexError) -> Self {
        WalletError::Hex(e)
    }
}

/// Resolves the passphrase: `--passphrase` flag > `ATOLL_PASSPHRASE` env var
/// > interactive stdin prompt.
///
/// Returns an error when no passphrase is available; there is no silent
/// plaintext fallback.
///
/// # Security notes
///
/// The `--passphrase` flag is visible in `ps` output and may be retained in
/// shell history. The `ATOLL_PASSPHRASE` environment variable is visible to
/// other processes running under the same user. For sensitive environments,
/// prefer the interactive prompt (which does not echo input) and run the CLI
/// with restricted permissions so only the intended user can invoke it.
pub fn resolve_passphrase(flag: Option<&str>) -> Result<String, WalletError> {
    if let Some(passphrase) = flag {
        if passphrase.is_empty() {
            return Err(WalletError::Passphrase(
                "the --passphrase flag must not be empty".to_string(),
            ));
        }
        return Ok(passphrase.to_string());
    }
    if let Ok(passphrase) = std::env::var("ATOLL_PASSPHRASE") {
        if passphrase.is_empty() {
            return Err(WalletError::Passphrase(
                "the ATOLL_PASSPHRASE environment variable must not be empty".to_string(),
            ));
        }
        return Ok(passphrase);
    }
    match rpassword::prompt_password("Passphrase: ") {
        Ok(passphrase) => {
            if passphrase.is_empty() {
                return Err(WalletError::Passphrase(
                    "passphrase must not be empty".to_string(),
                ));
            }
            Ok(passphrase)
        }
        Err(_) => Err(WalletError::Passphrase(
            "interactive prompt unavailable in non-TTY environment; use --passphrase or ATOLL_PASSPHRASE"
                .to_string(),
        )),
    }
}

/// CLI entry point.
#[derive(Parser)]
#[command(name = "wallet", about = "Atoll Protocol CLI wallet")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(
        long,
        help = "Keystore file path",
        default_value = "~/.atoll/keystore.json"
    )]
    pub keystore: PathBuf,

    #[arg(long, help = "Node RPC endpoint (stubbed in MVP)")]
    pub node: Option<String>,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    /// Generate a new Ed25519 key pair and save it to the keystore.
    New {
        #[arg(long, help = "Passphrase to encrypt the keystore")]
        passphrase: Option<String>,
    },
    /// Import an Ed25519 secret key from hex (32 bytes / 64 hex characters).
    Import {
        #[arg(help = "Secret key as hex")]
        secret_key: String,
        #[arg(long, help = "Passphrase to encrypt the keystore")]
        passphrase: Option<String>,
    },
    /// Query the ATOLL balance of an address (offline).
    Balance {
        #[arg(help = "Address in hex")]
        address: String,
    },
    /// Build, sign, and output an ATOLL transaction as hex (offline).
    Send {
        #[arg(help = "Sender address in hex")]
        from: String,
        #[arg(help = "Recipient address in hex")]
        to: String,
        #[arg(help = "Amount in ATOLL")]
        amount: String,
        #[arg(help = "Token symbol (ATOLL only)")]
        token: String,
        #[arg(long, help = "Passphrase to encrypt the keystore")]
        passphrase: Option<String>,
    },
    /// Show transaction history for an address (offline).
    History {
        #[arg(help = "Address in hex")]
        address: String,
    },
}

/// Executes the parsed command.
pub fn run(cli: Cli) -> Result<(), WalletError> {
    match cli.command {
        Commands::New { passphrase } => cmd_new(&cli.keystore, passphrase.as_deref()),
        Commands::Import {
            secret_key,
            passphrase,
        } => cmd_import(&cli.keystore, &secret_key, passphrase.as_deref()),
        Commands::Balance { address } => cmd_balance(&address),
        Commands::Send {
            from,
            to,
            amount,
            token,
            passphrase,
        } => cmd_send(
            &cli.keystore,
            &from,
            &to,
            &amount,
            &token,
            passphrase.as_deref(),
        ),
        Commands::History { address } => cmd_history(&address),
    }
}

fn cmd_new(keystore_path: &Path, passphrase: Option<&str>) -> Result<(), WalletError> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_bytes();
    let secret_key = signing_key.to_bytes().to_vec();
    let address = derive_address(&public_key);

    let passphrase = resolve_passphrase(passphrase)?;
    let mut keystore = Keystore::load(keystore_path.to_path_buf(), &passphrase)?;
    keystore.add(address, secret_key);
    keystore.save(&passphrase)?;

    println!("{address}");
    Ok(())
}

fn cmd_import(
    keystore_path: &Path,
    secret_key_hex: &str,
    passphrase: Option<&str>,
) -> Result<(), WalletError> {
    let secret_bytes = hex::decode(secret_key_hex)?;
    if secret_bytes.len() != 32 {
        return Err(WalletError::Crypto(
            "secret key must be 32 bytes (64 hexadecimal characters)".to_string(),
        ));
    }

    let mut array = [0u8; 32];
    array.copy_from_slice(&secret_bytes);
    let signing_key = SigningKey::from_bytes(&array);
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_bytes();
    let address = derive_address(&public_key);

    let passphrase = resolve_passphrase(passphrase)?;
    let mut keystore = Keystore::load(keystore_path.to_path_buf(), &passphrase)?;
    keystore.add(address, secret_bytes);
    keystore.save(&passphrase)?;

    println!("{address}");
    Ok(())
}

fn cmd_balance(address: &str) -> Result<(), WalletError> {
    let addr = Address::from_hex(address)
        .map_err(|e| WalletError::Address(format!("invalid address '{address}': {e}")))?;
    let state = ChainState::new();
    let balance = state.balance(&addr);
    println!("{balance} ATOLL");
    Ok(())
}

fn cmd_send(
    keystore_path: &Path,
    from: &str,
    to: &str,
    amount: &str,
    token: &str,
    passphrase: Option<&str>,
) -> Result<(), WalletError> {
    if token != ATOLL {
        return Err(WalletError::UnsupportedToken(token.to_string()));
    }

    let from_addr = Address::from_hex(from)
        .map_err(|e| WalletError::Address(format!("invalid 'from' address '{from}': {e}")))?;
    let to_addr = Address::from_hex(to)
        .map_err(|e| WalletError::Address(format!("invalid 'to' address '{to}': {e}")))?;
    let value: u128 = amount
        .parse()
        .map_err(|e| WalletError::Amount(format!("'{amount}': {e}")))?;

    // L4: Reject zero amounts before any signing work.
    if value == 0 {
        return Err(WalletError::Amount(
            "amount must be greater than zero".to_string(),
        ));
    }

    // M1: Check sender balance before signing.
    let state = ChainState::new();
    let available = state.balance(&from_addr);
    if available < value {
        return Err(WalletError::InsufficientBalance {
            available,
            requested: value,
        });
    }

    let passphrase = resolve_passphrase(passphrase)?;
    let keystore = Keystore::load(keystore_path.to_path_buf(), &passphrase)?;
    let secret_bytes = keystore
        .get(&from_addr)
        .ok_or(WalletError::KeyNotFound(from_addr))?;
    if secret_bytes.len() != 32 {
        return Err(WalletError::Crypto(
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
        .map_err(|_| WalletError::Crypto("failed to sign transaction".to_string()))?;

    let mut signed_tx = tx;
    signed_tx.signature.ed25519 = signature.to_bytes();

    // Sanity-check: the fresh signature must verify against the signed message.
    verifying_key
        .verify(&message, &signature)
        .map_err(|_| WalletError::Crypto("signature self-verification failed".to_string()))?;

    println!("{}", hex::encode(signed_tx.serialize_bytes()));
    Ok(())
}

fn cmd_history(address: &str) -> Result<(), WalletError> {
    let addr = Address::from_hex(address)
        .map_err(|e| WalletError::Address(format!("invalid address '{address}': {e}")))?;
    // MVP: offline mode, no transaction log available.
    println!("No transaction history for {addr} (offline mode)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("atoll-wallet-test-{name}"))
    }

    #[test]
    fn address_derivation_is_deterministic() {
        let public_key = [7u8; 32];
        assert_eq!(derive_address(&public_key), derive_address(&public_key));
    }

    #[test]
    fn address_derivation_differs_for_different_keys() {
        assert_ne!(derive_address(&[1u8; 32]), derive_address(&[2u8; 32]));
    }

    #[test]
    fn derived_address_is_not_zero() {
        assert_ne!(derive_address(&[0u8; 32]), Address::zero());
    }

    #[test]
    fn transaction_sign_and_verify() {
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);

        let signing_key = SigningKey::from_bytes(&[3u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let tx = Transaction {
            version: 1,
            from,
            to: Some(to),
            value: 1_000,
            data: vec![1, 2, 3],
            signature: Signature::default(),
        };
        let message = tx.serialize_bytes();
        let signature = signing_key
            .try_sign(&message)
            .expect("signing a valid message must not fail");

        assert!(verifying_key.verify(&message, &signature).is_ok());

        let mut signed_tx = tx;
        signed_tx.signature.ed25519 = signature.to_bytes();
        assert_eq!(signed_tx.signature.ed25519, signature.to_bytes());
        assert!(signed_tx.signature.dilithium.is_empty());
    }

    #[test]
    fn signed_transaction_serialize_bytes_is_deterministic() {
        let from = Address::from_bytes([5u8; 20]);
        let to = Address::from_bytes([6u8; 20]);

        let signing_key = SigningKey::from_bytes(&[9u8; 32]);

        let build_signed = || {
            let tx = Transaction {
                version: 1,
                from,
                to: Some(to),
                value: 42,
                data: vec![1, 2, 3],
                signature: Signature::default(),
            };
            let message = tx.serialize_bytes();
            let signature = signing_key
                .try_sign(&message)
                .expect("signing a valid message must not fail");
            let mut signed_tx = tx;
            signed_tx.signature.ed25519 = signature.to_bytes();
            signed_tx
        };

        let bytes1 = build_signed().serialize_bytes();
        let bytes2 = build_signed().serialize_bytes();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn send_zero_amount_is_rejected() {
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        let keystore_path = temp_path("send_zero_amount");
        let _ = fs::remove_file(&keystore_path);

        let mut keystore = Keystore::load(keystore_path.clone(), "test-passphrase").unwrap();
        keystore.add(from, vec![3u8; 32]);
        keystore.save("test-passphrase").unwrap();

        let result = cmd_send(
            &keystore_path,
            &from.to_hex(),
            &to.to_hex(),
            "0",
            ATOLL,
            None,
        );
        assert!(
            matches!(result, Err(WalletError::Amount(_))),
            "zero amount must be rejected"
        );

        let _ = fs::remove_file(&keystore_path);
    }

    #[test]
    fn send_insufficient_balance_is_rejected() {
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        let keystore_path = temp_path("send_insufficient_balance");
        let _ = fs::remove_file(&keystore_path);

        let mut keystore = Keystore::load(keystore_path.clone(), "test-passphrase").unwrap();
        keystore.add(from, vec![3u8; 32]);
        keystore.save("test-passphrase").unwrap();

        // ChainState::new() has no accounts, so every address has balance 0.
        let result = cmd_send(
            &keystore_path,
            &from.to_hex(),
            &to.to_hex(),
            "100",
            ATOLL,
            None,
        );
        assert!(
            matches!(
                result,
                Err(WalletError::InsufficientBalance {
                    available: 0,
                    requested: 100
                })
            ),
            "insufficient balance must be rejected"
        );

        let _ = fs::remove_file(&keystore_path);
    }

    #[test]
    fn resolve_passphrase_uses_flag() {
        let passphrase = resolve_passphrase(Some("flag-pass")).unwrap();
        assert_eq!(passphrase, "flag-pass");
    }

    /// Guards `ATOLL_PASSPHRASE` manipulation so env-based resolution tests
    /// cannot interleave when `cargo test` runs them in parallel threads.
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn resolve_passphrase_uses_env_var() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::remove_var("ATOLL_PASSPHRASE");
        env::set_var("ATOLL_PASSPHRASE", "env-pass");
        let passphrase = resolve_passphrase(None).unwrap();
        assert_eq!(passphrase, "env-pass");
        env::remove_var("ATOLL_PASSPHRASE");
    }

    #[test]
    fn resolve_passphrase_flag_takes_precedence_over_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("ATOLL_PASSPHRASE", "env-pass");
        let passphrase = resolve_passphrase(Some("flag-pass")).unwrap();
        assert_eq!(passphrase, "flag-pass");
        env::remove_var("ATOLL_PASSPHRASE");
    }

    #[test]
    fn resolve_passphrase_empty_flag_is_rejected() {
        let result = resolve_passphrase(Some(""));
        assert!(matches!(result, Err(WalletError::Passphrase(_))));
    }
}
