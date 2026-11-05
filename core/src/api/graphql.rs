//! GraphQL API for the Atoll Protocol Block Explorer.
//!
//! Provides read-only GraphQL access to chain state: blocks, transactions,
//! addresses, validators, and network stats. All resolvers read through
//! `Arc<RwLock>` handles in [`AppState`] and return proper GraphQL errors
//! instead of panicking.
//!
//! Large integers (u64, u128) are serialized as strings to avoid JSON
//! precision loss. Input parameters accept hex/decimal strings and are
//! parsed manually in resolvers.

use crate::api::rest::AppState;
use crate::primitives::address::Address as CoreAddress;
use crate::primitives::block::Block as CoreBlock;
use crate::primitives::hash::Hash512;
use crate::primitives::transaction::Transaction as CoreTransaction;
use async_graphql::{Context, EmptyMutation, EmptySubscription, Error, Object, Result, Schema};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// GraphQL object types
// ---------------------------------------------------------------------------

/// GraphQL representation of a blockchain address (0x-prefixed hex).
#[derive(Clone)]
pub struct GqlAddress {
    pub address: String,
}

#[Object]
impl GqlAddress {
    async fn address(&self) -> &str {
        &self.address
    }
}

/// GraphQL representation of a block.
#[derive(Clone)]
pub struct GqlBlock {
    pub hash: String,
    pub height: String,
    pub timestamp: String,
    pub shard_id: String,
    pub version: String,
    pub difficulty: String,
    pub proposer: GqlAddress,
    pub state_root: String,
    pub transaction_root: String,
    pub transaction_count: i32,
}

#[Object]
impl GqlBlock {
    async fn hash(&self) -> &str {
        &self.hash
    }

    async fn height(&self) -> &str {
        &self.height
    }

    async fn timestamp(&self) -> &str {
        &self.timestamp
    }

    async fn shard_id(&self) -> &str {
        &self.shard_id
    }

    async fn version(&self) -> &str {
        &self.version
    }

    async fn difficulty(&self) -> &str {
        &self.difficulty
    }

    async fn proposer(&self) -> &GqlAddress {
        &self.proposer
    }

    async fn state_root(&self) -> &str {
        &self.state_root
    }

    async fn transaction_root(&self) -> &str {
        &self.transaction_root
    }

    async fn transaction_count(&self) -> i32 {
        self.transaction_count
    }

    /// Transactions included in this block.
    async fn transactions(&self, ctx: &Context<'_>) -> Result<Vec<GqlTransaction>> {
        let app_state = ctx.data::<Arc<AppState>>()?;
        let blockchain = app_state
            .blockchain
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;

        let block_hash = Hash512::from_hex(&self.hash)
            .map_err(|e| Error::new(format!("Invalid block hash: {}", e)))?;

        let block = blockchain
            .get_block(&block_hash)
            .ok_or_else(|| Error::new("Block not found"))?;

        Ok(block
            .body
            .transactions
            .iter()
            .map(|tx| core_transaction_to_gql(tx, self.height.parse::<u64>().unwrap_or(0)))
            .collect())
    }
}

/// GraphQL representation of a transaction.
#[derive(Clone)]
pub struct GqlTransaction {
    pub hash: String,
    pub version: String,
    pub from: GqlAddress,
    pub to: Option<GqlAddress>,
    pub value: String,
    pub data: Option<String>,
    pub block_height: String,
}

#[Object]
impl GqlTransaction {
    async fn hash(&self) -> &str {
        &self.hash
    }

    async fn version(&self) -> &str {
        &self.version
    }

    async fn from(&self) -> &GqlAddress {
        &self.from
    }

    async fn to(&self) -> Option<&GqlAddress> {
        self.to.as_ref()
    }

    async fn value(&self) -> &str {
        &self.value
    }

    async fn data(&self) -> Option<&str> {
        self.data.as_deref()
    }

    async fn block_height(&self) -> &str {
        &self.block_height
    }
}

/// GraphQL representation of an address with chain state.
#[derive(Clone)]
pub struct GqlAddressInfo {
    pub address: String,
    pub balance: String,
    pub nonce: String,
    pub code_hash: Option<String>,
    pub storage_root: Option<String>,
    pub transaction_count: i32,
}

#[Object]
impl GqlAddressInfo {
    async fn address(&self) -> &str {
        &self.address
    }

    async fn balance(&self) -> &str {
        &self.balance
    }

    async fn nonce(&self) -> &str {
        &self.nonce
    }

    async fn code_hash(&self) -> Option<&str> {
        self.code_hash.as_deref()
    }

    async fn storage_root(&self) -> Option<&str> {
        self.storage_root.as_deref()
    }

    async fn transaction_count(&self) -> i32 {
        self.transaction_count
    }
}

