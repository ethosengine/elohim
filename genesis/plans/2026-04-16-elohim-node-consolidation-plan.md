# elohim-node Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate the 4-container human StatefulSet (edgenode + elohim-storage + socat + happ-installer) into a single `elohim-node` container where elohim-storage manages the Holochain conductor as a child process.

**Architecture:** Add a conductor process manager to elohim-storage behind an `--embedded-conductor` flag. When enabled, elohim-storage spawns the `holochain` binary, waits for readiness, installs the hApp, then starts its own services. The Dockerfile bundles the holochain binary alongside elohim-storage. Feature-gated so existing 4-container deploys continue working during transition.

**Tech Stack:** Rust (tokio, holochain_client), Docker multi-stage build, k8s StatefulSet manifests, Jenkins pipeline.

**Design spec:** `genesis/plans/2026-04-16-elohim-node-consolidation-design.md`

---

### Task 1: Conductor Process Manager Module

Add a module that spawns the holochain conductor as a child process, monitors it, and restarts on unexpected exit.

**Files:**
- Create: `elohim/elohim-storage/src/conductor.rs`
- Modify: `elohim/elohim-storage/src/main.rs` (add `mod conductor`)

- [ ] **Step 1: Create the conductor module with ConductorManager struct**

```rust
// elohim/elohim-storage/src/conductor.rs

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};

use holochain_client::AdminWebsocket;

/// Manages the Holochain conductor as a child process.
pub struct ConductorManager {
    conductor_binary: PathBuf,
    config_path: PathBuf,
    data_dir: PathBuf,
    admin_port: u16,
    child: Option<Child>,
}

impl ConductorManager {
    pub fn new(
        conductor_binary: PathBuf,
        config_path: PathBuf,
        data_dir: PathBuf,
        admin_port: u16,
    ) -> Self {
        Self {
            conductor_binary,
            config_path,
            data_dir,
            admin_port,
            child: None,
        }
    }

    /// Spawn the conductor binary as a child process.
    pub async fn start(&mut self) -> anyhow::Result<()> {
        info!(
            binary = %self.conductor_binary.display(),
            config = %self.config_path.display(),
            data_dir = %self.data_dir.display(),
            admin_port = self.admin_port,
            "Starting embedded conductor"
        );

        // Ensure data directory exists
        tokio::fs::create_dir_all(&self.data_dir).await?;

        let child = Command::new(&self.conductor_binary)
            .arg("--config-path")
            .arg(&self.config_path)
            .arg("--piped")
            .env("RUST_LOG", "warn,holochain=info,kitsune2=warn")
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;

        info!(pid = child.id(), "Conductor process spawned");
        self.child = Some(child);
        Ok(())
    }

    /// Wait for the conductor admin WebSocket to become responsive.
    /// Returns the connected AdminWebsocket.
    pub async fn wait_for_ready(&self, max_retries: u32) -> anyhow::Result<AdminWebsocket> {
        let admin_url = format!("localhost:{}", self.admin_port);
        info!(url = %admin_url, max_retries, "Waiting for conductor readiness");

        for attempt in 1..=max_retries {
            match AdminWebsocket::connect(&admin_url).await {
                Ok(ws) => {
                    info!(attempt, "Conductor admin WebSocket connected");
                    return Ok(ws);
                }
                Err(e) => {
                    if attempt % 10 == 0 || attempt == max_retries {
                        warn!(attempt, max_retries, error = %e, "Conductor not ready yet");
                    }
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }

        anyhow::bail!(
            "Conductor failed to become ready after {} attempts ({}s)",
            max_retries,
            max_retries * 2
        )
    }

    /// Check if the conductor process is still running.
    pub fn is_running(&mut self) -> bool {
        match &mut self.child {
            Some(child) => child.try_wait().ok().flatten().is_none(),
            None => false,
        }
    }

    /// Gracefully stop the conductor.
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            info!("Stopping conductor process");
            child.kill().await?;
            child.wait().await?;
            info!("Conductor process stopped");
        }
        Ok(())
    }
}

impl Drop for ConductorManager {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // kill_on_drop handles this, but be explicit
            let _ = child.start_kill();
        }
    }
}
```

- [ ] **Step 2: Register the module in main.rs**

Add to the module declarations in `elohim/elohim-storage/src/main.rs` (near the other `mod` statements):

```rust
mod conductor;
```

- [ ] **Step 3: Verify it compiles**

