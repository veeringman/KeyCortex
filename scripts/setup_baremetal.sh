#!/usr/bin/env bash
###############################################################################
# KeyCortex — Bare-Metal Setup, Build & Launch Script
# Target: Ubuntu 20.04+ (also works on 22.04 / 24.04)
#
# What this script does:
#   1. Installs system dependencies (clang, llvm, libclang-dev, etc.)
#   2. Installs Rust toolchain (via rustup) if not present
#   3. Installs wasm-pack for the WASM frontend
#   4. Builds the wallet-service (release mode)
#   5. Builds the WASM frontend
#   6. Optionally sets up PostgreSQL
#   7. Creates a systemd service (optional)
#   8. Starts everything and runs smoke tests
#
# Usage:
#   chmod +x scripts/setup_baremetal.sh
#   ./scripts/setup_baremetal.sh                   # Interactive
#   ./scripts/setup_baremetal.sh --no-postgres      # Skip Postgres
#   ./scripts/setup_baremetal.sh --with-postgres     # Include Postgres
#   ./scripts/setup_baremetal.sh --install-systemd   # Also install systemd unit
#   ./scripts/setup_baremetal.sh --dev               # Debug build (faster)
#
# Requirements:
#   - Ubuntu 20.04 or later
#   - sudo access (for apt installs)
#   - Internet access (for downloads)
#   - ~5 GB disk space (Rust toolchain + build artifacts)
###############################################################################
set -euo pipefail

# ─── Colors & helpers ────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }
step()  { echo -e "\n${CYAN}═══ $* ═══${NC}"; }

# ─── Parse arguments ─────────────────────────────────────────────────────────
WITH_POSTGRES="ask"
INSTALL_SYSTEMD=false
BUILD_MODE="--release"
BUILD_LABEL="release"
SKIP_BUILD=false

for arg in "$@"; do
  case "$arg" in
    --with-postgres)   WITH_POSTGRES="yes" ;;
    --no-postgres)     WITH_POSTGRES="no" ;;
    --install-systemd) INSTALL_SYSTEMD=true ;;
    --dev)             BUILD_MODE=""; BUILD_LABEL="debug" ;;
    --skip-build)      SKIP_BUILD=true ;;
    --help|-h)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --with-postgres    Install and configure PostgreSQL"
      echo "  --no-postgres      Skip PostgreSQL (RocksDB only)"
      echo "  --install-systemd  Install systemd service unit"
      echo "  --dev              Debug build (faster, larger binary)"
      echo "  --skip-build       Skip Rust build (use existing binary)"
      echo "  --help             Show this help"
      exit 0
      ;;
  esac
done

# ─── Detect project root ─────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"
info "Project root: $ROOT_DIR"

# ─── Check Ubuntu version ────────────────────────────────────────────────────
step "Checking system"
if [[ -f /etc/os-release ]]; then
  . /etc/os-release
  info "OS: $PRETTY_NAME"
  if [[ "$ID" != "ubuntu" && "$ID" != "debian" ]]; then
    warn "This script is designed for Ubuntu/Debian. Proceeding anyway..."
  fi
  UBUNTU_MAJOR="${VERSION_ID%%.*}"
  if [[ "$ID" == "ubuntu" && "$UBUNTU_MAJOR" -lt 20 ]]; then
    err "Ubuntu 20.04 or later required. Found: $VERSION_ID"
    exit 1
  fi
else
  warn "Cannot detect OS version. Proceeding anyway..."
fi

info "Arch: $(uname -m)"
info "Kernel: $(uname -r)"

# ─── Ask about Postgres if not specified ──────────────────────────────────────
if [[ "$WITH_POSTGRES" == "ask" ]]; then
  echo ""
  echo -e "${YELLOW}Do you want to set up PostgreSQL? (optional — RocksDB handles all storage)${NC}"
  echo "  PostgreSQL adds SQL-friendly audit logs and wallet binding queries."
  echo ""
  read -rp "Install PostgreSQL? [y/N]: " pg_answer
  case "$pg_answer" in
    [yY]|[yY][eE][sS]) WITH_POSTGRES="yes" ;;
    *) WITH_POSTGRES="no" ;;
  esac
