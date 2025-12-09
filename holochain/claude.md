# Holochain Infrastructure

This directory contains the Holochain P2P infrastructure for the Elohim Protocol, enabling browser-to-DHT connectivity via Edge Nodes.

## Architecture

```
Browser → WebSocket → Edge Node (K8s) → DHT (Holochain network)
                          ↓
                    socat sidecar (0.0.0.0:8444)
                          ↓
                    Conductor (127.0.0.1:4444)
```

### Security Model

**IMPORTANT:** The Admin WebSocket is protected by **IP whitelist** on the Ingress. Only internal network IPs can access it:
- `10.0.0.0/8` - Kubernetes pod network
- `172.16.0.0/12` - Docker/internal networks
- `192.168.0.0/16` - Private networks

The Holochain Admin API is highly privileged and allows:
- Installing/uninstalling apps
- Generating agent keys
- Granting zome call capabilities
- Dumping conductor state

Access is allowed from:
- Eclipse Che workspaces (internal network)
- Kubernetes pods
- VPN-connected clients

A secure proxy layer will be added later for production public web access.

```bash
# From Eclipse Che or internal network
wss://holochain-dev.elohim.host

# For external access, use port-forward
kubectl port-forward -n ethosengine deploy/elohim-edgenode-dev 4444:8444
# Then: ws://localhost:4444
```

## Directory Structure

```
holochain/
├── Jenkinsfile           # CI/CD pipeline for DNA build & Edge Node deployment
├── dna/
│   └── lamad-spike/      # Minimal DNA for browser connectivity testing
│       ├── flake.nix     # Nix flake for Holonix dev environment
│       ├── dna.yaml      # DNA manifest
│       ├── workdir/
│       │   └── happ.yaml # hApp manifest
│       └── zomes/
│           ├── content_store/           # Coordinator zome
│           └── content_store_integrity/ # Integrity zome
├── edgenode/
│   ├── Dockerfile        # Custom Edge Node image (extends holo-host/edgenode)
│   └── conductor-config.yaml  # (Not currently used - base image has own config)
└── manifests/
    ├── edgenode-dev.yaml    # Dev deployment (holochain-dev.elohim.host)
    ├── edgenode-alpha.yaml  # Alpha deployment (holochain-alpha.elohim.host)
    └── nix-cache-pvc.yaml   # PVC for Nix/Cargo cache (Jiva storage)
```

## Key Technical Details

### Edge Node Container

The Edge Node uses `ghcr.io/holo-host/edgenode:latest` which runs Holochain 0.5.6. The conductor binds to `127.0.0.1:4444` by default (not configurable), so we add a **socat sidecar** to proxy external connections:

```yaml
# In deployment manifest
containers:
  - name: edgenode
    # ... conductor binds to localhost:4444
  - name: ws-proxy
    image: alpine/socat:latest
    args:
      - "TCP-LISTEN:8444,fork,reuseaddr"
      - "TCP:127.0.0.1:4444"
```

Service targets port 8444 (socat), not 4444 (conductor directly).

### Health Probes

