# Atoll Block Explorer — Implementation Plan

**Status:** Planned (not implemented)
**Branch:** `main`
**Owner:** Atlas (Planner)

---

## 1. Goal

Build a Block Explorer web application for the Atoll Protocol that lets users browse blocks, transactions, validators, and network stats in real time. It is a read-only window onto a running Atoll node, backed by a real GraphQL API implemented in Rust on the existing Axum server.

**Target users:** node operators, researchers, and end users who want to inspect chain state without running the CLI wallet.

**Scope boundary:** This is **read-only exploration only**. No transaction submission, no wallet integration, no signing. The GUI Wallet (Tauri) already handles offline tx creation; the explorer is a separate, standalone web app.

---

## 2. Existing Infrastructure (read-only audit)

| Component | Location | Status |
|---|---|---|
| REST server (Axum) | `core/src/api/rest.rs` | `/health` and `/metrics` on port 8080; `AppState` holds `Blockchain`, `ChainState`, `PeerManager`, `HealthMonitor`, `Metrics`, `AuthState` |
| GraphQL module | `core/src/api/graphql.rs` | Placeholder — compiles only (`assert!(true)` test) |
| Auth | `core/src/api/auth.rs` | `X-API-Key` header + per-IP token-bucket rate limiting; applies to all routes via middleware |
| Blockchain | `core/src/chain/blockchain.rs` | `Blockchain { tip_hash, tip_height, finalized_height, blocks, blocks_by_height, state_roots }`; `get_block(hash)`, `get_block_by_height(height)`, `block_count()`, `proposer_at(height)` |
| Chain state | `core/src/chain/state.rs` | `ChainState { accounts, total_supply, mind_supply, block_number }`; `AccountState { balance, nonce, code_hash, storage_root }`; `balance(address)`, `nonce(address)`, `get_account(address)`, `account_count()` |
| Block primitive | `core/src/primitives/block.rs` | `Block { header, body }`; `BlockHeader { version, height, shard_id, timestamp, prev_hash, state_root, transaction_root, proposer, difficulty }`; `BlockBody { transactions }` |
| Transaction primitive | `core/src/primitives/transaction.rs` | `Transaction { version, from, to: Option<Address>, value, data, signature }` |
| Address primitive | `core/src/primitives/address.rs` | `Address(pub [u8; 20])`; `to_hex()` / `from_hex()` |
| Hash primitive | `core/src/primitives/hash.rs` | `Hash512(pub [u8; 64])`; `to_hex()` / `from_hex()` |
| Signature primitive | `core/src/primitives/signature.rs` | `Signature { ed25519: [u8; 64], dilithium: Vec<u8> }` |
| Entry point | `core/src/main.rs` | Builds `AppState`, calls `run_server(state)` |
| GUI Wallet styling | `wallet/gui/src/components/ui.tsx` | Tailwind v3, dark slate theme (`bg-slate-950`, `border-slate-700/50`), indigo primary, `Button`/`Input`/`Loading`/`ErrorDisplay`/`AddressDisplay`/`NavLink` components |
| Explorer dirs | `explorer/backend/`, `explorer/frontend/` | Both empty |

**Key constraint:** `Blockchain` and `ChainState` are behind `Arc<RwLock<...>>` inside `AppState`. GraphQL resolvers must read through these locks. No write paths are needed.

---

## 3. Tech Stack

| Layer | Choice | Rationale |
|---|---|---|
| Backend language | Rust | Existing codebase is Rust; reuse `atoll-core` crate |
| GraphQL server | `async-graphql` + `async-graphql-axum` | Most mature Rust GraphQL library; native Axum integration; supports subscriptions later |
| HTTP framework | Axum (already a dependency) | Already in `core/Cargo.toml`; `/health` and `/metrics` already served here |
| Frontend | React 18 + TypeScript + Vite | Matches GUI Wallet (`wallet/gui`) |
| Styling | Tailwind CSS v3 | Matches GUI Wallet; reusable `ui.tsx` component patterns |
| GraphQL client | `@apollo/client` or `graphql-request` | Decision deferred to Task 4; `@apollo/client` for cache + polling, `graphql-request` for minimal bundle |
| Real-time | Polling (MVP) / GraphQL subscriptions (future) | Polling is simpler and reliable; subscriptions need WebSocket + auth handling |
| Deployment | Same binary as `atoll-node` (`core/src/main.rs`) | GraphQL endpoint mounted on the same Axum router on port 8080; no separate service |

**New dependencies to add (Rust):**
- `async-graphql = "7"`
- `async-graphql-axum = "7"`

**New dependencies to add (frontend):**
- `react`, `react-dom`, `typescript`, `vite`, `tailwindcss`, `postcss`, `autoprefixer`
- GraphQL client (TBD in Task 4)
- `vitest` + `@testing-library/react` (matching GUI Wallet)

