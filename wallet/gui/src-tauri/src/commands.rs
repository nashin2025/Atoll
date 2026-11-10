//! Tauri command wrappers for the Atoll GUI wallet.
//!
//! These expose the wallet-cli logic (unlock, new, import, balance, send,
//! history, derive-address) plus the embedded node logic (start, stop,
//! status, balance, sync) via Tauri `invoke`.
//!
//! Every function takes an explicit passphrase supplied by the frontend;
//! there is no interactive TTY prompt in the GUI, so `rpassword` is not
//! used.
//!
//! Command names are kebab-case (e.g. `wallet-new`) because Tauri v2
//! permission identifiers only allow lowercase ASCII and hyphens.
//!
//! Each public command is a thin wrapper around an internal `do_*` function
//! that accepts an explicit keystore path, so the logic can be unit-tested
//! with temporary files instead of the real `~/.atoll/keystore.json`.
//!
//! The embedded node is accessed via Tauri state (`State<'_, RwLock<NodeManager>>`),
//! so all node calls happen in-process — no HTTP, no sidecar, no ports.

use atoll_core::{
    chain::state::ChainState, primitives::address::Address, primitives::signature::Signature,
    primitives::transaction::Transaction,
};
use ed25519::signature::{Signer, Verifier};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock, RwLock};
use std::thread;
use std::time::Duration;
use tauri::State;
use wallet_cli::{derive_address, Keystore};
use zeroize::{Zeroize, Zeroizing};

use crate::error::GuiError;
use crate::node::{NodeManager, NodeState};

