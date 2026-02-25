<img src="./keycortex_logo.png" alt="KeyCortex Logo" width="180" />

# FlowCortex L1 Integration Change Request (from KeyCortex)

Last Updated: 2026-02-25
Requested Chain: flowcortex-l1
MVP Assets: PROOF, FloweR

## Objective

This document is the actionable request packet for the FlowCortex team to finalize parameters required by KeyCortex wallet signing, transaction submission, and proof alignment.

## Blocking Inputs Required from FlowCortex

1) Chain Identity
- chain_slug = flowcortex-l1
- chain_id_numeric
- network_id
- genesis_hash
- protocol_version
- address_scheme
- signature_scheme (Ed25519 in MVP)

2) Signing and Transaction Domain
- tx_domain_tag
- auth_domain_tag
- proof_domain_tag
- canonical_serialization format
- tx_hash_algorithm
- nonce model
- fee model
- finality rule

3) Asset Metadata
- PROOF: decimals, min_transfer_unit, fee_payment_support
- FloweR: native vs token model, contract_or_module_id (if token), decimals, mint/burn authority, pause/freeze flags

4) Network Endpoints
- rpc_read_url
- rpc_write_url
- event_or_indexer_url
- environment mapping (devnet/testnet/mainnet)
- error code contract
- rate limits

5) Proof Alignment (with ProofCortex)
- proof_input_schema_version
- wallet_verification_fact_format
- commitment_hash_rule
- timestamp tolerance
- proof failure code mapping

## Required Acceptance Criteria

- A single canonical tuple must be published and immutable for current release:
  (chain_slug, chain_id_numeric, genesis_hash, protocol_version)
- Deterministic signing verification must match on KeyCortex and FlowCortex.
- MVP policy enforcement must allow only:
  - chain = flowcortex-l1
  - assets = PROOF, FloweR

## Delivery Format Expected from FlowCortex

Please return values in this schema:

```yaml
flowcortex_l1:
  chain_slug: flowcortex-l1
  chain_id_numeric: <value>
  network_id: <value>
  genesis_hash: <value>
  protocol_version: <value>
  address_scheme: <value>
  signature_scheme: ed25519

  domains:
    tx_domain_tag: <value>
    auth_domain_tag: <value>
    proof_domain_tag: <value>

  tx:
    canonical_serialization: <value>
    hash_algorithm: <value>
    nonce_model: <value>
    fee_model: <value>
    finality_rule: <value>

  assets:
    - symbol: PROOF
      decimals: <value>
      min_transfer_unit: <value>
      fee_payment_support: <value>
    - symbol: FloweR
      type: <native|token>
      contract_or_module_id: <value_if_token>
      decimals: <value>
      mint_burn_authority_model: <value>
      pause_or_freeze_flags: <value>

  endpoints:
    rpc_read_url: <value>
    rpc_write_url: <value>
    event_or_indexer_url: <value>
    environment_mapping: <value>
    error_code_contract: <value>
    rate_limits: <value>

  proof_alignment:
    proof_input_schema_version: <value>
    wallet_verification_fact_format: <value>
    commitment_hash_rule: <value>
    timestamp_tolerance_ms: <value>
    proof_failure_codes: <value>
```

## Operational Note

KeyCortex can lock deterministic signing and production-grade adapter behavior immediately after this parameter sheet is returned and frozen by FlowCortex + ProofCortex.
