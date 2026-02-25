use axum::{Json, extract::State};
use kc_api_types::{ProofCortexCommitmentRequest, ProofCortexCommitmentResponse};
use kc_storage::Keystore;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{AppState, ApiResult, bad_request, epoch_ms, internal_error, to_hex};

/// Domain separator for ProofCortex commitment generation.
/// Aligns with FlowCortex proof_domain_tag.
const PROOF_DOMAIN_SEPARATOR: &str = "keycortex:proof:v1";

/// Schema version for proof input compatibility.
const PROOF_INPUT_SCHEMA_VERSION: &str = "1.0.0";

/// Generate a ZKP-compatible commitment hash for ProofCortex circuits.
///
/// Commitment formula:
///   commitment = sha256(domain_separator + wallet_address + challenge + result + chain + tx_hash?)
///
/// This provides a deterministic, verifiable fact that ProofCortex can use
/// as input to STARK proof generation proving:
///   - wallet was verified
///   - wallet is bound to user
///   - signature authenticity
pub(crate) async fn proofcortex_commitment(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProofCortexCommitmentRequest>,
) -> ApiResult<ProofCortexCommitmentResponse> {
    if request.wallet_address.trim().is_empty() {
        return Err(bad_request("wallet_address is required"));
    }

    if request.challenge.trim().is_empty() {
        return Err(bad_request("challenge is required"));
    }

    if request.chain.trim().is_empty() {
        return Err(bad_request("chain is required"));
    }

    // Verify wallet exists in our keystore
    let wallet_exists = state
        .keystore
        .load_encrypted_key(&request.wallet_address)
        .await
        .map_err(internal_error)?
        .is_some();

    if !wallet_exists {
        return Err(bad_request("wallet not found"));
    }

    let now = epoch_ms().map_err(internal_error)?;

    // Build deterministic commitment input
    // commitment = hash(domain_separator | wallet_address | challenge | result | chain | tx_hash?)
    let result_str = if request.verification_result {
        "verified"
    } else {
        "unverified"
    };

    let mut hasher = Sha256::new();
    hasher.update(PROOF_DOMAIN_SEPARATOR.as_bytes());
    hasher.update(b":");
    hasher.update(request.wallet_address.as_bytes());
    hasher.update(b":");
    hasher.update(request.challenge.as_bytes());
    hasher.update(b":");
    hasher.update(result_str.as_bytes());
    hasher.update(b":");
    hasher.update(request.chain.as_bytes());

    if let Some(tx_hash) = &request.tx_hash {
        hasher.update(b":");
        hasher.update(tx_hash.as_bytes());
    }

    let commitment = to_hex(&hasher.finalize());

    // Audit the commitment generation
    crate::auth::append_audit_event(
        &state,
        kc_storage::AuditEventRecord {
            event_id: String::new(),
            event_type: "proofcortex_commitment".to_owned(),
            wallet_address: Some(request.wallet_address.clone()),
            user_id: None,
            chain: Some(request.chain.clone()),
            outcome: "success".to_owned(),
            message: Some(format!("commitment={}", &commitment[..16])),
            timestamp_epoch_ms: now,
        },
    )
    .await;

    Ok(Json(ProofCortexCommitmentResponse {
        commitment,
        wallet_address: request.wallet_address,
        chain: request.chain,
        verification_result: request.verification_result,
        domain_separator: PROOF_DOMAIN_SEPARATOR.to_owned(),
        proof_input_schema_version: PROOF_INPUT_SCHEMA_VERSION.to_owned(),
        generated_at_epoch_ms: now,
    }))
}
