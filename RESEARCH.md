# Research: CI/CD Best Practices for Cryptocurrency Wallet GUI Desktop Executables (Tauri v2)

**Date:** 2026-11-08  
**Scope:** Building Tauri v2 wallet desktop executables in GitHub Actions pipelines  
**Focus:** Code signing, secure build environments, reproducible builds, binary integrity, Tauri v2 CI, wallet-specific security, release workflows, Windows MSIX/AppX packaging

---

## Executive Summary

A cryptocurrency wallet desktop app is a high-value supply-chain target: anyone who compromises the build pipeline can ship a signed malicious wallet to every user, stealing keys and funds. Controls must therefore treat CI as a production system, not a convenience. This document consolidates current (2025–2026) best practices into eight domains with concrete GitHub Actions patterns.

**Core principles for wallet desktop executables:**
1. **No signing material ever reaches untrusted jobs.** Fork PRs must never read code-signing keys, wallet seed material, or deploy credentials.
2. **Every artifact is signed and verified before upload.** Gate on tag pushes; verify signatures before publishing; never ship silently-unsigned installers.
3. **Builds are deterministic and auditable.** Commit lockfiles, pin toolchains/actions/runners, generate SBOMs, attach SLSA provenance, and ship checksums.
4. **Wallet key material never touches CI.** Seeds/private keys are generated, stored, and used only on the user's device; CI never logs or exports them.

---

## 1. Code Signing Executables in CI

### 1.1 Why it matters
Without a valid code signature:
- **Windows:** SmartScreen shows "Unknown publisher" warnings; Windows 11 Smart App Control blocks unknown unsigned apps entirely (users cannot whitelist a single app).
- **macOS:** Gatekeeper blocks unsigned apps as "damaged"; notarization is required for apps downloaded outside the App Store.
- **Wallet-specific impact:** An unsigned wallet installer looks suspicious to security-conscious users and trains them to ignore warnings — a phishing vector.

### 1.2 Windows code signing

**Recommended approaches (in order of preference):**

| Approach | Description | CI suitability |
|---|---|---|
| **Azure Artifact Signing / Azure Trusted Signing** | Signing key lives in Microsoft's cloud HSM; no `.pfx` on the runner. | Best for CI/CD; no private key material on the runner. |
| **Cloud HSM via SignTool KSP/CSP** (Google Cloud CNG, DigiCert, SSL.com) | Private key stays in HSM; SignTool delegates signing over network. | Good for CI; requires runner-side credential config. |
| **Base64 `.pfx` in GitHub Secret** | Decode secret to `.pfx` under `$env:RUNNER_TEMP`, sign with `signtool`, delete file. | Acceptable for internal/OV certs; the `.pfx` briefly exists on disk. |
| **Hardware token / EV cert** | Physical USB token required. | **Not usable on GitHub-hosted runners.** Requires a self-hosted runner with the token always attached. |

**Key detail (2026):** Publicly trusted code-signing certificates issued since June 1, 2023 must have private keys generated and stored in a FIPS 140-2 Level 2 HSM under CA/Browser Forum requirements. A `.pfx` in a GitHub secret applies to internal CAs and pre-2023 certs only. For publicly trusted certs, call a network signing service or use a hardware token through `signtool` with `/csp` and `/kc`.

**Always timestamp signatures.** Use `/tr http://timestamp.digicert.com` (or `http://timestamp.acs.microsoft.com` for Azure Artifact Signing) with `/td SHA256`. Without timestamping, every signature becomes invalid the day the certificate expires, including installers users already downloaded.

**Verify before upload.** Run `signtool verify /pa /v` after signing; fail the job if verification fails. This prevents shipping unsigned binaries due to misconfigured secrets.

#### GitHub Actions example — Windows Authenticode signing with base64 `.pfx`

```yaml
jobs:
  sign-windows:
    name: Sign Windows artifacts
    runs-on: windows-latest
    environment: release  # secrets scoped to protected environment
    if: startsWith(github.ref, 'refs/tags/v')
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@b4ffde65f46336ab88eb53be80866792576f8620  # pinned SHA
      - name: Resolve signtool.exe
        shell: pwsh
        run: |
          $signtool = (Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe" | Sort-Object FullName)[-1].FullName
          echo "SIGNTOOL=$signtool" >> $env:GITHUB_ENV
      - name: Decode certificate to temp (outside workspace)
        shell: pwsh
        env:
          WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
          WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
        run: |
          $pfxPath = Join-Path $env:RUNNER_TEMP "wallet-signing.pfx"
          [IO.File]::WriteAllBytes($pfxPath, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))
          echo "PFX_PATH=$pfxPath" >> $env:GITHUB_ENV
      - name: Sign executables
        shell: pwsh
        env:
          WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
          PFX_PATH: ${{ env.PFX_PATH }}
        run: |
          $files = Get-ChildItem src-tauri/target/release/bundle/nsis -Filter *.exe -Recurse
          foreach ($f in $files) {
            & $env:SIGNTOOL sign /f $env:PFX_PATH /p $env:WINDOWS_CERTIFICATE_PASSWORD `
              /fd SHA256 /td SHA256 /tr http://timestamp.digicert.com /v $f.FullName
            & $env:SIGNTOOL verify /pa /v $f.FullName
          }
      - name: Clean up certificate
        if: always()
        shell: pwsh
        run: |
          if (Test-Path $env:PFX_PATH) { Remove-Item -Force $env:PFX_PATH }
