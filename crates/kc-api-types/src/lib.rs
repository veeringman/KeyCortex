use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignPurpose {
    Transaction,
    Auth,
    Proof,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WalletAddress(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetSymbol(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletCreateRequest {
    pub label: Option<String>,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletCreateResponse {
    pub wallet_address: String,
    pub public_key: String,
    pub chain: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSummary {
    pub wallet_address: String,
    pub chain: String,
    pub bound_user_id: Option<String>,
    pub public_key: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletListResponse {
    pub wallets: Vec<WalletSummary>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSignRequest {
    pub wallet_address: String,
    pub payload: String,
    pub purpose: SignPurpose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSignResponse {
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalanceResponse {
    pub wallet_address: String,
    pub chain: String,
    pub asset: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSubmitRequest {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub asset: String,
    pub chain: String,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSubmitResponse {
    pub accepted: bool,
    pub tx_hash: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletNonceResponse {
    pub wallet_address: String,
    pub last_nonce: u64,
    pub next_nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTxStatusResponse {
    pub tx_hash: String,
    pub status: String,
    pub accepted: bool,
    pub chain: String,
    pub from: String,
    pub to: String,
    pub asset: String,
    pub amount: String,
    pub submitted_at_epoch_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallengeResponse {
    pub challenge: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthVerifyRequest {
    pub wallet_address: String,
    pub signature: String,
    pub challenge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthVerifyResponse {
    pub valid: bool,
    pub wallet_address: String,
    pub verified_at_epoch_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthBindRequest {
    pub wallet_address: String,
    pub chain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthBindResponse {
    pub bound: bool,
    pub user_id: String,
    pub wallet_address: String,
    pub chain: String,
    pub bound_at_epoch_ms: u128,
}

// --- ProofCortex types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCortexCommitmentRequest {
    pub wallet_address: String,
    pub challenge: String,
    pub verification_result: bool,
    pub chain: String,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCortexCommitmentResponse {
    pub commitment: String,
    pub wallet_address: String,
    pub chain: String,
    pub verification_result: bool,
    pub domain_separator: String,
    pub proof_input_schema_version: String,
    pub generated_at_epoch_ms: u128,
}

// --- FortressDigital enhanced types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FortressDigitalWalletStatusRequest {
    pub wallet_address: String,
    pub chain: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FortressDigitalWalletStatusResponse {
    pub wallet_address: String,
    pub chain: String,
    pub wallet_exists: bool,
    pub binding_status: WalletBindingStatus,
    pub key_type: String,
    pub last_verification_epoch_ms: Option<u128>,
    pub signature_frequency_hint: String,
    pub risk_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBindingStatus {
    pub bound: bool,
    pub user_id: Option<String>,
    pub last_verified_epoch_ms: Option<u128>,
}

// --- Chain config types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfigResponse {
    pub chain_slug: String,
    pub chain_id_numeric: Option<u64>,
    pub signature_scheme: String,
    pub address_scheme: String,
    pub domains: ChainDomainTags,
    pub assets: Vec<ChainAssetInfo>,
    pub finality_rule: String,
    pub environment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainDomainTags {
    pub tx_domain_tag: String,
    pub auth_domain_tag: String,
    pub proof_domain_tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainAssetInfo {
    pub symbol: String,
    pub asset_type: String,
    pub decimals: u8,
    pub fee_payment_support: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletRestoreRequest {
    pub passphrase: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletRestoreResponse {
    pub wallet_address: String,
    pub public_key: String,
    pub chain: String,
    pub label: Option<String>,
    pub already_existed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletRenameRequest {
    pub wallet_address: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletRenameResponse {
    pub wallet_address: String,
    pub label: String,
}
