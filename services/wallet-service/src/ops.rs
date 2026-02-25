use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use kc_chain_flowcortex::FLOWCORTEX_L1;
use kc_storage::{AuditEventRecord, WalletBindingRecord};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{AppState, ApiResult, bad_request, epoch_ms, internal_error, unauthorized};

#[derive(Debug, Deserialize)]
pub(crate) struct OpsAuditQuery {
    pub(crate) limit: Option<usize>,
    pub(crate) event_type: Option<String>,
    pub(crate) wallet_address: Option<String>,
    pub(crate) outcome: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct OpsAuditResponse {
    pub(crate) events: Vec<AuditEventRecord>,
}

pub(crate) async fn ops_get_binding(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(wallet_address): Path<String>,
) -> ApiResult<WalletBindingRecord> {
    let _ops_user = require_ops_access(
        &state,
        &headers,
        "ops_get_binding",
        Some(wallet_address.as_str()),
    )
    .await?;

    if wallet_address.trim().is_empty() {
        return Err(bad_request("wallet_address is required"));
    }

    let record = if let Some(repo) = &state.postgres_repo {
        match repo.load_wallet_binding(&wallet_address).await {
            Ok(Some(record)) => record,
            Ok(None) => return Err(crate::not_found("wallet binding not found")),
            Err(err) => {
                state.db_fallback_counters.inc_binding_read_failures();
                warn!(
                    "failed to load wallet binding from Postgres for {}: {}. Falling back to RocksDB",
                    wallet_address, err
                );
                state
                    .keystore
                    .load_wallet_binding(&wallet_address)
                    .map_err(internal_error)?
                    .ok_or_else(|| crate::not_found("wallet binding not found"))?
            }
        }
    } else {
        state
            .keystore
            .load_wallet_binding(&wallet_address)
            .map_err(internal_error)?
            .ok_or_else(|| crate::not_found("wallet binding not found"))?
    };

    Ok(Json(record))
}

pub(crate) async fn ops_list_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OpsAuditQuery>,
) -> ApiResult<OpsAuditResponse> {
    let _ops_user = require_ops_access(
        &state,
        &headers,
        "ops_list_audit",
        query.wallet_address.as_deref(),
    )
    .await?;

    let limit = query.limit.unwrap_or(100).clamp(1, 500);

    let events = if let Some(repo) = &state.postgres_repo {
        match repo
            .list_audit_events(
                limit,
                query.event_type.as_deref(),
                query.wallet_address.as_deref(),
                query.outcome.as_deref(),
            )
            .await
        {
            Ok(events) => events,
            Err(err) => {
                state.db_fallback_counters.inc_audit_read_failures();
                warn!(
                    "failed to list audit events from Postgres: {}. Falling back to RocksDB",
                    err
                );
                state
                    .keystore
                    .list_audit_events(
                        limit,
                        query.event_type.as_deref(),
                        query.wallet_address.as_deref(),
                        query.outcome.as_deref(),
                    )
                    .map_err(internal_error)?
            }
        }
    } else {
        state
            .keystore
            .list_audit_events(
                limit,
                query.event_type.as_deref(),
                query.wallet_address.as_deref(),
                query.outcome.as_deref(),
            )
            .map_err(internal_error)?
    };

    Ok(Json(OpsAuditResponse { events }))
}

async fn require_ops_access(
    state: &AppState,
    headers: &HeaderMap,
    operation: &str,
    wallet_address: Option<&str>,
) -> Result<String, (axum::http::StatusCode, Json<crate::ErrorResponse>)> {
    let now = epoch_ms().unwrap_or_default();

    let principal = match crate::auth::parse_authbuddy_principal(headers, state) {
        Ok(principal) => principal,
        Err(message) => {
            crate::auth::append_audit_event(
                state,
                AuditEventRecord {
                    event_id: String::new(),
                    event_type: "ops_access".to_owned(),
                    wallet_address: wallet_address.map(ToOwned::to_owned),
                    user_id: None,
                    chain: Some(FLOWCORTEX_L1.to_owned()),
                    outcome: "denied".to_owned(),
                    message: Some(format!("{operation}: {message}")),
                    timestamp_epoch_ms: now,
                },
            )
            .await;
            return Err(unauthorized("ops access denied"));
        }
    };

    let has_ops_role = principal.roles.iter().any(|role| role == "ops-admin");
    if !has_ops_role {
        crate::auth::append_audit_event(
            state,
            AuditEventRecord {
                event_id: String::new(),
                event_type: "ops_access".to_owned(),
                wallet_address: wallet_address.map(ToOwned::to_owned),
                user_id: Some(principal.user_id.clone()),
                chain: Some(FLOWCORTEX_L1.to_owned()),
                outcome: "denied".to_owned(),
                message: Some(format!("{operation}: missing ops-admin role in JWT claims")),
                timestamp_epoch_ms: now,
            },
        )
        .await;
        return Err(unauthorized("ops access denied"));
    }

    crate::auth::append_audit_event(
        state,
        AuditEventRecord {
            event_id: String::new(),
            event_type: "ops_access".to_owned(),
            wallet_address: wallet_address.map(ToOwned::to_owned),
            user_id: Some(principal.user_id.clone()),
            chain: Some(FLOWCORTEX_L1.to_owned()),
            outcome: "success".to_owned(),
            message: Some(format!("{operation}: access granted")),
            timestamp_epoch_ms: now,
        },
    )
    .await;

    Ok(principal.user_id)
}
