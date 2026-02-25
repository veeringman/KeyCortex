<img src="./keycortex_logo.png" alt="KeyCortex Logo" width="180" />

# KeyCortex Operator Dashboard Spec (Diagnostics)

## Purpose

Define a compact operational dashboard for wallet-service runtime diagnostics using:

- `GET /health`
- `GET /readyz`
- `GET /startupz`

This dashboard is focused on production-safe visibility for:

- storage backend state (`rocksdb-only` vs `postgres+rocksdb`)
- Postgres startup and migration posture
- fallback counter behavior over time
- auth/JWKS readiness posture

---

## Dashboard Layout (Single View)

### A) Service Status Strip

Cards:

1. Service Status (`ok` / degraded)
2. Readiness (`ready` boolean)
3. Storage Mode (`rocksdb-only` or `postgres+rocksdb`)
4. Auth Mode (`rs256-jwks` or `hs256-fallback`)

Data sources:

- `/health.status`
- `/readyz.ready`
- `/health.storage_mode`
- `/health.auth_mode`

---

### B) Postgres Startup Panel

Fields:

- `configured`
- `enabled`
- `migrations_dir`
- `migration_files_applied`
- `last_error`

Data source:

- `/startupz.postgres_startup`

Rules:

- If `configured=true` and `enabled=false`, show CRITICAL.
- If `last_error != null`, show WARNING.
- If `migration_files_applied == 0` in Postgres-enabled environment, show WARNING unless explicitly expected.

---

### C) Fallback Counters Panel

Fields:

- `total`
- `postgres_unavailable`
- `challenge_persist_failures`
- `challenge_mark_used_failures`
- `binding_write_failures`
- `binding_read_failures`
- `audit_write_failures`
- `audit_read_failures`

Data source:

- `/startupz.db_fallback_counters`

Display:

- current counter value
- 5m delta
- 30m delta

Rules:

- Rising deltas indicate live fallback activity and should be correlated with Postgres/JWKS infrastructure events.

---

### D) Auth/JWKS Panel

Fields:

- `jwks_loaded`
- `jwks_source`
- `last_jwks_refresh_epoch_ms`
- `last_jwks_error`
- `auth_mode`

Data source:

- `/startupz` and `/health`

Rules:

- `auth_mode=hs256-fallback` in production should be WARNING unless explicitly approved.
- Non-null `last_jwks_error` should be WARNING.

---

## Polling and Retention

- Poll interval: 15s (default), configurable to 30s.
- Store time-series points for fallback deltas and readiness for 7 days.
- Keep latest startup snapshot as current state card set.

---

## Minimal Alert Conditions

1. Readiness down
   - Condition: `/readyz.ready == false` for 2 consecutive polls.
   - Severity: CRITICAL.

2. Postgres configured but disabled
   - Condition: `/startupz.postgres_startup.configured == true && enabled == false`.
   - Severity: CRITICAL.

3. Fallback activity spike
   - Condition: `db_fallback_counters.total` 5m delta exceeds threshold table.
   - Severity: WARNING/CRITICAL by threshold.

4. JWKS errors persisting
   - Condition: non-null `last_jwks_error` for >= 3 consecutive polls.
   - Severity: WARNING.

---

## API Mapping Cheat Sheet

- `/health`: quick liveness + mode snapshot
- `/readyz`: dependency readiness gate
- `/startupz`: deep startup + fallback + auth diagnostics
