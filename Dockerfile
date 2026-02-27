###############################################################################
# KeyCortex — Multi-stage Docker Build
# Stage 1: Build Rust backend + WASM frontend
# Stage 2: Minimal runtime image
###############################################################################

# ── Stage 1: Builder ─────────────────────────────────────────────────────────
FROM rust:1.93-bookworm AS builder

RUN apt-get update && apt-get install -y \
    clang llvm libclang-dev pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

# Install wasm-pack for WASM frontend build
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /src

# Copy dependency manifests first for better layer caching
COPY Cargo.toml Cargo.lock* ./
COPY crates/kc-api-types/Cargo.toml crates/kc-api-types/
COPY crates/kc-crypto/Cargo.toml crates/kc-crypto/
COPY crates/kc-wallet-core/Cargo.toml crates/kc-wallet-core/
COPY crates/kc-storage/Cargo.toml crates/kc-storage/
COPY crates/kc-chain-client/Cargo.toml crates/kc-chain-client/
COPY crates/kc-chain-flowcortex/Cargo.toml crates/kc-chain-flowcortex/
COPY crates/kc-auth-adapter/Cargo.toml crates/kc-auth-adapter/
COPY services/wallet-service/Cargo.toml services/wallet-service/
COPY ui/wallet-wasm/Cargo.toml ui/wallet-wasm/

# Create stub lib.rs for each crate so cargo can resolve the workspace and cache deps
RUN mkdir -p crates/kc-api-types/src crates/kc-crypto/src crates/kc-wallet-core/src \
             crates/kc-storage/src crates/kc-chain-client/src crates/kc-chain-flowcortex/src \
             crates/kc-auth-adapter/src services/wallet-service/src ui/wallet-wasm/src && \
    for d in crates/*/src services/*/src ui/wallet-wasm/src; do \
      echo '' > "$d/lib.rs"; \
    done && \
    echo 'fn main() {}' > services/wallet-service/src/main.rs

# Pre-build dependencies (this layer is cached as long as Cargo.toml/Cargo.lock don't change)
RUN cargo build -p wallet-service --release 2>/dev/null || true && \
    cargo build -p wallet-wasm --release 2>/dev/null || true

# Now copy actual source code
COPY . .

# Build wallet-service (release)
RUN cargo build -p wallet-service --release && \
    strip target/release/wallet-service

# Build WASM frontend (release)
RUN rustup target add wasm32-unknown-unknown && \
    wasm-pack build ui/wallet-wasm --target web --release \
      --out-dir ui/wallet-wasm/pkg --no-typescript

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 curl python3 jq git \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /usr/sbin/nologin -m -d /app keycortex

WORKDIR /app

# Copy binary
COPY --from=builder /src/target/release/wallet-service /app/bin/wallet-service

# Copy migrations
COPY --from=builder /src/migrations /app/migrations

# Copy JS baseline frontend
COPY --from=builder /src/ui/wallet-baseline /app/ui/wallet-baseline

# Copy WASM frontend (with built pkg/)
COPY --from=builder /src/ui/wallet-wasm /app/ui/wallet-wasm

# Copy scripts (for watchdog)
COPY --from=builder /src/scripts /app/scripts

# Prepare data directories
RUN mkdir -p /app/data/keystore/rocksdb /app/logs && \
    chown -R keycortex:keycortex /app

USER keycortex

# Environment defaults
ENV RUST_LOG=info
ENV KEYCORTEX_KEYSTORE_PATH=/app/data/keystore/rocksdb
ENV KEYCORTEX_POSTGRES_MIGRATIONS_DIR=/app/migrations/postgres
ENV AUTHBUDDY_JWT_SECRET=change-me-in-production

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --start-period=15s --retries=3 \
  CMD curl -sf http://localhost:8080/readyz || exit 1

ENTRYPOINT ["/app/bin/wallet-service"]
