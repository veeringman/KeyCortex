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
- [x] Define MVP transaction scope to `flowcortex-l1` + assets `PROOF` and `FloweR`
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

- [x] Implement key generation for Ed25519
- [ ] Add secp256k1 support path (feature-gated)
- [x] Implement encrypted private key storage interface (in-memory MVP)
- [x] Enforce “private keys never leave service boundary” (service-side key use)
- [x] Add purpose-tagged signing (`auth`, `transaction`, `proof`)
- [ ] Add zeroization for sensitive memory

### D) Auth Adapter Flow

- [x] Implement `POST /auth/challenge` (nonce + TTL + single use)
- [x] Implement `POST /auth/verify` (signature validation)
- [x] Implement `POST /auth/bind` (IdP token-based wallet-user binding)
- [x] Persist challenge lifecycle (`issued`, `used`, `expired`) (in-service store)
- [x] Persist wallet binding audit log

### E) Chain Integration (MVP)

- [x] Define `ChainAdapter` trait in `kc-chain-client`
- [x] Implement FlowCortex adapter in `kc-chain-flowcortex`
- [ ] Implement `GET /wallet/balance` via FlowCortex
- [ ] Implement transaction submit path via FlowCortex
- [ ] Enforce runtime allowlist:
  - [ ] chain = `flowcortex-l1`
  - [ ] assets = `PROOF`, `FloweR`

### F) API & Data Layer

- [x] Implement `POST /wallet/create` (skeleton)
- [x] Implement `POST /wallet/sign` (wallet-address scoped signing)
- [x] Define shared request/response DTOs (initial)
- [ ] Add Postgres migrations for:
  - [ ] `wallet_bindings`
  - [ ] `challenge_store`
  - [ ] `verification_logs`
- [x] Add RocksDB keystore persistence

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
- [x] Replace placeholder signing with real Ed25519 signing in wallet-service
- [x] Add per-wallet encrypted key custody for create/sign flow
- [x] Replace in-memory keystore with RocksDB-backed persistence
- [x] Add auth challenge TTL + one-time-use enforcement
- [x] Implement real cryptographic auth signature verification
- [x] Persist `/auth/bind` wallet bindings and audit events
- [x] Add ops read endpoints for bindings and audit logs
- [x] Add ops endpoint auth guard and role checks
- [x] Replace static auth parsing with AuthBuddy JWT validation
- [x] Enforce AuthBuddy JWT `exp`/`iss`/`aud` claim checks
- [x] Add AuthBuddy JWKS/RS256 verification mode with HS256 fallback
- [x] Add periodic JWKS refresh and key rotation handling
- [x] Add HTTPS JWKS fetch with retry/backoff cache refresh
- [x] Add startup/health auth mode and JWKS status visibility
- [x] Add `/readyz` dependency checks for keystore and auth readiness

### Next Up

- [ ] Add FlowCortex balance and submit transaction endpoints
- [ ] Add Postgres-backed audit logs for challenge/verify/bind events
- [ ] Add readiness check for external AuthBuddy JWKS reachability status

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
- 2026-02-25: Implemented real Ed25519 signing path (`kc-crypto::Ed25519Signer`) and wired `/wallet/sign` to sign decoded base64 payloads with purpose-domain separation.
- 2026-02-25: Added per-wallet encrypted key custody in `wallet-service`; `/wallet/create` stores encrypted private key material and `/wallet/sign` requires `wallet_address` to sign with that wallet key.
- 2026-02-25: Migrated MVP chain references from `flowcortex-l0` to `flowcortex-l1` across code, config, and docs; added FlowCortex-facing L1 change request parameter document.
- 2026-02-25: Replaced in-memory keystore with RocksDB-backed persistence and added `KEYCORTEX_KEYSTORE_PATH` runtime path configuration.
- 2026-02-25: Added challenge store lifecycle controls (`issued`, `used`, `expired`) with TTL and one-time-use enforcement for `/auth/challenge` and `/auth/verify`.
- 2026-02-25: Replaced placeholder `/auth/verify` logic with real Ed25519 verification against wallet-custodied keys and auth-domain payload verification.
- 2026-02-25: Added `/auth/bind` persistence and audit logging via RocksDB (`wallet_binding` + `audit` records) with success/denied event outcomes.
- 2026-02-25: Added ops read endpoints for binding lookup and filtered audit log retrieval backed by RocksDB.
- 2026-02-25: Added ops endpoint access guard requiring bearer principal + `x-role: ops-admin`, with success/denied access auditing.
- 2026-02-25: Integrated AuthBuddy JWT signature validation (HS256) for `/auth/bind` and `/ops/*` authorization, replacing raw bearer principal parsing.
- 2026-02-25: Hardened AuthBuddy JWT validation by enforcing `exp` and optional `iss`/`aud` checks via `AUTHBUDDY_JWT_ISSUER` and `AUTHBUDDY_JWT_AUDIENCE`.
- 2026-02-25: Added AuthBuddy RS256 verification using `AUTHBUDDY_JWKS_JSON` (kid-based JWK selection) with HS256 dev fallback when JWKS is not configured.
- 2026-02-25: Added periodic JWKS refresh from `AUTHBUDDY_JWKS_PATH` with live cache replacement and key rotation support (`AUTHBUDDY_JWKS_REFRESH_SECONDS`).
- 2026-02-25: Added `AUTHBUDDY_JWKS_URL` fetch path with retry/backoff and file fallback for resilient JWKS cache refresh.
- 2026-02-25: Extended `/health` to expose auth mode and JWKS runtime status (`source`, `loaded`, `last_refresh`, `last_error`).
- 2026-02-25: Added `/readyz` endpoint returning readiness status for keystore and auth dependency state.

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
  - MVP chain: `flowcortex-l1`
   - MVP assets: `PROOF`, `FloweR`
4. Continue updating this file at end of each work block.
