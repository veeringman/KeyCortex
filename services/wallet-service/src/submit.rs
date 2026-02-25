use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use kc_api_types::{
    AssetSymbol, ChainId, SignPurpose, WalletAddress, WalletNonceResponse, WalletSubmitRequest,
    WalletSubmitResponse, WalletTxStatusResponse,
};
use kc_chain_client::{ChainAdapter, SubmitTxRequest, TxStatusRequest};
use kc_chain_flowcortex::{FLOWCORTEX_L1, FlowCortexAdapter};
use kc_crypto::{Ed25519Signer, Signer, decrypt_key_material};
use kc_storage::{Keystore, SubmitIdempotencyRecord, SubmittedTxRecord, WalletNonceRecord};
use serde::Deserialize;
use tracing::warn;

use std::sync::Arc;

use crate::{AppState, ApiResult, bad_request, epoch_ms, internal_error, to_hex};

#[derive(Debug, Deserialize)]
pub(crate) struct WalletNonceQuery {
    wallet_address: String,
}

pub(crate) async fn wallet_nonce(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WalletNonceQuery>,
) -> ApiResult<WalletNonceResponse> {
    if query.wallet_address.trim().is_empty() {
        return Err(bad_request("wallet_address is required"));
    }

    let wallet_exists = state
        .keystore
        .load_encrypted_key(&query.wallet_address)
        .await
        .map_err(internal_error)?
        .is_some();

    if !wallet_exists {
        return Err(bad_request("wallet not found"));
    }

    let last_nonce = state
        .keystore
        .load_wallet_nonce(&query.wallet_address)
        .map_err(internal_error)?
        .map(|record| record.last_nonce)
        .unwrap_or(0);

    Ok(Json(WalletNonceResponse {
        wallet_address: query.wallet_address,
        last_nonce,
        next_nonce: last_nonce.saturating_add(1),
    }))
}

