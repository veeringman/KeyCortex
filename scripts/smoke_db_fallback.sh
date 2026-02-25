#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"

command -v curl >/dev/null 2>&1 || {
  echo "curl is required" >&2
  exit 1
}

command -v grep >/dev/null 2>&1 || {
  echo "grep is required" >&2
  exit 1
}

echo "[1/4] checking /health"
health_json="$(curl -fsS "${BASE_URL}/health")"
printf '%s\n' "$health_json" | grep -q '"storage_mode"'
printf '%s\n' "$health_json" | grep -q '"db_fallback_counters"'

echo "[2/4] checking /readyz"
ready_json="$(curl -fsS "${BASE_URL}/readyz")"
printf '%s\n' "$ready_json" | grep -q '"storage_mode"'
printf '%s\n' "$ready_json" | grep -q '"postgres_startup"'

echo "[3/4] checking /startupz"
startup_json="$(curl -fsS "${BASE_URL}/startupz")"
printf '%s\n' "$startup_json" | grep -q '"postgres_startup"'
printf '%s\n' "$startup_json" | grep -q '"db_fallback_counters"'

echo "[4/4] checking counter shape"
printf '%s\n' "$startup_json" | grep -q '"total"'
printf '%s\n' "$startup_json" | grep -q '"binding_read_failures"'
printf '%s\n' "$startup_json" | grep -q '"audit_read_failures"'

echo "smoke checks passed for ${BASE_URL}"