---

## 4. GraphQL Schema (design)

All types are read-only. Hashes and addresses are serialized as hex strings (`0x` prefix for addresses, plain hex for hashes).

```graphql
type Block {
  hash: String!
  height: UInt64!
  timestamp: UInt64!
  shardId: UInt16!
  version: UInt32!
  difficulty: String!
  proposer: Address!
  stateRoot: String!
  transactionRoot: String!
  transactionCount: Int!
  transactions: [Transaction!]!
}

type Transaction {
  hash: String!
  version: UInt32!
  from: Address!
  to: Address
  value: String!
  data: String
}

type Address {
  address: String!
  balance: String!
  nonce: UInt64!
  codeHash: String
  storageRoot: String
}

type Validator {
  address: Address!
  blocksProposed: Int!
  lastBlockHeight: UInt64!
}

type NetworkStats {
  blockHeight: UInt64!
  finalizedHeight: UInt64!
  blockCount: Int!
  totalSupply: String!
  mindSupply: String!
  peerCount: Int!
  syncing: Boolean!
}

type Query {
  block(hash: String): Block
  blockByHeight(height: UInt64): Block
  blocks(first: Int, offset: Int): [Block!]!
  transaction(hash: String): Transaction
  transactions(first: Int, offset: Int): [Transaction!]!
  account(address: String!): Address
  validator(address: String!): Validator
  validators(first: Int): [Validator!]!
  networkStats: NetworkStats!
}
```

