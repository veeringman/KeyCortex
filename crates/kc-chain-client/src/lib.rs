use anyhow::Result;
use async_trait::async_trait;
use kc_api_types::{AssetSymbol, ChainId, WalletAddress};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BalanceResult {
    pub wallet_address: WalletAddress,
    pub chain: ChainId,
    pub asset: AssetSymbol,
    pub amount: String,
}

#[derive(Debug, Clone)]
pub struct SubmitTxRequest {
    pub from: WalletAddress,
    pub to: WalletAddress,
    pub amount: String,
    pub asset: AssetSymbol,
    pub chain: ChainId,
    pub signed_payload: String,
}

#[derive(Debug, Clone)]
pub struct SubmitTxResult {
    pub tx_hash: String,
    pub accepted: bool,
}

#[derive(Debug, Clone)]
pub struct TxStatusRequest {
    pub tx_hash: String,
    pub chain: ChainId,
}

#[derive(Debug, Clone)]
pub struct TxStatusResult {
    pub tx_hash: String,
    pub status: String,
    pub accepted: bool,
}

#[async_trait]
pub trait ChainAdapter: Send + Sync {
    fn chain_id(&self) -> &str;
    async fn get_balance(&self, wallet_address: &WalletAddress, asset: &AssetSymbol) -> Result<BalanceResult>;
    async fn submit_transaction(&self, req: SubmitTxRequest) -> Result<SubmitTxResult>;
    async fn get_transaction_status(&self, req: TxStatusRequest) -> Result<TxStatusResult>;
}

#[derive(Default)]
pub struct ChainRegistry {
    adapters: HashMap<String, Arc<dyn ChainAdapter>>,
}

impl ChainRegistry {
    pub fn register(&mut self, adapter: Arc<dyn ChainAdapter>) {
        self.adapters.insert(adapter.chain_id().to_owned(), adapter);
    }

    pub fn adapter(&self, chain_id: &str) -> Option<Arc<dyn ChainAdapter>> {
        self.adapters.get(chain_id).cloned()
    }
}
