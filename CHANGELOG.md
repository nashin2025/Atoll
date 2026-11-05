# Changelog

All notable changes to the Atoll Protocol node are documented in this file.

## [v0.1.0-rc.1] — Release Candidate 1

### Overview
First release candidate for mainnet. Hardens the node for production operation: structured telemetry, finalized genesis, bootstrap-based peer discovery, TLS for RPC, and API authentication with rate limiting.

### Added

**Task 5 — Structured logging** (`core/src/telemetry/logging.rs`)
- JSON-formatted structured logs via `tracing` + `tracing-subscriber`.
- Log level controlled by the `RUST_LOG` environment variable (default `info`).
- Stdout output (container-friendly) or rotating file output via `tracing-appender` (`/var/log/atoll/`, daily rotation).
- Integrated into `core/src/main.rs`.

**Task 6 — Genesis block finalization** (`core/src/chain/genesis.rs`)
- `GenesisConfig`: `chain_id`, `genesis_timestamp`, `initial_difficulty`, `initial_distribution` (`HashMap<Address, u128>`).
- `GenesisLoader`: load config from JSON file or generate the default; builds the genesis block; validates the genesis hash (Morph-Hash); applies the initial distribution to chain state.
- `Blockchain::new(&GenesisConfig)` constructs the chain from the genesis config.

**Task 7 — Bootstrap nodes** (`core/src/network/bootstrap.rs`)
- `BootstrapNode` (`address`, `node_id`) with a hardcoded mainnet seed list.
- `BootstrapClient` connects to seed nodes over the Noise transport, exchanges `FindNode`/`Neighbors` discovery messages, and returns discovered peers.
- `BootstrapClient::register_discovered` integrates discovered peers with `DiscoveryProtocol` and `PeerManager`.
- Added `Neighbors` message deserialization to the P2P codec (`core/src/network/p2p.rs`).

**Task 8 — TLS for RPC** (`core/src/api/tls.rs`)
- Self-signed certificate generation via `rcgen` + `rcgen`'s `generate_simple_self_signed`.
- Certificates written to `/etc/atoll/tls/` (`cert.pem`, `key.pem`).
- `rustls` `ServerConfig` and `tokio-rustls` `TlsAcceptor` for HTTPS/TLS servers.
- Added `rustls`, `rcgen`, `tokio-rustls` to `core/Cargo.toml`.

**Task 9 — API auth + rate limiting** (`core/src/api/auth.rs`)
- API key validation via the `X-API-Key` header (keys from `ATOLL_API_KEYS`, dev fallback `atoll-local`).
- Per-IP token-bucket rate limiter, configurable RPM via `ATOLL_API_RATE_LIMIT_RPM` (default 60).
- Combined `auth_and_rate_limit` axum middleware returns `401 Unauthorized` / `429 Too Many Requests`.
- Wrapped around `/health` and `/metrics` in `core/src/api/rest.rs`.

### Changed
- `core/Cargo.toml`: added `serde_json`, `tracing-appender`, `rustls`, `rcgen`, `tokio-rustls`.
- `core/src/main.rs`: initialises structured logging and auth + rate-limit state.
- `Blockchain::default()` now builds genesis from `GenesisConfig::default()` (timestamp `GENESIS_TIMESTAMP`, difficulty `INITIAL_DIFFICULTY`).

### Testing
- 392 unit tests pass (`cargo test -p atoll-core --lib`), including 21 new tests for logging, genesis, bootstrap, TLS, and auth.
- `cargo clippy` and `cargo fmt --check` clean on all changed files.

### Note
This is a release candidate. The self-signed TLS certificate and dev API key (`atoll-local`) are intended for development/staging; production deployments should provide operator-managed certs and set `ATOLL_API_KEYS`.