fi

# ─── Install system dependencies ─────────────────────────────────────────────
step "Installing system dependencies"

info "Updating apt cache..."
sudo apt-get update -qq

PACKAGES=(
  build-essential
  pkg-config
  curl
  git
  jq
  python3
  libssl-dev
)

# clang/llvm — use versioned packages on Ubuntu 20.04 if default isn't available
if apt-cache show clang >/dev/null 2>&1; then
  PACKAGES+=(clang llvm libclang-dev)
else
  # Fallback: try clang-12 or clang-10 on older systems
  for v in 14 12 10; do
    if apt-cache show "clang-${v}" >/dev/null 2>&1; then
      PACKAGES+=("clang-${v}" "llvm-${v}" "libclang-${v}-dev")
      break
    fi
  done
fi

if [[ "$WITH_POSTGRES" == "yes" ]]; then
  PACKAGES+=(postgresql postgresql-client)
fi

info "Installing: ${PACKAGES[*]}"
sudo apt-get install -y "${PACKAGES[@]}"

# Verify clang is available
if ! command -v clang >/dev/null 2>&1; then
  # Create symlinks for versioned clang
  for v in 14 12 10; do
    if command -v "clang-${v}" >/dev/null 2>&1; then
      info "Creating clang symlink → clang-${v}"
      sudo update-alternatives --install /usr/bin/clang clang "/usr/bin/clang-${v}" 100
      sudo update-alternatives --install /usr/bin/clang++ clang++ "/usr/bin/clang++-${v}" 100
      if [[ -f "/usr/bin/llvm-config-${v}" ]]; then
        sudo update-alternatives --install /usr/bin/llvm-config llvm-config "/usr/bin/llvm-config-${v}" 100
      fi
      break
    fi
  done
fi

clang --version >/dev/null 2>&1 && ok "clang: $(clang --version | head -1)" || err "clang not found!"

# ─── Install Rust toolchain ──────────────────────────────────────────────────
step "Setting up Rust toolchain"

if command -v rustc >/dev/null 2>&1; then
  RUST_VER="$(rustc --version | awk '{print $2}')"
  RUST_MAJOR="${RUST_VER%%.*}"
  RUST_MINOR="$(echo "$RUST_VER" | cut -d. -f2)"
  if [[ "$RUST_MAJOR" -eq 1 && "$RUST_MINOR" -lt 85 ]]; then
    warn "Rust $RUST_VER found but 1.85+ required (edition 2024). Updating..."
    rustup update stable
  else
    ok "Rust $RUST_VER (>= 1.85 ✓)"
  fi
else
  info "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile default
  source "$HOME/.cargo/env"
fi

# Ensure cargo is on PATH for rest of script
export PATH="$HOME/.cargo/bin:$PATH"
ok "rustc $(rustc --version | awk '{print $2}')"
ok "cargo $(cargo --version | awk '{print $2}')"

# ─── Install wasm-pack ───────────────────────────────────────────────────────
step "Setting up wasm-pack"

if command -v wasm-pack >/dev/null 2>&1; then
  ok "wasm-pack $(wasm-pack --version | awk '{print $2}') already installed"
else
  info "Installing wasm-pack..."
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
  ok "wasm-pack $(wasm-pack --version | awk '{print $2}')"
fi

# Ensure wasm32 target is available
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
  info "Adding wasm32-unknown-unknown target..."
  rustup target add wasm32-unknown-unknown
fi
ok "wasm32-unknown-unknown target installed"

