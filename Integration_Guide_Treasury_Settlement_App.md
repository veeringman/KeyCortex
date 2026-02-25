# Treasury Settlement App — KeyCortex Integration Guide

> **Version:** 1.0 · **Last updated:** 2025-01-XX  
> **Audience:** Treasury Settlement App team — frontend/backend engineers  
> **Scope:** End-to-end wallet lifecycle, transaction flow, auth flow, platform orchestration

---

## 1. Overview

The Treasury Settlement App is the **primary client** of the KeyCortex ecosystem. It orchestrates the full wallet lifecycle by calling KeyCortex, AuthBuddy, FortressDigital, and ProofCortex in the correct sequence.

```
┌─────────────────┐
│  Treasury App   │
│  (UI + Backend) │
└────┬────────────┘
     │
     ├──▶ KeyCortex (wallet ops, signing, submit)
     ├──▶ AuthBuddy (login, JWT tokens)
     ├──▶ FortressDigital (risk check before actions)
     └──▶ ProofCortex (commitment for ZKP proof generation)
```

---

## 2. KeyCortex API Base URL

| Environment | URL |
|-------------|-----|
| Local dev | `http://localhost:8080` |
| Container | `http://wallet-service:8080` |
| Production | Configure via env var |

All endpoints return JSON. Error responses are:
```json
{ "error": "<message>" }
```

---

## 3. Complete Endpoint Reference

### 3.1 Wallet Management

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `POST /wallet/create` | POST | None | Create new wallet (random or passphrase-derived) |
| `GET /wallet/list` | GET | None | List all wallets with labels and bindings |
| `POST /wallet/restore` | POST | None | Restore wallet from passphrase |
| `POST /wallet/rename` | POST | None | Rename wallet label |
| `POST /wallet/sign` | POST | None | Sign arbitrary payload |
| `GET /wallet/balance?wallet_address=&asset=&chain=` | GET | None | Query wallet balance |
| `GET /wallet/nonce?wallet_address=` | GET | None | Get next nonce for wallet |
| `POST /wallet/submit` | POST | None | Submit signed transaction to chain |
| `GET /wallet/tx/{tx_hash}` | GET | None | Query transaction status |

### 3.2 Authentication

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `POST /auth/challenge` | POST | None | Request a challenge string |
| `POST /auth/verify` | POST | None | Verify signed challenge |
| `POST /auth/bind` | POST | Bearer JWT | Bind wallet to IdP user |

### 3.3 Platform Integration

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `POST /fortressdigital/wallet-status` | POST | None | Get wallet risk signals |
| `POST /fortressdigital/context` | POST | None | Get signed context payload |
| `POST /proofcortex/commitment` | POST | None | Generate ZKP commitment hash |
| `GET /chain/config` | GET | None | Get chain configuration |

### 3.4 Operations (Requires `ops-admin` JWT)

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `GET /ops/bindings/{wallet_address}` | GET | Bearer JWT + `ops-admin` | Query wallet binding |
| `GET /ops/audit` | GET | Bearer JWT + `ops-admin` | Search audit trail |

### 3.5 Health & Diagnostics

| Endpoint | Method | Description |
|----------|--------|-------------|
| `GET /health` | GET | Full service health |
| `GET /readyz` | GET | Readiness probe |
| `GET /startupz` | GET | Startup diagnostics |
| `GET /version` | GET | Service version |

---

## 4. Flow 1: Wallet Creation

### 4.1 Create New Wallet (Random Key)

```bash
POST /wallet/create
Content-Type: application/json

{
  "label": "My Treasury Wallet"
}
```

Response:
```json
{
  "wallet_address": "0xabc123...",
  "public_key": "64-char-hex-ed25519-public-key",
  "chain": "flowcortex-l1",
  "label": "My Treasury Wallet"
}
```

### 4.2 Create Wallet from Passphrase (Deterministic)

```bash
POST /wallet/create
Content-Type: application/json

{
  "label": "Recovery Wallet",
  "passphrase": "my secret recovery phrase"
}
```

Same response. **Same passphrase always produces the same wallet address.** If the wallet already exists, returns the existing wallet.

### 4.3 Restore Wallet from Passphrase

