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
- [x] Freeze v0.1 API contracts for wallet/auth endpoints

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
- [x] Add secp256k1 support path (feature-gated)
- [x] Implement encrypted private key storage interface (in-memory MVP)
- [x] Enforce “private keys never leave service boundary” (service-side key use)
- [x] Add purpose-tagged signing (`auth`, `transaction`, `proof`)
- [x] Add zeroization for sensitive memory

### D) Auth Adapter Flow

- [x] Implement `POST /auth/challenge` (nonce + TTL + single use)
- [x] Implement `POST /auth/verify` (signature validation)
- [x] Implement `POST /auth/bind` (IdP token-based wallet-user binding)
- [x] Persist challenge lifecycle (`issued`, `used`, `expired`) (in-service store)
- [x] Persist wallet binding audit log

### E) Chain Integration (MVP)

- [x] Define `ChainAdapter` trait in `kc-chain-client`
- [x] Implement FlowCortex adapter in `kc-chain-flowcortex`
- [x] Implement `GET /wallet/balance` via FlowCortex
- [x] Implement transaction submit path via FlowCortex
- [x] Enforce runtime allowlist:
  - [x] chain = `flowcortex-l1`
  - [x] assets = `PROOF`, `FloweR`

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
  - [x] create/import wallet
  - [x] connect wallet (auth)
  - [x] view balance
  - [x] sign settlement transaction
  - [x] view tx history
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
- [x] Integration tests for wallet/auth REST APIs
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
- [x] Add FlowCortex submit transaction endpoint with MVP allowlist checks
- [x] Add readiness check for external AuthBuddy JWKS reachability status
- [x] Add request idempotency key handling for `/wallet/submit`
- [x] Add tx replay protection nonce model for `/wallet/submit`
- [x] Persist idempotency records beyond process restart
- [x] Add `/wallet/nonce` query endpoint for client-side nonce discovery
- [x] Modularize submit/nonce features out of `wallet-service` main module
- [x] Split auth and ops handlers into dedicated modules
- [x] Add transaction status/read API for submitted tx hashes
- [x] Add tx-status polling integration to FlowCortex adapter
- [x] Add Postgres-backed audit logs for challenge/verify/bind events
- [x] Add Postgres migrations and DB repository layer
- [x] Wire Postgres-backed repositories into auth and ops handlers
- [x] Add startup migration runner for Postgres SQL files
- [x] Add fallback mode metrics for RocksDB vs Postgres path
- [x] Add structured startup report for Postgres migration status
- [x] Add health/readiness counters for DB fallback events
- [x] Add startup diagnostics endpoint for dependency details
- [x] Add API contract docs for `/startupz` diagnostics endpoint
- [x] Add lightweight smoke tests for DB fallback behavior
- [x] Add integration tests for Postgres-backed auth/ops flows
- [x] Add endpoint examples for `/startupz` in integration docs
- [x] Add CI hook to run fallback smoke test against staging service
- [x] Add CI job for Postgres integration tests with `TEST_DATABASE_URL`
- [x] Add local runbook section for Postgres test setup
- [x] Add release gate checklist for diagnostics endpoints and DB fallback
- [x] Add operations troubleshooting section for fallback counter spikes
- [x] Add compact operator dashboard spec for diagnostics endpoint fields
- [x] Add alerting threshold recommendations for fallback counters
- [x] Add premium wallet-shaped web UI baseline with functional create/connect/balance/sign/history screens

### Next Up

