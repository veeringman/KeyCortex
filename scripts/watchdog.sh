#!/usr/bin/env bash
###############################################################################
# KeyCortex — Transactional Flow Watchdog
#
# Continuously monitors all KeyCortex transactional API flows, logs detailed
# errors, and pushes diagnostic reports to the integration debug repo:
#   github.com:veeringman/fd_demo_integ.git  →  keycortex/
#
# Monitored flows:
#   1. Service health / readyz / startupz / DB fallback counters
#   2. Wallet lifecycle: create, list, restore, rename, balance
#   3. Signing flow: sign (with purpose variants)
#   4. Auth flow: challenge → sign → verify → bind
#   5. Submit flow: nonce → submit → tx status
#   6. Integrations: FortressDigital context/wallet-status, ProofCortex, chain/config
#   7. Frontend availability: JS (8090) + WASM (8091)
#
# Each cycle:
#   - Runs all transactional probes
#   - On ANY failure → writes detailed JSON error log
#   - Pushes error logs to the debug repo every N cycles (or on failure)
#
# Usage:
#   ./scripts/watchdog.sh                       # Interactive (default 60s interval)
#   ./scripts/watchdog.sh --interval 30         # Check every 30 seconds
#   ./scripts/watchdog.sh --once                # Single pass (for cron)
#   ./scripts/watchdog.sh --daemon              # Background daemon mode
#   WATCHDOG_REPO_DIR=/path/to/fd_demo_integ ./scripts/watchdog.sh
#
# Environment:
#   KEYCORTEX_API_URL     API base (default: http://127.0.0.1:8080)
#   KEYCORTEX_JS_URL      JS frontend URL (default: http://127.0.0.1:8090)
#   KEYCORTEX_WASM_URL    WASM frontend URL (default: http://127.0.0.1:8091)
#   WATCHDOG_REPO_DIR     Path to cloned fd_demo_integ repo
#   WATCHDOG_INTERVAL     Seconds between cycles (default: 60)
#   WATCHDOG_PUSH_EVERY   Push to git every N cycles (default: 5, or immediately on error)
#   GIT_REMOTE_URL        Override repo URL (default: git@github.com:veeringman/fd_demo_integ.git)
###############################################################################
set -euo pipefail

# ─── Colors ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

info()  { echo -e "${BLUE}[WATCHDOG]${NC} $(date '+%H:%M:%S') $*"; }
ok()    { echo -e "${GREEN}[PASS]${NC}    $(date '+%H:%M:%S') $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}    $(date '+%H:%M:%S') $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}    $(date '+%H:%M:%S') $*"; }

# ─── Configuration ───────────────────────────────────────────────────────────
API_URL="${KEYCORTEX_API_URL:-http://127.0.0.1:8080}"
JS_URL="${KEYCORTEX_JS_URL:-http://127.0.0.1:8090}"
WASM_URL="${KEYCORTEX_WASM_URL:-http://127.0.0.1:8091}"
INTERVAL="${WATCHDOG_INTERVAL:-60}"
PUSH_EVERY="${WATCHDOG_PUSH_EVERY:-5}"
GIT_REMOTE="${GIT_REMOTE_URL:-git@github.com:veeringman/fd_demo_integ.git}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_DIR="${WATCHDOG_REPO_DIR:-$ROOT_DIR/.watchdog/fd_demo_integ}"
LOG_DIR="$REPO_DIR/keycortex"
RUN_ONCE=false
DAEMON=false

for arg in "$@"; do
  case "$arg" in
    --once)    RUN_ONCE=true ;;
    --daemon)  DAEMON=true ;;
    --interval) shift; INTERVAL="${2:-60}" ;;
    --help|-h)
      head -40 "$0" | grep '^#' | sed 's/^# \?//'
      exit 0 ;;
  esac
  # Handle --interval N (next arg)
  if [[ "${prev_arg:-}" == "--interval" ]]; then INTERVAL="$arg"; fi
  prev_arg="$arg"
done

