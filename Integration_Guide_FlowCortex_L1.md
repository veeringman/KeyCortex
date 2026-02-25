# FlowCortex L1 — KeyCortex Integration Guide

> **Version:** 1.0 · **Last updated:** 2025-01-XX  
> **Audience:** FlowCortex chain team — node operators, protocol engineers  
> **Scope:** Chain identity, transaction signing, balance queries, domain tags, asset metadata

---

## 1. Overview

KeyCortex is a self-custody wallet service that generates Ed25519 key pairs, signs transactions, and submits them to the **FlowCortex L1** chain. FlowCortex is the **settlement layer** — it must:

- Accept signed transactions from KeyCortex
- Provide balance query endpoints
- Provide transaction status endpoints
- Confirm chain identity parameters (chain_id, genesis_hash, etc.)

```
┌───────────────┐     submit_tx       ┌──────────────────┐
│  KeyCortex    │ ──────────────────▶  │  FlowCortex L1   │
│  wallet-svc   │                      │  node / RPC      │
│               │ ◀── balance/status ─ │                   │
└───────────────┘                      └──────────────────┘
```

---

## 2. Current Chain Configuration (Implemented)

KeyCortex serves chain config at **`GET /chain/config`**. This is what's hardcoded today — FlowCortex team must **confirm or update** each field:

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
    {
      "symbol": "PROOF",
      "asset_type": "native",
      "decimals": 18,
      "fee_payment_support": true
    },
    {
      "symbol": "FloweR",
      "asset_type": "native-stablecoin",
      "decimals": 6,
      "fee_payment_support": false
    }
  ],
  "finality_rule": "deterministic-single-confirmation",
  "environment": "devnet"
}
```

### 2.1 Fields Awaiting FlowCortex Confirmation

| Field | Current Value | Status | Action Needed |
|-------|---------------|--------|---------------|
| `chain_id_numeric` | `null` | **TBD** | FlowCortex must provide the numeric chain ID |
| `tx_domain_tag` | `keycortex:v1:transaction` | Implemented | Confirm or propose alternative |
| `auth_domain_tag` | `keycortex:v1:auth` | Implemented | Confirm or propose alternative |
| `proof_domain_tag` | `keycortex:v1:proof` | Implemented | Confirm or propose alternative |
| `PROOF` decimals | `18` | Implemented | Confirm |
| `FloweR` decimals | `6` | Implemented | Confirm |
| `finality_rule` | `deterministic-single-confirmation` | Implemented | Confirm finality model |
| `environment` | `devnet` | Implemented | Will change per deployment |

---

## 3. Transaction Signing Contract

### 3.1 How KeyCortex Signs Transactions

KeyCortex constructs a **canonical payload string** and signs it with Ed25519:

```
payload = "from={from};to={to};amount={amount};asset={asset};chain={chain};nonce={nonce}"
```

The signing process applies a **domain tag** prefix before signing:

```
signing_input = "keycortex:v1:transaction:" + payload_bytes
signature = ed25519_sign(signing_key, signing_input)
```

### 3.2 Transaction Submit Request (from KeyCortex → FlowCortex)

KeyCortex calls `FlowCortexAdapter::submit_transaction()` with:

```rust
SubmitTxRequest {
    from: WalletAddress,      // "0x..." (20-byte SHA-256 truncated public key)
    to: WalletAddress,        // "0x..."
    amount: String,            // Decimal string (e.g., "1000000000000000000" for 1 PROOF)
    asset: AssetSymbol,        // "PROOF" or "FloweR"
    chain: ChainId,            // "flowcortex-l1"
    signed_payload: String,    // Hex-encoded Ed25519 signature (128 hex chars = 64 bytes)
}
```

### 3.3 Expected Response from FlowCortex

```rust
SubmitTxResult {
    tx_hash: String,   // Unique transaction hash assigned by the chain
    accepted: bool,    // true if transaction was accepted into mempool/block
}
```

### 3.4 Validation FlowCortex Must Perform

| Check | Detail |
|-------|--------|
| **Signature verification** | Reconstruct `signing_input = "keycortex:v1:transaction:" + payload`, verify Ed25519 signature against sender's public key |
| **Nonce ordering** | Nonce must be strictly increasing per wallet address. KeyCortex enforces this locally too. |
| **Asset support** | Only `PROOF` and `FloweR` for MVP |
| **Chain match** | `chain` must be `"flowcortex-l1"` |
| **Balance sufficiency** | Verify sender has enough balance (including fees if PROOF) |

---

## 4. Address Scheme

| Property | Value |
|----------|-------|
| **Signature scheme** | Ed25519 (RFC 8032) |
| **Public key** | 32 bytes (hex: 64 characters) |
| **Address derivation** | `SHA-256(public_key)[0..20]` — first 20 bytes of hash |
| **Address format** | `0x` + 40 hex characters (e.g., `0xa1b2c3...`) |

FlowCortex must use this same derivation to map public keys to addresses.

---

## 5. Nonce Management

| Property | Value |
|----------|-------|
| **Nonce type** | `u64` |
| **Starting value** | 1 (nonce 0 is rejected) |
| **Ordering** | Strictly increasing per wallet address |
| **KeyCortex tracking** | Local nonce state in RocksDB + in-memory cache |
| **Replay protection** | KeyCortex rejects `nonce ≤ last_known_nonce` before submitting to chain |

### 5.1 Nonce Query Endpoint

```
GET /wallet/nonce?wallet_address=0x...
```

Response:
```json
{
  "wallet_address": "0x...",
  "last_nonce": 5,
  "next_nonce": 6
}
```

---

## 6. Balance Query

```
GET /wallet/balance?wallet_address=0x...&asset=PROOF&chain=flowcortex-l1
```

Response:
```json
{
  "wallet_address": "0x...",
  "chain": "flowcortex-l1",
  "asset": "PROOF",
  "amount": "0"
}
```

**Note:** The current `FlowCortexAdapter` returns `"0"` as a placeholder. FlowCortex team must provide the actual RPC endpoint for balance queries.

---

## 7. Transaction Status Query

```
GET /wallet/tx/{tx_hash}
```

KeyCortex calls `FlowCortexAdapter::get_transaction_status()` to refresh the status:

```rust
TxStatusRequest {
    tx_hash: String,
    chain: ChainId,  // "flowcortex-l1"
}

