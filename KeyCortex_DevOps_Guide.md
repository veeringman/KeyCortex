# KeyCortex — Dev/Ops Setup, Build, Config & Deploy Guide

> **Version:** 1.0 · **Last updated:** 2025-02-25
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
11. [UI (Static Frontend)](#11-ui-static-frontend)
12. [Smoke Tests](#12-smoke-tests)
13. [Linting & Formatting](#13-linting--formatting)
14. [Release Build & Packaging](#14-release-build--packaging)
15. [Systemd Service (Production Deploy)](#15-systemd-service-production-deploy)
16. [Docker (Optional)](#16-docker-optional)
17. [Backup & Restore](#17-backup--restore)
18. [Troubleshooting](#18-troubleshooting)
19. [Quick Reference Card](#19-quick-reference-card)

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

## 11. UI (Static Frontend)

The wallet UI is a static HTML/CSS/JS app served separately.

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

## 12. Smoke Tests

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

## 13. Linting & Formatting

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

## 14. Release Build & Packaging

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

## 15. Systemd Service (Production Deploy)

### 15.1 Create system user

```bash
sudo useradd -r -s /usr/sbin/nologin -d /opt/keycortex keycortex
sudo mkdir -p /opt/keycortex/{bin,data/keystore/rocksdb,migrations/postgres,ui,config}
sudo cp target/release/wallet-service /opt/keycortex/bin/
sudo cp -r migrations/postgres/* /opt/keycortex/migrations/postgres/
sudo cp -r ui/wallet-baseline/* /opt/keycortex/ui/
sudo chown -R keycortex:keycortex /opt/keycortex
sudo chmod 700 /opt/keycortex/data/keystore/rocksdb
```

### 15.2 Environment file

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

### 15.3 Systemd unit file

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

### 15.4 Enable and start

```bash
sudo systemctl daemon-reload
sudo systemctl enable keycortex-wallet
sudo systemctl start keycortex-wallet
sudo systemctl status keycortex-wallet
```

### 15.5 Logs

```bash
sudo journalctl -u keycortex-wallet -f              # Follow live
sudo journalctl -u keycortex-wallet --since today    # Today's logs
sudo journalctl -u keycortex-wallet -n 100           # Last 100 lines
```

### 15.6 Common systemd operations

```bash
sudo systemctl restart keycortex-wallet    # Restart
sudo systemctl stop keycortex-wallet       # Stop
sudo systemctl status keycortex-wallet     # Status + last log lines
```

---

## 16. Docker (Optional)

### Dockerfile

```dockerfile
# Stage 1: Build
FROM rust:1.85-bookworm AS builder

RUN apt-get update && apt-get install -y \
    clang llvm libclang-dev pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .
RUN cargo build -p wallet-service --release && \
    strip target/release/wallet-service

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /usr/sbin/nologin keycortex
WORKDIR /app

COPY --from=builder /src/target/release/wallet-service /app/
COPY migrations /app/migrations
COPY ui/wallet-baseline /app/ui

RUN mkdir -p /app/data/keystore/rocksdb && \
    chown -R keycortex:keycortex /app

USER keycortex

ENV RUST_LOG=info
ENV KEYCORTEX_KEYSTORE_PATH=/app/data/keystore/rocksdb
ENV KEYCORTEX_POSTGRES_MIGRATIONS_DIR=/app/migrations/postgres

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=3s \
  CMD curl -f http://localhost:8080/readyz || exit 1

ENTRYPOINT ["/app/wallet-service"]
```

### Build and run

```bash
docker build -t keycortex:latest .
docker run -d \
  --name keycortex \
  -p 8080:8080 \
  -v keycortex-data:/app/data \
  -e AUTHBUDDY_JWT_SECRET=my-secret \
  keycortex:latest
```

### Docker Compose (with Postgres)

```yaml
# docker-compose.yml
version: '3.8'
services:
  wallet-service:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - keycortex-data:/app/data
    environment:
      - RUST_LOG=info
      - AUTHBUDDY_JWT_SECRET=dev-secret
      - DATABASE_URL=postgres://keycortex:keycortex@postgres:5432/keycortex
    depends_on:
      postgres:
        condition: service_healthy

  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: keycortex
      POSTGRES_PASSWORD: keycortex
      POSTGRES_DB: keycortex
    volumes:
      - pg-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-LINE", "pg_isready -U keycortex"]
      interval: 5s
      timeout: 5s
      retries: 3

  ui:
    image: nginx:alpine
    ports:
      - "8090:80"
    volumes:
      - ./ui/wallet-baseline:/usr/share/nginx/html:ro

volumes:
  keycortex-data:
  pg-data:
```

```bash
docker compose up -d
docker compose logs -f wallet-service
```

---

## 17. Backup & Restore

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

## 18. Troubleshooting

### Build Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `Unable to find libclang` | Missing libclang-dev | `sudo apt install libclang-dev` |
| `failed to run custom build command for librocksdb-sys` | Missing C++ toolchain | `sudo apt install clang llvm build-essential` |
| `error[E0658]: edition 2024 is experimental` | Rust too old | `rustup update stable` (need 1.85+) |
| `linker 'cc' not found` | Missing GCC | `sudo apt install build-essential` |
| Linking errors with `-lssl` | Missing OpenSSL dev headers | `sudo apt install libssl-dev` |
| Windows cross: `cannot find -lstdc++` | Missing mingw C++ | `sudo apt install g++-mingw-w64-x86-64` |

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

## 19. Quick Reference Card

```
╔══════════════════════════════════════════════════════════════╗
║  KeyCortex Dev/Ops Quick Reference                          ║
╠══════════════════════════════════════════════════════════════╣
║                                                              ║
║  BUILD                                                       ║
║    cargo build -p wallet-service              (debug)        ║
║    cargo build -p wallet-service --release    (production)   ║
║    ./scripts/cross_build_windows.sh release   (win64)        ║
║                                                              ║
║  RUN                                                         ║
║    RUST_LOG=info cargo run -p wallet-service  (dev)          ║
║    RUST_LOG=info ./target/release/wallet-service (prod)      ║
║                                                              ║
║  TEST                                                        ║
║    cargo test --workspace                                    ║
║    cargo clippy --workspace -- -D warnings                   ║
║    ./scripts/smoke_db_fallback.sh                            ║
║                                                              ║
║  HEALTH CHECK                                                ║
║    curl http://localhost:8080/health | jq .                  ║
║    curl http://localhost:8080/readyz | jq .ready             ║
║                                                              ║
║  UI                                                          ║
║    cd ui/wallet-baseline && python3 -m http.server 8090      ║
║                                                              ║
║  DEPLOY                                                      ║
║    sudo systemctl start keycortex-wallet                     ║
║    sudo journalctl -u keycortex-wallet -f                    ║
║                                                              ║
║  BACKUP                                                      ║
║    tar czf backup.tar.gz data/keystore/rocksdb               ║
║                                                              ║
║  PORTS                                                       ║
║    8080  wallet-service API                                  ║
║    8090  UI (dev server)                                     ║
║    5432  PostgreSQL (optional)                               ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```
