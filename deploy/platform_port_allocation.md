# Platform Port Allocation — Unified Ecosystem

> **Version:** 1.0 · **Last updated:** 2026-02-27
> **Scope:** All 6 platforms in the KeyCortex / Treasury Settlement ecosystem

---

## Port Allocation Scheme

Each platform gets a **10-port range** for clean isolation. Within each range:

| Offset | Purpose               | Example (KeyCortex) |
|--------|-----------------------|---------------------|
| `+0`   | API server            | 8080                |
| `+1`   | Frontend JS (nginx)   | 8081                |
| `+2`   | Frontend WASM (nginx) | 8082                |
| `+3`   | Admin / Dashboard     | 8083                |
| `+4`   | WebSocket / streaming | 8084                |
| `+5…9` | Reserved for growth   | 8085–8089           |

---

## Master Port Table

| Platform              | Range       | API    | UI-JS  | UI-WASM | Admin  | DB (Postgres) |
|-----------------------|-------------|--------|--------|---------|--------|---------------|
| **KeyCortex**         | 8080–8089   | 8080   | 8081   | 8082    | 8083   | 5432          |
| **AuthBuddy IdP**     | 8100–8109   | 8100   | 8101   | 8102    | 8103   | 5433          |
| **FlowCortex L1**     | 8200–8209   | 8200   | 8201   | 8202    | 8203   | 5434          |
| **ProofCortex**       | 8300–8309   | 8300   | 8301   | 8302    | 8303   | 5435          |
| **FortressDigital**   | 8400–8409   | 8400   | 8401   | 8402    | 8403   | 5436          |
| **Treasury App**      | 8500–8509   | 8500   | 8501   | 8502    | 8503   | 5437          |

### Notes

- **DB ports** (5432–5437): Each platform gets its own Postgres instance in dev/staging.
  In production, a shared Postgres cluster with separate databases is recommended.
- **FlowCortex 8200**: This is the RPC/API port. The P2P/gossip port (30303) is separate.
- **ProofCortex 8303**: Admin dashboard shows proof queue status and circuit metrics.
- **Treasury App 8500**: The orchestrator — it calls all other platforms' API ports.
- Platforms that don't need WASM leave `+2` unused.
- Platforms that don't need a UI (e.g., ProofCortex STARK prover) leave `+1` and `+2` unused.

---

## Docker Network Topology

All platforms on a shared Docker bridge network `ecosystem-net` can address each other by service name:

```
┌──────────────────────────────────────────────────────────────────┐
│  ecosystem-net (bridge)                                          │
│                                                                  │
│  keycortex-api:8080     authbuddy-api:8100     flowcortex:8200   │
│  keycortex-ui-js:80     authbuddy-ui-js:80     flowcortex-ui:80  │
│  keycortex-ui-wasm:80   authbuddy-db:5432      flowcortex-db:5432│
│  keycortex-db:5432                                               │
│                                                                  │
│  proofcortex-api:8300   fortress-api:8400     treasury-api:8500  │
│  proofcortex-db:5432    fortress-ui-js:80     treasury-ui-js:80  │
│                         fortress-db:5432      treasury-db:5432   │
└──────────────────────────────────────────────────────────────────┘
```

Inside the network, services talk on their internal ports (80, 5432, 8080 etc.).
The host port mapping (8080, 8100, …) is only for external access.

---

## Environment Variables (Cross-Platform)

Each platform's API server should accept these for inter-service communication:

```bash
# KeyCortex .env
KEYCORTEX_API_URL=http://keycortex-api:8080
AUTHBUDDY_API_URL=http://authbuddy-api:8100
FLOWCORTEX_RPC_URL=http://flowcortex-api:8200
PROOFCORTEX_API_URL=http://proofcortex-api:8300
FORTRESS_API_URL=http://fortress-api:8400
TREASURY_API_URL=http://treasury-api:8500
```

---

## KeyCortex Migration Note

KeyCortex currently uses 8090 (UI-JS) and 8091 (UI-WASM) from the original setup.
To align with this scheme, update to 8081/8082 when all teams adopt it.
During transition, both mappings can coexist:

```yaml
ports:
  - "8081:80"   # New standard
  - "8090:80"   # Legacy compat (remove after migration)
```
