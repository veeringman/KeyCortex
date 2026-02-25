use anyhow::Context;
use kc_storage::{AuditEventRecord, WalletBindingRecord};
use std::fs;
use std::path::PathBuf;
use tokio_postgres::{Client, NoTls};
use tracing::warn;
use uuid::Uuid;

pub(crate) struct PostgresRepository {
    client: Client,
}

impl PostgresRepository {
    pub(crate) async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls)
            .await
            .context("failed to connect to Postgres")?;

        tokio::spawn(async move {
            if let Err(err) = connection.await {
                warn!("postgres connection error: {}", err);
            }
        });

        Ok(Self { client })
    }

    pub(crate) async fn save_wallet_binding(&self, record: &WalletBindingRecord) -> anyhow::Result<()> {
        self.client
            .execute(
                "INSERT INTO wallet_bindings (wallet_address, user_id, chain, last_verified_epoch_ms, updated_at)
                 VALUES ($1, $2, $3, $4, NOW())
                 ON CONFLICT (wallet_address)
                 DO UPDATE SET
                   user_id = EXCLUDED.user_id,
                   chain = EXCLUDED.chain,
                   last_verified_epoch_ms = EXCLUDED.last_verified_epoch_ms,
                   updated_at = NOW()",
                &[
                    &record.wallet_address,
                    &record.user_id,
                    &record.chain,
                    &to_i64(record.last_verified_epoch_ms),
                ],
            )
            .await
            .context("failed to save wallet binding to Postgres")?;

        Ok(())
    }

    pub(crate) async fn run_migrations_from_dir(&self, migrations_dir: &str) -> anyhow::Result<usize> {
        let mut files: Vec<PathBuf> = fs::read_dir(migrations_dir)
            .with_context(|| format!("failed to read migrations directory: {migrations_dir}"))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("sql"))
            .collect();

        files.sort();

        for file_path in &files {
            let sql = fs::read_to_string(file_path)
                .with_context(|| format!("failed to read migration file: {}", file_path.display()))?;
            self.client
                .batch_execute(&sql)
                .await
                .with_context(|| format!("failed to execute migration file: {}", file_path.display()))?;
        }

        Ok(files.len())
    }

    pub(crate) async fn load_wallet_binding(
        &self,
        wallet_address: &str,
    ) -> anyhow::Result<Option<WalletBindingRecord>> {
        let row = self
            .client
            .query_opt(
                "SELECT wallet_address, user_id, chain, last_verified_epoch_ms
                 FROM wallet_bindings
                 WHERE wallet_address = $1",
                &[&wallet_address],
            )
            .await
            .context("failed to load wallet binding from Postgres")?;

        Ok(row.map(|entry| WalletBindingRecord {
            wallet_address: entry.get::<_, String>(0),
            user_id: entry.get::<_, String>(1),
            chain: entry.get::<_, String>(2),
            last_verified_epoch_ms: from_i64(entry.get::<_, i64>(3)),
        }))
    }

    pub(crate) async fn append_audit_event(&self, record: &AuditEventRecord) -> anyhow::Result<String> {
        let event_id = if record.event_id.trim().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            record.event_id.clone()
        };

        self.client
            .execute(
                "INSERT INTO verification_logs
                 (log_id, event_type, wallet_address, user_id, chain, outcome, message, timestamp_epoch_ms)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                &[
                    &event_id,
                    &record.event_type,
                    &record.wallet_address,
                    &record.user_id,
                    &record.chain,
                    &record.outcome,
                    &record.message,
                    &to_i64(record.timestamp_epoch_ms),
                ],
            )
            .await
            .context("failed to append audit event to Postgres")?;

        Ok(event_id)
    }

    pub(crate) async fn list_audit_events(
        &self,
        limit: usize,
        event_type: Option<&str>,
        wallet_address: Option<&str>,
        outcome: Option<&str>,
    ) -> anyhow::Result<Vec<AuditEventRecord>> {
        let rows = self
            .client
            .query(
                "SELECT log_id, event_type, wallet_address, user_id, chain, outcome, message, timestamp_epoch_ms
                 FROM verification_logs
                 WHERE ($1::TEXT IS NULL OR event_type = $1)
                   AND ($2::TEXT IS NULL OR wallet_address = $2)
                   AND ($3::TEXT IS NULL OR outcome = $3)
                 ORDER BY timestamp_epoch_ms DESC
                 LIMIT $4",
                &[
                    &event_type,
                    &wallet_address,
                    &outcome,
                    &(limit as i64),
                ],
            )
            .await
            .context("failed to list audit events from Postgres")?;

        let events = rows
            .into_iter()
            .map(|row| AuditEventRecord {
                event_id: row.get::<_, String>(0),
                event_type: row.get::<_, String>(1),
                wallet_address: row.get::<_, Option<String>>(2),
                user_id: row.get::<_, Option<String>>(3),
                chain: row.get::<_, Option<String>>(4),
                outcome: row.get::<_, String>(5),
                message: row.get::<_, Option<String>>(6),
                timestamp_epoch_ms: from_i64(row.get::<_, i64>(7)),
            })
            .collect();

        Ok(events)
    }

    pub(crate) async fn upsert_challenge(
        &self,
        challenge: &str,
        issued_at_epoch_ms: u128,
        expires_at_epoch_ms: u128,
    ) -> anyhow::Result<()> {
        self.client
            .execute(
                "INSERT INTO challenge_store (challenge, issued_at_epoch_ms, expires_at_epoch_ms, used, used_at_epoch_ms, updated_at)
                 VALUES ($1, $2, $3, FALSE, NULL, NOW())
                 ON CONFLICT (challenge)
                 DO UPDATE SET
                   issued_at_epoch_ms = EXCLUDED.issued_at_epoch_ms,
                   expires_at_epoch_ms = EXCLUDED.expires_at_epoch_ms,
                   used = FALSE,
                   used_at_epoch_ms = NULL,
                   updated_at = NOW()",
                &[
                    &challenge,
                    &to_i64(issued_at_epoch_ms),
                    &to_i64(expires_at_epoch_ms),
                ],
            )
            .await
            .context("failed to upsert challenge in Postgres")?;

        Ok(())
    }

    pub(crate) async fn mark_challenge_used(
        &self,
        challenge: &str,
        used_at_epoch_ms: u128,
    ) -> anyhow::Result<()> {
        self.client
            .execute(
                "UPDATE challenge_store
                 SET used = TRUE, used_at_epoch_ms = $2, updated_at = NOW()
                 WHERE challenge = $1",
                &[&challenge, &to_i64(used_at_epoch_ms)],
            )
            .await
            .context("failed to mark challenge used in Postgres")?;

        Ok(())
    }
}

