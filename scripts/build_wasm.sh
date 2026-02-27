#!/usr/bin/env bash
# Build the wallet-wasm crate for the browser.
# Usage: ./scripts/build_wasm.sh [--release]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/ui/wallet-wasm"

MODE="--dev"
if [[ "${1:-}" == "--release" ]]; then
  MODE="--release"
fi

echo "▸ Building wallet-wasm ($MODE)…"

# Auto-install wasm-pack if missing
if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "▸ wasm-pack not found — installing…"
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Ensure wasm32 target is installed
rustup target add wasm32-unknown-unknown 2>/dev/null || true

wasm-pack build "$CRATE" --target web $MODE --out-dir "$CRATE/pkg" --no-typescript

echo "✓ WASM build complete → $CRATE/pkg/"
echo "  Serve with: python3 -m http.server 4173   (from repo root)"
echo "  Open:       http://127.0.0.1:4173/ui/wallet-wasm/"