The conductor binds to localhost, so TCP probes from outside fail. We use exec probes with bash (not sh/ash which don't support `/dev/tcp`):

```yaml
livenessProbe:
  exec:
    command:
      - bash
      - -c
      - "echo > /dev/tcp/127.0.0.1/4444"
```

### WebSocket Origin Header

Holochain conductor **requires** an `Origin` header for WebSocket connections. Without it, you get `400 Bad Request` with body "Missing `Origin` header". Browsers send this automatically; for CLI testing use:

```bash
wscat -c wss://holochain-dev.elohim.host --origin https://elohim.host
```

### DNA Build (Nix/Holonix)

The DNA is built using Nix with Holonix 0.5:

```nix
# flake.nix
inputs.holonix.url = "github:holochain/holonix?ref=main-0.5";
```

Build commands (run inside nix develop):
```bash
nix develop
cargo build --release --target wasm32-unknown-unknown
hc dna pack dna.yaml
hc app pack workdir/
```

### Holochain 0.5 API Changes

- `hash_entry` no longer accepts String directly
- DNA manifest doesn't support `origin_time` field
- Client API: `AdminWebsocket.connect({ url: new URL(...) })`

## Endpoints

| Environment | URL | Access | Purpose |
|-------------|-----|--------|---------|
| Dev | `wss://holochain-dev.elohim.host` | Internal network (IP whitelist) | Development/testing |
| Alpha | `wss://holochain-alpha.elohim.host` | Internal network (IP whitelist) | Integration testing |

**Note:** Endpoints are protected by IP whitelist - only accessible from internal cluster network, Eclipse Che workspaces, or via VPN.

## Development Workflow (Eclipse Che)

1. Edit code in Che workspace
2. Run `npm run start` in elohim-app
3. Angular app connects to `wss://holochain-dev.elohim.host` (accessible from Che internal network)
4. HolochainClientService handles admin/app WebSocket connections
5. No port-forward needed when working from Eclipse Che

## Jenkins Pipeline

The `Jenkinsfile` builds DNA with Nix and deploys Edge Node:

1. **Build DNA** - Uses `ci-builder-nix` image with Holonix
2. **Package hApp** - Creates `lamad-spike.happ`
3. **Build Edge Node Image** - Docker image with hApp baked in
4. **Push to Harbor** - `harbor.ethosengine.com/ethosengine/elohim-edgenode`
5. **Deploy** - Applies K8s manifests to ethosengine namespace

Resource limits are tuned for the operations node (Intel NUC i5, 8 cores, 16GB):
- Builder: 4Gi/2cpu request, 8Gi/4cpu limit
- Edge Node: 512Mi/250m request, 2Gi/1cpu limit

## Angular Integration

The HolochainClientService (`elohim-app/src/app/elohim/services/holochain-client.service.ts`) manages:

1. AdminWebsocket connection
2. Agent key generation
3. Signing credentials (stored in localStorage)
4. hApp installation (if not pre-installed)
5. AppWebsocket connection with auth token
6. Zome calls

Environment config (`elohim-app/src/environments/environment.ts`):
```typescript
holochain: {
  adminUrl: 'wss://holochain-dev.elohim.host',  // IP whitelist protected
  appUrl: 'wss://holochain-dev.elohim.host',
}
```

## Troubleshooting

### Cannot connect to Edge Node
1. Check you're on internal network (Eclipse Che, cluster pod, or VPN)
2. Check pod is running: `kubectl get pods -n ethosengine -l app=elohim-edgenode`
3. If external, use port-forward: `kubectl port-forward -n ethosengine deploy/elohim-edgenode-dev 4444:8444`

### 403 Forbidden
- Your IP is not in the whitelist (internal networks only)
- Connect via Eclipse Che workspace or use port-forward

### 502 Bad Gateway
- Check if socat sidecar is running: `kubectl get pods -n ethosengine`
- Verify service targets port 8444: `kubectl get endpoints -n ethosengine`

### 400 Bad Request
- Missing Origin header - browsers send automatically, CLI tools need `--origin`
- Check conductor logs: `kubectl logs -n ethosengine <pod> -c edgenode`

### Connection Refused
- Port-forward not running or wrong port
- Conductor not ready yet (check readiness probe)

### Pod CrashLooping
- Check probe configuration uses `bash` not `sh`
- Verify resource limits fit on node

## Human Onboarding Flow

The Elohim Protocol supports a progressive onboarding journey that allows users to start with zero commitment and gradually increase their sovereignty over time. Each stage builds on the previous, enabling smooth migration of identity and data.

### Stage 1: Visitor (Browser Session)
**Access**: Public web at elohim.host
**Identity**: Anonymous session, no persistence
**Capabilities**:
- Browse public "commons" content
- View community resources
- Experience the interface

**Technical**: No Holochain connection required. Content served from DNS-exposed nodes acting as public bridges.

### Stage 2: Hosted User (Custodial Keys)
**Access**: Account creation on elohim.host
**Identity**: Custodial keys managed by Edge Node
**Capabilities**:
- Full DHT participation
- Create and publish content
- Build reputation and relationships
- Access gated community features

**Technical**:
- Browser connects to Edge Node via Admin Proxy (secure)
- Keys generated and stored server-side
- Signing performed by custody service
- Data replicated across DHT

**Migration Path**: Export keys to local app when ready

### Stage 3: App User (Local Holochain, Intermittent)
**Access**: Desktop/laptop app installation
**Identity**: Self-sovereign keys on local device
**Capabilities**:
- All Stage 2 features
- Offline access to synced content
- Local AI inference (if hardware supports)
- Reduced dependency on hosted infrastructure

**Technical**:
- Local Holochain conductor on user device
- Syncs with DHT when online
- Hub-and-spoke model (like [Learning Equality](https://learningequality.org))
- Keys imported from Stage 2 or generated fresh

**Use Cases**:
- Users without always-on infrastructure
- Intermittent connectivity environments
- Privacy-conscious users who want local-first
- Communities with shared hub nodes

**Limitations**: Not always-on, so:
- Cannot serve as DHT shard holder
- Cannot be a bootstrap/signal node
- Cannot host public content reliably

### Stage 4: Node Operator (Always-On)
**Access**: Mobile device (lightweight apps) to Elohim Family Node (full infrastructure)
**Identity**: Self-sovereign keys with full network participation
**Capabilities**:
- All Stage 3 features
- DHT shard hosting
- Bootstrap/signal node capability
- Public content hosting via DNS
- Backup services for trust network
- Full local AI inference (Tier 3 hardware)

**Technical**:
- Always-on Holochain conductor
- Static IP or dynamic DNS (for public hosting)
- Sufficient storage for DHT participation
- Can run Edge Node for family/community

**Hardware Flexibility**: Stage 4 requirements depend on the hApp:
- **Lightweight** (messaging, identity): Modern smartphone sufficient
- **Medium** (content sharing, small communities): Laptop/desktop
- **Heavy** (media storage, AI inference, community hub): Tier 3 Family Node

**Infrastructure Role**:
- Provides resilience for the network
- Can host Stage 2 users (custodial)
- Backs up data for trusted relationships (family, church, etc.)
- Serves as geographic redundancy point

### Migration Between Stages

```
Stage 1 → Stage 2: Create account, generate custodial keys
Stage 2 → Stage 3: Export keys, install app, import identity
Stage 3 → Stage 4: Deploy always-on hardware, migrate conductor
```

Each transition preserves:
- Agent public key (identity)
- DHT entries (content)
- Reputation and relationships
- Source chain history

### Hub-and-Spoke Model (Stage 3)

For communities with intermittent connectivity or limited infrastructure:

```
                    ┌─────────────┐
                    │  Hub Node   │ (Stage 4 operator)
                    │ (Always-On) │
                    └──────┬──────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
      ┌────▼────┐    ┌─────▼─────┐   ┌─────▼─────┐
      │ Spoke 1 │    │  Spoke 2  │   │  Spoke 3  │
      │(Stage 3)│    │ (Stage 3) │   │ (Stage 3) │
      └─────────┘    └───────────┘   └───────────┘
```

- Hub provides always-on DHT presence
- Spokes sync when online
- Content available offline after sync
- Ideal for schools, churches, rural communities

### Security Considerations by Stage

| Stage | Key Location | Signing | Risk Level |
|-------|--------------|---------|------------|
| 1 | None | N/A | Lowest |
| 2 | Server | Server | Medium (trust required) |
| 3 | Local device | Local | Low (device security) |
| 4 | Local node | Local | Lowest (full control) |

The Admin Proxy (future) will handle Stage 2 security by:
- Whitelisting safe operations
- Requiring authentication
- Audit logging all actions
- Rate limiting abuse

## Related Files

- `elohim-app/src/app/elohim/services/holochain-client.service.ts` - Client service
- `elohim-app/src/app/elohim/models/holochain-connection.model.ts` - Types
- `elohim-app/src/environments/environment*.ts` - Endpoint configuration
