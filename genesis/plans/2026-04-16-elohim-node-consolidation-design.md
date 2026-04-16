# elohim-node: Container Consolidation Design

> Consolidate the 4-container human StatefulSet (edgenode + elohim-storage + socat + happ-installer) into a single `elohim-node` container where elohim-storage manages the conductor as a child process.

## Problem

Each human agent runs 4 containers in one pod:

| Container | Purpose | Status |
|-----------|---------|--------|
| edgenode | Holochain conductor (Holo Host image) | Black-box binary, localhost-only |
| elohim-storage | Content DB, P2P mesh, import API, HTTP API | Where Elohim's value lives |
| ws-proxy (socat) | Bridges conductor localhost → pod network | Band-aid for conductor's localhost binding |
| happ-installer | Installs hApp on startup, then sleeps forever | Startup script pretending to be a service |

**Pain points:**
- **Memory isolation**: Each container has its own cgroup. If edgenode hits 4Gi while storage sits at 500Mi, k8s OOM-kills edgenode even though the pod has 3.5Gi free. Two processes on the same machine can't cooperatively share memory across cgroup boundaries.
- **Operational overhead**: 4 containers means 4 image pulls, 4 resource blocks, 4 readiness probes, boot-race coordination, and a socat bridge that exists only because the conductor binds to localhost.
- **Dependency on Holo release cadence**: Pinned to `ghcr.io/holo-host/edgenode` image tags. Conductor version is whatever Holo ships, not what Elohim needs.
- **hApp installer anti-pattern**: A long-running container that does work for ~30 seconds on startup then runs `sleep infinity`. Wastes a cgroup, memory reservation, and readiness probe.

## Constraints

- **No in-process conductor embedding**: `holochain_conductor_api` is a WebSocket client crate, not an embedding API. The `ConductorBuilder` is internal to the `holochain` binary and not exposed as a public Rust crate. `tauri-plugin-holochain` wraps this but is Tauri-specific.
- **Conductor communication stays WebSocket**: elohim-storage already connects to the conductor via `ws://localhost:4444` (admin) and `ws://localhost:4445` (app) using `holochain_client`. This boundary is well-tested and doesn't change.
- **Must support doorway**: Doorway connects to the conductor's admin/app WebSocket for browser user proxying. Doorway also needs content registration from storage (richer API, partially built).

## Design: Subprocess Management

elohim-storage becomes `elohim-node` — the single process in a single container that:
1. Spawns the `holochain` conductor binary as a managed child process
2. Waits for conductor readiness (admin WebSocket responds)
3. Installs/validates hApp (replaces happ-installer)
4. Starts its own services (HTTP API, P2P mesh, content DB)
5. Monitors conductor health, restarts if needed

```
elohim-node container (single cgroup, shared memory pool)
└── elohim-node process (Rust)
    ├── child: holochain conductor (spawned binary)
    │   ├── admin WebSocket :4444 (localhost)
    │   └── app WebSocket :4445 (localhost)
    ├── HTTP API :8090 (content, blobs, import)
    ├── libp2p P2P :9876 (peer mesh)
    ├── SQLite content DB
    └── hApp lifecycle management (install, validate, upgrade)
```

### What gets eliminated

| Component | Replacement |
|-----------|-------------|
| edgenode container | `holochain` binary bundled in elohim-node image, spawned as child process |
| socat ws-proxy | Gone. Conductor binds localhost, elohim-node connects directly. Doorway connects via pod network to elohim-node's HTTP API or bridged WebSocket port |
| happ-installer | Startup function in elohim-node: wait for conductor → check hApp → install if needed |
| 4 separate cgroups | 1 cgroup. Conductor and storage share the memory pool cooperatively |

### Conductor lifecycle

```rust
// Simplified — actual implementation will handle errors, signals, restarts
fn start_conductor(config_path: &Path, data_dir: &Path) -> Child {
    Command::new("holochain")
        .arg("--config-path").arg(config_path)
        .arg("--piped")  // don't read stdin
        .env("RUST_LOG", "warn,holochain=info")
        .spawn()
        .expect("failed to start conductor")
}

async fn wait_for_conductor(admin_url: &str, max_retries: u32) -> AdminWebsocket {
    // Same retry logic as current happ-installer, but in Rust
    for _ in 0..max_retries {
        if let Ok(ws) = AdminWebsocket::connect(admin_url).await {
            return ws;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    panic!("conductor failed to start");
}

async fn ensure_happ_installed(admin_ws: &AdminWebsocket, happ_path: &Path) {
    let apps = admin_ws.list_apps(None).await.unwrap();
    if apps.iter().any(|a| a.installed_app_id == "elohim") {
        return; // already installed
    }
    admin_ws.install_app(/* ... */).await.unwrap();
    admin_ws.enable_app("elohim").await.unwrap();
}
```

### Startup sequence