# ─── Initialize debug repo ──────────────────────────────────────────────────
init_repo() {
  if [[ -d "$REPO_DIR/.git" ]]; then
    info "Debug repo exists: $REPO_DIR"
    cd "$REPO_DIR"
    git pull --rebase --quiet 2>/dev/null || warn "git pull failed (will retry on push)"
  else
    info "Cloning debug repo: $GIT_REMOTE → $REPO_DIR"
    mkdir -p "$(dirname "$REPO_DIR")"
    if git clone "$GIT_REMOTE" "$REPO_DIR" 2>/dev/null; then
      ok "Cloned debug repo"
    else
      warn "Clone failed — creating local repo (will push when remote is available)"
      mkdir -p "$REPO_DIR"
      cd "$REPO_DIR"
      git init -q
      git remote add origin "$GIT_REMOTE" 2>/dev/null || true
    fi
  fi
  mkdir -p "$LOG_DIR/errors" "$LOG_DIR/health" "$LOG_DIR/flows" "$LOG_DIR/summary"
}

# ─── Git push helper ────────────────────────────────────────────────────────
git_push() {
  local reason="${1:-scheduled}"
  cd "$REPO_DIR"
  git add -A keycortex/ 2>/dev/null || true
  if git diff --cached --quiet 2>/dev/null; then
    return 0  # Nothing to commit
  fi
  local ts
  ts="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  git commit -q -m "watchdog: ${reason} @ ${ts}" 2>/dev/null || return 0
  if git push origin HEAD --quiet 2>/dev/null; then
    ok "Pushed diagnostics to debug repo ($reason)"
  else
    warn "Push failed — will retry next cycle"
    # Try setting upstream if needed
    git push -u origin main --quiet 2>/dev/null || \
    git push -u origin master --quiet 2>/dev/null || true
  fi
}

# ─── Probe helpers ───────────────────────────────────────────────────────────
# probe URL [method] [body] [content-type]
# Returns: HTTP status code. Captures response body in $PROBE_BODY
PROBE_BODY=""
PROBE_TIME=""

probe() {
  local url="$1"
  local method="${2:-GET}"
  local body="${3:-}"
  local ct="${4:-application/json}"
  local tmpfile
  tmpfile=$(mktemp)

  local curl_args=(-s -w "\n%{http_code}\n%{time_total}" -o "$tmpfile" --max-time 10)
  if [[ "$method" == "POST" ]]; then
    curl_args+=(-X POST -H "Content-Type: $ct")
    if [[ -n "$body" ]]; then
      curl_args+=(-d "$body")
    fi
  fi

  local raw_out
  raw_out=$(curl "${curl_args[@]}" "$url" 2>/dev/null) || raw_out=$'\n000\n0'
  PROBE_BODY=$(cat "$tmpfile" 2>/dev/null || echo "")
  rm -f "$tmpfile"

  local status_code
  status_code=$(echo "$raw_out" | tail -2 | head -1)
  PROBE_TIME=$(echo "$raw_out" | tail -1)
  echo "${status_code:-000}"
}

# ─── Structured error logger ────────────────────────────────────────────────
# log_error FLOW_NAME STEP_NAME HTTP_STATUS EXPECTED_STATUS RESPONSE_BODY DETAILS
ERRORS_THIS_CYCLE=0
CYCLE_ERRORS_JSON="[]"

log_error() {
  local flow="$1" step="$2" http_status="$3" expected="$4" body="$5" details="${6:-}"
  local ts
  ts="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  local ts_file
  ts_file="$(date -u '+%Y%m%d-%H%M%S')"

  ((ERRORS_THIS_CYCLE++)) || true

  # Escape body for JSON
  local escaped_body
  escaped_body=$(echo "$body" | jq -Rs '.' 2>/dev/null || echo "\"(raw) $(echo "$body" | head -c 500 | tr '"' "'")\"")

  local error_json
  error_json=$(cat <<EJSON
{
  "timestamp": "$ts",
  "flow": "$flow",
  "step": "$step",
  "url": "$API_URL",
  "http_status": $http_status,
  "expected_status": $expected,
  "response_body": $escaped_body,
  "details": $(echo "$details" | jq -Rs '.'),
  "latency_seconds": "$PROBE_TIME",
  "hostname": "$(hostname)",
  "cycle": $CYCLE_COUNT
}
EJSON
)

  # Write individual error file
  local error_file="$LOG_DIR/errors/${ts_file}_${flow}_${step}.json"
  echo "$error_json" | jq '.' > "$error_file" 2>/dev/null || echo "$error_json" > "$error_file"

  # Accumulate for cycle summary
  CYCLE_ERRORS_JSON=$(echo "$CYCLE_ERRORS_JSON" | jq --argjson e "$error_json" '. + [$e]' 2>/dev/null || echo "$CYCLE_ERRORS_JSON")

  fail "$flow/$step → HTTP $http_status (expected $expected)"
}