```

**Trade-off:** Shipping Windows unsigned avoids certificate cost (~$200–$400/yr OV) and CI complexity, but SmartScreen warnings degrade trust. For a wallet, signing is strongly recommended.

### 1.3 macOS code signing + notarization

macOS requires two separate steps:
1. **Code signing** — embeds a Developer ID certificate into the `.app` bundle.
2. **Notarization** — uploads the signed app to Apple's notary service for malware scanning; Apple staples a ticket so Gatekeeper trusts it offline.

**Prerequisites:** Apple Developer account ($99/yr or free for testing), an Apple device to create the certificate, and a `Developer ID Application` certificate (not App Store, for direct distribution).

**GitHub Actions pattern — import certificate into a throwaway keychain** (do this only in the macOS job; never pass Apple credentials to the Windows job):

```yaml
- name: Import Apple Developer certificate (macOS)
  if: matrix.platform == 'macos-latest'
  env:
    APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
    APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
  run: |
    KEYCHAIN_PASSWORD=$(openssl rand -base64 24)
    echo "$APPLE_CERTIFICATE" | base64 --decode > "$RUNNER_TEMP/certificate.p12"
    security create-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
    security default-keychain -s build.keychain
    security unlock-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
    security set-keychain-settings -t 3600 -u build.keychain
    security import "$RUNNER_TEMP/certificate.p12" -k build.keychain \
      -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
    security set-key-partition-list -S apple-tool:,apple:,codesign: \
      -s -k "$KEYCHAIN_PASSWORD" build.keychain
    security find-identity -v -p codesigning build.keychain
    rm "$RUNNER_TEMP/certificate.p12"
```

**Secrets required:** `APPLE_CERTIFICATE` (base64 `.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (app-specific password, **not** the account password), `APPLE_TEAM_ID`. Use App Store Connect API key (`APPLE_API_KEY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_BASE64`) instead of password-based notarization to avoid account lockouts from repeated CI logins.

**tauri.conf.json:**
```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "-",
      "entitlements": "./Entitlements.plist",
      "minimumSystemVersion": "11.0"
    }
  }
}
```
Leave `"-"` (ad-hoc) for local builds; override with `APPLE_SIGNING_IDENTITY` env var in CI for Developer ID signing.

### 1.4 Tauri `signCommand` for Windows (cloud HSM / Azure)

Tauri supports a custom `signCommand` in `tauri.conf.json` > `bundle.windows`. Tauri replaces `%1` with each file path and invokes the command for every binary/installer. This is how Azure Trusted Signing (relic / `artifact-signing-cli`) integrates:

```json
{
  "bundle": {
    "windows": {
      "signCommand": "artifact-signing-cli sign --file %1 --key azure --config relic.conf",
      "webviewInstallMode": { "type": "downloadBootstrapper" },
      "nsis": { "installMode": "both" }
    }
  }
}
```

Use a CI-only overlay config (`tauri.windows-sign.conf.json`) passed via `tauri build --config`, applied only when signing secrets exist. Local and fork builds produce unsigned installers without failing.

### 1.5 Tauri updater signing (critical for wallets)

Tauri's auto-updater signs `latest.json` and `.sig` files with `TAURI_SIGNING_PRIVATE_KEY`. **This key is entirely your responsibility** — no platform vendor can revoke it. If it leaks, an attacker can push malicious updates to every installed wallet.

Pattern: gate updater signing on a tag-triggered job; store the updater private key as a repository/environment secret; rotate after any suspected exposure.

---

## 2. Secure Build Environments for Wallet Apps

### 2.1 GitHub-hosted vs self-hosted runners

| Factor | GitHub-hosted | Self-hosted |
|---|---|---|
| Ephemeral isolation | Yes — fresh VM per job, destroyed after | No — persists state between jobs unless `--ephemeral` |
| Public repo safety | Safe | **Never** use for public repos (any fork PR can compromise it) |
| Hardware token / EV signing | Not possible | Required for EV certs with USB tokens |
| Cloud HSM signing (Azure, DigiCert, Google Cloud) | Works over network | Works over network |
| Maintenance burden | Low | High (patching, hardening, runner groups) |

**Recommendation for a wallet:**
- Use **GitHub-hosted runners** for the standard build/sign matrix when using cloud HSM (Azure Artifact Signing) or base64 `.pfx` secrets.
- Use **self-hosted, ephemeral (JIT) runners** only if you need an EV cert with a hardware token, or if signing keys must never leave a private network. Protect with:
  - `--ephemeral` flag (one job, then destroyed)
  - Dedicated runner groups scoped per repository
  - Network egress allowlisting (registries + signing services only)
  - Separate `trusted` (releases/main) and `untrusted` (PRs) runner labels
  - Runner never as root; dedicated service account
  - No static cloud credentials on the runner — use OIDC federation instead