TxStatusResult {
    tx_hash: String,
    status: String,   // e.g., "submitted", "confirmed", "failed"
    accepted: bool,
}
```

Response:
```json
{
  "tx_hash": "abc123...",
  "status": "confirmed",
  "accepted": true,
  "chain": "flowcortex-l1",
  "from": "0x...",
  "to": "0x...",
  "asset": "PROOF",
  "amount": "1000000000000000000",
  "submitted_at_epoch_ms": 1706140800000
}
```

---

## 8. ChainAdapter Trait — Interface Contract

FlowCortex integration is implemented via a Rust trait. If FlowCortex provides an SDK, it must implement:

```rust
#[async_trait]
pub trait ChainAdapter: Send + Sync {
    fn chain_id(&self) -> &str;  // must return "flowcortex-l1"

    async fn get_balance(
        &self,
        wallet_address: &WalletAddress,
        asset: &AssetSymbol
    ) -> Result<BalanceResult>;

    async fn submit_transaction(
        &self,
        req: SubmitTxRequest
    ) -> Result<SubmitTxResult>;

    async fn get_transaction_status(
        &self,
        req: TxStatusRequest
    ) -> Result<TxStatusResult>;
}
```

---

## 9. Domain Tags — Cryptographic Alignment

KeyCortex uses domain-tagged signing to prevent cross-context signature reuse:

| Purpose | Domain Tag Prefix | Used By |
|---------|-------------------|---------|
| **Transaction signing** | `keycortex:v1:transaction:` | `POST /wallet/submit` |
| **Auth challenge signing** | `keycortex:v1:auth:` | `POST /auth/verify` |
| **Proof commitment** | `keycortex:v1:proof:` | `POST /proofcortex/commitment`, FortressDigital context |

FlowCortex's verifier must prepend the matching domain tag before verifying any signature.

---

## 10. Idempotency Support

`POST /wallet/submit` supports an **`Idempotency-Key`** header:

| Header | Required | Behavior |
|--------|----------|----------|
| `Idempotency-Key: <unique-string>` | Optional | If the same key is sent again, returns the cached response without re-submitting |

FlowCortex should also handle duplicate transaction submissions gracefully (same nonce = reject).

---

## 11. What FlowCortex Must Provide to KeyCortex

These are **blocking inputs** — KeyCortex cannot go to production without them:

| # | Item | Format | Status |
|---|------|--------|--------|
| 1 | **Chain ID (numeric)** | `u64` | TBD |
| 2 | **Genesis hash** | Hex string | TBD |
| 3 | **RPC endpoint URL** (submit tx) | HTTPS URL | TBD |
| 4 | **RPC endpoint URL** (balance query) | HTTPS URL | TBD |
| 5 | **RPC endpoint URL** (tx status) | HTTPS URL | TBD |
| 6 | **Domain tag confirmation** | Confirm `keycortex:v1:transaction:` prefix | TBD |
| 7 | **PROOF decimals confirmation** | Confirm 18 | TBD |
| 8 | **FloweR decimals confirmation** | Confirm 6 | TBD |
| 9 | **Finality rule** | Confirm `deterministic-single-confirmation` | TBD |
| 10 | **Fee model** | Which asset pays fees? Fee calculation formula? | TBD |

### Delivery Format

Please provide as YAML or JSON:

```yaml
chain_slug: flowcortex-l1
chain_id_numeric: <FILL>
genesis_hash: <FILL>
rpc_endpoint_submit: <FILL>
rpc_endpoint_balance: <FILL>
rpc_endpoint_tx_status: <FILL>
domain_tags:
  tx: "keycortex:v1:transaction"     # confirm or change
  auth: "keycortex:v1:auth"          # confirm or change
  proof: "keycortex:v1:proof"        # confirm or change
assets:
  PROOF:
    decimals: 18                     # confirm or change
    fee_payment: true                # confirm or change
  FloweR:
    decimals: 6                      # confirm or change
    fee_payment: false               # confirm or change
finality_rule: "deterministic-single-confirmation"  # confirm or change
fee_model: <FILL>
```

---

## 12. Checklist for FlowCortex Team

- [ ] **Chain ID**: Provide numeric chain ID
- [ ] **Genesis hash**: Provide genesis hash string
- [ ] **RPC endpoints**: Provide URLs for submit, balance, tx-status
- [ ] **Domain tags**: Confirm `keycortex:v1:{purpose}:` prefix format
- [ ] **Ed25519 verifier**: Implement signature verification with domain tag prepend
- [ ] **Address scheme**: Confirm `SHA-256(pubkey)[0..20]` with `0x` prefix
- [ ] **Asset metadata**: Confirm PROOF (18 decimals, native) and FloweR (6 decimals, stablecoin)
- [ ] **Nonce model**: Confirm strictly-increasing u64 nonce per address
- [ ] **Fee model**: Document fee calculation and which asset pays
- [ ] **Canonical payload**: Confirm `from={};to={};amount={};asset={};chain={};nonce={}` format
- [ ] **Transaction hash format**: Specify hash algorithm and format for tx_hash
