//! Serializable error type for the Tauri IPC boundary.
//!
//! `WalletError` (from `wallet-cli`) is not `Serialize`, so it cannot cross
//! the Tauri `invoke` boundary directly. `GuiError` mirrors each `WalletError`
//! variant with plain data (strings and numbers) that the frontend can render.

use serde::Serialize;
use wallet_cli::WalletError;

/// Serializable mirror of `WalletError` for frontend consumption.
#[derive(Debug, Clone, Serialize)]
pub enum GuiError {
    Io(String),
    Json(String),
    Hex(String),
    Crypto(String),
    Passphrase(String),
    PlaintextKeystore(String),
    Address(String),
    Amount(String),
    KeyNotFound(String),
    UnsupportedToken(String),
    InsufficientBalance { available: u128, requested: u128 },
}

impl From<WalletError> for GuiError {
    fn from(error: WalletError) -> Self {
        match error {
            WalletError::Io(e) => GuiError::Io(e.to_string()),
            WalletError::Json(e) => GuiError::Json(e.to_string()),
            WalletError::Hex(e) => GuiError::Hex(e.to_string()),
            WalletError::Crypto(message) => GuiError::Crypto(message),
            WalletError::Passphrase(message) => GuiError::Passphrase(message),
            WalletError::PlaintextKeystore(message) => GuiError::PlaintextKeystore(message),
            WalletError::Address(message) => GuiError::Address(message),
            WalletError::Amount(message) => GuiError::Amount(message),
            WalletError::KeyNotFound(address) => GuiError::KeyNotFound(address.to_string()),
            WalletError::UnsupportedToken(token) => GuiError::UnsupportedToken(token),
            WalletError::InsufficientBalance {
                available,
                requested,
            } => GuiError::InsufficientBalance {
                available,
                requested,
            },
        }
    }
}

impl From<std::io::Error> for GuiError {
    fn from(e: std::io::Error) -> Self {
        GuiError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for GuiError {
    fn from(e: serde_json::Error) -> Self {
        GuiError::Json(e.to_string())
    }
}

impl From<hex::FromHexError> for GuiError {
    fn from(e: hex::FromHexError) -> Self {
        GuiError::Hex(e.to_string())
    }
}

impl std::fmt::Display for GuiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuiError::Io(message) => write!(f, "I/O error: {message}"),
            GuiError::Json(message) => write!(f, "serialization error: {message}"),
            GuiError::Hex(message) => write!(f, "invalid hex: {message}"),
            GuiError::Crypto(message) => write!(f, "crypto error: {message}"),
            GuiError::Passphrase(message) => write!(f, "passphrase error: {message}"),
            GuiError::PlaintextKeystore(message) => {
                write!(f, "unencrypted plaintext keystore: {message}")
            }
            GuiError::Address(message) => write!(f, "invalid address: {message}"),
            GuiError::Amount(message) => write!(f, "invalid amount: {message}"),
            GuiError::KeyNotFound(address) => {
                write!(f, "no key found for address {address} in keystore")
            }
            GuiError::UnsupportedToken(token) => write!(
                f,
                "unsupported token '{token}'; only 'ATOLL' is supported in MVP"
            ),
            GuiError::InsufficientBalance {
                available,
                requested,
            } => write!(
                f,
                "insufficient balance: have {available} ATOLL, requested {requested} ATOLL"
            ),
        }
    }
}

impl std::error::Error for GuiError {}
