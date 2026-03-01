<img src="./keycortex_logo.png" alt="KeyCortex Logo" width="180" />

# FlowCortex L1 Chain Parameters — Change Request from KeyCortex

Last Updated: 2026-02-25
Scope: KeyCortex MVP integration baseline (chain = `flowcortex-l1`, assets = `PROOF`, `FloweR`)

---

## 1) Why this document

This is the required parameter contract the KeyCortex wallet/auth side needs from the FlowCortex team to safely integrate with FlowCortex L1.

---

## 2) Mandatory Chain Identity Inputs

These must be finalized before production signing/submit is enabled.

- **`chain_slug`**: Human-readable canonical ID (expected: `flowcortex-l1`)
- **`chain_id_numeric`**: Unique numeric chain ID used in tx domain separation / replay protection
- **`network_id`**: Network identifier if distinct from chain ID
- **`genesis_hash`**: 32-byte genesis block hash
- **`protocol_version`**: Active protocol/upgrade epoch version
- **`address_scheme`**: Address format and checksum rules
- **`signature_scheme`**: Ed25519 (MVP) with exact verification rules

Acceptance rule: all services must use exactly one canonical tuple:  
(`chain_slug`, `chain_id_numeric`, `genesis_hash`, `protocol_version`).

---

## 3) Transaction Domain & Signing Parameters

Needed so KeyCortex can build deterministic signing payloads and downstream proof systems can verify facts consistently.

- **`tx_domain_tag`**: e.g., `flowcortex:tx:v1`
- **`auth_domain_tag`**: e.g., `flowcortex:auth:v1`
- **`proof_domain_tag`**: e.g., `flowcortex:proof:v1`
- **`tx_hash_algorithm`**: e.g., `sha256` or chain-native alternative
- **`canonical_serialization`**: Exact field order/encoding (JSON canonicalization, protobuf, RLP, etc.)
- **`nonce_model`**: account nonce semantics and replay constraints
- **`fee_model`**: fee fields and unit precision
- **`finality_rule`**: required confirmation depth / deterministic finality condition

Without these, signature validity can diverge between wallet and FlowCortex verification paths.

---

## 4) Asset Parameters (MVP assets only)

### PROOF (native coin)

- **`asset_symbol`**: `PROOF`
- **`asset_type`**: native
- **`decimals`**: required
- **`min_transfer_unit`**: required
- **`fee_payment_support`**: yes/no

### FloweR (stablecoin on FlowCortex L1)

- **`asset_symbol`**: `FloweR`
- **`asset_type`**: native stablecoin (or token, if chain model says tokenized)
- **`asset_contract_or_module_id`**: required if not native coin
- **`decimals`**: required
- **`mint/burn_authority_model`**: required for policy checks
- **`pause_or_freeze_flags`**: required for risk policy

Acceptance rule: KeyCortex allowlist is strict to `PROOF` and `FloweR` in MVP.

---

## 5) Endpoint & Environment Inputs

- **`rpc_read_url`**: balance/read endpoint
- **`rpc_write_url`**: tx submit endpoint
- **`event_or_indexer_url`**: tx status / history source
- **`chain_time_source`**: timestamp sync assumptions
- **`environment`**: devnet/testnet/mainnet mapping
- **`rate_limits`**: QPS and burst ceilings
- **`error_code_contract`**: stable machine-readable submit/verify errors

---

## 6) Proof Coordination Inputs (FlowCortex + ProofCortex)

FlowCortex should confirm or provide the following so ProofCortex circuits and wallet validation remain aligned:

- **`proof_input_schema_version`**: versioned schema ID
- **`wallet_verification_fact_format`**: exact fields consumed by circuits
- **`commitment_hash_rule`**: formula and hash function for commitment generation
  - current target pattern: `hash(wallet_address + challenge + result + chain_slug + tx_hash?)`
- **`domain_separator_alignment`**: must match wallet signing domain tags
- **`attestation_timestamp_tolerance_ms`**: accepted clock skew
- **`proof_failure_codes`**: deterministic failure reason mapping

---

## 7) Recommended Canonical Parameter Template (to be filled by FlowCortex)

```yaml
# ── Filled in from flow-cortex/flowcortex-l1/src/chain_params.rs ──────────────
flowcortex_l1:
  chain_slug: flowcortex-l1
  chain_id_numeric: 1337
  network_id: 1337
  genesis_hash: a6e2b404caa93426a8f608aa8e633d63f6a5b1d44772d59fc230bb505bdbb4ff
  protocol_version: v1
  address_scheme: fc-string-v1          # human-readable UTF-8 string addresses
  signature_scheme: ed25519

  domains:
    tx_domain_tag: "flowcortex:tx:v1"
    auth_domain_tag: "flowcortex:auth:v1"
    proof_domain_tag: "flowcortex:proof:v1"

  tx:
    hash_algorithm: sha256
    canonical_serialization: json-canonical-rfc8785
    nonce_model: commitment-hash-idempotent   # nonce = sha256(canonical(fields))
    fee_model: none-mvp                       # no fees in MVP
    finality_rule: single-node-immediate-1-block

  assets:
    - symbol: PROOF
      type: native
      decimals: 6
      min_transfer_unit: 1                    # 1 micro-PROOF
      fee_payment_support: false
    - symbol: FloweR
      type: native-stablecoin
      contract_or_module_id: built-in         # no separate contract; part of ledger
      decimals: 6
      mint_burn_authority_model: settlement-banks-only
      pause_or_freeze_flags: true

  endpoints:
    rpc_read_url: http://localhost:8082
    rpc_write_url: http://localhost:8082
    event_or_indexer_url: none-mvp            # no indexer in MVP

  proofcortex:
    proof_input_schema_version: proofcortex-mvp-v1
    wallet_verification_fact_format: "{wallet_address: <string>, chain_slug: \"flowcortex-l1\"}"
    commitment_hash_rule: "sha256(json_canonical(witness_fields))"
    domain_separator_alignment: required
    attestation_timestamp_tolerance_ms: 10000
    proof_failure_codes:
      INVALID_COMMITMENT: 4001
      ANCHOR_MISMATCH: 4002
      TIMESTAMP_EXPIRED: 4003
      SCHEMA_VIOLATION: 4004
```

---

## 8) Immediate asks to FlowCortex team

Please provide these first (blocking set):

1. `chain_id_numeric`, `network_id`, `genesis_hash`
2. Canonical tx serialization + hash algorithm
3. Domain tags for tx/auth/proof
4. Finality rule and tx status lifecycle
5. FloweR asset model details (native vs token + identifier)
6. Proof input schema version + commitment hash rule (with ProofCortex alignment)

Once these are provided, KeyCortex can lock deterministic signing, verification, and proof-circuit input compatibility for FlowCortex L1.
