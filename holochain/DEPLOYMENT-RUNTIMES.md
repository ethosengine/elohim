# Deployment Runtimes Guide

This guide explains the different runtime environments for developing and deploying the Elohim Protocol with Holochain.

## Quick Reference: Environment Matrix

| Build Config | App Deployment | Holochain URL | Holochain Deployment |
|--------------|----------------|---------------|----------------------|
| `alpha` | `alpha.elohim.host` | `wss://holochain-dev.elohim.host` | `elohim-edgenode-dev` |
| `staging` | `staging.elohim.host` | `wss://holochain-dev.elohim.host` | `elohim-edgenode-dev` |
| `production` | `elohim.host` | `wss://holochain.elohim.host` | `elohim-edgenode-prod` |

**Consolidated Holochain Architecture:**
- **Pre-production** (`holochain-dev.elohim.host`): Serves both alpha and staging apps
  - Enables RNA (DNA version) testing before production
  - Single conductor can host multiple hApp versions
  - Reduces infrastructure complexity
- **Production** (`holochain.elohim.host`): Reserved for production deployments

**Branch to Build Config Mapping:**
- `dev` branch, `feat-*`, `claude/*` branches → `alpha` config
- `staging` branch → `staging` config
- `main` branch → `production` config

---

## Runtime 1: Local Development (Eclipse Che + Local Holochain)

**Use case:** Active development, debugging, testing zome calls

```
┌─────────────────────────────────────────────────────────────────────┐
│  Eclipse Che Workspace                                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Browser (Angular)                                                   │
│      │                                                               │
│      │ wss://<workspace>-hc-dev.code.ethosengine.com/admin          │
│      ▼                                                               │
│  Che Endpoint (hc-dev :8888)                                        │
│      │                                                               │
│      ▼                                                               │
│  Dev Proxy (:8888)                                                  │
│      ├── /admin     → Local Conductor (:4444)                       │
│      └── /app/:port → Local Conductor (:4445-4500)                  │
│                                                                      │
│  Local Holochain Conductor (sandbox)                                │
│      ├── Admin WebSocket (:4444)                                    │
│      ├── App WebSocket (:4445)                                      │
│      └── DHT (via holostrap.elohim.host)                            │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**How it works:**
1. Angular app auto-detects Che environment (`*.code.ethosengine.com`)
2. Resolves hc-dev endpoint URL (replaces `-angular-dev` with `-hc-dev`)
3. Doorway routes WebSocket paths to local conductor
4. No authentication needed (internal network)

**Start the stack:**
```bash
npm run hc:start  # In elohim-app/
```

---

## Runtime 2: Eclipse Che → Deployed Dev Holochain

**Use case:** Testing against shared infrastructure from Che

```
┌─────────────────────────────────────────────────────────────────────┐
│  Eclipse Che Workspace                                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Browser (Angular with useLocalProxy: false)                        │
│      │                                                               │
│      │ wss://holochain-dev.elohim.host?apiKey=dev-elohim-auth-2024  │
│      ▼                                                               │
└──────┼──────────────────────────────────────────────────────────────┘
       │
       │ (Internal network - IP whitelisted)
       ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Kubernetes Cluster (ethosengine namespace)                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Ingress (holochain-dev.elohim.host)                                │
│      │                                                               │
│      ▼                                                               │
│  Doorway Gateway (:8080)                                            │
│      ├── API Key / JWT validation                                   │
│      ├── Permission filtering (PUBLIC/AUTHENTICATED/ADMIN)          │
│      └── Operation whitelist + Worker Pool                          │
│      │                                                               │
│      ▼                                                               │
│  Socat Proxy (:8444) → Conductor (:4444, localhost-only)            │
│      │                                                               │
│      ▼                                                               │
│  DHT Network (via holostrap.elohim.host)                            │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Configuration for this mode:**
```typescript
// In environment.ts or environment.alpha.ts
holochain: {
  adminUrl: 'wss://holochain-dev.elohim.host',
  appUrl: 'wss://holochain-dev.elohim.host',
  proxyApiKey: 'dev-elohim-auth-2024',
  useLocalProxy: false,  // <-- Key setting
}
```