```bash
POST /wallet/restore
Content-Type: application/json

{
  "passphrase": "my secret recovery phrase",
  "label": "Restored Wallet"
}
```

Response:
```json
{
  "wallet_address": "0xabc123...",
  "public_key": "...",
  "chain": "flowcortex-l1",
  "label": "Restored Wallet",
  "already_existed": true
}
```

### 4.4 Rename Wallet

```bash
POST /wallet/rename
Content-Type: application/json

{
  "wallet_address": "0xabc123...",
  "label": "New Name"
}
```

### 4.5 List All Wallets

```bash
GET /wallet/list
```

Response:
```json
{
  "wallets": [
    {
      "wallet_address": "0xabc123...",
      "chain": "flowcortex-l1",
      "bound_user_id": "user-a1b2c3d4",
      "public_key": "64-char-hex",
      "label": "My Treasury Wallet"
    }
  ],
  "total": 1
}
```

---

## 5. Flow 2: Auth Challenge → Verify → Bind

This is the **identity proof flow** — proves the user controls a wallet and binds it to their IdP identity.

### Step 1: Get Auth Challenge

```bash
POST /auth/challenge

# No request body needed
```

Response:
```json
{
  "challenge": "550e8400-e29b-41d4-a716-446655440000",
  "expires_in": 300
}
```

Challenge is a UUID, valid for 5 minutes, single-use.

### Step 2: Sign the Challenge

```bash
POST /wallet/sign
Content-Type: application/json

{
  "wallet_address": "0xabc123...",
  "payload": "550e8400-e29b-41d4-a716-446655440000",
  "purpose": "auth"
}
```

Response:
```json
{
  "signature": "128-char-hex-ed25519-signature"
}
```

### Step 3: Verify Signature

```bash
POST /auth/verify
Content-Type: application/json

{
  "wallet_address": "0xabc123...",
  "challenge": "550e8400-e29b-41d4-a716-446655440000",
  "signature": "128-char-hex-ed25519-signature"
}
```

Response:
```json
{
  "valid": true,
  "wallet_address": "0xabc123...",
  "verified_at_epoch_ms": 1706140800000
}
```

### Step 4: Bind Wallet to User (Requires AuthBuddy JWT)

```bash
POST /auth/bind
Authorization: Bearer <authbuddy-jwt>
Content-Type: application/json

{
  "wallet_address": "0xabc123...",
  "chain": "flowcortex-l1"
}
```

Response:
```json
{
  "bound": true,
  "user_id": "user-a1b2c3d4",
  "wallet_address": "0xabc123...",
  "chain": "flowcortex-l1",
  "bound_at_epoch_ms": 1706140800000
}
```

The `user_id` is extracted from the JWT `sub` claim. AuthBuddy receives an async callback.

---

## 6. Flow 3: Transaction Submission

### Step 1: Get Current Nonce

```bash
GET /wallet/nonce?wallet_address=0xabc123...
```

Response:
```json
{
  "wallet_address": "0xabc123...",
  "last_nonce": 5,
  "next_nonce": 6
}
```

### Step 2: Submit Transaction

```bash
POST /wallet/submit
Content-Type: application/json
Idempotency-Key: unique-request-id-123

{
  "from": "0xabc123...",
  "to": "0xdef456...",
  "amount": "1000000000000000000",
  "asset": "PROOF",
  "chain": "flowcortex-l1",
  "nonce": 6
}
```

Response:
```json
{
  "accepted": true,
  "tx_hash": "pending-integration",
  "signature": "128-char-hex-signature"
}
```

**Mandatory fields:** `from`, `to`, `amount`, `asset`, `chain`, `nonce`

**Validation rules:**
- `nonce` must be > 0 and strictly greater than last used nonce
- `chain` must be `"flowcortex-l1"`
- `asset` must be `"PROOF"` or `"FloweR"`
- `from` must be a wallet in KeyCortex keystore

**Idempotency:** If you send the same `Idempotency-Key`, you get the same response without re-submitting.

### Step 3: Check Transaction Status

```bash
GET /wallet/tx/pending-integration
```

Response:
```json
{
  "tx_hash": "pending-integration",
  "status": "submitted",
  "accepted": true,
  "chain": "flowcortex-l1",
  "from": "0xabc123...",
  "to": "0xdef456...",
  "asset": "PROOF",
  "amount": "1000000000000000000",
  "submitted_at_epoch_ms": 1706140800000
}
```

