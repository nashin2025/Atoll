# GUI Wallet — Atoll Protocol (VIBE_PLAN.md)

## Goal
Build a desktop GUI wallet for the Atoll Protocol using TypeScript + Tauri, giving users a graphical interface to generate/import Ed25519 keys, check ATOLL balances, build and sign transactions, and view history. The Rust CLI wallet (`wallet-cli/`) already works offline; the GUI reuses that logic via Tauri commands so users never touch the command line.

## Target User
Atoll Protocol users who want to hold and send ATOLL from a desktop application without using the CLI. Developers who need to sign transactions and export signed tx hex for manual submission.

## MVP Scope
- **Create wallet**: generate a new Ed25519 key pair, encrypt to keystore, display address
- **Import wallet**: import an existing Ed25519 secret key (hex), encrypt to keystore
- **Unlock**: passphrase prompt to decrypt the keystore before balance/send operations
- **Check balance**: query ATOLL balance for an address (offline ChainState)
- **Send**: build, sign, and output an ATOLL transaction as hex (offline signing; signed hex ready for manual submission or future RPC)
- **History**: view transaction history for an address (MVP: offline stub, no tx log yet)
- **Keystore**: same encrypted `~/.atoll/keystore.json` as the CLI (XChaCha20-Poly1305 + argon2id), shared between CLI and GUI

Deferred (post-MVP): Dilithium/post-quantum signing, MIND/PROOF token balances, live node RPC submission, transaction broadcasting, hardware wallet support, address book, transaction details view.

## Tech Stack
| Layer | Choice | Rationale |
|-------|--------|-----------|
| Desktop framework | Tauri v2 (Rust backend) | Lightweight, native webview, Rust backend reuses existing wallet-cli logic |
| Frontend | React + TypeScript + Vite | Matches project's TS preference, fast dev server, type-safe |
| Styling | Tailwind CSS | Utility-first, rapid UI builds, no custom CSS framework needed |
| Backend language | Rust (edition 2021) | Reuses `atoll-core` types and `wallet-cli` crypto/keystore logic |
| Crypto | `ed25519`, `ed25519-dalek`, `chacha20poly1305`, `argon2` (via atoll-core) | Same as CLI wallet; no new crypto primitives |
| Serialization | `serde`, `serde_json` | Keystore envelope format, Tauri command arguments/responses |
| IPC | Tauri `invoke` (`#[tauri::command]`) | Frontend calls Rust commands, returns JSON-serializable results |
| Keystore path | `~/.atoll/keystore.json` (shared with CLI) | Single keystore file used by both CLI and GUI |

## Reuse Strategy
The GUI backend will call into the existing `wallet-cli` library functions (`cmd_new`, `cmd_import`, `cmd_balance`, `cmd_send`, `cmd_history`, `derive_address`, `Keystore::load`/`save`) rather than reimplementing them. The `WalletError` enum will be converted to a serializable error type for frontend consumption. The `rpassword` crate (interactive prompt) is not needed in the GUI — the passphrase comes from the frontend via Tauri invoke.

## Frontend Views
1. **Welcome / Unlock**: passphrase prompt to open an existing keystore, or option to create/import
2. **Create Wallet**: generate new key, show address, prompt for passphrase to encrypt
3. **Import Wallet**: paste secret key hex, show derived address, prompt for passphrase
4. **Dashboard**: show address, balance, recent activity (history stub)
5. **Send**: from address (auto-selected), to address, amount, passphrase, sign button, signed tx hex output with copy
6. **History**: address input, history list (MVP: "offline mode" message)

## Task Breakdown (max 10)

| # | Task | Layer | Description |
|---|------|-------|-------------|
| 1 | Scaffold Tauri project | Config | Create `wallet/gui/` Tauri v2 app with Rust backend + React/TypeScript/Vite frontend; add Tauri dependencies; configure `tauri.conf.json`; verify `cargo build` and `pnpm dev`/`npm run dev` work |
| 2 | Rust command wrappers | Backend | Add `#[tauri::command]` functions wrapping `wallet-cli` logic: `wallet_new`, `wallet_import`, `wallet_balance`, `wallet_send`, `wallet_history`, `wallet_derive_address`; define serializable request/response types (serde) |
| 3 | Error type for IPC | Backend | Create `GuiError` enum (serde Serialize) mapping `WalletError` variants to frontend-consumable errors; ensure all command errors surface as `GuiError` not panics |
| 4 | Keystore path + passphrase plumbing | Backend | Default keystore path `~/.atoll/keystore.json` (expand `~`, same as CLI); passphrase passed from frontend via invoke args (no `rpassword` interactive prompt); no plaintext fallback |
| 5 | Frontend: auth & wallet creation | Frontend | React views for welcome/unlock, create wallet, import wallet; passphrase input; call Tauri commands; display derived address; basic form validation |
| 6 | Frontend: balance & send | Frontend | React views for balance check (address input, balance display) and send (from/to/amount/passphrase, sign, display signed tx hex with copy button); amount validation (>0), balance pre-check |
| 7 | Frontend: history | Frontend | React view for transaction history (address input, history list); MVP shows offline-mode message since no tx log exists yet |
| 8 | Shared UI shell | Frontend | App layout, navigation between views, address display component, loading/error states, passphrase prompt modal; Tailwind styling |
| 9 | Tests | Testing | Rust unit tests for command wrappers (roundtrip new→import→address, sign+verify determinism, insufficient balance error); React component render tests; verify error propagation from Rust to frontend |
| 10 | Verification & docs | Verification | `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`; frontend `npm run build`/`npm run lint`; document keystore location, passphrase requirements, and how to submit signed tx hex in README |