pub(crate) async fn wallet_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<WalletSubmitRequest>,
) -> ApiResult<WalletSubmitResponse> {
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if let Some(key) = idempotency_key.as_deref() {
        {
            let cache = state.submit_idempotency_cache.read().await;
            if let Some(existing) = cache.get(key) {
                return Ok(Json(existing.clone()));
            }
        }

        if let Some(existing) = state
            .keystore
            .load_submit_idempotency(key)
            .map_err(internal_error)?
        {
            let response = WalletSubmitResponse {
                accepted: existing.accepted,
                tx_hash: existing.tx_hash,
                signature: existing.signature,
            };
            let mut cache = state.submit_idempotency_cache.write().await;
            cache.insert(key.to_owned(), response.clone());
            return Ok(Json(response));
        }
    }

    if request.from.trim().is_empty() {
        return Err(bad_request("from is required"));
    }
    if request.to.trim().is_empty() {
        return Err(bad_request("to is required"));
    }
    if request.amount.trim().is_empty() {
        return Err(bad_request("amount is required"));
    }
    if request.nonce == 0 {
        return Err(bad_request("nonce must be greater than 0"));
    }
    if request.chain != FLOWCORTEX_L1 {
        return Err(bad_request("unsupported chain for MVP; only flowcortex-l1 is enabled"));
    }
    if request.asset != "PROOF" && request.asset != "FloweR" {
        return Err(bad_request("unsupported asset for MVP; only PROOF and FloweR are enabled"));
    }

    let encrypted_key = state
        .keystore
        .load_encrypted_key(&request.from)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| bad_request("source wallet not found"))?;

    let mut secret_key = decrypt_key_material(&encrypted_key, state.encryption_key.as_ref())
        .map_err(internal_error)?;
    let signer = Ed25519Signer::from_secret_key_bytes(secret_key);
    secret_key.fill(0);

    if signer.wallet_address() != request.from {
        return Err(bad_request("source wallet address does not match custodied key"));
    }

    {
        let mut nonce_state = state.submit_nonce_state.write().await;
        let mut last_nonce = nonce_state.get(&request.from).copied().unwrap_or(0);
        if last_nonce == 0 {
            last_nonce = state
                .keystore
                .load_wallet_nonce(&request.from)
                .map_err(internal_error)?
                .map(|record| record.last_nonce)
                .unwrap_or(0);
        }

        if request.nonce <= last_nonce {
            return Err(bad_request(
                "nonce replay detected; nonce must be strictly increasing per wallet",
            ));
        }

        nonce_state.insert(request.from.clone(), request.nonce);
    }

    let payload = format!(
        "from={};to={};amount={};asset={};chain={};nonce={}",
        request.from, request.to, request.amount, request.asset, request.chain, request.nonce
    );

    let signature = signer
        .sign(payload.as_bytes(), SignPurpose::Transaction)
        .map_err(internal_error)?;
    let signature_hex = to_hex(&signature);

    let adapter = FlowCortexAdapter;
    let result = adapter
        .submit_transaction(SubmitTxRequest {
            from: WalletAddress(request.from.clone()),
            to: WalletAddress(request.to.clone()),
            amount: request.amount.clone(),
            asset: AssetSymbol(request.asset.clone()),
            chain: ChainId(request.chain.clone()),
            signed_payload: signature_hex.clone(),
        })
        .await
        .map_err(internal_error)?;

    let response = WalletSubmitResponse {
        accepted: result.accepted,
        tx_hash: result.tx_hash,
        signature: signature_hex,
    };

    let now = epoch_ms().map_err(internal_error)?;

    state
        .keystore
        .save_submitted_tx(&SubmittedTxRecord {
            tx_hash: response.tx_hash.clone(),
            status: if response.accepted {
                "submitted".to_owned()
            } else {
                "rejected".to_owned()
            },
            accepted: response.accepted,
            chain: request.chain.clone(),
            from: request.from.clone(),
            to: request.to.clone(),
            asset: request.asset.clone(),
            amount: request.amount.clone(),
            submitted_at_epoch_ms: now,
        })
        .map_err(internal_error)?;

    state
        .keystore
        .save_wallet_nonce(&WalletNonceRecord {
            wallet_address: request.from.clone(),
            last_nonce: request.nonce,
            updated_at_epoch_ms: now,
        })
        .map_err(internal_error)?;

    if let Some(key) = idempotency_key {
        state
            .keystore
            .save_submit_idempotency(&SubmitIdempotencyRecord {
                idempotency_key: key.clone(),
                accepted: response.accepted,
                tx_hash: response.tx_hash.clone(),
                signature: response.signature.clone(),
                created_at_epoch_ms: now,
            })
            .map_err(internal_error)?;

        let mut cache = state.submit_idempotency_cache.write().await;
        cache.insert(key, response.clone());
    }

    Ok(Json(response))
}

pub(crate) async fn wallet_tx_status(
    State(state): State<Arc<AppState>>,
    Path(tx_hash): Path<String>,
) -> ApiResult<WalletTxStatusResponse> {
    if tx_hash.trim().is_empty() {
        return Err(bad_request("tx_hash is required"));
    }

    let mut record = state
        .keystore
        .load_submitted_tx(&tx_hash)
        .map_err(internal_error)?
        .ok_or_else(|| bad_request("transaction not found"))?;

    if record.chain == FLOWCORTEX_L1 {
        let adapter = FlowCortexAdapter;
        match adapter
            .get_transaction_status(TxStatusRequest {
                tx_hash: record.tx_hash.clone(),
                chain: ChainId(record.chain.clone()),
            })
            .await
        {
            Ok(status) => {
                record.status = status.status;
                record.accepted = status.accepted;
                state
                    .keystore
                    .save_submitted_tx(&record)
                    .map_err(internal_error)?;
            }
            Err(err) => {
                warn!(
                    "failed to refresh tx status for {}: {}. Returning last persisted state",
                    record.tx_hash, err
                );
            }
        }
    }

    Ok(Json(WalletTxStatusResponse {
        tx_hash: record.tx_hash,
        status: record.status,
        accepted: record.accepted,
        chain: record.chain,
        from: record.from,
        to: record.to,
        asset: record.asset,
        amount: record.amount,
        submitted_at_epoch_ms: record.submitted_at_epoch_ms,
    }))
}