Run:
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/conductor.rs elohim/elohim-storage/src/main.rs
git commit -m "feat(storage): add conductor process manager module"
```

---

### Task 2: hApp Lifecycle Manager

Port the hApp installation logic from `install-happ.cjs` (Node.js) to Rust. Handles first install, stale detection, and re-install.

**Files:**
- Create: `elohim/elohim-storage/src/happ_manager.rs`
- Modify: `elohim/elohim-storage/src/main.rs` (add `mod happ_manager`)

- [ ] **Step 1: Create the hApp manager module**

```rust
// elohim/elohim-storage/src/happ_manager.rs

use std::path::Path;
use tracing::{info, warn};

use holochain_client::{AdminWebsocket, InstallAppPayload, AppStatusFilter};

/// Expected DNA roles in the elohim hApp.
/// Must match the roles defined in the hApp manifest (workdir/happ.yaml).
const EXPECTED_ROLES: &[&str] = &[
    "lamad",
    "infrastructure",
    "imagodei",
    "mishpat",
    "node_registry",
];

const APP_ID: &str = "elohim";
const APP_INTERFACE_PORT: u16 = 4445;

/// Ensure the hApp is installed and healthy on the conductor.
/// Detects stale installs (missing roles, missing cells) and reinstalls.
pub async fn ensure_happ_installed(
    admin_ws: &mut AdminWebsocket,
    happ_path: &Path,
    app_id: &str,
) -> anyhow::Result<()> {
    let apps = admin_ws.list_apps(None).await?;
    let existing = apps.iter().find(|a| a.installed_app_id.as_ref() == app_id);

    match existing {
        Some(app_info) => {
            if is_stale(app_info) {
                warn!(app_id, "Detected stale hApp install — reinstalling");
                admin_ws.uninstall_app(app_id.to_string()).await?;
                install_fresh(admin_ws, happ_path, app_id).await?;
            } else {
                info!(app_id, "hApp already installed and healthy");
                // Ensure it's enabled
                let status_str = format!("{:?}", app_info.status);
                if status_str.contains("Disabled") || status_str.contains("Paused") {
                    info!(app_id, "Enabling disabled hApp");
                    admin_ws.enable_app(app_id.to_string()).await?;
                }
            }
        }
        None => {
            info!(app_id, "hApp not found — installing fresh");
            install_fresh(admin_ws, happ_path, app_id).await?;
        }
    }

    // Ensure app interface exists on the expected port
    ensure_app_interface(admin_ws).await?;

    info!(app_id, port = APP_INTERFACE_PORT, "hApp ready");
    Ok(())
}

/// Check if an existing install is stale (missing roles or cells).
fn is_stale(app_info: &holochain_client::AppInfo) -> bool {
    let cell_info = &app_info.cell_info;

    // Check all expected roles are present
    for role in EXPECTED_ROLES {
        match cell_info.get(*role) {
            None => {
                warn!(role, "Missing role in installed hApp");
                return true;
            }
            Some(cells) => {
                if cells.is_empty() {
                    warn!(role, "Role has zero provisioned cells");
                    return true;
                }
            }
        }
    }

    false
}

/// Install hApp from bundle path.
async fn install_fresh(
    admin_ws: &mut AdminWebsocket,
    happ_path: &Path,
    app_id: &str,
) -> anyhow::Result<()> {
    info!(path = %happ_path.display(), app_id, "Installing hApp from bundle");

    let happ_bundle = tokio::fs::read(happ_path).await?;

    let payload = InstallAppPayload {
        installed_app_id: Some(app_id.to_string()),
        source: holochain_client::AppBundleSource::Bundle(
            holochain_types::prelude::AppBundle::decode(&happ_bundle)?
        ),
        agent_key: None,  // conductor generates agent key
        membrane_proofs: std::collections::HashMap::new(),
        network_seed: None,
        existing_cells: std::collections::HashMap::new(),
        ignore_genesis_failure: false,
        allow_throwaway_random_agent_key: false,
    };

    admin_ws.install_app(payload).await?;
    admin_ws.enable_app(app_id.to_string()).await?;

    info!(app_id, "hApp installed and enabled");
    Ok(())
}

