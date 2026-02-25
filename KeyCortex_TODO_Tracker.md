<img src="./keycortex_logo.png" alt="KeyCortex Logo" width="180" />

# KeyCortex TODO Tracker (Alive)

Last Updated: 2026-02-25
Owner: KeyCortex Team

Purpose: Persistent progress tracker to survive crashes, container restarts, and context resets.

---

## Status Legend

- [ ] Not started
- [~] In progress
- [x] Completed
- [!] Blocked

---

## Current Phase

Phase: Foundation & Architecture

---

## Master TODO List

### A) Product & Architecture

- [x] Capture Digital Wallet + Wallet Auth Adapter detailed specification
- [x] Decide cross-platform Rust-first tech stack (desktop/web/mobile)
- [x] Define multi-chain extensible approach with FlowCortex-first MVP
- [x] Define MVP transaction scope to `flowcortex-l0` + assets `PROOF` and `FloweR`
- [x] Finalize repository-wide folder structure for Rust workspace
- [ ] Freeze v0.1 API contracts for wallet/auth endpoints

### B) Rust Workspace Bootstrap

- [x] Create Rust workspace root (`Cargo.toml`) and standard project layout
- [x] Create shared crates:
  - [x] `kc-crypto`
  - [x] `kc-wallet-core`
  - [x] `kc-storage`
  - [x] `kc-chain-client`
  - [x] `kc-chain-flowcortex`
  - [x] `kc-auth-adapter`
  - [x] `kc-api-types`
- [x] Create `services/wallet-service` with Axum skeleton
- [x] Add baseline lint/format settings (`rustfmt`, `clippy`)

### C) Security & Signing Core

- [ ] Implement key generation for Ed25519
- [ ] Add secp256k1 support path (feature-gated)
- [ ] Implement encrypted private key storage interface
- [ ] Enforce “private keys never leave service boundary”
- [ ] Add purpose-tagged signing (`auth`, `transaction`, `proof`)
- [ ] Add zeroization for sensitive memory

### D) Auth Adapter Flow

- [ ] Implement `POST /auth/challenge` (nonce + TTL + single use)
- [ ] Implement `POST /auth/verify` (signature validation)
- [ ] Implement `POST /auth/bind` (IdP token-based wallet-user binding)
- [ ] Persist challenge lifecycle (`issued`, `used`, `expired`)
- [ ] Persist wallet binding audit log

### E) Chain Integration (MVP)

- [x] Define `ChainAdapter` trait in `kc-chain-client`
- [x] Implement FlowCortex adapter in `kc-chain-flowcortex`
- [ ] Implement `GET /wallet/balance` via FlowCortex
- [ ] Implement transaction submit path via FlowCortex
- [ ] Enforce runtime allowlist:
  - [ ] chain = `flowcortex-l0`
  - [ ] assets = `PROOF`, `FloweR`

### F) API & Data Layer

- [x] Implement `POST /wallet/create` (skeleton)
- [x] Implement `POST /wallet/sign` (skeleton)
- [x] Define shared request/response DTOs (initial)
- [ ] Add Postgres migrations for:
  - [ ] `wallet_bindings`
  - [ ] `challenge_store`
  - [ ] `verification_logs`
- [ ] Add RocksDB keystore persistence

### G) UI & Client Surfaces

- [x] Add placeholder network and coin icons + centralized icon manifest
- [x] Add shared icon resolver module with fallback + MVP checks
- [ ] Desktop shell baseline (Tauri)
- [ ] Web wallet baseline (Next.js)
- [ ] Mobile bridge baseline (UniFFI generation)
- [ ] Wallet UI screens:
  - [ ] create/import wallet
  - [ ] connect wallet (auth)
  - [ ] view balance
  - [ ] sign settlement transaction
  - [ ] view tx history
- [ ] Ops/Auth console screens:
  - [ ] view wallet bindings
  - [ ] revoke binding
  - [ ] view verification logs

### H) Integrations

- [ ] AuthBuddy integration: wallet binding callback contract
- [ ] FortressDigital context payload integration
- [ ] ProofCortex commitment payload generation
- [ ] FlowCortex final settlement + anchor flow verification

### I) Quality Gates

- [ ] Unit tests for crypto/signing and challenge flow
- [ ] Integration tests for wallet/auth REST APIs
- [ ] E2E happy path: login → bind wallet → sign tx → submit
- [ ] Security checks (nonce replay, invalid sig, expired challenge)
- [ ] Release checklist for v0.1 MVP

---

## Active Sprint Board

### In Progress

- [ ] None currently

### Completed (Recent)

- [x] Prepare persistent TODO tracker and operating process
- [x] Add placeholder network/coin icon pack and UI icon manifest
- [x] Add shared icon resolver module for network/coin icon lookup
- [x] Scaffold Rust workspace, shared crates, and wallet-service baseline
- [x] Add initial wallet/auth API skeleton routes in Axum

### Next Up

- [ ] Add FlowCortex balance and submit transaction endpoints
- [ ] Implement challenge issuance, verify, and bind endpoint contracts
- [ ] Replace placeholder signer with real Ed25519 signing flow

### Blockers

- [ ] None currently

---

## Change Log

- 2026-02-25: Created tracker with baseline scope, phases, and actionable breakdown.
- 2026-02-25: Marked architecture/stack/spec decisions as completed based on existing docs.
- 2026-02-25: Added UI placeholder icons for major networks/coins and a centralized icon manifest with MVP FlowCortex + PROOF/FloweR constraints.
- 2026-02-25: Added shared icon resolver module for consistent icon path resolution, fallback handling, and MVP allowlist checks.
- 2026-02-25: Scaffolded Rust workspace with core crates, chain adapter interfaces, FlowCortex adapter baseline, and Axum wallet-service starter.
- 2026-02-25: Added wallet/auth API skeleton endpoints (`/wallet/create`, `/wallet/sign`, `/wallet/balance`, `/auth/challenge`, `/auth/verify`, `/auth/bind`) with MVP FlowCortex + PROOF/FloweR guardrails.

---

## Update Protocol (Use Every Session)

1. Update `Last Updated` date.
2. Move items between `In Progress`, `Completed`, `Blocked`.
3. Add a one-line entry in `Change Log` for every meaningful update.
4. Keep `Next Up` limited to top 3 priorities.
5. Never delete completed items; only mark `[x]` for audit continuity.

---

## Recovery Protocol (After Crash/Restart)

1. Open this tracker first.
2. Resume from `In Progress`; if empty, start from first item under `Next Up`.
3. Validate scope constraints before coding:
   - MVP chain: `flowcortex-l0`
   - MVP assets: `PROOF`, `FloweR`
4. Continue updating this file at end of each work block.
