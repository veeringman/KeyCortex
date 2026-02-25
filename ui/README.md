<img src="../keycortex_logo.png" alt="KeyCortex Logo" width="180" />

# UI Asset Baseline

This folder contains shared UI placeholder assets for networks and coins.

## Current Purpose

- Provide placeholder logos/icons for wallet UI surfaces now.
- Keep a stable path contract so final uploaded brand icons can replace placeholders later.
- Keep one centralized icon manifest for desktop, web, and mobile rendering parity.

## Canonical Manifest

- Path: `ui/config/icon-manifest.json`

## Shared Icon Resolver

- Path: `ui/shared/icon-resolver.ts`
- Example: `ui/shared/icon-resolver.example.ts`

Use this module to resolve network/coin icons with fallback behavior and MVP constraint checks.

Core functions:

- `resolveNetworkIcon(manifest, chain)`
- `resolveCoinIcon(manifest, asset)`
- `resolveWalletVisuals(manifest, chain, asset)`
- `isMvpChainAllowed(manifest, chain)`
- `isMvpAssetAllowed(manifest, asset)`

## Placeholder Asset Folders

- Networks: `ui/assets/icons/networks/`
- Coins: `ui/assets/icons/coins/`
- Fallbacks: `ui/assets/icons/common/`

## Swap Process (Later)

1. Replace SVG files with final logos while keeping the same file names, or
2. Add new files and update `ui/config/icon-manifest.json` icon paths.

No UI business logic should hardcode icon paths outside the manifest.

## MVP Constraints Reference

- Active chain: `flowcortex-l1`
- Active assets: `PROOF`, `FloweR`

Other network/coin placeholders are pre-created for future expansion.
