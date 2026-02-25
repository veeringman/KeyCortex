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
