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
SKIP_BUILD=false

for arg in "$@"; do
  case "$arg" in
    --no-postgres)  WITH_POSTGRES=false ;;
    --build-only)   BUILD_ONLY=true ;;
    --down)         DO_DOWN=true ;;
    --rebuild)      FORCE_REBUILD=true ;;
    --skip-build)   SKIP_BUILD=true ;;
    --no-watchdog)  WITH_WATCHDOG=false ;;
    --help|-h)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --no-postgres   Skip PostgreSQL container"
      echo "  --build-only    Build images only, don't start"
      echo "  --down          Stop all containers and exit"
      echo "  --rebuild       Force rebuild (no cache)"
      echo "  --skip-build    Skip build, just (re)start containers"
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

# ─── Verify & restore Docker config files ────────────────────────────────────
# These files are committed to git. Restore from HEAD if missing or corrupted
# (e.g. previously overwritten by an older version of this script).
step "Checking Docker configuration files"

mkdir -p "$ROOT_DIR/deploy"

DOCKER_CONFIG_FILES="Dockerfile Dockerfile.watchdog docker-compose.yml deploy/nginx-wasm.conf .dockerignore"

for f in $DOCKER_CONFIG_FILES; do
  if [[ -f "$ROOT_DIR/$f" ]]; then
    ok "$f ✓"
  else
    warn "$f missing — restoring from git HEAD"
    (cd "$ROOT_DIR" && git checkout HEAD -- "$f" 2>/dev/null) || {
      err "Cannot restore $f. Run: git checkout -- $DOCKER_CONFIG_FILES"
      exit 1
    }
    ok "$f restored"
  fi
done

# Also force-restore from git to undo any stale local edits from old script versions
info "Ensuring Docker configs match committed versions…"
(cd "$ROOT_DIR" && git checkout HEAD -- $DOCKER_CONFIG_FILES 2>/dev/null) || true
ok "Docker configs verified"

# ─── Build WASM frontend (required — pkg/ is gitignored) ─────────────────────
step "Building WASM frontend"

if [[ -f "$ROOT_DIR/ui/wallet-wasm/pkg/wallet_wasm_bg.wasm" ]] && [[ "$FORCE_REBUILD" != true ]]; then
  ok "WASM pkg/ already built (use --rebuild to force)"
else
  if command -v wasm-pack >/dev/null 2>&1; then
    info "Building WASM frontend with wasm-pack..."
    rustup target add wasm32-unknown-unknown 2>/dev/null || true
    (cd "$ROOT_DIR" && wasm-pack build ui/wallet-wasm --target web --release \
      --out-dir "$ROOT_DIR/ui/wallet-wasm/pkg" --no-typescript)
    ok "WASM frontend built → ui/wallet-wasm/pkg/"
  else
    info "wasm-pack not installed. Installing..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    rustup target add wasm32-unknown-unknown 2>/dev/null || true
    (cd "$ROOT_DIR" && wasm-pack build ui/wallet-wasm --target web --release \
      --out-dir "$ROOT_DIR/ui/wallet-wasm/pkg" --no-typescript)
    ok "WASM frontend built → ui/wallet-wasm/pkg/"
  fi
fi

# ─── Build images ────────────────────────────────────────────────────────────
if [[ "$SKIP_BUILD" == true ]]; then
  info "Skipping build (--skip-build). Using existing images."
else
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