/// Ensure an app interface exists on the expected port.
async fn ensure_app_interface(admin_ws: &mut AdminWebsocket) -> anyhow::Result<()> {
    let interfaces = admin_ws.list_app_interfaces().await?;

    let has_interface = interfaces.iter().any(|iface| {
        iface.port == APP_INTERFACE_PORT
    });

    if !has_interface {
        info!(port = APP_INTERFACE_PORT, "Attaching app interface");
        admin_ws
            .attach_app_interface(APP_INTERFACE_PORT, Default::default(), None)
            .await?;
    }

    Ok(())
}
```

**Note:** The exact `InstallAppPayload` and `AppInfo` types may differ slightly from the `holochain_client` version in use. Adapt field names to match `holochain_client = "0.9.0-dev.5"`. The structure above matches the intent — check the actual type definitions during implementation and adjust accordingly.

- [ ] **Step 2: Add holochain_types dependency**

The `AppBundle::decode` call requires `holochain_types`. Add to `elohim/elohim-storage/Cargo.toml` in `[dependencies]`:

```toml
holochain_types = { version = "0.7.0-dev.11", default-features = false, features = ["fuzzing"] }
```

**Note:** The exact version must match what `holochain_client = "0.9.0-dev.5"` expects. Check `Cargo.lock` for the transitive version used by holochain_client, and pin to that.

- [ ] **Step 3: Register the module in main.rs**

Add to `elohim/elohim-storage/src/main.rs`:

```rust
mod happ_manager;
```

- [ ] **Step 4: Verify it compiles**

Run:
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: compiles. If `holochain_types` version conflicts, check `Cargo.lock` and adjust the version pin.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/happ_manager.rs elohim/elohim-storage/src/main.rs elohim/elohim-storage/Cargo.toml
git commit -m "feat(storage): add hApp lifecycle manager (Rust port of install-happ.cjs)"
```

---

### Task 3: CLI Args and Embedded Conductor Startup Integration

Wire the conductor process manager and hApp lifecycle into elohim-storage's main startup sequence behind an `--embedded-conductor` flag.

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs`

- [ ] **Step 1: Read the current CLI args struct**

Read `elohim/elohim-storage/src/main.rs` lines 73-178 to understand the existing `Args` struct (clap-based). Note the existing fields and how they're used.

- [ ] **Step 2: Add embedded conductor CLI args**

Add these fields to the `Args` struct in `main.rs`:

```rust
    /// Enable embedded conductor mode. When set, elohim-storage spawns and
    /// manages the holochain conductor as a child process instead of
    /// connecting to an external conductor.
    #[arg(long, env = "EMBEDDED_CONDUCTOR", default_value_t = false)]
    embedded_conductor: bool,

    /// Path to the holochain conductor binary.
    /// Only used when --embedded-conductor is set.
    #[arg(long, env = "CONDUCTOR_BINARY", default_value = "holochain")]
    conductor_binary: PathBuf,

    /// Path to the conductor configuration YAML file.
    /// Only used when --embedded-conductor is set.
    #[arg(long, env = "CONDUCTOR_CONFIG_PATH", default_value = "/etc/holochain/conductor-config.yaml")]
    conductor_config_path: PathBuf,

    /// Conductor data root directory (lair keystore, chain data).
    /// Only used when --embedded-conductor is set.
    #[arg(long, env = "CONDUCTOR_DATA_DIR", default_value = "/var/local/lib/holochain")]
    conductor_data_dir: PathBuf,

    /// Path to the hApp bundle file.
    /// Only used when --embedded-conductor is set.
    #[arg(long, env = "HAPP_PATH", default_value = "/opt/holochain/elohim.happ")]
    happ_path: PathBuf,

    /// Maximum retries waiting for conductor readiness (2s between retries).
    /// Only used when --embedded-conductor is set.
    #[arg(long, env = "CONDUCTOR_MAX_RETRIES", default_value_t = 60)]
    conductor_max_retries: u32,
```

- [ ] **Step 3: Add conductor startup to main()**

In the `main()` function, after config parsing but before HTTP server startup, add the embedded conductor logic. Find the section where the existing `HcClient` or import handler is initialized (where `HOLOCHAIN_ADMIN_URL` is used) and add this block before it:

```rust
    // --- Embedded conductor mode ---
    let _conductor_manager = if args.embedded_conductor {
        use crate::conductor::ConductorManager;
        use crate::happ_manager;

        info!("Embedded conductor mode enabled");

        let mut manager = ConductorManager::new(
            args.conductor_binary.clone(),
            args.conductor_config_path.clone(),
            args.conductor_data_dir.clone(),
            4444, // admin port — must match conductor config
        );

        // Start conductor as child process
        manager.start().await
            .expect("Failed to start embedded conductor");

        // Wait for conductor readiness
        let mut admin_ws = manager.wait_for_ready(args.conductor_max_retries).await
            .expect("Conductor failed to become ready");

        // Install/validate hApp
        happ_manager::ensure_happ_installed(
            &mut admin_ws,
            &args.happ_path,
            &args.holochain_app_id,
        ).await
            .expect("Failed to install hApp");

        info!("Embedded conductor ready, hApp installed");
        Some(manager)
    } else {
        None
    };