fn to_i64(value: u128) -> i64 {
    value.min(i64::MAX as u128) as i64
}

fn from_i64(value: i64) -> u128 {
    if value < 0 {
        0
    } else {
        value as u128
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn migrations_dir() -> String {
        if let Ok(path) = env::var("TEST_MIGRATIONS_DIR") {
            return path;
        }

        let candidates = [
            "./migrations/postgres",
            "../../migrations/postgres",
            "../../../migrations/postgres",
        ];

        for path in candidates {
            if std::path::Path::new(path).exists() {
                return path.to_owned();
            }
        }

        "./migrations/postgres".to_owned()
    }

    async fn setup_repo() -> anyhow::Result<Option<PostgresRepository>> {
        let database_url = match env::var("TEST_DATABASE_URL") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(None),
        };

        let repo = PostgresRepository::connect(&database_url).await?;
        repo.run_migrations_from_dir(&migrations_dir()).await?;
        Ok(Some(repo))
    }

    #[tokio::test]
    async fn postgres_binding_roundtrip() -> anyhow::Result<()> {
        let Some(repo) = setup_repo().await? else {
            return Ok(());
        };

        let wallet_address = format!("test-wallet-{}", Uuid::new_v4());
        let record = WalletBindingRecord {
            wallet_address: wallet_address.clone(),
            user_id: "test-user-1".to_owned(),
            chain: "flowcortex-l1".to_owned(),
            last_verified_epoch_ms: 1_700_000_000_000,
        };

        repo.save_wallet_binding(&record).await?;
        let loaded = repo
            .load_wallet_binding(&wallet_address)
            .await?
            .expect("wallet binding should exist");

        assert_eq!(loaded.wallet_address, record.wallet_address);
        assert_eq!(loaded.user_id, record.user_id);
        assert_eq!(loaded.chain, record.chain);
        assert_eq!(loaded.last_verified_epoch_ms, record.last_verified_epoch_ms);

        Ok(())
    }

    #[tokio::test]
    async fn postgres_audit_append_and_filter() -> anyhow::Result<()> {
        let Some(repo) = setup_repo().await? else {
            return Ok(());
        };

        let wallet_address = format!("test-wallet-{}", Uuid::new_v4());
        let event = AuditEventRecord {
            event_id: String::new(),
            event_type: "auth_bind".to_owned(),
            wallet_address: Some(wallet_address.clone()),
            user_id: Some("test-user-2".to_owned()),
            chain: Some("flowcortex-l1".to_owned()),
            outcome: "success".to_owned(),
            message: Some("integration test".to_owned()),
            timestamp_epoch_ms: 1_700_000_000_123,
        };

        let event_id = repo.append_audit_event(&event).await?;
        assert!(!event_id.trim().is_empty());

        let events = repo
            .list_audit_events(10, Some("auth_bind"), Some(wallet_address.as_str()), Some("success"))
            .await?;

        assert!(events.iter().any(|entry| {
            entry.event_id == event_id
                && entry.event_type == "auth_bind"
                && entry.wallet_address.as_deref() == Some(wallet_address.as_str())
                && entry.outcome == "success"
        }));

        Ok(())
    }

    #[tokio::test]
    async fn postgres_challenge_lifecycle_roundtrip() -> anyhow::Result<()> {
        let Some(repo) = setup_repo().await? else {
            return Ok(());
        };

        let challenge = format!("challenge-{}", Uuid::new_v4());
        let issued = 1_700_000_000_500_u128;
        let expires = issued + 120_000;
        let used_at = issued + 5_000;

        repo.upsert_challenge(&challenge, issued, expires).await?;
        repo.mark_challenge_used(&challenge, used_at).await?;

        let row = repo
            .client
            .query_one(
                "SELECT challenge, used, used_at_epoch_ms
                 FROM challenge_store
                 WHERE challenge = $1",
                &[&challenge],
            )
            .await?;

        let saved_challenge: String = row.get(0);
        let used: bool = row.get(1);
        let used_at_epoch_ms: Option<i64> = row.get(2);

        assert_eq!(saved_challenge, challenge);
        assert!(used);
        assert_eq!(used_at_epoch_ms, Some(to_i64(used_at)));

        Ok(())
    }
}
