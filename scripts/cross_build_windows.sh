#!/usr/bin/env bash
set -euo pipefail

echo "=== KeyCortex Windows Cross-Build ==="

# Check prerequisites
command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1 || {
  echo "ERROR: mingw not installed. Run:"
  echo "  sudo apt install gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64"
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
  echo "Building release (optimized)..."
  cargo build -p wallet-service --target x86_64-pc-windows-gnu --release
  BIN="target/x86_64-pc-windows-gnu/release/wallet-service.exe"
else
  echo "Building debug..."
  cargo build -p wallet-service --target x86_64-pc-windows-gnu
  BIN="target/x86_64-pc-windows-gnu/debug/wallet-service.exe"
fi

echo ""
echo "Build complete: $BIN"
ls -lh "$BIN"
echo ""
echo "To deploy on Windows:"
echo "  1. Copy wallet-service.exe to the Windows machine"
echo "  2. Copy migrations/postgres/ directory (if using Postgres)"
echo "  3. Copy ui/wallet-baseline/ directory (for frontend)"
echo "  4. Set environment variables (see KeyCortex_DevOps_Guide.md §7)"
echo "  5. Run: wallet-service.exe"
