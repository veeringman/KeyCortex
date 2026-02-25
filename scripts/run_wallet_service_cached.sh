#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

DEFAULT_TARGET_DIR="$HOME/.cache/keycortex/cargo-target"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$DEFAULT_TARGET_DIR}"

WORKSPACE_BIN="$ROOT_DIR/target/debug/wallet-service"
CACHED_BIN="$CARGO_TARGET_DIR/debug/wallet-service"

if [[ "${1:-}" == "--rebuild" ]]; then
  cargo build -p wallet-service
elif [[ -x "$CACHED_BIN" ]]; then
  exec "$CACHED_BIN"
elif [[ -x "$WORKSPACE_BIN" ]]; then
  mkdir -p "$(dirname "$CACHED_BIN")"
  cp "$WORKSPACE_BIN" "$CACHED_BIN"
  exec "$CACHED_BIN"
else
  cargo build -p wallet-service
fi

exec "$CACHED_BIN"