```

The `_conductor_manager` is held in scope so the child process lives for the duration of the program. When main() exits, `Drop` kills the conductor.

- [ ] **Step 4: Verify it compiles**

Run:
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: compiles. The embedded conductor path is only taken when `--embedded-conductor` is passed, so existing behavior is unchanged.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/main.rs
git commit -m "feat(storage): wire embedded conductor into startup sequence (feature-gated)"
```

---

### Task 4: Dockerfile — Bundle Holochain Binary

Modify the elohim-storage Dockerfile to include the holochain conductor binary and lair-keystore, so the image can run in embedded mode.

**Files:**
- Modify: `elohim/elohim-storage/Dockerfile`

- [ ] **Step 1: Read the current Dockerfile**

Read `elohim/elohim-storage/Dockerfile` to understand the multi-stage build and runtime image.

- [ ] **Step 2: Add holochain binary extraction stage**

Add a new stage at the top of the Dockerfile (before the builder stage) that extracts binaries from the edgenode image:

```dockerfile
# --- Extract holochain binaries from edgenode image ---
# This copies the conductor binary we've been running in production.
# When we own the conductor version pin, replace this with a direct build.
FROM ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom AS conductor-source
```

- [ ] **Step 3: Find the holochain binary path in edgenode**

The binary location in the edgenode image needs to be verified. Add a CI or local check:

```bash
docker run --rm --entrypoint which ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom holochain
```

Use the output path (likely `/usr/local/bin/holochain` or `/holochain`) in the COPY below. If `which` isn't available, try:

```bash
docker run --rm --entrypoint find ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom / -name "holochain" -type f 2>/dev/null
```

- [ ] **Step 4: Add COPY to runtime stage**

In the runtime stage (after `FROM debian:bookworm-slim`), add the conductor binary copy. Add after the existing `COPY --from=builder` line:

```dockerfile
# Holochain conductor binary — for embedded conductor mode.
# Extracted from the same edgenode image we've been running in production.
COPY --from=conductor-source /usr/local/bin/holochain /usr/local/bin/holochain
# Lair keystore — manages conductor's cryptographic keys
COPY --from=conductor-source /usr/local/bin/lair-keystore /usr/local/bin/lair-keystore
```

Adjust the source paths based on Step 3's findings.

- [ ] **Step 5: Add conductor data directory**

In the runtime stage, after the existing `mkdir -p /data`, add:

```dockerfile
# Conductor data directory (lair keystore, chain state)
# Separate from /data (storage) so PVCs can be independent
RUN mkdir -p /var/local/lib/holochain && chown storage:storage /var/local/lib/holochain
```

- [ ] **Step 6: Verify the image builds locally**

Run:
```bash
cd elohim/elohim-storage
docker build -t elohim-node-test .
docker run --rm elohim-node-test holochain --version
```

