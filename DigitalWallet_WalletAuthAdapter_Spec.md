![KeyCortex Logo](./keycortex_logo.png)

# KeyCortex Digital Wallet + Wallet Auth Adapter

## Context & Detailed Specifications (Independent Component)

This spec assumes:

- Rust-based implementation
- Real signing capability (not mocked)
- Integration with AuthBuddy IdP, FortressDigital, ProofCortex, and FlowCortex
- Minimal but production-realistic functionality
- Extensible multi-chain adapter architecture, with MVP chain execution limited to FlowCortex
- MVP transactable assets limited to PROOF (native coin) and FloweR (native stablecoin)

---

## 1. Purpose & Positioning

### 1.1 Why This Should Be an Independent Component

The Digital Wallet + Wallet Auth Adapter should be a standalone platform because it serves two distinct but tightly coupled roles:

1. Wallet = cryptographic key custody + transaction signing
2. Auth Adapter = wallet ownership authentication (challenge–response)

Separating this from:

- IdP (human identity)
- FortressDigital (policy engine)
- FlowCortex (network ledger)

ensures clean responsibility boundaries and future extensibility.

---

## 2. Role in Overall Demo Architecture

```text
Treasury Settlement UI
        │
        ▼
 AuthBuddy IdP (user identity)
        │
        ▼
 Digital Wallet + Auth Adapter  ← (this component)
        │
        ▼
 FortressDigital Control Plane (policy + risk)
        │
        ├── ProofCortex (ZKP proofs)
        ▼
 FlowCortex L0 (anchor chain + token ledger)
```

---

## 3. Conceptual Responsibilities

### 3.1 Digital Wallet Responsibilities

- Generate / import keypair
- Secure key storage
- Sign transaction payloads
- Sign authentication challenges
- Display wallet balance (from FlowCortex)
- Submit signed transactions

### 3.2 Wallet Auth Adapter Responsibilities

- Issue authentication challenges
- Verify wallet signatures
- Bind wallet address to IdP user
- Provide wallet verification context to FortressDigital

---

## 4. Logical Sub-Modules Inside the Component

```text
wallet-service/
 ├── key_manager        (key generation + custody)
 ├── signer             (tx + challenge signing)
 ├── auth_adapter       (challenge + verification)
 ├── wallet_api         (REST/gRPC endpoints)
 ├── wallet_ui          (web wallet interface)
 └── chain_client       (chain adapter abstraction; FlowCortex adapter first)
```

---

## 5. Functional Capabilities

### 5.1 Key Custody & Signing

- Generate Ed25519 or secp256k1 keys (FlowCortex-compatible)
- Store keys encrypted (local keystore or HSM mock)
- Sign:
  - authentication challenge
  - settlement transaction payload
  - proof commitments (if required)

Security rule:

- Raw private keys must never leave wallet service boundary.

### 5.2 Wallet Authentication (Auth Adapter Mode)

Implements challenge–response authentication:

1. IdP session active
2. Wallet service requests challenge
3. Wallet signs challenge
4. Auth Adapter verifies signature
5. Wallet bound to user identity

This proves:

- User controls wallet private key + user identity verified

### 5.3 Transaction Signing & Submission

Wallet prepares:

```json
{
  "from": "wallet_address",
  "to": "counterparty_address",
  "amount": "1000",
  "asset": "FloweR",
  "chain": "flowcortex-l0"
}
```

Steps:

- FortressDigital approves policy
- Wallet signs transaction payload
- Submit to FlowCortex network

MVP transaction constraint:

- Allowed chain: `flowcortex-l0`
- Allowed assets: `PROOF`, `FloweR`
- Any other chain or asset returns validation error until corresponding chain adapter is enabled.

### 5.4 Wallet Context Enrichment

Provides signals to FortressDigital:

- wallet address
- key type (local / MPC future)
- last verification time
- signing frequency
- chain network

These feed risk scoring.

---

## 6. API Specifications

### 6.1 Wallet APIs (External)

#### `POST /wallet/create`

Creates new wallet.

Response:

```json
{
  "wallet_address": "0xABC123...",
  "public_key": "...",
  "chain": "flowcortex-l0"
}
```

#### `POST /wallet/sign`

Signs arbitrary payload.

Request:

```json
{
  "payload": "...base64...",
  "purpose": "transaction|auth|proof"
}
```

Validation notes:

- For `purpose=transaction`, payload must resolve to chain `flowcortex-l0` and asset `PROOF` or `FloweR` in MVP.

#### `GET /wallet/balance`

Fetch token balance via FlowCortex.

### 6.2 Auth Adapter APIs

