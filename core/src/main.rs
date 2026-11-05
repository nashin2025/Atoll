//! Atoll Protocol Binary Entry Point
//!
//! Starts the node: initialises the blockchain, chain state, peer manager and
//! health monitor, then serves the REST API (including `GET /health`).

use atoll_core::{
    api::auth::{AuthConfig, AuthState, RateLimiter},
    api::rest::{run_server, AppState},
    chain::blockchain::Blockchain,
    chain::state::ChainState,
    network::peer_manager::PeerManager,
    telemetry::health::HealthMonitor,
    telemetry::logging::{init, LogOutput},
    telemetry::metrics::Metrics,
};
use clap::Parser;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Parser)]
#[command(name = "atoll-node")]
#[command(about = "Atoll Protocol Mainnet Node")]
struct Cli {
    /// Path to the node configuration file.
    #[arg(long, default_value = "/etc/atoll/config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Structured JSON logging to stdout (container-friendly). Level is
    // controlled via the `RUST_LOG` environment variable (default `info`).
    let _log_guard = init(LogOutput::Stdout)?;

    tracing::info!(
        "Starting Atoll node (chain_id={}, version={})",
        atoll_core::CHAIN_ID,
        env!("CARGO_PKG_VERSION")
    );
    tracing::debug!("Configuration file: {}", cli.config);

    // Initialise protocol state. Genesis finalisation (Task 5) will replace
    // Blockchain::default() with a block built from GenesisConfig.
    let blockchain = Arc::new(RwLock::new(Blockchain::default()));
    let chain_state = Arc::new(RwLock::new(ChainState::new()));
    let peer_manager = Arc::new(RwLock::new(PeerManager::new(256)));

    // Prometheus metrics registry, exposed at `GET /metrics`.
    let metrics = Arc::new(Metrics::new()?);

    let monitor = HealthMonitor::new(Arc::clone(&blockchain), Arc::clone(&peer_manager));

    // API authentication + per-IP rate limiting.
    let auth_config = AuthConfig::from_env();
    let rate_limiter = RateLimiter::from_env();
    let auth = Arc::new(AuthState::new(auth_config, rate_limiter));

    let state = AppState {
        blockchain,
        chain_state,
        peer_manager,
        monitor: Arc::new(monitor),
        metrics,
        auth,
    };

    run_server(state).await?;

    Ok(())
}
