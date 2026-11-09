//! Embedded node manager for the Atoll GUI wallet.
//!
//! Instead of proxying HTTP RPC calls to a separate `atoll-node` process,
//! this module holds `atoll-core` types directly in-process behind
//! `Arc<RwLock<>>`. Every Tauri command calls core functions directly —
//! no HTTP, no sidecar, no ports, no port-collision handling.
//!
//! This follows the embedded-library pattern (Shekyl, DOM Wallet V3):
//! the GUI *is* the node. All business logic lives in Rust; the frontend
//! only sees typed JSON over Tauri's IPC bridge.

use atoll_core::{
    chain::blockchain::Blockchain,
    chain::genesis::{GenesisConfig, GenesisLoader},
    chain::state::ChainState,
    consensus::penta::PentaConsensus,
    network::peer_manager::PeerManager,
    primitives::address::Address,
    CHAIN_ID, GENESIS_TIMESTAMP, INITIAL_DIFFICULTY,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

/// Node running state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    Stopped,
    Starting,
    Running,
    Syncing,
}

/// Node status returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub running: bool,
    pub chain_id: u64,
    pub tip_height: u64,
    pub finalized_height: u64,
    pub block_count: u64,
    pub peer_count: usize,
    pub syncing: bool,
    pub version: String,
}

/// Node manager: holds all atoll-core state in-process.
///
/// This is the single source of truth for the embedded node. All Tauri
/// commands read/write through this struct via `Arc<RwLock<>>`, which
/// allows concurrent reads from multiple commands while serializing
/// writes.
pub struct NodeManager {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub chain_state: Arc<RwLock<ChainState>>,
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub consensus: Arc<RwLock<PentaConsensus>>,
    pub state: RwLock<NodeState>,
    pub data_dir: Option<PathBuf>,
}

impl NodeManager {
    /// Creates a new NodeManager with default (genesis) state.
    pub fn new() -> Self {
        let config = GenesisConfig {
            chain_id: CHAIN_ID,
            genesis_timestamp: GENESIS_TIMESTAMP,
            initial_difficulty: INITIAL_DIFFICULTY as u64,
            initial_distribution: HashMap::new(),
        };

        let blockchain = Arc::new(RwLock::new(Blockchain::new(&config)));
        let chain_state = Arc::new(RwLock::new(ChainState::new()));
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(256)));
        let consensus = Arc::new(RwLock::new(PentaConsensus::new()));

        Self {
            blockchain,
            chain_state,
            peer_manager,
            consensus,
            state: RwLock::new(NodeState::Stopped),
            data_dir: None,
        }
    }

    /// Starts the embedded node. Initialises genesis state and marks the
    /// node as running. Safe to call multiple times (idempotent).
    pub fn start(&self) -> Result<(), String> {
        {
            let mut state = self
                .state
                .write()
                .map_err(|e| format!("node state lock poisoned: {e}"))?;
            if *state == NodeState::Running {
                return Ok(());
            }
            *state = NodeState::Starting;
        }

        // Apply genesis distribution to ChainState if empty
        {
            let mut state = self
                .chain_state
                .write()
                .map_err(|e| format!("chain state lock poisoned: {e}"))?;
            if state.total_supply() == 0 {
                let config = GenesisConfig {
                    chain_id: CHAIN_ID,
                    genesis_timestamp: GENESIS_TIMESTAMP,
                    initial_difficulty: INITIAL_DIFFICULTY as u64,
                    initial_distribution: HashMap::new(),
                };
                GenesisLoader::apply_distribution(&config, &mut state)
                    .map_err(|e| format!("genesis distribution failed: {e}"))?;
            }
        }

        {
            let mut state = self
                .state
                .write()
                .map_err(|e| format!("node state lock poisoned: {e}"))?;
            *state = NodeState::Running;
        }

        Ok(())
    }

    /// Stops the embedded node.
    pub fn stop(&self) -> Result<(), String> {
        let mut state = self
            .state
            .write()
            .map_err(|e| format!("node state lock poisoned: {e}"))?;
        *state = NodeState::Stopped;
        Ok(())
    }

    /// Returns the current node status.
    pub fn status(&self) -> Result<NodeStatus, String> {
        let state = self
            .state
            .read()
            .map_err(|e| format!("node state lock poisoned: {e}"))?;
        let running = *state == NodeState::Running;

        let blockchain = self
            .blockchain
            .read()
            .map_err(|e| format!("blockchain lock poisoned: {e}"))?;
        let _chain_state = self
            .chain_state
            .read()
            .map_err(|e| format!("chain state lock poisoned: {e}"))?;
        let peer_manager = self
            .peer_manager
            .read()
            .map_err(|e| format!("peer manager lock poisoned: {e}"))?;

        Ok(NodeStatus {
            running,
            chain_id: CHAIN_ID,
            tip_height: blockchain.tip_height(),
            finalized_height: blockchain.finalized_height(),
            block_count: blockchain.block_count() as u64,
            peer_count: peer_manager.connected_count(),
            syncing: *state == NodeState::Syncing,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Returns the ATOLL balance of an address from the embedded ChainState.
    pub fn get_balance(&self, address: &Address) -> u128 {
        let state = self.chain_state.read().unwrap();
        state.balance(address)
    }

    /// Returns the total ATOLL supply from the embedded ChainState.
    pub fn total_supply(&self) -> u128 {
        let state = self.chain_state.read().unwrap();
        state.total_supply()
    }

    /// Returns the MIND supply from the embedded ChainState.
    pub fn mind_supply(&self) -> u128 {
        let state = self.chain_state.read().unwrap();
        state.mind_supply()
    }

    /// Returns the PROOF balance of an address.
    pub fn proof_balance(&self, address: &Address) -> u128 {
        let state = self.chain_state.read().unwrap();
        state.proof_balance(address)
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}
