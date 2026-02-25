use reqwest::Client;
use serde::Serialize;
#[derive(Debug, Serialize, Clone)]
pub struct AuthBuddyBindCallback {
    pub user_id: String,
    pub wallet_address: String,
    pub chain: String,
    pub bound_at_epoch_ms: u128,
}

pub trait AuthBuddyCallback: Send + Sync {
    fn callback_url(&self) -> Option<&str>;
    fn notify_bind(&self, payload: &AuthBuddyBindCallback);
}

pub struct DefaultAuthBuddyCallback {
    pub url: Option<String>,
}

impl AuthBuddyCallback for DefaultAuthBuddyCallback {
    fn callback_url(&self) -> Option<&str> {
        self.url.as_deref()
    }
    fn notify_bind(&self, payload: &AuthBuddyBindCallback) {
        if let Some(url) = &self.url {
            let client = Client::new();
            let url = self.url.clone().unwrap_or_default();
            let payload = payload.clone();
            tokio::spawn(async move {
                let res = client.post(url)
                    .json(&payload)
                    .send()
                    .await;
                if let Err(err) = res {
                    tracing::warn!("AuthBuddy callback failed: {}", err);
                }
            });
        }
    }
}
use axum::{
    Json,
    extract::State,
    http::HeaderMap,
};
use axum::http::StatusCode;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use kc_api_types::{AuthBindRequest, AuthBindResponse, AuthChallengeResponse, AuthVerifyRequest, AuthVerifyResponse};
use kc_chain_flowcortex::FLOWCORTEX_L1;
use kc_crypto::{Ed25519Signer, decrypt_key_material};
use kc_storage::{AuditEventRecord, Keystore, WalletBindingRecord};
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::warn;
use uuid::Uuid;

use crate::{AppState, ApiResult, ChallengeRecord, bad_request, epoch_ms, from_hex, internal_error, unauthorized};

#[derive(Debug, Deserialize)]
struct AuthBuddyClaims {
    sub: String,
    roles: Option<Vec<String>>,
    role: Option<String>,
    exp: Option<u64>,
    iss: Option<String>,
    aud: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthPrincipal {
    pub(crate) user_id: String,
    pub(crate) roles: Vec<String>,
}


pub(crate) async fn auth_challenge(
    State(state): State<Arc<AppState>>,
) -> ApiResult<AuthChallengeResponse> {
    let now = epoch_ms().map_err(internal_error)?;
    let challenge = Uuid::new_v4().to_string();
    let expires_in: u64 = 300; // 5 minutes
    let expires_at = now + (expires_in as u128 * 1000);

    let record = ChallengeRecord {
        issued_at_epoch_ms: now,
        expires_at_epoch_ms: expires_at,
        used: false,
        used_at_epoch_ms: None,
    };

    {
        let mut store = state.challenge_store.write().await;
        store.insert(challenge.clone(), record);
    }

    if let Some(repo) = &state.postgres_repo {
        if let Err(err) = repo.upsert_challenge(&challenge, now, expires_at).await {
            state.db_fallback_counters.inc_challenge_persist_failures();
            warn!("failed to persist challenge in Postgres: {}", err);
        }
    }

    Ok(Json(AuthChallengeResponse {
        challenge,
        expires_in,
    }))
}

pub(crate) async fn auth_verify(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AuthVerifyRequest>,
) -> ApiResult<AuthVerifyResponse> {
    if request.wallet_address.trim().is_empty() {
        return Err(bad_request("wallet_address is required"));
    }

    if request.challenge.trim().is_empty() {
        return Err(bad_request("challenge is required"));
    }

    if request.signature.trim().is_empty() {
        return Err(bad_request("signature is required"));
    }

    let now = epoch_ms().map_err(internal_error)?;

    {
        let mut store = state.challenge_store.write().await;
        let record = store
            .get_mut(&request.challenge)
            .ok_or_else(|| bad_request("challenge not found"))?;

        if record.used {
            return Err(bad_request("challenge already used"));
        }

        if now > record.expires_at_epoch_ms {
            record.used = true;
            record.used_at_epoch_ms = Some(now);
            if let Some(repo) = &state.postgres_repo {
                if let Err(err) = repo.mark_challenge_used(&request.challenge, now).await {
                    state.db_fallback_counters.inc_challenge_mark_used_failures();
                    warn!("failed to mark challenge used in Postgres: {}", err);
                }
            }
            return Err(bad_request("challenge expired"));
        }

        record.used = true;
        record.used_at_epoch_ms = Some(now);
    }

    let encrypted_key = state
        .keystore
        .load_encrypted_key(&request.wallet_address)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| bad_request("wallet not found"))?;

