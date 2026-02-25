# KeyCortex Learnings

A running log of practical engineering lessons from local development, testing, and operations.

## 2026-02-25 — Wallet Service Build/Test Learnings

### 1) Avoid repeat `librocksdb-sys` rebuilds

- `cargo run` can trigger long rebuilds when profile/artifact state changes.
- Prefer running the compiled binary directly for repeated smoke checks:
  - `target/debug/wallet-service`
- Use persistent target cache for long-term reuse across sessions:
  - `export CARGO_TARGET_DIR="$HOME/.cache/keycortex/cargo-target"`

### 2) Standard cached launcher

- Use:
  - `./scripts/run_wallet_service_cached.sh`
- Script behavior:
  - runs cached binary from `$CARGO_TARGET_DIR/debug/wallet-service` if present
  - seeds cache from `./target/debug/wallet-service` if available
  - builds only when no binary exists
- Force refresh build when needed:
  - `./scripts/run_wallet_service_cached.sh --rebuild`

### 3) Fast smoke-test pattern

After service is running on `:8080`:

- `curl -sS -w '\nHTTP %{http_code}\n' http://127.0.0.1:8080/health`
- `curl -sS -w '\nHTTP %{http_code}\n' http://127.0.0.1:8080/readyz`
- `curl -sS -w '\nHTTP %{http_code}\n' http://127.0.0.1:8080/startupz`

Expected for baseline local run:

- all three endpoints return `HTTP 200`
- `storage_mode` is commonly `rocksdb-only` unless Postgres is configured

### 4) Build failure root-cause reminder

A recent compile break came from deriving `Clone` on a struct holding `tokio_postgres::Client`.

- `tokio_postgres::Client` is not `Clone`
- avoid `#[derive(Clone)]` on repository wrappers that store it directly
- shared usage should be done via `Arc<PostgresRepository>` at app state boundaries

### 5) Practical workflow (recommended)

1. Run tests: `cargo test -p wallet-service`
2. Start service via cache: `./scripts/run_wallet_service_cached.sh`
3. Smoke-check diagnostics endpoints (`/health`, `/readyz`, `/startupz`)
4. Rebuild only when code changed and binary mismatch is expected
