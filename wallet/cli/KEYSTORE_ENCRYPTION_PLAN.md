# Keystore Encryption at Rest — CLI Wallet (H1)

**Project:** Atoll Protocol  
**Feature branch:** `feature/wallet-keystore-encryption`  
**Addressing:** PR #7 code review finding H1 — plaintext secret keys stored in `~/.atoll/keystore.json`  
**Status:** Plan only — no implementation code written yet

---

## Current state

`wallet/cli/src/keystore.rs` persists the keystore as a plaintext JSON map of `address (hex) -> secret key (hex)` at `~/.atoll/keystore.json`. `Keystore::load` reads and deserializes the file; `Keystore::save` writes it with default file permissions. Secret key bytes live in memory as `HashMap<Address, Vec<u8>>`.

`wallet/cli/src/lib.rs` exposes five subcommands: `New`, `Import`, `Balance`, `Send`, `History`. Only `New`, `Import`, and `Send` currently touch the keystore; `Balance` and `History` are read-only chain queries that do not load it.

## Dependencies check

| Crate | Needed for | Status |
|-------|-----------|--------|
| `argon2 = "0.5"` | Passphrase → key derivation (argon2id) | Already a direct dependency of `atoll-core` (`core/Cargo.toml:29`) |
| `chacha20poly1305` | XChaCha20-Poly1305 AEAD encryption | NOT a direct dependency of `wallet-cli`. Present only transitively in `Cargo.lock` (v0.10.1, via `snow` → `core`). Must be added to `wallet/cli/Cargo.toml`. |
| `rand = "0.8"` | Random salt + nonce generation | Already a direct dependency of `wallet-cli` |
| `std::fs::set_permissions` | 0600 file permissions | Standard library, no dependency |

**Decision:** Add `chacha20poly1305 = "0.10"` to `wallet-cli/Cargo.toml` (lock already resolves to 0.10.1, so no network churn beyond adding the direct dep). Re-export/use `argon2` through `atoll-core` rather than adding a second direct copy.

---

## Approach

### 1. On-disk format
The keystore JSON envelope becomes:

```json
{
  "salt": "<hex-encoded 16-byte random salt>",
  "nonce": "<hex-encoded 24-byte XChaCha20 nonce>",
  "ciphertext": "<hex-encoded XChaCha20-Poly1305 ciphertext>"
}
```

The ciphertext is the AEAD-encrypted serialization of the current plaintext `HashMap<String, String>` (address hex → secret key hex). Salt and nonce are stored alongside so the same file is self-describing; no sidecar metadata.

### 2. Key derivation
Derive a 32-byte encryption key from the user passphrase using **argon2id** with the random 16-byte salt. Parameters: modest but defensive (e.g. 64 MiB memory, 3 iterations, 1 lane) — chosen to be tolerable on a CLI while raising the cost of offline brute-force. The passphrase is never written to disk.

### 3. Encryption / decryption
- `save()`: serialize keystore → encrypt with XChaCha20-Poly1305 → write envelope JSON → `fs::set_permissions(path, 0o600)`.
- `load()`: read envelope → decrypt with derived key → deserialize keystore map. Wrong passphrase or corrupted ciphertext returns a clear error (authentication tag failure), never silent garbage.

### 4. Passphrase resolution (priority order)
1. `--passphrase` CLI flag
2. `ATOLL_PASSPHRASE` environment variable
3. Interactive terminal prompt (read from stdin; skipped in non-TTY/CI)

A missing passphrase when one is required returns a clear error rather than falling back to plaintext.

### 5. Command wiring
Add `--passphrase` to `new`, `import`, `balance`, `send`, `history` per the requirement. `new`/`import`/`send` thread it into `Keystore` load/save. `balance`/`history` accept the flag for CLI consistency (they do not read the keystore in MVP); this is flagged as an open question below.

### 6. Migration / backward compatibility
No silent fallback to plaintext. An existing unencrypted `keystore.json` is detected (missing envelope fields / fails AEAD decrypt) and produces a clear error instructing the user to re-encrypt (decrypt with old plaintext path, re-save with passphrase). There is no passphrase recovery — losing the passphrase means losing the keys.

