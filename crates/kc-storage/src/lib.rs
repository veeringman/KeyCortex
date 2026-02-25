use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Keystore: Send + Sync {
    async fn save_encrypted_key(&self, wallet_address: &str, encrypted_key: Vec<u8>) -> Result<()>;
    async fn load_encrypted_key(&self, wallet_address: &str) -> Result<Option<Vec<u8>>>;
}

#[derive(Default)]
pub struct NoopKeystore;

#[async_trait]
impl Keystore for NoopKeystore {
    async fn save_encrypted_key(&self, _wallet_address: &str, _encrypted_key: Vec<u8>) -> Result<()> {
        Ok(())
    }

    async fn load_encrypted_key(&self, _wallet_address: &str) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
}
