# KeyCortex — Team Handoff & Knowledge Transfer Document

> **Version:** 1.0 · **Date:** 2026-02-27
> **From:** KeyCortex Core Team
> **To:** Integration, QA, DevOps, and Partner Engineering Teams
> **Repository:** `github.com:veeringman/KeyCortex.git`
> **Debug/Integration Repo:** `github.com:veeringman/fd_demo_integ.git` → `keycortex/`

---

## Table of Contents

1. [What is KeyCortex](#1-what-is-keycortex)
2. [Architecture Overview](#2-architecture-overview)
3. [Repository Structure](#3-repository-structure)
4. [Getting Started (Two Paths)](#4-getting-started-two-paths)
5. [API Surface (24 Endpoints)](#5-api-surface-24-endpoints)
6. [Transactional Flows](#6-transactional-flows)
7. [Security Model — Key Protection](#7-security-model--key-protection)
8. [Frontend Options (JS + WASM)](#8-frontend-options-js--wasm)
9. [Database & Storage](#9-database--storage)
10. [Configuration Reference](#10-configuration-reference)
11. [Watchdog & Integration Debugging](#11-watchdog--integration-debugging)
12. [Scripts Inventory](#12-scripts-inventory)
13. [Docker Stack](#13-docker-stack)
14. [Integration Points (External Systems)](#14-integration-points-external-systems)
15. [Known Gaps & Roadmap](#15-known-gaps--roadmap)
16. [Troubleshooting Checklist](#16-troubleshooting-checklist)
17. [Contacts & Resources](#17-contacts--resources)

---

## 1. What is KeyCortex

KeyCortex is a **Rust-first digital wallet and cryptographic signing engine** for the FlowCortex L1 blockchain. It provides:

- **Ed25519 wallet creation** with encrypted key storage
- **Server-side key custody** — private keys never leave the backend
- **Challenge/response auth** with AuthBuddy IdP integration
- **Transaction signing & submission** to FlowCortex L1
- **Integration APIs** for FortressDigital (risk scoring) and ProofCortex (ZKP commitments)
- **Two browser frontends** — JS baseline + Pure Rust/WASM (feature parity)

### MVP Constraints

- **Chain:** `flowcortex-l1` only
- **Assets:** `PROOF`, `FloweR`
- **Signing:** Ed25519 (primary), secp256k1 (optional feature flag)

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                         Browser                                      │
│  ┌─────────────────┐    ┌─────────────────┐                         │
│  │  JS Baseline    │    │  WASM Frontend   │                         │
│  │  (app.js)       │    │  (Rust→WebAssembly)                       │
│  │  Port 8090      │    │  Port 8091       │                         │
│  └────────┬────────┘    └────────┬─────────┘                         │
│           │      HTTP/JSON       │                                   │
└───────────┼──────────────────────┼───────────────────────────────────┘
            │                      │
            ▼                      ▼
┌──────────────────────────────────────────────────────────────────────┐
│  wallet-service (Axum, Rust)  ─  Port 8080                          │
│                                                                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐               │
│  │ kc-crypto│ │kc-storage│ │kc-auth-  │ │kc-chain- │               │
│  │ Ed25519  │ │ RocksDB  │ │ adapter  │ │flowcortex│               │
│  │ encrypt  │ │ keystore │ │ JWT/JWKS │ │ L1 chain │               │
│  │ zeroize  │ │          │ │          │ │          │               │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘               │
│                    │                                                 │
│              ┌─────┴─────┐                                           │
│              │ PostgreSQL │  (optional dual-write)                    │
│              │ Port 5432  │                                           │
│              └────────────┘                                           │
└──────────────────────────────────────────────────────────────────────┘
```

**Key principle:** Both frontends are **thin clients**. All cryptographic operations, key storage, and signing happen server-side. The UI only sends HTTP requests and renders results.

---

## 3. Repository Structure

```
KeyCortex/
├── Cargo.toml                    # Workspace root (9 crates)
├── Dockerfile                    # Multi-stage build (API + WASM)
├── Dockerfile.watchdog           # Monitoring container
├── docker-compose.yml            # Full stack (5 services)
├── .env.example                  # Environment template
│
├── crates/                       # Shared Rust libraries
│   ├── kc-api-types/             #   Request/response DTOs
│   ├── kc-auth-adapter/          #   Auth abstraction
│   ├── kc-chain-client/          #   ChainAdapter trait
│   ├── kc-chain-flowcortex/      #   FlowCortex L1 adapter
│   ├── kc-crypto/                #   Ed25519, encryption, zeroize
│   ├── kc-storage/               #   RocksDB keystore + records
│   └── kc-wallet-core/           #   Wallet domain logic
│
├── services/
│   └── wallet-service/           # Axum HTTP service (24 endpoints)
│       └── src/
│           ├── main.rs           # Entry point, config, routes, handlers
│           ├── auth.rs           # Challenge/verify/bind, JWT parsing
│           ├── submit.rs         # Tx submission, nonce, idempotency
│           ├── ops.rs            # Ops endpoints (bindings, audit)
│           ├── db.rs             # PostgresRepository
│           ├── chain_config.rs   # FlowCortex L1 metadata
│           ├── fortressdigital.rs# FortressDigital integration
│           └── proofcortex.rs    # ProofCortex ZKP commitment
│
├── ui/
│   ├── wallet-baseline/          # JS frontend (no build step)
│   │   ├── index.html
│   │   ├── app.js                # 1035 lines, all UI + API calls
│   │   ├── styles.css            # 1736 lines, shared with WASM
│   │   └── themes.json           # 5 skins (default, ocean, forest, sunset, midnight)
│   └── wallet-wasm/              # Pure Rust/WASM frontend
│       ├── Cargo.toml            # wasm-bindgen, web-sys, gloo-*
│       ├── index.html            # Loads ./pkg/wallet_wasm.js
│       └── src/                  # 12 Rust modules, 2200+ lines
│
├── migrations/
│   └── postgres/
│       └── 0001_init.sql         # wallet_bindings, challenge_store, verification_logs
│
├── scripts/
│   ├── setup_baremetal.sh        # One-command bare-metal setup
│   ├── setup_docker.sh           # One-command Docker setup
│   ├── watchdog.sh               # Transactional flow monitor
│   ├── build_wasm.sh             # WASM frontend build helper
│   ├── smoke_db_fallback.sh      # Health/readiness smoke tests
│   ├── run_wallet_service_cached.sh
│   └── cross_build_windows.sh
│
├── deploy/
│   ├── nginx-keycortex.conf      # Full nginx reverse proxy config
│   └── nginx-wasm.conf           # WASM-specific nginx config
│
└── data/
    └── keystore/rocksdb/         # RocksDB data (encrypted keys)
```

---

## 4. Getting Started (Two Paths)

### Path A: Bare-Metal (Ubuntu 20.04+)

```bash
git clone https://github.com/veeringman/KeyCortex.git
cd KeyCortex
chmod +x scripts/setup_baremetal.sh
./scripts/setup_baremetal.sh
```

This single script:
1. Installs all system deps (clang, llvm, libclang-dev, libssl-dev, etc.)
2. Installs/verifies Rust ≥1.85, wasm-pack, wasm32 target
3. Builds wallet-service (release, ~5-15 min first time)
4. Builds WASM frontend
5. Creates `.env` config, data directories
6. Optionally sets up PostgreSQL
7. Starts API (:8080) + JS UI (:8090) + WASM UI (:8091) + watchdog
8. Runs smoke tests

**Options:**
```bash
./scripts/setup_baremetal.sh --with-postgres      # Include Postgres
./scripts/setup_baremetal.sh --no-postgres         # RocksDB only
./scripts/setup_baremetal.sh --install-systemd     # Production systemd unit
./scripts/setup_baremetal.sh --dev                 # Debug build (faster)
```

### Path B: Docker

```bash
git clone https://github.com/veeringman/KeyCortex.git
cd KeyCortex
chmod +x scripts/setup_docker.sh
./scripts/setup_docker.sh
```

This single script:
1. Checks/installs Docker + Docker Compose
2. Builds multi-stage Docker image (Rust build + WASM build → slim runtime)
3. Starts 5-service stack via docker-compose
4. Runs smoke tests

**Options:**
```bash
./scripts/setup_docker.sh --no-postgres   # Without Postgres
./scripts/setup_docker.sh --rebuild       # Force rebuild (no cache)
./scripts/setup_docker.sh --down          # Stop everything
```

**Or use Docker Compose directly:**
```bash
docker compose up -d                          # Core (API + UIs)
docker compose --profile postgres up -d       # + Postgres
docker compose --profile full up -d           # Everything + watchdog
```

### After Setup: Verify

```bash
curl -s http://localhost:8080/health | jq .       # API health
curl -s http://localhost:8080/version | jq .      # Version
open http://localhost:8090                         # JS frontend
open http://localhost:8091                         # WASM frontend
```

---

## 5. API Surface (24 Endpoints)

### Health & Diagnostics

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Full health with storage, auth, JWKS, fallback counters |
| GET | `/readyz` | Readiness probe (keystore + auth ready) |
| GET | `/startupz` | Startup diagnostics (migration status, JWKS) |
| GET | `/version` | Service name + version |

### Wallet Operations

| Method | Path | Description |
|--------|------|-------------|
| POST | `/wallet/create` | Create new Ed25519 wallet (returns address + public key) |
| GET | `/wallet/list` | List all wallets (address, label, chain) |
| POST | `/wallet/restore` | Restore wallet from passphrase |
| POST | `/wallet/rename` | Rename wallet label |
| POST | `/wallet/sign` | Sign payload (purpose: `auth`, `tx`, `commitment`) |
| GET | `/wallet/balance` | Query wallet balance (asset, chain) |

### Transaction Submission

| Method | Path | Description |
|--------|------|-------------|
| GET | `/wallet/nonce` | Get next nonce for wallet |
| POST | `/wallet/submit` | Submit signed transaction (requires `Idempotency-Key` header) |
| GET | `/wallet/tx/{tx_hash}` | Get transaction status |

### Authentication (AuthBuddy)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/auth/challenge` | Generate UUID challenge (5-min expiry) |
| POST | `/auth/verify` | Verify Ed25519 signature against challenge |
| POST | `/auth/bind` | Bind wallet to IdP user (requires JWT Bearer token) |

### Operations

| Method | Path | Description |
|--------|------|-------------|
| GET | `/ops/bindings/{wallet_address}` | Lookup wallet binding |
| GET | `/ops/audit` | List audit events (filterable) |

### Integration APIs

| Method | Path | Description |
|--------|------|-------------|
| POST | `/fortressdigital/context` | Generate FortressDigital context payload |
| POST | `/fortressdigital/wallet-status` | Wallet verification status for risk scoring |
| POST | `/proofcortex/commitment` | Generate ZKP commitment hash |
| GET | `/chain/config` | FlowCortex L1 chain configuration metadata |

### Common Error Response

All errors return:
```json
{ "error": "descriptive message" }
```
with appropriate HTTP status codes (400, 401, 404, 409, 500).

---

## 6. Transactional Flows

### 6.1 Wallet Creation Flow

```
Client                          wallet-service
  │                                   │
  │ POST /wallet/create               │
  │ {"label": "My Wallet"}            │
  │──────────────────────────────────▶│
  │                                   │── Generate Ed25519 keypair
  │                                   │── Encrypt secret key with server key
  │                                   │── Store encrypted key in RocksDB
  │                                   │── Dual-write to Postgres (if enabled)
  │                                   │
  │ 200 {wallet_address, public_key,  │
  │      chain: "flowcortex-l1"}      │
  │◀──────────────────────────────────│
```

### 6.2 Auth Challenge/Verify Flow

```
Client                          wallet-service
  │                                   │
  │ POST /auth/challenge              │
  │──────────────────────────────────▶│
  │                                   │── Generate UUID challenge
  │                                   │── Set 5-min expiry
  │                                   │── Store in memory + Postgres
  │ 200 {challenge, expires_at}       │
  │◀──────────────────────────────────│
  │                                   │
  │ POST /wallet/sign                 │
  │ {wallet_address, payload:         │
  │  challenge, purpose: "auth"}      │
  │──────────────────────────────────▶│
  │                                   │── Load encrypted key
  │                                   │── Decrypt in memory
  │                                   │── Sign with domain prefix
  │                                   │── Zeroize secret key
  │ 200 {signature, public_key}       │
  │◀──────────────────────────────────│
  │                                   │
  │ POST /auth/verify                 │
  │ {wallet_address, challenge,       │
  │  signature}                       │
  │──────────────────────────────────▶│
  │                                   │── Validate challenge not expired/used
  │                                   │── Verify Ed25519 signature
  │                                   │── Mark challenge used
  │ 200 {valid: true}                 │
  │◀──────────────────────────────────│
```

### 6.3 Transaction Submit Flow

```
Client                          wallet-service
  │                                   │
  │ GET /wallet/nonce?wallet_address= │
  │──────────────────────────────────▶│
  │ 200 {next_nonce: N}               │
  │◀──────────────────────────────────│
  │                                   │
  │ POST /wallet/submit               │
  │ Idempotency-Key: <uuid>           │
  │ {from, to, amount, asset,         │
  │  chain, nonce: N}                 │
  │──────────────────────────────────▶│
  │                                   │── Check idempotency cache
  │                                   │── Load + decrypt key
  │                                   │── Build + sign transaction
  │                                   │── Submit to FlowCortex L1
  │                                   │── Store TX record
  │                                   │── Increment nonce
  │ 200 {tx_hash, status, accepted}   │
  │◀──────────────────────────────────│
  │                                   │
  │ GET /wallet/tx/<tx_hash>          │
  │──────────────────────────────────▶│
  │ 200 {status, accepted, from, to,  │
  │      amount, asset, chain}        │
  │◀──────────────────────────────────│
```

---

## 7. Security Model — Key Protection

### Server-Side Key Custody

- **Private keys NEVER leave the server.** Both JS and WASM frontends are thin HTTP clients.
- Keys are generated server-side using `ed25519-dalek` with OS CSPRNG (`OsRng`).

### Encryption at Rest

- Secret keys are encrypted before storage using XOR with a SHA-256 keystream derived from the server encryption key.
- Encrypted bytes are stored in RocksDB under prefix `wallet-key:{address}`.
- RocksDB directory must be `chmod 700` — restricted to the service user.

### Memory Safety

- `zeroize` crate: All secret key material is zeroed on drop + after use.
- After signing, `secret_key.fill(0)` is called explicitly.
- Domain-separated signing: payloads are prefixed with `keycortex:v1:{purpose}:` before signing.

### Authentication

- JWT verification: HS256 (shared secret) + RS256 (JWKS from AuthBuddy).
- JWKS auto-refresh (default: 60s, configurable).
- Challenge expiry: 5 minutes, single-use, marked used after verification.

### What Teams Should Know

| Concern | Answer |
|---------|--------|
| Can the frontend access private keys? | **No.** Keys are encrypted in RocksDB, decrypted only in-memory during signing, then zeroed. |
| What if someone steals the RocksDB files? | They get encrypted key material. They also need the server's `AUTHBUDDY_JWT_SECRET` / encryption key. |
| Can you restore from passphrase? | Yes — `POST /wallet/restore` re-derives the key using PBKDF-style 1000-round SHA-256 stretching. |
| How are keys differentiated? | Ed25519 (primary). Optional secp256k1 behind a feature flag. |

---

## 8. Frontend Options (JS + WASM)

### JS Baseline (`ui/wallet-baseline/`)

- **No build step** — pure HTML/CSS/JS.
- `app.js` (1035 lines) contains all logic.
- Shared `styles.css` (1736 lines) + `themes.json` (5 skins).
- Dev server: `python3 -m http.server 8090` from `ui/wallet-baseline/`.
- API base URL auto-detected from browser location.

### WASM Frontend (`ui/wallet-wasm/`)

- **Pure Rust** compiled to WebAssembly via `wasm-pack`.
- 12 source modules, 2200+ lines, **100% feature parity** with JS version.
- All 18 API endpoints bound, all 30+ event bindings matched.
- Uses `wasm-bindgen`, `web-sys`, `gloo-*`, `serde`.
- Shares CSS + themes + icons from JS baseline (via relative paths).

**Build:**
```bash
./scripts/build_wasm.sh              # Dev build
./scripts/build_wasm.sh --release    # Optimized build
```

**Serve:**
```bash
python3 -m http.server 8091 --directory ui/wallet-wasm
```

### Parity Table

| Feature | JS | WASM |
|---------|:--:|:----:|
| Wallet create/list/restore/rename | ✅ | ✅ |
| Sign (auth, tx, commitment) | ✅ | ✅ |
| Auth challenge → verify → bind | ✅ | ✅ |
| Submit with idempotency | ✅ | ✅ |
| TX status lookup | ✅ | ✅ |
| Balance query | ✅ | ✅ |
| FortressDigital + ProofCortex | ✅ | ✅ |
| Chain config + health | ✅ | ✅ |
| Fold state (folded/half/unfolded) | ✅ | ✅ |
| 5 theme skins | ✅ | ✅ |
| Auto-fold timers | ✅ | ✅ |
| Icon manifest loading | ✅ | ✅ |

---

## 9. Database & Storage

### Primary: RocksDB (always active)

- Embedded key-value store — no separate server process.
- Data at `data/keystore/rocksdb/` (configurable via `KEYCORTEX_KEYSTORE_PATH`).
- Key prefixes:

| Prefix | Content |
|--------|---------|
| `wallet-key:{addr}` | Encrypted Ed25519 secret key |
| `wallet-binding:{addr}` | User↔wallet binding record |
| `wallet-label:{addr}` | Human-readable name |
| `wallet-nonce:{addr}` | Last used nonce |
| `audit:{timestamp}:{uuid}` | Audit events |
| `idempotency:{key}` | Submit dedup cache |
| `submitted-tx:{hash}` | TX records |

### Optional: PostgreSQL (dual-write)

Enabled when `DATABASE_URL` is set. Provides SQL-friendly queries for:

| Table | Purpose |
|-------|---------|
| `wallet_bindings` | Wallet↔user binding (PK: `wallet_address`) |
| `challenge_store` | Auth challenges with expiry/used tracking |
| `verification_logs` | Audit trail |

**Migrations auto-run at startup** from `KEYCORTEX_POSTGRES_MIGRATIONS_DIR` (default: `./migrations/postgres/`).

**Failover:** If Postgres becomes unavailable, the service **does not crash** — it continues with RocksDB and increments fallback counters (visible on `/health`).

---

## 10. Configuration Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `KEYCORTEX_KEYSTORE_PATH` | No | `./data/keystore/rocksdb` | RocksDB data path |
| `RUST_LOG` | No | — | Log level (`info`, `debug`, `trace`) |
| `DATABASE_URL` | No | — | Postgres connection string |
| `KEYCORTEX_POSTGRES_MIGRATIONS_DIR` | No | `./migrations/postgres` | SQL migration path |
| `AUTHBUDDY_JWT_SECRET` | Yes | `authbuddy-dev-secret-change-me` | HS256 JWT secret (**change in prod!**) |
| `AUTHBUDDY_JWKS_URL` | Recommended | — | JWKS endpoint for RS256 |
| `AUTHBUDDY_JWKS_PATH` | Optional | — | Local JWKS file path |
| `AUTHBUDDY_JWKS_JSON` | Optional | — | Inline JWKS JSON |
| `AUTHBUDDY_JWKS_REFRESH_SECONDS` | No | `60` | JWKS refresh interval (min 10s) |
| `AUTHBUDDY_JWT_ISSUER` | Optional | — | Expected JWT `iss` |
| `AUTHBUDDY_JWT_AUDIENCE` | Optional | — | Expected JWT `aud` |
| `AUTHBUDDY_CALLBACK_URL` | Optional | — | Wallet-binding notification URL |

**Port:** `0.0.0.0:8080` (hardcoded in MVP).

---

## 11. Watchdog & Integration Debugging

### What It Does

The watchdog (`scripts/watchdog.sh`) runs continuously and exercises **every transactional flow** against the live service:

| Flow | Probes |
|------|--------|
| Health | `/health`, `/readyz`, `/startupz`, `/version`, DB counters, JWKS |
| Wallet | create → list → rename → balance |
| Signing | sign(auth), sign(tx) |
| Auth | challenge → sign → verify |
| Submit | nonce → submit (with Idempotency-Key) → tx status |
| Integrations | FortressDigital context + wallet-status, ProofCortex, chain config |
| Frontend | JS (index + app.js) + WASM (index + .js + .wasm) |

### Error Logging

On failure, writes detailed JSON to the debug repo:

```
github.com:veeringman/fd_demo_integ.git
└── keycortex/
    ├── errors/           ← individual error JSON files
    ├── health/           ← latest /health snapshot
    ├── summary/          ← cycle summaries (latest + archived errors)
    └── flows/            ← reserved for flow traces
```

**Push behavior:** Immediately on error, every 5th cycle when healthy, final push on shutdown.

### Running It

```bash
./scripts/watchdog.sh                  # Continuous (60s interval)
./scripts/watchdog.sh --once           # Single pass (for cron/CI)
./scripts/watchdog.sh --interval 30    # Custom interval
```

In Docker: `docker compose --profile watchdog up -d`

### Key Environment Variables

| Variable | Default |
|----------|---------|
| `KEYCORTEX_API_URL` | `http://127.0.0.1:8080` |
| `KEYCORTEX_JS_URL` | `http://127.0.0.1:8090` |
| `KEYCORTEX_WASM_URL` | `http://127.0.0.1:8091` |
| `WATCHDOG_INTERVAL` | `60` |
| `GIT_REMOTE_URL` | `git@github.com:veeringman/fd_demo_integ.git` |

---

## 12. Scripts Inventory

| Script | Purpose | Usage |
|--------|---------|-------|
| `scripts/setup_baremetal.sh` | Full bare-metal setup on Ubuntu 20.04+ | `./scripts/setup_baremetal.sh [--with-postgres] [--install-systemd]` |
| `scripts/setup_docker.sh` | Full Docker-based setup | `./scripts/setup_docker.sh [--no-postgres] [--rebuild]` |
| `scripts/watchdog.sh` | Continuous transactional flow monitor | `./scripts/watchdog.sh [--once] [--interval N]` |
| `scripts/build_wasm.sh` | Build WASM frontend | `./scripts/build_wasm.sh [--release]` |
| `scripts/smoke_db_fallback.sh` | Health/readiness smoke tests | `./scripts/smoke_db_fallback.sh` |
| `scripts/run_wallet_service_cached.sh` | Run with build cache | `./scripts/run_wallet_service_cached.sh [--rebuild]` |
| `scripts/cross_build_windows.sh` | Cross-compile for Windows | `./scripts/cross_build_windows.sh [release]` |

---

## 13. Docker Stack

### Files

| File | Description |
|------|-------------|
| `Dockerfile` | Multi-stage: `rust:1.85-bookworm` builder → `debian:bookworm-slim` runtime |
| `Dockerfile.watchdog` | Lightweight bash/curl/jq/git container |
| `docker-compose.yml` | 5-service stack with profiles |
| `deploy/nginx-wasm.conf` | Nginx with `application/wasm` MIME type |
| `.dockerignore` | Excludes `target/`, `data/`, `.git/` |

### Services

| Service | Container Name | Port | Profile |
|---------|---------------|------|---------|
| `wallet-service` | `keycortex-api` | 8080 | (always) |
| `postgres` | `keycortex-postgres` | 5432 | `postgres`, `full` |
| `ui-js` | `keycortex-ui-js` | 8090 | (always) |
| `ui-wasm` | `keycortex-ui-wasm` | 8091 | (always) |
| `watchdog` | `keycortex-watchdog` | — | `watchdog`, `full` |

### Common Commands

```bash
docker compose up -d                          # Core
docker compose --profile full up -d           # Everything
docker compose logs -f wallet-service         # API logs
docker compose logs -f watchdog               # Monitor logs
docker compose down                           # Stop
docker compose ps                             # Status
```

---

## 14. Integration Points (External Systems)

### AuthBuddy IdP

- **Purpose:** Identity provider for wallet↔user binding.
- **Protocol:** JWT (HS256 shared secret + RS256 via JWKS).
- **Endpoints used:** JWKS URL for key rotation, callback URL for binding notifications.
- **Config:** `AUTHBUDDY_JWT_SECRET`, `AUTHBUDDY_JWKS_URL`, `AUTHBUDDY_JWT_ISSUER`, `AUTHBUDDY_JWT_AUDIENCE`.
- **Docs:** `Integration_Guide_AuthBuddy_IdP.md`

### FortressDigital

- **Purpose:** Risk scoring and policy gating for wallet operations.
- **Endpoints:** `POST /fortressdigital/context`, `POST /fortressdigital/wallet-status`.
- **Returns:** Signed context payloads with wallet verification signals.
- **Docs:** `Integration_Guide_FortressDigital.md`

### ProofCortex

- **Purpose:** Zero-knowledge proof commitment generation.
- **Endpoint:** `POST /proofcortex/commitment`.
- **Returns:** SHA-256 commitment hash over claim data.
- **Docs:** `Integration_Guide_ProofCortex.md`

### FlowCortex L1

- **Purpose:** Target blockchain for transaction submission.
- **Endpoint:** `GET /chain/config` returns chain metadata.
- **Chain adapter:** `kc-chain-flowcortex` crate implements `ChainAdapter` trait.
- **Docs:** `Integration_Guide_FlowCortex_L1.md`

### Treasury Settlement App

- **Purpose:** Settlement layer integration (future).
- **Docs:** `Integration_Guide_Treasury_Settlement_App.md`

---

## 15. Known Gaps & Roadmap

### Completed ✅

- [x] Rust backend — all 24 API endpoints
- [x] RocksDB keystore with encrypted key storage
- [x] PostgreSQL optional dual-write with auto-migrations
- [x] JS baseline frontend (full feature set)
- [x] WASM frontend (100% parity with JS)
- [x] Bare-metal + Docker setup scripts
- [x] Watchdog transactional flow monitor
- [x] DB fallback graceful degradation
- [x] Auth challenge/verify/bind with JWT
- [x] FortressDigital + ProofCortex integration endpoints
- [x] Chain config + FlowCortex L1 adapter

### In Progress / Next

- [ ] Ops console screens (wallet search, audit viewer, dashboard)
- [ ] Desktop shell (Tauri 2)
- [ ] Mobile bridge (UniFFI for iOS/Android)
- [ ] E2E integration tests
- [ ] Security audit + penetration testing
- [ ] Prometheus metrics endpoint
- [ ] Rate limiting middleware
- [ ] TLS termination configuration

### Future

- [ ] Multi-chain support (additional `ChainAdapter` implementations)
- [ ] Hardware security module (HSM) integration
- [ ] Key rotation mechanism
- [ ] WebSocket push for real-time TX status

---

## 16. Troubleshooting Checklist

### "It won't build"

1. Check Rust version: `rustc --version` → must be ≥1.85
2. Check clang: `clang --version` → must be ≥14
3. Missing `libclang-dev`: `sudo apt install libclang-dev`
4. Missing `libssl-dev`: `sudo apt install libssl-dev`
5. WASM target: `rustup target add wasm32-unknown-unknown`

### "wallet-service won't start"

1. Port 8080 in use: `lsof -i :8080` → kill conflicting process
2. RocksDB LOCK: another instance running → stop it
3. Permission denied on data dir: `chmod 700 data/keystore/rocksdb`
4. Missing migrations dir: check `KEYCORTEX_POSTGRES_MIGRATIONS_DIR` path

### "Postgres errors in logs"

1. Postgres not running: `sudo systemctl start postgresql`
2. Connection refused: check `DATABASE_URL` format
3. Auth failed: check `pg_hba.conf` (may need `md5` instead of `peer`)
4. **Not fatal** — service continues with RocksDB, check fallback counters on `/health`

### "Frontend won't load"

1. WASM 404: Did you run `./scripts/build_wasm.sh`?
2. Wrong MIME type for .wasm: Use `deploy/nginx-wasm.conf` for nginx
3. Styles broken: WASM references `../wallet-baseline/styles.css` — ensure relative path works from serving root

### "Watchdog can't push to git"

1. SSH key: ensure `~/.ssh/id_*` key has access to `fd_demo_integ` repo
2. No git remote: watchdog falls back to local logging (still useful)
3. Check `WATCHDOG_REPO_DIR` and `GIT_REMOTE_URL` env vars

---

## 17. Contacts & Resources

### Repositories

| Repo | Purpose |
|------|---------|
| `github.com/veeringman/KeyCortex` | Main codebase |
| `github.com/veeringman/fd_demo_integ` | Integration debug logs (→ `keycortex/`) |

### Documentation (in-repo)

| Document | Content |
|----------|---------|
| `README.md` | Project overview + crate map |
| `KeyCortex_DevOps_Guide.md` | Full ops manual (22 sections) |
| `KeyCortex_API_v0.1_Contract.md` | Frozen API contract |
| `KeyCortex_TechStack_Decision.md` | Architecture decisions |
| `KeyCortex_Operator_Dashboard_Spec.md` | Ops console spec |
| `KeyCortex_Postgres_Degradation_Playbook.md` | DB failover playbook |
| `KeyCortex_Startupz_Field_Ownership_Matrix.md` | Startup check ownership |
| `KeyCortex_TODO_Tracker.md` | Current task tracker |
| `KeyCortex_Learnings.md` | Lessons learned |
| `Integration_Guide_*.md` | Per-system integration docs |

### Ports Quick Reference

| Port | Service |
|------|---------|
| 8080 | wallet-service API |
| 8090 | JS baseline frontend |
| 8091 | WASM frontend |
| 5432 | PostgreSQL (optional) |

---

*This document is version-controlled in the KeyCortex repository. Update it when adding new flows, endpoints, or deployment paths.*
