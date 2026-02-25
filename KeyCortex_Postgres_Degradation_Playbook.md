<img src="./keycortex_logo.png" alt="KeyCortex Logo" width="180" />

# KeyCortex Operations Playbook: Sustained Postgres Degradation

## Scope

Use this playbook when wallet-service continues operating in fallback mode and Postgres-related failures persist.

Indicators:

- `/startupz.db_fallback_counters.total` rises continuously for >10 minutes.
- `/startupz.postgres_startup.enabled` is false in an environment that expects Postgres.
- `binding_*_failures`, `audit_*_failures`, or `challenge_*_failures` counters repeatedly increase.

---

## Severity Classification

### SEV-2 (Major Degradation)

- fallback counters rising above warning thresholds
- user traffic still served
- no full service outage

### SEV-1 (Critical)

- fallback counters exceed critical thresholds in 2 consecutive windows
- Postgres unavailable and recovery not immediate
- readiness/critical user operations degraded

---

## Immediate Actions (First 15 Minutes)

1. Confirm current state:

```bash
curl -s http://<service>/startupz | jq
curl -s http://<service>/readyz | jq
```

2. Capture counters snapshot and 5-minute delta.
3. Validate Postgres reachability from service network.
4. Check DB credentials/secret rotation status.
5. Check migration status (`postgres_startup.migration_files_applied`).

If Postgres is expected but disabled, escalate to DB/platform on-call immediately.

---

## Containment Strategy

- Keep wallet-service online; fallback writes/reads continue through RocksDB.
- Avoid restarting service repeatedly unless required; preserve diagnostic continuity.
- If DB saturation is suspected:
  - reduce non-critical DB read load
  - increase DB resources/connections as per platform policy
- If network path is unstable:
  - fail traffic over to healthy DB endpoint/region (if supported)

---

## Recovery Procedure

1. Restore Postgres health (platform/DB team).
2. Verify service sees Postgres as enabled:

```bash
curl -s http://<service>/startupz | jq '.postgres_startup'
```

3. Confirm fallback counters plateau (delta approaches 0).
4. Run smoke checks:

```bash
BASE_URL=http://<service> ./scripts/smoke_db_fallback.sh
```

5. Keep incident in monitor phase for at least 30 minutes.

---

## Verification Checklist

- `/readyz.ready == true`
- `/startupz.postgres_startup.enabled == true`
- `/startupz.postgres_startup.last_error == null`
- fallback counter growth returns to baseline
- no new user-facing error spike

---

## Communications Template

### Initial Update

"KeyCortex is operating with elevated DB fallback activity due to Postgres degradation. Service remains available via RocksDB fallback. DB/platform recovery in progress."

### Recovery Update

"Postgres connectivity has been restored, migration/startup diagnostics are healthy, and fallback counters have stabilized. Monitoring remains active."

---

## Post-Incident Follow-Up

- attach startup and fallback snapshots to incident record
- identify dominant failing counter type(s)
- document root cause and timeline
- add/update alert tuning if thresholds were noisy
- create engineering follow-up tasks for prevention
