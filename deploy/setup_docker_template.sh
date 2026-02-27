#!/usr/bin/env bash
###############################################################################
# {{PLATFORM_NAME}} — Docker Setup, Build & Launch Script
#
# TEMPLATE — Copy this file, search-replace the {{...}} placeholders, and
# customise the service-specific sections marked with "# CUSTOMISE:".
#
# What this script does:
#   1. Verifies Docker & Docker Compose are installed (installs if missing)
#   2. Verifies committed Docker config files exist (auto-restores from git)
#   3. Builds WASM frontend if applicable
#   4. Builds Docker images
#   5. Starts containers with docker compose
#   6. Runs smoke tests
#   7. (Optional) Starts watchdog monitor
#
# Usage:
#   chmod +x scripts/setup_docker.sh
#   ./scripts/setup_docker.sh                   # Full setup + launch
#   ./scripts/setup_docker.sh --no-postgres      # Without Postgres
#   ./scripts/setup_docker.sh --build-only        # Build only, don't start
#   ./scripts/setup_docker.sh --down              # Stop everything
#   ./scripts/setup_docker.sh --rebuild           # Force rebuild (no-cache)
#   ./scripts/setup_docker.sh --skip-build        # Skip build, just restart
#   ./scripts/setup_docker.sh --no-watchdog       # Skip watchdog
#
# Port Allocation (see deploy/platform_port_allocation.md):
#   {{PORT_API}}   — API server
#   {{PORT_UI_JS}} — JS frontend
#   {{PORT_UI_WASM}} — WASM frontend (if applicable)
#   {{PORT_DB}}    — PostgreSQL
#
# Requirements:
#   - Ubuntu 20.04 or later
#   - sudo access (for Docker install if needed)
#   - Internet access
###############################################################################
set -euo pipefail

# ─── Platform identity ───────────────────────────────────────────────────────
# CUSTOMISE: Replace these with your platform's values
PLATFORM_NAME="{{PLATFORM_NAME}}"          # e.g. "AuthBuddy"
PLATFORM_SLUG="{{PLATFORM_SLUG}}"          # e.g. "authbuddy" (used in container names)
PORT_API={{PORT_API}}                       # e.g. 8100
PORT_UI_JS={{PORT_UI_JS}}                   # e.g. 8101  (set to 0 if no JS frontend)
PORT_UI_WASM={{PORT_UI_WASM}}              # e.g. 8102  (set to 0 if no WASM frontend)
PORT_DB={{PORT_DB}}                         # e.g. 5433
# CUSTOMISE: API server binary name
API_BINARY="{{API_BINARY}}"                # e.g. "authbuddy-service"
# CUSTOMISE: Health check endpoint
HEALTH_ENDPOINT="/readyz"                   # e.g. "/readyz" or "/health"
# CUSTOMISE: Does this platform have a WASM frontend?
HAS_WASM_UI=false                           # true or false
# CUSTOMISE: WASM crate path (relative to repo root, only if HAS_WASM_UI=true)
WASM_CRATE="ui/wallet-wasm"

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
      echo "  --skip-build    Skip Docker build, just (re)start"
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
info "Platform:     $PLATFORM_NAME ($PLATFORM_SLUG)"

# ─── Stop if --down ──────────────────────────────────────────────────────────
if [[ "$DO_DOWN" == true ]]; then
  step "Stopping $PLATFORM_NAME containers"
  docker compose -f "$ROOT_DIR/docker-compose.yml" down --remove-orphans 2>/dev/null || \
    docker-compose -f "$ROOT_DIR/docker-compose.yml" down --remove-orphans 2>/dev/null || true
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
  sudo install -m 0755 -d /etc/apt/keyrings
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg | \
    sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg 2>/dev/null || true
  sudo chmod a+r /etc/apt/keyrings/docker.gpg
  DISTRO=$(lsb_release -cs)
  echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
    https://download.docker.com/linux/ubuntu $DISTRO stable" | \
    sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
  sudo apt-get update -qq
  sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
  sudo usermod -aG docker "$USER" || true
  ok "Docker installed"
}

if command -v docker >/dev/null 2>&1; then
  ok "Docker $(docker --version | awk '{print $3}')"
else
  install_docker
fi

# Docker Compose
if docker compose version >/dev/null 2>&1; then
  ok "Docker Compose (plugin)"
elif command -v docker-compose >/dev/null 2>&1; then
  ok "docker-compose (standalone)"
else
  info "Installing Docker Compose plugin..."
  sudo apt-get install -y docker-compose-plugin 2>/dev/null || {
    DC_VERSION="2.24.5"
    sudo curl -fsSL "https://github.com/docker/compose/releases/download/v${DC_VERSION}/docker-compose-linux-$(uname -m)" \
      -o /usr/local/bin/docker-compose && sudo chmod +x /usr/local/bin/docker-compose
  }
  ok "Docker Compose installed"
fi

# ─── Verify & restore Docker config files ────────────────────────────────────
# CUSTOMISE: List all Docker config files committed to your repo.
# This auto-restores them from git HEAD to prevent stale local copies.
step "Checking Docker configuration files"

mkdir -p "$ROOT_DIR/deploy"

# CUSTOMISE: Add or remove files based on your platform
DOCKER_CONFIG_FILES="Dockerfile docker-compose.yml .dockerignore"
# Uncomment if you have these:
# DOCKER_CONFIG_FILES="$DOCKER_CONFIG_FILES Dockerfile.watchdog deploy/nginx-wasm.conf"

for f in $DOCKER_CONFIG_FILES; do
  if [[ -f "$ROOT_DIR/$f" ]]; then
    ok "$f ✓"
  else
    warn "$f missing — restoring from git HEAD"
    (cd "$ROOT_DIR" && git checkout HEAD -- "$f" 2>/dev/null) || {
      err "Cannot restore $f — check your git checkout"
      exit 1
    }
    ok "$f restored"
  fi
