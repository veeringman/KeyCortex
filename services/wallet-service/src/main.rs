use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use kc_api_types::{
    AuthBindRequest, AuthBindResponse, AuthChallengeResponse, AuthVerifyRequest, AuthVerifyResponse,
    WalletBalanceResponse, WalletCreateResponse, WalletSignRequest, WalletSignResponse,
};
use kc_auth_adapter::{challenge_response, issue_challenge, verify_signature_placeholder};
use kc_api_types::{AssetSymbol, WalletAddress};
use kc_chain_client::ChainAdapter;
use kc_chain_flowcortex::{FLOWCORTEX_L0, FlowCortexAdapter};
use kc_crypto::{Ed25519Signer, Signer};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
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

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

#[derive(Clone)]
struct AppState {
    signer: Arc<Ed25519Signer>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let state = AppState {
        signer: Arc::new(Ed25519Signer::new_random()),
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

async fn wallet_create(State(state): State<AppState>) -> Json<WalletCreateResponse> {
    let wallet_address = state.signer.wallet_address();
    let public_key = state.signer.public_key_hex();

    Json(WalletCreateResponse {
        wallet_address,
        public_key,
        chain: FLOWCORTEX_L0.to_owned(),
    })
}

async fn wallet_sign(
    State(state): State<AppState>,
    Json(request): Json<WalletSignRequest>,
) -> ApiResult<WalletSignResponse> {
    if request.payload.trim().is_empty() {
        return Err(bad_request("payload cannot be empty"));
    }

    let payload_bytes = STANDARD
        .decode(request.payload.as_bytes())
        .map_err(|_| bad_request("payload must be valid base64"))?;

    let signature_bytes = state
        .signer
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

    let chain = query.chain.unwrap_or_else(|| FLOWCORTEX_L0.to_owned());
    if chain != FLOWCORTEX_L0 {
        return Err(bad_request("unsupported chain for MVP; only flowcortex-l0 is enabled"));
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

async fn auth_challenge() -> Json<AuthChallengeResponse> {
    let challenge = issue_challenge(120);
    Json(challenge_response(&challenge))
}

async fn auth_verify(Json(request): Json<AuthVerifyRequest>) -> ApiResult<AuthVerifyResponse> {
    let response = verify_signature_placeholder(
        &request.wallet_address,
        &request.challenge,
        &request.signature,
    )
    .map_err(internal_error)?;

    Ok(Json(response))
}

async fn auth_bind(headers: HeaderMap, Json(request): Json<AuthBindRequest>) -> ApiResult<AuthBindResponse> {
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| unauthorized("missing Authorization header"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(unauthorized("invalid Authorization format"));
    }

    if request.wallet_address.trim().is_empty() {
        return Err(bad_request("wallet_address is required"));
    }

    if request.chain != FLOWCORTEX_L0 {
        return Err(bad_request("unsupported chain for MVP; only flowcortex-l0 is enabled"));
    }

    let user_id = auth_header.trim_start_matches("Bearer ").trim();
    let user_id = if user_id.is_empty() {
        "authbuddy-user".to_owned()
    } else {
        user_id.to_owned()
    };

    Ok(Json(AuthBindResponse {
        bound: true,
        user_id,
        wallet_address: request.wallet_address,
        chain: request.chain,
        bound_at_epoch_ms: epoch_ms().unwrap_or_default(),
    }))
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
