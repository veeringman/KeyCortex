<img src="./keycortex_logo.png" alt="KeyCortex Logo" width="180" />

# KeyCortex `/startupz` Field Ownership Matrix

## Purpose

Define who owns each diagnostics field, who responds to alerts, and expected action timeline.

---

## Ownership Matrix

| Field / Group | Primary Owner | Secondary Owner | Alert Trigger | First Response SLA | Typical Action |
|---|---|---|---|---|---|
| `storage_mode`, `postgres_enabled` | Service Team | Platform Team | Unexpected mode in environment | 15 min | Validate config + DB availability |
| `postgres_startup.configured` | Platform Team | Service Team | false where Postgres required | 15 min | Fix environment config |
| `postgres_startup.enabled` | Platform Team | Service Team | false with required Postgres | 10 min | Restore DB connectivity |
| `postgres_startup.migration_files_applied` | Service Team | Platform Team | 0 or unexpected count | 30 min | Validate migration dir and startup logs |
| `postgres_startup.last_error` | Service Team | Platform Team | non-null for >1 poll | 15 min | Triage startup/migration error |
| `db_fallback_counters.postgres_unavailable` | Platform Team | Service Team | >=1 in 5m | 10 min | Resolve DB init/connectivity |
| `db_fallback_counters.binding_write_failures` | Service Team | Platform Team | >=5 in 5m | 10 min | Check DB write path, logs, latency |
| `db_fallback_counters.audit_write_failures` | Service Team | Platform Team | >=5 in 5m | 10 min | Check DB write path, log throughput |
| `db_fallback_counters.binding_read_failures` | Service Team | Platform Team | >=10 in 5m | 15 min | Check read query health/indexes |
| `db_fallback_counters.audit_read_failures` | Service Team | Platform Team | >=10 in 5m | 15 min | Check read query health/indexes |
| `db_fallback_counters.challenge_persist_failures` | Service Team | Platform Team | >=3 in 5m | 10 min | Validate challenge write path |
| `db_fallback_counters.challenge_mark_used_failures` | Service Team | Platform Team | >=3 in 5m | 10 min | Validate challenge update path |
| `db_fallback_counters.total` | Service Team | Platform Team | >10 in 5m | 10 min | Run fallback incident playbook |
| `auth_mode`, `jwks_loaded` | Identity/Auth Team | Service Team | fallback mode unexpected | 15 min | Validate JWKS auth pipeline |
| `last_jwks_error`, `last_jwks_refresh_epoch_ms` | Identity/Auth Team | Service Team | non-null or stale refresh | 15 min | Fix JWKS source/reachability |

---

## Escalation Rules

- If field owner does not acknowledge within SLA, escalate to secondary owner.
- If two or more critical fields alert simultaneously, open major incident bridge.
- If `db_fallback_counters.total` critical condition persists >15 minutes, escalate to platform incident commander.

---

## Review Cadence

- Review matrix monthly.
- Update owners after org/on-call rotation changes.
- Recalibrate thresholds after each major incident.