Expected: prints the holochain version (e.g., `holochain 0.6.0-...`).

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/Dockerfile
git commit -m "feat(storage): bundle holochain binary in Docker image for embedded mode"
```

---

### Task 5: Consolidated Human Manifest Template

Create a consolidated manifest variant that runs a single `elohim-node` container instead of 4 containers. Keep the existing manifests working — this is a new template for opt-in testing.

**Files:**
- Create: `genesis/orchestrator/manifests/humans/consolidated-template.yaml`

- [ ] **Step 1: Create the consolidated template**

Copy the structure from an existing human manifest (e.g., `matthew-manager.yaml`) but replace the 4-container pod spec with a single container. The template uses the same placeholder conventions.

Key changes from the 4-container template:
- Single container `elohim-node` replaces edgenode + socat + elohim-storage + happ-installer
- One unified resource block (cooperative memory pool)
- Conductor config still mounted from ConfigMap
- hApp still fetched by init container (oras from Harbor)
- Both holochain-data and storage-data PVCs mounted into the single container
- Conductor ports exposed directly (no socat bridge needed)

```yaml
      containers:
        # elohim-node: unified conductor + storage + P2P
        # Manages holochain conductor as embedded child process.
        # Replaces: edgenode + ws-proxy + elohim-storage + happ-installer
        - name: elohim-node
          image: harbor.ethosengine.com/ethosengine/elohim-storage:STORAGE_TAG_PLACEHOLDER
          imagePullPolicy: Always
          env:
            - name: HUMAN_ID
              value: "HUMAN_ID_PLACEHOLDER"
            - name: RUST_LOG
              value: "info,elohim_storage=debug"
            # --- Embedded conductor mode ---
            - name: EMBEDDED_CONDUCTOR
              value: "true"
            - name: CONDUCTOR_CONFIG_PATH
              value: "/etc/holochain/conductor-config.yaml"
            - name: CONDUCTOR_DATA_DIR
              value: "/var/local/lib/holochain"
            - name: HAPP_PATH
              value: "/opt/holochain/elohim.happ"
            # --- Existing storage config (unchanged) ---
            - name: HOLOCHAIN_ADMIN_URL
              value: "ws://localhost:4444"
            - name: HOLOCHAIN_APP_URL
              value: "ws://localhost:4445"
            - name: HOLOCHAIN_APP_ID
              value: "elohim"
            - name: ENABLE_IMPORT_API
              value: "true"
            - name: ENABLE_CONTENT_DB
              value: "true"
            - name: IMPORT_CHUNK_SIZE
              value: "50"
            - name: IMPORT_CHUNK_DELAY_MS
              value: "300"
            - name: ENABLE_P2P
              value: "true"
            - name: P2P_PORT
              value: "9876"
            - name: DISABLE_MDNS
              value: "true"
            - name: P2P_BOOTSTRAP_NODES
              value: "P2P_BOOTSTRAP_NODES_PLACEHOLDER"
            - name: RELAY_MODE
              value: "server"
          ports:
            # Conductor admin WebSocket (direct, no socat bridge)
            - name: admin-ws
              containerPort: 4444
              protocol: TCP
            # Conductor app WebSocket (direct, no socat bridge)
            - name: app-ws
              containerPort: 4445
              protocol: TCP
            # elohim-storage HTTP API
            - name: storage-http
              containerPort: 8090
              protocol: TCP
            # P2P libp2p transport
            - name: p2p
              containerPort: 9876
              protocol: TCP
          # Cooperative memory pool — conductor + storage share one cgroup
          resources:
            requests:
              memory: "2Gi"
              cpu: "500m"
            limits:
              memory: "6Gi"
              cpu: "2000m"
          volumeMounts:
            - name: holochain-data
              mountPath: /var/local/lib/holochain
            - name: storage-data
              mountPath: /data
            - name: conductor-config
              mountPath: /etc/holochain/conductor-config.yaml
              subPath: conductor-config.yaml
            - name: happ-volume
              mountPath: /opt/holochain
          readinessProbe:
            httpGet:
              path: /health
              port: 8090
            initialDelaySeconds: 30
            periodSeconds: 10
          livenessProbe:
            httpGet:
              path: /health
              port: 8090
            initialDelaySeconds: 60
            periodSeconds: 30
            failureThreshold: 5
```

Note: This is the container spec only. The full manifest template includes the ConfigMap, StatefulSet wrapper, init container (happ-fetcher), volumes, services, and PVCs — copy these from the existing templates unchanged.

- [ ] **Step 2: Verify YAML is valid**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('genesis/orchestrator/manifests/humans/consolidated-template.yaml'))" && echo "YAML valid"
```

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/manifests/humans/consolidated-template.yaml
git commit -m "feat(orchestrator): add consolidated single-container human manifest template"
```

---

### Task 6: Convert One Human to Consolidated (Adam — Test on Shem)

Convert adam-firstman.yaml to use the consolidated single-container template. Adam is the test subject because he's on shem (remote, isolated from the local family), has node-steward resources, and is the remote-side bootstrap anchor — if he works, the pattern is validated.

**Files:**
- Modify: `genesis/orchestrator/manifests/humans/adam-firstman.yaml`

- [ ] **Step 1: Read the current adam manifest**

Read `genesis/orchestrator/manifests/humans/adam-firstman.yaml` in full to understand the current 4-container pod spec.

- [ ] **Step 2: Replace the containers section**

Replace the `containers:` block (edgenode + ws-proxy + elohim-storage + happ-installer) with the single `elohim-node` container from Task 5. Keep:
- The ConfigMap (unchanged)
- The init container (happ-fetcher, unchanged)
- The volumes section (add conductor-config mount to single container)
- The volumeClaimTemplates (unchanged)
- The Services (update port mappings — admin-ws and app-ws now target the container directly, not socat bridge ports)

Key service port changes:
```yaml
  # Before (socat bridge):
  - name: admin-ws
    port: 4444
    targetPort: 8444    # socat bridge port
  - name: app-ws
    port: 4445
    targetPort: 8445    # socat bridge port

  # After (direct):
  - name: admin-ws
    port: 4444
    targetPort: 4444    # conductor port directly
  - name: app-ws
    port: 4445
    targetPort: 4445    # conductor port directly