# ─── Build wallet-service ────────────────────────────────────────────────────
if [[ "$SKIP_BUILD" == false ]]; then
  step "Building wallet-service ($BUILD_LABEL)"

  info "This may take 3-15 minutes on first build (RocksDB C++ compilation)..."
  if [[ "$BUILD_MODE" == "--release" ]]; then
    cargo build -p wallet-service --release
    BINARY="$ROOT_DIR/target/release/wallet-service"
  else
    cargo build -p wallet-service
    BINARY="$ROOT_DIR/target/debug/wallet-service"
  fi

  ok "wallet-service built: $BINARY"
  ls -lh "$BINARY"

  # Strip release binary
  if [[ "$BUILD_MODE" == "--release" ]]; then
    strip "$BINARY" 2>/dev/null && ok "Binary stripped" || warn "strip not available"
    ls -lh "$BINARY"
  fi

  # ─── Build WASM frontend ────────────────────────────────────────────────────
  step "Building WASM frontend"

  WASM_CRATE="$ROOT_DIR/ui/wallet-wasm"
  if [[ "$BUILD_MODE" == "--release" ]]; then
    wasm-pack build "$WASM_CRATE" --target web --release --out-dir "$WASM_CRATE/pkg" --no-typescript
  else
    wasm-pack build "$WASM_CRATE" --target web --dev --out-dir "$WASM_CRATE/pkg" --no-typescript
  fi
  ok "WASM frontend built: $WASM_CRATE/pkg/"
else
  if [[ "$BUILD_MODE" == "--release" ]]; then
    BINARY="$ROOT_DIR/target/release/wallet-service"
  else
    BINARY="$ROOT_DIR/target/debug/wallet-service"
  fi
  info "Skipping build (--skip-build). Using: $BINARY"
  [[ -x "$BINARY" ]] || { err "Binary not found: $BINARY"; exit 1; }
fi

# ─── Create data directories ─────────────────────────────────────────────────
step "Preparing data directories"

mkdir -p "$ROOT_DIR/data/keystore/rocksdb"
chmod 700 "$ROOT_DIR/data/keystore/rocksdb"
ok "RocksDB data dir: $ROOT_DIR/data/keystore/rocksdb"

# ─── Create .env file ────────────────────────────────────────────────────────
step "Creating environment configuration"

ENV_FILE="$ROOT_DIR/.env"
if [[ -f "$ENV_FILE" ]]; then
  warn ".env already exists — not overwriting"
  info "Review: $ENV_FILE"
else
  cat > "$ENV_FILE" <<'ENVEOF'
# ─── KeyCortex Environment Configuration ────────────────────────────────────
# Generated by setup_baremetal.sh
# Source this file before running: source .env

# ─── Core ────────────────────────────────────────────────────────────────────
export KEYCORTEX_KEYSTORE_PATH="./data/keystore/rocksdb"
export RUST_LOG="info"

# ─── PostgreSQL (optional — uncomment to enable dual-write) ─────────────────
# export DATABASE_URL="postgres://keycortex:keycortex@localhost:5432/keycortex"
# export KEYCORTEX_POSTGRES_MIGRATIONS_DIR="./migrations/postgres"

# ─── AuthBuddy IdP ──────────────────────────────────────────────────────────
export AUTHBUDDY_JWT_SECRET="change-me-in-production"
# export AUTHBUDDY_JWKS_URL="https://authbuddy.example.com/.well-known/jwks.json"
# export AUTHBUDDY_JWKS_REFRESH_SECONDS="60"
# export AUTHBUDDY_JWT_ISSUER="https://authbuddy.example.com"
# export AUTHBUDDY_JWT_AUDIENCE="keycortex-wallet-service"
# export AUTHBUDDY_CALLBACK_URL="https://authbuddy.example.com/api/wallet-binding"
ENVEOF
  ok "Created $ENV_FILE"
fi

# ─── PostgreSQL Setup ────────────────────────────────────────────────────────
if [[ "$WITH_POSTGRES" == "yes" ]]; then
  step "Configuring PostgreSQL"

  # Start PostgreSQL if not running
  sudo systemctl enable --now postgresql 2>/dev/null || true

  # Create user and database (idempotent)
  info "Creating keycortex database and user..."
  sudo -u postgres psql -v ON_ERROR_STOP=0 <<'SQL' 2>/dev/null || true
DO $$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'keycortex') THEN
    CREATE ROLE keycortex WITH LOGIN PASSWORD 'keycortex';
  END IF;
