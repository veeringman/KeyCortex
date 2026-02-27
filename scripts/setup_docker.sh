#!/usr/bin/env bash
###############################################################################
# KeyCortex — Docker Setup, Build & Launch Script
#
# What this script does:
#   1. Verifies Docker & Docker Compose are installed (installs if missing)
#   2. Creates Dockerfile + docker-compose.yml
#   3. Builds containers (wallet-service, postgres, UI servers)
#   4. Starts everything with docker compose
#   5. Runs smoke tests
#   6. Starts the watchdog (logs errors to fd_demo_integ repo)
#
# Usage:
#   chmod +x scripts/setup_docker.sh
#   ./scripts/setup_docker.sh                   # Full setup + launch
#   ./scripts/setup_docker.sh --no-postgres      # Without Postgres
#   ./scripts/setup_docker.sh --build-only        # Build only, don't start
#   ./scripts/setup_docker.sh --down              # Stop everything
#   ./scripts/setup_docker.sh --rebuild           # Force rebuild
#   ./scripts/setup_docker.sh --no-watchdog       # Skip watchdog
#
# Requirements:
#   - Ubuntu 20.04 or later
#   - sudo access (for Docker install if needed)
#   - Internet access
###############################################################################
set -euo pipefail

# ─── Colors & helpers ────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }
step()  { echo -e "\n${CYAN}═══ $* ═══${NC}"; }

# ─── Parse arguments ─────────────────────────────────────────────────────────
WITH_POSTGRES=true
BUILD_ONLY=false
DO_DOWN=false
FORCE_REBUILD=false
WITH_WATCHDOG=true

for arg in "$@"; do
  case "$arg" in
    --no-postgres)  WITH_POSTGRES=false ;;
    --build-only)   BUILD_ONLY=true ;;
    --down)         DO_DOWN=true ;;
    --rebuild)      FORCE_REBUILD=true ;;
    --no-watchdog)  WITH_WATCHDOG=false ;;
    --help|-h)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --no-postgres   Skip PostgreSQL container"
      echo "  --build-only    Build images only, don't start"
      echo "  --down          Stop all containers and exit"
      echo "  --rebuild       Force rebuild (no cache)"
      echo "  --no-watchdog   Don't start the watchdog"
      echo "  --help          Show this help"
      exit 0
      ;;
  esac
done

# ─── Detect project root ─────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"
info "Project root: $ROOT_DIR"

# ─── Stop if --down ──────────────────────────────────────────────────────────
if [[ "$DO_DOWN" == true ]]; then
  step "Stopping KeyCortex containers"
  docker compose -f "$ROOT_DIR/docker-compose.yml" down --remove-orphans 2>/dev/null || \
    docker-compose -f "$ROOT_DIR/docker-compose.yml" down --remove-orphans 2>/dev/null || true
  # Also stop watchdog if running
  pkill -f "watchdog.sh" 2>/dev/null || true
  ok "All containers stopped."
  exit 0
fi

# ─── Check / Install Docker ──────────────────────────────────────────────────
step "Checking Docker"

install_docker() {
  info "Installing Docker Engine..."
  sudo apt-get update -qq
  sudo apt-get install -y apt-transport-https ca-certificates curl gnupg lsb-release

  # Add Docker GPG key
  sudo install -m 0755 -d /etc/apt/keyrings
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg 2>/dev/null || true
  sudo chmod a+r /etc/apt/keyrings/docker.gpg

  # Add repo
  echo \
    "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
    $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
    sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

  sudo apt-get update -qq
  sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

  # Add current user to docker group
  sudo usermod -aG docker "$USER" 2>/dev/null || true

  sudo systemctl enable --now docker
  ok "Docker installed"
}

if command -v docker >/dev/null 2>&1; then
  ok "Docker: $(docker --version)"
else
  warn "Docker not found"
  install_docker
fi

# Check docker compose
COMPOSE_CMD=""
if docker compose version >/dev/null 2>&1; then
  COMPOSE_CMD="docker compose"
  ok "Docker Compose: $(docker compose version --short 2>/dev/null || echo 'v2')"
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE_CMD="docker-compose"
  ok "Docker Compose: $(docker-compose --version)"
else
  warn "Docker Compose not found — installing plugin..."
  sudo apt-get install -y docker-compose-plugin 2>/dev/null || \
    sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" \
      -o /usr/local/bin/docker-compose && sudo chmod +x /usr/local/bin/docker-compose
  COMPOSE_CMD="docker compose"
fi