```

Also update the headless service port mappings identically.

- [ ] **Step 3: Update adam's resource block**

Use the cooperative memory pool from the design:

```yaml
          resources:
            requests:
              memory: "2Gi"
              cpu: "500m"
            limits:
              memory: "6Gi"
              cpu: "2000m"
```

- [ ] **Step 4: Verify YAML is valid**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('genesis/orchestrator/manifests/humans/adam-firstman.yaml'))" && echo "YAML valid"
```

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/manifests/humans/adam-firstman.yaml
git commit -m "feat(orchestrator): convert adam to consolidated single-container (test on shem)"
```

---

### Task 7: Conductor Config — Allow External Binding

The conductor currently binds admin WebSocket to localhost only. For doorway to connect (via ClusterIP service), the conductor needs to bind to `0.0.0.0`. Update the conductor config template in adam's ConfigMap.

**Files:**
- Modify: `genesis/orchestrator/manifests/humans/adam-firstman.yaml` (ConfigMap section)

- [ ] **Step 1: Read the ConfigMap conductor config**

Read the `conductor-config.yaml` data in adam's ConfigMap. The relevant section:

```yaml
    admin_interfaces:
      - driver:
          type: websocket
          port: 4444
          allowed_origins: "*"
```

The stock conductor binds to `127.0.0.1` by default. For the consolidated container, the conductor still runs on localhost from elohim-node's perspective, but doorway needs to reach it via the pod's ClusterIP.

- [ ] **Step 2: Check if conductor config supports bind address**

The Holochain conductor config may support a `host` field in the admin interface driver. Check the Holochain documentation or conductor source. If supported:

```yaml
    admin_interfaces:
      - driver:
          type: websocket
          port: 4444
          host: "0.0.0.0"
          allowed_origins: "*"
```

If the `host` field is not supported in conductor config, the socat bridge approach remains necessary as a single lightweight process within the container (not a separate container):

```rust
// In conductor.rs, after starting the conductor:
// Spawn a lightweight TCP proxy from 0.0.0.0:8444 -> 127.0.0.1:4444
// using tokio's TcpListener + TcpStream::connect (no socat binary needed)
```

- [ ] **Step 3: Apply the config change or proxy fallback**

Update the ConfigMap in adam's manifest based on findings from Step 2.

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/manifests/humans/adam-firstman.yaml
git commit -m "feat(orchestrator): configure conductor for pod-network binding"
```

---

### Task 8: Health Endpoint — Add Conductor Status

