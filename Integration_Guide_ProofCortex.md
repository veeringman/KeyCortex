# ProofCortex — KeyCortex Integration Guide

> **Version:** 1.0 · **Last updated:** 2025-01-XX  
> **Audience:** ProofCortex team — ZKP engineers, circuit designers, proof pipeline  
> **Scope:** Commitment generation, proof inputs, domain alignment, STARK circuit compatibility

---

## 1. Overview

KeyCortex generates **deterministic commitment hashes** that ProofCortex uses as inputs to STARK proof circuits. The commitment proves:

- A wallet was verified (or not)
- The verification was against a specific challenge
- The wallet is custodied by KeyCortex on a specific chain
- (Optionally) linked to a transaction hash

KeyCortex **does not generate proofs** — it produces the **commitment input** that ProofCortex feeds into its STARK prover.

```
┌───────────────┐   commitment hash   ┌──────────────────┐
│  KeyCortex    │ ──────────────────▶  │  ProofCortex     │
│  wallet-svc   │                      │  STARK prover    │
│               │                      │                   │
│  POST /proof  │                      │  circuit input    │
│  cortex/      │                      │  → proof gen      │
│  commitment   │                      │  → on-chain anchor│
└───────────────┘                      └──────────────────┘
```

---

## 2. Commitment Endpoint

### `POST /proofcortex/commitment`

Generates a SHA-256 commitment hash from wallet verification data.

### 2.1 Request

```json
POST /proofcortex/commitment
Content-Type: application/json

{
  "wallet_address": "0xabc123...",
  "challenge": "550e8400-e29b-41d4-a716-446655440000",
  "verification_result": true,
  "chain": "flowcortex-l1",
  "tx_hash": "pending-integration"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `wallet_address` | `string` | **MANDATORY** | Hex wallet address (`0x` prefix). Must exist in KeyCortex keystore. |
| `challenge` | `string` | **MANDATORY** | The auth challenge string that was verified. Typically a UUID. |
| `verification_result` | `bool` | **MANDATORY** | `true` = wallet was successfully verified, `false` = verification failed |
| `chain` | `string` | **MANDATORY** | Chain identifier. Must be `"flowcortex-l1"` for MVP. |
| `tx_hash` | `string` | Optional | Transaction hash to bind the commitment to a specific tx |

### 2.2 Response

```json
{
  "commitment": "a1b2c3d4e5f6...64-char-hex-sha256-hash",
  "wallet_address": "0xabc123...",
  "chain": "flowcortex-l1",
  "verification_result": true,
  "domain_separator": "keycortex:proof:v1",
  "proof_input_schema_version": "1.0.0",
  "generated_at_epoch_ms": 1706140800000
}
```

| Field | Type | Description |
|-------|------|-------------|
| `commitment` | `string` | 64-char hex SHA-256 hash — **the proof circuit input** |
| `wallet_address` | `string` | Echo of request wallet address |
| `chain` | `string` | Echo of request chain |
| `verification_result` | `bool` | Echo of request result |
| `domain_separator` | `string` | The domain separator used: `"keycortex:proof:v1"` |
| `proof_input_schema_version` | `string` | Schema version: `"1.0.0"` |
| `generated_at_epoch_ms` | `number` | Timestamp when commitment was generated |

---

## 3. Commitment Hash Formula

The commitment is a **deterministic SHA-256 hash** of concatenated fields with a domain separator:

```
commitment = SHA-256(
    "keycortex:proof:v1"
    + ":" + wallet_address
    + ":" + challenge
    + ":" + ("verified" | "unverified")
    + ":" + chain
    + (":" + tx_hash)?          ← only if tx_hash is provided
)
```

### 3.1 Step-by-Step Pseudocode

```python
import hashlib

def compute_commitment(wallet_address, challenge, verification_result, chain, tx_hash=None):
    hasher = hashlib.sha256()
    hasher.update(b"keycortex:proof:v1")
    hasher.update(b":")
    hasher.update(wallet_address.encode())
    hasher.update(b":")
    hasher.update(challenge.encode())
    hasher.update(b":")
    hasher.update(b"verified" if verification_result else b"unverified")
    hasher.update(b":")
    hasher.update(chain.encode())
    if tx_hash is not None:
        hasher.update(b":")
        hasher.update(tx_hash.encode())
    return hasher.hexdigest()
```

### 3.2 Example

```
Input:
  wallet_address = "0xa1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
  challenge      = "550e8400-e29b-41d4-a716-446655440000"
  verification   = true → "verified"
  chain          = "flowcortex-l1"
  tx_hash        = None

Hash input bytes:
  "keycortex:proof:v1:0xa1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2:550e8400-e29b-41d4-a716-446655440000:verified:flowcortex-l1"