# Ensure jq is available for smoke tests
command -v jq >/dev/null 2>&1 || { info "Installing jq..."; sudo apt-get install -y jq; }

# ─── Generate Dockerfile ─────────────────────────────────────────────────────
step "Generating Dockerfile"

cat > "$ROOT_DIR/Dockerfile" <<'DOCKERFILE'
###############################################################################
# KeyCortex — Multi-stage Docker Build
# Stage 1: Build Rust backend + WASM frontend
# Stage 2: Minimal runtime image
###############################################################################

# ── Stage 1: Builder ─────────────────────────────────────────────────────────
FROM rust:1.93-bookworm AS builder

RUN apt-get update && apt-get install -y \
    clang llvm libclang-dev pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

# Install wasm-pack for WASM frontend build
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /src
COPY . .

# Build wallet-service (release)
RUN cargo build -p wallet-service --release && \
    strip target/release/wallet-service

# Build WASM frontend (release)
RUN rustup target add wasm32-unknown-unknown && \
    wasm-pack build ui/wallet-wasm --target web --release \
      --out-dir ui/wallet-wasm/pkg --no-typescript

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 curl python3 jq git \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /usr/sbin/nologin -m -d /app keycortex

WORKDIR /app

# Copy binary
COPY --from=builder /src/target/release/wallet-service /app/bin/wallet-service

# Copy migrations
COPY --from=builder /src/migrations /app/migrations

# Copy JS baseline frontend
COPY --from=builder /src/ui/wallet-baseline /app/ui/wallet-baseline

# Copy WASM frontend (with built pkg/)
COPY --from=builder /src/ui/wallet-wasm /app/ui/wallet-wasm

# Copy scripts (for watchdog)
COPY --from=builder /src/scripts /app/scripts

# Prepare data directories
RUN mkdir -p /app/data/keystore/rocksdb /app/logs && \
    chown -R keycortex:keycortex /app

USER keycortex

# Environment defaults
ENV RUST_LOG=info
ENV KEYCORTEX_KEYSTORE_PATH=/app/data/keystore/rocksdb
ENV KEYCORTEX_POSTGRES_MIGRATIONS_DIR=/app/migrations/postgres
ENV AUTHBUDDY_JWT_SECRET=change-me-in-production

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --start-period=15s --retries=3 \
  CMD curl -sf http://localhost:8080/readyz || exit 1

ENTRYPOINT ["/app/bin/wallet-service"]
DOCKERFILE

ok "Dockerfile created"

# ─── Generate docker-compose.yml ─────────────────────────────────────────────
step "Generating docker-compose.yml"

cat > "$ROOT_DIR/docker-compose.yml" <<'COMPOSEYML'
###############################################################################
# KeyCortex — Docker Compose
#
# Services:
#   wallet-service   Rust API server (port 8080)
#   postgres         PostgreSQL 16 (optional, port 5432)
#   ui-js            JS baseline frontend via nginx (port 8090)
#   ui-wasm          WASM frontend via nginx (port 8091)
#   watchdog         Transactional flow monitor
#
# Usage:
#   docker compose up -d                      # Start all
#   docker compose up -d --no-deps ui-js      # Start just JS UI
#   docker compose logs -f wallet-service     # Follow API logs
#   docker compose down                       # Stop all
###############################################################################

