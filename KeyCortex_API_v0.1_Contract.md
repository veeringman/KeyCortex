<img src="./keycortex_logo.png" alt="KeyCortex Logo" width="180" />

# KeyCortex Wallet/Auth API Contract v0.1 (Frozen)

## Scope

This document freezes the wallet/auth API surface for v0.1.

- Service: `wallet-service`
- Base path: `/`
- Content type: `application/json`
- Error shape (all error responses):

```json
{
  "error": "message"
}
```

MVP constraints:

- chain: `flowcortex-l1`
- assets: `PROOF`, `FloweR`

---

## Wallet APIs

### `POST /wallet/create`

Request body: none

Success `200`:

```json
{
  "wallet_address": "0x...",
  "public_key": "...",
  "chain": "flowcortex-l1"
}
```

---

### `POST /wallet/sign`

Request:

```json
{
  "wallet_address": "0x...",
  "payload": "<base64>",
  "purpose": "transaction"
}
```

`purpose` enum: `transaction | auth | proof`

Success `200`:

```json
{
  "signature": "<hex>"
}
```

Validation errors `400` include:

- `wallet_address is required`
- `payload cannot be empty`
- `payload must be valid base64`
- `wallet not found`

---

### `GET /wallet/balance`

Query params:

- `wallet_address` (required)
- `asset` (optional, default `PROOF`)
- `chain` (optional, default `flowcortex-l1`)

Success `200`:

```json
{
  "wallet_address": "0x...",
  "chain": "flowcortex-l1",
  "asset": "PROOF",
  "amount": "0"
}
```

Validation errors `400` include:

- `wallet_address is required`
- `unsupported chain for MVP; only flowcortex-l1 is enabled`
- `unsupported asset for MVP; only PROOF and FloweR are enabled`

---

### `POST /wallet/submit`

Headers:

- `Idempotency-Key` (optional)

Request:

```json
{
  "from": "0x...",
  "to": "0x...",
  "amount": "1000",
  "asset": "FloweR",
  "chain": "flowcortex-l1",
  "nonce": 1
}
```

Success `200`:

```json
{
  "accepted": true,
  "tx_hash": "pending-integration",
  "signature": "<hex>"
}
```

Validation errors `400` include:

- required field checks (`from`, `to`, `amount`)
- `nonce must be greater than 0`
- chain/asset MVP guardrails
- `source wallet not found`
- `source wallet address does not match custodied key`
- `nonce replay detected; nonce must be strictly increasing per wallet`

---

### `GET /wallet/nonce`

Query params:

- `wallet_address` (required)

Success `200`:

```json
{
  "wallet_address": "0x...",
  "last_nonce": 3,
  "next_nonce": 4
}
```

Validation errors `400` include:

- `wallet_address is required`
- `wallet not found`

---

### `GET /wallet/tx/{tx_hash}`

Path params:

- `tx_hash` (required)

Success `200`:

```json
{
  "tx_hash": "pending-integration",
  "status": "submitted",
  "accepted": true,
  "chain": "flowcortex-l1",
  "from": "0x...",
  "to": "0x...",
  "asset": "FloweR",
  "amount": "1000",
  "submitted_at_epoch_ms": 1700000000000
}
```

Validation errors `400` include:

- `tx_hash is required`
- `transaction not found`

---

## Auth APIs

### `POST /auth/challenge`

Request body: none

Success `200`:

```json
{
  "challenge": "...",
  "expires_in": 120
}
```

---

### `POST /auth/verify`

Request:

```json
{
  "wallet_address": "0x...",
  "signature": "<hex>",
  "challenge": "..."
}
```

Success `200`:

```json
{
  "valid": true,
  "wallet_address": "0x...",
  "verified_at_epoch_ms": 1700000000000
}
```

Validation errors `400` include:

- `wallet_address is required`
- `challenge is required`
- `signature is required`
- `challenge not found`
- `challenge already used`
- `challenge expired`
- `wallet not found`
- `wallet address does not match custodied key`
- `signature must be valid hex`

---

### `POST /auth/bind`

Headers:

- `Authorization: Bearer <token>` (required)

Request:

```json
{
  "wallet_address": "0x...",
  "chain": "flowcortex-l1"
}
```

Success `200`:

```json
{
  "bound": true,
  "user_id": "user-123",
  "wallet_address": "0x...",
  "chain": "flowcortex-l1",
  "bound_at_epoch_ms": 1700000000000
}
```

Error codes:

- `400` validation errors (wallet, chain)
- `401` auth/token errors

---

## Compatibility Rule

For v0.1, fields/types/endpoints in this document are frozen and backward-compatible changes must remain additive only.