- [ ] Build wallet UI baseline screens (create/connect/balance/sign/history)
- [ ] Implement E2E happy path: login → bind wallet → sign tx → submit
- [ ] Run wallet-service test suite in environment with `cargo` available

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
- 2026-02-25: Added `POST /wallet/submit` with local tx signing + FlowCortex adapter submission and strict MVP chain/asset enforcement.
- 2026-02-25: Extended `/readyz` with external AuthBuddy JWKS reachability signal and URL dependency readiness gating.
- 2026-02-25: Added `Idempotency-Key` support for `/wallet/submit` with cached response replay to prevent accidental duplicate submissions.
- 2026-02-25: Added per-wallet strictly increasing nonce checks in `/wallet/submit` to reject replayed or stale transaction submissions.
- 2026-02-25: Persisted submit idempotency and wallet nonce records in RocksDB and added `/wallet/nonce` endpoint for client nonce discovery.
- 2026-02-25: Refactored submit and nonce handlers into `services/wallet-service/src/submit.rs` to keep `main.rs` focused and smaller.
- 2026-02-25: Split auth and ops handlers into `services/wallet-service/src/auth.rs` and `services/wallet-service/src/ops.rs`, including JWT principal parsing and ops access checks.
- 2026-02-25: Added persisted submitted-transaction records and `GET /wallet/tx/{tx_hash}` status/read endpoint.
- 2026-02-25: Added FlowCortex transaction-status polling via chain adapter and wired `GET /wallet/tx/{tx_hash}` to refresh and persist latest status before responding.
- 2026-02-25: Added Postgres schema migration SQL (`wallet_bindings`, `challenge_store`, `verification_logs`) and a `PostgresRepository` for bindings/challenges/audit logs.
- 2026-02-25: Wired optional Postgres integration via `DATABASE_URL`; auth and ops handlers now read/write Postgres when configured, with RocksDB fallback behavior retained.
- 2026-02-25: Added startup Postgres migration runner (`KEYCORTEX_POSTGRES_MIGRATIONS_DIR`, default `./migrations/postgres`) that applies ordered SQL files on service boot.
- 2026-02-25: Added storage backend visibility to `/health` and `/readyz` (`rocksdb-only` vs `postgres+rocksdb`) to surface fallback mode at runtime.
- 2026-02-25: Added structured Postgres startup report in `/health` and `/readyz` including configuration status, migration directory, applied migration file count, and startup error details.
- 2026-02-25: Added DB fallback counters in `/health` and `/readyz`, plus runtime counter increments for Postgres failures with RocksDB fallback in auth/ops paths.
- 2026-02-25: Added `GET /startupz` startup diagnostics endpoint with consolidated auth/JWKS, Postgres startup, storage mode, and DB fallback counter visibility.
- 2026-02-25: Documented `/startupz` diagnostics API contract in root README and digital wallet specification.
- 2026-02-25: Added executable smoke test script `scripts/smoke_db_fallback.sh` for `/health`, `/readyz`, and `/startupz` fallback diagnostics validation.
- 2026-02-25: Added environment-gated Postgres integration tests for wallet binding, audit log filtering, and challenge lifecycle persistence in `services/wallet-service/src/db.rs`.
- 2026-02-25: Added concrete `/startupz` curl usage and sample response examples in README and wallet/auth specification docs.
- 2026-02-25: Added GitHub Actions workflow `.github/workflows/wallet-service-ci.yml` with Postgres integration test job and dispatchable staging fallback smoke-test hook.
- 2026-02-25: Added local runbook steps in README for Postgres test setup, integration test execution, and diagnostics smoke checks.
- 2026-02-25: Added release gate checklist and fallback-counter troubleshooting guidance in README for operations handoff.
- 2026-02-25: Added dedicated operator dashboard specification for `/health`, `/readyz`, and `/startupz` diagnostics views and alerting rules.
- 2026-02-25: Added concrete fallback counter threshold recommendations and paging guidance in README.
- 2026-02-25: Added sustained Postgres degradation operations playbook and `/startupz` field ownership matrix for on-call handoff.
- 2026-02-25: Added feature-gated secp256k1 signing support in `kc-crypto` and zeroization hardening for decrypted key material across wallet/auth/submit flows.
- 2026-02-25: Froze v0.1 wallet/auth API contracts in `KeyCortex_API_v0.1_Contract.md` and aligned auth verify response field naming (`verified_at_epoch_ms`) in architecture spec.
- 2026-02-25: Added wallet/auth REST integration tests in `wallet-service` for create/sign, challenge/verify, bind auth, submit/nonce/tx-status contract coverage.
- 2026-02-25: Added `ui/wallet-baseline` premium wallet-shaped web UI with tabbed screens for create/bind/balance/sign/tx lookup and wired live calls to wallet-service endpoints.

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