## Dependencies

### Rust (backend, `wallet/gui/src-tauri/Cargo.toml`)
| Crate | Purpose |
|-------|---------|
| `tauri = "2"` | Tauri framework core |
| `tauri-build = "2"` | Build macro for `#[tauri::command]` |
| `serde`, `serde_json` | Serialization for command args/responses and keystore |
| `atoll-core = { path = "../../core" }` | Core types: `Address`, `Transaction`, `Signature`, `ChainState` |
| `wallet-cli = { path = "../cli" }` | Reuse existing wallet logic (if refactored as a library; otherwise duplicate needed functions) |
| `ed25519`, `ed25519-dalek` | Ed25519 key generation + signing |
| `chacha20poly1305`, `aead` | Keystore encryption (XChaCha20-Poly1305) |
| `argon2` (via atoll-core) | Passphrase → key derivation |
| `hex`, `rand` | Hex encoding, random key generation |
| `zeroize` | Scrub passphrase bytes from memory |

Note: `rpassword` is NOT needed (no TTY/passphrase prompt in GUI); passphrase comes from frontend.

### Frontend (`wallet/gui/`)
| Package | Purpose |
|---------|---------|
| `@tauri-apps/api` | Tauri `invoke` from TypeScript |
| `react`, `react-dom` | UI framework |
| `typescript` | Type safety |
| `vite` | Dev server + build |
| `tailwindcss`, `postcss`, `autoprefixer` | Styling |
| `@vitejs/plugin-react` | Vite React plugin |

## Risks

1. **Wallet-cli is a binary crate, not a library** — `wallet-cli` has `src/main.rs` and its functions are `pub` but the crate is designed as a CLI. The GUI backend may need to refactor shared logic into a library module or duplicate a small amount of code. Mitigation: make `wallet-cli` lib functions callable from Tauri commands; add `wallet-cli` as a path dependency if its public API suffices.

2. **No live node API** — `ChainState::new()` creates an empty in-memory state; `balance` returns 0 for all addresses unless accounts are pre-seeded. `send` validates against this empty state so all sends will fail with insufficient balance until a real node/RPC is connected. Mitigation: MVP is offline signing with signed tx hex output; document clearly that balance/send are offline until RPC integration.

3. **Error serialization across Tauri boundary** — `WalletError` is not `Serialize`. Need a separate `GuiError` enum with string messages that the frontend can display. Mitigation: map each `WalletError` variant to a `GuiError` variant with a user-facing message string.

4. **Passphrase in frontend memory** — The passphrase lives in React state briefly and is sent via Tauri invoke. It is never written to disk by the frontend. Mitigation: clear passphrase state after use; do not log it; Tauri invoke args are not logged by default.

5. **Windows file permissions** — `set_restrictive_permissions` (0600) is a no-op on Windows (`#[cfg(unix)]`). Keystore file on Windows will have default permissions. Mitigation: document that on Windows the keystore relies on OS-level user account protection.

6. **Address format** — `Address::to_hex()` produces `0x`-prefixed hex; `from_hex` accepts with or without prefix. Frontend must consistently use `0x` prefix for display and input parsing. Mitigation: standardize on `0x` prefix in all frontend address fields.

7. **Tauri v2 API changes** — Tauri v2 has different command registration and security defaults than v1. Mitigation: pin Tauri v2, follow v2 docs for `tauri.conf.json` and command allowlists.

8. **Keystore shared between CLI and GUI** — Both tools read/write `~/.atoll/keystore.json`. If both are open simultaneously, file conflicts are possible. Mitigation: document that only one wallet instance should access the keystore at a time; file is small and writes are atomic-ish (write-then-replace not implemented, but acceptable for MVP).

## Timeline

Estimated complexity: **Medium (≈ 5–7 business days)** for MVP.

| Phase | Work | Tasks | Estimate |
|-------|------|-------|----------|
| Scaffold | Tauri project setup, command wrappers, error type | 1–3 | 1.5 days |
| Backend plumbing | Keystore path, passphrase plumbing, command wiring | 4 | 0.5 day |
| Frontend core | Auth views, create/import, shell, navigation | 5, 8 | 1.5 days |
| Frontend transactions | Balance, send, history views | 6, 7 | 1.5 days |
| Tests + verification | Unit tests, render tests, clippy, fmt, frontend build | 9, 10 | 1.5 days |

Total: ~6.5 business days (round to **1 work week**).

## Next Step
Run `/build-feature` against the implementation issue once created from this plan, then `/review-code`.