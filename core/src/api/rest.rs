//! REST API server for the Atoll Protocol node.
//!
//! Serves the `/health` endpoint, the `/metrics` endpoint (Prometheus text
//! format), and the `/graphql` endpoint (GraphQL over HTTP for the Block
//! Explorer).

use crate::api::auth::auth_and_rate_limit;
use crate::api::graphql::build_schema;
use crate::api::health::health_handler;
use crate::chain::blockchain::Blockchain;
use crate::chain::state::ChainState;
use crate::network::peer_manager::PeerManager;
use crate::primitives::address::Address;
use crate::telemetry::health::HealthMonitor;
use crate::telemetry::metrics::Metrics;
use async_graphql::Request as GraphQLRequest;
use axum::extract::ws::{Message, WebSocket};
use axum::{
    body::Body,
    extract::{State, WebSocketUpgrade},
    http::{header, StatusCode},
    middleware,
    response::Response,
    routing::{get, post},
    Router,
};
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use serde_json::{json, Value};
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
    let schema = build_schema();

    Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/rpc", post(rpc_handler))
        .route("/ws", get(ws_handler))
        .route(
            "/graphql",
            post(
                move |State(app_state): State<Arc<AppState>>, body: String| {
                    let schema = schema.clone();
                    async move {
                        let request: GraphQLRequest =
                            serde_json::from_str(&body).unwrap_or_else(|_| GraphQLRequest::new(""));
                        let request = request.data(app_state);
                        let response = schema.execute(request).await;
                        let json =
                            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
                        Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from(json))
                            .unwrap()
                    }
                },
            ),
        )
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

/// Starts the REST API server on port 8080.
///
/// Serves over plain HTTP. For production deployments, TLS termination should
/// be handled by a reverse proxy (nginx, traefik, cloud load balancer) in
/// front of this server. Self-signed TLS certificates can be generated with
/// [`crate::api::tls::generate_self_signed`] and configured at the proxy level.
pub async fn run_server(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "REST API listening");
    axum::serve(listener, router(Arc::new(state)))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// JSON-RPC 2.0 handler for `POST /rpc`.
///
/// Supports a minimal set of methods:
/// - `atoll.getInfo` — returns node info (chain ID, version, tip height)
/// - `atoll.getBalance` — returns balance for an address (params: [address])
/// - `atoll.getPeerCount` — returns connected peer count
///
/// Returns JSON-RPC 2.0 response format.
pub async fn rpc_handler(
    State(app_state): State<Arc<AppState>>,
    body: String,
) -> Result<Response, StatusCode> {
    let request: Value = serde_json::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

    let params = request
        .get("params")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or(vec![]);

    let id = request.get("id").cloned();

    let result = match method {
        "atoll.getInfo" => {
            let blockchain = app_state
                .blockchain
                .read()
                .expect("blockchain lock poisoned");
            json!({
                "chain_id": crate::CHAIN_ID,
                "version": env!("CARGO_PKG_VERSION"),
                "tip_height": blockchain.tip_height(),
                "tip_hash": format!("{:x}", blockchain.tip_hash().0[0]),
            })
        }
        "atoll.getBalance" => {
            let address_hex = params
                .get(0)
                .and_then(|p| p.as_str())
                .ok_or(StatusCode::BAD_REQUEST)?;
            let address = Address::from_hex(address_hex).map_err(|_| StatusCode::BAD_REQUEST)?;
            let chain_state = app_state
                .chain_state
                .read()
                .expect("chain state lock poisoned");
            json!({
                "address": address_hex,
                "balance": chain_state.balance(&address),
            })
        }
        "atoll.getPeerCount" => {
            let peer_manager = app_state
                .peer_manager
                .read()
                .expect("peer manager lock poisoned");
            json!({
                "peer_count": peer_manager.connected_count(),
            })
        }
        _ => {
            let error_response = json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {method}")
                },
                "id": id
            });
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&error_response).unwrap_or_default(),
                ))
                .unwrap());
        }
    };

    let response = json!({
        "jsonrpc": "2.0",
        "result": result,
        "id": id
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&response).unwrap_or_default(),
        ))
        .unwrap())
}

/// WebSocket handler for `GET /ws`.
///
/// Accepts WebSocket connections and sends a welcome message with node info.
/// Clients can subscribe to real-time updates (tip height, peer count, etc.).
pub async fn ws_handler(ws: WebSocketUpgrade, State(app_state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket: WebSocket| handle_socket(socket, app_state))
}

async fn handle_socket(socket: WebSocket, app_state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Send welcome message
    let welcome = json!({
        "type": "welcome",
        "chain_id": crate::CHAIN_ID,
        "version": env!("CARGO_PKG_VERSION"),
    });
    if let Err(_) = sender.send(Message::Text(welcome.to_string())).await {
        return;
    }

    // Echo loop: receive messages and respond with current state
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                let response = json!({
                    "type": "echo",
                    "received": text,
                    "tip_height": app_state.blockchain.read()
                        .map(|b| b.tip_height())
                        .unwrap_or(0),
                });
                if sender
                    .send(Message::Text(response.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