---

## 7. Flow 4: Full Orchestration (Recommended Sequence)

This is the **recommended end-to-end flow** for a treasury transfer with all platform checks:

```
1. USER AUTHENTICATION
   a. User logs into AuthBuddy → receives JWT
   b. If no wallet exists:
      POST /wallet/create { label: "My Wallet" }
   c. Challenge-verify-bind flow (§5) to link wallet to identity

2. PRE-TRANSFER RISK CHECK
   a. POST /fortressdigital/wallet-status
      { wallet_address, chain: "flowcortex-l1" }
   b. Check risk_signals:
      - wallet_not_found → abort
      - wallet_not_bound → require binding first
      - verification_stale_24h → re-verify (challenge + verify flow)
      - never_verified → require first verification

3. GENERATE PROOF COMMITMENT (Optional — for compliance)
   a. POST /proofcortex/commitment
      { wallet_address, challenge, verification_result: true, chain }
   b. Store commitment hash for audit trail

4. EXECUTE TRANSFER
   a. GET /wallet/nonce?wallet_address=0x...
   b. POST /wallet/submit { from, to, amount, asset, chain, nonce }
   c. GET /wallet/tx/{tx_hash} (poll for confirmation)

5. POST-TRANSFER AUDIT
   a. POST /fortressdigital/context
      { wallet_address, user_id, chain, session_id, context_data }
   b. Store signed context payload
```

---

## 8. Assets

| Symbol | Type | Decimals | Fee Payment | Description |
|--------|------|----------|-------------|-------------|
| `PROOF` | native | 18 | Yes | Native chain token (used for gas) |
| `FloweR` | native-stablecoin | 6 | No | Stablecoin for settlement |

**Amount encoding:** String decimal representation of the smallest unit.  
Example: 1.0 PROOF = `"1000000000000000000"` (18 zeros)  
Example: 1.0 FloweR = `"1000000"` (6 zeros)

---

## 9. Error Handling Guide

### Common Errors

| HTTP | Error | Resolution |
|------|-------|------------|
| 400 | `wallet_address is required` | Provide non-empty wallet address |
| 400 | `wallet not found` | Create wallet first |
| 400 | `challenge not found` | Request a new challenge |
| 400 | `challenge already used` | Each challenge is single-use; request a new one |
| 400 | `challenge expired` | Challenge TTL is 5 minutes; request a new one |
| 400 | `unsupported chain` | Only `"flowcortex-l1"` for MVP |
| 400 | `unsupported asset` | Only `"PROOF"` and `"FloweR"` for MVP |
| 400 | `nonce must be greater than 0` | First nonce must be 1 |
| 400 | `nonce replay detected` | Use `next_nonce` from `/wallet/nonce` |
| 401 | `missing Authorization header` | Send `Authorization: Bearer <jwt>` |
| 401 | `expired AuthBuddy JWT` | Refresh JWT from AuthBuddy |
| 401 | `ops access denied` | Requires `ops-admin` role in JWT |
| 500 | (internal) | Server-side error; retry with backoff |

### Retry Strategy

| Scenario | Approach |
|----------|----------|
| 400 errors | Do not retry — fix the request |
| 401 errors | Refresh JWT and retry once |
| 500 errors | Retry with exponential backoff (max 3 attempts) |
| Network timeout | Retry with `Idempotency-Key` for submit |
| Submit 500 | **Always use** `Idempotency-Key` to prevent double-submission |

---

## 10. Chain Configuration

Fetch once at startup and cache:

```bash
GET /chain/config
```

Response:
```json
{
  "chain_slug": "flowcortex-l1",
  "chain_id_numeric": null,
  "signature_scheme": "ed25519",
  "address_scheme": "sha256-truncated-20",
  "domains": {
    "tx_domain_tag": "keycortex:v1:transaction",
    "auth_domain_tag": "keycortex:v1:auth",
    "proof_domain_tag": "keycortex:v1:proof"
  },
  "assets": [...],
  "finality_rule": "deterministic-single-confirmation",
  "environment": "devnet"
}
```

---

## 11. Health Checks

Before making API calls, verify KeyCortex is ready:

