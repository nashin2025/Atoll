//! REST API server for the Atoll Protocol node.
//!
//! Serves the `/health` endpoint and the `/metrics` endpoint (Prometheus text
//! format). RPC, GraphQL, and WebSocket endpoints are added in later tasks.

use crate::api::auth::auth_and_rate_limit;
use crate::api::health::health_handler;
use crate::chain::blockchain::Blockchain;
use crate::chain::state::ChainState;
use crate::network::peer_manager::PeerManager;
use crate::telemetry::health::HealthMonitor;
use crate::telemetry::metrics::Metrics;
use axum::{extract::State, http::StatusCode, middleware, routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;

/// Shared application state injected into every request handler.
pub struct AppState {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub chain_state: Arc<RwLock<ChainState>>,
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub monitor: Arc<HealthMonitor>,
    pub metrics: Arc<Metrics>,
    pub auth: Arc<crate::api::auth::AuthState>,
}

/// Builds the axum router with all REST routes.
///
/// Every route is wrapped with [`auth_and_rate_limit`], which validates the
/// `X-API-Key` header and applies per-IP token-bucket rate limiting.
pub fn router(state: Arc<AppState>) -> Router {
    let auth_state = state.auth.clone();
    Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            auth_state,
            auth_and_rate_limit,
        ))
}

/// Axum handler for `GET /metrics`.
///
/// Refreshes the live gauges (tip height, token supplies) from chain state so
/// each scrape observes current values, then returns all registered Prometheus
/// metrics in text format.
pub async fn metrics_handler(State(app_state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    {
        let blockchain = app_state
            .blockchain
            .read()
            .expect("blockchain lock poisoned");
        app_state.metrics.set_block_height(blockchain.tip_height());
    }
    {
        let chain_state = app_state
            .chain_state
            .read()
            .expect("chain state lock poisoned");
        app_state
            .metrics
            .set_supply_atoll(chain_state.total_supply());
        app_state.metrics.set_supply_mind(chain_state.mind_supply());
    }

    app_state
        .metrics
        .gather()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Starts the HTTP server on port 8080.
pub async fn run_server(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "REST API listening");
    axum::serve(listener, router(Arc::new(state)))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
