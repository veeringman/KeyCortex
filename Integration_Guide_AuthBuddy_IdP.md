# AuthBuddy IdP — KeyCortex Integration Guide

> **Version:** 1.0 · **Last updated:** 2025-01-XX  
> **Audience:** AuthBuddy IdP team — identity, tokens, user management  
> **Scope:** JWT issuance, JWKS hosting, wallet-binding callback, ops access roles

---

## 1. Overview

KeyCortex is a self-custody wallet service that manages Ed25519 key pairs on FlowCortex L1. AuthBuddy is the **identity provider (IdP)** — it **does not touch private keys** but is the authoritative source for:

- **User identity** (the `sub` claim in JWT tokens)
- **Role assignments** (ops-admin, user, etc.)
- **Token issuance** (HS256 or RS256 JWT)
- **JWKS endpoint** (for RS256 public key rotation)
- **Wallet-binding callback** (receives notification when a wallet is bound to a user)

```
┌─────────────┐       JWT token        ┌───────────────┐
│  AuthBuddy  │ ─────────────────────▶  │  KeyCortex    │
│     IdP     │                         │  wallet-svc   │
│             │ ◀── bind callback ────  │               │
└─────────────┘                         └───────────────┘
```

---

## 2. JWT Token Contract

KeyCortex validates AuthBuddy JWTs on **two endpoints**:

| Endpoint | Method | Auth Required | Purpose |
|----------|--------|---------------|---------|
| `POST /auth/bind` | Bearer JWT | **Yes** | Bind wallet to IdP user |
| `GET /ops/bindings/{wallet_address}` | Bearer JWT | **Yes** (+ `ops-admin` role) | Query wallet binding |
| `GET /ops/audit` | Bearer JWT | **Yes** (+ `ops-admin` role) | List audit trail |

### 2.1 Required JWT Claims

| Claim | Type | Required | Description |
|-------|------|----------|-------------|
| `sub` | `string` | **MANDATORY** | Unique user identifier (e.g., `user-uuid-1234`). Must be non-empty after trimming. |
| `exp` | `number` | **MANDATORY** | Expiration timestamp (Unix epoch seconds). KeyCortex rejects expired tokens. |
| `iss` | `string` | Conditional | Issuer. **Mandatory** if KeyCortex is configured with `AUTHBUDDY_JWT_ISSUER`. Must match exactly. |
| `aud` | `string` | Conditional | Audience. **Mandatory** if KeyCortex is configured with `AUTHBUDDY_JWT_AUDIENCE`. Must match exactly. |
| `roles` | `string[]` | Optional | Array of role strings. Used for ops-admin gating. |
| `role` | `string` | Optional | Comma-separated roles (alternative to `roles` array). Both are merged. |

### 2.2 Example JWT Payload

```json
{
  "sub": "user-a1b2c3d4",
  "exp": 1706140800,
  "iss": "https://authbuddy.example.com",
  "aud": "keycortex-wallet-service",
  "roles": ["user", "ops-admin"],
  "role": "treasury-viewer"
}
```

All roles from both `roles` (array) and `role` (comma-separated string) are **merged** into a single set.

### 2.3 Signing Algorithms

KeyCortex supports **two** token signing modes (in priority order):

| Priority | Algorithm | Key Source | When Used |
|----------|-----------|------------|-----------|
| 1 (preferred) | **RS256** | JWKS endpoint or file | If JWKS is loaded (URL, file, or inline JSON) |
| 2 (fallback) | **HS256** | Shared secret | If no JWKS is available |

**RS256 is strongly recommended for production.** HS256 is a dev/fallback path.

---

## 3. JWKS Configuration

AuthBuddy must expose a **JWKS endpoint** so KeyCortex can verify RS256 tokens.

### 3.1 JWKS Endpoint Requirements

| Requirement | Detail |
|-------------|--------|
| **Format** | Standard JWKS (`{"keys": [...]}`) per RFC 7517 |
| **Algorithm** | RS256 (RSA + SHA-256) |
| **`kid` header** | **MANDATORY** in every JWT. KeyCortex matches `kid` against JWKS keys. |
| **Availability** | Must be reachable from KeyCortex service network |
| **TLS** | HTTPS in production |
| **Rotation** | KeyCortex refreshes JWKS every `AUTHBUDDY_JWKS_REFRESH_SECONDS` (default: 60s, min: 10s). Old `kid` values are dropped on refresh. |

### 3.2 Example JWKS Response

```json
{
  "keys": [
    {
      "kty": "RSA",
      "kid": "authbuddy-key-2025-01",
      "alg": "RS256",
      "use": "sig",
      "n": "<base64url-encoded modulus>",
      "e": "AQAB"
    }
  ]
}
```

### 3.3 JWKS Delivery Options