/// GraphQL representation of a validator (derived from block proposers).
#[derive(Clone)]
pub struct GqlValidator {
    pub address: GqlAddress,
    pub blocks_proposed: i32,
    pub last_block_height: String,
}

#[Object]
impl GqlValidator {
    async fn address(&self) -> &GqlAddress {
        &self.address
    }

    async fn blocks_proposed(&self) -> i32 {
        self.blocks_proposed
    }

    async fn last_block_height(&self) -> &str {
        &self.last_block_height
    }
}

/// GraphQL representation of network statistics.
#[derive(Clone)]
pub struct GqlNetworkStats {
    pub block_height: String,
    pub finalized_height: String,
    pub block_count: i32,
    pub total_supply: String,
    pub mind_supply: String,
    pub peer_count: i32,
    pub syncing: bool,
}

#[Object]
impl GqlNetworkStats {
    async fn block_height(&self) -> &str {
        &self.block_height
    }

    async fn finalized_height(&self) -> &str {
        &self.finalized_height
    }

    async fn block_count(&self) -> i32 {
        self.block_count
    }

    async fn total_supply(&self) -> &str {
        &self.total_supply
    }

    async fn mind_supply(&self) -> &str {
        &self.mind_supply
    }

    async fn peer_count(&self) -> i32 {
        self.peer_count
    }