END $$;
SQL
  sudo -u postgres psql -tc "SELECT 1 FROM pg_database WHERE datname = 'keycortex'" \
    | grep -q 1 || sudo -u postgres psql -c "CREATE DATABASE keycortex OWNER keycortex;"
  sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE keycortex TO keycortex;" 2>/dev/null || true

  # Test connection
  if psql "postgres://keycortex:keycortex@localhost/keycortex" -c "SELECT 1;" >/dev/null 2>&1; then
    ok "PostgreSQL connection verified"
  else
    warn "PostgreSQL connection failed — check pg_hba.conf for local password auth"
    info "You may need to edit /etc/postgresql/*/main/pg_hba.conf"
    info "Change 'peer' to 'md5' for local connections, then: sudo systemctl restart postgresql"
  fi

  # Enable Postgres in .env
  sed -i 's|^# export DATABASE_URL=|export DATABASE_URL=|' "$ENV_FILE"
  sed -i 's|^# export KEYCORTEX_POSTGRES_MIGRATIONS_DIR=|export KEYCORTEX_POSTGRES_MIGRATIONS_DIR=|' "$ENV_FILE"
  ok "Postgres enabled in .env"
fi

# ─── Systemd Service (optional) ──────────────────────────────────────────────
if [[ "$INSTALL_SYSTEMD" == true ]]; then
  step "Installing systemd service"

  INSTALL_DIR="/opt/keycortex"

  info "Setting up $INSTALL_DIR ..."
  sudo useradd -r -s /usr/sbin/nologin -d "$INSTALL_DIR" keycortex 2>/dev/null || true
  sudo mkdir -p "$INSTALL_DIR"/{bin,data/keystore/rocksdb,migrations/postgres,ui/wallet-baseline,ui/wallet-wasm,config}

  sudo cp "$BINARY" "$INSTALL_DIR/bin/wallet-service"
  sudo cp -r "$ROOT_DIR/migrations/postgres/." "$INSTALL_DIR/migrations/postgres/"
  sudo cp -r "$ROOT_DIR/ui/wallet-baseline/." "$INSTALL_DIR/ui/wallet-baseline/"
  if [[ -d "$ROOT_DIR/ui/wallet-wasm/pkg" ]]; then
    sudo cp -r "$ROOT_DIR/ui/wallet-wasm/." "$INSTALL_DIR/ui/wallet-wasm/"
  fi

  # Environment file for systemd
  sudo tee "$INSTALL_DIR/config/wallet-service.env" > /dev/null <<'SYSENV'
RUST_LOG=info
KEYCORTEX_KEYSTORE_PATH=/opt/keycortex/data/keystore/rocksdb
KEYCORTEX_POSTGRES_MIGRATIONS_DIR=/opt/keycortex/migrations/postgres
AUTHBUDDY_JWT_SECRET=CHANGE_THIS_TO_A_REAL_SECRET
# DATABASE_URL=postgres://keycortex:SECURE_PASSWORD@localhost:5432/keycortex
# AUTHBUDDY_JWKS_URL=https://authbuddy.example.com/.well-known/jwks.json
SYSENV

  sudo chown -R keycortex:keycortex "$INSTALL_DIR"
  sudo chmod 700 "$INSTALL_DIR/data/keystore/rocksdb"
  sudo chmod 600 "$INSTALL_DIR/config/wallet-service.env"

  # Systemd unit
  sudo tee /etc/systemd/system/keycortex-wallet.service > /dev/null <<'UNIT'
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

NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/opt/keycortex/data
PrivateTmp=yes
LimitNOFILE=65536
MemoryMax=2G

[Install]
WantedBy=multi-user.target
UNIT

  sudo systemctl daemon-reload
  sudo systemctl enable keycortex-wallet
  ok "Systemd service installed: keycortex-wallet"
  info "Start with: sudo systemctl start keycortex-wallet"
  info "Logs with:  sudo journalctl -u keycortex-wallet -f"
fi

# ─── Create nginx config ─────────────────────────────────────────────────────
step "Generating nginx config (optional)"

NGINX_CONF="$ROOT_DIR/deploy/nginx-keycortex.conf"
mkdir -p "$ROOT_DIR/deploy"
cat > "$NGINX_CONF" <<'NGINX'
# KeyCortex nginx reverse proxy + static UI
# Install: sudo cp deploy/nginx-keycortex.conf /etc/nginx/sites-available/keycortex
#          sudo ln -sf /etc/nginx/sites-available/keycortex /etc/nginx/sites-enabled/
#          sudo nginx -t && sudo systemctl reload nginx

