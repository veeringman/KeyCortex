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
flowcortex_l1:
  chain_slug: flowcortex-l1
  chain_id_numeric: <TBD>
  network_id: <TBD>
  genesis_hash: <TBD>
  protocol_version: <TBD>
  address_scheme: <TBD>
  signature_scheme: ed25519

  domains:
    tx_domain_tag: <TBD>
    auth_domain_tag: <TBD>
    proof_domain_tag: <TBD>

  tx:
    hash_algorithm: <TBD>
    canonical_serialization: <TBD>
    nonce_model: <TBD>
    fee_model: <TBD>
    finality_rule: <TBD>

  assets:
    - symbol: PROOF
      type: native
      decimals: <TBD>
      min_transfer_unit: <TBD>
      fee_payment_support: <TBD>
    - symbol: FloweR
      type: <native|token>
      contract_or_module_id: <TBD>
      decimals: <TBD>
      mint_burn_authority_model: <TBD>
      pause_or_freeze_flags: <TBD>

  endpoints:
    rpc_read_url: <TBD>
    rpc_write_url: <TBD>
    event_or_indexer_url: <TBD>

  proofcortex:
    proof_input_schema_version: <TBD>
    wallet_verification_fact_format: <TBD>
    commitment_hash_rule: <TBD>
    domain_separator_alignment: required
    attestation_timestamp_tolerance_ms: <TBD>
    proof_failure_codes: <TBD>
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