---

## Runtime 3: Deployed Alpha/Staging Apps → Deployed Dev Holochain

**Use case:** Pre-production testing by stakeholders at `alpha.elohim.host` or `staging.elohim.host`

```
┌─────────────────────────────────────────────────────────────────────┐
│  User's Browser                                                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  https://alpha.elohim.host  OR  https://staging.elohim.host         │
│      │                                                               │
│      │ Loads Angular app (built with --configuration=alpha/staging) │
│      │                                                               │
│      │ wss://holochain-dev.elohim.host?apiKey=dev-elohim-auth-2024  │
│      ▼                                                               │
└──────┼──────────────────────────────────────────────────────────────┘
       │
       │ (Requires internal network access)
       ▼
┌─────────────────────────────────────────────────────────────────────┐
│  elohim-edgenode-dev (Kubernetes)                                   │
│  Same as Runtime 2                                                  │
│                                                                      │
│  Consolidated architecture - supports multiple hApp versions:       │
│  ├── lamad-spike (current version)                                  │
│  └── lamad-v2 (future RNA upgrades)                                 │
└─────────────────────────────────────────────────────────────────────┘
```

**Note:** Both alpha and staging apps connect to `holochain-dev.elohim.host` which is IP-whitelisted. This means:
- Works from internal network (Che, cluster pods, VPN)
- Does NOT work from public internet without VPN/port-forward
- Enables testing RNA version upgrades before production deployment

---

## Runtime 4: Future Production (Public Web)

**Use case:** Public visitors at `elohim.host`

```
┌─────────────────────────────────────────────────────────────────────┐
│  Public Internet                                                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Browser → https://elohim.host                                      │
│      │                                                               │
│      │ Stage 1 (Visitor): Read-only commons content                 │
│      │                    No Holochain connection needed            │
│      │                                                               │
│      │ Stage 2 (Hosted): wss://holochain.elohim.host                │
│      │                   Custodial keys, authenticated              │
│      ▼                                                               │
│  Doorway Gateway (public, no IP whitelist)                          │
│      ├── Rate limiting                                              │
│      ├── Abuse detection                                            │
│      └── Audit logging                                              │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Status:** Not yet deployed. Requires removing IP whitelist and hardening Doorway gateway.

---

## User Experience Pipeline

The Elohim Protocol supports a progressive journey from anonymous visitor to self-sovereign node operator:

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   Stage 1    │    │   Stage 2    │    │   Stage 3    │    │   Stage 4    │
│   Visitor    │───▶│   Hosted     │───▶│   App User   │───▶│   Node       │
│              │    │   Human      │    │  (Installed) │    │   Operator   │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
      │                    │                   │                    │
      ▼                    ▼                   ▼                    ▼
  No account         Browser +          Desktop app           Always-on
  Read commons       Custodial keys     Self-sovereign        Full DHT
  No Holochain       Edge Node          Intermittent DHT      participation
```

### Stage 1: Visitor
- **Access:** Public web at elohim.host
- **Identity:** Anonymous session
- **Holochain:** None - content served from DNS-exposed nodes
- **Capabilities:** Browse commons content

### Stage 2: Hosted Human
- **Access:** Account on elohim.host
- **Identity:** Custodial keys managed by Edge Node
- **Holochain:** Browser → Admin Proxy → Conductor
- **Capabilities:** Full DHT participation, create content, build reputation
- **Migration:** Export keys when ready for Stage 3

### Stage 3: App User (Installed)
- **Access:** Desktop/laptop app (Electron/Tauri)
- **Identity:** Self-sovereign keys on device
- **Holochain:** Local conductor, intermittent DHT sync
- **Capabilities:** Offline access, local AI inference, reduced dependency
- **Migration:** Deploy always-on hardware for Stage 4

### Stage 4: Node Operator
- **Access:** Always-on device (Family Node)
- **Identity:** Self-sovereign with full network participation
- **Holochain:** Always-on conductor, DHT shard hosting
- **Capabilities:** Bootstrap/signal capability, public DNS hosting, backup services

---

## Holochain Infrastructure Components

