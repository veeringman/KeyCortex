# KeyCortex — Dev/Ops Setup, Build, Config & Deploy Guide

> **Version:** 2.0 · **Last updated:** 2026-02-27
> **Audience:** You (developer/operator)
> **OS targets:** Ubuntu/Debian Linux (primary), Windows (cross-compile)
> **MSRV:** Rust 1.85+ (edition 2024)

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Clone & Repository Structure](#2-clone--repository-structure)
3. [Rust Toolchain Setup](#3-rust-toolchain-setup)
4. [System Dependencies](#4-system-dependencies)
5. [Build — Linux Native](#5-build--linux-native)
6. [Build — Windows Cross-Compile](#6-build--windows-cross-compile)
7. [Configuration (Environment Variables)](#7-configuration-environment-variables)
8. [Data Directories](#8-data-directories)
9. [Running Locally](#9-running-locally)
10. [PostgreSQL Setup (Optional)](#10-postgresql-setup-optional)
11. [UI — JS Baseline Frontend](#11-ui--js-baseline-frontend)
12. [UI — WASM Frontend (Pure Rust)](#12-ui--wasm-frontend-pure-rust)
13. [Smoke Tests](#13-smoke-tests)
14. [Linting & Formatting](#14-linting--formatting)
15. [Release Build & Packaging](#15-release-build--packaging)
16. [Systemd Service (Production Deploy)](#16-systemd-service-production-deploy)
17. [Docker (Full Stack)](#17-docker-full-stack)
18. [One-Command Setup Scripts](#18-one-command-setup-scripts)
19. [Watchdog — Transactional Flow Monitor](#19-watchdog--transactional-flow-monitor)
20. [Backup & Restore](#20-backup--restore)
21. [Troubleshooting](#21-troubleshooting)
22. [Quick Reference Card](#22-quick-reference-card)

---

## 1. Prerequisites

### Minimum Hardware

| Resource | Dev/Test | Production |
|----------|----------|------------|
| CPU | 2 cores | 4+ cores |
| RAM | 2 GB | 4+ GB |
| Disk | 5 GB | 20+ GB (RocksDB + logs) |

### Software Required

| Tool | Version | Purpose |
|------|---------|---------|
| Ubuntu/Debian | 22.04+ / 24.04 | Host OS |
| Rust | 1.85+ | Compiler (edition 2024) |
| git | 2.x | Source control |
| curl | any | HTTP testing |
| jq | any | JSON parsing (optional but useful) |
| clang + llvm | 14+ | RocksDB build dependency |
| libclang-dev | 14+ | bindgen (RocksDB Rust bindings) |
| pkg-config | any | Library discovery |
| python3 | 3.x | UI static file server (dev) |

---

## 2. Clone & Repository Structure

```bash
git clone https://github.com/veeringman/KeyCortex.git
cd KeyCortex
```

```
KeyCortex/
├── Cargo.toml                  # Workspace root (8 crates + 1 service)
├── clippy.toml                 # MSRV = 1.85
├── rustfmt.toml                # edition 2024, max_width 100
├── crates/
│   ├── kc-api-types/           # Shared request/response types
│   ├── kc-auth-adapter/        # Auth abstraction
│   ├── kc-chain-client/        # ChainAdapter trait
│   ├── kc-chain-flowcortex/    # FlowCortex L1 adapter
│   ├── kc-crypto/              # Ed25519 signing, encryption
│   ├── kc-storage/             # RocksDB keystore
│   └── kc-wallet-core/         # Wallet core logic
├── services/
│   └── wallet-service/         # Main Axum HTTP service (24 endpoints)
├── migrations/
│   └── postgres/
│       └── 0001_init.sql       # Postgres schema (optional)
├── scripts/
│   ├── run_wallet_service_cached.sh
│   └── smoke_db_fallback.sh
├── data/
│   └── keystore/rocksdb/       # Default RocksDB data directory
└── ui/
    └── wallet-baseline/        # Static HTML/CSS/JS frontend
```

---

## 3. Rust Toolchain Setup

### Install Rust (if not installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile default
source "$HOME/.cargo/env"
```

### Verify

```bash
rustc --version    # Must be >= 1.85.0
cargo --version
rustup show
```

### Update existing installation

```bash
rustup update stable
```

---

## 4. System Dependencies

### Ubuntu/Debian

```bash
sudo apt update && sudo apt install -y \
  build-essential \
  pkg-config \
  clang \
  llvm \
  libclang-dev \
  libssl-dev \
  curl \
  jq \
  python3
```

**Why each package:**

| Package | Reason |
|---------|--------|
| `build-essential` | GCC, make, libc headers |
| `clang` + `llvm` + `libclang-dev` | RocksDB C++ compilation + bindgen for Rust FFI |
| `pkg-config` | Locates system libraries |
| `libssl-dev` | OpenSSL headers (reqwest TLS features) |
| `curl` + `jq` | API testing |
| `python3` | Static file server for UI |

### Verify clang

```bash
clang --version      # Should show 14+
llvm-config --version
```

If `libclang-dev` is missing, you'll see errors like:
```
error: failed to run custom build command for `librocksdb-sys`
```
or
```
thread 'main' panicked at 'Unable to find libclang'
```

---

## 5. Build — Linux Native

### Debug build (fast compile, no optimizations)

```bash
cd /path/to/KeyCortex
cargo build -p wallet-service
```

Binary location: `./target/debug/wallet-service`

### Release build (optimized, slower compile)

```bash
cargo build -p wallet-service --release
```

Binary location: `./target/release/wallet-service`

### Build entire workspace

```bash
cargo build --workspace
```

### Build with cached target directory (faster rebuilds)

```bash
# Uses ~/.cache/keycortex/cargo-target to avoid rebuilding on git operations
./scripts/run_wallet_service_cached.sh --rebuild
```

### Expected build time

| Mode | First build | Incremental |
|------|-------------|-------------|
| Debug | 2-5 min | 10-30 sec |
| Release | 5-15 min | 30-60 sec |

---

## 6. Build — Windows Cross-Compile

### 6.1 Install Windows Target

```bash
rustup target add x86_64-pc-windows-gnu
```

### 6.2 Install MinGW Cross-Compiler

```bash
sudo apt install -y \
  gcc-mingw-w64-x86-64 \
  g++-mingw-w64-x86-64 \
  mingw-w64-tools
```

### 6.3 Configure Cargo for Cross-Compilation

Create/edit `~/.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"

[target.x86_64-pc-windows-gnu.rocksdb]
# RocksDB needs C++ cross-compiler
rustflags = ["-C", "link-arg=-lstdc++"]
```

### 6.4 Set Environment for RocksDB Cross-Build

RocksDB's build script needs to find the cross C/C++ compilers:

```bash
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
```

### 6.5 Cross-Compile

```bash
cargo build -p wallet-service --target x86_64-pc-windows-gnu --release
```

Binary location: `./target/x86_64-pc-windows-gnu/release/wallet-service.exe`

### 6.6 Alternative: MSVC Target (requires more setup)

```bash
# For x86_64-pc-windows-msvc target, you need either:
# - Cross-compilation tools (xwin) or
# - Build natively on Windows
# The GNU target above is simpler for cross-compilation from Linux

# If you prefer MSVC:
# cargo install xwin
# xwin --accept-license splat --output /opt/xwin
# See: https://github.com/Jake-Shadle/xwin
rustup target add x86_64-pc-windows-msvc
```

### 6.7 Cross-Compile Helper Script

Save as `scripts/cross_build_windows.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=== KeyCortex Windows Cross-Build ==="

# Check prerequisites
command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1 || {
  echo "ERROR: mingw not installed. Run: sudo apt install gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64"
  exit 1
}

rustup target list --installed | grep -q x86_64-pc-windows-gnu || {
  echo "Adding Windows GNU target..."
  rustup target add x86_64-pc-windows-gnu
}

export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc

MODE="${1:-release}"

if [[ "$MODE" == "release" ]]; then
  cargo build -p wallet-service --target x86_64-pc-windows-gnu --release
  BIN="target/x86_64-pc-windows-gnu/release/wallet-service.exe"
else
  cargo build -p wallet-service --target x86_64-pc-windows-gnu
  BIN="target/x86_64-pc-windows-gnu/debug/wallet-service.exe"
fi

echo ""
echo "Build complete: $BIN"
ls -lh "$BIN"
echo ""
echo "To deploy on Windows:"
echo "  1. Copy wallet-service.exe to Windows machine"
echo "  2. Copy migrations/postgres/ (if using Postgres)"
echo "  3. Copy ui/wallet-baseline/ (for frontend)"
echo "  4. Set environment variables (see guide §7)"
echo "  5. Run: wallet-service.exe"
```

```bash
chmod +x scripts/cross_build_windows.sh
./scripts/cross_build_windows.sh release
```

### 6.8 Known Cross-Compile Issues

| Issue | Solution |
|-------|----------|
| `librocksdb-sys` fails to build | Ensure `g++-mingw-w64-x86-64` is installed and `CXX_x86_64_pc_windows_gnu` is set |
| `cannot find -lstdc++` | Install `g++-mingw-w64-x86-64` |
| `undefined reference to __imp_` | Use `-gnu` target, not `-msvc` |
| Very slow first build | Normal — RocksDB C++ compilation takes time; subsequent builds are cached |
| `ring` or TLS errors | reqwest uses `rustls-tls` (not OpenSSL), so no OpenSSL cross-compile needed |

---

## 7. Configuration (Environment Variables)

### 7.1 Core Service

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `KEYCORTEX_KEYSTORE_PATH` | No | `./data/keystore/rocksdb` | Path to RocksDB data directory |
| `RUST_LOG` | No | (none) | Log level: `info`, `debug`, `warn`, `trace` |

### 7.2 PostgreSQL (Optional Dual-Write)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | No | — | Postgres connection string (e.g., `postgres://user:pass@localhost/keycortex`) |
| `KEYCORTEX_POSTGRES_MIGRATIONS_DIR` | No | `./migrations/postgres` | Path to SQL migration files |

### 7.3 AuthBuddy IdP

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `AUTHBUDDY_JWT_SECRET` | Yes | `authbuddy-dev-secret-change-me` | HS256 shared secret (change in production!) |
| `AUTHBUDDY_JWKS_URL` | Recommended | — | JWKS endpoint URL for RS256 |
| `AUTHBUDDY_JWKS_PATH` | Optional | — | Local JWKS file path |
| `AUTHBUDDY_JWKS_JSON` | Optional | — | Inline JWKS JSON |
| `AUTHBUDDY_JWKS_REFRESH_SECONDS` | No | `60` | JWKS refresh interval (min 10s) |
| `AUTHBUDDY_JWT_ISSUER` | Optional | — | Expected JWT `iss` claim |
| `AUTHBUDDY_JWT_AUDIENCE` | Optional | — | Expected JWT `aud` claim |
| `AUTHBUDDY_CALLBACK_URL` | Optional | — | URL for wallet-binding notifications |

### 7.4 Network

The service listens on **`0.0.0.0:8080`** (hardcoded in MVP). All interfaces, port 8080.

### 7.5 Example `.env` File

Create `.env` in the project root (source before running):

```bash
# .env — KeyCortex wallet-service configuration

# Core
export KEYCORTEX_KEYSTORE_PATH="./data/keystore/rocksdb"
export RUST_LOG="info"

# Postgres (optional — comment out to use RocksDB only)
# export DATABASE_URL="postgres://keycortex:keycortex@localhost:5432/keycortex"
# export KEYCORTEX_POSTGRES_MIGRATIONS_DIR="./migrations/postgres"

# AuthBuddy
export AUTHBUDDY_JWT_SECRET="change-me-in-production"
# export AUTHBUDDY_JWKS_URL="https://authbuddy.example.com/.well-known/jwks.json"
# export AUTHBUDDY_JWT_ISSUER="https://authbuddy.example.com"
# export AUTHBUDDY_JWT_AUDIENCE="keycortex-wallet-service"
# export AUTHBUDDY_CALLBACK_URL="https://authbuddy.example.com/api/wallet-binding"
```

```bash
source .env
```

---

## 8. Data Directories

### Default layout

```
data/
└── keystore/
    └── rocksdb/
        ├── CURRENT
        ├── IDENTITY
        ├── LOCK
        ├── LOG
        ├── MANIFEST-*
        ├── OPTIONS-*
        └── *.sst          # Data files (appear after writes)
```

### Key prefixes in RocksDB

| Prefix | Data |
|--------|------|
| `wallet-key:{addr}` | Encrypted Ed25519 secret key |
| `wallet-binding:{addr}` | User ↔ wallet binding record |
| `wallet-label:{addr}` | Human-readable wallet name |
| `wallet-nonce:{addr}` | Last used nonce |
| `audit:{timestamp}:{uuid}` | Audit event log |
| `idempotency:{key}` | Submit idempotency cache |
| `submitted-tx:{hash}` | Transaction records |

### Permissions

```bash
# Ensure the data directory exists and is writable
mkdir -p ./data/keystore/rocksdb
chmod 700 ./data/keystore/rocksdb
```

⚠️ **The RocksDB directory contains encrypted private keys.** Restrict access to the service user only.

---

## 9. Running Locally

### Quick start (dev mode)

```bash
cd /path/to/KeyCortex
export RUST_LOG=info
cargo run -p wallet-service
```

### Run pre-built binary

```bash
# Debug
./target/debug/wallet-service

# Release
./target/release/wallet-service
```

### Run in background

```bash
RUST_LOG=info ./target/release/wallet-service &
echo $!  # PID for later
```

### Verify it's running

```bash
curl -s http://127.0.0.1:8080/health | jq .
curl -s http://127.0.0.1:8080/version | jq .
curl -s http://127.0.0.1:8080/readyz | jq .ready
```

### Using the cached runner script

```bash
./scripts/run_wallet_service_cached.sh            # Run from cache (or build if missing)
./scripts/run_wallet_service_cached.sh --rebuild   # Force rebuild then run
```

---

## 10. PostgreSQL Setup (Optional)

KeyCortex works **without Postgres** — RocksDB handles all storage. Postgres is optional dual-write for:
- Audit log queries (SQL-friendly)
- Wallet binding persistence
- Challenge store persistence

### 10.1 Install PostgreSQL

```bash
sudo apt install -y postgresql postgresql-client
sudo systemctl enable --now postgresql
```

### 10.2 Create Database and User

```bash
sudo -u postgres psql <<'SQL'
CREATE USER keycortex WITH PASSWORD 'keycortex';
CREATE DATABASE keycortex OWNER keycortex;
GRANT ALL PRIVILEGES ON DATABASE keycortex TO keycortex;
SQL
```

### 10.3 Verify Connection

```bash
psql "postgres://keycortex:keycortex@localhost/keycortex" -c "SELECT 1;"
```

### 10.4 Apply Migrations

Migrations are applied **automatically** at startup when `DATABASE_URL` is set. The service reads `.sql` files from `KEYCORTEX_POSTGRES_MIGRATIONS_DIR` (default: `./migrations/postgres/`), sorts them, and executes in order.

Manual apply:

```bash
psql "postgres://keycortex:keycortex@localhost/keycortex" \
  -f migrations/postgres/0001_init.sql
```

### 10.5 Tables Created

| Table | Purpose |
|-------|---------|
| `wallet_bindings` | Wallet ↔ user binding records |
| `challenge_store` | Auth challenges with expiry/used tracking |
| `verification_logs` | Audit event trail |

### 10.6 Fallback Behavior

If Postgres is configured but **unavailable** (network issue, crash):
- Service **does not crash** — continues with RocksDB
- Fallback counters increment (visible on `/health` and `/readyz`)
- Warnings logged: `"failed to persist ... in Postgres"`
- `/readyz` still reports `ready: true` (keystore is RocksDB)

---

## 11. UI — JS Baseline Frontend

The JS wallet UI is a static HTML/CSS/JS app (no build step) served separately.

### Dev server

```bash
cd ui/wallet-baseline
python3 -m http.server 8090
```

Open: `http://localhost:8090`

The UI auto-detects the API base URL from the browser location. For local dev, it talks to `http://localhost:8080`.

### Production: serve with nginx

```nginx
server {
    listen 80;
    server_name wallet.example.com;

    # Static UI
    location / {
        root /opt/keycortex/ui/wallet-baseline;
        index index.html;
        try_files $uri $uri/ /index.html;
    }

    # Reverse proxy to wallet-service API
    location /wallet/ {
        proxy_pass http://127.0.0.1:8080;
    }
    location /auth/ {
        proxy_pass http://127.0.0.1:8080;
    }
    location /chain/ {
        proxy_pass http://127.0.0.1:8080;
    }
    location /fortressdigital/ {
        proxy_pass http://127.0.0.1:8080;
    }
    location /proofcortex/ {
        proxy_pass http://127.0.0.1:8080;
    }
    location /ops/ {
        proxy_pass http://127.0.0.1:8080;
    }
    location /health {
        proxy_pass http://127.0.0.1:8080;
    }
    location /readyz {
        proxy_pass http://127.0.0.1:8080;
    }
    location /startupz {
        proxy_pass http://127.0.0.1:8080;
    }
    location /version {
        proxy_pass http://127.0.0.1:8080;
    }
}
```

---

---

## 12. UI — WASM Frontend (Pure Rust)

The WASM frontend is a complete rewrite of the JS baseline in pure Rust, compiled to WebAssembly. It provides full feature parity with the JS version — all 18 API endpoints, 30+ event bindings, fold/theme/skin engine.

### 12.1 Prerequisites

```bash
# Install wasm-pack (one-time)
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### 12.2 Build

```bash
# Dev build (fast, unoptimized)
./scripts/build_wasm.sh

# Release build (optimized, smaller .wasm)
./scripts/build_wasm.sh --release
```

Output goes to `ui/wallet-wasm/pkg/` — generates `wallet_wasm.js` + `wallet_wasm_bg.wasm`.

### 12.3 Dev Server

```bash
# From repo root (WASM index.html references ../wallet-baseline/ for shared CSS)
cd /path/to/KeyCortex
python3 -m http.server 8091 --directory ui/wallet-wasm
```

Open: `http://localhost:8091`

### 12.4 Source Structure

```
ui/wallet-wasm/
├── Cargo.toml        # cdylib + rlib, wasm-bindgen, web-sys, gloo-*
├── index.html        # Entry point (loads ./pkg/wallet_wasm.js)
├── src/
│   ├── lib.rs        # WASM entry (#[wasm_bindgen(start)])
│   ├── api.rs        # HTTP client (fetch to wallet-service)
│   ├── dom.rs        # DOM element cache & helpers
│   ├── events.rs     # All event bindings (click, input, focus)
│   ├── fold.rs       # Fold state machine (Folded/Half/Unfolded)
│   ├── theme.rs      # Skin/form-factor engine (CSS var injection)
│   ├── wallet_ops.rs # 11 wallet API operations
│   ├── wallet_list.rs# Wallet list rendering
│   ├── platform.rs   # Platform integration handlers
│   ├── profile.rs    # Profile management
│   ├── state.rs      # Local state persistence
│   └── icons.rs      # Icon manifest loader
└── pkg/              # Build output (git-ignored)
    ├── wallet_wasm.js
    └── wallet_wasm_bg.wasm
```

### 12.5 Production: serve with nginx

Use `deploy/nginx-wasm.conf` which sets the correct `application/wasm` MIME type:

```bash
sudo cp deploy/nginx-wasm.conf /etc/nginx/sites-available/keycortex-wasm
sudo ln -sf /etc/nginx/sites-available/keycortex-wasm /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
```

### 12.6 Key Differences from JS Version

| Aspect | JS Baseline | WASM Frontend |
|--------|-------------|---------------|
| Language | JavaScript (1035 lines) | Rust (2200+ lines, 12 modules) |
| Build step | None | wasm-pack (~15s dev, ~30s release) |
| Bundle size | ~50KB (app.js + styles.css) | ~250KB (.wasm + .js glue) |
| Shared assets | styles.css, themes.json, icons | Same (referenced from ../wallet-baseline/) |
| API parity | Baseline | 100% — all 18 endpoints |
| Event parity | Baseline | 100% — all 30+ bindings |

---

## 13. Smoke Tests

### Run the built-in smoke script

```bash
./scripts/smoke_db_fallback.sh
```

Checks: `/health`, `/readyz`, `/startupz`, fallback counter shape.

### Manual end-to-end test

```bash
BASE=http://127.0.0.1:8080

# Health
curl -s "$BASE/health" | jq '{status, storage_mode, auth_mode}'

# Create wallet
WALLET=$(curl -s -X POST "$BASE/wallet/create" \
  -H "Content-Type: application/json" \
  -d '{"label":"smoke-test"}' | jq -r '.wallet_address')
echo "Created: $WALLET"

# List wallets
curl -s "$BASE/wallet/list" | jq '.total'

# Auth challenge + sign + verify
CHALLENGE=$(curl -s -X POST "$BASE/auth/challenge" | jq -r '.challenge')
SIG=$(curl -s -X POST "$BASE/wallet/sign" \
  -H "Content-Type: application/json" \
  -d "{\"wallet_address\":\"$WALLET\",\"payload\":\"$CHALLENGE\",\"purpose\":\"auth\"}" \
  | jq -r '.signature')
curl -s -X POST "$BASE/auth/verify" \
  -H "Content-Type: application/json" \
  -d "{\"wallet_address\":\"$WALLET\",\"challenge\":\"$CHALLENGE\",\"signature\":\"$SIG\"}" \
  | jq '{valid, wallet_address}'

# Balance
curl -s "$BASE/wallet/balance?wallet_address=$WALLET" | jq .

# Submit + check
NONCE=$(curl -s "$BASE/wallet/nonce?wallet_address=$WALLET" | jq -r '.next_nonce')
TX=$(curl -s -X POST "$BASE/wallet/submit" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: smoke-$(date +%s)" \
  -d "{\"from\":\"$WALLET\",\"to\":\"0x0000000000000000000000000000000000000000\",\"amount\":\"1000\",\"asset\":\"PROOF\",\"chain\":\"flowcortex-l1\",\"nonce\":$NONCE}" \
  | jq -r '.tx_hash')
curl -s "$BASE/wallet/tx/$TX" | jq '{status, accepted}'

echo "All smoke tests passed"
```

---

## 14. Linting & Formatting

### Format

```bash
cargo fmt --all              # Format all crates
cargo fmt --all -- --check   # Check without modifying (CI mode)
```

Config: `rustfmt.toml` → edition 2024, max_width 100.

### Clippy

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Config: `clippy.toml` → msrv = "1.85".

### Run tests

```bash
cargo test --workspace
```

---

## 15. Release Build & Packaging

### Build optimized binary

```bash
cargo build -p wallet-service --release
strip target/release/wallet-service   # Remove debug symbols (~70% smaller)
```

### Check binary size

```bash
ls -lh target/release/wallet-service
# Typical: 15-25 MB (stripped)
```

### Package for deployment

```bash
#!/usr/bin/env bash
set -euo pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
DIST="keycortex-${VERSION}-linux-x86_64"

mkdir -p "dist/$DIST"
cp target/release/wallet-service "dist/$DIST/"
cp -r migrations "dist/$DIST/"
cp -r ui/wallet-baseline "dist/$DIST/ui"
cp .env.example "dist/$DIST/.env" 2>/dev/null || true

cat > "dist/$DIST/README.txt" <<EOF
KeyCortex Wallet Service v${VERSION}
====================================
1. Edit .env with your configuration
2. source .env
3. ./wallet-service
4. UI: serve ui/ with any static HTTP server
5. API: http://localhost:8080
EOF

cd dist
tar czf "${DIST}.tar.gz" "$DIST"
echo "Package: dist/${DIST}.tar.gz"
ls -lh "${DIST}.tar.gz"
```

### Package for Windows

```bash
VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
DIST="keycortex-${VERSION}-windows-x86_64"

mkdir -p "dist/$DIST"
cp target/x86_64-pc-windows-gnu/release/wallet-service.exe "dist/$DIST/"
cp -r migrations "dist/$DIST/"
cp -r ui/wallet-baseline "dist/$DIST/ui"

cat > "dist/$DIST/run.bat" <<'BAT'
@echo off
set RUST_LOG=info
set KEYCORTEX_KEYSTORE_PATH=.\data\keystore\rocksdb
set AUTHBUDDY_JWT_SECRET=change-me-in-production
wallet-service.exe
BAT

cd dist
zip -r "${DIST}.zip" "$DIST"
echo "Package: dist/${DIST}.zip"
```

---

## 16. Systemd Service (Production Deploy)

### 16.1 Create system user

```bash
sudo useradd -r -s /usr/sbin/nologin -d /opt/keycortex keycortex
sudo mkdir -p /opt/keycortex/{bin,data/keystore/rocksdb,migrations/postgres,ui,config}
sudo cp target/release/wallet-service /opt/keycortex/bin/
sudo cp -r migrations/postgres/* /opt/keycortex/migrations/postgres/
sudo cp -r ui/wallet-baseline/* /opt/keycortex/ui/
sudo chown -R keycortex:keycortex /opt/keycortex
sudo chmod 700 /opt/keycortex/data/keystore/rocksdb
```

### 16.2 Environment file

```bash
sudo tee /opt/keycortex/config/wallet-service.env <<'EOF'
# /opt/keycortex/config/wallet-service.env
RUST_LOG=info
KEYCORTEX_KEYSTORE_PATH=/opt/keycortex/data/keystore/rocksdb
KEYCORTEX_POSTGRES_MIGRATIONS_DIR=/opt/keycortex/migrations/postgres

# Uncomment for Postgres dual-write
# DATABASE_URL=postgres://keycortex:SECURE_PASSWORD@localhost:5432/keycortex

# AuthBuddy
AUTHBUDDY_JWT_SECRET=CHANGE_THIS_TO_A_REAL_SECRET
# AUTHBUDDY_JWKS_URL=https://authbuddy.example.com/.well-known/jwks.json
# AUTHBUDDY_JWT_ISSUER=https://authbuddy.example.com
# AUTHBUDDY_JWT_AUDIENCE=keycortex-wallet-service
# AUTHBUDDY_CALLBACK_URL=https://authbuddy.example.com/api/wallet-binding
EOF

sudo chmod 600 /opt/keycortex/config/wallet-service.env
sudo chown keycortex:keycortex /opt/keycortex/config/wallet-service.env
```

### 16.3 Systemd unit file

```bash
sudo tee /etc/systemd/system/keycortex-wallet.service <<'EOF'
[Unit]
Description=KeyCortex Wallet Service
After=network.target postgresql.service
Wants=network.target

[Service]
Type=simple
User=keycortex
Group=keycortex
WorkingDirectory=/opt/keycortex
EnvironmentFile=/opt/keycortex/config/wallet-service.env
ExecStart=/opt/keycortex/bin/wallet-service
Restart=on-failure
RestartSec=5
StartLimitBurst=3
StartLimitIntervalSec=60

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/opt/keycortex/data
PrivateTmp=yes

# Resource limits
LimitNOFILE=65536
MemoryMax=2G

[Install]
WantedBy=multi-user.target
EOF
```

### 16.4 Enable and start

```bash
sudo systemctl daemon-reload
sudo systemctl enable keycortex-wallet
sudo systemctl start keycortex-wallet
sudo systemctl status keycortex-wallet
```

### 16.5 Logs

```bash
sudo journalctl -u keycortex-wallet -f              # Follow live
sudo journalctl -u keycortex-wallet --since today    # Today's logs
sudo journalctl -u keycortex-wallet -n 100           # Last 100 lines
```

### 16.6 Common systemd operations

```bash
sudo systemctl restart keycortex-wallet    # Restart
sudo systemctl stop keycortex-wallet       # Stop
sudo systemctl status keycortex-wallet     # Status + last log lines
```

---

## 17. Docker (Full Stack)

KeyCortex provides a complete Docker stack with multi-stage builds, health checks, and optional Postgres + watchdog services.

### 17.1 Files

| File | Purpose |
|------|---------|
| `Dockerfile` | Multi-stage build: `rust:1.85-bookworm` → `debian:bookworm-slim` (builds API + WASM) |
| `Dockerfile.watchdog` | Lightweight monitoring container (bash, curl, jq, git) |
| `docker-compose.yml` | Full stack orchestration (5 services) |
| `deploy/nginx-wasm.conf` | Nginx config with correct WASM MIME types |
| `.dockerignore` | Excludes `target/`, `data/`, `.git/`, logs |

### 17.2 Quick Start

```bash
# Core services only (API + JS UI + WASM UI)
docker compose up -d

# With PostgreSQL
docker compose --profile postgres up -d

# Everything (API + Postgres + both UIs + watchdog)
docker compose --profile full up -d
```

### 17.3 Services

| Service | Container | Port | Description |
|---------|-----------|------|-------------|
| `wallet-service` | `keycortex-api` | 8080 | Rust API server |
| `postgres` | `keycortex-postgres` | 5432 | PostgreSQL 16 (profile: `postgres`) |
| `ui-js` | `keycortex-ui-js` | 8090 | JS baseline via nginx |
| `ui-wasm` | `keycortex-ui-wasm` | 8091 | WASM frontend via nginx |
| `watchdog` | `keycortex-watchdog` | — | Flow monitor (profile: `watchdog`) |

### 17.4 Build & Run

```bash
# Build all images
docker compose build

# Force rebuild (no cache)
docker compose build --no-cache

# Start with environment overrides
AUTHBUDDY_JWT_SECRET=my-prod-secret \
DATABASE_URL=postgres://keycortex:secret@postgres:5432/keycortex \
docker compose --profile postgres up -d

# View logs
docker compose logs -f wallet-service
docker compose logs -f watchdog
```

### 17.5 Volumes

| Volume | Mounted at | Contains |
|--------|-----------|----------|
| `keycortex-data` | `/app/data` | RocksDB keystore (encrypted keys) |
| `pg-data` | `/var/lib/postgresql/data` | PostgreSQL data |
| `watchdog-data` | `/app/.watchdog` | Watchdog debug repo clone |

### 17.6 Health Checks

All services have Docker health checks:
- **wallet-service**: `curl -sf http://localhost:8080/readyz` (10s interval, 15s start period)
- **postgres**: `pg_isready -U keycortex` (5s interval)
- **ui-js / ui-wasm**: `wget -qO- http://localhost/index.html` (15s interval)

### 17.7 Production Notes

```bash
# Deploy with specific image tag
docker compose -f docker-compose.yml build
docker tag keycortex-wallet-service:latest registry.example.com/keycortex:v0.1.0
docker push registry.example.com/keycortex:v0.1.0

# Backup Docker volumes
docker run --rm -v keycortex-data:/data -v $(pwd):/backup alpine \
  tar czf /backup/keycortex-data-backup.tar.gz -C /data .
```

---

## 18. One-Command Setup Scripts

### 18.1 Bare-Metal Setup

The `scripts/setup_baremetal.sh` script handles the **entire setup** from a fresh Ubuntu 20.04+ machine:

```bash
chmod +x scripts/setup_baremetal.sh

# Interactive (asks about Postgres)
./scripts/setup_baremetal.sh

# Non-interactive options
./scripts/setup_baremetal.sh --with-postgres      # Include Postgres
./scripts/setup_baremetal.sh --no-postgres         # Skip Postgres (RocksDB only)
./scripts/setup_baremetal.sh --install-systemd     # Also install systemd unit
./scripts/setup_baremetal.sh --dev                 # Debug build (faster)
./scripts/setup_baremetal.sh --skip-build          # Use existing binary
```

**What it does:**
1. Installs system dependencies (clang, llvm, libclang-dev, libssl-dev, etc.)
2. Installs/updates Rust toolchain via rustup (requires ≥1.85)
3. Installs wasm-pack + wasm32-unknown-unknown target
4. Builds wallet-service (release mode) + strips binary
5. Builds WASM frontend via wasm-pack
6. Creates data directories + `.env` config file
7. Optionally sets up PostgreSQL (user, database, migrations)
8. Optionally installs systemd service unit
9. Generates nginx config → `deploy/nginx-keycortex.conf`
10. Starts wallet-service (:8080) + JS UI (:8090) + WASM UI (:8091) + watchdog
11. Runs smoke tests (health, wallet create, frontend checks)
12. Creates `stop_keycortex.sh` for easy shutdown

### 18.2 Docker Setup

The `scripts/setup_docker.sh` script sets up everything via Docker:

```bash
chmod +x scripts/setup_docker.sh

# Full setup + launch
./scripts/setup_docker.sh

# Without Postgres
./scripts/setup_docker.sh --no-postgres

# Build only (don't start)
./scripts/setup_docker.sh --build-only

# Stop everything
./scripts/setup_docker.sh --down

# Force rebuild (no cache)
./scripts/setup_docker.sh --rebuild

# Without watchdog
./scripts/setup_docker.sh --no-watchdog
```

**What it does:**
1. Checks/installs Docker Engine + Docker Compose
2. Generates `Dockerfile`, `Dockerfile.watchdog`, `docker-compose.yml` if missing
3. Builds wallet-service image (multi-stage: Rust build + WASM build → slim runtime)
4. Starts all containers with `docker compose`
5. Waits for wallet-service health check
6. Runs smoke tests
7. Displays connection info + container management commands

---

## 19. Watchdog — Transactional Flow Monitor

The `scripts/watchdog.sh` script continuously probes **every transactional API flow** and logs detailed JSON error reports to the integration debug repo.

### 19.1 What It Monitors

| Flow | Probes |
|------|--------|
| **Health** | `/health`, `/readyz`, `/startupz`, `/version`, DB fallback counters, JWKS status |
| **Wallet** | create → list → rename → balance |
| **Signing** | sign (purpose=auth), sign (purpose=tx) |
| **Auth** | challenge → sign challenge → verify |
| **Submit** | nonce → submit (with Idempotency-Key) → tx status |
| **Integrations** | FortressDigital context + wallet-status, ProofCortex commitment, chain config |
| **Frontend** | JS index + app.js, WASM index + .js module + .wasm binary |

### 19.2 Usage

```bash
chmod +x scripts/watchdog.sh

# Continuous monitoring (60s interval)
./scripts/watchdog.sh

# Single pass (good for cron)
./scripts/watchdog.sh --once

# Custom interval
./scripts/watchdog.sh --interval 30

# Background daemon
./scripts/watchdog.sh --daemon
```

### 19.3 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `KEYCORTEX_API_URL` | `http://127.0.0.1:8080` | API base URL |
| `KEYCORTEX_JS_URL` | `http://127.0.0.1:8090` | JS frontend URL |
| `KEYCORTEX_WASM_URL` | `http://127.0.0.1:8091` | WASM frontend URL |
| `WATCHDOG_INTERVAL` | `60` | Seconds between cycles |
| `WATCHDOG_PUSH_EVERY` | `5` | Push to git every N cycles (errors push immediately) |
| `GIT_REMOTE_URL` | `git@github.com:veeringman/fd_demo_integ.git` | Debug repo |
| `WATCHDOG_REPO_DIR` | `.watchdog/fd_demo_integ` | Local clone path |

### 19.4 Error Logging

On any probe failure, the watchdog writes a JSON error file:

```json
{
  "timestamp": "2026-02-27T12:00:00Z",
  "flow": "auth",
  "step": "verify",
  "url": "http://127.0.0.1:8080",
  "http_status": 500,
  "expected_status": 200,
  "response_body": "{\"error\":\"challenge expired\"}",
  "details": "Auth verify failed",
  "latency_seconds": "0.042",
  "hostname": "prod-wallet-01",
  "cycle": 42
}
```

### 19.5 Debug Repo Structure

Errors are pushed to `github.com:veeringman/fd_demo_integ.git`:

```
keycortex/
├── README.md                          # Auto-generated docs
├── errors/                            # Individual error files
│   └── 20260227-120000_auth_verify.json
├── health/
│   └── latest.json                    # Latest /health snapshot
├── summary/
│   ├── latest.json                    # Most recent cycle result
│   └── cycle_20260227-120000.json     # Archived error cycles
└── flows/                             # Reserved for flow traces
```

### 19.6 Push Behavior

- **On error**: Immediately commits + pushes to debug repo
- **On healthy cycle**: Pushes every `WATCHDOG_PUSH_EVERY` cycles (default: 5)
- **On shutdown** (Ctrl+C / SIGTERM): Final push before exit
- If git push fails (no SSH key, network issue), watchdog continues logging locally

---

## 20. Backup & Restore

### RocksDB Backup

```bash
# Stop service first (or use a consistent snapshot)
sudo systemctl stop keycortex-wallet

# Backup
sudo tar czf "keycortex-backup-$(date +%Y%m%d-%H%M%S).tar.gz" \
  -C /opt/keycortex data/keystore/rocksdb

sudo systemctl start keycortex-wallet
```

### RocksDB Restore

```bash
sudo systemctl stop keycortex-wallet
sudo rm -rf /opt/keycortex/data/keystore/rocksdb
sudo tar xzf keycortex-backup-YYYYMMDD-HHMMSS.tar.gz \
  -C /opt/keycortex
sudo chown -R keycortex:keycortex /opt/keycortex/data
sudo systemctl start keycortex-wallet
```

### Postgres Backup (if used)

```bash
pg_dump -U keycortex -h localhost keycortex > keycortex-pg-backup.sql
```

### Postgres Restore

```bash
psql -U keycortex -h localhost keycortex < keycortex-pg-backup.sql
```

⚠️ **Critical:** Back up the RocksDB directory regularly — it contains encrypted wallet keys. Loss = loss of all wallets that weren't created with a passphrase.

---

## 21. Troubleshooting

### Build Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `Unable to find libclang` | Missing libclang-dev | `sudo apt install libclang-dev` |
| `failed to run custom build command for librocksdb-sys` | Missing C++ toolchain | `sudo apt install clang llvm build-essential` |
| `error[E0658]: edition 2024 is experimental` | Rust too old | `rustup update stable` (need 1.85+) |
| `linker 'cc' not found` | Missing GCC | `sudo apt install build-essential` |
| Linking errors with `-lssl` | Missing OpenSSL dev headers | `sudo apt install libssl-dev` |
| Windows cross: `cannot find -lstdc++` | Missing mingw C++ | `sudo apt install g++-mingw-w64-x86-64` |
| wasm-pack: `wasm32-unknown-unknown` missing | Target not installed | `rustup target add wasm32-unknown-unknown` |
| WASM build: `HtmlOptionsCollection` not found | Missing web-sys feature | Check `ui/wallet-wasm/Cargo.toml` web-sys features list |

### Runtime Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `Address already in use (port 8080)` | Port conflict | Kill other process: `lsof -i :8080` then `kill <PID>` |
| `failed to open RocksDB` | Permission denied on data dir | `chmod 700 ./data/keystore/rocksdb` |
| `LOCK file` error from RocksDB | Another instance running | Stop other wallet-service process |
| Postgres `connection refused` | Postgres not running | `sudo systemctl start postgresql` |
| `failed to run postgres migrations` | Missing migration files | Check `KEYCORTEX_POSTGRES_MIGRATIONS_DIR` path |
| JWKS refresh warnings | AuthBuddy endpoint unreachable | Falls back to HS256 — not fatal |

### Performance

| Symptom | Investigation | Fix |
|---------|---------------|-----|
| Slow responses | Check RocksDB compaction: `ls -la data/keystore/rocksdb/*.sst` | Normal; RocksDB self-optimizes |
| High memory | Many wallets + cache | Increase `MemoryMax` in systemd unit |
| JWKS refresh spam | JWKS endpoint down | Fix endpoint or remove `AUTHBUDDY_JWKS_URL` |

### Log Level Guide

```bash
RUST_LOG=trace    # Everything (very verbose)
RUST_LOG=debug    # Debug + info + warn + error
RUST_LOG=info     # Normal operation (recommended)
RUST_LOG=warn     # Only warnings and errors
RUST_LOG=error    # Only errors

# Per-module filtering
RUST_LOG=wallet_service=debug,tower_http=info
```

---

## 22. Quick Reference Card

```
╔══════════════════════════════════════════════════════════════════╗
║  KeyCortex Dev/Ops Quick Reference  (v2.0 — 2026-02-27)        ║
╠══════════════════════════════════════════════════════════════════╣
║                                                                  ║
║  ONE-COMMAND SETUP                                               ║
║    ./scripts/setup_baremetal.sh              (bare metal)        ║
║    ./scripts/setup_docker.sh                 (Docker)            ║
║                                                                  ║
║  BUILD                                                           ║
║    cargo build -p wallet-service              (debug)            ║
║    cargo build -p wallet-service --release    (production)       ║
║    ./scripts/build_wasm.sh                    (WASM dev)         ║
║    ./scripts/build_wasm.sh --release          (WASM prod)        ║
║    ./scripts/cross_build_windows.sh release   (win64)            ║
║                                                                  ║
║  RUN                                                             ║
║    RUST_LOG=info cargo run -p wallet-service  (dev)              ║
║    RUST_LOG=info ./target/release/wallet-service (prod)          ║
║                                                                  ║
║  DOCKER                                                          ║
║    docker compose up -d                       (core)             ║
║    docker compose --profile postgres up -d    (+ Postgres)       ║
║    docker compose --profile full up -d        (everything)       ║
║    docker compose down                        (stop all)         ║
║                                                                  ║
║  TEST & MONITOR                                                  ║
║    cargo test --workspace                                        ║
║    cargo clippy --workspace -- -D warnings                       ║
║    ./scripts/smoke_db_fallback.sh                                ║
║    ./scripts/watchdog.sh                      (flow monitor)     ║
║    ./scripts/watchdog.sh --once               (single pass)      ║
║                                                                  ║
║  HEALTH CHECK                                                    ║
║    curl http://localhost:8080/health | jq .                      ║
║    curl http://localhost:8080/readyz | jq .ready                 ║
║                                                                  ║
║  UI                                                              ║
║    JS:   http://localhost:8090  (wallet-baseline)                ║
║    WASM: http://localhost:8091  (wallet-wasm)                    ║
║                                                                  ║
║  DEPLOY                                                          ║
║    sudo systemctl start keycortex-wallet                         ║
║    sudo journalctl -u keycortex-wallet -f                        ║
║                                                                  ║
║  BACKUP                                                          ║
║    tar czf backup.tar.gz data/keystore/rocksdb                   ║
║                                                                  ║
║  PORTS                                                           ║
║    8080  wallet-service API                                      ║
║    8090  JS baseline UI                                          ║
║    8091  WASM frontend UI                                        ║
║    5432  PostgreSQL (optional)                                   ║
║                                                                  ║
╚══════════════════════════════════════════════════════════════════╝
```
