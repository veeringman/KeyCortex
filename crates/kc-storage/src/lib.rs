use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

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

#[derive(Default)]
pub struct InMemoryKeystore {
    keys: RwLock<HashMap<String, Vec<u8>>>,
}

#[async_trait]
impl Keystore for InMemoryKeystore {
    async fn save_encrypted_key(&self, wallet_address: &str, encrypted_key: Vec<u8>) -> Result<()> {
        let mut guard = self.keys.write().await;
        guard.insert(wallet_address.to_owned(), encrypted_key);
        Ok(())
    }

    async fn load_encrypted_key(&self, wallet_address: &str) -> Result<Option<Vec<u8>>> {
        let guard = self.keys.read().await;
        Ok(guard.get(wallet_address).cloned())
    }
}
