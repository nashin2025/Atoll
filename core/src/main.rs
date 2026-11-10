//! Atoll Protocol Binary Entry Point
//!
//! Starts the node: initialises the blockchain, chain state, peer manager and
//! health monitor, then serves the REST API (including `GET /health`).
//!
//! The `genesis` subcommand generates `genesis.json` (config + block + hash)
//! and applies the initial token distribution.

use atoll_core::{
    api::auth::{AuthConfig, AuthState, RateLimiter},
    api::rest::{run_server, AppState},
    api::tls::generate_self_signed,
    chain::blockchain::Blockchain,
    chain::genesis::{GenesisConfig, GenesisLoader},
    chain::state::ChainState,
    network::peer_manager::PeerManager,
    primitives::address::Address,
    telemetry::health::HealthMonitor,
    telemetry::logging::{init, LogOutput},
    telemetry::metrics::Metrics,
};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Parser)]
#[command(name = "atoll-node")]
#[command(about = "Atoll Protocol Mainnet Node")]
struct Cli {
    /// Path to the node configuration file.
    #[arg(long, default_value = "/etc/atoll/config.toml")]
    config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate the genesis block, write genesis.json, and compute the genesis hash.
    Genesis {
        /// Output path for genesis.json (default: genesis.json)
        #[arg(long, default_value = "genesis.json")]
        output: PathBuf,

        /// Optional path to a distribution.json file defining initial token allocations.
        /// If omitted, a default distribution with a seed address is used.
        #[arg(long)]
        distribution: Option<PathBuf>,
    },
}

/// Path to the genesis record file. Created on first run and reused on
/// subsequent runs so every node start shares the same genesis block.
const GENESIS_PATH: &str = "genesis.json";

/// Default token distribution: seed address gets 21M ATOLL.
fn default_distribution() -> HashMap<Address, u128> {
    let mut dist = HashMap::new();
    // Seed address: blake3 of the zero public key, first 20 bytes
    let seed = Address::from_public_key(&[0u8; 32]);
    // 21 million ATOLL (6 decimals) to the seed/treasury address
    dist.insert(seed, 21_000_000_000_000);
    dist
}

/// Loads genesis config from `genesis.json` if it exists, otherwise generates
/// a default config, builds the genesis block, writes `genesis.json`, and
/// returns the config. This ensures every node start uses the same genesis
/// block and a persistent record is kept on disk.
fn load_or_create_genesis() -> Result<GenesisConfig, Box<dyn std::error::Error>> {
    let path = PathBuf::from(GENESIS_PATH);

    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let config: GenesisConfig = serde_json::from_str(&content)?;
        tracing::info!("Loaded existing genesis from {}", path.display());
        Ok(config)
    } else {
        let config = GenesisConfig::default();
        let block = GenesisLoader::build_genesis_block(&config);
        let genesis_hash = GenesisLoader::genesis_hash(&config);

        let genesis_json = json!({
            "config": config,
            "block": block,
            "genesis_hash": genesis_hash.to_hex(),
        });
        fs::write(&path, serde_json::to_string_pretty(&genesis_json)?)?;

        tracing::info!("Generated genesis block at {}", path.display());
        tracing::info!("  Genesis hash: {}", genesis_hash.to_hex());
        tracing::info!("  Chain ID: {}", config.chain_id);
        tracing::info!("  Timestamp: {}", config.genesis_timestamp);

        Ok(config)
    }
}