1. Write conductor config from template (currently done by k8s ConfigMap mount — stays the same)
2. Spawn `holochain` binary as child process
3. Wait for admin WebSocket on :4444 (retry loop, ~30s max)
4. Check if hApp installed; install if missing or stale
5. Enable app interface on :4445
6. Start elohim-node services (HTTP, P2P, content DB)
7. Expose readiness on :8090/health (only healthy when conductor + storage both ready)
8. Monitor conductor child process; restart on unexpected exit

### Docker image

```dockerfile
FROM rust:1.82-slim AS builder
# Build elohim-node (was elohim-storage)
COPY elohim/elohim-storage/ ./
RUN cargo build --release

FROM debian:bookworm-slim
# Install holochain binary (from nix or pre-built)
COPY --from=holochain-builder /holochain /usr/local/bin/holochain
# Install lair-keystore
COPY --from=holochain-builder /lair-keystore /usr/local/bin/lair-keystore
# Install elohim-node
COPY --from=builder /target/release/elohim-storage /usr/local/bin/elohim-node
# Bundle hApp (fetched at build time or by init container)
COPY elohim.happ /opt/holochain/elohim.happ

ENTRYPOINT ["elohim-node"]
```

### Memory model

With a single cgroup, the kernel manages memory across both processes:

| Scenario | Before (separate cgroups) | After (shared cgroup) |
|----------|--------------------------|----------------------|
| Conductor spike during sync | OOM at 4Gi even if storage at 500Mi | Shares pool, kernel balances |
| Storage spike during import | OOM at 4Gi even if conductor at 1Gi | Shares pool, kernel balances |
| Both spike simultaneously | Both hit limits independently | Both share limit, kernel decides who pages |
| Steady state | ~1.5Gi conductor + ~800Mi storage, 5.7Gi wasted headroom | ~2.3Gi total, rest available for spikes |

Pod resource spec becomes one block:
```yaml
resources:
  requests:
    memory: "2Gi"
    cpu: "500m"
  limits:
    memory: "6Gi"
    cpu: "2000m"
```

### Doorway integration

Doorway currently connects to the conductor via socat-bridged ports on the ClusterIP service. With consolidation:

**Option 1 (minimal change):** elohim-node binds conductor's WebSocket to `0.0.0.0` instead of localhost (pass `--admin-host 0.0.0.0` or configure in conductor YAML). Doorway connects directly, socat unnecessary.

**Option 2 (cleaner):** elohim-node exposes a unified API on :8090 that handles both content requests AND proxied conductor calls. Doorway talks to one endpoint. This aligns with the peer registration vision (doorway needs content inventory, not just conductor proxy).

Recommend Option 1 for initial consolidation, Option 2 as a follow-on.

### Device archetype mapping

The single-binary model maps naturally to capability levels:

| Level | Build profile | What runs |
|-------|--------------|-----------|
| 0-1 | N/A | No conductor — streams to nearest node |
| 2 | `elohim-node --light` | Conductor only, no storage/P2P (phone) |
| 3-4 | `elohim-node` | Conductor + storage + P2P (laptop, desktop) |
| 5 | `elohim-node --doorway` | Full stack + doorway registration (family node) |

These could be compile-time feature flags or runtime configuration. Runtime is more flexible for the "swappable device archetypes" vision.

## Migration path

### Phase 1: Feature-gated (parallel operation)
- Add `holochain` binary to elohim-storage Docker image
- Add `--embedded-conductor` flag (default: off)
- When enabled: spawn conductor, install hApp, skip WebSocket env vars
- When disabled: existing behavior (connect to external conductor)
- Deploy in parallel: some humans on consolidated, some on 4-container
- Compare behavior, resource usage, stability

### Phase 2: Default flip
- Make `--embedded-conductor` the default
- Remove edgenode, socat, happ-installer from manifests
- Update doorway to connect to elohim-node's exposed ports
- Rename image: `elohim-storage` → `elohim-node`

### Phase 3: Cleanup
- Remove feature gate (embedded is the only mode)
- Remove WebSocket client connection code for external conductor
- Update CI/CD: one image to build, not two to coordinate

## What this does NOT change

- **Holochain conductor behavior**: Same binary, same config, same DHT participation
- **DHT interop**: Same DNA hashes, same network. Other Holochain apps see no difference.
- **Doorway API**: Doorway still proxies conductor WebSocket for browser users
- **P2P protocol**: libp2p mesh unchanged
- **Content DB**: SQLite unchanged
- **steward/Tauri**: Desktop embedding via tauri-plugin-holochain is a separate path, unaffected

## Risks

| Risk | Mitigation |
|------|-----------|
| Conductor crash takes down storage | Subprocess monitor restarts conductor; storage stays up and serves cached content |
| Holochain binary version management | Pin version in Dockerfile; same cadence as current edgenode tag pinning |
| Larger image size | ~50MB larger (holochain + lair-keystore binaries). Negligible vs current multi-image pull |
| Conductor config changes | Same ConfigMap mount pattern; elohim-node writes config before spawning |