    let mut secret_key = decrypt_key_material(&encrypted_key, state.encryption_key.as_ref())
        .map_err(internal_error)?;

    let signer = Ed25519Signer::from_secret_key_bytes(secret_key);
    secret_key.fill(0);
    let derived_wallet_address = signer.wallet_address();
    if derived_wallet_address != request.wallet_address {
        return Err(bad_request("wallet address mismatch"));
    }

    let signature_bytes = from_hex(&request.signature)
        .map_err(|e| bad_request(&format!("invalid signature hex: {e}")))?;

    let valid = signer
        .verify(
            request.challenge.as_bytes(),
            kc_api_types::SignPurpose::Auth,
            &signature_bytes,
        )
        .map_err(internal_error)?;

    if let Some(repo) = &state.postgres_repo {
        if let Err(err) = repo.mark_challenge_used(&request.challenge, now).await {
            state.db_fallback_counters.inc_challenge_mark_used_failures();
            warn!("failed to mark challenge used in Postgres: {}", err);
        }
    }

    Ok(Json(AuthVerifyResponse {
        valid,
        wallet_address: request.wallet_address,
        verified_at_epoch_ms: now,
    }))
}

pub(crate) async fn auth_bind(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<AuthBindRequest>,
) -> ApiResult<AuthBindResponse> {
    let now = epoch_ms().map_err(internal_error)?;

    let principal = match parse_authbuddy_principal(&headers, &state) {
        Ok(p) => p,
        Err(msg) => return Err(unauthorized(&msg)),
    };

    if request.wallet_address.trim().is_empty() {
        return Err(bad_request("wallet_address is required"));
    }

    if request.chain != FLOWCORTEX_L1 {
        return Err(bad_request("unsupported chain; only flowcortex-l1 is supported"));
    }

    let user_id = principal.user_id;

    let wallet_exists = state
        .keystore
        .load_encrypted_key(&request.wallet_address)
        .await
        .map_err(internal_error)?
        .is_some();
    if !wallet_exists {
        return Err(bad_request("wallet not found"));
    }

    let binding = WalletBindingRecord {
        wallet_address: request.wallet_address.clone(),
        user_id: user_id.clone(),
        chain: request.chain.clone(),
        last_verified_epoch_ms: now,
    };

    state
        .keystore
        .save_wallet_binding(&binding)
        .map_err(internal_error)?;

    if let Some(repo) = &state.postgres_repo {
        if let Err(err) = repo.save_wallet_binding(&binding).await {
            state.db_fallback_counters.inc_binding_write_failures();
            warn!("failed to persist wallet binding in Postgres: {}", err);
        }
    }

    append_audit_event(
        &state,
        AuditEventRecord {
            event_id: String::new(),
            event_type: "auth_bind".to_owned(),
            wallet_address: Some(request.wallet_address.clone()),
            user_id: Some(user_id.clone()),
            chain: Some(request.chain.clone()),
            outcome: "success".to_owned(),
            message: Some("wallet binding persisted".to_owned()),
            timestamp_epoch_ms: now,
        },
    )
    .await;

    // Notify AuthBuddy callback if configured
    if let Some(callback) = &state.authbuddy_callback {
        let payload = AuthBuddyBindCallback {
            user_id: user_id.clone(),
            wallet_address: request.wallet_address.clone(),
            chain: request.chain.clone(),
            bound_at_epoch_ms: now,
        };
        callback.notify_bind(&payload);
    }

    Ok(Json(AuthBindResponse {
        bound: true,
        user_id,
        wallet_address: request.wallet_address,
        chain: request.chain,
        bound_at_epoch_ms: now,
    }))
}

