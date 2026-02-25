use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use kc_api_types::{
    AuthBindRequest, AuthBindResponse, AuthChallengeResponse, AuthVerifyRequest, AuthVerifyResponse,
    SignPurpose, WalletBalanceResponse, WalletCreateResponse, WalletSignRequest, WalletSignResponse,
};
use kc_auth_adapter::{challenge_response, issue_challenge};
use kc_api_types::{AssetSymbol, WalletAddress};
use kc_chain_client::ChainAdapter;
use kc_chain_flowcortex::{FLOWCORTEX_L1, FlowCortexAdapter};
use kc_crypto::{Ed25519Signer, Signer, decrypt_key_material, encrypt_key_material};
use kc_storage::{AuditEventRecord, Keystore, RocksDbKeystore, WalletBindingRecord};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct VersionResponse {
    service: &'static str,
    version: &'static str,
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
    challenge_store: Arc<RwLock<HashMap<String, ChallengeRecord>>>,
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

    let state = AppState {
        keystore: Arc::new(keystore),
        encryption_key: Arc::<str>::from("keycortex-dev-master-key"),
        challenge_store: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/wallet/create", post(wallet_create))
        .route("/wallet/sign", post(wallet_sign))
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

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "wallet-service",
        status: "ok",
    })
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        service: "wallet-service",
        version: env!("CARGO_PKG_VERSION"),
    })
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

    let auth_header = match headers.get("authorization").and_then(|v| v.to_str().ok()) {
        Some(value) => value,
        None => {
            append_audit_event(
                &state,
                AuditEventRecord {
                    event_id: String::new(),
                    event_type: "auth_bind".to_owned(),
                    wallet_address: Some(request.wallet_address.clone()),
                    user_id: None,
                    chain: Some(request.chain.clone()),
                    outcome: "denied".to_owned(),
                    message: Some("missing Authorization header".to_owned()),
                    timestamp_epoch_ms: now,
                },
            );
            return Err(unauthorized("missing Authorization header"));
        }
    };

    if !auth_header.starts_with("Bearer ") {
        append_audit_event(
            &state,
            AuditEventRecord {
                event_id: String::new(),
                event_type: "auth_bind".to_owned(),
                wallet_address: Some(request.wallet_address.clone()),
                user_id: None,
                chain: Some(request.chain.clone()),
                outcome: "denied".to_owned(),
                message: Some("invalid Authorization format".to_owned()),
                timestamp_epoch_ms: now,
            },
        );
        return Err(unauthorized("invalid Authorization format"));
    }

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

    let user_id = auth_header.trim_start_matches("Bearer ").trim();
    let user_id = if user_id.is_empty() {
        "authbuddy-user".to_owned()
    } else {
        user_id.to_owned()
    };

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
    Path(wallet_address): Path<String>,
) -> ApiResult<WalletBindingRecord> {
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
    Query(query): Query<OpsAuditQuery>,
) -> ApiResult<OpsAuditResponse> {
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
