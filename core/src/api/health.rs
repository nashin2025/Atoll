//! `GET /health` handler for the Atoll Protocol node.
//!
//! Returns the node's current operational status as JSON:
//! `{ status, version, chain_id, block_height, syncing, peers }`.

use crate::api::auth::{AuthConfig, AuthState, RateLimiter};
use crate::api::rest::AppState;
use crate::telemetry::health::HealthMonitor;
use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use std::sync::Arc;

/// JSON response body for `GET /health`.
#[derive(Clone, serde::Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub chain_id: u64,
    pub block_height: u64,
    pub syncing: bool,
    pub peers: usize,
}

/// Axum handler for `GET /health`.
///
/// Reads a fresh health snapshot from the [`HealthMonitor`] held in
/// [`AppState`] and returns it as JSON.
pub async fn health_handler(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let status = app_state.monitor.check();
    Ok(Json(HealthResponse {
        status: status.status,
        version: status.version,
        chain_id: status.chain_id,
        block_height: status.block_height,
        syncing: status.syncing,
        peers: status.peers,
    }))
}

/// Builds the axum router exposing `GET /health`.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use crate::telemetry::metrics::Metrics;

    use super::*;
    use crate::chain::blockchain::Blockchain;
    use crate::chain::state::ChainState;
    use crate::network::peer_manager::PeerManager;
    use std::sync::RwLock;

    fn test_state() -> Arc<AppState> {
        let blockchain = Arc::new(RwLock::new(Blockchain::default()));
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(256)));
        let metrics = Arc::new(Metrics::new().expect("metrics should build in tests"));
        let auth = Arc::new(AuthState::new(
            AuthConfig::new(vec!["test-key".to_string()]),
            RateLimiter::new(60),
        ));
        let monitor = Arc::new(HealthMonitor::new(
            Arc::clone(&blockchain),
            Arc::clone(&peer_manager),
        ));
        Arc::new(AppState {
            blockchain,
            chain_state: Arc::new(RwLock::new(ChainState::new())),
            peer_manager,
            monitor,
            metrics,
            auth,
        })
    }

    #[tokio::test]
    async fn health_handler_returns_healthy_status() {
        let state = test_state();
        let Json(parsed) = health_handler(State(Arc::clone(&state))).await.unwrap();
        assert_eq!(parsed.status, "healthy");
        assert!(!parsed.syncing);
        assert_eq!(parsed.chain_id, crate::CHAIN_ID);
        assert_eq!(parsed.block_height, 0);
        assert_eq!(parsed.peers, 0);
        assert!(!parsed.version.is_empty());
    }

    #[tokio::test]
    async fn health_handler_reports_syncing() {
        let state = test_state();
        state.monitor.set_syncing(true);
        let Json(parsed) = health_handler(State(Arc::clone(&state))).await.unwrap();
        assert_eq!(parsed.status, "syncing");
        assert!(parsed.syncing);
    }

    #[tokio::test]
    async fn health_handler_reports_peer_count() {
        let state = test_state();
        state.peer_manager.write().expect("lock").add_peer(
            [1u8; 32],
            crate::network::PeerInfo {
                node_id: [1u8; 32],
                address: "127.0.0.1".to_string(),
                port: 30333,
                last_seen: 0,
            },
        );
        let Json(parsed) = health_handler(State(Arc::clone(&state))).await.unwrap();
        assert_eq!(parsed.peers, 1);
    }
}