server {
    listen 80;
    server_name localhost;
    # server_name wallet.yourdomain.com;  # ← change for production

    # ─── JS baseline frontend ────────────────────────────────────────────
    location / {
        alias /opt/keycortex/ui/wallet-baseline/;
        index index.html;
        try_files $uri $uri/ /index.html;
    }

    # ─── WASM frontend (served at /wasm/) ────────────────────────────────
    location /wasm/ {
        alias /opt/keycortex/ui/wallet-wasm/;
        index index.html;
        try_files $uri $uri/ /wasm/index.html;

        # Required MIME types for WASM
        types {
            application/wasm wasm;
            application/javascript js;
            text/html html;
            text/css css;
        }
    }

    # ─── API reverse proxy → wallet-service on :8080 ────────────────────
    location /wallet/  { proxy_pass http://127.0.0.1:8080; proxy_set_header Host $host; }
    location /auth/    { proxy_pass http://127.0.0.1:8080; proxy_set_header Host $host; }
    location /chain/   { proxy_pass http://127.0.0.1:8080; proxy_set_header Host $host; }
    location /ops/     { proxy_pass http://127.0.0.1:8080; proxy_set_header Host $host; }
    location /fortressdigital/ { proxy_pass http://127.0.0.1:8080; proxy_set_header Host $host; }
    location /proofcortex/     { proxy_pass http://127.0.0.1:8080; proxy_set_header Host $host; }
    location /health   { proxy_pass http://127.0.0.1:8080; }
    location /readyz   { proxy_pass http://127.0.0.1:8080; }
    location /startupz { proxy_pass http://127.0.0.1:8080; }
    location /version  { proxy_pass http://127.0.0.1:8080; }
}
NGINX
ok "Nginx config: $NGINX_CONF"

# ─── Launch services ─────────────────────────────────────────────────────────
step "Launching KeyCortex"

# Source environment
source "$ENV_FILE"

# Kill any existing wallet-service
pkill -f "wallet-service" 2>/dev/null || true
sleep 1

# Start wallet-service in background
info "Starting wallet-service on port 8080..."
cd "$ROOT_DIR"
nohup "$BINARY" > "$ROOT_DIR/wallet-service.log" 2>&1 &
WS_PID=$!
info "wallet-service PID: $WS_PID"

# Wait for readiness
info "Waiting for wallet-service to be ready..."
for i in $(seq 1 30); do
  if curl -sf http://127.0.0.1:8080/health >/dev/null 2>&1; then
    ok "wallet-service is up!"
    break
  fi
  if [[ $i -eq 30 ]]; then
    err "wallet-service failed to start within 30s"
    err "Check logs: tail -f $ROOT_DIR/wallet-service.log"
    exit 1
  fi
  sleep 1
done

# Start UI dev servers
info "Starting JS frontend on port 8090..."
cd "$ROOT_DIR/ui/wallet-baseline"
nohup python3 -m http.server 8090 > "$ROOT_DIR/ui-js.log" 2>&1 &
JS_PID=$!

info "Starting WASM frontend on port 8091..."
cd "$ROOT_DIR"
nohup python3 -m http.server 8091 --directory "$ROOT_DIR/ui/wallet-wasm" > "$ROOT_DIR/ui-wasm.log" 2>&1 &
WASM_PID=$!

sleep 2

# ─── Smoke tests ─────────────────────────────────────────────────────────────
step "Running smoke tests"

PASS=0
FAIL=0

smoke() {
  local label="$1" url="$2" expect="$3"
  local status
  status=$(curl -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null || echo "000")
  if [[ "$status" == "$expect" ]]; then
    ok "$label → HTTP $status"
    ((PASS++))
  else
    err "$label → HTTP $status (expected $expect)"
    ((FAIL++))
  fi
}

smoke "Health endpoint"   "http://127.0.0.1:8080/health"   "200"
smoke "Readyz endpoint"   "http://127.0.0.1:8080/readyz"   "200"
smoke "Version endpoint"  "http://127.0.0.1:8080/version"  "200"
smoke "Wallet list"       "http://127.0.0.1:8080/wallet/list" "200"
smoke "JS frontend"       "http://127.0.0.1:8090/"         "200"
smoke "WASM frontend"     "http://127.0.0.1:8091/"         "200"
smoke "WASM .js module"   "http://127.0.0.1:8091/pkg/wallet_wasm.js" "200"
smoke "WASM .wasm binary" "http://127.0.0.1:8091/pkg/wallet_wasm_bg.wasm" "200"

# Create test wallet
info "Testing wallet creation..."
CREATE_RESP=$(curl -sf -X POST http://127.0.0.1:8080/wallet/create \
  -H "Content-Type: application/json" \
  -d '{"label":"smoke-test"}' 2>/dev/null || echo "{}")
WALLET_ADDR=$(echo "$CREATE_RESP" | jq -r '.wallet_address // empty')
if [[ -n "$WALLET_ADDR" ]]; then
  ok "Wallet created: ${WALLET_ADDR:0:16}..."
  ((PASS++))
else
  err "Wallet creation failed"
  ((FAIL++))
fi

echo ""
info "Smoke tests: ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC}"

# ─── Start Watchdog ──────────────────────────────────────────────────────────
step "Starting Watchdog"

info "Launching transactional flow watchdog (logs to github.com:veeringman/fd_demo_integ.git → keycortex/)"
info "Configure git SSH keys for auto-push, or watchdog will log locally."

# Git config for watchdog commits
git config --global user.name "KeyCortex Watchdog" 2>/dev/null || true
git config --global user.email "watchdog@keycortex.local" 2>/dev/null || true

nohup "$ROOT_DIR/scripts/watchdog.sh" --interval 60 > "$ROOT_DIR/watchdog.log" 2>&1 &
WD_PID=$!
ok "Watchdog PID: $WD_PID (logs: watchdog.log)"
info "Debug errors pushed to: github.com:veeringman/fd_demo_integ.git → keycortex/"

# ─── Summary ─────────────────────────────────────────────────────────────────
step "Setup Complete!"

cat <<EOF

╔══════════════════════════════════════════════════════════════════════╗
║                    KeyCortex is running!                            ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║  API Server:      http://127.0.0.1:8080                              ║
║  JS  Frontend:    http://127.0.0.1:8090                              ║
║  WASM Frontend:   http://127.0.0.1:8091                              ║
║                                                                      ║
║  Process IDs:                                                        ║
║    wallet-service: $WS_PID
║    JS UI server:   $JS_PID
║    WASM UI server: $WASM_PID
║    Watchdog:       $WD_PID
║                                                                      ║
║  Logs:                                                               ║
║    tail -f wallet-service.log                                        ║
║    tail -f ui-js.log                                                 ║
║    tail -f ui-wasm.log                                               ║
║    tail -f watchdog.log                                              ║
║                                                                      ║
║  Stop all:                                                           ║
║    kill $WS_PID $JS_PID $WASM_PID $WD_PID
║                                                                      ║
║  Rebuild & restart:                                                  ║
║    cargo build -p wallet-service --release                           ║
║    ./scripts/build_wasm.sh --release                                 ║
║                                                                      ║
╚══════════════════════════════════════════════════════════════════════╝

EOF

if [[ "$INSTALL_SYSTEMD" == true ]]; then
  cat <<EOF
  Systemd service installed:
    sudo systemctl start keycortex-wallet
    sudo journalctl -u keycortex-wallet -f

EOF
fi

# ─── Save stop script ────────────────────────────────────────────────────────
cat > "$ROOT_DIR/stop_keycortex.sh" <<STOPEOF
#!/usr/bin/env bash
echo "Stopping KeyCortex..."
kill $WS_PID $JS_PID $WASM_PID $WD_PID 2>/dev/null || true
pkill -f "wallet-service" 2>/dev/null || true
pkill -f "http.server 8090" 2>/dev/null || true
pkill -f "http.server 8091" 2>/dev/null || true
pkill -f "watchdog.sh" 2>/dev/null || true
echo "Stopped."
STOPEOF
chmod +x "$ROOT_DIR/stop_keycortex.sh"
ok "Stop script: ./stop_keycortex.sh"
