# KeyCortex Cross-Platform Tech Stack Decision

## Goal
Support KeyCortex Wallet on major desktop, web, and mobile platforms with a Rust-first architecture, real cryptographic signing, and an extensible multi-chain model.

---

## 1) Core Architecture (Rust-first)

Use a Rust workspace with shared crates consumed by all platform shells.

```text
keycortex/
 ├── crates/
 │   ├── kc-crypto        (keys, signing, verification, hashing)
 │   ├── kc-wallet-core   (wallet domain logic, challenge flow, tx payloads)
 │   ├── kc-storage       (keystore abstraction + encrypted persistence)
 │   ├── kc-chain-client  (chain trait + routing abstraction)
 │   ├── kc-chain-flowcortex (FlowCortex implementation)
 │   ├── kc-auth-adapter  (challenge/verify/bind logic)
 │   └── kc-api-types     (shared DTOs/contracts)
 ├── services/
 │   └── wallet-service   (Axum + Tonic)
 ├── apps/
 │   ├── desktop          (Tauri 2)
 │   ├── web              (Next.js + WASM client)
 │   └── mobile           (iOS/Android shells using UniFFI)
```

This keeps signing/auth logic in one audited Rust codebase while UIs are platform-specific.

---

## 1.1) Blockchain Extensibility Strategy

- Define a `ChainAdapter` trait in `kc-chain-client` for:
  - balance query
  - transaction build/submit
  - commitment anchoring
  - chain-specific address/asset validation
- Start with `kc-chain-flowcortex` as the only enabled adapter.
- Keep adapter registration config-driven so additional networks can be added without changing wallet domain logic.
- Enforce MVP allowlist at runtime:
  - chain = `flowcortex-l0`
  - assets = `PROOF`, `FloweR`

---

## 2) Chosen Tech Stack

### Rust Core (shared everywhere)

- Rust stable (edition 2024)
- Workspace + feature flags per platform (`desktop`, `web`, `mobile`, `server`)
- `serde`, `thiserror`, `anyhow`, `tracing`

### Cryptography

- `ed25519-dalek` (primary signing path)
- `k256` (secp256k1 compatibility path)
- `sha2`, `blake3` (hashing/commitments)
- `rand_core` / `rand`
- `zeroize` (wipe secret material)
- `base64` / `hex` for transport encoding

### Wallet Service (backend/API)

- `axum` (REST)
- `tonic` (gRPC)
- `tower` + `tower-http` (middleware/CORS/trace)
- `sqlx` + Postgres (bindings, challenges, verification logs)
- `rocksdb` (encrypted keystore blobs)
- `tokio` runtime

### Desktop

- Tauri 2 (Windows, macOS, Linux)
- Rust commands from Tauri invoke shared `kc-wallet-core`
- Web UI layer: React + TypeScript in Tauri frontend

### Web

- Next.js (wallet web app + auth console)
- Rust-to-WASM package for client-side verify/sign helpers where needed
  - `wasm-bindgen`, `wasm-pack`, `web-sys`
- Server-side signing remains in wallet-service boundary for high-trust mode

### Mobile

- Native apps with shared Rust core via `UniFFI`
  - iOS shell: Swift/SwiftUI
  - Android shell: Kotlin/Jetpack Compose
- Use OS secure storage wrappers for key encryption keys
  - iOS Keychain/Secure Enclave (where available)
  - Android Keystore/StrongBox (where available)

---

## 3) Why This Stack

- **Single cryptographic source of truth:** signing/verification in Rust, reused across all clients.
- **Production-realistic custody:** encrypted key blobs + platform keystore integration.
- **Fast MVP path:** Axum + Tauri + Next.js are low-friction and well-supported.
- **Scales to enterprise controls:** easy integration with AuthBuddy, FortressDigital, ProofCortex, FlowCortex.
- **Future-proofing:** can add MPC/HSM provider behind `kc-storage`/`kc-crypto` traits later.

---

## 4) Platform Support Matrix

| Platform | Delivery | Core Logic Source |
|---|---|---|
| Windows/macOS/Linux | Tauri desktop app | Rust shared crates |
| Web (Chrome/Edge/Safari/Firefox) | Next.js app (+ optional WASM helper) | Rust service + Rust WASM |
| iOS | Native Swift shell + UniFFI bridge | Rust shared crates |
| Android | Native Kotlin shell + UniFFI bridge | Rust shared crates |

---

## 5) MVP Build Scope (first implementation)

1. Build `wallet-service` (Axum REST + challenge/verify + sign tx + balance query).
2. Implement encrypted keystore in `kc-storage`.
3. Integrate only `flowcortex-l0` for transaction execution in MVP.
4. Restrict transactable assets to `PROOF` and `FloweR` in MVP.
5. Ship desktop first with Tauri (fastest real wallet UX).
6. Ship web app against same backend APIs.
7. Generate UniFFI bindings and deliver minimal iOS/Android wallet shells.

---

## 6) Non-Negotiable Security Rules

- Raw private keys never leave Rust wallet boundary.
- Challenge nonce is single-use + short TTL.
- Signature verification logs are tamper-evident and timestamped.
- All signing actions are purpose-tagged (`auth`, `transaction`, `proof`).
- Key material is zeroized in memory where practical.

---

## 7) Final Selection Summary

**Selected stack:**
Rust core (`ed25519-dalek`/`k256`, `axum`, `tonic`, `sqlx`, `rocksdb`) +
Tauri desktop + Next.js web + native mobile shells (Swift/Kotlin) through UniFFI.

Delivery strategy is multi-chain extensible by design, but MVP chain execution is intentionally scoped to FlowCortex with `PROOF` and `FloweR` only.
