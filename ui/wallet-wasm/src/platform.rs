//! Platform integration handlers.
//!
//! FlowCortex chain config, FortressDigital wallet status,
//! ProofCortex commitment, and ops health endpoints.
//! Extend by adding new platform integration functions.

use crate::api;
use crate::dom::{self, Elements};

/// GET /chain/config
pub async fn on_chain_config(els: &Elements) {
    match api::request("/chain/config", "GET", None).await {
        Ok(result) => api::set_result(&els.chain_config_result, &result),
        Err(e) => api::set_result_error(&els.chain_config_result, &e),
    }
}

/// POST /fortressdigital/wallet-status
pub async fn on_wallet_status(els: &Elements) {
    let addr = dom::get_input_value(&els.fd_wallet_address);
    if addr.is_empty() {
        api::set_result_error(&els.wallet_status_result, "wallet address required");
        return;
    }
    let body = serde_json::json!({
        "wallet_address": addr,
        "chain": "flowcortex-l1",
    });
    match api::request(
        "/fortressdigital/wallet-status",
        "POST",
        Some(body.to_string()),
    )
    .await
    {
        Ok(result) => api::set_result(&els.wallet_status_result, &result),
        Err(e) => api::set_result_error(&els.wallet_status_result, &e),
    }
}

/// POST /proofcortex/commitment
pub async fn on_commitment(els: &Elements) {
    let addr = dom::get_input_value(&els.pc_wallet_address);
    let challenge = dom::get_input_value(&els.pc_challenge);
    if addr.is_empty() {
        api::set_result_error(&els.commitment_result, "wallet address required");
        return;
    }
    if challenge.is_empty() {
        api::set_result_error(&els.commitment_result, "challenge required");
        return;
    }

    let mut body = serde_json::json!({
        "wallet_address": addr,
        "challenge": challenge,
        "verification_result": true,
        "chain": "flowcortex-l1",
    });

    let tx_hash = dom::get_input_value(&els.pc_tx_hash);
    if !tx_hash.is_empty() {
        body["tx_hash"] = serde_json::Value::String(tx_hash);
    }

    match api::request("/proofcortex/commitment", "POST", Some(body.to_string())).await {
        Ok(result) => api::set_result(&els.commitment_result, &result),
        Err(e) => api::set_result_error(&els.commitment_result, &e),
    }
}

/// GET /health
pub async fn on_ops_health(els: &Elements) {
    match api::request("/health", "GET", None).await {
        Ok(result) => api::set_result(&els.ops_result, &result),
        Err(e) => api::set_result_error(&els.ops_result, &e),
    }
}

/// GET /readyz
pub async fn on_ops_readyz(els: &Elements) {
    match api::request("/readyz", "GET", None).await {
        Ok(result) => api::set_result(&els.ops_result, &result),
        Err(e) => api::set_result_error(&els.ops_result, &e),
    }
}

/// GET /startupz
pub async fn on_ops_startupz(els: &Elements) {
    match api::request("/startupz", "GET", None).await {
        Ok(result) => api::set_result(&els.ops_result, &result),
        Err(e) => api::set_result_error(&els.ops_result, &e),
    }
}