services:
  # ── Wallet Service API ────────────────────────────────────────────────────
  wallet-service:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: keycortex-api
    ports:
      - "8080:8080"
    volumes:
      - keycortex-data:/app/data
    environment:
      - RUST_LOG=info
      - KEYCORTEX_KEYSTORE_PATH=/app/data/keystore/rocksdb
      - AUTHBUDDY_JWT_SECRET=${AUTHBUDDY_JWT_SECRET:-dev-secret-change-me}
      - DATABASE_URL=${DATABASE_URL:-}
      - KEYCORTEX_POSTGRES_MIGRATIONS_DIR=/app/migrations/postgres
    depends_on:
      postgres:
        condition: service_healthy
        required: false
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://localhost:8080/readyz"]
      interval: 10s
      timeout: 5s
      start_period: 15s
      retries: 3

  # ── PostgreSQL ────────────────────────────────────────────────────────────
  postgres:
    image: postgres:16-alpine
    container_name: keycortex-postgres
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: keycortex
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-keycortex}
      POSTGRES_DB: keycortex
    volumes:
      - pg-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U keycortex"]
      interval: 5s
      timeout: 5s
      retries: 5
    restart: unless-stopped
    profiles:
      - postgres
      - full

  # ── JS Baseline Frontend ─────────────────────────────────────────────────
  ui-js:
    image: nginx:alpine
    container_name: keycortex-ui-js
    ports:
      - "8090:80"
    volumes:
      - ./ui/wallet-baseline:/usr/share/nginx/html:ro
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost/index.html"]
      interval: 15s
      timeout: 3s
      retries: 2

  # ── WASM Frontend ────────────────────────────────────────────────────────
  ui-wasm:
    image: nginx:alpine
    container_name: keycortex-ui-wasm
    ports:
      - "8091:80"
    volumes:
      - ./ui/wallet-wasm:/usr/share/nginx/html:ro
      - ./deploy/nginx-wasm.conf:/etc/nginx/conf.d/default.conf:ro
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost/index.html"]
      interval: 15s
      timeout: 3s
      retries: 2

  # ── Watchdog ──────────────────────────────────────────────────────────────
  watchdog:
    build:
      context: .
      dockerfile: Dockerfile.watchdog
    container_name: keycortex-watchdog
    environment:
      - KEYCORTEX_API_URL=http://wallet-service:8080
      - KEYCORTEX_JS_URL=http://ui-js:80
      - KEYCORTEX_WASM_URL=http://ui-wasm:80
      - WATCHDOG_INTERVAL=${WATCHDOG_INTERVAL:-60}
      - GIT_USER_NAME=${GIT_USER_NAME:-KeyCortex Watchdog}
      - GIT_USER_EMAIL=${GIT_USER_EMAIL:-watchdog@keycortex.local}
    volumes:
      - watchdog-data:/app/.watchdog
      - ${SSH_AUTH_SOCK:-/dev/null}:/ssh-agent:ro
      - ${HOME}/.ssh:/root/.ssh:ro
    depends_on:
      wallet-service:
        condition: service_healthy
    restart: unless-stopped
    profiles:
      - watchdog
      - full

volumes:
  keycortex-data:
    driver: local
  pg-data:
    driver: local
  watchdog-data:
    driver: local
COMPOSEYML

ok "docker-compose.yml created"

# ─── Generate Dockerfile.watchdog ────────────────────────────────────────────
cat > "$ROOT_DIR/Dockerfile.watchdog" <<'WDFILE'
###############################################################################
# KeyCortex Watchdog — Lightweight container for monitoring
###############################################################################
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    bash curl jq git openssh-client ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY scripts/watchdog.sh /app/scripts/watchdog.sh
RUN chmod +x /app/scripts/watchdog.sh

# Git config
RUN git config --global user.name "KeyCortex Watchdog" && \
    git config --global user.email "watchdog@keycortex.local" && \
    git config --global init.defaultBranch main

# SSH known hosts for github.com
RUN mkdir -p /root/.ssh && \
    ssh-keyscan github.com >> /root/.ssh/known_hosts 2>/dev/null

ENV KEYCORTEX_API_URL=http://wallet-service:8080
ENV KEYCORTEX_JS_URL=http://ui-js:80
ENV KEYCORTEX_WASM_URL=http://ui-wasm:80
ENV WATCHDOG_INTERVAL=60
ENV GIT_REMOTE_URL=git@github.com:veeringman/fd_demo_integ.git
ENV WATCHDOG_REPO_DIR=/app/.watchdog/fd_demo_integ

ENTRYPOINT ["/app/scripts/watchdog.sh"]
WDFILE

ok "Dockerfile.watchdog created"

# ─── Generate nginx config for WASM (needs .wasm MIME type) ──────────────────
mkdir -p "$ROOT_DIR/deploy"
cat > "$ROOT_DIR/deploy/nginx-wasm.conf" <<'NGINXWASM'
server {
    listen 80;
    server_name localhost;
    root /usr/share/nginx/html;
    index index.html;

    # Correct MIME types for WASM
    types {
        text/html                 html htm;
        text/css                  css;
        application/javascript    js mjs;
        application/wasm          wasm;
        application/json          json;
        image/png                 png;
        image/svg+xml             svg;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }

    # Cache WASM artifacts aggressively
    location ~* \.(wasm|js)$ {
        add_header Cache-Control "public, max-age=31536000, immutable";
    }
}
NGINXWASM
ok "nginx WASM config: deploy/nginx-wasm.conf"

# ─── Create .dockerignore ────────────────────────────────────────────────────
cat > "$ROOT_DIR/.dockerignore" <<'DIGNORE'
target/
data/
.git/
.watchdog/
*.log
node_modules/
dist/
.env
DIGNORE
ok ".dockerignore created"

