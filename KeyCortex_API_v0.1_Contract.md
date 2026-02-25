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

## Wallet APIs (v0.1.1 Additive)

The following endpoints were added in v0.1.1 as backward-compatible additions.

---

### `GET /wallet/list`

Returns all wallets in the keystore with labels and binding status.

Success `200`:

```json
{
  "wallets": [
    {
      "wallet_address": "0x...",
      "chain": "flowcortex-l1",
      "bound_user_id": "user-123",
      "public_key": "<64-char-hex>",
      "label": "My Wallet"
    }
  ],
  "total": 1
}
```

Fields `bound_user_id`, `public_key`, and `label` may be `null`.

---

### `POST /wallet/create` (Updated)

Request body now accepts optional fields:

```json
{
  "label": "My Wallet",
  "passphrase": "optional-deterministic-seed"
}
```

Both fields are optional. If `passphrase` is provided, the wallet is derived deterministically (same passphrase = same wallet). If wallet already exists, returns the existing wallet.

Success `200`:

```json
{
  "wallet_address": "0x...",
  "public_key": "<64-char-hex>",
  "chain": "flowcortex-l1",
  "label": "My Wallet"
}
```

---

### `POST /wallet/restore`

Restore a wallet from a passphrase. Same deterministic derivation as passphrase-based create.

Request:

```json
{
  "passphrase": "my recovery phrase",
  "label": "Restored Wallet"
}
```

`passphrase` is **mandatory**. `label` is optional.

Success `200`:

```json
{
  "wallet_address": "0x...",
  "public_key": "<64-char-hex>",
  "chain": "flowcortex-l1",
  "label": "Restored Wallet",
  "already_existed": true
}
```

Validation errors `400` include:

- `passphrase is required`

---

### `POST /wallet/rename`

Rename a wallet's label.

Request:

```json
{
  "wallet_address": "0x...",
  "label": "New Name"
}
```

Both fields are **mandatory**.

Success `200`:

```json
{
  "wallet_address": "0x...",
  "label": "New Name"
}
```

Validation errors `400` include:

- `wallet_address is required`
- `label is required`
- `wallet not found`

---

## Platform Integration APIs (v0.1.1 Additive)

### `POST /fortressdigital/wallet-status`

Request:

```json
{
  "wallet_address": "0x...",
  "chain": "flowcortex-l1",
  "user_id": "optional",
  "session_id": "optional"
}
```

Success `200`:

```json
{
  "wallet_address": "0x...",
  "chain": "flowcortex-l1",
  "wallet_exists": true,
  "binding_status": {
    "bound": true,
    "user_id": "user-123",
    "last_verified_epoch_ms": 1700000000000
  },
  "key_type": "local-ed25519",
  "last_verification_epoch_ms": 1700000000000,
  "signature_frequency_hint": "moderate",
  "risk_signals": []
}
```

---

### `POST /fortressdigital/context`

Request:

```json
{
  "wallet_address": "0x...",
  "user_id": "user-123",
  "chain": "flowcortex-l1",
  "session_id": "sess-001",
  "context_data": "{}",
  "expires_in_seconds": 600
}
```

Success `200`:

```json
{
  "wallet_address": "0x...",
  "user_id": "user-123",
  "chain": "flowcortex-l1",
  "session_id": "sess-001",
  "issued_at_epoch_ms": 1700000000000,
  "expires_at_epoch_ms": 1700000600000,
  "context_data": "{}",
  "signature": "<base64-ed25519-signature>"
}
```

---

### `POST /proofcortex/commitment`

Request:

```json
{
  "wallet_address": "0x...",
  "challenge": "uuid-string",
  "verification_result": true,
  "chain": "flowcortex-l1",
  "tx_hash": "optional"
}
```

All fields except `tx_hash` are **mandatory**.

Success `200`:

```json
{
  "commitment": "<64-char-hex-sha256>",
  "wallet_address": "0x...",
  "chain": "flowcortex-l1",
  "verification_result": true,
  "domain_separator": "keycortex:proof:v1",
  "proof_input_schema_version": "1.0.0",
  "generated_at_epoch_ms": 1700000000000
}
```

Validation errors `400` include:

- `wallet_address is required`
- `challenge is required`
- `chain is required`
- `wallet not found`

---

### `GET /chain/config`

Success `200`:

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
  "assets": [
    { "symbol": "PROOF", "asset_type": "native", "decimals": 18, "fee_payment_support": true },
    { "symbol": "FloweR", "asset_type": "native-stablecoin", "decimals": 6, "fee_payment_support": false }
  ],
  "finality_rule": "deterministic-single-confirmation",
  "environment": "devnet"
}
```

---

## Operations APIs (v0.1.1 Additive)

All ops endpoints require `Authorization: Bearer <authbuddy-jwt>` with `ops-admin` role.

### `GET /ops/bindings/{wallet_address}`

Success `200`:

```json
{
  "wallet_address": "0x...",
  "user_id": "user-123",
  "chain": "flowcortex-l1",
  "last_verified_epoch_ms": 1700000000000
}
```

Error codes: `401` (auth), `404` (not found)

---

### `GET /ops/audit`

Query params:

- `limit` (optional, default 100, max 500)
- `event_type` (optional)
- `wallet_address` (optional)
- `outcome` (optional)

Success `200`:

```json
{
  "events": [
    {
      "event_id": "...",
      "event_type": "auth_bind",
      "wallet_address": "0x...",
      "user_id": "user-123",
      "chain": "flowcortex-l1",
      "outcome": "success",
      "message": "wallet binding persisted",
      "timestamp_epoch_ms": 1700000000000
    }
  ]
}
```

---

## Health & Diagnostics (v0.1.1 Additive)

### `GET /health`

### `GET /readyz`

### `GET /startupz`

### `GET /version`

See service documentation for full response schemas.

---

## Compatibility Rule

For v0.1, fields/types/endpoints in this document are frozen and backward-compatible changes must remain additive only. The v0.1.1 additions above are additive and do not break existing consumers.