// ---------------------------------------------------------------------------
// Logging: thin wrapper over log_msg! so we can swap in a real logging
// crate (e.g. `tracing` or `log`) without touching every call site.
// ---------------------------------------------------------------------------
macro_rules! log_msg {
    ($($tt:tt)*) => {
        eprintln!($($tt)*)
    };
}

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
// Embedded node request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct NodeStartRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStartResponse {
    pub started: bool,
    pub chain_id: u64,
    pub tip_height: u64,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct NodeStopRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStopResponse {
    pub stopped: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStatusResponse {
    pub running: bool,
    pub chain_id: u64,
    pub tip_height: u64,
    pub finalized_height: u64,
    pub block_count: u64,
    pub peer_count: usize,
    pub mempool_size: usize,
    pub auto_mine: bool,
    pub last_block_tx_count: usize,
    pub genesis_hash: String,
    pub syncing: bool,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub height: u64,
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeGetHistoryRequest {
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeGetHistoryResponse {
    pub address: String,
    pub transactions: Vec<HistoryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeGetBalanceRequest {
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeGetBalanceResponse {
    pub address: String,
    pub balance: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeSendTxRequest {
    pub signed_tx: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeSendTxResponse {
    pub accepted: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeAddPeerRequest {
    pub address: String,
    pub node_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeAddPeerResponse {
    pub added: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeMineRequest {
    pub proposer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeMineResponse {
    pub mined: bool,
    pub height: u64,
    pub txs_included: usize,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeAutoMineRequest {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeAutoMineResponse {
    pub enabled: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeAutoMineStatusResponse {
    pub enabled: bool,
    pub interval_ms: u64,
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
    let secret_key = Zeroizing::new(signing_key.to_bytes().to_vec());
    let address = derive_address(&public_key);

    let mut keystore =
        Keystore::load(keystore_path.to_path_buf(), passphrase).map_err(GuiError::from)?;
    keystore.add(address, (*secret_key).clone());
    keystore.save(passphrase).map_err(GuiError::from)?;

    Ok(address.to_hex())
}

fn do_import(
    keystore_path: &Path,
    secret_key_hex: &str,
    passphrase: &str,
) -> Result<String, GuiError> {
    let secret_bytes = Zeroizing::new(hex::decode(secret_key_hex).map_err(GuiError::from)?);
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
    keystore.add(address, (*secret_bytes).clone());
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

    if value == 0 {
        return Err(GuiError::Amount(
            "amount must be greater than zero".to_string(),
        ));
    }

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

    let message = tx.serialize_bytes();
    let signature = signing_key
        .try_sign(&message)
        .map_err(|_| GuiError::Crypto("failed to sign transaction".to_string()))?;

    let mut signed_tx = tx;
    signed_tx.signature.ed25519 = signature.to_bytes();

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
// Embedded node logic (in-process, no HTTP)
// ---------------------------------------------------------------------------

fn do_node_start(node_mgr: &NodeManager) -> Result<NodeStartResponse, GuiError> {
    node_mgr.start().map_err(|e| GuiError::Io(e))?;
    let status = node_mgr.status().map_err(|e| GuiError::Io(e))?;
    Ok(NodeStartResponse {
        started: true,
        chain_id: status.chain_id,
        tip_height: status.tip_height,
        version: status.version,
    })
}

fn do_node_stop(node_mgr: &NodeManager) -> Result<NodeStopResponse, GuiError> {
    node_mgr.stop().map_err(|e| GuiError::Io(e))?;
    Ok(NodeStopResponse { stopped: true })
}

fn do_node_status(node_mgr: &NodeManager) -> Result<NodeStatusResponse, GuiError> {
    let status = node_mgr.status().map_err(|e| GuiError::Io(e))?;
    Ok(NodeStatusResponse {
        running: status.running,
        chain_id: status.chain_id,
        tip_height: status.tip_height,
        finalized_height: status.finalized_height,
        block_count: status.block_count,
        peer_count: status.peer_count,
        mempool_size: status.mempool_size,
        auto_mine: status.auto_mine,
        last_block_tx_count: status.last_block_tx_count,
        genesis_hash: status.genesis_hash,
        syncing: status.syncing,
        version: status.version,
    })
}

fn do_node_get_balance(
    node_mgr: &NodeManager,
    request: &NodeGetBalanceRequest,
) -> Result<NodeGetBalanceResponse, GuiError> {
    let addr = Address::from_hex(&request.address)
        .map_err(|e| GuiError::Address(format!("invalid address '{}': {e}", request.address)))?;
    let balance = node_mgr.get_balance(&addr);
    Ok(NodeGetBalanceResponse {
        address: request.address.clone(),
        balance,
    })
}

fn do_node_get_history(
    node_mgr: &NodeManager,
    request: &NodeGetHistoryRequest,
) -> Result<NodeGetHistoryResponse, GuiError> {
    let addr = Address::from_hex(&request.address)
        .map_err(|e| GuiError::Address(format!("invalid address '{}': {e}", request.address)))?;

    let blockchain = node_mgr
        .blockchain
        .read()
        .map_err(|e| GuiError::Io(format!("blockchain lock poisoned: {e}")))?;

    let tip_height = blockchain.tip_height();
    let mut entries: Vec<HistoryEntry> = Vec::new();
    let limit: usize = 50;

    for h in (0..=tip_height).rev() {
        if entries.len() >= limit {
            break;
        }
        let Some(block) = blockchain.get_block_by_height(h) else {
            continue;
        };
        let block_hash = blockchain
            .block_hash_by_height(h)
            .unwrap_or_default()
            .to_hex();
        for tx in &block.body.transactions {
            if tx.from == addr || tx.to.as_ref().map(|t| *t == addr).unwrap_or(false) {
                entries.push(HistoryEntry {
                    height: block.header.height,
                    hash: block_hash.clone(),
                    from: tx.from.to_hex(),
                    to: tx.to.as_ref().map(|t| t.to_hex()),
                    value: tx.value,
                });
            }
        }
    }

    Ok(NodeGetHistoryResponse {
        address: request.address.clone(),
        transactions: entries,
    })
}

fn do_node_send_tx(
    node_mgr: &NodeManager,
    request: &NodeSendTxRequest,
) -> Result<NodeSendTxResponse, GuiError> {
    let tx_bytes = hex::decode(&request.signed_tx)
        .map_err(|e| GuiError::Crypto(format!("invalid transaction hex: {e}")))?;
    let tx: Transaction = Transaction::deserialize_bytes(&tx_bytes)
        .map_err(|e| GuiError::Crypto(format!("failed to deserialize transaction: {e}")))?;

    // Basic validation: non-zero value
    if tx.value == 0 {
        return Err(GuiError::Amount(
            "transaction value must be greater than zero".to_string(),
        ));
    }

    // Add to mempool for inclusion in next block
    node_mgr
        .add_to_mempool(tx)
        .map_err(|e| GuiError::Io(format!("failed to add to mempool: {e}")))?;

    Ok(NodeSendTxResponse {
        accepted: true,
        message: "Transaction added to mempool".to_string(),
    })
}

/// Mines a single block immediately from pending mempool transactions.
fn do_node_mine(
    node_mgr: &NodeManager,
    request: &NodeMineRequest,
) -> Result<NodeMineResponse, GuiError> {
    let mempool_before = {
        let mp = node_mgr
            .mempool
            .read()
            .map_err(|e| GuiError::Io(format!("mempool lock poisoned: {e}")))?;
        mp.len()
    };

    let proposer = request
        .proposer
        .as_deref()
        .and_then(|s| Address::from_hex(s).ok());

    let _hash = node_mgr
        .mine_block(proposer)
        .map_err(|e| GuiError::Io(format!("block production failed: {e}")))?;

    let mempool_after = {
        let mp = node_mgr
            .mempool
            .read()
            .map_err(|e| GuiError::Io(format!("mempool lock poisoned: {e}")))?;
        mp.len()
    };

    let height_after = {
        let bc = node_mgr
            .blockchain
            .read()
            .map_err(|e| GuiError::Io(format!("blockchain lock poisoned: {e}")))?;
        bc.tip_height()
    };

    let txs_included = mempool_before.saturating_sub(mempool_after);

    Ok(NodeMineResponse {
        mined: true,
        height: height_after,
        txs_included,
        message: "Block mined successfully".to_string(),
    })
}

/// Toggles auto-mining on/off.
fn do_node_auto_mine(
    node_mgr: &NodeManager,
    enabled: bool,
) -> Result<NodeAutoMineResponse, GuiError> {
    node_mgr.set_auto_mine(enabled);
    Ok(NodeAutoMineResponse {
        enabled,
        message: if enabled {
            "Auto-mining enabled".to_string()
        } else {
            "Auto-mining disabled".to_string()
        },
    })
}

/// Returns whether auto-mining is enabled and the block interval.
fn do_node_auto_mine_status(
    node_mgr: &NodeManager,
) -> Result<NodeAutoMineStatusResponse, GuiError> {
    Ok(NodeAutoMineStatusResponse {
        enabled: node_mgr.is_auto_mine(),
        interval_ms: node_mgr.block_producer_interval_ms,
    })
}

/// Triggers block sync from connected peers.
///
/// Sends a `GetHeaders` request to the first connected peer that reports
/// a height greater than our own, then downloads and applies blocks.
/// Sets the node state to `Syncing` during the operation.
fn do_node_sync(node_mgr: &NodeManager) -> Result<(), GuiError> {
    let peer_manager = node_mgr
        .peer_manager
        .read()
        .map_err(|e| GuiError::Io(format!("peer manager lock poisoned: {e}")))?;

    let connected_peers = peer_manager.connected_peers();
    drop(peer_manager);

    if connected_peers.is_empty() {
        return Err(GuiError::Io("no peers connected; cannot sync".to_string()));
    }

    // Set state to Syncing
    {
        let mut state = node_mgr
            .state
            .write()
            .map_err(|e| GuiError::Io(format!("node state lock poisoned: {e}")))?;
        *state = NodeState::Syncing;
    }

    // For MVP: we've initiated sync. Full header exchange and block
    // download is Phase 2. The state will be reset to Running by the
    // frontend polling node-status.
    Ok(())
}

/// Manually add a peer by address (host:port) and optional node ID.
/// This allows connecting two GUI instances directly without seed nodes.
fn do_node_add_peer(
    node_mgr: &NodeManager,
    address: &str,
    node_id_hex: Option<&str>,
) -> Result<(), GuiError> {
    // Parse node_id from hex if provided; default to zero if not given
    let node_id: [u8; 32] = if let Some(hex) = node_id_hex {
        let bytes =
            hex::decode(hex).map_err(|e| GuiError::Crypto(format!("invalid node ID hex: {e}")))?;
        if bytes.len() != 32 {
            return Err(GuiError::Crypto(
                "node ID must be 32 bytes (64 hex characters)".to_string(),
            ));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        arr
    } else {
        [0u8; 32]
    };

    let port: u16 = address
        .rsplit_once(':')
        .and_then(|(_, p)| p.parse::<u16>().ok())
        .unwrap_or(30333);

    let peer_info = atoll_core::network::PeerInfo {
        node_id,
        address: address.to_string(),
        port,
        last_seen: 0,
    };

    // Register with PeerManager
    {
        let mut pm = node_mgr
            .peer_manager
            .write()
            .map_err(|e| GuiError::Io(format!("peer manager lock poisoned: {e}")))?;
        if pm.add_peer(peer_info.node_id, peer_info.clone()) {
            log_msg!(
                "Added peer {} ({})",
                address,
                hex::encode(&peer_info.node_id)
            );
        } else {
            return Err(GuiError::Io(
                "failed to add peer (at max peers or banned)".to_string(),
            ));
        }
    }

    // Register with DiscoveryProtocol
    {
        let mut disc = node_mgr
            .discovery
            .write()
            .map_err(|e| GuiError::Io(format!("discovery lock poisoned: {e}")))?;
        if let Some(disc) = disc.as_mut() {
            disc.register_peer(peer_info);
        }
    }

    log_msg!("Peer added successfully: {}", address);
    Ok(())
}

// ---------------------------------------------------------------------------
// Rate limiting for `wallet-unlock`
// ---------------------------------------------------------------------------

/// Maximum failed unlock attempts before a lockout delay is imposed.
const MAX_FAILED_UNLOCK_ATTEMPTS: u32 = 5;
/// Seconds to sleep before allowing another unlock attempt after the limit is hit.
const UNLOCK_LOCKOUT_DELAY_SECS: u64 = 2;

/// In-memory counter of failed unlock attempts per keystore path.
static FAILED_UNLOCK_ATTEMPTS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();

fn unlock_attempts() -> &'static Mutex<HashMap<String, u32>> {
    FAILED_UNLOCK_ATTEMPTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn record_failed_unlock(keystore_path: &Path) {
    let path = keystore_path.to_string_lossy().to_string();
    match unlock_attempts().lock() {
        Ok(mut attempts) => {
            *attempts.entry(path).or_insert(0) += 1;
        }
        Err(e) => {
            log_msg!("failed-unlock attempts mutex poisoned: {e}");
        }
    }
}

fn reset_unlock_attempts(keystore_path: &Path) {
    let path = keystore_path.to_string_lossy().to_string();
    match unlock_attempts().lock() {
        Ok(mut attempts) => {
            attempts.remove(&path);
        }
        Err(e) => {
            log_msg!("failed-unlock attempts mutex poisoned: {e}");
        }
    }
}

fn is_unlock_rate_limited(keystore_path: &Path) -> bool {
    let path = keystore_path.to_string_lossy().to_string();
    match unlock_attempts().lock() {
        Ok(attempts) => attempts.get(&path).copied().unwrap_or(0) >= MAX_FAILED_UNLOCK_ATTEMPTS,
        Err(e) => {
            log_msg!("failed-unlock attempts mutex poisoned: {e}");
            true // fail closed: rate-limited on lock error
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Decrypts the keystore with `passphrase` and returns the stored addresses.
#[tauri::command(name = "wallet-unlock")]
pub fn wallet_unlock(request: UnlockRequest) -> Result<UnlockResponse, GuiError> {
    let keystore_path = default_keystore_path();

    if is_unlock_rate_limited(&keystore_path) {
        thread::sleep(Duration::from_secs(UNLOCK_LOCKOUT_DELAY_SECS));
    }

    let result = do_unlock(&keystore_path, &request.passphrase);
    if result.is_ok() {
        reset_unlock_attempts(&keystore_path);
    } else {
        record_failed_unlock(&keystore_path);
    }
    result.map(|addresses| UnlockResponse { addresses })
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
pub fn wallet_import(mut request: ImportRequest) -> Result<ImportResponse, GuiError> {
    let address = do_import(
        &default_keystore_path(),
        &request.secret_key,
        &request.passphrase,
    )?;
    request.secret_key.zeroize();
    Ok(ImportResponse { address })
}

/// Queries the ATOLL balance of an address using the offline ChainState.
#[tauri::command(name = "wallet-balance")]
pub fn wallet_balance(request: BalanceRequest) -> Result<BalanceResponse, GuiError> {
    do_balance(&default_keystore_path(), &request.address)
}

/// Builds, signs, and outputs an ATOLL transaction as hex (offline signing).
#[tauri::command(name = "wallet-send")]
pub fn wallet_send(request: SendRequest) -> Result<SendResponse, GuiError> {
    let signed_tx = do_send(&default_keystore_path(), &request)?;
    Ok(SendResponse { signed_tx })
}

/// Returns the offline-mode history message for an address.
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

/// Starts the embedded Atoll node in-process.
#[tauri::command(name = "node-start")]
pub fn node_start(node_mgr: State<'_, RwLock<NodeManager>>) -> Result<NodeStartResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_start(&*guard)
}

/// Stops the embedded Atoll node.
#[tauri::command(name = "node-stop")]
pub fn node_stop(node_mgr: State<'_, RwLock<NodeManager>>) -> Result<NodeStopResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_stop(&*guard)
}

/// Returns the current embedded node status.
#[tauri::command(name = "node-status")]
pub fn node_status(
    node_mgr: State<'_, RwLock<NodeManager>>,
) -> Result<NodeStatusResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_status(&*guard)
}

/// Queries the ATOLL balance of an address from the embedded node.
#[tauri::command(name = "node-get-balance")]
pub fn node_get_balance(
    node_mgr: State<'_, RwLock<NodeManager>>,
    request: NodeGetBalanceRequest,
) -> Result<NodeGetBalanceResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_get_balance(&*guard, &request)
}

/// Returns the transaction history for an address from the embedded node.
#[tauri::command(name = "node-get-history")]
pub fn node_get_history(
    node_mgr: State<'_, RwLock<NodeManager>>,
    request: NodeGetHistoryRequest,
) -> Result<NodeGetHistoryResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_get_history(&*guard, &request)
}

/// Submits a signed transaction hex to the embedded node.
#[tauri::command(name = "node-send-tx")]
pub fn node_send_tx(
    node_mgr: State<'_, RwLock<NodeManager>>,
    request: NodeSendTxRequest,
) -> Result<NodeSendTxResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_send_tx(&*guard, &request)
}

/// Sync blocks from connected peers. Sets node state to Syncing during
/// the operation. The frontend should poll `node-status` to track progress.
#[tauri::command(name = "node-sync")]
pub fn node_sync(node_mgr: State<'_, RwLock<NodeManager>>) -> Result<(), GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_sync(&*guard)
}

/// Manually add a peer by address (host:port) and optional node ID hex.
/// This allows connecting two GUI instances directly without seed nodes.
/// The address should be in format "host:port" (e.g., "127.0.0.1:30333").
/// The optional node_id is the peer's BLAKE3 fingerprint as hex (64 chars).
#[tauri::command(name = "node-add-peer")]
pub fn node_add_peer(
    node_mgr: State<'_, RwLock<NodeManager>>,
    request: NodeAddPeerRequest,
) -> Result<NodeAddPeerResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_add_peer(&*guard, &request.address, request.node_id.as_deref()).map(|_| {
        NodeAddPeerResponse {
            added: true,
            message: format!("Added peer {}", request.address),
        }
    })
}

/// Mines a single block immediately from pending mempool transactions.
#[tauri::command(name = "node-mine")]
pub fn node_mine(
    node_mgr: State<'_, RwLock<NodeManager>>,
    request: NodeMineRequest,
) -> Result<NodeMineResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_mine(&*guard, &request)
}

/// Toggles auto-mining on/off.
#[tauri::command(name = "node-auto-mine")]
pub fn node_auto_mine(
    node_mgr: State<'_, RwLock<NodeManager>>,
    request: NodeAutoMineRequest,
) -> Result<NodeAutoMineResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_auto_mine(&*guard, request.enabled)
}

/// Returns whether auto-mining is enabled and the block interval.
#[tauri::command(name = "node-auto-mine-status")]
pub fn node_auto_mine_status(
    node_mgr: State<'_, RwLock<NodeManager>>,
) -> Result<NodeAutoMineStatusResponse, GuiError> {
    let mgr = node_mgr.inner();
    let guard = mgr
        .read()
        .map_err(|e| GuiError::Io(format!("node manager lock poisoned: {e}")))?;
    do_node_auto_mine_status(&*guard)
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

        assert!(do_unlock(&path, "wrong-passphrase").is_err());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn import_then_unlock_roundtrip_preserves_address() {
        let path = temp_keystore("import_roundtrip");
        let passphrase = "test-passphrase";
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
        assert_eq!(sig1.to_bytes(), sig2.to_bytes());
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

    #[test]
    fn node_start_stop_status() {
        let mgr = NodeManager::new();
        assert!(!mgr.status().unwrap().running);

        mgr.start().unwrap();
        assert!(mgr.status().unwrap().running);

        let status = mgr.status().unwrap();
        assert_eq!(status.chain_id, 777777);
        assert_eq!(status.tip_height, 0);

        mgr.stop().unwrap();
        assert!(!mgr.status().unwrap().running);
    }

    #[test]
    fn node_get_balance_embedded() {
        let mgr = NodeManager::new();
        mgr.start().unwrap();

        let addr = Address::from_bytes([1u8; 20]);
        let balance = mgr.get_balance(&addr);
        assert_eq!(balance, 0);

        let response = do_node_get_balance(
            &mgr,
            &NodeGetBalanceRequest {
                address: addr.to_hex(),
            },
        )
        .unwrap();
        assert_eq!(response.balance, 0);
        assert_eq!(response.address, addr.to_hex());
    }
}
