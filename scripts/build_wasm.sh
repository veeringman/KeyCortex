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
wasm-pack build "$CRATE" --target web $MODE --out-dir "$CRATE/pkg" --no-typescript

echo "✓ WASM build complete → $CRATE/pkg/"
echo "  Serve with: python3 -m http.server 4173   (from repo root)"
echo "  Open:       http://127.0.0.1:4173/ui/wallet-wasm/"