---

## Task breakdown (max 8)

| # | Task | Layer | Description |
|---|------|-------|-------------|
| 1 | Add `chacha20poly1305` dependency | Config | Add `chacha20poly1305 = "0.10"` to `wallet-cli/Cargo.toml`; run `cargo check` to verify resolution against the existing lock. |
| 2 | Crypto module: argon2id + XChaCha20-Poly1305 | Crypto | New module deriving a 32-byte key from passphrase+salt (argon2id) and encrypting/decrypting the keystore envelope (salt+nonce+ciphertext). |
| 3 | Encrypted `Keystore::load`/`save` + 0600 perms | Keystore | Refactor `Keystore` to encrypt on save and decrypt on load using the passphrase-derived key; set file permissions to 0600 on write. |
| 4 | Passphrase resolution | CLI | Resolve passphrase from `--passphrase` flag > `ATOLL_PASSPHRASE` env > interactive stdin prompt; add `WalletError::Passphrase` variant. |
| 5 | Wire `--passphrase` into `new`, `import`, `send` | CLI | Add flag to the three keystore-writing/reading subcommands and pass it through to `Keystore`. |
| 6 | Wire `--passphrase` into `balance`, `history` | CLI | Add flag to the two read-only subcommands (accepted for consistency; keystore not accessed in MVP). |
| 7 | Update tests + add encryption tests | Tests | Update existing plaintext `save`/`load` tests; add roundtrip encrypt/decrypt, wrong-passphrase error, missing-passphrase error, and 0600 permission tests. |
| 8 | Plaintext keystore migration handling | Keystore | Detect an unencrypted `keystore.json`, return a clear error with re-encryption instructions; no silent plaintext fallback. |

---

## Risks

| Risk | Mitigation / notes |
|------|--------------------|
| Passphrase loss = irreversible key loss | No key escrow or recovery in MVP; document clearly in CLI help and README. |
| Backward incompatibility with existing plaintext keystore | Task 8 handles detection + clear re-encryption error; no silent downgrade. |
| Non-TTY / CI environments can't prompt interactively | Require `--passphrase` or `ATOLL_PASSPHRASE` in those contexts; error clearly if neither is available. |
| `balance`/`history` don't use the keystore | Adding `--passphrase` to them is UX noise unless they later read the keystore. Open question — consider applying the flag only to `new`/`import`/`send` and revisit if those commands ever need the keystore. |
| Argon2 parameter choice | Too weak → brute-force risk; too strong → poor CLI UX. Pick conservative defaults and document them. |
| Existing tests call plaintext `save()`/`load()` | Task 7 updates them to exercise the encrypted path. |
| Passphrase lingering in process memory | Clear the passphrase `String` after derivation (consider `zeroize`, already a transitive dependency). |

---

## Timeline

Estimated **2–3 business days** (XS–S scope). All tasks are contained within `wallet/cli/` with one new dependency; no changes to `core`, `explorer`, or network layers.

- Day 1: Tasks 1–3 (dependency, crypto module, encrypted Keystore)
- Day 2: Tasks 4–6 (passphrase resolution + CLI wiring)
- Day 3: Tasks 7–8 (tests, migration handling), self-review

---

## Open questions (to resolve before implementation)

1. Should `--passphrase` be added to `balance` and `history`, given they do not read the keystore in MVP? (Task 6 includes it per the requirement, but it may be unnecessary noise.)
2. Interactive passphrase prompt: read from stdin with echo (simple, no new dep) vs. add `rpassword` for hidden input?
3. Argon2id parameters (memory / iterations / lanes) — propose 64 MiB / 3 iterations / 1 lane as a starting point.

---

## Definition of done

- `cargo check`, `cargo build`, `cargo test` pass for `wallet-cli`
- Keystore JSON on disk is never plaintext; file permissions are 0600
- `new`, `import`, `send` require a passphrase (flag, env, or prompt)
- Wrong passphrase produces a clear error, not corrupted keys
- Existing plaintext keystore is rejected with re-encryption instructions
