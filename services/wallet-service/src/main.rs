use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet,
};
use kc_api_types::{
    AuthBindRequest, AuthBindResponse, AuthChallengeResponse, AuthVerifyRequest, AuthVerifyResponse,
    SignPurpose, WalletBalanceResponse, WalletCreateResponse, WalletSignRequest, WalletSignResponse,
    WalletSubmitRequest, WalletSubmitResponse,
};
use kc_auth_adapter::{challenge_response, issue_challenge};
use kc_api_types::{AssetSymbol, ChainId, WalletAddress};
use kc_chain_client::{ChainAdapter, SubmitTxRequest};
use kc_chain_flowcortex::{FLOWCORTEX_L1, FlowCortexAdapter};
use kc_crypto::{Ed25519Signer, Signer, decrypt_key_material, encrypt_key_material};
use kc_storage::{AuditEventRecord, Keystore, RocksDbKeystore, WalletBindingRecord};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock as TokioRwLock;
use tracing::{info, warn};

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    status: &'static str,
    auth_mode: String,
    jwks_source: Option<String>,
    jwks_loaded: bool,
    last_jwks_refresh_epoch_ms: Option<u128>,
    last_jwks_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct VersionResponse {
    service: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct ReadinessResponse {
    service: &'static str,
    ready: bool,
    keystore_ready: bool,
    auth_ready: bool,
    auth_mode: String,
    jwks_reachable: Option<bool>,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Deserialize)]
