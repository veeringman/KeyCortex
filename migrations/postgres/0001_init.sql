CREATE TABLE IF NOT EXISTS wallet_bindings (
  wallet_address TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  chain TEXT NOT NULL,
  last_verified_epoch_ms BIGINT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS challenge_store (
  challenge TEXT PRIMARY KEY,
  issued_at_epoch_ms BIGINT NOT NULL,
  expires_at_epoch_ms BIGINT NOT NULL,
  used BOOLEAN NOT NULL DEFAULT FALSE,
  used_at_epoch_ms BIGINT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS verification_logs (
  log_id TEXT PRIMARY KEY,
  event_type TEXT NOT NULL,
  wallet_address TEXT NULL,
  user_id TEXT NULL,
  chain TEXT NULL,
  outcome TEXT NOT NULL,
  message TEXT NULL,
  timestamp_epoch_ms BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_wallet_bindings_user_id
  ON wallet_bindings(user_id);

CREATE INDEX IF NOT EXISTS idx_challenge_store_expires
  ON challenge_store(expires_at_epoch_ms);

CREATE INDEX IF NOT EXISTS idx_verification_logs_timestamp
  ON verification_logs(timestamp_epoch_ms DESC);

CREATE INDEX IF NOT EXISTS idx_verification_logs_wallet
  ON verification_logs(wallet_address);

CREATE INDEX IF NOT EXISTS idx_verification_logs_event_outcome
  ON verification_logs(event_type, outcome);
