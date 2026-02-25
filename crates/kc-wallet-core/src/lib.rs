use anyhow::{Result, anyhow};
use kc_api_types::SignPurpose;
use kc_chain_client::{ChainRegistry, SubmitTxRequest, SubmitTxResult};
use kc_crypto::Signer;
use kc_storage::Keystore;

pub struct WalletCore<S, K> {
    signer: S,
    keystore: K,
    chain_registry: ChainRegistry,
}

impl<S, K> WalletCore<S, K>
where
    S: Signer,
    K: Keystore,
{
    pub fn new(signer: S, keystore: K, chain_registry: ChainRegistry) -> Self {
        Self {
            signer,
            keystore,
            chain_registry,
        }
    }

    pub async fn sign_payload(&self, payload: &[u8], purpose: SignPurpose) -> Result<Vec<u8>> {
        self.signer.sign(payload, purpose)
    }

    pub async fn submit_transaction(&self, req: SubmitTxRequest) -> Result<SubmitTxResult> {
        let Some(adapter) = self.chain_registry.adapter(&req.chain.0) else {
            return Err(anyhow!("unsupported chain: {}", req.chain.0));
        };

        adapter.submit_transaction(req).await
    }

    pub async fn persist_encrypted_key(&self, wallet_address: &str, encrypted_key: Vec<u8>) -> Result<()> {
        self.keystore
            .save_encrypted_key(wallet_address, encrypted_key)
            .await
    }
}