# ─── Probes ──────────────────────────────────────────────────────────────────
PASS_COUNT=0
FAIL_COUNT=0
TOTAL_PROBES=0

check() {
  local flow="$1" step="$2" url="$3" expected="${4:-200}" method="${5:-GET}" body="${6:-}" details="${7:-}"
  ((TOTAL_PROBES++)) || true
  local status
  status=$(probe "$url" "$method" "$body")
  if [[ "$status" == "$expected" ]]; then
    ((PASS_COUNT++)) || true
    return 0
  else
    ((FAIL_COUNT++)) || true
    log_error "$flow" "$step" "$status" "$expected" "$PROBE_BODY" "$details"
    return 1
  fi
}

# ─── Run full probe cycle ────────────────────────────────────────────────────
CYCLE_COUNT=0
CREATED_WALLET=""

run_cycle() {
  local ts
  ts="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  ERRORS_THIS_CYCLE=0
  CYCLE_ERRORS_JSON="[]"
  PASS_COUNT=0
  FAIL_COUNT=0
  TOTAL_PROBES=0
  ((CYCLE_COUNT++)) || true

  info "═══ Cycle $CYCLE_COUNT @ $ts ═══"

  # ── 1. Health & Diagnostics ──────────────────────────────────────────────
  check "health" "health_endpoint" "$API_URL/health"
  check "health" "readyz_endpoint" "$API_URL/readyz"
  check "health" "startupz_endpoint" "$API_URL/startupz"
  check "health" "version_endpoint" "$API_URL/version"

  # Check DB fallback counters for degradation
  probe "$API_URL/health" GET >/dev/null
  local fallback_total
  fallback_total=$(echo "$PROBE_BODY" | jq -r '.db_fallback_counters.total // 0' 2>/dev/null || echo "0")
  if [[ "$fallback_total" -gt 0 ]]; then
    local counter_details
    counter_details=$(echo "$PROBE_BODY" | jq -c '.db_fallback_counters' 2>/dev/null || echo "{}")
    log_error "health" "db_fallback_counters" 200 200 "$PROBE_BODY" \
      "Postgres fallback detected: total=$fallback_total counters=$counter_details"
    warn "DB fallback counters non-zero: total=$fallback_total"
  fi

  # Check JWKS status
  local jwks_error
  jwks_error=$(echo "$PROBE_BODY" | jq -r '.last_jwks_error // empty' 2>/dev/null || echo "")
  if [[ -n "$jwks_error" ]]; then
    log_error "health" "jwks_status" 200 200 "$PROBE_BODY" "JWKS error: $jwks_error"
    warn "JWKS error: $jwks_error"
  fi

  # ── 2. Wallet Lifecycle ──────────────────────────────────────────────────
  # Create
  local create_status
  create_status=$(probe "$API_URL/wallet/create" POST '{"label":"watchdog-probe"}')
  if [[ "$create_status" == "200" ]]; then
    CREATED_WALLET=$(echo "$PROBE_BODY" | jq -r '.wallet_address // empty' 2>/dev/null || echo "")
    ((PASS_COUNT++)) || true; ((TOTAL_PROBES++)) || true
    if [[ -z "$CREATED_WALLET" ]]; then
      log_error "wallet" "create_parse" "$create_status" 200 "$PROBE_BODY" "wallet_address missing from response"
    fi
  else
    ((FAIL_COUNT++)) || true; ((TOTAL_PROBES++)) || true
    log_error "wallet" "create" "$create_status" 200 "$PROBE_BODY" "Wallet creation failed"
    CREATED_WALLET=""
  fi

  # List
  check "wallet" "list" "$API_URL/wallet/list"

  if [[ -n "$CREATED_WALLET" ]]; then
    # Rename
    check "wallet" "rename" "$API_URL/wallet/rename" 200 POST \
      "{\"wallet_address\":\"$CREATED_WALLET\",\"new_label\":\"watchdog-renamed\"}" \
      "Renaming wallet $CREATED_WALLET"

    # Balance
    check "wallet" "balance" "$API_URL/wallet/balance?wallet_address=$CREATED_WALLET"

    # ── 3. Signing Flow ────────────────────────────────────────────────────
    # Sign with purpose=auth
    local sign_status
    sign_status=$(probe "$API_URL/wallet/sign" POST \
      "{\"wallet_address\":\"$CREATED_WALLET\",\"payload\":\"watchdog-test-payload\",\"purpose\":\"auth\"}")
    local signature=""
    if [[ "$sign_status" == "200" ]]; then
      signature=$(echo "$PROBE_BODY" | jq -r '.signature // empty' 2>/dev/null || echo "")
      ((PASS_COUNT++)) || true; ((TOTAL_PROBES++)) || true
      if [[ -z "$signature" ]]; then
        log_error "signing" "sign_auth_parse" "$sign_status" 200 "$PROBE_BODY" "signature missing from response"
      fi
    else
      ((FAIL_COUNT++)) || true; ((TOTAL_PROBES++)) || true
      log_error "signing" "sign_auth" "$sign_status" 200 "$PROBE_BODY" "Sign with purpose=auth failed"
    fi

    # Sign with purpose=tx
    check "signing" "sign_tx" "$API_URL/wallet/sign" 200 POST \
      "{\"wallet_address\":\"$CREATED_WALLET\",\"payload\":\"tx-probe-data\",\"purpose\":\"tx\"}" \
      "Sign with purpose=tx"

    # ── 4. Auth Flow (challenge → sign → verify) ──────────────────────────
    local challenge_status
    challenge_status=$(probe "$API_URL/auth/challenge" POST)
    local challenge=""
    if [[ "$challenge_status" == "200" ]]; then
      challenge=$(echo "$PROBE_BODY" | jq -r '.challenge // empty' 2>/dev/null || echo "")
      ((PASS_COUNT++)) || true; ((TOTAL_PROBES++)) || true
      if [[ -z "$challenge" ]]; then
        log_error "auth" "challenge_parse" "$challenge_status" 200 "$PROBE_BODY" "challenge missing from response"
      fi
    else
      ((FAIL_COUNT++)) || true; ((TOTAL_PROBES++)) || true
      log_error "auth" "challenge" "$challenge_status" 200 "$PROBE_BODY" "Auth challenge generation failed"
    fi

    if [[ -n "$challenge" ]]; then
      # Sign the challenge
      local auth_sign_status
      auth_sign_status=$(probe "$API_URL/wallet/sign" POST \
        "{\"wallet_address\":\"$CREATED_WALLET\",\"payload\":\"$challenge\",\"purpose\":\"auth\"}")
      local auth_sig=""
      if [[ "$auth_sign_status" == "200" ]]; then
        auth_sig=$(echo "$PROBE_BODY" | jq -r '.signature // empty' 2>/dev/null || echo "")
        ((PASS_COUNT++)) || true; ((TOTAL_PROBES++)) || true
      else
        ((FAIL_COUNT++)) || true; ((TOTAL_PROBES++)) || true
        log_error "auth" "sign_challenge" "$auth_sign_status" 200 "$PROBE_BODY" "Signing auth challenge failed"
      fi

      if [[ -n "$auth_sig" ]]; then
        # Verify
        local verify_status
        verify_status=$(probe "$API_URL/auth/verify" POST \
          "{\"wallet_address\":\"$CREATED_WALLET\",\"challenge\":\"$challenge\",\"signature\":\"$auth_sig\"}")
        if [[ "$verify_status" == "200" ]]; then
          local valid
          valid=$(echo "$PROBE_BODY" | jq -r '.valid // false' 2>/dev/null || echo "false")
          ((PASS_COUNT++)) || true; ((TOTAL_PROBES++)) || true
          if [[ "$valid" != "true" ]]; then
            log_error "auth" "verify_result" "$verify_status" 200 "$PROBE_BODY" "Verification returned valid=false"
          fi
        else
          ((FAIL_COUNT++)) || true; ((TOTAL_PROBES++)) || true
          log_error "auth" "verify" "$verify_status" 200 "$PROBE_BODY" "Auth verify failed"
        fi
      fi
    fi

    # ── 5. Submit Flow (nonce → submit → tx status) ───────────────────────
    local nonce_status
    nonce_status=$(probe "$API_URL/wallet/nonce?wallet_address=$CREATED_WALLET" GET)
    local next_nonce=""
    if [[ "$nonce_status" == "200" ]]; then
      next_nonce=$(echo "$PROBE_BODY" | jq -r '.next_nonce // 0' 2>/dev/null || echo "0")
      ((PASS_COUNT++)) || true; ((TOTAL_PROBES++)) || true
    else
      ((FAIL_COUNT++)) || true; ((TOTAL_PROBES++)) || true
      log_error "submit" "nonce" "$nonce_status" 200 "$PROBE_BODY" "Nonce fetch failed for $CREATED_WALLET"
    fi

    if [[ -n "$next_nonce" ]]; then
      local idem_key="watchdog-$(date +%s)-$$"
      local submit_status
      submit_status=$(probe "$API_URL/wallet/submit" POST \
        "{\"from\":\"$CREATED_WALLET\",\"to\":\"0x0000000000000000000000000000000000000000\",\"amount\":\"1\",\"asset\":\"PROOF\",\"chain\":\"flowcortex-l1\",\"nonce\":$next_nonce}" \
        "application/json")
      # Submit also needs Idempotency-Key header — use probe directly
      local submit_tmpfile
      submit_tmpfile=$(mktemp)
      local submit_raw
      submit_raw=$(curl -s -w "\n%{http_code}\n%{time_total}" -o "$submit_tmpfile" \
        --max-time 10 -X POST \
        -H "Content-Type: application/json" \
        -H "Idempotency-Key: $idem_key" \
        -d "{\"from\":\"$CREATED_WALLET\",\"to\":\"0x0000000000000000000000000000000000000000\",\"amount\":\"1\",\"asset\":\"PROOF\",\"chain\":\"flowcortex-l1\",\"nonce\":$next_nonce}" \
        "$API_URL/wallet/submit" 2>/dev/null) || submit_raw=$'\n000\n0'
      PROBE_BODY=$(cat "$submit_tmpfile" 2>/dev/null || echo "")
      rm -f "$submit_tmpfile"
      submit_status=$(echo "$submit_raw" | tail -2 | head -1)
      PROBE_TIME=$(echo "$submit_raw" | tail -1)

      local tx_hash=""
      if [[ "$submit_status" == "200" ]]; then
        tx_hash=$(echo "$PROBE_BODY" | jq -r '.tx_hash // empty' 2>/dev/null || echo "")
        ((PASS_COUNT++)) || true; ((TOTAL_PROBES++)) || true
        if [[ -z "$tx_hash" ]]; then
          log_error "submit" "submit_parse" "$submit_status" 200 "$PROBE_BODY" "tx_hash missing from response"
        fi
      else
        ((FAIL_COUNT++)) || true; ((TOTAL_PROBES++)) || true
        log_error "submit" "submit_tx" "$submit_status" 200 "$PROBE_BODY" "Transaction submit failed"
      fi

      # TX status lookup
      if [[ -n "$tx_hash" ]]; then
        check "submit" "tx_status" "$API_URL/wallet/tx/$tx_hash" 200 GET "" "TX status for $tx_hash"
      fi
    fi

    # ── 6. Integration Endpoints ──────────────────────────────────────────
    # FortressDigital context
    check "integration" "fortressdigital_context" "$API_URL/fortressdigital/context" 200 POST \
      "{\"wallet_address\":\"$CREATED_WALLET\",\"user_id\":\"watchdog-user\",\"chain\":\"flowcortex-l1\",\"session_id\":\"wd-session\",\"context_data\":\"probe\",\"expires_in_seconds\":60}" \
      "FortressDigital context payload"

    # FortressDigital wallet-status
    check "integration" "fortressdigital_wallet_status" "$API_URL/fortressdigital/wallet-status" 200 POST \
      "{\"wallet_address\":\"$CREATED_WALLET\",\"user_id\":\"watchdog-user\",\"chain\":\"flowcortex-l1\"}" \
      "FortressDigital wallet status"

    # ProofCortex commitment
    check "integration" "proofcortex_commitment" "$API_URL/proofcortex/commitment" 200 POST \
      "{\"wallet_address\":\"$CREATED_WALLET\",\"claim_type\":\"balance\",\"claim_value\":\"1000\"}" \
      "ProofCortex commitment hash"
  fi

  # Chain config (no wallet needed)
  check "integration" "chain_config" "$API_URL/chain/config"

  # ── 7. Frontend Availability ──────────────────────────────────────────
  check "frontend" "js_index" "$JS_URL/" 200 GET "" "JS baseline frontend"
  check "frontend" "js_appjs" "$JS_URL/app.js" 200 GET "" "JS app.js"
  check "frontend" "wasm_index" "$WASM_URL/" 200 GET "" "WASM frontend index"
  check "frontend" "wasm_js_module" "$WASM_URL/pkg/wallet_wasm.js" 200 GET "" "WASM JS glue module"
  check "frontend" "wasm_binary" "$WASM_URL/pkg/wallet_wasm_bg.wasm" 200 GET "" "WASM binary"

  # ── Write cycle summary ─────────────────────────────────────────────────
  local cycle_ts
  cycle_ts="$(date -u '+%Y%m%d-%H%M%S')"
  local status_label="HEALTHY"
  [[ "$ERRORS_THIS_CYCLE" -gt 0 ]] && status_label="DEGRADED"
  [[ "$FAIL_COUNT" -gt 3 ]] && status_label="CRITICAL"

  local summary
  summary=$(cat <<SJSON
{
  "timestamp": "$(date -u '+%Y-%m-%dT%H:%M:%SZ')",
  "cycle": $CYCLE_COUNT,
  "status": "$status_label",
  "total_probes": $TOTAL_PROBES,
  "passed": $PASS_COUNT,
  "failed": $FAIL_COUNT,
  "errors_logged": $ERRORS_THIS_CYCLE,
  "api_url": "$API_URL",
  "js_url": "$JS_URL",
  "wasm_url": "$WASM_URL",
  "hostname": "$(hostname)",
  "errors": $CYCLE_ERRORS_JSON
}
SJSON
)

  # Always write latest summary
  echo "$summary" | jq '.' > "$LOG_DIR/summary/latest.json" 2>/dev/null || echo "$summary" > "$LOG_DIR/summary/latest.json"

  # Archive if errors
  if [[ "$ERRORS_THIS_CYCLE" -gt 0 ]]; then
    echo "$summary" | jq '.' > "$LOG_DIR/summary/cycle_${cycle_ts}.json" 2>/dev/null || \
      echo "$summary" > "$LOG_DIR/summary/cycle_${cycle_ts}.json"
  fi

  # Health snapshot (always)
  probe "$API_URL/health" GET >/dev/null
  echo "$PROBE_BODY" | jq '.' > "$LOG_DIR/health/latest.json" 2>/dev/null || \
    echo "$PROBE_BODY" > "$LOG_DIR/health/latest.json"

  # ── Report ──────────────────────────────────────────────────────────────
  if [[ "$ERRORS_THIS_CYCLE" -eq 0 ]]; then
    ok "Cycle $CYCLE_COUNT: ALL $TOTAL_PROBES probes passed [$status_label]"
  else
    fail "Cycle $CYCLE_COUNT: $FAIL_COUNT/$TOTAL_PROBES failed, $ERRORS_THIS_CYCLE errors logged [$status_label]"
  fi

  # ── Git push logic ─────────────────────────────────────────────────────
  if [[ "$ERRORS_THIS_CYCLE" -gt 0 ]]; then
    git_push "error-detected-cycle-$CYCLE_COUNT"
  elif (( CYCLE_COUNT % PUSH_EVERY == 0 )); then
    git_push "scheduled-cycle-$CYCLE_COUNT"
  fi
}

