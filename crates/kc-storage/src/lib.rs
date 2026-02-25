use anyhow::Result;
use async_trait::async_trait;
use rocksdb::{DB, IteratorMode, Options};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

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

pub struct RocksDbKeystore {
    db: Arc<DB>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBindingRecord {
    pub wallet_address: String,
    pub user_id: String,
    pub chain: String,
    pub last_verified_epoch_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventRecord {
    pub event_id: String,
    pub event_type: String,
    pub wallet_address: Option<String>,
    pub user_id: Option<String>,
    pub chain: Option<String>,
    pub outcome: String,
    pub message: Option<String>,
    pub timestamp_epoch_ms: u128,
}

impl RocksDbKeystore {
    pub fn open_default(path: &str) -> Result<Self> {
        let mut options = Options::default();
        options.create_if_missing(true);
        let db = DB::open(&options, path)?;
        Ok(Self { db: Arc::new(db) })
    }

    fn key_for_wallet(wallet_address: &str) -> String {
        format!("wallet-key:{wallet_address}")
    }

    fn key_for_wallet_binding(wallet_address: &str) -> String {
        format!("wallet-binding:{wallet_address}")
    }

    fn key_for_audit_event(timestamp_epoch_ms: u128, event_id: &str) -> String {
        format!("audit:{timestamp_epoch_ms}:{event_id}")
    }

    pub fn save_wallet_binding(&self, record: &WalletBindingRecord) -> Result<()> {
        let key = Self::key_for_wallet_binding(&record.wallet_address);
        let value = serde_json::to_vec(record)?;
        self.db.put(key.as_bytes(), value)?;
        Ok(())
    }

    pub fn load_wallet_binding(&self, wallet_address: &str) -> Result<Option<WalletBindingRecord>> {
        let key = Self::key_for_wallet_binding(wallet_address);
        let value = self.db.get(key.as_bytes())?;
        match value {
            Some(raw) => Ok(Some(serde_json::from_slice::<WalletBindingRecord>(&raw)?)),
            None => Ok(None),
        }
    }

    pub fn append_audit_event(&self, mut record: AuditEventRecord) -> Result<String> {
        if record.event_id.trim().is_empty() {
            record.event_id = Uuid::new_v4().to_string();
        }
        let key = Self::key_for_audit_event(record.timestamp_epoch_ms, &record.event_id);
        let value = serde_json::to_vec(&record)?;
        self.db.put(key.as_bytes(), value)?;
        Ok(record.event_id)
    }

    pub fn list_audit_events(
        &self,
        limit: usize,
        event_type: Option<&str>,
        wallet_address: Option<&str>,
        outcome: Option<&str>,
    ) -> Result<Vec<AuditEventRecord>> {
        let mut events = Vec::new();

        for (key, value) in self.db.iterator(IteratorMode::Start) {
            if !key.as_ref().starts_with(b"audit:") {
                continue;
            }

            let record = serde_json::from_slice::<AuditEventRecord>(&value)?;

            if let Some(expected) = event_type {
                if record.event_type != expected {
                    continue;
                }
            }

            if let Some(expected) = wallet_address {
                if record.wallet_address.as_deref() != Some(expected) {
                    continue;
                }
            }

            if let Some(expected) = outcome {
                if record.outcome != expected {
                    continue;
                }
            }

            events.push(record);
        }

        events.sort_by(|a, b| b.timestamp_epoch_ms.cmp(&a.timestamp_epoch_ms));
        if events.len() > limit {
            events.truncate(limit);
        }

        Ok(events)
    }
}

#[async_trait]
impl Keystore for RocksDbKeystore {
    async fn save_encrypted_key(&self, wallet_address: &str, encrypted_key: Vec<u8>) -> Result<()> {
        let key = Self::key_for_wallet(wallet_address);
        self.db.put(key.as_bytes(), encrypted_key)?;
        Ok(())
    }

    async fn load_encrypted_key(&self, wallet_address: &str) -> Result<Option<Vec<u8>>> {
        let key = Self::key_for_wallet(wallet_address);
        let value = self.db.get(key.as_bytes())?;
        Ok(value.map(|v| v.to_vec()))
    }
}