fn run_genesis(
    output: PathBuf,
    distribution_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load or build genesis config
    let config = if let Some(dist_path) = &distribution_path {
        let content = fs::read_to_string(dist_path)?;
        let dist: HashMap<String, u128> = serde_json::from_str(&content)?;
        let mut initial_distribution = HashMap::new();
        for (hex, amount) in &dist {
            let addr = Address::from_hex(hex).map_err(|e| format!("invalid address {hex}: {e}"))?;
            initial_distribution.insert(addr, *amount);
        }
        GenesisConfig {
            chain_id: atoll_core::CHAIN_ID,
            genesis_timestamp: atoll_core::GENESIS_TIMESTAMP,
            initial_difficulty: atoll_core::INITIAL_DIFFICULTY as u64,
            initial_distribution,
        }
    } else {
        let dist = default_distribution();
        GenesisConfig {
            chain_id: atoll_core::CHAIN_ID,
            genesis_timestamp: atoll_core::GENESIS_TIMESTAMP,
            initial_difficulty: atoll_core::INITIAL_DIFFICULTY as u64,
            initial_distribution: dist,
        }
    };

    // Build genesis block and compute hash
    let block = GenesisLoader::build_genesis_block(&config);
    let genesis_hash = GenesisLoader::genesis_hash(&config);

    // Write genesis.json
    let genesis_json = json!({
        "config": config,
        "block": block,
        "genesis_hash": genesis_hash.to_hex(),
    });
    fs::write(&output, serde_json::to_string_pretty(&genesis_json)?)?;

    // Apply distribution to a fresh ChainState and verify
    let mut chain_state = ChainState::new();
    GenesisLoader::apply_distribution(&config, &mut chain_state)?;
    let total_supply = chain_state.total_supply();

    println!("Genesis block generated");
    println!("  Output: {}", output.display());
    println!("  Genesis hash: {}", genesis_hash.to_hex());
    println!("  Chain ID: {}", config.chain_id);
    println!("  Timestamp: {}", config.genesis_timestamp);
    println!("  Total supply: {} ATOLL (smallest units)", total_supply);
    println!(
        "  Distribution entries: {}",
        config.initial_distribution.len()
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Genesis {
            output,
            distribution,
        }) => {
            return run_genesis(output, distribution);
        }
        None => {}
    }

    // Structured JSON logging to stdout (container-friendly). Level is
    // controlled via the `RUST_LOG` environment variable (default `info`).
    let _log_guard = init(LogOutput::Stdout)?;

    tracing::info!(
        "Starting Atoll node (chain_id={}, version={})",
        atoll_core::CHAIN_ID,
        env!("CARGO_PKG_VERSION")
    );
    tracing::debug!("Configuration file: {}", cli.config);

    // Initialise protocol state. Load genesis from disk (creating it on first
    // run) so every node start shares the same genesis block.
    let genesis_config = load_or_create_genesis()?;
    let blockchain = Arc::new(RwLock::new(Blockchain::new(&genesis_config)));
    let chain_state = Arc::new(RwLock::new(ChainState::new()));
    let peer_manager = Arc::new(RwLock::new(PeerManager::new(256)));

    // Prometheus metrics registry, exposed at `GET /metrics`.
    let metrics = Arc::new(Metrics::new()?);

    let monitor = HealthMonitor::new(Arc::clone(&blockchain), Arc::clone(&peer_manager));

    // API authentication + per-IP rate limiting.
    let auth_config = AuthConfig::from_env();
    if !auth_config.has_keys() {
        tracing::error!("ATOLL_API_KEYS is not set; no API keys configured. Refusing to start.");
        std::process::exit(1);
    }
    let rate_limiter = RateLimiter::from_env();
    let auth = Arc::new(AuthState::new(auth_config, rate_limiter));

    // Generate self-signed TLS certificates for HTTPS (Task 8).
    // Certs are written to /etc/atoll/tls/ and used for the REST API,
    // including /rpc and /ws endpoints.
    let tls = match generate_self_signed() {
        Ok(creds) => {
            tracing::info!("TLS certificates written to {}", creds.cert_path.display());
            Some(creds)
        }
        Err(e) => {
            tracing::warn!(error = %e, "TLS certificate generation failed; serving plain HTTP");
            None
        }
    };

    let state = AppState {
        blockchain,
        chain_state,
        peer_manager,
        monitor: Arc::new(monitor),
        metrics,
        auth,
    };

    run_server(state, tls).await?;

    Ok(())
}