KeyCortex can consume JWKS via **three** paths (configure whichever AuthBuddy supports):

| Env Variable | Method | Example |
|-------------|--------|---------|
| `AUTHBUDDY_JWKS_URL` | HTTP(S) GET — auto-refreshed on interval | `https://authbuddy.example.com/.well-known/jwks.json` |
| `AUTHBUDDY_JWKS_PATH` | Local file — auto-reloaded on interval | `/etc/keycortex/authbuddy-jwks.json` |
| `AUTHBUDDY_JWKS_JSON` | Inline JSON in env var (static) | `{"keys":[...]}` |

Priority: URL > File > Inline. If URL fetch fails, falls back to file path if configured.

---

## 4. Wallet-Binding Callback

When a user binds a wallet via `POST /auth/bind`, KeyCortex **fires an async HTTP POST** to AuthBuddy's callback URL (if configured via `AUTHBUDDY_CALLBACK_URL`).

### 4.1 Callback Payload

```json
POST <AUTHBUDDY_CALLBACK_URL>
Content-Type: application/json

{
  "user_id": "user-a1b2c3d4",
  "wallet_address": "0xabc123...",
  "chain": "flowcortex-l1",
  "bound_at_epoch_ms": 1706140800000
}
```

| Field | Type | Description |
|-------|------|-------------|
| `user_id` | `string` | The `sub` from the JWT used in the bind request |
| `wallet_address` | `string` | Hex address (`0x` prefix, 20-byte SHA-256 truncated) |
| `chain` | `string` | Always `"flowcortex-l1"` for MVP |
| `bound_at_epoch_ms` | `number` | Binding timestamp (milliseconds since Unix epoch) |

### 4.2 Callback Contract

| Aspect | Expectation |
|--------|-------------|
| **Method** | POST |
| **Content-Type** | `application/json` |
| **Idempotency** | AuthBuddy should handle duplicate callbacks gracefully (same user+wallet may re-bind) |
| **Response** | Any 2xx = success. Non-2xx or timeout is logged but **does not block** the bind operation. |
| **Timeout** | KeyCortex uses a 10s HTTP client timeout |
| **Fire-and-forget** | The callback is async — `/auth/bind` returns *before* the callback completes |

### 4.3 What AuthBuddy Should Do On Callback

1. **Persist the binding** in its user profile store (user → wallet_address mapping)
2. **Update user metadata** if needed (e.g., "has_wallet: true")
3. **Trigger downstream** events if other services need to know about wallet bindings

---

## 5. Auth Bind Flow — End to End

```
1. User logs into AuthBuddy → receives JWT
2. Client calls POST /auth/bind with:
     - Authorization: Bearer <jwt>
     - Body: { "wallet_address": "0x...", "chain": "flowcortex-l1" }
3. KeyCortex validates JWT:
     a. RS256 via JWKS (preferred) or HS256 fallback
     b. Check exp claim (reject if expired)
     c. Check iss claim (if AUTHBUDDY_JWT_ISSUER is set)
     d. Check aud claim (if AUTHBUDDY_JWT_AUDIENCE is set)
     e. Extract sub → user_id
     f. Extract roles/role → merged role set
4. KeyCortex verifies wallet exists in keystore
5. KeyCortex persists binding: wallet_address ↔ user_id ↔ chain
6. KeyCortex fires async callback to AUTHBUDDY_CALLBACK_URL
7. Returns AuthBindResponse:
     {
       "bound": true,
       "user_id": "user-a1b2c3d4",
       "wallet_address": "0x...",
       "chain": "flowcortex-l1",
       "bound_at_epoch_ms": 1706140800000
     }
```

---

## 6. Ops Access — Role Gating

The `/ops/*` endpoints require an AuthBuddy JWT with the **`ops-admin`** role.

### 6.1 Role Check Logic

```
1. Parse JWT (same as auth/bind)
2. Merge roles[] + role (comma-separated) → role set
3. Check if "ops-admin" ∈ role set
4. If missing → 401 "ops access denied" + audit event logged
5. If present → access granted + audit event logged
```

### 6.2 Required Role

| Role | Description | Required For |
|------|-------------|-------------|
| `ops-admin` | Operator dashboard access | `GET /ops/bindings/{addr}`, `GET /ops/audit` |

AuthBuddy must include `"ops-admin"` in the `roles` array (or `role` string) for operator accounts.

---

## 7. Environment Variables Reference

