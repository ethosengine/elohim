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

| Environment | URL | Purpose |
|-------------|-----|---------|
| Dev | `wss://holochain-dev.elohim.host` | Development/testing |
| Alpha | `wss://holochain-alpha.elohim.host` | Integration testing |

## Development Workflow (Eclipse Che)

1. Edit code in Che workspace
2. Run `npm run start` in elohim-app
3. Angular app connects to `wss://holochain-dev.elohim.host` (configured in environment.ts)
4. HolochainClientService handles admin/app WebSocket connections
5. No local Holochain needed - uses shared deployed Edge Node

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
  adminUrl: 'wss://holochain-dev.elohim.host',
  appUrl: 'wss://holochain-dev.elohim.host',
}
```

## Troubleshooting

### 502 Bad Gateway
- Check if socat sidecar is running: `kubectl get pods -n ethosengine`
- Verify service targets port 8444: `kubectl get endpoints -n ethosengine`

### 400 Bad Request
- Missing Origin header - browsers send automatically, CLI tools need `--origin`
- Check conductor logs: `kubectl logs -n ethosengine <pod> -c edgenode`

### Connection Refused
- Conductor not ready yet (check readiness probe)
- Wrong port (should be 4444 via service, 8444 internally)

### Pod CrashLooping
- Check probe configuration uses `bash` not `sh`
- Verify resource limits fit on node

## Related Files

- `elohim-app/src/app/elohim/services/holochain-client.service.ts` - Client service
- `elohim-app/src/app/elohim/models/holochain-connection.model.ts` - Types
- `elohim-app/src/environments/environment*.ts` - Endpoint configuration