    async fn syncing(&self) -> bool {
        self.syncing
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

fn core_address_to_gql(addr: &CoreAddress) -> GqlAddress {
    GqlAddress {
        address: addr.to_hex(),
    }
}

fn core_transaction_to_gql(tx: &CoreTransaction, block_height: u64) -> GqlTransaction {
    let tx_hash = Hash512::blake3(&tx.serialize_bytes()).to_hex();
    GqlTransaction {
        hash: tx_hash,
        version: tx.version.to_string(),
        from: core_address_to_gql(&tx.from),
        to: tx.to.as_ref().map(core_address_to_gql),
        value: tx.value.to_string(),
        data: if tx.data.is_empty() {
            None
        } else {
            Some(hex::encode(&tx.data))
        },
        block_height: block_height.to_string(),
    }
}

fn core_block_to_gql(block_hash: &Hash512, block: &CoreBlock) -> GqlBlock {
    GqlBlock {
        hash: block_hash.to_hex(),
        height: block.header.height.to_string(),
        timestamp: block.header.timestamp.to_string(),
        shard_id: block.header.shard_id.to_string(),
        version: block.header.version.to_string(),
        difficulty: block.header.difficulty.to_string(),
        proposer: core_address_to_gql(&block.header.proposer),
        state_root: block.header.state_root.to_hex(),
        transaction_root: block.header.transaction_root.to_hex(),
        transaction_count: i32::try_from(block.body.transactions.len()).unwrap_or(i32::MAX),
    }
}

// ---------------------------------------------------------------------------
// Query root
// ---------------------------------------------------------------------------

/// Root query type for the Block Explorer GraphQL API.
pub struct Query;

#[Object]
impl Query {
    /// Look up a block by its hash.
    async fn block(&self, ctx: &Context<'_>, hash: Option<String>) -> Result<Option<GqlBlock>> {
        let Some(hash) = hash else {
            return Ok(None);
        };

        let block_hash = Hash512::from_hex(&hash)
            .map_err(|e| Error::new(format!("Invalid block hash: {}", e)))?;

        let app_state = ctx.data::<Arc<AppState>>()?;
        let blockchain = app_state
            .blockchain
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;

        let block = blockchain.get_block(&block_hash);
        Ok(block.map(|b| core_block_to_gql(&block_hash, b)))
    }

    /// Look up a block by its height.
    async fn block_by_height(&self, ctx: &Context<'_>, height: String) -> Result<Option<GqlBlock>> {
        let height = height
            .parse::<u64>()
            .map_err(|e| Error::new(format!("Invalid block height: {}", e)))?;

        let app_state = ctx.data::<Arc<AppState>>()?;
        let blockchain = app_state
            .blockchain
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;

        let (block_hash, block) = blockchain
            .get_block_and_hash_by_height(height)
            .ok_or_else(|| Error::new("Block not found at height"))?;

        Ok(Some(core_block_to_gql(&block_hash, block)))
    }

    /// List blocks with pagination.
    async fn blocks(
        &self,
        ctx: &Context<'_>,
        first: Option<String>,
        offset: Option<String>,
    ) -> Result<Vec<GqlBlock>> {
        let app_state = ctx.data::<Arc<AppState>>()?;
        let blockchain = app_state
            .blockchain
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;

        let limit = first
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(20)
            .clamp(0, 100);
        let start = offset.and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
        let tip_height = usize::try_from(blockchain.tip_height()).unwrap_or(usize::MAX);

        let mut blocks = Vec::new();
        for height in start..tip_height {
            if blocks.len() >= limit {
                break;
            }
            if let Some((block_hash, block)) =
                blockchain.get_block_and_hash_by_height(height as u64)
            {
                blocks.push(core_block_to_gql(&block_hash, block));
            }
        }

        Ok(blocks)
    }

    /// Look up a transaction by its hash.
    ///
    /// Scans all blocks for a transaction whose BLAKE3 hash of its serialized
    /// bytes matches the requested hash. This is a linear scan and suitable
    /// for the MVP; a transaction index can be added later.
    async fn transaction(&self, ctx: &Context<'_>, hash: String) -> Result<Option<GqlTransaction>> {
        let app_state = ctx.data::<Arc<AppState>>()?;
        let blockchain = app_state
            .blockchain
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;

        let tip_height = blockchain.tip_height();

        for height in 0..=tip_height {
            if let Some((_, block)) = blockchain.get_block_and_hash_by_height(height) {
                for tx in &block.body.transactions {
                    let tx_hash = Hash512::blake3(&tx.serialize_bytes()).to_hex();
                    if tx_hash == hash {
                        return Ok(Some(core_transaction_to_gql(tx, height)));
                    }
                }
            }
        }

        Ok(None)
    }

    /// List transactions with pagination.
    async fn transactions(
        &self,
        ctx: &Context<'_>,
        first: Option<String>,
        offset: Option<String>,
    ) -> Result<Vec<GqlTransaction>> {
        let app_state = ctx.data::<Arc<AppState>>()?;
        let blockchain = app_state
            .blockchain
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;

        let limit = first
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(20)
            .clamp(0, 100);
        let start_offset = offset.and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
        let tip_height = usize::try_from(blockchain.tip_height()).unwrap_or(usize::MAX);

        let mut transactions = Vec::new();
        let mut skipped = 0usize;

        for height in 0..tip_height {
            if transactions.len() >= limit {
                break;
            }
            if let Some((_, block)) = blockchain.get_block_and_hash_by_height(height as u64) {
                for tx in &block.body.transactions {
                    if skipped < start_offset {
                        skipped += 1;
                        continue;
                    }
                    transactions.push(core_transaction_to_gql(tx, height as u64));
                    if transactions.len() >= limit {
                        break;
                    }
                }
            }
        }

        Ok(transactions)
    }

    /// Look up an address and its chain state.
    async fn address(&self, ctx: &Context<'_>, address: String) -> Result<Option<GqlAddressInfo>> {
        let core_address = CoreAddress::from_hex(&address)
            .map_err(|e| Error::new(format!("Invalid address: {}", e)))?;

        let app_state = ctx.data::<Arc<AppState>>()?;
        let chain_state = app_state
            .chain_state
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;

        let account = chain_state.get_account(&core_address);
        let balance = chain_state.balance(&core_address);
        let nonce = chain_state.nonce(&core_address);

        let (code_hash, storage_root) = match account {
            Some(acc) => (
                acc.code_hash.as_ref().map(|h| h.to_hex()),
                Some(acc.storage_root.to_hex()),
            ),
            None => (None, None),
        };

        Ok(Some(GqlAddressInfo {
            address: core_address.to_hex(),
            balance: balance.to_string(),
            nonce: nonce.to_string(),
            code_hash,
            storage_root,
            transaction_count: nonce as i32,
        }))
    }

    /// List validators (proposers) ranked by blocks produced.
    async fn validators(
        &self,
        ctx: &Context<'_>,
        first: Option<String>,
    ) -> Result<Vec<GqlValidator>> {
        let app_state = ctx.data::<Arc<AppState>>()?;
        let blockchain = app_state
            .blockchain
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;

        let limit = first
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(50)
            .clamp(0, 500);
        let tip_height = usize::try_from(blockchain.tip_height()).unwrap_or(usize::MAX);

        // Count blocks proposed by each address, tracking the last height.
        let mut proposer_counts: HashMap<CoreAddress, (i32, u64)> = HashMap::new();

        for height in 0..tip_height {
            if let Some((_, block)) = blockchain.get_block_and_hash_by_height(height as u64) {
                let proposer = block.header.proposer;
                let height_u64 = height as u64;
                let entry = proposer_counts.entry(proposer).or_insert((0, height_u64));
                entry.0 += 1;
                entry.1 = height_u64;
            }
        }

        let mut validators: Vec<GqlValidator> = proposer_counts
            .into_iter()
            .map(|(addr, (count, last_height))| GqlValidator {
                address: core_address_to_gql(&addr),
                blocks_proposed: count,
                last_block_height: last_height.to_string(),
            })
            .collect();

        validators.sort_by(|a, b| b.blocks_proposed.cmp(&a.blocks_proposed));
        validators.truncate(limit);

        Ok(validators)
    }

    /// Network-wide statistics.
    async fn network_stats(&self, ctx: &Context<'_>) -> Result<GqlNetworkStats> {
        let app_state = ctx.data::<Arc<AppState>>()?;

        let blockchain = app_state
            .blockchain
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;
        let chain_state = app_state
            .chain_state
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;
        let peer_manager = app_state
            .peer_manager
            .read()
            .map_err(|e| Error::new(format!("database lock error: {}", e)))?;

        Ok(GqlNetworkStats {
            block_height: blockchain.tip_height().to_string(),
            finalized_height: blockchain.finalized_height().to_string(),
            block_count: blockchain.block_count() as i32,
            total_supply: chain_state.total_supply().to_string(),
            mind_supply: chain_state.mind_supply().to_string(),
            peer_count: peer_manager.connected_count() as i32,
            syncing: app_state.monitor.is_syncing(),
        })
    }
}

// ---------------------------------------------------------------------------
// Schema builder
// ---------------------------------------------------------------------------

/// Builds the GraphQL schema with the `Query` root type.
pub fn build_schema() -> Schema<Query, EmptyMutation, EmptySubscription> {
    Schema::build(Query, EmptyMutation, EmptySubscription).finish()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::api::auth::{AuthConfig, AuthState, RateLimiter};
    use crate::api::graphql::{build_schema, AppState};
    use crate::chain::blockchain::Blockchain;
    use crate::chain::state::ChainState;
    use crate::network::peer_manager::PeerManager;
    use std::sync::{Arc, RwLock};

    fn test_state() -> Arc<AppState> {
        let blockchain = Arc::new(RwLock::new(Blockchain::default()));
        let chain_state = Arc::new(RwLock::new(ChainState::new()));
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(256)));
        let auth = Arc::new(AuthState::new(
            AuthConfig::new(vec!["test-key".to_string()]),
            RateLimiter::new(60),
        ));
        let metrics = Arc::new(
            crate::telemetry::metrics::Metrics::new().expect("metrics should build in tests"),
        );
        let monitor = Arc::new(crate::telemetry::health::HealthMonitor::new(
            Arc::clone(&blockchain),
            Arc::clone(&peer_manager),
        ));

        Arc::new(AppState {
            blockchain,
            chain_state,
            peer_manager,
            monitor,
            metrics,
            auth,
        })
    }