KeyCortex reads these AuthBuddy-related env vars at startup:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `AUTHBUDDY_JWT_SECRET` | Yes (for HS256) | `authbuddy-dev-secret-change-me` | Shared HMAC secret for HS256 fallback |
| `AUTHBUDDY_JWKS_URL` | Recommended | — | HTTPS URL to AuthBuddy JWKS endpoint |
| `AUTHBUDDY_JWKS_PATH` | Optional | — | Local file path to JWKS JSON |
| `AUTHBUDDY_JWKS_JSON` | Optional | — | Inline JWKS JSON string |
| `AUTHBUDDY_JWKS_REFRESH_SECONDS` | Optional | `60` | JWKS refresh interval (min 10s) |
| `AUTHBUDDY_JWT_ISSUER` | Optional | — | Expected `iss` claim value. If set, tokens without matching `iss` are rejected. |
| `AUTHBUDDY_JWT_AUDIENCE` | Optional | — | Expected `aud` claim value. If set, tokens without matching `aud` are rejected. |
| `AUTHBUDDY_CALLBACK_URL` | Optional | — | URL to POST wallet-binding notifications to |

---

## 8. Health & Diagnostic Visibility

AuthBuddy JWKS status is exposed on KeyCortex operational endpoints:

| Endpoint | Fields |
|----------|--------|
| `GET /health` | `auth_mode`, `jwks_source`, `jwks_loaded`, `last_jwks_refresh_epoch_ms`, `last_jwks_error` |
| `GET /readyz` | `auth_ready`, `auth_mode`, `jwks_reachable` |
| `GET /startupz` | `auth_mode`, `jwks_source`, `jwks_loaded`, `last_jwks_refresh_epoch_ms`, `last_jwks_error` |

If JWKS fetch fails, KeyCortex logs warnings but **does not crash** — it falls back to HS256 if a shared secret is configured.

---

## 9. Error Responses

All auth-related errors return:

```json
{
  "error": "<message>"
}
```

| HTTP Status | Error Message | Cause |
|-------------|---------------|-------|
| 401 | `missing Authorization header` | No `Authorization` header sent |
| 401 | `invalid Authorization format` | Not `Bearer <token>` |
| 401 | `missing bearer token` | Empty token after "Bearer " |
| 401 | `invalid AuthBuddy JWT` | Token decode failed (bad signature, malformed) |
| 401 | `invalid AuthBuddy JWT algorithm; expected RS256` | Token uses wrong algorithm when JWKS is loaded |
| 401 | `missing AuthBuddy JWT kid header` | RS256 token without `kid` |
| 401 | `no matching JWK found for token kid` | `kid` not in current JWKS |
| 401 | `missing AuthBuddy JWT exp claim` | No `exp` in token |
| 401 | `expired AuthBuddy JWT` | `exp` ≤ current time |
| 401 | `missing AuthBuddy JWT iss claim` | `iss` required but missing |
| 401 | `invalid AuthBuddy JWT issuer` | `iss` doesn't match expected |
| 401 | `missing AuthBuddy JWT aud claim` | `aud` required but missing |
| 401 | `invalid AuthBuddy JWT audience` | `aud` doesn't match expected |
| 401 | `invalid AuthBuddy JWT subject` | `sub` is empty |
| 401 | `ops access denied` | Missing `ops-admin` role on `/ops/*` endpoints |

---

## 10. Checklist for AuthBuddy Team

- [ ] **JWT claims**: Include `sub` (user ID), `exp` (expiry), `iss`, `aud` in all tokens
- [ ] **RS256 signing**: Issue tokens signed with RS256 using a key with a `kid` header
- [ ] **JWKS endpoint**: Host a JWKS endpoint at a stable URL (e.g., `/.well-known/jwks.json`)
- [ ] **Key rotation**: When rotating keys, keep old key in JWKS for at least 2× refresh interval (120s default)
- [ ] **`ops-admin` role**: Assign to operator accounts that need `/ops/*` access
- [ ] **Callback endpoint**: Implement `POST` handler to receive wallet-binding notifications (§4)
- [ ] **Idempotency**: Handle re-binding callbacks gracefully (same wallet may be re-bound)
- [ ] **Share with KeyCortex team**: JWKS URL, expected issuer string, expected audience string
- [ ] **HS256 secret** (dev only): Agree on shared secret for local/dev environments

---

## 11. Quick-Start: Dev Environment

```bash
# Minimal dev setup — HS256 mode (no JWKS needed)
export AUTHBUDDY_JWT_SECRET="my-dev-secret"

# Generate a test JWT (example using Node.js)
node -e "
  const jwt = require('jsonwebtoken');
  const token = jwt.sign(
    { sub: 'test-user-1', roles: ['user', 'ops-admin'], iss: 'authbuddy-dev', aud: 'keycortex' },
    'my-dev-secret',
    { algorithm: 'HS256', expiresIn: '1h' }
  );
  console.log(token);
"

# Bind a wallet
curl -X POST http://localhost:8080/auth/bind \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"wallet_address": "0x...", "chain": "flowcortex-l1"}'
```
