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
pub struct WalletCreateResponse {
    pub wallet_address: String,
    pub public_key: String,
    pub chain: String,
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
