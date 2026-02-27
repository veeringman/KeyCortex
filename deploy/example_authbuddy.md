###############################################################################
# AuthBuddy — Filled Example
#
# This shows what the templates look like after filling in {{...}} placeholders
# for the AuthBuddy IdP platform. Copy and adapt for your platform.
###############################################################################

# ─── Placeholder Mapping ─────────────────────────────────────────────────────
#
#   {{PLATFORM_NAME}}   → AuthBuddy
#   {{PLATFORM_SLUG}}   → authbuddy
#   {{PORT_API}}        → 8100
#   {{PORT_UI_JS}}      → 8101
#   {{PORT_UI_WASM}}    → 0        (AuthBuddy has no WASM frontend)
#   {{PORT_DB}}         → 5433
#   {{API_BINARY}}      → authbuddy-service

# ─── setup_docker.sh customisation ───────────────────────────────────────────
#
# PLATFORM_NAME="AuthBuddy"
# PLATFORM_SLUG="authbuddy"
# PORT_API=8100
# PORT_UI_JS=8101
# PORT_UI_WASM=0
# PORT_DB=5433
# API_BINARY="authbuddy-service"
# HEALTH_ENDPOINT="/health"
# HAS_WASM_UI=false

# ─── docker-compose.yml (key services) ───────────────────────────────────────
#
# services:
#   authbuddy-api:
#     build: .
#     container_name: authbuddy-api
#     ports:
#       - "8100:8100"
#     environment:
#       - KEYCORTEX_API_URL=http://keycortex-api:8080
#       - JWT_SIGNING_KEY=${JWT_SIGNING_KEY:-dev-key}
#     healthcheck:
#       test: ["CMD", "curl", "-sf", "http://localhost:8100/health"]
#
#   postgres:
#     container_name: authbuddy-db
#     ports:
#       - "5433:5432"             # Note: 5433 on host, 5432 inside
#     environment:
#       POSTGRES_USER: authbuddy
#       POSTGRES_DB: authbuddy
#
#   ui-js:
#     container_name: authbuddy-ui-js
#     ports:
#       - "8101:80"
#
# No ui-wasm service needed for AuthBuddy.

# ─── Smoke tests to add ──────────────────────────────────────────────────────
#
# smoke_check "JWKS endpoint"   "http://localhost:8100/.well-known/jwks.json"
# smoke_check "Token endpoint"  "http://localhost:8100/auth/token" "405"  # POST only
# smoke_check "Health"          "http://localhost:8100/health"

# ─── Quick-start steps for AuthBuddy team ────────────────────────────────────
#
# 1. Copy templates from deploy/ into your AuthBuddy repo
#    cp deploy/setup_docker_template.sh  authbuddy-repo/scripts/setup_docker.sh
#    cp deploy/docker-compose_template.yml authbuddy-repo/docker-compose.yml
#    cp deploy/Dockerfile_template       authbuddy-repo/Dockerfile
#
# 2. Search-replace placeholders
#    sed -i 's/{{PLATFORM_NAME}}/AuthBuddy/g; s/{{PLATFORM_SLUG}}/authbuddy/g; \
#            s/{{PORT_API}}/8100/g; s/{{PORT_UI_JS}}/8101/g; \
#            s/{{PORT_UI_WASM}}/0/g; s/{{PORT_DB}}/5433/g; \
#            s/{{API_BINARY}}/authbuddy-service/g' \
#      scripts/setup_docker.sh docker-compose.yml Dockerfile
#
# 3. Customise platform-specific sections (marked CUSTOMISE: in templates)
#
# 4. chmod +x scripts/setup_docker.sh && ./scripts/setup_docker.sh