commitment = sha256(above) → hex string (64 chars)
```

---

## 4. Domain Separator Alignment

| Constant | Value | Where Used |
|----------|-------|------------|
| `PROOF_DOMAIN_SEPARATOR` | `"keycortex:proof:v1"` | Commitment hash prefix |
| `proof_domain_tag` (chain config) | `"keycortex:v1:proof"` | Ed25519 signature domain tag |

**Important:** The commitment domain separator (`keycortex:proof:v1`) and the signing domain tag (`keycortex:v1:proof`) are **different strings** used for different purposes:

- `keycortex:proof:v1` → SHA-256 commitment hash input (this endpoint)
- `keycortex:v1:proof:` → Ed25519 signature prefix (used when signing FortressDigital context payloads)

ProofCortex circuits must use **`keycortex:proof:v1`** when verifying commitment hashes.

---

## 5. Proof Input Schema

| Property | Value |
|----------|-------|
| **Schema version** | `1.0.0` |
| **Hash algorithm** | SHA-256 |
| **Output format** | Lowercase hex string, 64 characters |
| **Deterministic** | Yes — same inputs always produce same commitment |
| **Collision resistance** | SHA-256 standard (256-bit) |

### 5.1 Circuit Input Structure

ProofCortex should structure its STARK circuit inputs as:

```
circuit_input = {
    commitment: bytes32,           // The SHA-256 hash from KeyCortex
    wallet_address: bytes,         // Public input for verification
    chain: bytes,                  // "flowcortex-l1"
    verification_result: bool,     // true/false
    domain_separator: bytes,       // "keycortex:proof:v1" (for re-computation check)
    schema_version: "1.0.0"
}
```

The circuit should:
1. Accept the `commitment` as a public input
2. Accept the preimage fields (wallet_address, challenge, result, chain, tx_hash) as private inputs
3. Re-compute SHA-256 inside the circuit
4. Assert `computed_hash == commitment`

---

## 6. Verification Result Encoding

| Boolean Value | String in Hash | Meaning |
|---------------|----------------|---------|
| `true` | `"verified"` | Wallet ownership was cryptographically verified via auth challenge |
| `false` | `"unverified"` | Verification failed or was not performed |

---

## 7. Wallet Existence Check

Before generating a commitment, KeyCortex verifies the wallet exists in its keystore. If the wallet is not found, the endpoint returns:

```json
HTTP 400
{
  "error": "wallet not found"
}
```

This means ProofCortex can **trust** that any commitment returned references a real, custodied wallet.

---

## 8. Audit Trail

Every commitment generation is logged in KeyCortex's audit system:

```json
{
  "event_type": "proofcortex_commitment",
  "wallet_address": "0x...",
  "chain": "flowcortex-l1",
  "outcome": "success",
  "message": "commitment=a1b2c3d4e5f6..."
}
```

The `message` field contains the first 16 characters of the commitment hash for traceability.

---

## 9. Error Responses

```json
{
  "error": "<message>"
}
```

| HTTP Status | Error | Cause |
|-------------|-------|-------|
| 400 | `wallet_address is required` | Empty or missing wallet_address |
| 400 | `challenge is required` | Empty or missing challenge |
| 400 | `chain is required` | Empty or missing chain |
| 400 | `wallet not found` | Wallet address not in KeyCortex keystore |
| 500 | (internal) | Keystore read failure or system error |

---

## 10. Integration Flow — End to End

```
1. AuthBuddy issues JWT to user
2. User performs auth challenge + verify via KeyCortex
3. Treasury app (or ProofCortex directly) calls:
     POST /proofcortex/commitment
     {
       "wallet_address": "0x...",
       "challenge": "<the-verified-challenge-uuid>",
       "verification_result": true,
       "chain": "flowcortex-l1",
       "tx_hash": "<optional-tx-hash>"
     }
4. KeyCortex returns commitment hash + metadata
5. ProofCortex feeds commitment into STARK circuit
6. ProofCortex generates proof
7. Proof is anchored on FlowCortex L1 (via separate flow)
```

---

## 11. Checklist for ProofCortex Team

- [ ] **Hash verification**: Implement SHA-256 re-computation in STARK circuit matching the formula in §3
- [ ] **Domain separator**: Hard-code `"keycortex:proof:v1"` as the commitment domain separator
- [ ] **Schema version**: Track `proof_input_schema_version` for forward compatibility
- [ ] **Hex parsing**: Parse 64-char lowercase hex commitment into 32-byte input
- [ ] **Circuit inputs**: Structure public/private inputs per §5.1
- [ ] **Verification result encoding**: Map `true` → `"verified"`, `false` → `"unverified"`
- [ ] **Optional tx_hash**: Handle both with and without tx_hash in commitment re-computation
- [ ] **Error handling**: Handle 400/500 responses gracefully when calling `/proofcortex/commitment`
- [ ] **Proof anchoring**: Define how/where generated proofs are anchored on FlowCortex L1

---

## 12. Quick-Start: Test Commitment

```bash
# Create a wallet first
WALLET=$(curl -s -X POST http://localhost:8080/wallet/create \
  -H "Content-Type: application/json" \
  -d '{"label": "proof-test"}' | jq -r '.wallet_address')

# Generate a commitment
curl -s -X POST http://localhost:8080/proofcortex/commitment \
  -H "Content-Type: application/json" \
  -d "{
    \"wallet_address\": \"$WALLET\",
    \"challenge\": \"test-challenge-123\",
    \"verification_result\": true,
    \"chain\": \"flowcortex-l1\"
  }" | jq .
```

Expected output:
```json
{
  "commitment": "...",
  "wallet_address": "0x...",
  "chain": "flowcortex-l1",
  "verification_result": true,
  "domain_separator": "keycortex:proof:v1",
  "proof_input_schema_version": "1.0.0",
  "generated_at_epoch_ms": 1706140800000
}
```
