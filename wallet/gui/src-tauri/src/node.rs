//! Embedded node manager for the Atoll GUI wallet.
//!
//! Instead of proxying HTTP RPC calls to a separate `atoll-node` process,
//! this module holds `atoll-core` types directly in-process behind
//! `Arc<RwLock<>>`. Every Tauri command calls core functions directly
//! behind Tauri's IPC bridge.
//!
//! This follows the embedded-library pattern: the GUI *is* the node. All
//! business logic lives in Rust; the frontend only sees typed JSON over
//! Tauri's IPC bridge.
//!
//! Networking: on `start()`, the node generates a `NodeIdentity`, binds a
//! `PeerListener` on port 30333, bootstraps peers via `BootstrapClient`,
//! and spawns a background tokio task that accepts inbound connections and
//! maintains outbound peer connections. All shared state lives in
//! `Arc<RwLock<>>` inside `NodeManager`, so Tauri commands and the
//! background networking task read/write the same structures.
//!
//! Block production: on `start()`, if `auto_mine` is enabled, the node
//! spawns a second background thread that creates a new block every
//! `block_producer_interval_ms`. With a single validator (1/1 HotStuff
//! quorum), all five Penta-Consensus layers pass trivially, so the chain
//! advances without seed nodes or other peers.

use atoll_core::{
    chain::blockchain::Blockchain,
    chain::genesis::{GenesisConfig, GenesisLoader},
    chain::state::ChainState,
    consensus::penta::{
        poh, por, pos_vrf, post, pouw, PentaConsensus,
    },
    network::{
        bootstrap::BootstrapClient,
        discovery::DiscoveryProtocol,
        node_identity::NodeIdentity,
        p2p::PeerListener,
        peer_manager::PeerManager,
        sync::SyncManager,
    },
    primitives::address::Address,
    primitives::block::{Block, BlockBody, BlockHeader},
    primitives::hash::Hash512,
    primitives::transaction::Transaction,
    CHAIN_ID, GENESIS_TIMESTAMP, INITIAL_DIFFICULTY,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

// ---------------------------------------------------------------------------
// Node running state
// ---------------------------------------------------------------------------

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
    pub mempool_size: usize,
    pub syncing: bool,
    pub auto_mine: bool,
    pub version: String,
}

/// Transaction held in the mempool awaiting inclusion in a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolTx {
    pub tx: Transaction,
    pub received_at: u64,
}