# ─── Build images ────────────────────────────────────────────────────────────
step "Building Docker images"

BUILD_ARGS=""
if [[ "$FORCE_REBUILD" == true ]]; then
  BUILD_ARGS="--no-cache"
fi

info "Building wallet-service image (this takes 5-15 min on first run)..."
$COMPOSE_CMD build $BUILD_ARGS wallet-service
ok "wallet-service image built"

if [[ "$BUILD_ONLY" == true ]]; then
  ok "Build complete (--build-only). Run: docker compose up -d"
  exit 0
fi

# ─── Start containers ────────────────────────────────────────────────────────
step "Starting KeyCortex containers"

PROFILES=""
if [[ "$WITH_POSTGRES" == true ]]; then
  PROFILES="--profile postgres"
  info "Starting with PostgreSQL..."
  export DATABASE_URL="postgres://keycortex:keycortex@postgres:5432/keycortex"
fi
if [[ "$WITH_WATCHDOG" == true ]]; then
  PROFILES="$PROFILES --profile watchdog"
fi

$COMPOSE_CMD $PROFILES up -d
ok "Containers started"

# Wait for wallet-service to be ready
info "Waiting for wallet-service to be healthy..."
for i in $(seq 1 60); do
  if curl -sf http://127.0.0.1:8080/health >/dev/null 2>&1; then
    ok "wallet-service is up!"
    break
  fi
  if [[ $i -eq 60 ]]; then
    err "wallet-service failed to start within 60s"
    $COMPOSE_CMD logs wallet-service | tail -30
    exit 1
  fi
  sleep 2
done

# ─── Smoke tests ─────────────────────────────────────────────────────────────
step "Running smoke tests"

PASS=0; FAIL=0

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

smoke "Health"        "http://127.0.0.1:8080/health"   "200"
smoke "Readyz"        "http://127.0.0.1:8080/readyz"   "200"
smoke "Version"       "http://127.0.0.1:8080/version"  "200"
smoke "Wallet list"   "http://127.0.0.1:8080/wallet/list" "200"
smoke "JS frontend"   "http://127.0.0.1:8090/"         "200"
smoke "WASM frontend" "http://127.0.0.1:8091/"         "200"

# Wallet create
CREATE_RESP=$(curl -sf -X POST http://127.0.0.1:8080/wallet/create \
  -H "Content-Type: application/json" -d '{"label":"docker-smoke"}' 2>/dev/null || echo "{}")
WALLET=$(echo "$CREATE_RESP" | jq -r '.wallet_address // empty')
if [[ -n "$WALLET" ]]; then
  ok "Wallet created: ${WALLET:0:16}..."
  ((PASS++))
else
  err "Wallet creation failed"
  ((FAIL++))
fi

echo ""
info "Smoke: ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC}"

# ─── Summary ─────────────────────────────────────────────────────────────────
step "Setup Complete!"

echo ""
echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║           KeyCortex Docker Stack is Running!                    ║${NC}"
echo -e "${CYAN}╠══════════════════════════════════════════════════════════════════╣${NC}"
echo -e "${CYAN}║                                                                  ║${NC}"
echo -e "${CYAN}║  API Server:      http://127.0.0.1:8080                          ║${NC}"
echo -e "${CYAN}║  JS  Frontend:    http://127.0.0.1:8090                          ║${NC}"
echo -e "${CYAN}║  WASM Frontend:   http://127.0.0.1:8091                          ║${NC}"
if [[ "$WITH_POSTGRES" == true ]]; then
echo -e "${CYAN}║  PostgreSQL:      localhost:5432 (keycortex/keycortex)           ║${NC}"
fi
echo -e "${CYAN}║                                                                  ║${NC}"
echo -e "${CYAN}║  Containers:                                                     ║${NC}"
echo -e "${CYAN}║    docker compose ps                                             ║${NC}"
echo -e "${CYAN}║    docker compose logs -f wallet-service                         ║${NC}"
if [[ "$WITH_WATCHDOG" == true ]]; then
echo -e "${CYAN}║    docker compose logs -f watchdog                               ║${NC}"
fi
echo -e "${CYAN}║                                                                  ║${NC}"
echo -e "${CYAN}║  Stop:                                                           ║${NC}"
echo -e "${CYAN}║    ./scripts/setup_docker.sh --down                              ║${NC}"
echo -e "${CYAN}║    docker compose down                                           ║${NC}"
echo -e "${CYAN}║                                                                  ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}"
echo ""