#### `POST /auth/challenge`

Generates nonce challenge.

Response:

```json
{
  "challenge": "...",
  "expires_in": 120
}
```

#### `POST /auth/verify`

Verifies wallet signature.

Request:

```json
{
  "wallet_address": "...",
  "signature": "...",
  "challenge": "..."
}
```

Response:

```json
{
  "valid": true,
  "wallet_address": "...",
  "verified_at": "timestamp"
}
```

#### `POST /auth/bind`

Binds wallet to logged-in IdP user.

Header:

```http
Authorization: Bearer <AuthBuddy Token>
```

---

## 7. Integration Points with Other Components

### 7.1 Integration with AuthBuddy IdP

Purpose:

- Bind human identity ↔ wallet address

Flow:

- User logs in via AuthBuddy (OIDC)
- Wallet signs challenge
- Auth Adapter verifies
- Adapter calls IdP:

```http
POST /idp/user/wallet-binding
```

IdP stores:

```json
{
  "user_id": "vijay",
  "wallet_address": "0xABC..."
}
```

IdP token enriched with:

```json
{
  "wallet_address": "0xABC..."
}
```

### 7.2 Integration with FortressDigital

Adapter → FortressDigital provides wallet verification context:

```json
{
  "wallet_address": "...",
  "chain": "...",
  "signature_valid": true,
  "last_verified": "...",
  "user_id": "..."
}
```

Used for:

- risk scoring
- policy decision gating
- execution authorization

### 7.3 Integration with ProofCortex

Role:

- Provide input facts for ZKP circuit:
  - wallet verified
  - wallet bound to user
  - signature authenticity proof

Adapter outputs:

```text
commitment = hash(wallet_address + challenge + result)
```

ProofCortex uses this in STARK proof generation.

### 7.4 Integration with FlowCortex (Blockchain)

Wallet interacts directly:

- submit signed settlement tx
- query token balances
- anchor commitments

Flow:

- Wallet → sign tx → FlowCortex submit → ledger update

MVP scope note:

- FlowCortex is the only active chain integration for transaction submission.
- `PROOF` and `FloweR` are the only supported transaction assets in MVP.

---

## 8. Data Storage Model

### `wallet_keystore` (encrypted)

| field | description |
|---|---|
| wallet_address | derived address |
| encrypted_private_key | encrypted key blob |
| created_at | timestamp |

### `wallet_bindings`

| wallet_address | user_id | chain | last_verified |
|---|---|---|---|

### `challenge_store`

| nonce | expires_at | used |
|---|---|---|

---

## 9. UI Interfaces Required

### 9.1 Digital Wallet Web App

Features:

- create/import wallet
- view balance
- connect wallet (auth)
- sign settlement transaction
- show transaction history

### 9.2 Wallet Auth Console (Developer / Ops)

- view wallet bindings
- revoke wallet-user mapping
- view verification logs
- audit signature attempts

---

## 10. Security & Trust Model

| Layer | Guarantee |
|---|---|
| Wallet signing | private key ownership |
| IdP login | human identity verified |
| Adapter verification | cryptographic authenticity |
| FortressDigital | policy + risk compliance |
| ProofCortex | cryptographic proof of decision |
| FlowCortex | immutable settlement record |

---

## 11. Minimal MVP Scope (Realistic 10-Day Build)

Must implement:

- Key generation + signing
- Challenge + signature verification
- Wallet ↔ user binding
- Submit signed tx to FlowCortex
- REST APIs + simple web wallet UI

Optional later:

- MPC custody
- Hardware keystore
- multi-chain wallets
- zk-friendly signing attestations

Clarification:

- Multi-chain support is an architectural design goal from day one, but only FlowCortex is enabled in the MVP runtime.

---

## 12. Tech Stack (Rust)

- Axum (REST)
- Tonic (gRPC)
- ed25519-dalek / k256 (signing)
- sqlx + Postgres (bindings)
- RocksDB (keystore)
- WASM front-end wallet UI (optional)

---

## 13. Final Conceptual Summary

The Digital Wallet + Wallet Auth Adapter is an independent cryptographic identity and transaction signing platform that:

- Custodies and signs FloweR settlement transactions
- Authenticates wallet ownership via challenge–response signatures
- Binds wallet to enterprise user identity via AuthBuddy IdP
- Supplies verified wallet context to FortressDigital for policy decisions
- Anchors signed transactions and proof commitments to FlowCortex

This component enables the core vision:

A unified Zero Trust + Zero Proof settlement flow where human identity, wallet ownership, policy authorization, and cryptographic proofs converge before any stablecoin transfer is executed.
