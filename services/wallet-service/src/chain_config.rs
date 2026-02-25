use axum::{Json, extract::State};
use kc_api_types::{ChainAssetInfo, ChainConfigResponse, ChainDomainTags};
use std::sync::Arc;

use crate::{AppState, ApiResult};

/// Returns the canonical chain configuration for FlowCortex L1.
///
/// This provides clients (Treasury UI, FortressDigital, ProofCortex) with
/// the authoritative chain identity, domain tags, and asset metadata needed
/// for deterministic signing, verification, and proof-circuit alignment.
///
/// MVP: only flowcortex-l1 with PROOF and FloweR.
pub(crate) async fn chain_config(
    State(_state): State<Arc<AppState>>,
) -> ApiResult<ChainConfigResponse> {
    Ok(Json(ChainConfigResponse {
        chain_slug: "flowcortex-l1".to_owned(),
        chain_id_numeric: None, // TBD — awaiting FlowCortex team confirmation
        signature_scheme: "ed25519".to_owned(),
        address_scheme: "sha256-truncated-20".to_owned(),
        domains: ChainDomainTags {
            tx_domain_tag: "keycortex:v1:transaction".to_owned(),
            auth_domain_tag: "keycortex:v1:auth".to_owned(),
            proof_domain_tag: "keycortex:v1:proof".to_owned(),
        },
        assets: vec![
            ChainAssetInfo {
                symbol: "PROOF".to_owned(),
                asset_type: "native".to_owned(),
                decimals: 18,
                fee_payment_support: true,
            },
            ChainAssetInfo {
                symbol: "FloweR".to_owned(),
                asset_type: "native-stablecoin".to_owned(),
                decimals: 6,
                fee_payment_support: false,
            },
        ],
        finality_rule: "deterministic-single-confirmation".to_owned(),
        environment: "devnet".to_owned(),
    }))
}
