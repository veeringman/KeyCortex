# FortressDigital — KeyCortex Integration Guide

> **Version:** 1.0 · **Last updated:** 2025-01-XX  
> **Audience:** FortressDigital team — risk engineering, policy engine, compliance  
> **Scope:** Wallet verification status, risk signals, context payloads, policy gating

---

## 1. Overview

KeyCortex provides FortressDigital with **wallet intelligence signals** for risk scoring and policy decisions. FortressDigital is the **risk and compliance layer** — it consumes:

- **Wallet verification status** — does the wallet exist, is it bound to a user, when was it last verified?
- **Risk signal flags** — stale verification, unbound wallet, missing wallet
- **Signed context payloads** — cryptographically signed session context for audit trails
- **Signing frequency hints** — how active is the wallet?

FortressDigital **does not access private keys** and **does not submit transactions**.

```
┌───────────────┐   wallet-status     ┌──────────────────┐
│  KeyCortex    │ ──────────────────▶  │  FortressDigital │
│  wallet-svc   │                      │  Risk Engine     │
│               │   context payload    │                   │
│               │ ──────────────────▶  │  Policy Engine   │
└───────────────┘                      └──────────────────┘
```

---

## 2. Wallet Status Endpoint

### `POST /fortressdigital/wallet-status`

Returns enriched wallet signals for risk scoring and policy gating.

### 2.1 Request

```json
POST /fortressdigital/wallet-status
Content-Type: application/json

{
  "wallet_address": "0xabc123...",
  "chain": "flowcortex-l1",
  "user_id": "user-a1b2c3d4",
  "session_id": "sess-xyz-789"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `wallet_address` | `string` | **MANDATORY** | Hex wallet address to check |
| `chain` | `string` | **MANDATORY** | Chain identifier (must be `"flowcortex-l1"` for MVP) |
| `user_id` | `string` | Optional | IdP user ID (for cross-referencing binding) |
| `session_id` | `string` | Optional | Session ID for audit correlation |

### 2.2 Response

```json
{
  "wallet_address": "0xabc123...",
  "chain": "flowcortex-l1",
  "wallet_exists": true,
  "binding_status": {
    "bound": true,
    "user_id": "user-a1b2c3d4",
    "last_verified_epoch_ms": 1706140800000
  },
  "key_type": "local-ed25519",
  "last_verification_epoch_ms": 1706140800000,
  "signature_frequency_hint": "moderate",
  "risk_signals": []
}
```

### 2.3 Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `wallet_address` | `string` | Echo of request |
| `chain` | `string` | Echo of request |
| `wallet_exists` | `bool` | Whether the wallet is in KeyCortex keystore |
| `binding_status.bound` | `bool` | Whether wallet is bound to an IdP user |
| `binding_status.user_id` | `string?` | Bound user ID (null if not bound) |
| `binding_status.last_verified_epoch_ms` | `number?` | Last verification timestamp (null if not bound) |
| `key_type` | `string` | Always `"local-ed25519"` — indicates local custody |
| `last_verification_epoch_ms` | `number?` | Same as binding last_verified (null if never verified) |
| `signature_frequency_hint` | `string` | Activity level: `"none"`, `"low"`, `"moderate"`, `"high"` |
| `risk_signals` | `string[]` | Array of risk flag strings (see §3) |

---

## 3. Risk Signals

KeyCortex returns zero or more risk signal flags. FortressDigital should incorporate these into its risk scoring model.

| Signal | Condition | Risk Implication |
|--------|-----------|------------------|
| `wallet_not_found` | Wallet address not in KeyCortex keystore | **HIGH** — unknown wallet, likely invalid or foreign |
| `wallet_not_bound` | Wallet exists but not bound to any IdP user | **MEDIUM** — wallet has no verified owner |
| `verification_stale_24h` | Last verification > 24 hours ago | **LOW-MEDIUM** — wallet owner hasn't re-verified recently |
| `never_verified` | Wallet exists but has never been verified | **MEDIUM** — wallet was created but never proven ownership |

### 3.1 Signal Logic (Pseudocode)

```python
signals = []