# ─── Cleanup handler ─────────────────────────────────────────────────────────
cleanup() {
  info "Shutting down watchdog..."
  # Final push
  git_push "watchdog-shutdown" 2>/dev/null || true
  exit 0
}
trap cleanup SIGINT SIGTERM

# ─── Create README in debug repo ────────────────────────────────────────────
write_repo_readme() {
  cat > "$LOG_DIR/README.md" <<'REOF'
# KeyCortex Integration Debug Logs

This directory is auto-populated by the KeyCortex watchdog script.

## Structure

```
keycortex/
├── README.md               ← this file
├── errors/                  ← individual error JSON files
│   └── YYYYMMDD-HHMMSS_flow_step.json
├── health/                  ← latest health endpoint snapshot
│   └── latest.json
├── flows/                   ← (reserved for flow trace logs)
├── summary/
│   ├── latest.json          ← most recent cycle summary
│   └── cycle_YYYYMMDD-HHMMSS.json  ← archived error cycles
```

## Error JSON Format

```json
{
  "timestamp": "2026-02-27T12:00:00Z",
  "flow": "auth",
  "step": "verify",
  "url": "http://127.0.0.1:8080",
  "http_status": 500,
  "expected_status": 200,
  "response_body": "{\"error\":\"...\"}",
  "details": "Auth verify failed",
  "latency_seconds": "0.123",
  "hostname": "myhost",
  "cycle": 42
}
```

## Monitored Flows

| Flow | Steps |
|------|-------|
| health | health, readyz, startupz, version, db_fallback_counters, jwks_status |
| wallet | create, list, rename, balance |
| signing | sign_auth, sign_tx |
| auth | challenge, sign_challenge, verify |
| submit | nonce, submit_tx, tx_status |
| integration | fortressdigital_context, fortressdigital_wallet_status, proofcortex_commitment, chain_config |
| frontend | js_index, js_appjs, wasm_index, wasm_js_module, wasm_binary |

## Running the Watchdog

```bash
# From KeyCortex repo root
./scripts/watchdog.sh                  # Continuous (60s interval)
./scripts/watchdog.sh --once           # Single pass
./scripts/watchdog.sh --interval 30    # Custom interval
```
REOF
}

# ─── Main ────────────────────────────────────────────────────────────────────
main() {
  echo -e "${CYAN}"
  echo "╔══════════════════════════════════════════════════╗"
  echo "║      KeyCortex Transactional Flow Watchdog       ║"
  echo "╚══════════════════════════════════════════════════╝"
  echo -e "${NC}"

  info "API:  $API_URL"
  info "JS:   $JS_URL"
  info "WASM: $WASM_URL"
  info "Interval: ${INTERVAL}s"
  info "Push every: $PUSH_EVERY cycles (or immediately on error)"
  info "Debug repo: $REPO_DIR"

  init_repo
  write_repo_readme

  if [[ "$RUN_ONCE" == true ]]; then
    run_cycle
    git_push "single-run"
    exit $ERRORS_THIS_CYCLE
  fi

  if [[ "$DAEMON" == true ]]; then
    info "Running as daemon — output goes to watchdog.log"
    exec >> "$ROOT_DIR/watchdog.log" 2>&1
  fi

  while true; do
    run_cycle
    info "Next cycle in ${INTERVAL}s... (Ctrl+C to stop)"
    sleep "$INTERVAL"
  done
}

main