Update the /health endpoint to report conductor status when running in embedded mode.

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`
- Modify: `elohim/elohim-storage/src/conductor.rs`

- [ ] **Step 1: Add a health check method to ConductorManager**

Add to `conductor.rs`:

```rust
    /// Check conductor health by attempting a lightweight admin call.
    pub async fn health_check(&mut self) -> bool {
        if !self.is_running() {
            return false;
        }
        let admin_url = format!("localhost:{}", self.admin_port);
        match AdminWebsocket::connect(&admin_url).await {
            Ok(ws) => {
                // Try a lightweight call
                match ws.list_apps(None).await {
                    Ok(_) => true,
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }
```

- [ ] **Step 2: Expose conductor status in health response**

In `http.rs`, find the `/health` handler. Add a `conductor` field to the response when embedded mode is active. The conductor manager needs to be shared via `Arc<Mutex<Option<ConductorManager>>>` in the app state.

At the `info` detail level, add:

```json
{
  "status": "ok",
  "conductor": {
    "mode": "embedded",
    "running": true
  },
  ...
}
```

When not in embedded mode:
```json
{
  "status": "ok",
  "conductor": {
    "mode": "external"
  },
  ...
}
```

- [ ] **Step 3: Verify it compiles**

Run:
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/conductor.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add conductor status to /health endpoint"
```

---

### Task 9: Jenkinsfile — Skip Edgenode/Installer Builds for Consolidated Humans

Update the build pipeline to handle the consolidated container. For now: keep building edgenode and happ-installer images (other humans still use them), but add a flag to skip them when all humans are consolidated.

**Files:**
- Modify: `elohim/holochain/Jenkinsfile`

- [ ] **Step 1: Read the current image build stages**

Read `elohim/holochain/Jenkinsfile` around lines 916 (storage build), 977 (edgenode build), and 1033 (happ-installer build) to understand the current flow.

- [ ] **Step 2: Add CONSOLIDATED_MODE parameter**

Add to the pipeline's `parameters` block:

```groovy
booleanParam(
    name: 'SKIP_LEGACY_IMAGES',
    defaultValue: false,
    description: 'Skip building edgenode and happ-installer images (all humans using consolidated mode)'
)
```

- [ ] **Step 3: Guard edgenode and happ-installer builds**

Wrap the edgenode build stage with:

```groovy
when {
    expression { !(params.SKIP_LEGACY_IMAGES ?: false) }
}
```

Do the same for the happ-installer build stage.

- [ ] **Step 4: Add HUMAN_ID_PLACEHOLDER substitution to deployHumanManifest**

In the `deployHumanManifest` function (line 397), add to the sedArgs list:

```groovy
"s|HUMAN_ID_PLACEHOLDER|human-${humanConfig.name}|g",
```

This is needed for the consolidated template's `HUMAN_ID` env var.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/Jenkinsfile
git commit -m "feat(pipeline): add SKIP_LEGACY_IMAGES param for consolidated container transition"
```

---

### Task 10: Integration Test — Embedded Conductor Locally

Test the embedded conductor mode end-to-end in the dev environment before deploying to shem.

**Files:**
- No new files — this is a manual verification task.

- [ ] **Step 1: Build the updated image locally**

```bash
cd elohim/elohim-storage
docker build -t elohim-node-test .
```

- [ ] **Step 2: Run with embedded conductor**

```bash
docker run --rm -it \
  -e EMBEDDED_CONDUCTOR=true \
  -e CONDUCTOR_DATA_DIR=/tmp/hc-data \
  -e HAPP_PATH=/opt/holochain/elohim.happ \
  -e ENABLE_IMPORT_API=false \
  -e ENABLE_P2P=false \
  -p 8090:8090 \
  -p 4444:4444 \
  -p 4445:4445 \
  elohim-node-test
```

Expected output:
```
INFO Starting embedded conductor
INFO Conductor process spawned pid=...
INFO Waiting for conductor readiness
INFO Conductor admin WebSocket connected attempt=N
INFO hApp not found — installing fresh
INFO hApp installed and enabled
INFO Embedded conductor ready, hApp installed
INFO HTTP server listening on 0.0.0.0:8090
```

- [ ] **Step 3: Verify health endpoint**

```bash
curl http://localhost:8090/health?detail=info | jq .
```

Expected: JSON response with `"conductor": {"mode": "embedded", "running": true}`.

- [ ] **Step 4: Verify conductor is accessible**

```bash
# If holochain_client CLI is available, or use wscat:
wscat -c ws://localhost:4444
```

Expected: WebSocket connection opens (conductor admin port).

- [ ] **Step 5: Run without embedded conductor (regression)**

```bash
docker run --rm -it \
  -e HOLOCHAIN_ADMIN_URL=ws://localhost:4444 \
  -e HOLOCHAIN_APP_URL=ws://localhost:4445 \
  -e ENABLE_IMPORT_API=false \
  -e ENABLE_P2P=false \
  -p 8090:8090 \
  elohim-node-test
```

Expected: starts normally, no conductor spawn, health endpoint shows `"conductor": {"mode": "external"}`.

- [ ] **Step 6: Document results**

If tests pass, note the working state. If issues found, fix and re-test before proceeding.

---

### Task 11: Deploy Adam to Shem and Validate

Deploy the consolidated adam to shem and verify P2P connectivity across the WireGuard boundary.

**Files:**
- No file changes — deployment validation.

- [ ] **Step 1: Delete adam's existing PVCs**

```bash
kubectl delete pvc holochain-data-elohim-adam-alpha-0 -n elohim-alpha
kubectl delete pvc storage-data-elohim-adam-alpha-0 -n elohim-alpha
```

Wait for PVC deletion to complete.

- [ ] **Step 2: Delete adam's existing pod**

```bash
kubectl delete pod elohim-adam-alpha-0 -n elohim-alpha
```

The StatefulSet controller will recreate the pod with the new manifest.

- [ ] **Step 3: Watch pod startup**

```bash
kubectl logs -f elohim-adam-alpha-0 -n elohim-alpha -c elohim-node
```

Expected: conductor spawns, hApp installs, storage starts, health goes ready.

- [ ] **Step 4: Verify health**

```bash
kubectl exec elohim-adam-alpha-0 -n elohim-alpha -c elohim-node -- curl -s http://localhost:8090/health?detail=info | jq .
```

Expected: `"conductor": {"mode": "embedded", "running": true}`.

- [ ] **Step 5: Verify P2P connectivity to local family**

Check that adam (on shem, over WireGuard) can sync with matthew (on performance, local LAN):

```bash
kubectl logs elohim-adam-alpha-0 -n elohim-alpha -c elohim-node | grep -i "peer\|connect\|sync\|bootstrap"
```

Expected: bootstrap connection to matthew's P2P endpoint, peer discovery, sync initiation.

- [ ] **Step 6: Verify doorway can reach adam's conductor**

```bash
kubectl exec deploy/elohim-doorway-alpha -n elohim-alpha -- curl -s http://elohim-adam-alpha-0.elohim-adam-alpha-headless.elohim-alpha:4444/
```

Or check doorway logs for adam's conductor connection status.

- [ ] **Step 7: Commit deployment validation notes**

If deployment succeeds, no code changes needed. If issues found, fix manifests and recommit.

---

### Task 12: Roll Out to Remaining Humans

Once adam is validated on shem, convert the remaining 5 human manifests to consolidated single-container mode.

**Files:**
- Modify: `genesis/orchestrator/manifests/humans/matthew-manager.yaml`
- Modify: `genesis/orchestrator/manifests/humans/jessica-spouse.yaml`
- Modify: `genesis/orchestrator/manifests/humans/terrance-tutor.yaml`
- Modify: `genesis/orchestrator/manifests/humans/frank-farmer.yaml`
- Modify: `genesis/orchestrator/manifests/humans/pete-pastor.yaml`

- [ ] **Step 1: Convert each manifest**

Apply the same transformation as Task 6 to each manifest:
1. Replace 4-container pod spec with single `elohim-node` container
2. Update service targetPorts (admin-ws: 4444→4444, app-ws: 4445→4445)
3. Set resource limits appropriate to the device archetype:

| Human | Memory request/limit | CPU request/limit | Rationale |
|-------|---------------------|-------------------|-----------|
| matthew | 2Gi / 6Gi | 500m / 2000m | Bootstrap anchor + doorway host |
| jessica | 1Gi / 3Gi | 250m / 1000m | Typical device steward |
| terrance | 512Mi / 1.5Gi | 150m / 500m | Chromebook profile |
| frank | 1Gi / 3Gi | 250m / 1000m | Remote peer |
| pete | 1Gi / 3Gi | 250m / 1000m | Remote peer |

2. Keep the `EMBEDDED_CONDUCTOR=true` env var
3. Keep the conductor config ConfigMap unchanged
4. Keep the init container (happ-fetcher) unchanged

- [ ] **Step 2: Validate YAML**

```bash
for f in genesis/orchestrator/manifests/humans/*-*.yaml; do
  python3 -c "import yaml; yaml.safe_load(open('$f'))" && echo "$f: OK" || echo "$f: FAIL"
done
```

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/manifests/humans/
git commit -m "feat(orchestrator): convert all humans to consolidated single-container mode"
```

- [ ] **Step 4: Push and deploy**

```bash
git push
```

Pipeline deploys all humans with consolidated containers. Delete PVCs for each human that needs to reschedule (frank, pete, jessica, terrance — matthew may need careful handling as bootstrap anchor).

---

### Task 13: Cleanup — Remove Legacy Artifacts

Once all humans are running consolidated and stable, remove the artifacts that are no longer needed.

**Files:**
- Delete: `elohim/holochain/edgenode/scripts/install-happ.cjs`
- Delete: `elohim/holochain/edgenode/scripts/Dockerfile` (happ-installer image)
- Modify: `elohim/holochain/Jenkinsfile` (set SKIP_LEGACY_IMAGES default to true)
- Delete: `genesis/orchestrator/manifests/humans/consolidated-template.yaml` (merged into actual manifests)

- [ ] **Step 1: Set SKIP_LEGACY_IMAGES default to true**

In `elohim/holochain/Jenkinsfile`, change:

```groovy
booleanParam(
    name: 'SKIP_LEGACY_IMAGES',
    defaultValue: true,
    description: 'Skip building edgenode and happ-installer images (all humans using consolidated mode)'
)
```

- [ ] **Step 2: Remove happ-installer artifacts**

```bash
rm elohim/holochain/edgenode/scripts/install-happ.cjs
rm elohim/holochain/edgenode/scripts/Dockerfile
rm genesis/orchestrator/manifests/humans/consolidated-template.yaml
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: remove legacy edgenode/happ-installer artifacts (consolidated mode is default)"
```

- [ ] **Step 4: Push**

```bash
git push
```