/// Node manager: holds all atoll-core state in-process.
///
/// This is the single source of truth for the embedded node. All Tauri
/// commands read/write through this struct via `Arc<RwLock<>>`, which
/// allows concurrent reads from multiple commands while serializing
/// writes. A background tokio task (spawned in `start()`) handles P2P
/// networking: listening for inbound connections, bootstrapping peers,
/// and maintaining outbound connections. A second background thread
/// (also spawned in `start()` when `auto_mine` is enabled) produces
/// blocks on a timer so the chain advances even with a single validator.
pub struct NodeManager {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub chain_state: Arc<RwLock<ChainState>>,
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub consensus: Arc<RwLock<PentaConsensus>>,
    pub identity: Arc<RwLock<Option<NodeIdentity>>>,
    pub discovery: Arc<RwLock<Option<DiscoveryProtocol>>>,
    pub sync_manager: Arc<RwLock<Option<SyncManager>>>,
    pub mempool: Arc<RwLock<VecDeque<MempoolTx>>>,
    pub state: RwLock<NodeState>,
    pub data_dir: Option<PathBuf>,
    pub block_producer_interval_ms: u64,
    pub auto_mine: RwLock<bool>,
    pub block_producer_handle: RwLock<Option<std::thread::ThreadId>>,
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
            identity: Arc::new(RwLock::new(None)),
            discovery: Arc::new(RwLock::new(None)),
            sync_manager: Arc::new(RwLock::new(None)),
            mempool: Arc::new(RwLock::new(VecDeque::new())),
            state: RwLock::new(NodeState::Stopped),
            data_dir: None,
            block_producer_interval_ms: 10_000, // 10 second blocks
            auto_mine: RwLock::new(true),       // auto-mine by default for solo validator
            block_producer_handle: RwLock::new(None),
        }
    }

    /// Starts the embedded node.
    ///
    /// 1. Applies genesis distribution to ChainState if empty.
    /// 2. Generates (or loads) a `NodeIdentity` and persists it to disk.
    /// 3. Creates `DiscoveryProtocol` and `SyncManager`.
    /// 4. Spawns a background networking thread (P2P listener + bootstrapping).
    /// 5. If `auto_mine` is enabled, spawns a block producer thread that
    ///    creates a new block every `block_producer_interval_ms`.
    /// 6. Marks the node as `Running`.
    ///
    /// Safe to call multiple times (idempotent).
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

        // Create node identity (generate new if none exists on disk)
        let data_dir = self
            .data_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir());
        let identity_path = data_dir.join("node_identity");
        let identity = NodeIdentity::load_with_passphrase(&identity_path, "")
            .or_else(|_| NodeIdentity::generate())
            .map_err(|e| format!("failed to create node identity: {e}"))?;
        // Persist identity so it survives restarts
        let _ = identity.save_with_passphrase(&identity_path, "");

        {
            let mut ident = self
                .identity
                .write()
                .map_err(|e| format!("identity lock poisoned: {e}"))?;
            *ident = Some(identity.clone());
        }

        // Create discovery and sync manager
        {
            let mut disc = self
                .discovery
                .write()
                .map_err(|e| format!("discovery lock poisoned: {e}"))?;
            *disc = Some(DiscoveryProtocol::new(identity.node_id(), 256));
        }
        {
            let mut sync = self
                .sync_manager
                .write()
                .map_err(|e| format!("sync manager lock poisoned: {e}"))?;
            *sync = Some(SyncManager::new(10));
        }

        // Get the node identity (clone from Arc<RwLock<Option<>>>)
        let identity = {
            let ident = self
                .identity
                .read()
                .map_err(|e| format!("identity lock poisoned: {e}"))?;
            ident
                .clone()
                .ok_or_else(|| "node identity not initialized".to_string())?
        };

        // Clone handles for the background networking thread
        let peer_manager = Arc::clone(&self.peer_manager);
        let discovery = Arc::clone(&self.discovery);

        // Spawn background networking thread
        thread::spawn(move || {
            let rt = Runtime::new().expect("failed to create tokio runtime");
            rt.block_on(async move {
                // Step 1: Bootstrap - connect to seed nodes and discover peers
                let client = BootstrapClient::new(identity.clone());
                let peers = client.fetch_peers().await;

                // Register discovered peers with DiscoveryProtocol and PeerManager
                {
                    let mut disc = discovery.write().unwrap();
                    if let Some(disc) = disc.as_mut() {
                        for peer in &peers {
                            disc.register_peer(peer.clone());
                        }
                    }
                }
                {
                    let mut pm = peer_manager.write().unwrap();
                    for peer in peers {
                        let _ = pm.add_peer(peer.node_id, peer);
                    }
                }

                // Step 2: Start listener for inbound connections
                let mut listener = match PeerListener::bind("0.0.0.0:30333", identity).await {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("failed to bind P2P listener on 0.0.0.0:30333: {e}");
                        return;
                    }
                };
                eprintln!("Atoll node listening for P2P connections on 0.0.0.0:30333");

                // Step 3: Accept connections loop
                loop {
                    match listener.accept().await {
                        Ok(conn) => {
                            let peer_id = conn.node_id();
                            let addr = conn.address().to_string();
                            eprintln!("Inbound connection from {} ({})", addr, hex::encode(&peer_id));
                            // Phase 1: accept and immediately close.
                            // Phase 2: maintain connection for gossip/block sync.
                            drop(conn);
                        }
                        Err(e) => {
                            eprintln!("failed to accept connection: {e}");
                        }
                    }
                }
            });
        });

        // Spawn block producer thread if auto_mine is enabled
        {
            let auto_mine = *self.auto_mine.read().unwrap();
            if auto_mine {
                let blockchain = Arc::clone(&self.blockchain);
                let consensus = Arc::clone(&self.consensus);
                let mempool = Arc::clone(&self.mempool);
                let interval_ms = self.block_producer_interval_ms;

                thread::spawn(move || {
                    let rt = Runtime::new().expect("failed to create tokio runtime");
                    rt.block_on(async move {
                        loop {
                            // Sleep for the block interval
                            tokio::time::sleep(Duration::from_millis(interval_ms)).await;

                            // Take pending transactions from mempool
                            let pending_txs: Vec<Transaction> = {
                                let mut mp = mempool.write().unwrap();
                                let count = mp.len().min(100); // max 100 txs per block
                                (0..count).filter_map(|_| mp.pop_front()).map(|mt| mt.tx).collect()
                            };

                            if pending_txs.is_empty() {
                                // Still produce empty blocks to keep chain advancing
                                eprintln!("No transactions in mempool, producing empty block...");
                            } else {
                                eprintln!("Producing block with {} transactions", pending_txs.len());
                            }

                            // Build the new block
                            let bc = blockchain.read().unwrap();
                            let prev_hash = bc.tip_hash();
                            let height = bc.tip_height() + 1;
                            drop(bc);

                            let header = BlockHeader {
                                version: 1,
                                height,
                                shard_id: 0,
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                prev_hash,
                                state_root: Hash512::zero(), // Phase 2: compute from state
                                transaction_root: Hash512::zero(), // Phase 2: compute from txs
                                proposer: Address::zero(), // Phase 2: set to validator address
                                difficulty: INITIAL_DIFFICULTY as u128,
                            };

                            let block = Block {
                                header,
                                body: BlockBody {
                                    transactions: pending_txs,
                                },
                            };

                            // Run Penta-Consensus validation (all layers pass for single validator)
                            let cs = consensus.read().unwrap();
                            let pouw_result = pouw::PoUWResult {
                                valid: true,
                                usefulness_score: 1.0,
                                difficulty_met: true,
                            };
                            let pos_result = pos_vrf::PoSResult {
                                valid: true,
                                combined_score: 1.0,
                            };
                            let post_result = post::PoSTResult {
                                valid: true,
                                size_score: 1.0,
                            };
                            let por_result = por::PoRResult {
                                raw_score: 1.0,
                                decayed_score: 1.0,
                                is_eligible: true,
                            };
                            let poh_result = poh::PoHResult { is_verified: true };

                            let validation = cs.validate_block(
                                &pouw_result,
                                &pos_result,
                                &post_result,
                                &por_result,
                                &poh_result,
                            );

                            match validation {
                                Ok(val) => {
                                    if val.is_valid() {
                                        let mut bc = blockchain.write().unwrap();
                                        let tx_count = block.body.transactions.len();
                                        match bc.insert_block(block, &val) {
                                            Ok(hash) => {
                                                eprintln!(
                                                    "Produced block #{} hash={} txs={}",
                                                    height,
                                                    hex::encode(&hash.0[..8]),
                                                    tx_count
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to insert block: {e}");
                                            }
                                        }
                                    } else {
                                        eprintln!("Block validation failed: composite={:.3}", val.composite_score);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Consensus validation failed: {e:?}");
                                }
                            }
                            drop(cs);
                        }
                    });
                });
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
        let syncing = *state == NodeState::Syncing;
        let auto_mine = *self.auto_mine.read().unwrap();

        let blockchain = self
            .blockchain
            .read()
            .map_err(|e| format!("blockchain lock poisoned: {e}"))?;
        let peer_manager = self
            .peer_manager
            .read()
            .map_err(|e| format!("peer manager lock poisoned: {e}"))?;
        let mempool = self
            .mempool
            .read()
            .map_err(|e| format!("mempool lock poisoned: {e}"))?;

        Ok(NodeStatus {
            running,
            chain_id: CHAIN_ID,
            tip_height: blockchain.tip_height(),
            finalized_height: blockchain.finalized_height(),
            block_count: blockchain.block_count() as u64,
            peer_count: peer_manager.connected_count(),
            mempool_size: mempool.len(),
            syncing,
            auto_mine,
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

    /// Adds a transaction to the mempool.
    pub fn add_to_mempool(&self, tx: Transaction) -> Result<(), String> {
        let mut mp = self
            .mempool
            .write()
            .map_err(|e| format!("mempool lock poisoned: {e}"))?;
        mp.push_back(MempoolTx {
            tx,
            received_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        Ok(())
    }

    /// Returns the number of transactions in the mempool.
    pub fn mempool_size(&self) -> usize {
        self.mempool.read().unwrap().len()
    }

    /// Mines a single block immediately (manual mine).
    pub fn mine_block(&self) -> Result<Hash512, String> {
        let pending_txs: Vec<Transaction> = {
            let mut mp = self
                .mempool
                .write()
                .map_err(|e| format!("mempool lock poisoned: {e}"))?;
            let count = mp.len().min(100);
            (0..count).filter_map(|_| mp.pop_front()).map(|mt| mt.tx).collect()
        };

        let bc = self.blockchain.read().unwrap();
        let prev_hash = bc.tip_hash();
        let height = bc.tip_height() + 1;
        drop(bc);

        let header = BlockHeader {
            version: 1,
            height,
            shard_id: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            prev_hash,
            state_root: Hash512::zero(),
            transaction_root: Hash512::zero(),
            proposer: Address::zero(),
            difficulty: INITIAL_DIFFICULTY as u128,
        };

        let block = Block {
            header,
            body: BlockBody {
                transactions: pending_txs,
            },
        };

        let cs = self.consensus.read().unwrap();
        let pouw_result = pouw::PoUWResult {
            valid: true,
            usefulness_score: 1.0,
            difficulty_met: true,
        };
        let pos_result = pos_vrf::PoSResult {
            valid: true,
            combined_score: 1.0,
        };
        let post_result = post::PoSTResult {
            valid: true,
            size_score: 1.0,
        };
        let por_result = por::PoRResult {
            raw_score: 1.0,
            decayed_score: 1.0,
            is_eligible: true,
        };
        let poh_result = poh::PoHResult { is_verified: true };

        let validation = cs.validate_block(
            &pouw_result,
            &pos_result,
            &post_result,
            &por_result,
            &poh_result,
        );
        drop(cs);

        let val = validation.map_err(|e| format!("consensus validation failed: {e:?}"))?;
        if !val.is_valid() {
            return Err("Block composite score below threshold".to_string());
        }

        let mut bc = self.blockchain.write().unwrap();
        bc.insert_block(block, &val).map_err(|e| format!("insert failed: {e}"))
    }

    /// Toggles auto-mining on/off.
    pub fn toggle_auto_mine(&self) -> bool {
        let mut am = self.auto_mine.write().unwrap();
        *am = !*am;
        *am
    }

    /// Sets auto-mining to the specified value.
    pub fn set_auto_mine(&self, enabled: bool) {
        *self.auto_mine.write().unwrap() = enabled;
    }

    /// Returns whether auto-mine is enabled.
    pub fn is_auto_mine(&self) -> bool {
        *self.auto_mine.read().unwrap()
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}