pub(crate) fn parse_authbuddy_principal(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<AuthPrincipal, String> {
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "missing Authorization header".to_owned())?;

    if !auth_header.starts_with("Bearer ") {
        return Err("invalid Authorization format".to_owned());
    }

    let token = auth_header.trim_start_matches("Bearer ").trim();
    if token.is_empty() {
        return Err("missing bearer token".to_owned());
    }

    let jwks_snapshot = state
        .authbuddy_jwks
        .read()
        .ok()
        .and_then(|guard| (*guard).clone());

    let claims = if let Some(jwks) = jwks_snapshot.as_ref() {
        decode_authbuddy_rs256_claims(token, jwks)?
    } else {
        decode_authbuddy_hs256_claims(token, state.authbuddy_jwt_secret.as_ref())?
    };

    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs();

    let exp = claims
        .exp
        .ok_or_else(|| "missing AuthBuddy JWT exp claim".to_owned())?;
    if exp <= now_epoch {
        return Err("expired AuthBuddy JWT".to_owned());
    }

    if let Some(expected_issuer) = &state.authbuddy_expected_issuer {
        let issuer = claims
            .iss
            .as_deref()
            .ok_or_else(|| "missing AuthBuddy JWT iss claim".to_owned())?;
        if issuer != expected_issuer.as_ref() {
            return Err("invalid AuthBuddy JWT issuer".to_owned());
        }
    }

    if let Some(expected_audience) = &state.authbuddy_expected_audience {
        let audience = claims
            .aud
            .as_deref()
            .ok_or_else(|| "missing AuthBuddy JWT aud claim".to_owned())?;
        if audience != expected_audience.as_ref() {
            return Err("invalid AuthBuddy JWT audience".to_owned());
        }
    }

    let user_id = claims.sub.trim().to_owned();
    if user_id.is_empty() {
        return Err("invalid AuthBuddy JWT subject".to_owned());
    }

    let mut roles = claims.roles.unwrap_or_default();
    if let Some(role) = claims.role {
        for entry in role.split(',') {
            let value = entry.trim();
            if !value.is_empty() {
                roles.push(value.to_owned());
            }
        }
    }

    Ok(AuthPrincipal { user_id, roles })
}

fn decode_authbuddy_hs256_claims(token: &str, jwt_secret: &str) -> Result<AuthBuddyClaims, String> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false;
    validation.required_spec_claims.clear();

    let token_data = decode::<AuthBuddyClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| "invalid AuthBuddy JWT".to_owned())?;

    Ok(token_data.claims)
}

fn decode_authbuddy_rs256_claims(token: &str, jwks: &JwkSet) -> Result<AuthBuddyClaims, String> {
    let header = decode_header(token).map_err(|_| "invalid AuthBuddy JWT header".to_owned())?;

    if header.alg != Algorithm::RS256 {
        return Err("invalid AuthBuddy JWT algorithm; expected RS256".to_owned());
    }

    let kid = header
        .kid
        .ok_or_else(|| "missing AuthBuddy JWT kid header".to_owned())?;

    let jwk = jwks
        .keys
        .iter()
        .find(|entry| entry.common.key_id.as_deref() == Some(kid.as_str()))
        .ok_or_else(|| "no matching JWK found for token kid".to_owned())?;

    let decoding_key = DecodingKey::from_jwk(jwk)
        .map_err(|_| "unable to construct decoding key from JWK".to_owned())?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = false;
    validation.required_spec_claims.clear();

    let token_data = decode::<AuthBuddyClaims>(token, &decoding_key, &validation)
        .map_err(|_| "invalid AuthBuddy JWT".to_owned())?;

    Ok(token_data.claims)
}

pub(crate) async fn append_audit_event(state: &AppState, event: AuditEventRecord) {
    let event_for_postgres = event.clone();
    let _ = state.keystore.append_audit_event(event);
    if let Some(repo) = &state.postgres_repo {
        if let Err(err) = repo.append_audit_event(&event_for_postgres).await {
            state.db_fallback_counters.inc_audit_write_failures();
            warn!("failed to append audit event to Postgres: {}", err);
        }
    }
}