done

# Force-restore from git to undo any stale local edits
info "Ensuring Docker configs match committed versions…"
(cd "$ROOT_DIR" && git checkout HEAD -- $DOCKER_CONFIG_FILES 2>/dev/null) || true
ok "Docker configs verified"

# ─── Build WASM frontend (if applicable) ─────────────────────────────────────
if [[ "$HAS_WASM_UI" == true ]]; then
  step "Building WASM frontend"

  WASM_OUTPUT="$ROOT_DIR/$WASM_CRATE/pkg/$(basename "$WASM_CRATE" | tr '-' '_')_bg.wasm"

  if [[ -f "$WASM_OUTPUT" ]] && [[ "$FORCE_REBUILD" != true ]]; then
    ok "WASM pkg/ already built (use --rebuild to force)"
  else
    # Auto-install wasm-pack if missing
    if ! command -v wasm-pack >/dev/null 2>&1; then
      info "wasm-pack not found — installing…"
      curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    fi
    rustup target add wasm32-unknown-unknown 2>/dev/null || true
    info "Building WASM frontend with wasm-pack..."
    (cd "$ROOT_DIR" && wasm-pack build "$WASM_CRATE" --target web --release \
      --out-dir "$ROOT_DIR/$WASM_CRATE/pkg" --no-typescript)
    ok "WASM frontend built → $WASM_CRATE/pkg/"
  fi
fi

# ─── Build Docker images ─────────────────────────────────────────────────────
if [[ "$SKIP_BUILD" == true ]]; then
  info "Skipping build (--skip-build). Using existing images."
else
  step "Building Docker images"

  BUILD_ARGS=""
  if [[ "$FORCE_REBUILD" == true ]]; then
    BUILD_ARGS="--no-cache"
  fi

  docker compose -f "$ROOT_DIR/docker-compose.yml" build $BUILD_ARGS 2>&1 | \
    tail -20
  ok "Docker images built"
fi

if [[ "$BUILD_ONLY" == true ]]; then
  ok "Build complete (--build-only). Exiting."
  exit 0
fi

# ─── Start containers ────────────────────────────────────────────────────────
step "Starting $PLATFORM_NAME services"

COMPOSE_CMD="docker compose -f $ROOT_DIR/docker-compose.yml"

# Build the profile flags
PROFILES=""
if [[ "$WITH_POSTGRES" == true ]]; then
  PROFILES="$PROFILES --profile postgres"
fi
if [[ "$WITH_WATCHDOG" == true ]]; then
  PROFILES="$PROFILES --profile watchdog"
fi

$COMPOSE_CMD $PROFILES up -d --remove-orphans 2>&1 | tail -20
ok "Containers started"

# ─── Wait for API health ─────────────────────────────────────────────────────
step "Waiting for $PLATFORM_NAME API health"

MAX_WAIT=60
WAITED=0
while [[ $WAITED -lt $MAX_WAIT ]]; do
  if curl -sf "http://localhost:${PORT_API}${HEALTH_ENDPOINT}" >/dev/null 2>&1; then
    ok "$PLATFORM_NAME API healthy (port $PORT_API) after ${WAITED}s"
    break
  fi
  sleep 2
  WAITED=$((WAITED + 2))
done

if [[ $WAITED -ge $MAX_WAIT ]]; then
  err "$PLATFORM_NAME API not healthy after ${MAX_WAIT}s"
  docker compose -f "$ROOT_DIR/docker-compose.yml" logs --tail=30 2>/dev/null
  exit 1
fi

# ─── Smoke tests ─────────────────────────────────────────────────────────────
step "Running smoke tests"

smoke_check() {
  local label="$1" url="$2" expected="${3:-200}"
  local code
  code=$(curl -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null || echo "000")
  if [[ "$code" == "$expected" ]]; then
    ok "$label → $code"
  else
    warn "$label → $code (expected $expected)"
  fi
}

# CUSTOMISE: Add your platform's smoke test endpoints
smoke_check "API health"     "http://localhost:${PORT_API}${HEALTH_ENDPOINT}"
smoke_check "API readyz"     "http://localhost:${PORT_API}/readyz"

if [[ $PORT_UI_JS -gt 0 ]]; then
  smoke_check "UI-JS index"  "http://localhost:${PORT_UI_JS}/"
fi
if [[ $PORT_UI_WASM -gt 0 ]] && [[ "$HAS_WASM_UI" == true ]]; then
  smoke_check "UI-WASM index" "http://localhost:${PORT_UI_WASM}/"
fi

# CUSTOMISE: Platform-specific smoke tests
# smoke_check "Create resource" "http://localhost:${PORT_API}/some/endpoint" "200"

# ─── Summary ─────────────────────────────────────────────────────────────────
step "$PLATFORM_NAME — Running"

echo ""
echo "  Services:"
echo "    API:        http://localhost:${PORT_API}"
[[ $PORT_UI_JS -gt 0 ]]   && echo "    UI (JS):    http://localhost:${PORT_UI_JS}"
[[ $PORT_UI_WASM -gt 0 ]] && echo "    UI (WASM):  http://localhost:${PORT_UI_WASM}"
[[ "$WITH_POSTGRES" == true ]] && echo "    Postgres:   localhost:${PORT_DB}"
echo ""
echo "  Commands:"
echo "    docker compose ps                    # Status"
echo "    docker compose logs -f               # Logs"
echo "    $0 --down              # Stop all"
echo "    $0 --rebuild           # Full rebuild"
echo ""
ok "Done."