### 2.2 Protecting secrets in CI

**Never commit certificate files:** `.pfx`, `.p12`, `.key`, or their base64 encodings. Add them to `.gitignore`.

**Environment-scoped secrets over repository-scoped secrets:** A repository-wide secret is readable by any workflow, including one added from a PR branch. An environment-scoped secret (e.g., `release` environment) is only readable by jobs naming that environment, and can be gated behind required reviewers.

**Least-privilege `GITHUB_TOKEN`:**
```yaml
permissions:
  contents: read   # default at workflow level
jobs:
  release:
    permissions:
      contents: write   # only the release job can write releases
```

**Pin all third-party Actions by commit SHA**, not mutable tags. Tags can be retagged to point to malicious code (the `tj-actions/changed-files` supply-chain incident is the canonical example).

```yaml
# Unsafe
- uses: actions/checkout@v4
# Safe
- uses: actions/checkout@b4ffde65f46336ab88eb53be80866792576f8620
```

**Pass secrets via `env:` blocks, never inline interpolation.** A `${{ secrets.X }}` inside a script body is substituted into the script file on disk before execution and can be read back. Mapping to an environment variable keeps it in process memory only.

**Never log secrets.** GitHub masks registered secret values, but structured data (JSON/YAML blobs) defeats redaction. Register transformed values (e.g., JWTs derived from keys) as secrets too.

**Rotate signing certificates and secrets regularly** (and immediately after any suspected exposure). GitHub secret access logs should be audited.

---

## 3. Deterministic / Reproducible Builds

### 3.1 Why it matters
Reproducible builds let anyone verify that a released binary matches the reviewed source — reducing trust in build infrastructure. A compromised build server would be instantly detected because independent rebuilds would differ. **For a wallet, this is a core supply-chain defense.**