if not wallet_exists:
    signals.append("wallet_not_found")

if not binding_status.bound:
    signals.append("wallet_not_bound")

if binding_status.bound and last_verified_ms:
    age_hours = (now - last_verified_ms) / (1000 * 60 * 60)
    if age_hours > 24:
        signals.append("verification_stale_24h")
elif wallet_exists:
    signals.append("never_verified")
```

### 3.2 Signature Frequency Hints

Based on audit event count for the wallet:

| Event Count | Hint | Meaning |
|-------------|------|---------|
| 0 | `"none"` | No signing activity recorded |
| 1-5 | `"low"` | Minimal activity |
| 6-20 | `"moderate"` | Regular usage |
| 21+ | `"high"` | Frequent signing |

---

## 4. Context Payload Endpoint

### `POST /fortressdigital/context`

Generates a signed context payload for audit-grade session tracking.

### 4.1 Request

```json
POST /fortressdigital/context
Content-Type: application/json

{
  "wallet_address": "0xabc123...",
  "user_id": "user-a1b2c3d4",
  "chain": "flowcortex-l1",
  "session_id": "sess-xyz-789",
  "context_data": "{\"action\": \"transfer\", \"amount\": \"100\"}",
  "expires_in_seconds": 600
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `wallet_address` | `string` | **MANDATORY** | Wallet address |
| `user_id` | `string` | **MANDATORY** | IdP user ID |
| `chain` | `string` | **MANDATORY** | Chain identifier |
| `session_id` | `string` | **MANDATORY** | Session ID for correlation |
| `context_data` | `string` | **MANDATORY** | JSON or free-text context (serialized to string) |
| `expires_in_seconds` | `number` | Optional | TTL in seconds (default: 600 = 10 minutes) |

### 4.2 Response

```json
{
  "wallet_address": "0xabc123...",
  "user_id": "user-a1b2c3d4",
  "chain": "flowcortex-l1",
  "session_id": "sess-xyz-789",
  "issued_at_epoch_ms": 1706140800000,
  "expires_at_epoch_ms": 1706141400000,
  "context_data": "{\"action\": \"transfer\", \"amount\": \"100\"}",
  "signature": "base64-encoded-ed25519-signature"
}
```

### 4.3 Signature Verification

The `signature` field is a **base64-encoded Ed25519 signature** over the JSON-serialized payload (with `signature` field set to empty string during signing).

**Verification steps:**
1. Take the response JSON
2. Set `signature` to `""` (empty string)
3. JSON-serialize the modified object
4. Verify the original base64-decoded signature against the serialized bytes using the **proof** domain tag: `keycortex:v1:proof:`

This proves the payload was issued by KeyCortex and has not been tampered with.

---

## 5. Binding Status Data Source

The wallet-status endpoint reads binding data from:
1. **Postgres** (primary, if configured)
2. **RocksDB** (fallback if Postgres unavailable)

Binding records are created via `POST /auth/bind` (requires AuthBuddy JWT).

### Binding Record Structure

```json
{
  "wallet_address": "0x...",
  "user_id": "user-a1b2c3d4",
  "chain": "flowcortex-l1",
  "last_verified_epoch_ms": 1706140800000
}
```

---

## 6. Error Responses

```json
{
  "error": "<message>"
}
```

| Endpoint | HTTP Status | Error | Cause |
|----------|-------------|-------|-------|
| `/fortressdigital/wallet-status` | 400 | `wallet_address is required` | Empty wallet_address |
| `/fortressdigital/wallet-status` | 500 | (internal) | Keystore read failure |
| `/fortressdigital/context` | 500 | (internal) | Signing failure |

**Note:** The wallet-status endpoint does **not** return 404 for unknown wallets — it returns a valid response with `wallet_exists: false` and `risk_signals: ["wallet_not_found"]`.

---

## 7. Integration Flow — Risk Decision

```
1. User initiates action in Treasury app
2. Treasury app calls FortressDigital for risk check
3. FortressDigital calls KeyCortex:
     POST /fortressdigital/wallet-status
     { "wallet_address": "0x...", "chain": "flowcortex-l1" }
4. KeyCortex returns wallet signals + risk flags
5. FortressDigital evaluates risk:
     - wallet_not_found → BLOCK
     - wallet_not_bound → REQUIRE_BINDING
     - verification_stale_24h → REQUIRE_REVERIFICATION
     - never_verified → REQUIRE_VERIFICATION
     - no signals → ALLOW
6. FortressDigital returns risk decision to Treasury app
7. Treasury app proceeds or blocks based on decision
```

### Context Payload Flow (Audit Trail)

```
1. Before executing a high-value action, Treasury app calls:
     POST /fortressdigital/context
     { wallet_address, user_id, chain, session_id, context_data }
2. KeyCortex returns signed context payload
3. Treasury app forwards payload to FortressDigital
4. FortressDigital stores signed payload as audit evidence
5. The signature proves KeyCortex attested to the context at the given time
```

---

## 8. Chain Config Access

FortressDigital can query chain configuration directly:

```
GET /chain/config
```

Returns chain identity, domain tags, and asset metadata (see FlowCortex Integration Guide for full schema).

---

## 9. Ops Endpoints (with AuthBuddy JWT)

FortressDigital compliance officers with `ops-admin` role can access:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `GET /ops/bindings/{wallet_address}` | GET | Query specific wallet binding details |
| `GET /ops/audit?limit=100&event_type=auth_bind&wallet_address=0x...` | GET | Search audit trail with filters |

Both require `Authorization: Bearer <authbuddy-jwt>` with `ops-admin` role.

### Audit Query Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `limit` | `int` | Optional | 100 | Max events (1-500) |
| `event_type` | `string` | Optional | — | Filter by event type |
| `wallet_address` | `string` | Optional | — | Filter by wallet |
| `outcome` | `string` | Optional | — | Filter by outcome (`success`, `denied`, etc.) |

---

## 10. Checklist for FortressDigital Team

- [ ] **Risk model**: Map KeyCortex risk signals to FortressDigital risk scores/tiers
- [ ] **Policy rules**: Define policy actions for each risk signal (`BLOCK`, `ALLOW`, `REQUIRE_*`)
- [ ] **Wallet-status integration**: Call `POST /fortressdigital/wallet-status` before risk-gated operations
- [ ] **Context payload verification**: Implement Ed25519 signature verification for context payloads (§4.3)
- [ ] **Context storage**: Store signed context payloads as audit evidence
- [ ] **Stale threshold**: Confirm or adjust the 24-hour stale verification threshold
- [ ] **Frequency model**: Integrate `signature_frequency_hint` into behavioral analysis
- [ ] **Ops access**: Request `ops-admin` JWT role from AuthBuddy for compliance officers
- [ ] **Error handling**: Handle 400/500 responses; treat network errors as `wallet_not_found` risk signal
- [ ] **Chain config**: Fetch `/chain/config` to validate expected chain parameters

---

## 11. Quick-Start: Test Wallet Status

```bash
# Create and check a wallet
WALLET=$(curl -s -X POST http://localhost:8080/wallet/create \
  -H "Content-Type: application/json" \
  -d '{"label": "fd-test"}' | jq -r '.wallet_address')

# Query wallet status (no binding yet)
curl -s -X POST http://localhost:8080/fortressdigital/wallet-status \
  -H "Content-Type: application/json" \
  -d "{
    \"wallet_address\": \"$WALLET\",
    \"chain\": \"flowcortex-l1\"
  }" | jq .

# Expected: wallet_exists=true, bound=false, risk_signals=["wallet_not_bound", "never_verified"]
```

```bash
# Generate a signed context payload
curl -s -X POST http://localhost:8080/fortressdigital/context \
  -H "Content-Type: application/json" \
  -d "{
    \"wallet_address\": \"$WALLET\",
    \"user_id\": \"test-user\",
    \"chain\": \"flowcortex-l1\",
    \"session_id\": \"sess-001\",
    \"context_data\": \"{\\\"action\\\": \\\"test\\\"}\",
    \"expires_in_seconds\": 300
  }" | jq .
```