### Doorway Gateway (`holochain/doorway/`)
Rust-based secure gateway protecting the Holochain Admin API:
- **Port:** 8080 (public via ingress), 8888 (local dev)
- **Auth:** API key via query parameter (`?apiKey=...`) or JWT tokens
- **Permissions:** PUBLIC (read-only), AUTHENTICATED (normal workflow), ADMIN (destructive)
- **Whitelist:** Only known operations allowed (unknown operations blocked)
- **Worker Pool:** 4 persistent connections to conductor prevent thread starvation

### Edge Node (`holochain/edgenode/`)
Kubernetes-deployed Holochain conductor:
- **Base image:** `ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom` (Holochain 0.6.0)
- **Ports:** 4444 (admin, localhost-only), 8444 (socat proxy), 8080 (doorway)
- **Storage:** emptyDir (ephemeral)
- **Network:** holostrap.elohim.host for bootstrap/signal

### Socat Sidecar
Bridges localhost-only conductor to network:
```yaml
# Conductor binds to localhost:4444 (not configurable)
# Socat forwards external traffic to it
args: ["TCP-LISTEN:8444,fork,reuseaddr", "TCP:127.0.0.1:4444"]
```

---

## Troubleshooting

### "WebSocket connection failed" in browser
1. **Check build config:** Verify the app was built with correct configuration
   - Look at console log: "Admin URL resolved (direct): wss://..."
   - If it shows `holochain.elohim.host` but should be `holochain-dev.elohim.host`, rebuild with correct config

2. **Check network access:**
   - Edge nodes are IP-whitelisted (internal network only)
   - From Che: Direct access works
   - From external: Use VPN or port-forward

3. **Check conductor is running:**
   ```bash
   kubectl get pods -n ethosengine -l app=elohim-edgenode
   kubectl logs -n ethosengine <pod> -c edgenode
   ```

### "Operation not permitted" from Doorway
- Check if operation is in the whitelist (`holochain/doorway/src/auth/permissions.rs`)
- Verify API key is correct for required permission level
- Check Doorway logs: `kubectl logs -n ethosengine <pod> -c doorway`

### Connection timeout
- Verify conductor readiness: check probe status
- Check socat sidecar is running
- Verify ingress is properly configured

---

## Content Seeding

The seeder populates Holochain with structured content from `/data/lamad`.

### Local Seeding

```bash
# From elohim-app/
npm run hc:seed              # Full seed
npm run hc:seed:sample       # Sample (10 items)
```

### Remote Seeding

```bash
# Dev environment (holochain-dev.elohim.host)
npm run hc:seed:dev
npm run hc:seed:dev:sample   # Test with 10 items first

# Production (holochain.elohim.host)
export ELOHIM_PROD_API_KEY="your-production-api-key"
npm run hc:seed:prod
```

### How Remote Seeding Works

```
┌─────────────────────────────────────────────────────────────────────┐
│  Seeder Script                                                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. Connect to Admin WebSocket                                       │
│     wss://holochain-dev.elohim.host?apiKey=dev-elohim-auth-2024     │
│                                                                      │
│  2. Get app auth token                                               │
│     issueAppAuthenticationToken() → token                            │
│                                                                      │
│  3. List/create app interface                                        │
│     listAppInterfaces() → port (e.g., 43733)                        │
│                                                                      │
│  4. Connect to App WebSocket (auto-resolved URL)                     │
│     wss://holochain-dev.elohim.host/app/43733?apiKey=...            │
│                                                                      │
│  5. Seed content via zome calls                                      │
│     bulk_create_content, create_path, etc.                          │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Important:** Only set `HOLOCHAIN_ADMIN_URL` for remote seeding. The seeder
automatically resolves the app URL using the `/app/:port` routing pattern.

### Checking Content Stats

```bash
npm run hc:stats:dev         # Dev environment
npm run hc:stats:prod        # Production
```

For detailed seeder documentation, see `holochain/seeder/README.md`.

---

## Related Documentation

- `holochain/claude.md` - Main architecture documentation
- `holochain/edgenode/README.md` - Edge node quick reference
- `holochain/seeder/README.md` - Seeder documentation
- `elohim-app/src/app/elohim/services/holochain-client.service.ts` - Client implementation