Tauri itself does **not** yet guarantee byte-for-byte reproducible output (open issue tauri-apps/tauri#13612). Windows `.exe` artifacts are notably non-deterministic across machines; macOS is more deterministic. You must layer controls on top.

### 3.2 Implementation checklist

| Control | How |
|---|---|
| Lock dependencies | Commit `src-tauri/Cargo.lock` and frontend `pnpm-lock.yaml`/`package-lock.json`; use `cargo build --locked` and `pnpm install --frozen-lockfile` |
| Pin Rust toolchain | `rust-toolchain.toml` with exact version |
| Pin Node.js | Exact version in `actions/setup-node` |
| Pin GitHub Actions | By commit SHA (see §2.2) |
| Pin runner images | Specific OS versions (e.g., `windows-2022`, `ubuntu-22.04`), not `-latest` |
| Normalize paths | `RUSTFLAGS="--remap-path-prefix=$(pwd)=/builddir --remap-path-prefix=$HOME/.cargo=/cargo"` |
| Single codegen unit | `RUSTFLAGS="$RUSTFLAGS -C codegen-units=1"` |
| Remove build timestamps | Set `SOURCE_DATE_EPOCH`; strip `build-id` (`LDFLAGS="-Wl,--build-id=none"`); fix PE header link date |
| Avoid `const-random` | Set `CONST_RANDOM_SEED=0` if the crate graph uses `ahash` |
| Commit lockfiles + preflight guard | Fail CI when `Cargo.lock` is missing or stale |

### 3.3 GitHub Actions example — pinned, reproducible Tauri build matrix

```yaml
jobs:
  build:
    name: Build ${{ matrix.platform }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      max-parallel: 1  # serializes to avoid tauri-action release race
    matrix:
      include:
        - os: windows-latest
          target: x86_64-pc-windows-msvc
          rust_target: x86_64-pc-windows-msvc
        - os: macos-latest
          target: aarch64-apple-darwin
          rust_target: aarch64-apple-darwin
        - os: ubuntu-22.04
          target: x86_64-unknown-linux-gnu
          rust_target: x86_64-unknown-linux-gnu
    steps:
      - uses: actions/checkout@b4ffde65f46336ab88eb53be80866792576f8620
      - uses: actions/setup-node@1902457f250559479b82e3be6317e3844459a52f  # v4 pinned SHA
        with:
          node-version: '22.12.0'
          cache: 'pnpm'
      - uses: dtolnay/rust-toolchain@f7ccc83f9ed1e5b9c81d8a67d7ad1a747e22a561  # pinned SHA
        with:
          toolchain: stable
          targets: ${{ matrix.rust_target }}
      - name: Rust cache
        uses: swatinem/rust-cache@c19371144df3bb44fab255c43d04cbc2ab54d1c4  # v2.9.1 pinned SHA
        with:
          workspaces: 'src-tauri -> target'
          key: ${{ matrix.target }}-${{ hashFiles('src-tauri/Cargo.lock') }}
      - name: Install frontend deps
        run: pnpm install --frozen-lockfile
      - name: Build Tauri
        run: cargo tauri build --target ${{ matrix.rust_target }} --locked
      - name: Verify lockfile unchanged
        run: git diff --exit-code -- src-tauri/Cargo.lock pnpm-lock.yaml
```

**Trade-off:** Full byte-for-byte reproducibility of installers is not yet achievable in Tauri without controlling platform toolchains and signing inputs. Treat reproducibility as an aspiration with incremental guarantees (locked dependency graph, pinned toolchain, verifiable SBOMs), not a binary gate unless you invest in `ducible`-style post-processing for Windows PE files.

---

## 4. Binary Integrity Verification

### 4.1 SBOM generation

Generate an SBOM for every release artifact. Use **SPDX** (standard) and/or **CycloneDX**. Attach SBOMs to the GitHub Release.

**GitHub Actions example — `anchore/sbom-action`:**

```yaml
- name: Generate desktop SBOM (SPDX)
  uses: anchore/sbom-action@e22c389904149dbc22b58101806040fa8d37a610  # v0.24.0 pinned SHA
  with:
    path: src-tauri/target/${{ matrix.target }}/release/bundle
    format: spdx-json
    output-file: ${{ runner.temp }}/wallet-${{ matrix.target }}.spdx.json
    artifact-name: sbom-${{ matrix.target }}
    upload-artifact: false
```

### 4.2 SLSA provenance attestations

Use `actions/attest-build-provenance` (GitHub's Sigstore-backed attestation) so anyone can verify that published binaries were built from the expected source in the expected CI environment:

```yaml
- name: Attest build provenance and SBOM
  uses: actions/attest@f7c74d28b9d84cb8768d0b8ca14a4bac6ef463e6  # v4 pinned SHA
  with:
    subject-path: src-tauri/target/${{ matrix.target }}/release/bundle/**/*
    sbom-path: ${{ runner.temp }}/wallet-${{ matrix.target }}.spdx.json
```

Verify locally:
```bash
gh attestation verify <artifact-file> --repo owner/repo
```

**Note:** `actions/attest-build-provenance` is not available on Free/Pro/Team private repos or GHES; use `cosign` (keyless Sigstore via OIDC) instead.

### 4.3 Checksums and artifact signing

Generate `SHA256SUMS.txt` for every release artifact and sign it:

```yaml
- name: Generate checksums
  working-directory: dist
  run: sha256sum * > SHA256SUMS.txt

- name: Sign checksums (keyless, Sigstore)
  uses: sigstore/cosign-installer@v3  # pin SHA in production
  with: ...
  run: |
    cosign sign-blob --yes \
      --output-signature SHA256SUMS.txt.sig \
      --output-certificate SHA256SUMS.txt.pem \
      SHA256SUMS.txt
```

Users verify:
```bash
sha256sum -c SHA256SUMS.txt --ignore-missing
cosign verify-blob --cert SHA256SUMS.txt.pem --signature SHA256SUMS.txt.sig SHA256SUMS.txt
```

### 4.4 Verify platform signatures before publishing

Before signing the hash manifest, verify each artifact carries its expected platform signature:
- Windows: `signtool verify /pa /v` on every `.exe`/`.msi`/`.msix`
- macOS: `codesign --verify --deep --strict` + `spctl -a -t exec -vv` (Gatekeeper + notarization check)
- Linux: GPG signature on AppImage/deb/rpm

A release with even one unsigned artifact is refused. The manifest attests bytes, not publisher identity — signing a manifest over unsigned installers would verify perfectly forever and nobody could detect it later.

---

## 5. Tauri v2 Specific CI Best Practices

### 5.1 Workflow structure

Use `tauri-apps/tauri-action` for the build/bundle/release step. Matrix across platforms with `fail-fast: false` so one platform failure doesn't cancel others (especially important when macOS notarization is queued in Apple's system).

**Trigger:** Push a version tag (`v*`). Tag-based releases are cleaner than branch-based heuristics.

```yaml
on:
  push:
    tags: ['v*']
```

### 5.2 Platform-specific notes

- **macOS:** Universal binary (`universal-apple-darwin`) runs on both Apple Silicon and Intel but doubles file size. Separate `aarch64` and `x86_64` DMGs keep downloads smaller. macOS-13 Intel runner was retired by GitHub on 2024-12-04; use `macos-latest` with universal builds or separate arch jobs.
- **Windows:** WebView2 is required. Use `webviewInstallMode: { type: "downloadBootstrapper" }` to keep the installer small (downloads WebView2 on demand). Tauri's default bundling produces NSIS + MSI.
- **Linux:** Requires `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`, `xdg-utils`. AppImage via linuxdeploy is unreliable under Tauri 2 (FUSE issues); ship `.deb` primarily.

### 5.3 Caching

Cache three things, keyed so they refresh exactly when dependencies change:

1. Cargo registry + git checkouts
2. `src-tauri/target` keyed on `src-tauri/Cargo.lock`
3. Frontend package-manager store keyed on `pnpm-lock.yaml`/`package-lock.json`

```yaml
- name: Rust cache
  uses: swatinem/rust-cache@c19371144df3bb44fab255c43d04cbc2ab54d1c4
  with:
    workspaces: 'src-tauri -> target'
    key: ${{ runner.os }}-cargo-${{ hashFiles('src-tauri/Cargo.lock') }}
```

**Windows cache caveat:** WarpBuild Cache is not supported on Windows runners; use `actions/cache` on the Windows leg.

### 5.4 tauri-action parameters

| Parameter | Purpose |
|---|---|
| `tagName` | `v__VERSION__` — `__VERSION__` replaced with version from `tauri.conf.json` |
| `releaseDraft: true` | Create draft release; review before publishing |
| `prerelease: false` | Mark as prerelease for betas/RCs |
| `args` | Passed to `tauri build` (e.g., `--target aarch64-apple-darwin`) |
| `projectPath` | Path to Tauri project (for monorepos) |

**Race condition:** `tauri-action`'s find-or-create-release can race across parallel matrix jobs, creating duplicate releases. Use `max-parallel: 1` for the release job or serialize publishing behind a separate job that runs after all builds succeed.

### 5.5 Updater artifacts

Enable `createUpdaterArtifacts` in tauri.conf.json to generate `.sig` files and `latest.json` for auto-updates. Sign these with `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

---

## 6. Cryptocurrency Wallet Specific Security Concerns

### 6.1 Never commit or log key material

- **Never commit** private keys, seed phrases/mnemonics, wallet JSON files, or API keys to version control.
- **Never log** keys or seeds. Crash dumps, error messages, and debug logs must exclude secret material.
- **Never expose** keys in frontend bundles or preload scripts.
- CI must never generate, store, or export user seed phrases. Keys are generated, stored, and used **only on the user's device**.

### 6.2 Wallet threat model (build/release relevant)

| Asset | Consequence of compromise | CI control |
|---|---|---|
| BIP39 seed phrase | Full loss of all derived funds | Never in CI; device-only |
| Vault master key | Decrypts vault and all wallet records | Never in CI |
| Code-signing key | Malware signed as your wallet publisher | Environment-scoped secrets, HSM, tag-gated |
| Tauri updater key | Malicious push updates to every install | Tag-gated, rotate on exposure, offline backup |
| Release signing key | Tampered artifacts distributed as official | SLSA provenance, SBOMs, checksums |

### 6.3 Dependency auditing

Wallets pull hundreds of crates through `tauri`/`wry` and hundreds of npm packages through the frontend. A compromised dependency in the build step can read signing credentials held by that step.

**Required gates:**
- `cargo audit` / `cargo-deny` on `Cargo.lock` (fail on any vulnerability)
- `pnpm audit --prod --audit-level=high` on frontend deps
- GitHub Dependabot + Dependency Review on PRs
- Gitleaks / secret scanning to catch accidentally committed keys
- CodeQL for static analysis
- Narrow credential scope per build step: macOS job gets only Apple cert, Windows job gets only Azure/Windows cert, Linux job gets only GPG key. Every job gets the updater key (because all platforms sign updater artifacts).

### 6.4 Key isolation at runtime (defense-in-depth)

While not a CI concern per se, CI-built binaries should enforce:
- Sandboxed renderer, strict CSP, `contextIsolation`, `nodeIntegration: false`
- Key operations in an isolated signer process (not the renderer); zeroize secret buffers after signing
- OS keychain wrap (DPAPI on Windows, Keychain on macOS, libsecret on Linux) for at-rest key encryption
- Argon2id-derived password KEK + AES-256-GCM vault encryption
- Pinned egress allowlist for network hosts

---

## 7. GitHub Actions Workflow Patterns for Desktop App Releases

### 7.1 Release workflow anatomy

1. **Trigger:** `push` tags matching `v*`
2. **Matrix build:** one job per platform (macOS ARM64, macOS x64, Windows x64, Linux x64), `fail-fast: false`
3. **Sign:** platform-specific signing (Windows Authenticode, macOS Developer ID + notarization)
4. **Verify:** signature verification before any upload
5. **SBOM + provenance:** `anchore/sbom-action` + `actions/attest-build-provenance`
6. **Checksums:** `SHA256SUMS.txt` + optional Cosign/keyless signature
7. **Draft release:** `tauri-action` creates a draft GitHub Release; a maintainer publishes manually
8. **Attest tag:** release tag signed with maintainer's offline key; CI verifies before any secret-bearing job runs

### 7.2 Environment gating

Use a `release` GitHub environment with required reviewers for production deployments. Secrets in that environment are only readable after approval — an unreviewed branch/tag cannot reach the signing key.

```yaml
environment: release
```

### 7.3 Complete example workflow (wallet Tauri v2)

```yaml
name: Release

on:
  push:
    tags: ['v*']
  workflow_dispatch:

permissions:
  contents: read
  id-token: write   # for OIDC / Sigstore keyless signing

jobs:
  build-and-sign:
    name: Build and sign ${{ matrix.platform }}
    runs-on: ${{ matrix.os }}
    environment: release
    strategy:
      fail-fast: false
      max-parallel: 1
    matrix:
      include:
        - os: windows-latest
          platform: windows
          target: x86_64-pc-windows-msvc
        - os: macos-latest
          platform: macos
          target: aarch64-apple-darwin
        - os: ubuntu-22.04
          platform: linux
          target: x86_64-unknown-linux-gnu

    steps:
      - uses: actions/checkout@b4ffde65f46336ab88eb53be80866792576f8620

      - uses: actions/setup-node@1902457f250559479b82e3be6317e3844459a52f
        with:
          node-version: '22.12.0'
          cache: 'pnpm'

      - uses: dtolnay/rust-toolchain@f7ccc83f9ed1e5b9c81d8a67d7ad1a747e22a561
        with:
          toolchain: stable
          targets: ${{ matrix.target }}

      - name: Rust cache
        uses: swatinem/rust-cache@c19371144fab255c43d04cbc2ab54d1c4
        with:
          workspaces: 'src-tauri -> target'
          key: ${{ matrix.platform }}-cargo-${{ hashFiles('src-tauri/Cargo.lock') }}

      - name: Install frontend deps
        run: pnpm install --frozen-lockfile

      # ---- Platform-specific signing setup ----
      - name: Import Windows certificate
        if: matrix.platform == 'windows'
        shell: pwsh
        env:
          WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
          WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
        run: |
          $pfx = Join-Path $env:RUNNER_TEMP "wallet.pfx"
          [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))
          echo "PFX_PATH=$pfx" >> $env:GITHUB_ENV

      - name: Import Apple certificate
        if: matrix.platform == 'macos'
        env:
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
        run: |
          KEYCHAIN_PASSWORD=$(openssl rand -base64 24)
          echo "$APPLE_CERTIFICATE" \| base64 --decode > "$RUNNER_TEMP/cert.p12"
          security create-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
          security default-keychain -s build.keychain
          security unlock-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
          security import "$RUNNER_TEMP/cert.p12" -k build.keychain -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
          security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" build.keychain
          rm "$RUNNER_TEMP/cert.p12"

      # ---- Build ----
      - name: Build Tauri bundle
        uses: tauri-apps/tauri-action@v1  # pin SHA in production
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: v__VERSION__
          releaseName: Wallet v__VERSION__
          releaseDraft: true
          prerelease: false
          args: --target ${{ matrix.target }}

      # ---- SBOM ----
      - name: Generate SBOM
        uses: anchore/sbom-action@e22c389904149dbc22b58101806040fa8d37a610
        with:
          path: src-tauri/target/${{ matrix.target }}/release/bundle
          format: spdx-json
          output-file: ${{ runner.temp }}/wallet-${{ matrix.platform }}.spdx.json
          upload-artifact: false

      # ---- Provenance ----
      - name: Attest build provenance
        uses: actions/attest@f7c74d28b9d84cb8768d0b8ca14a4bac6ef463e6
        with:
          subject-path: src-tauri/target/${{ matrix.target }}/release/bundle/**/*
          sbom-path: ${{ runner.temp }}/wallet-${{ matrix.platform }}.spdx.json

      # ---- Checksums ----
      - name: Generate checksums
        working-directory: src-tauri/target/${{ matrix.target }}/release/bundle
        run: sha256sum * > SHA256SUMS-${{ matrix.platform }}.txt

      # ---- Upload artifacts ----
      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02
        with:
          name: wallet-${{ matrix.platform }}
          path: |
            src-tauri/target/${{ matrix.target }}/release/bundle/**
            ${{ runner.temp }}/wallet-${{ matrix.platform }}.spdx.json
          retention-days: 90
```

### 7.4 Release asset naming

Use `artifactName` in `tauri.conf.json` or `tauri-action` `with` to produce predictable names like `Wallet-1.2.3-windows-x64-setup.exe`. Avoid reliance on default names that may change between Tauri versions.

---

## 8. Windows MSIX / AppX Packaging Considerations

### 8.1 MSIX vs NSIS/MSI

| Format | Use case | Signing | Store |
|---|---|---|---|
| **NSIS installer** | Standard direct download (Tauri default) | Authenticode (`signtool`) | No |
| **MSI** | Enterprise direct download | Authenticode | No |
| **MSIX / MSIXBundle** | Microsoft Store, winget, modern Windows app model | Must be signed (Store re-signs) | Yes (Partner Center) |
| **AppX** | Legacy UWP; superseded by MSIX | Must be signed | Yes |

**MSIX benefits for wallets:** Package identity (access to Windows APIs like Notifications, Security, AI APIs), clean install/uninstall, delta updates, Store distribution.

**MSIX drawbacks:** Must be signed (even for sideloading); publisher in certificate must match `Package.appxmanifest`; Windows version must be higher to update an installed package; requires Windows SDK / `MakeAppx.exe` / `msixbundle-cli`.

### 8.2 Tauri MSIX tooling

- **`@choochmeque/tauri-windows-bundle`** — npm package that generates Store-ready MSIX bundles from Tauri, supports multiarch (x64 + arm64), PFX certs and Windows certificate store signing, Windows extensions (file associations, protocol handlers, toast activation).
- **Microsoft `winapp` CLI** — `winapp init` creates `Package.appxmanifest` and assets; `winapp pack .\dist --cert .\devcert.pfx` packages and signs as MSIX.
- **Custom PowerShell script** (e.g., `packaging/msix/build-msix.ps1`) — generates `AppxManifest.xml`, copies binary + assets, runs `MakeAppx.exe`.

### 8.3 GitHub Actions MSIX example

```yaml
jobs:
  msix:
    name: Build Windows MSIX
    runs-on: windows-latest
    environment: release
    if: startsWith(github.ref, 'refs/tags/v')
    steps:
      - uses: actions/checkout@b4ffde65f46336ab88eb53be80866792576f8620
      - uses: actions/setup-node@1902457f250559479b82e3be6317e3844459a52f
        with:
          node-version: '20'
          cache: 'pnpm'
      - uses: dtolnay/rust-toolchain@f7ccc83f9ed1e5b9c81d8a67d7ad1a747e22a561
        with:
          toolchain: stable
          targets: x86_64-pc-windows-msvc,aarch64-pc-windows-msvc
      - name: Cache Rust
        uses: actions/cache@v4
        with:
          path: |
            ~\.cargo\registry
            ~\.cargo\git
            src-tauri\target
          key: windows-tauri-${{ hashFiles('src-tauri/Cargo.lock') }}
      - run: pnpm install --frozen-lockfile
      - run: cargo tauri build --no-bundle
      # Decode PFX to temp (outside workspace)
      - name: Decode signing certificate
        shell: pwsh
        env:
          MSIX_CERTIFICATE_BASE64: ${{ secrets.MSIX_CERTIFICATE_BASE64 }}
        run: |
          [IO.File]::WriteAllBytes(
            "$env:RUNNER_TEMP\wallet-msix.pfx",
            [Convert]::FromBase64String($env:MSIX_CERTIFICATE_BASE64)
          )
      # Build + sign MSIX
      - name: Build and sign MSIX
        shell: pwsh
        env:
          MSIX_CERTIFICATE_PASSWORD: ${{ secrets.MSIX_CERTIFICATE_PASSWORD }}
          MSIX_PUBLISHER: ${{ secrets.MSIX_PUBLISHER }}
        run: |
          ./packaging/msix/build-msix.ps1 `
            -Executable 'src-tauri/target/release/wallet.exe' `
            -Version '1.2.3.0' `
            -Publisher $env:MSIX_PUBLISHER `
            -CertificatePath "$env:RUNNER_TEMP\wallet-msix.pfx" `
            -CertificatePassword $env:MSIX_CERTIFICATE_PASSWORD `
            -OutputPath 'Wallet-Windows-x64.msix'
      - name: Clean up certificate
        if: always()
        shell: pwsh
        run: Remove-Item -Force "$env:RUNNER_TEMP\wallet-msix.pfx" -ErrorAction SilentlyContinue
      - uses: actions/upload-artifact@v4
        with:
          name: msix-bundle
          path: '*.msix'
          retention-days: 90
```

**Microsoft Store flow:** Build MSIX unsigned (Store re-signs with Partner Center identity), upload via `msstore` CLI, gate behind `STORE_AUTOPUBLISH` variable and manual dispatch. Keep Store MSIX workflow separate from direct-download NSIS/MSI workflow to avoid toolchain/output collisions.

---

## Consolidated Recommendations Checklist

### Immediate (MVP)
- [ ] Pin all Actions by commit SHA; pin runner images (not `-latest`)
- [ ] Commit `Cargo.lock` and frontend lockfile; use `--locked` / `--frozen-lockfile`
- [ ] Matrix build (macOS, Windows, Linux) with `fail-fast: false`
- [ ] Windows Authenticode signing with timestamping + `signtool verify` before upload
- [ ] macOS Developer ID signing + notarization + staple (throwaway keychain per job)
- [ ] Tauri updarter key (`TAURI_SIGNING_PRIVATE_KEY`) tag-gated, never in PR jobs
- [ ] SBOM (`anchore/sbom-action`) + SLSA provenance (`actions/attest`) per platform
- [ ] `SHA256SUMS.txt` per release; sign with Cosign keyless (OIDC) or GPG
- [ ] Draft release; manual publish by maintainer
- [ ] Environment-scoped secrets (`release` env) with required reviewers
- [ ] `cargo audit`, `pnpm audit`, secret scanning, CodeQL on every PR

### Hardening (post-MVP)
- [ ] Reproducible build guards: `SOURCE_DATE_EPOCH`, single codegen unit, path remapping, diffoscope comparison on PRs
- [ ] Self-hosted ephemeral runners only if EV cert/hardware token or private-network signing required
- [ ] MSIX packaging for Store/winget distribution (separate workflow)
- [ ] Offline tag-signing key verification before any release job runs
- [ ] Dependency-abort: fail CI on any vulnerability in wallet-critical dependency paths

---

## Sources

| # | Source | URL |
|---|--------|-----|
| 1 | Tauri v2 — GitHub Actions pipelines (official docs) | https://v2.tauri.app/distribute/pipelines/github/ |
| 2 | Tauri v2 — Windows Code Signing (official docs) | https://v2.tauri.app/distribute/sign/windows/ |
| 3 | Tauri v2 — macOS Code Signing (official docs) | https://v2.tauri.app/distribute/sign/macos/ |
| 4 | Urhoba — Signing and Releasing Tauri Apps with GitHub Actions (2026-09) | https://www.urhoba.net/en/post/github-actions-cicd-signing-and-releasing-tauri-applications |
| 5 | ytyng — Build, sign, and notarize a Tauri app on GitHub Actions (2026-07) | https://www.ytyng.com/en/blog/tauri-github-actions-macos-sign-notarize |
| 6 | DEV — Ship Your Tauri v2 App Like a Pro (Parts 1 & 2, 2026-02) | https://dev.to/tomtomdu73/ |
| 7 | XChain Developers — Release CI signing lane protection | https://docs.xchain.io/components/wallet/release/ci-setup |
| 8 | XChain Developers — Wallet Security & Threat Model | https://docs.xchain.io/components/wallet/security |
| 9 | CertiK — Crypto Wallet Security Assessment Checklist | https://www.certik.com/blog/cryptowalletsecurityassessmentchecklist |
| 10 | Open Wallet Standard — Key Isolation | https://github.com/open-wallet-standard/core/blob/main/docs/05-key-isolation.md |
| 11 | ACM — Security Principles for Designing an Unhackable Crypto Wallet (2025-07) | https://cacm.acm.org/blogcacm/security-principles-for-designing-an-unhackable-crypto-wallet/ |
| 12 | WarpBuild — Windows Code Signing in GitHub Actions | https://www.warpbuild.com/guides/windows-code-signing-github-actions |
| 13 | WarpBuild — Tauri Desktop Builds on GitHub Actions | https://www.warpbuild.com/solutions/tauri-github-actions |
| 14 | DigiCert — Binary signing using GitHub Actions | https://docs.digicert.com/en/software-trust-manager/ci-cd-integrations-and-deployment-pipelines/plugins/github/binary-signing-using-github-actions |
| 15 | Hanselman — Azure Trusted Signing with GitHub Actions (2025-11) | https://www.hanselman.com/blog/automatically-signing-a-windows-exe-with-azure-trusted-signing-dotnet-sign-and-github-actions |
| 16 | Microsoft Learn — winapp CLI with Tauri | https://learn.microsoft.com/en-us/windows/apps/dev-tools/winapp-cli/guides/tauri |
| 17 | Choochmeque — tauri-windows-bundle (MSIX packaging) | https://github.com/Choochmeque/tauri-windows-bundle |
| 18 | Refract MC — build-msix.yml example | https://github.com/RefractMC/Refract_MC/blob/main/.github/workflows/build-msix.yml |
| 19 | anchore/sbom-action | https://github.com/anchore/sbom-action |
| 20 | GitHub Docs — Secure use of GitHub Actions / self-hosted runners hardening | https://docs.github.com/en/actions/reference/security/secure-use |
| 21 | GitHub Docs — Compromised runners | https://docs.github.com/en/actions/concepts/security/compromised-runners |
| 22 | GitHub Docs — Implement supply chain best practices / securing builds | https://docs.github.com/en/code-security/tutorials/implement-supply-chain-best-practices/securing-builds |
| 23 | Tauri issue #13612 — Reproducible builds | https://github.com/tauri-apps/tauri/issues/13612 |
| 24 | Tauri issue #13430 — Windows building not deterministic | https://github.com/tauri-apps/tauri/issues/13430 |
| 25 | reproducible-builds.org — Rust | https://reproducible-builds.org/docs/rust/ |
| 26 | DumbMonit — Reproducible builds | https://dumbmonit.app/docs/development/reproducible-builds/ |
| 27 | Hermes IDE — BUILDING.md (reproducibility + SLSA) | https://github.com/hermes-hq/hermes-ide/blob/main/BUILDING.md |
| 28 | Constellation — release-candidate.yml (SBOM + attest example) | https://github.com/ZShamsi987/Constellation/blob/main/.github/workflows/release-candidate.yml |
| 29 | Tauri action repository | https://github.com/tauri-apps/tauri-action |
| 30 | CSA Research Note — JINX-0164 CI/CD developer targeting (2026-05) | https://labs.cloudsecurityalliance.org/research/csa-research-note-jinx0164-cicd-developer-targeting-20260529/ |