```bash
GET /readyz
```

```json
{
  "service": "keycortex-wallet-service",
  "ready": true,
  "keystore_ready": true,
  "storage_mode": "rocksdb",
  "postgres_enabled": false,
  "auth_ready": true,
  "auth_mode": "hs256-fallback",
  "reason": null
}
```

If `ready` is `false`, do not proceed with wallet operations.

---

## 12. Checklist for Treasury App Team

### Wallet Lifecycle
- [ ] **Create wallet**: Call `POST /wallet/create` with optional label + passphrase
- [ ] **List wallets**: Call `GET /wallet/list` to populate wallet picker
- [ ] **Restore wallet**: Support passphrase-based restore via `POST /wallet/restore`
- [ ] **Rename wallet**: Allow label editing via `POST /wallet/rename`

### Authentication
- [ ] **AuthBuddy login**: Integrate AuthBuddy SSO/OAuth for JWT tokens
- [ ] **Challenge flow**: Implement challenge → sign → verify → bind sequence
- [ ] **JWT refresh**: Handle expired JWTs gracefully

### Transactions
- [ ] **Nonce management**: Always fetch fresh nonce before submit
- [ ] **Idempotency**: Always include `Idempotency-Key` header on submit
- [ ] **Status polling**: Poll `/wallet/tx/{hash}` until `status` != `"submitted"`
- [ ] **Amount formatting**: Convert human-readable amounts to smallest-unit strings

### Risk & Compliance
- [ ] **Pre-action risk check**: Call FortressDigital wallet-status before sensitive operations
- [ ] **Risk signal handling**: Map risk signals to UX actions (block, warn, require reverification)
- [ ] **Proof commitment**: Generate ProofCortex commitment for compliance-required actions
- [ ] **Audit context**: Generate signed context payload after significant actions

### Operations
- [ ] **Health monitoring**: Poll `/readyz` before API calls; show service status in UI
- [ ] **Error handling**: Implement error handling per §9
- [ ] **Chain config caching**: Fetch `/chain/config` at startup, refresh periodically

---

## 13. Quick-Start: Complete Flow

```bash
# 1. Health check
curl -s http://localhost:8080/readyz | jq .ready

# 2. Create wallet
WALLET=$(curl -s -X POST http://localhost:8080/wallet/create \
  -H "Content-Type: application/json" \
  -d '{"label": "Treasury Main"}' | jq -r '.wallet_address')
echo "Wallet: $WALLET"

# 3. Auth challenge
CHALLENGE=$(curl -s -X POST http://localhost:8080/auth/challenge | jq -r '.challenge')
echo "Challenge: $CHALLENGE"

# 4. Sign challenge
SIG=$(curl -s -X POST http://localhost:8080/wallet/sign \
  -H "Content-Type: application/json" \
  -d "{\"wallet_address\": \"$WALLET\", \"payload\": \"$CHALLENGE\", \"purpose\": \"auth\"}" \
  | jq -r '.signature')

# 5. Verify
curl -s -X POST http://localhost:8080/auth/verify \
  -H "Content-Type: application/json" \
  -d "{\"wallet_address\": \"$WALLET\", \"challenge\": \"$CHALLENGE\", \"signature\": \"$SIG\"}" | jq .

# 6. Risk check
curl -s -X POST http://localhost:8080/fortressdigital/wallet-status \
  -H "Content-Type: application/json" \
  -d "{\"wallet_address\": \"$WALLET\", \"chain\": \"flowcortex-l1\"}" | jq .risk_signals

# 7. Get nonce
NONCE=$(curl -s "http://localhost:8080/wallet/nonce?wallet_address=$WALLET" | jq -r '.next_nonce')

# 8. Submit transaction
TX=$(curl -s -X POST http://localhost:8080/wallet/submit \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: tx-$(date +%s)" \
  -d "{\"from\": \"$WALLET\", \"to\": \"0x0000000000000000000000000000000000000000\", \"amount\": \"1000000\", \"asset\": \"FloweR\", \"chain\": \"flowcortex-l1\", \"nonce\": $NONCE}" \
  | jq -r '.tx_hash')
echo "TX: $TX"

# 9. Check status
curl -s "http://localhost:8080/wallet/tx/$TX" | jq '{status, accepted}'
```