    #[tokio::test]
    async fn query_network_stats() {
        let schema = build_schema();
        let state = test_state();

        let query =
            "{ networkStats { blockHeight finalizedHeight blockCount totalSupply mindSupply peerCount syncing } }";
        let request = async_graphql::Request::new(query).data(state.clone());
        let result = schema.execute(request).await;
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[tokio::test]
    async fn query_block_by_height() {
        let schema = build_schema();
        let state = test_state();

        let query =
            "{ blockByHeight(height: \"0\") { hash height timestamp proposer { address } } }";
        let request = async_graphql::Request::new(query).data(state.clone());
        let result = schema.execute(request).await;
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[tokio::test]
    async fn query_blocks_list() {
        let schema = build_schema();
        let state = test_state();

        let query = "{ blocks(first: \"5\") { hash height } }";
        let request = async_graphql::Request::new(query).data(state.clone());
        let result = schema.execute(request).await;
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[tokio::test]
    async fn query_address() {
        let schema = build_schema();
        let state = test_state();

        let query = "{ address(address: \"0x0000000000000000000000000000000000000000\") { address balance nonce } }";
        let request = async_graphql::Request::new(query).data(state.clone());
        let result = schema.execute(request).await;
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[tokio::test]
    async fn query_invalid_address_returns_error() {
        let schema = build_schema();
        let state = test_state();

        let query = "{ address(address: \"not-an-address\") { address } }";
        let request = async_graphql::Request::new(query).data(state.clone());
        let result = schema.execute(request).await;
        assert!(!result.errors.is_empty());
    }

    #[tokio::test]
    async fn query_validators() {
        let schema = build_schema();
        let state = test_state();

        let query =
            "{ validators(first: \"10\") { address { address } blocksProposed lastBlockHeight } }";
        let request = async_graphql::Request::new(query).data(state.clone());
        let result = schema.execute(request).await;
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[tokio::test]
    async fn numeric_fields_are_strings() {
        let schema = build_schema();
        let state = test_state();

        let query = "{ networkStats { blockHeight } }";
        let request = async_graphql::Request::new(query).data(state.clone());
        let result = schema.execute(request).await;
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        // Verify blockHeight is serialized as a string (not a number)
        let json = serde_json::to_value(&result).unwrap();
        let block_height = json["data"]["networkStats"]["blockHeight"].as_str();
        assert!(
            block_height.is_some(),
            "blockHeight should be a string, got: {:?}",
            json
        );
    }
}