struct WalletBalanceQuery {
    wallet_address: String,
    asset: Option<String>,
    chain: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpsAuditQuery {
    limit: Option<usize>,
    event_type: Option<String>,
    wallet_address: Option<String>,
    outcome: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpsAuditResponse {
    events: Vec<AuditEventRecord>,
}

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
struct AuthPrincipal {
    user_id: String,
    roles: Vec<String>,
}

#[derive(Debug, Clone)]
struct JwksRuntimeStatus {
    source: Option<String>,
    loaded: bool,
    last_refresh_epoch_ms: Option<u128>,
    last_error: Option<String>,
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

#[derive(Debug, Clone)]
struct ChallengeRecord {
    issued_at_epoch_ms: u128,
    expires_at_epoch_ms: u128,
    used: bool,
    used_at_epoch_ms: Option<u128>,
}

#[derive(Clone)]
struct AppState {
    keystore: Arc<RocksDbKeystore>,
    encryption_key: Arc<str>,
    authbuddy_jwt_secret: Arc<str>,
    authbuddy_jwks: Arc<StdRwLock<Option<JwkSet>>>,
    jwks_status: Arc<StdRwLock<JwksRuntimeStatus>>,
    authbuddy_expected_issuer: Option<Arc<str>>,
    authbuddy_expected_audience: Option<Arc<str>>,
    challenge_store: Arc<TokioRwLock<HashMap<String, ChallengeRecord>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let keystore_path = env::var("KEYCORTEX_KEYSTORE_PATH")
        .unwrap_or_else(|_| "./data/keystore/rocksdb".to_owned());
    if let Some(parent) = std::path::Path::new(&keystore_path).parent() {
        fs::create_dir_all(parent)?;
    }

    let keystore = RocksDbKeystore::open_default(&keystore_path)?;

    let authbuddy_jwks_path = env::var("AUTHBUDDY_JWKS_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let authbuddy_jwks_url = env::var("AUTHBUDDY_JWKS_URL")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let authbuddy_jwks_refresh_seconds = env::var("AUTHBUDDY_JWKS_REFRESH_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(60)
        .max(10);

    let initial_jwks = env::var("AUTHBUDDY_JWKS_JSON")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .and_then(|json| serde_json::from_str::<JwkSet>(&json).ok());

    let initial_jwks_source = if authbuddy_jwks_url.is_some() {
        Some("url".to_owned())
    } else if authbuddy_jwks_path.is_some() {
        Some("file".to_owned())
    } else if initial_jwks.is_some() {
        Some("inline".to_owned())
    } else {
        None
    };

    let jwks_status = JwksRuntimeStatus {
        source: initial_jwks_source,
        loaded: initial_jwks.is_some(),
        last_refresh_epoch_ms: initial_jwks
            .as_ref()
            .and_then(|_| epoch_ms().ok()),
        last_error: None,
    };

    let state = AppState {
        keystore: Arc::new(keystore),
        encryption_key: Arc::<str>::from("keycortex-dev-master-key"),
        authbuddy_jwt_secret: Arc::<str>::from(
            env::var("AUTHBUDDY_JWT_SECRET")
                .unwrap_or_else(|_| "authbuddy-dev-secret-change-me".to_owned()),
        ),
        authbuddy_jwks: Arc::new(StdRwLock::new(initial_jwks)),
        jwks_status: Arc::new(StdRwLock::new(jwks_status)),
        authbuddy_expected_issuer: env::var("AUTHBUDDY_JWT_ISSUER")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(Arc::<str>::from),
        authbuddy_expected_audience: env::var("AUTHBUDDY_JWT_AUDIENCE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(Arc::<str>::from),
        challenge_store: Arc::new(TokioRwLock::new(HashMap::new())),
    };

    if authbuddy_jwks_url.is_some() || authbuddy_jwks_path.is_some() {
        let jwks_cache = Arc::clone(&state.authbuddy_jwks);
        let jwks_status = Arc::clone(&state.jwks_status);
        let jwks_url = authbuddy_jwks_url.clone();
        let jwks_path = authbuddy_jwks_path.clone();
        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .ok();
            let mut failure_count: u32 = 0;

            loop {
                let mut refreshed = false;

                if let (Some(url), Some(http_client)) = (jwks_url.as_ref(), client.as_ref()) {
                    match fetch_jwks_from_url(http_client, url).await {
                        Ok(parsed) => {
                            if let Ok(mut guard) = jwks_cache.write() {
                                *guard = Some(parsed);
                            }
                            if let Ok(mut status) = jwks_status.write() {
                                status.loaded = true;
                                status.last_refresh_epoch_ms = epoch_ms().ok();
                                status.last_error = None;
                            }
                            info!("reloaded AuthBuddy JWKS from URL {}", url);
                            refreshed = true;
                        }
                        Err(err) => {
                            if let Ok(mut status) = jwks_status.write() {
                                status.last_error = Some(format!("url refresh failed: {err}"));
                            }
                            warn!("failed to refresh AuthBuddy JWKS from URL {}: {}", url, err);
                        }
                    }
                }

                if !refreshed {
                    if let Some(path) = jwks_path.as_ref() {
                        match fs::read_to_string(path) {
                            Ok(content) => match serde_json::from_str::<JwkSet>(&content) {
                                Ok(parsed) => {
                                    if let Ok(mut guard) = jwks_cache.write() {
                                        *guard = Some(parsed);
                                    }
                                    if let Ok(mut status) = jwks_status.write() {
                                        status.loaded = true;
                                        status.last_refresh_epoch_ms = epoch_ms().ok();
                                        status.last_error = None;
                                    }
                                    info!("reloaded AuthBuddy JWKS from file {}", path);
                                    refreshed = true;
                                }
                                Err(err) => {
                                    if let Ok(mut status) = jwks_status.write() {
                                        status.last_error = Some(format!("file parse failed: {err}"));
                                    }
                                    warn!("failed to parse AuthBuddy JWKS file {}: {}", path, err);
                                }
                            },
                            Err(err) => {
                                if let Ok(mut status) = jwks_status.write() {
                                    status.last_error = Some(format!("file read failed: {err}"));
                                }
                                warn!("failed to read AuthBuddy JWKS file {}: {}", path, err);
                            }
                        }
                    }
                }

                let sleep_seconds = if refreshed {
                    failure_count = 0;
                    authbuddy_jwks_refresh_seconds
                } else {
                    failure_count = failure_count.saturating_add(1);
                    let backoff = authbuddy_jwks_refresh_seconds.saturating_mul(1_u64 << failure_count.min(5));
                    backoff.min(300)
                };

                tokio::time::sleep(Duration::from_secs(sleep_seconds)).await;
            }
        });
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/readyz", get(readyz))
        .route("/version", get(version))
        .route("/wallet/create", post(wallet_create))
        .route("/wallet/sign", post(wallet_sign))
        .route("/wallet/submit", post(wallet_submit))
        .route("/wallet/balance", get(wallet_balance))
        .route("/auth/challenge", post(auth_challenge))
        .route("/auth/verify", post(auth_verify))
        .route("/auth/bind", post(auth_bind))
        .route("/ops/bindings/{wallet_address}", get(ops_get_binding))
        .route("/ops/audit", get(ops_list_audit))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("wallet-service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let status_snapshot = state
        .jwks_status
        .read()
        .ok()
        .map(|status| status.clone())
        .unwrap_or(JwksRuntimeStatus {
            source: None,
            loaded: false,
            last_refresh_epoch_ms: None,
            last_error: Some("jwks status unavailable".to_owned()),
        });

    let auth_mode = if status_snapshot.loaded {
        "rs256-jwks"
    } else {
        "hs256-fallback"
    }
    .to_owned();

    Json(HealthResponse {
        service: "wallet-service",
        status: "ok",
        auth_mode,
        jwks_source: status_snapshot.source,
        jwks_loaded: status_snapshot.loaded,
        last_jwks_refresh_epoch_ms: status_snapshot.last_refresh_epoch_ms,
        last_jwks_error: status_snapshot.last_error,
    })
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        service: "wallet-service",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let keystore_ready = state
        .keystore
        .load_encrypted_key("__readiness_probe__")
        .await
        .is_ok();

    let jwks_snapshot = state
        .jwks_status
        .read()
        .ok()
        .map(|status| status.clone())
        .unwrap_or(JwksRuntimeStatus {
            source: None,
            loaded: false,
            last_refresh_epoch_ms: None,
            last_error: Some("jwks status unavailable".to_owned()),
        });

    let has_hs256_fallback = !state.authbuddy_jwt_secret.trim().is_empty();
    let auth_ready = jwks_snapshot.loaded || has_hs256_fallback;
    let auth_mode = if jwks_snapshot.loaded {
        "rs256-jwks"
    } else {
        "hs256-fallback"
    }
    .to_owned();

    let jwks_reachable = match jwks_snapshot.source.as_deref() {
        Some("url") => Some(jwks_snapshot.last_error.is_none()),
        _ => None,
    };

    let ready = keystore_ready && auth_ready && jwks_reachable.unwrap_or(true);
    let reason = if ready {
        None
    } else if !keystore_ready {
        Some("keystore not ready".to_owned())
    } else if jwks_reachable == Some(false) {
        Some("jwks endpoint not reachable".to_owned())
    } else {
        Some("auth mode not ready".to_owned())
    };

    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(ReadinessResponse {
            service: "wallet-service",
            ready,
            keystore_ready,
            auth_ready,
            auth_mode,
            jwks_reachable,
            reason,
        }),
    )
}

async fn wallet_create(State(state): State<AppState>) -> ApiResult<WalletCreateResponse> {
    let signer = Ed25519Signer::new_random();
    let wallet_address = signer.wallet_address();
    let public_key = signer.public_key_hex();

    let encrypted_key = encrypt_key_material(&signer.secret_key_bytes(), state.encryption_key.as_ref())
        .map_err(internal_error)?;

    state
        .keystore
        .save_encrypted_key(&wallet_address, encrypted_key)
        .await
        .map_err(internal_error)?;

    Ok(Json(WalletCreateResponse {
        wallet_address,
        public_key,
        chain: FLOWCORTEX_L1.to_owned(),
    }))
}

async fn wallet_sign(
    State(state): State<AppState>,
    Json(request): Json<WalletSignRequest>,
) -> ApiResult<WalletSignResponse> {
    if request.wallet_address.trim().is_empty() {
        return Err(bad_request("wallet_address is required"));
    }

    if request.payload.trim().is_empty() {
        return Err(bad_request("payload cannot be empty"));
    }

    let payload_bytes = STANDARD
        .decode(request.payload.as_bytes())
        .map_err(|_| bad_request("payload must be valid base64"))?;

    let encrypted_key = state
        .keystore
        .load_encrypted_key(&request.wallet_address)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| bad_request("wallet not found"))?;

    let secret_key = decrypt_key_material(&encrypted_key, state.encryption_key.as_ref())
        .map_err(internal_error)?;

    let signer = Ed25519Signer::from_secret_key_bytes(secret_key);
    let signature_bytes = signer
        .sign(&payload_bytes, request.purpose)
        .map_err(internal_error)?;

    Ok(Json(WalletSignResponse {
        signature: to_hex(&signature_bytes),
    }))
}

async fn wallet_balance(Query(query): Query<WalletBalanceQuery>) -> ApiResult<WalletBalanceResponse> {
    if query.wallet_address.trim().is_empty() {
        return Err(bad_request("wallet_address is required"));
    }

    let chain = query.chain.unwrap_or_else(|| FLOWCORTEX_L1.to_owned());
    if chain != FLOWCORTEX_L1 {
        return Err(bad_request("unsupported chain for MVP; only flowcortex-l1 is enabled"));
    }

    let asset = query.asset.unwrap_or_else(|| "PROOF".to_owned());
    if asset != "PROOF" && asset != "FloweR" {
        return Err(bad_request("unsupported asset for MVP; only PROOF and FloweR are enabled"));
    }

    let adapter = FlowCortexAdapter;
    let result = adapter
        .get_balance(&WalletAddress(query.wallet_address.clone()), &AssetSymbol(asset.clone()))
        .await
        .map_err(internal_error)?;

    Ok(Json(WalletBalanceResponse {
        wallet_address: result.wallet_address.0,
        chain: result.chain.0,
        asset: result.asset.0,
        amount: result.amount,
    }))
}

async fn wallet_submit(
    State(state): State<AppState>,
    Json(request): Json<WalletSubmitRequest>,
) -> ApiResult<WalletSubmitResponse> {
    if request.from.trim().is_empty() {
        return Err(bad_request("from is required"));
    }
    if request.to.trim().is_empty() {
        return Err(bad_request("to is required"));
    }
    if request.amount.trim().is_empty() {
        return Err(bad_request("amount is required"));
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

    let secret_key = decrypt_key_material(&encrypted_key, state.encryption_key.as_ref())
        .map_err(internal_error)?;
    let signer = Ed25519Signer::from_secret_key_bytes(secret_key);

    if signer.wallet_address() != request.from {
        return Err(bad_request("source wallet address does not match custodied key"));
    }

    let payload = format!(
        "from={};to={};amount={};asset={};chain={}",
        request.from, request.to, request.amount, request.asset, request.chain
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

    Ok(Json(WalletSubmitResponse {
        accepted: result.accepted,
        tx_hash: result.tx_hash,
        signature: signature_hex,
    }))
}

async fn auth_challenge(State(state): State<AppState>) -> Json<AuthChallengeResponse> {
    let challenge = issue_challenge(120);
    let now = epoch_ms().unwrap_or_default();
    let expires_at_epoch_ms = now + (challenge.expires_in_seconds as u128 * 1000);

    let mut store = state.challenge_store.write().await;
    store.retain(|_, record| !record.used && record.expires_at_epoch_ms > now);
    store.insert(
        challenge.challenge.clone(),
        ChallengeRecord {
            issued_at_epoch_ms: now,
            expires_at_epoch_ms,
            used: false,
            used_at_epoch_ms: None,
        },
    );

    Json(challenge_response(&challenge))
}

async fn auth_verify(
    State(state): State<AppState>,
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

    let secret_key = decrypt_key_material(&encrypted_key, state.encryption_key.as_ref())
        .map_err(internal_error)?;

    let signer = Ed25519Signer::from_secret_key_bytes(secret_key);
    let derived_wallet_address = signer.wallet_address();
    if derived_wallet_address != request.wallet_address {
        return Err(bad_request("wallet address does not match custodied key"));
    }

    let signature_bytes = from_hex(&request.signature)
        .map_err(|_| bad_request("signature must be valid hex"))?;

    let valid = signer
        .verify(request.challenge.as_bytes(), SignPurpose::Auth, &signature_bytes)
        .map_err(internal_error)?;

    Ok(Json(AuthVerifyResponse {
        valid,
        wallet_address: request.wallet_address,
        verified_at_epoch_ms: now,
    }))
}

async fn auth_bind(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AuthBindRequest>,
) -> ApiResult<AuthBindResponse> {
    let now = epoch_ms().map_err(internal_error)?;

    let principal = match parse_authbuddy_principal(&headers, &state) {
        Ok(principal) => principal,
        Err(message) => {
            append_audit_event(
                &state,
                AuditEventRecord {
                    event_id: String::new(),
                    event_type: "auth_bind".to_owned(),
                    wallet_address: Some(request.wallet_address.clone()),
                    user_id: None,
                    chain: Some(request.chain.clone()),
                    outcome: "denied".to_owned(),
                    message: Some(message.clone()),
                    timestamp_epoch_ms: now,
                },
            );
            return Err(unauthorized(&message));
        }
    };

    if request.wallet_address.trim().is_empty() {
        append_audit_event(
            &state,
            AuditEventRecord {
                event_id: String::new(),
                event_type: "auth_bind".to_owned(),
                wallet_address: Some(request.wallet_address.clone()),
                user_id: None,
                chain: Some(request.chain.clone()),
                outcome: "denied".to_owned(),
                message: Some("wallet_address is required".to_owned()),
                timestamp_epoch_ms: now,
            },
        );
        return Err(bad_request("wallet_address is required"));
    }

    if request.chain != FLOWCORTEX_L1 {
        append_audit_event(
            &state,
            AuditEventRecord {
                event_id: String::new(),
                event_type: "auth_bind".to_owned(),
                wallet_address: Some(request.wallet_address.clone()),
                user_id: None,
                chain: Some(request.chain.clone()),
                outcome: "denied".to_owned(),
                message: Some("unsupported chain for MVP".to_owned()),
                timestamp_epoch_ms: now,
            },
        );
        return Err(bad_request("unsupported chain for MVP; only flowcortex-l1 is enabled"));
    }

    let user_id = principal.user_id;

    let wallet_exists = state
        .keystore
        .load_encrypted_key(&request.wallet_address)
        .await
        .map_err(internal_error)?
        .is_some();

    if !wallet_exists {
        append_audit_event(
            &state,
            AuditEventRecord {
                event_id: String::new(),
                event_type: "auth_bind".to_owned(),
                wallet_address: Some(request.wallet_address.clone()),
                user_id: Some(user_id.clone()),
                chain: Some(request.chain.clone()),
                outcome: "denied".to_owned(),
                message: Some("wallet not found".to_owned()),
                timestamp_epoch_ms: now,
            },
        );
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
    );

    Ok(Json(AuthBindResponse {
        bound: true,
        user_id,
        wallet_address: request.wallet_address,
        chain: request.chain,
        bound_at_epoch_ms: now,
    }))
}

async fn ops_get_binding(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(wallet_address): Path<String>,
) -> ApiResult<WalletBindingRecord> {
    let _ops_user = require_ops_access(
        &state,
        &headers,
        "ops_get_binding",
        Some(wallet_address.as_str()),
    )?;

    if wallet_address.trim().is_empty() {
        return Err(bad_request("wallet_address is required"));
    }

    let record = state
        .keystore
        .load_wallet_binding(&wallet_address)
        .map_err(internal_error)?
        .ok_or_else(|| not_found("wallet binding not found"))?;

    Ok(Json(record))
}

async fn ops_list_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OpsAuditQuery>,
) -> ApiResult<OpsAuditResponse> {
    let _ops_user = require_ops_access(
        &state,
        &headers,
        "ops_list_audit",
        query.wallet_address.as_deref(),
    )?;

    let limit = query.limit.unwrap_or(100).clamp(1, 500);

    let events = state
        .keystore
        .list_audit_events(
            limit,
            query.event_type.as_deref(),
            query.wallet_address.as_deref(),
            query.outcome.as_deref(),
        )
        .map_err(internal_error)?;

    Ok(Json(OpsAuditResponse { events }))
}

fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

fn unauthorized(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

fn not_found(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: err.to_string(),
        }),
    )
}

fn epoch_ms() -> anyhow::Result<u128> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

fn to_hex(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    for byte in input {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn from_hex(input: &str) -> anyhow::Result<Vec<u8>> {
    if input.len() % 2 != 0 {
        anyhow::bail!("hex input length must be even");
    }

    let mut output = Vec::with_capacity(input.len() / 2);
    let bytes = input.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let pair = std::str::from_utf8(&bytes[idx..idx + 2])?;
        let value = u8::from_str_radix(pair, 16)?;
        output.push(value);
        idx += 2;
    }
    Ok(output)
}

fn append_audit_event(state: &AppState, event: AuditEventRecord) {
    let _ = state.keystore.append_audit_event(event);
}

fn require_ops_access(
    state: &AppState,
    headers: &HeaderMap,
    operation: &str,
    wallet_address: Option<&str>,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let now = epoch_ms().unwrap_or_default();

    let principal = match parse_authbuddy_principal(headers, state) {
        Ok(principal) => principal,
        Err(message) => {
            append_audit_event(
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
            );
            return Err(unauthorized("ops access denied"));
        }
    };

    let has_ops_role = principal.roles.iter().any(|role| role == "ops-admin");
    if !has_ops_role {
        append_audit_event(
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
        );
        return Err(unauthorized("ops access denied"));
    }

    append_audit_event(
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
    );

    Ok(principal.user_id)
}

fn parse_authbuddy_principal(headers: &HeaderMap, state: &AppState) -> Result<AuthPrincipal, String> {
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
        let issuer = token_data
            .iss
            .as_deref()
            .ok_or_else(|| "missing AuthBuddy JWT iss claim".to_owned())?;
        if issuer != expected_issuer.as_ref() {
            return Err("invalid AuthBuddy JWT issuer".to_owned());
        }
    }

    if let Some(expected_audience) = &state.authbuddy_expected_audience {
        let audience = token_data
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

async fn fetch_jwks_from_url(client: &reqwest::Client, url: &str) -> Result<JwkSet, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| "request failed".to_owned())?;

    if !response.status().is_success() {
        return Err(format!("unexpected status {}", response.status()));
    }

    response
        .json::<JwkSet>()
        .await
        .map_err(|_| "invalid JWKS payload".to_owned())
}
