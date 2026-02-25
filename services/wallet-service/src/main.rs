use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use jsonwebtoken::jwk::JwkSet;
use kc_api_types::{
    AssetSymbol, WalletBalanceResponse, WalletCreateResponse, WalletSignRequest,
    WalletSignResponse, WalletSubmitResponse, WalletAddress,
};
use kc_chain_client::ChainAdapter;
use kc_chain_flowcortex::{FLOWCORTEX_L1, FlowCortexAdapter};
use kc_crypto::{Ed25519Signer, Signer, decrypt_key_material, encrypt_key_material};
use kc_storage::{Keystore, RocksDbKeystore};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::sync::{
    Arc, RwLock as StdRwLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock as TokioRwLock;
use tracing::{info, warn};

mod submit;
mod auth;
mod ops;
mod db;

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    status: &'static str,
    storage_mode: String,
    postgres_enabled: bool,
    db_fallback_counters: DbFallbackCountersSnapshot,
    postgres_startup: PostgresStartupReport,
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
struct StartupDiagnosticsResponse {
    service: &'static str,
    storage_mode: String,
    postgres_enabled: bool,
    postgres_startup: PostgresStartupReport,
    db_fallback_counters: DbFallbackCountersSnapshot,
    auth_mode: String,
    jwks_source: Option<String>,
    jwks_loaded: bool,
    last_jwks_refresh_epoch_ms: Option<u128>,
    last_jwks_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReadinessResponse {
    service: &'static str,
    ready: bool,
    keystore_ready: bool,
    storage_mode: String,
    postgres_enabled: bool,
    db_fallback_counters: DbFallbackCountersSnapshot,
    postgres_startup: PostgresStartupReport,
    auth_ready: bool,
    auth_mode: String,
    jwks_reachable: Option<bool>,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ErrorResponse {
    error: String,
}

#[derive(Debug, Deserialize)]
struct WalletBalanceQuery {
    wallet_address: String,
    asset: Option<String>,
    chain: Option<String>,
}

#[derive(Debug, Clone)]
struct JwksRuntimeStatus {
    source: Option<String>,
    loaded: bool,
    last_refresh_epoch_ms: Option<u128>,
    last_error: Option<String>,
}

pub(crate) type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

#[derive(Debug, Clone, Serialize)]
struct DbFallbackCountersSnapshot {
    postgres_unavailable: u64,
    challenge_persist_failures: u64,
    challenge_mark_used_failures: u64,
    binding_write_failures: u64,
    binding_read_failures: u64,
    audit_write_failures: u64,
    audit_read_failures: u64,
    total: u64,
}

#[derive(Debug, Default)]
pub(crate) struct DbFallbackCounters {
    postgres_unavailable: AtomicU64,
    challenge_persist_failures: AtomicU64,
    challenge_mark_used_failures: AtomicU64,
    binding_write_failures: AtomicU64,
    binding_read_failures: AtomicU64,
    audit_write_failures: AtomicU64,
    audit_read_failures: AtomicU64,
}

impl DbFallbackCounters {
    pub(crate) fn inc_postgres_unavailable(&self) {
        self.postgres_unavailable.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_challenge_persist_failures(&self) {
        self.challenge_persist_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_challenge_mark_used_failures(&self) {
        self.challenge_mark_used_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_binding_write_failures(&self) {
        self.binding_write_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_binding_read_failures(&self) {
        self.binding_read_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_audit_write_failures(&self) {
        self.audit_write_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_audit_read_failures(&self) {
        self.audit_read_failures.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> DbFallbackCountersSnapshot {
        let postgres_unavailable = self.postgres_unavailable.load(Ordering::Relaxed);
        let challenge_persist_failures = self.challenge_persist_failures.load(Ordering::Relaxed);
        let challenge_mark_used_failures = self.challenge_mark_used_failures.load(Ordering::Relaxed);
        let binding_write_failures = self.binding_write_failures.load(Ordering::Relaxed);
        let binding_read_failures = self.binding_read_failures.load(Ordering::Relaxed);
        let audit_write_failures = self.audit_write_failures.load(Ordering::Relaxed);
        let audit_read_failures = self.audit_read_failures.load(Ordering::Relaxed);
        let total = postgres_unavailable
            + challenge_persist_failures
            + challenge_mark_used_failures
            + binding_write_failures
            + binding_read_failures
            + audit_write_failures
            + audit_read_failures;

        DbFallbackCountersSnapshot {
            postgres_unavailable,
            challenge_persist_failures,
            challenge_mark_used_failures,
            binding_write_failures,
            binding_read_failures,
            audit_write_failures,
            audit_read_failures,
            total,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct PostgresStartupReport {
    configured: bool,
    enabled: bool,
    migrations_dir: Option<String>,
    migration_files_applied: usize,
    last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ChallengeRecord {
    pub(crate) issued_at_epoch_ms: u128,
    pub(crate) expires_at_epoch_ms: u128,
    pub(crate) used: bool,
    pub(crate) used_at_epoch_ms: Option<u128>,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) keystore: Arc<RocksDbKeystore>,
    pub(crate) postgres_repo: Option<Arc<db::PostgresRepository>>,
    pub(crate) db_fallback_counters: Arc<DbFallbackCounters>,
    postgres_startup: Arc<StdRwLock<PostgresStartupReport>>,
    pub(crate) encryption_key: Arc<str>,
    pub(crate) authbuddy_jwt_secret: Arc<str>,
    pub(crate) authbuddy_jwks: Arc<StdRwLock<Option<JwkSet>>>,
    jwks_status: Arc<StdRwLock<JwksRuntimeStatus>>,
    pub(crate) authbuddy_expected_issuer: Option<Arc<str>>,
    pub(crate) authbuddy_expected_audience: Option<Arc<str>>,
    pub(crate) challenge_store: Arc<TokioRwLock<HashMap<String, ChallengeRecord>>>,
    pub(crate) submit_idempotency_cache: Arc<TokioRwLock<HashMap<String, WalletSubmitResponse>>>,
    pub(crate) submit_nonce_state: Arc<TokioRwLock<HashMap<String, u64>>>,
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

    let db_fallback_counters = Arc::new(DbFallbackCounters::default());
    let mut postgres_startup = PostgresStartupReport {
        configured: false,
        enabled: false,
        migrations_dir: None,
        migration_files_applied: 0,
        last_error: None,
    };

    let postgres_repo = if let Ok(database_url) = env::var("DATABASE_URL") {
        if database_url.trim().is_empty() {
            None
        } else {
            postgres_startup.configured = true;
            match db::PostgresRepository::connect(&database_url).await {
                Ok(repo) => {
                    let migrations_dir = env::var("KEYCORTEX_POSTGRES_MIGRATIONS_DIR")
                        .unwrap_or_else(|_| "./migrations/postgres".to_owned());
                    postgres_startup.migrations_dir = Some(migrations_dir.clone());
                    match repo.run_migrations_from_dir(&migrations_dir).await {
                        Ok(applied) => {
                            postgres_startup.migration_files_applied = applied;
                            info!(
                                "applied {} postgres migration file(s) from {}",
                                applied, migrations_dir
                            );
                        }
                        Err(err) => {
                            postgres_startup.last_error = Some(format!(
                                "migration failure from {}: {}",
                                migrations_dir, err
                            ));
                            warn!(
                                "failed to run postgres migrations from {}: {}",
                                migrations_dir, err
                            );
                        }
                    }
                    postgres_startup.enabled = true;
                    info!("connected Postgres repository");
                    Some(Arc::new(repo))
                }
                Err(err) => {
                    postgres_startup.last_error = Some(format!("connect failure: {}", err));
                    db_fallback_counters.inc_postgres_unavailable();
                    warn!("failed to initialize Postgres repository: {}", err);
                    None
                }
            }
        }
    } else {
        None
    };

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
        postgres_repo,
        db_fallback_counters,
        postgres_startup: Arc::new(StdRwLock::new(postgres_startup)),
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
        submit_idempotency_cache: Arc::new(TokioRwLock::new(HashMap::new())),
        submit_nonce_state: Arc::new(TokioRwLock::new(HashMap::new())),
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

    let app = build_app(state);

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

    let postgres_enabled = state.postgres_repo.is_some();
    let storage_mode = if postgres_enabled {
        "postgres+rocksdb"
    } else {
        "rocksdb-only"
    }
    .to_owned();

    let db_fallback_counters = state.db_fallback_counters.snapshot();
    let postgres_startup = state
        .postgres_startup
        .read()
        .ok()
        .map(|entry| entry.clone())
        .unwrap_or(PostgresStartupReport {
            configured: false,
            enabled: false,
            migrations_dir: None,
            migration_files_applied: 0,
            last_error: Some("postgres startup report unavailable".to_owned()),
        });

    Json(HealthResponse {
        service: "wallet-service",
        status: "ok",
        storage_mode,
        postgres_enabled,
        db_fallback_counters,
        postgres_startup,
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

async fn startupz(State(state): State<AppState>) -> Json<StartupDiagnosticsResponse> {
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

    let postgres_enabled = state.postgres_repo.is_some();
    let storage_mode = if postgres_enabled {
        "postgres+rocksdb"
    } else {
        "rocksdb-only"
    }
    .to_owned();

    let auth_mode = if status_snapshot.loaded {
        "rs256-jwks"
    } else {
        "hs256-fallback"
    }
    .to_owned();

    let postgres_startup = state
        .postgres_startup
        .read()
        .ok()
        .map(|entry| entry.clone())
        .unwrap_or(PostgresStartupReport {
            configured: false,
            enabled: false,
            migrations_dir: None,
            migration_files_applied: 0,
            last_error: Some("postgres startup report unavailable".to_owned()),
        });

    let db_fallback_counters = state.db_fallback_counters.snapshot();

    Json(StartupDiagnosticsResponse {
        service: "wallet-service",
        storage_mode,
        postgres_enabled,
        postgres_startup,
        db_fallback_counters,
        auth_mode,
        jwks_source: status_snapshot.source,
        jwks_loaded: status_snapshot.loaded,
        last_jwks_refresh_epoch_ms: status_snapshot.last_refresh_epoch_ms,
        last_jwks_error: status_snapshot.last_error,
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
    let postgres_enabled = state.postgres_repo.is_some();
    let storage_mode = if postgres_enabled {
        "postgres+rocksdb"
    } else {
        "rocksdb-only"
    }
    .to_owned();
    let db_fallback_counters = state.db_fallback_counters.snapshot();
    let postgres_startup = state
        .postgres_startup
        .read()
        .ok()
        .map(|entry| entry.clone())
        .unwrap_or(PostgresStartupReport {
            configured: false,
            enabled: false,
            migrations_dir: None,
            migration_files_applied: 0,
            last_error: Some("postgres startup report unavailable".to_owned()),
        });
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
            storage_mode,
            postgres_enabled,
            db_fallback_counters,
            postgres_startup,
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

    let mut secret_key = decrypt_key_material(&encrypted_key, state.encryption_key.as_ref())
        .map_err(internal_error)?;

    let signer = Ed25519Signer::from_secret_key_bytes(secret_key);
    secret_key.fill(0);
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

pub(crate) fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

pub(crate) fn unauthorized(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

pub(crate) fn not_found(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

pub(crate) fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: err.to_string(),
        }),
    )
}

pub(crate) fn epoch_ms() -> anyhow::Result<u128> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

pub(crate) fn to_hex(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    for byte in input {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

pub(crate) fn from_hex(input: &str) -> anyhow::Result<Vec<u8>> {
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

fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/readyz", get(readyz))
        .route("/startupz", get(startupz))
        .route("/version", get(version))
        .route("/wallet/create", post(wallet_create))
        .route("/wallet/sign", post(wallet_sign))
        .route("/wallet/submit", post(submit::wallet_submit))
        .route("/wallet/nonce", get(submit::wallet_nonce))
        .route("/wallet/tx/{tx_hash}", get(submit::wallet_tx_status))
        .route("/wallet/balance", get(wallet_balance))
        .route("/auth/challenge", post(auth::auth_challenge))
        .route("/auth/verify", post(auth::auth_verify))
        .route("/auth/bind", post(auth::auth_bind))
        .route("/ops/bindings/{wallet_address}", get(ops::ops_get_binding))
        .route("/ops/audit", get(ops::ops_list_audit))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::{HeaderValue, Method, Request};
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde_json::{Value, json};
    use tempfile::TempDir;
    use tower::util::ServiceExt;

    fn test_state(temp_dir: &TempDir) -> AppState {
        let keystore = RocksDbKeystore::open_default(
            temp_dir
                .path()
                .join("keystore.rocksdb")
                .to_string_lossy()
                .as_ref(),
        )
        .expect("rocksdb should initialize");

        AppState {
            keystore: Arc::new(keystore),
            postgres_repo: None,
            db_fallback_counters: Arc::new(DbFallbackCounters::default()),
            postgres_startup: Arc::new(StdRwLock::new(PostgresStartupReport {
                configured: false,
                enabled: false,
                migrations_dir: None,
                migration_files_applied: 0,
                last_error: None,
            })),
            encryption_key: Arc::<str>::from("test-master-key"),
            authbuddy_jwt_secret: Arc::<str>::from("test-auth-secret"),
            authbuddy_jwks: Arc::new(StdRwLock::new(None)),
            jwks_status: Arc::new(StdRwLock::new(JwksRuntimeStatus {
                source: None,
                loaded: false,
                last_refresh_epoch_ms: None,
                last_error: None,
            })),
            authbuddy_expected_issuer: None,
            authbuddy_expected_audience: None,
            challenge_store: Arc::new(TokioRwLock::new(HashMap::new())),
            submit_idempotency_cache: Arc::new(TokioRwLock::new(HashMap::new())),
            submit_nonce_state: Arc::new(TokioRwLock::new(HashMap::new())),
        }
    }

    async fn send_json(
        app: &Router,
        method: Method,
        uri: &str,
        body: Value,
        headers: Vec<(&str, HeaderValue)>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        for (key, value) in headers {
            request = request.header(key, value);
        }

        let request = request
            .body(Body::from(body.to_string()))
            .expect("request should build");
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("request should be handled");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let parsed = serde_json::from_slice::<Value>(&bytes).expect("response should be json");
        (status, parsed)
    }

    async fn send_empty(app: &Router, method: Method, uri: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .expect("request should build");
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("request should be handled");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let parsed = serde_json::from_slice::<Value>(&bytes).expect("response should be json");
        (status, parsed)
    }

    fn build_hs256_token(secret: &str, sub: &str) -> String {
        #[derive(serde::Serialize)]
        struct Claims<'a> {
            sub: &'a str,
            exp: u64,
            role: &'a str,
        }

        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_secs()
            + 3600;

        encode(
            &Header::default(),
            &Claims {
                sub,
                exp,
                role: "ops-admin",
            },
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("token should encode")
    }

    #[tokio::test]
    async fn wallet_create_and_sign_contract_fields_are_present() {
        let temp_dir = TempDir::new().expect("temp dir should create");
        let app = build_app(test_state(&temp_dir));

        let (create_status, create_body) = send_empty(&app, Method::POST, "/wallet/create").await;
        assert_eq!(create_status, StatusCode::OK);
        assert_eq!(create_body["chain"], "flowcortex-l1");
        let wallet_address = create_body["wallet_address"]
            .as_str()
            .expect("wallet_address should be string");

        let payload_b64 = base64::engine::general_purpose::STANDARD.encode("hello-sign");
        let (sign_status, sign_body) = send_json(
            &app,
            Method::POST,
            "/wallet/sign",
            json!({
                "wallet_address": wallet_address,
                "payload": payload_b64,
                "purpose": "proof"
            }),
            vec![],
        )
        .await;

        assert_eq!(sign_status, StatusCode::OK);
        let signature = sign_body["signature"]
            .as_str()
            .expect("signature should be string");
        assert!(!signature.is_empty());
    }

    #[tokio::test]
    async fn auth_challenge_verify_marks_challenge_used() {
        let temp_dir = TempDir::new().expect("temp dir should create");
        let app = build_app(test_state(&temp_dir));

        let (create_status, create_body) = send_empty(&app, Method::POST, "/wallet/create").await;
        assert_eq!(create_status, StatusCode::OK);
        let wallet_address = create_body["wallet_address"]
            .as_str()
            .expect("wallet_address should be string")
            .to_owned();

        let (challenge_status, challenge_body) =
            send_empty(&app, Method::POST, "/auth/challenge").await;
        assert_eq!(challenge_status, StatusCode::OK);
        let challenge = challenge_body["challenge"]
            .as_str()
            .expect("challenge should be string")
            .to_owned();

        let challenge_b64 = base64::engine::general_purpose::STANDARD.encode(challenge.as_bytes());
        let (sign_status, sign_body) = send_json(
            &app,
            Method::POST,
            "/wallet/sign",
            json!({
                "wallet_address": wallet_address,
                "payload": challenge_b64,
                "purpose": "auth"
            }),
            vec![],
        )
        .await;
        assert_eq!(sign_status, StatusCode::OK);
        let signature = sign_body["signature"]
            .as_str()
            .expect("signature should be string")
            .to_owned();

        let (verify_status, verify_body) = send_json(
            &app,
            Method::POST,
            "/auth/verify",
            json!({
                "wallet_address": wallet_address,
                "signature": signature,
                "challenge": challenge
            }),
            vec![],
        )
        .await;
        assert_eq!(verify_status, StatusCode::OK);
        assert_eq!(verify_body["valid"], true);
        assert!(verify_body.get("verified_at_epoch_ms").is_some());

        let (reverify_status, reverify_body) = send_json(
            &app,
            Method::POST,
            "/auth/verify",
            json!({
                "wallet_address": wallet_address,
                "signature": signature,
                "challenge": challenge
            }),
            vec![],
        )
        .await;
        assert_eq!(reverify_status, StatusCode::BAD_REQUEST);
        assert_eq!(reverify_body["error"], "challenge already used");
    }

    #[tokio::test]
    async fn wallet_submit_nonce_and_idempotency_contract() {
        let temp_dir = TempDir::new().expect("temp dir should create");
        let app = build_app(test_state(&temp_dir));

        let (create_status, create_body) = send_empty(&app, Method::POST, "/wallet/create").await;
        assert_eq!(create_status, StatusCode::OK);
        let wallet_address = create_body["wallet_address"]
            .as_str()
            .expect("wallet_address should be string")
            .to_owned();

        let submit_body = json!({
            "from": wallet_address,
            "to": "0xdeadbeef",
            "amount": "1000",
            "asset": "FloweR",
            "chain": "flowcortex-l1",
            "nonce": 1
        });

        let (submit_status, submit_response) = send_json(
            &app,
            Method::POST,
            "/wallet/submit",
            submit_body.clone(),
            vec![("idempotency-key", HeaderValue::from_static("idem-1"))],
        )
        .await;
        assert_eq!(submit_status, StatusCode::OK);
        assert_eq!(submit_response["accepted"], true);

        let (submit_replay_status, submit_replay_response) = send_json(
            &app,
            Method::POST,
            "/wallet/submit",
            submit_body,
            vec![("idempotency-key", HeaderValue::from_static("idem-1"))],
        )
        .await;
        assert_eq!(submit_replay_status, StatusCode::OK);
        assert_eq!(submit_replay_response, submit_response);

        let tx_hash = submit_response["tx_hash"]
            .as_str()
            .expect("tx_hash should be string")
            .to_owned();
        let (nonce_status, nonce_body) = send_empty(
            &app,
            Method::GET,
            &format!("/wallet/nonce?wallet_address={}", create_body["wallet_address"].as_str().unwrap()),
        )
        .await;
        assert_eq!(nonce_status, StatusCode::OK);
        assert_eq!(nonce_body["last_nonce"], 1);
        assert_eq!(nonce_body["next_nonce"], 2);

        let (tx_status, tx_body) = send_empty(&app, Method::GET, &format!("/wallet/tx/{tx_hash}")).await;
        assert_eq!(tx_status, StatusCode::OK);
        assert_eq!(tx_body["tx_hash"], tx_hash);
        assert_eq!(tx_body["chain"], "flowcortex-l1");
    }

    #[tokio::test]
    async fn auth_bind_requires_token_and_succeeds_with_hs256() {
        let temp_dir = TempDir::new().expect("temp dir should create");
        let app = build_app(test_state(&temp_dir));

        let (create_status, create_body) = send_empty(&app, Method::POST, "/wallet/create").await;
        assert_eq!(create_status, StatusCode::OK);
        let wallet_address = create_body["wallet_address"]
            .as_str()
            .expect("wallet_address should be string")
            .to_owned();

        let (unauth_status, unauth_body) = send_json(
            &app,
            Method::POST,
            "/auth/bind",
            json!({
                "wallet_address": wallet_address,
                "chain": "flowcortex-l1"
            }),
            vec![],
        )
        .await;
        assert_eq!(unauth_status, StatusCode::UNAUTHORIZED);
        assert!(unauth_body.get("error").is_some());

        let token = build_hs256_token("test-auth-secret", "user-123");
        let auth_value = HeaderValue::from_str(&format!("Bearer {token}"))
            .expect("authorization header should build");

        let (bind_status, bind_body) = send_json(
            &app,
            Method::POST,
            "/auth/bind",
            json!({
                "wallet_address": create_body["wallet_address"].as_str().unwrap(),
                "chain": "flowcortex-l1"
            }),
            vec![("authorization", auth_value)],
        )
        .await;

        assert_eq!(bind_status, StatusCode::OK);
        assert_eq!(bind_body["bound"], true);
        assert_eq!(bind_body["user_id"], "user-123");
        assert_eq!(bind_body["chain"], "flowcortex-l1");
        assert!(bind_body.get("bound_at_epoch_ms").is_some());
    }
}
