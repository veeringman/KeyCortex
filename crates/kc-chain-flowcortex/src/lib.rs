use anyhow::Result;
use async_trait::async_trait;
use kc_api_types::{AssetSymbol, ChainId, WalletAddress};
use kc_chain_client::{BalanceResult, ChainAdapter, SubmitTxRequest, SubmitTxResult};

pub const FLOWCORTEX_L1: &str = "flowcortex-l1";

#[derive(Default)]
pub struct FlowCortexAdapter;

#[async_trait]
impl ChainAdapter for FlowCortexAdapter {
    fn chain_id(&self) -> &str {
        FLOWCORTEX_L1
    }

    async fn get_balance(&self, wallet_address: &WalletAddress, asset: &AssetSymbol) -> Result<BalanceResult> {
        Ok(BalanceResult {
            wallet_address: wallet_address.clone(),
            chain: ChainId(FLOWCORTEX_L1.to_owned()),
            asset: asset.clone(),
            amount: "0".to_owned(),
        })
    }

    async fn submit_transaction(&self, _req: SubmitTxRequest) -> Result<SubmitTxResult> {
        Ok(SubmitTxResult {
            tx_hash: "pending-integration".to_owned(),
            accepted: true,
        })
    }
}