Scalar types: `UInt64`, `UInt32`, `UInt16` as custom scalars (JSON numbers can't safely represent u64; serialize as strings).

**Lookups required:**
- Block hash lookup → `Query.block(hash:)`
- Address lookup → `Query.account(address:)`
- Transaction hash lookup → `Query.transaction(hash:)`

---

## 5. Frontend Pages

| Page | Path | Purpose |
|---|---|---|
| Home | `/` | Network stats (tip height, finalized height, block count, supplies, peer count, syncing) + global search bar |
| Blocks list | `/blocks` | Paginated list of recent blocks |
| Block detail | `/blocks/:hash` | Full block header + transactions |
| Transactions list | `/transactions` | Paginated list of recent transactions |
| Transaction detail | `/transactions/:hash` | Full transaction fields |
| Validators list | `/validators` | Proposer addresses ranked by blocks proposed |
| Validator detail | `/validators/:address` | Address info + blocks proposed |

**Shared components** (matching `wallet/gui/src/components/ui.tsx`):
- `Button`, `Input`, `Loading`, `ErrorDisplay`, `AddressDisplay`, `NavLink`
- `Table` (paginated, sortable)
- `SearchBar` (block hash / address / tx hash → route to detail)

**Styling:** Dark slate theme (`bg-slate-950`, `border-slate-700/50`, `text-slate-300`), indigo primary (`indigo-600`), system font stack — identical to GUI Wallet so the two apps feel like one product.

---

## 6. Task Breakdown (max 10)

| # | Task | Layer | Size | Dependencies |
|---|---|---|---|---|
| 1 | Add `async-graphql` + `async-graphql-axum` to `core/Cargo.toml`; verify `cargo build` passes | Backend | XS | None |
| 2 | Implement GraphQL schema, custom scalars, types, and resolvers in `core/src/api/graphql.rs` (blocks, transactions, accounts, validators, stats, lookups by hash/address) | Backend | M | Task 1 |
| 3 | Mount `/graphql` endpoint on Axum router in `rest.rs`; wire into `main.rs`; apply existing auth + rate-limit middleware | Backend | S | Task 2 |
| 4 | Scaffold Explorer frontend (`explorer/frontend/`): Vite + React + TS + Tailwind; copy GUI Wallet theme (colors, fonts, base CSS); choose GraphQL client (`@apollo/client` vs `graphql-request`) | Frontend | S | None |
| 5 | Build shared UI components (`Button`, `Input`, `Loading`, `ErrorDisplay`, `AddressDisplay`, `NavLink`, `Table`, `SearchBar`) matching `wallet/gui/src/components/ui.tsx` | Frontend | S | Task 4 |
| 6 | Implement Home page (network stats) + global search (block hash / address / tx hash lookup) | Frontend | M | Task 5 |
| 7 | Implement Blocks page (list + detail) with block hash lookup | Frontend | M | Task 5, Task 3 |
| 8 | Implement Transactions page (list + detail) with transaction hash lookup | Frontend | M | Task 5, Task 3 |
| 9 | Implement Validators page (list + detail) with address lookup | Frontend | S | Task 5, Task 3 |
| 10 | Add real-time updates (polling interval, e.g. 5–10s) + build/test/docs (`package.json` scripts, README snippet, `.env.example`) | Frontend + Backend | S | Task 3, Tasks 6–9 |

**Estimated total:** ~21 business days (~4 weeks). Size key: XS=1 day, S=2 days, M=3 days (per VIBE_PLAN.md convention).

---

## 7. Dependencies (external)

| Dependency | Purpose | Version constraint |
|---|---|---|
| `async-graphql` | GraphQL schema/execution in Rust | `7.x` |
| `async-graphql-axum` | Axum integration for GraphQL endpoint | `7.x` |
| `@apollo/client` or `graphql-request` | GraphQL client in React | latest stable |
| `tailwindcss`, `postcss`, `autoprefix` | Styling | v3 (matches GUI Wallet) |
| `vitest`, `@testing-library/react` | Frontend testing | matches GUI Wallet |

No new system dependencies. The explorer frontend runs in the browser; the backend runs inside the existing `atoll-node` binary.

---

## 8. Risks

| Risk | Severity | Mitigation |
|---|---|---|
| **GraphQL + Axum + `Arc<RwLock<AppState>>` state sharing** — `async-graphql-axum` expects its own state type; bridging to `AppState` requires careful adapter code | High | Keep resolver context thin: pass `Arc<AppState>` (or clones of the inner `Arc<RwLock<>>` handles) into the GraphQL context; resolvers read locks synchronously (short reads only) |
| **Auth middleware applies to GraphQL** — frontend must send `X-API-Key` on every GraphQL request; dev key is `atoll-local` by default | Medium | Document `ATOLL_API_KEYS` env var; frontend reads `VITE_API_KEY` from `.env` and sends it as header |
| **Real-time via polling is not truly real-time** — chain updates appear after the poll interval | Medium | Use 5–10s polling interval for MVP; note GraphQL subscriptions over WebSocket as a post-MVP enhancement |
| **Large blocks / many transactions** — returning all transactions in a block could produce large payloads | Medium | Add `first`/`offset` pagination on `transactions` list and `block.transactions`; cap page size |
| **Address/hash format mismatch** — `0x` prefix vs plain hex, case sensitivity | Low | Normalize all hex output to lowercase with `0x` prefix for addresses; accept both with/without prefix on input |
| **Validator derivation** — `Blockchain.proposer_at(height)` gives proposers; "validator" is a derived concept (proposer over a window) | Low | Define validator = unique proposer address over recent blocks; blocksProposed = count in that window |
| **No existing GraphQL client in the repo** — frontend team must pick and integrate one | Low | Decision in Task 4; `graphql-request` is the minimal fallback if `@apollo/client` is too heavy |
| **Frontend and backend are separate repos/builds** — version drift between GraphQL schema and client queries | Medium | Keep schema in `core/src/api/graphql.rs` as source of truth; frontend queries are strings matched by hand; consider codegen post-MVP |

---

## 9. Timeline

| Week | Tasks | Deliverable |
|---|---|---|
| 1 | Tasks 1–3 (backend GraphQL + endpoint) | `GET /graphql` on port 8080 serving block/tx/account/validator/stats queries |
| 2 | Tasks 4–5 (frontend scaffold + components) | Vite + TS + Tailwind app; shared UI kit matching GUI Wallet |
| 3 | Tasks 6–8 (Home, Blocks, Transactions pages) | Browse blocks and transactions; global search |
| 4 | Tasks 9–10 (Validators, polling, docs) | Validator pages; real-time updates; build/test scripts |

**Total:** ~4 weeks (21 business days).

---

## 10. Decision Points (for user confirmation)

1. **GraphQL client:** `@apollo/client` (cache + polling built-in, larger bundle) vs `graphql-request` (minimal, manual polling). Recommendation: `@apollo/client` for the explorer's read-heavy, polling-based usage.
2. **GraphQL endpoint path:** `/graphql` on port 8080 (same as `/health`, `/metrics`) vs a separate port. Recommendation: same port, same Axum router — simplest deployment, single binary.
3. **Real-time:** Polling (MVP) vs WebSocket subscriptions (post-MVP). Recommendation: polling for MVP; subscriptions are a tracked follow-up.
4. **Plan file location:** `explorer/PLAN.md` — keep it next to the empty `backend/` and `frontend/` directories so implementation lands in the right place.

---

## 11. What is NOT in scope (MVP)

- Transaction submission / signing (GUI Wallet handles this offline)
- Wallet connection / address binding
- GraphQL subscriptions / WebSocket (post-MVP)
- Smart contract exploration (no VM in MVP)
- Cross-shard / sharding views (shardId is shown on blocks, no shard-specific pages)
- User accounts, authentication for the explorer UI itself (backend API key only)
- Caching layer (Redis, etc.) — resolvers read directly from `Blockchain`/`ChainState`

---

**Next step:** On approval, run `/build-feature` against the implementation issue created from this plan, starting with Task 1 (backend GraphQL dependencies).
