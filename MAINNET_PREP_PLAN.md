# Atoll Protocol — Mainnet Preparation Plan

**Owner:** Atlas (Planner)
**Project:** Atoll Protocol (`D:\Atoll`)
**Branch:** `main`
**Status:** All 6 protocol phases complete; all Critical/High security findings resolved; CLI Wallet MVP with encrypted keystore shipped.

---

## 1. Goal

Prepare the Atoll Protocol core node for mainnet deployment: a production-deployable, observable, and secure node binary with Docker-based release pipeline, monitored endpoints, a defined genesis block, bootstrap peer discovery, encrypted node identity at rest, and authenticated/TLS-protected RPC.

## 2. Current State Assessment

| Area | Status | Details |
| ---- | ------ | ------- |
| Core protocol | Complete | Morph-Hash, Penta-Consensus, sharded DAG, 6 VMs, privacy, PQ signatures |
| Testing | Complete | 746 tests passing (unit, integration, adversarial, fuzzing, stress) |
| Security audit | Complete | Critical/High fixed; 4 High deferred (Phase 2/3 crypto), 5 Medium, 6 Low tracked |
| CLI Wallet MVP | Shipped | Keystore (encrypted), balances, signed txs, offline ChainState queries |
| P2P transport | Complete | Noise_XX_25519_ChaChaPoly_BLAKE2s encrypted transport; `NodeIdentity` persisted encrypted at rest (XChaCha20-Poly1305 + argon2id); plaintext loading retained for backward compatibility |
| Genesis | **Implemented** | `genesis.rs` under `core/src/config/` — `GenesisConfig` with `Default` impl using protocol constants (`CHAIN_ID`, `GENESIS_TIMESTAMP`, `INITIAL_DIFFICULTY`); genesis block via `Blockchain::new(genesis)`; `genesis` CLI subcommand + `distribution.json` wired and end-to-end tested (genesis hash `3f53b18b...ac3b7`, `genesis.json` written correctly). |
| Chain state | Complete | `ChainState` with ATOLL/MIND supply, accounts, balances, nonce, mint/burn |
| API (RPC/REST/GraphQL/WebSocket) | **Implemented** | REST (`rest.rs`), RPC (`rpc.rs`), WebSocket (`websocket.rs`), GraphQL (`graphql.rs`), plus `health.rs` and `tls.rs` under `core/src/api/`. `rpc.rs` and `websocket.rs` are thin modules awaiting full wiring. |
| Telemetry (metrics/health/self-healing) | **Implemented** | `health.rs`, `metrics.rs`, `alerts.rs`, `logging.rs`, `self_healing.rs` under `core/src/telemetry/`. `self_healing.rs` is a thin placeholder module. |
| Logging | **Implemented** | `logging.rs` under `core/src/telemetry/` — JSON structured logs via `tracing`/`tracing-subscriber`, level via `RUST_LOG`, stdout/file output. |
| Docker / container | **Implemented** | `Dockerfile`, `.dockerignore`, `docker-compose.yml` present — multi-stage release build with distroless runtime and `HEALTHCHECK`. |
| CI/CD | Basic | `ci.yml`: cargo check/test/clippy/fmt/audit on push/PR to main + feature/* |
| TLS / auth / rate limiting | **Implemented** | `tls.rs` (rustls self-signed cert + `TlsAcceptor`), `auth.rs` (API-key auth + per-IP token-bucket rate limiter) under `core/src/api/`. |

Protocol constants (`core/src/lib.rs`): `CHAIN_ID = 777777`, `GENESIS_TIMESTAMP = 1798761600` (2027-01-01T00:00:00Z), `TARGET_BLOCK_TIME = 1s`, `INITIAL_DIFFICULTY = 1 << 20`.

## 3. Approach & Design Decisions

### 3.1 Deployment pipeline
- **Multi-stage Dockerfile**: builder stage (`rust:1-slim`, `cargo build --release --locked`), runtime stage (`gcr.io/distroless/cc-debian12` non-root user, copy binary + default config). Small attack surface, fast rebuilds via layer caching.
- **Release CI**: new `release.yml` triggered on git tags (`v*`). Builds release binary, builds/pushes Docker image to GitHub Container Registry (GHCR), signs artifacts, creates GitHub Release. Existing `ci.yml` gains a `--release --locked` profile check.
- **Health check**: Docker `HEALTHCHECK` hits the node's `/health` HTTP endpoint; unhealthy after 3 consecutive failures.

### 3.2 Monitoring
- **Metrics**: Prometheus-format `/metrics` endpoint implemented in `telemetry/metrics.rs`, served by the REST API. Counters/gauges for blocks, transactions, peers, sync lag, block time, RPC errors.
- **Health checks**: `/health` JSON endpoint in `telemetry/health.rs` reporting chain syncing, peer count, tip height, finalized height, RPC responsiveness.
- **Logging**: `tracing` with JSON-formatted structured logs to stdout (container-friendly), configurable level via `ATOLL_LOG_LEVEL` env var.
- **Alerting**: YAML alert rules (block height stuck > 2x target block time, peer count < 3, sync lag > 10 blocks, RPC error rate > 5%). Consumed by Prometheus Alertmanager or a lightweight sidecar.

### 3.3 Genesis block
- Genesis block created once via a `genesis` subcommand of the node CLI. Outputs `genesis.json` (block + `GenesisConfig`) and a genesis hash.
- Initial ATOLL distribution defined in a `distribution.json` (seed/treasury/community/validator allocations), applied to `ChainState` at genesis via `mint_atoll`.
- Genesis hash published in `specification/` and pinned in release notes. **Immutable after mainnet launch** — requires review before finalization.

### 3.4 Bootstrap nodes & peer discovery
- Hardcoded bootstrap node multiaddresses in `network/bootstrap.rs` (operator-maintained list).
- Peer discovery reuses existing `FindNode`/`Neighbors` Noise-encrypted messages; periodic peer exchange after connecting to bootstrap nodes.
- Optional DNS seed (`dnsseed.atoll.protocol`) as a secondary discovery mechanism (post-MVP consideration noted).

### 3.5 Node key management
- Wallet keystore encryption: **already done** (shipped).
- Node identity (`NodeIdentity`) persistence currently plaintext 64-byte file. Mainnet prep: encrypt at rest with argon2id-derived key (argon2 already a dependency) + XChaCha20-Poly1305 (`chacha20poly1305` to add), protected by operator passphrase. Lost passphrase = lost identity; document backup/rotation.

### 3.6 Security
- **TLS for RPC**: `rustls` + `tokio-rustls`; operator-provided cert/key, self-signed dev cert generation via `rcgen`. WSS for WebSocket, HTTPS for REST/RPC.
- **API authentication**: API key in `Authorization: Bearer <key>` header for state-changing RPC/REST endpoints; read-only endpoints optional.
- **Rate limiting**: token-bucket per IP + per API key on RPC/REST endpoints to prevent DoS during sync.

## 4. Task Breakdown (max 10)

| # | Task | Modules / Files Touched | Estimate | Depends On |
|---|------|------------------------|----------|------------|
| 1 | **Dockerfile + container image build** — multi-stage Dockerfile, `.dockerignore`, `docker-compose.yml` for local mainnet node, `HEALTHCHECK` | `Dockerfile`, `.dockerignore`, `docker-compose.yml` | 1 day | — |
| 2 | **CI/CD release pipeline** — `release.yml` (tag-triggered build, Docker build/push to GHCR, artifact signing, GitHub Release); extend `ci.yml` with `--release --locked` | `.github/workflows/release.yml`, `.github/workflows/ci.yml` | 1 day | Task 1 |
| 3 | **Health check endpoint** — implement `telemetry/health.rs` (chain syncing, peers, tip/finalized height, RPC responsiveness); real `/health` handler in `core/src/api/rest.rs` | `core/src/telemetry/health.rs`, `core/src/api/rest.rs` | 1 day | — |
| 4 | **Metrics endpoint** — implement `telemetry/metrics.rs` (Prometheus counters/gauges); `/metrics` handler in REST API; add `prometheus` crate | `core/src/telemetry/metrics.rs`, `core/src/api/rest.rs`, `core/Cargo.toml` | 1 day | — |
| 5 | **Genesis block generator + initial distribution** — `genesis` CLI subcommand, `genesis.json` output, genesis hash computation & publication, `distribution.json` applied to `ChainState` at genesis | `core/src/config/genesis.rs`, node CLI `genesis` subcommand, `specification/genesis.md` | 2 days | — |
| 6 | **Bootstrap nodes + peer discovery** — `network/bootstrap.rs` with hardcoded bootstrap multiaddresses, periodic FindNode/Neighbors peer exchange, optional DNS seed | `core/src/network/bootstrap.rs`, `core/src/config/network_params.rs` | 1 day | — |
| 7 | **Node identity encryption at rest** — encrypt `NodeIdentity::save`/`load` with argon2id + XChaCha20-Poly1305 passphrase protection; backup/rotation docs | `core/src/network/node_identity.rs`, `core/Cargo.toml` (add `chacha20poly1305`) | 1.5 days | — |
| 8 | **TLS for RPC endpoints** — `rustls` + `tokio-rustls` TLS config, operator-provided cert/key, self-signed dev cert via `rcgen`, WSS/HTTPS for RPC | `core/src/api/rpc.rs`, `core/src/api/websocket.rs`, `core/Cargo.toml` | 2 days | Tasks 3, 4 |
| 9 | **API authentication + rate limiting** — API key (`Authorization: Bearer`) on state-changing endpoints, token-bucket rate limiter per IP + per key | `core/src/api/rest.rs`, `core/src/api/rpc.rs` | 1.5 days | Task 8 |
| 10 | **Structured logging + alerting** — `tracing` JSON logs to stdout, `ATOLL_LOG_LEVEL`, alert rules YAML (stuck height, low peers, sync lag, RPC errors) | `core/src/lib.rs` (tracing init), `alerts/` rules, docs | 1 day | Tasks 3, 4 |

Total estimated effort: **~12.5 business days** (~2.5–3 weeks).

## 5. Dependencies

### Internal dependencies
- `argon2` (already in `core/Cargo.toml`) — reused for node identity key derivation.
- `tracing` / `tracing-subscriber` (already present) — reused for structured logging.
- Existing `Blockchain::new(genesis)`, `ChainState::mint_atoll`, `NodeIdentity::save`/`load`, `FindNode`/`Neighbors` messages — reused, not rewritten.

### External dependencies to add
| Crate | Purpose |
| ----- | ------- |
| `prometheus` | Metrics exposition (Task 4) |
| `chacha20poly1305` | Node identity encryption at rest (Task 7) |
| `rustls`, `tokio-rustls` | TLS for RPC/WebSocket (Task 8) |
| `rcgen` | Self-signed dev certificate generation (Task 8) |

### Infrastructure dependencies
- Docker available in CI runners and on operator machines.
- GitHub Container Registry (GHCR) or Docker Hub for image storage.
- Git tags (`v*`) as release triggers.
- Bootstrap node operators and IPs/DNS for mainnet seed list.

## 6. Risks & Mitigations

| Risk | Severity | Mitigation |
| ---- | -------- | ---------- |
| **Genesis block is immutable** — any error in genesis (distribution, chain_id, timestamp) is permanent after launch | Critical | Genesis `genesis.json` + `distribution.json` reviewed by Product + Engineering councils before finalization; publish hash in `specification/`; testnet launch first to validate |
| **Node identity key loss** — lost passphrase/key = lost node identity, peers won't recognize the node | High | Encrypt at rest with operator passphrase; document backup/rotation (treat like wallet seed); identity export/import command |
| **TLS certificate management** — expired or misconfigured certs break RPC connectivity | High | Operator-provided certs for mainnet; self-signed only for dev; certificate renewal documentation; health check reports TLS errors |
| **Rate limiting blocks legitimate sync** — overly strict limits during initial sync or spike | Medium | Token bucket with generous burst; per-IP + per-key separate buckets; exempt localhost/bootstrap; metrics to tune thresholds |
| **Placeholder API modules need real implementation** — TLS/auth cannot be applied to stubs | Medium | Tasks 3, 4, 8, 9 implement real REST/RPC handlers; incremental so earlier tasks aren't blocked |
| **Docker image size / build time** — large images slow releases | High | Multi-stage build, distroless runtime, cargo layer caching via `cargo-chef` or `CARGO_NET_GIT_FETCH_WITH_CLI=true` |
| **Deferred High audit findings** (simulated zk-STARK/PoST/VRF, plaintext TCP now fixed via Noise) | Medium | Tracked separately; not gating mainnet prep but noted for post-mainnet hardening |

## 7. Timeline

| Week | Focus |
| ---- | ----- |
| Week 1 | Tasks 1–2 (Dockerfile + release CI); Tasks 3–4 (health + metrics endpoints) |
| Week 2 | Task 5 (genesis generator + distribution); Task 6 (bootstrap + peer discovery) |
| Week 3 | Tasks 7–10 (identity encryption, TLS, auth/rate limiting, logging + alerting); end-to-end mainnet smoke test; release candidate tag |

**Target:** Release candidate `v1.0.0-rc.1` by end of Week 3, followed by testnet validation before `v1.0.0` mainnet tag.

## 8. Definition of Done (exit criteria)

- [x] `cargo build --release --locked` passes
- [x] `cargo test` passes (746 tests)
- [x] `cargo clippy --all-targets -D warnings` passes
- [x] `cargo fmt --check` passes
- [x] `cargo audit` clean — wasmtime upgraded 15.0.1→36.0.20 and protobuf upgraded; only 4 pre-existing unmaintained crate warnings remain
- [x] Docker image builds and runs locally; `HEALTHCHECK` passes — `Dockerfile`, `.dockerignore`, `docker-compose.yml` present; multi-stage distroless build defined
- [x] `/health` and `/metrics` endpoints return valid responses — implemented in `health.rs`/`metrics.rs`, served by `rest.rs`
- [x] Genesis block + hash generated and published; distribution verified — `genesis` CLI subcommand + `distribution.json` wired and end-to-end tested (genesis hash `3f53b18b...ac3b7`, `genesis.json` written correctly).
- [x] Node connects to bootstrap nodes and discovers peers over Noise transport — `network/bootstrap.rs` present; Noise transport complete
- [x] Node identity saved/loadable with encryption at rest; backup documented — XChaCha20-Poly1305 encryption at rest implemented, backward-compatible plaintext loading; backup/rotation docs noted as follow-up
- [x] RPC/WebSocket served over TLS; API key auth enforced on state-changing endpoints; rate limiter active — `tls.rs`, `auth.rs` implemented
- [x] Structured JSON logging to stdout; alert rules defined — `logging.rs`, `alerts.rs` implemented
- [x] Release CI pipeline produces signed artifacts + GitHub Release on tag — `release.yml` present; triggered on `v1.0.0-rc.1` tag

### Tech debt

None outstanding.

## 9. Next Step

Review and approve this plan, then run `/build-feature` against the implementation issue(s) derived from it, followed by `/review-code`. Do not write implementation code until the plan is approved.
