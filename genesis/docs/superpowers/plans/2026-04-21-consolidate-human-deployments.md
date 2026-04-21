# Consolidate Human Deployments to Single-Pod elohim-node Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish Option B' (extend elohim-storage's forwarder to bridge the conductor's admin port, not just app port) and migrate the 5 remaining legacy-pattern humans (matthew, jessica, pete, timothy, frank) from the 4-container `edgenode + socat + elohim-storage + happ-installer` pattern to Adam's single-container `elohim-node` pattern. At plan completion all 6 humans are consolidated, the conductor binds safely to 127.0.0.1 on every pod, and elohim-storage owns pod-network exposure with our own auth boundary.

**Architecture:**
- The forwarder already exists as a tested tokio `copy_bidirectional` TCP proxy (`elohim-storage/src/forwarder.rs`) but currently only forwards the app port (4445). We extend `NetworkConfig` with admin fields and spawn a second forwarder for port 4444. The conductor keeps Holochain's default localhost bind (no `danger_bind_addr`) — elohim-storage is the pod-network exposure layer, not Holochain directly.
- Once Adam is verified end-to-end, we extract a reusable `_edgenode-consolidated.template.yaml` (mirror of Adam's current manifest, parameterized like the legacy template) and migrate legacy humans one at a time, validating the sprint-report fingerprint collapse after each before moving to the next.
- Matthew is migrated last because he's the seeder host, carries a temporary resource bump, and a regression there blocks content seeding across the cluster.

**Tech Stack:** Rust (tokio, serde, anyhow), TOML config, Kubernetes manifests (yaml), Groovy (Jenkinsfile), Cucumber (.feature scenarios).

---

## File Structure

**Files created:**
- `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` — generalized Adam manifest for legacy-human migration
- `genesis/a2o/features/deployment/conductor-admin-reachability.feature` — regression guard scenario

**Files modified:**
- `elohim/elohim-storage/src/policy/config.rs:48-53` — add admin fields to `NetworkConfig`
- `elohim/elohim-storage/config/peer-policy.example.toml` — document new admin fields
- `elohim/elohim-storage/src/policy/evaluator.rs:64` — update default `NetworkConfig` with admin fields (keeps existing tests passing)
- `elohim/elohim-storage/src/heartbeat.rs:385` — same default update for the heartbeat test fixture
- `elohim/elohim-storage/src/main.rs:434-443` — spawn admin forwarder alongside app forwarder
- `elohim/elohim-storage/tests/forwarder_integration.rs` (new or extended) — integration test asserting both forwarders come up
- `genesis/orchestrator/manifests/humans/adam-firstman.yaml:25-34, ~260` — drop `host: "0.0.0.0"`, add peer-policy ConfigMap section and volume mount
- `genesis/orchestrator/data/deployments.json` — flip `pattern: legacy` → `consolidated` + `template` → `_edgenode-consolidated.template.yaml` for each legacy human (one per Phase D task)
- `elohim/holochain/Jenkinsfile:343-358` — simplify `computeConductorUrls` once all humans consolidated

**Files deleted (after full migration):**
- `genesis/orchestrator/manifests/humans/_edgenode-legacy.template.yaml`
- `genesis/orchestrator/manifests/humans/{matthew-manager,jessica-spouse,pete-pastor,timothy-tutor,frank-farmer}.yaml` (all were rendered from the legacy template; no longer useful)

---

## Pre-flight

- [ ] **Pre-flight Step 1: Confirm we're on dev with no in-flight work**

Run: `git status && git log --oneline -3`
Expected: `working tree clean`, and the top commit is `be1dea87 docs(skills): augment pipeline-diagnostics…` (the local-only `danger_bind_addr` commit `4d94be9c` was never pushed — it must not exist locally either, because we've rejected that approach in favor of this plan).

- [ ] **Pre-flight Step 2: Drop the local-only `danger_bind_addr` commit if present**

Run: `git log --oneline -5 | grep danger_bind_addr`

If it returns `4d94be9c fix(manifests): use danger_bind_addr…`, reset it:

```bash
git reset --hard HEAD~1
git log --oneline -3
```

Expected after reset: top commit is `be1dea87`. If the commit isn't present (it wasn't made in this worktree), skip.

- [ ] **Pre-flight Step 3: Confirm peer-policy reachability in the current Rust code**

Run: `grep -n 'expose_conductor_externally\|spawn_forwarder' elohim/elohim-storage/src/main.rs`
Expected: at least one match around line 434 showing `if policy_cfg.network.expose_conductor_externally { spawn_forwarder(...) }`. This is the wiring we'll extend.

---

## Phase A — Option B': Forwarder for the conductor admin port

### Task 1: Extend `NetworkConfig` with admin-port fields

**Files:**
- Modify: `elohim/elohim-storage/src/policy/config.rs:48-53`
- Modify: `elohim/elohim-storage/src/policy/config.rs:75-90` (test `parses_example_config`)
- Modify: `elohim/elohim-storage/src/policy/evaluator.rs:64` (default NetworkConfig)
- Modify: `elohim/elohim-storage/src/heartbeat.rs:385` (default NetworkConfig in tests)
- Modify: `elohim/elohim-storage/config/peer-policy.example.toml`

- [ ] **Step 1: Write the failing test**

Edit `elohim/elohim-storage/src/policy/config.rs` around the existing `parses_example_config` test (line 75). Add a new test below it:

```rust
#[test]
fn network_config_parses_admin_fields() {
    let toml_str = r#"
[pool]
accept_general_traffic = "auto"
min_free_storage_pct = 20
require_conductor_healthy = true

[stewardship]
accept_new_reserves = "auto"
max_storage_pct = 80

[network]
expose_conductor_externally = true
conductor_external_bind = "0.0.0.0:4445"
conductor_internal_port = 4445
conductor_admin_external_bind = "0.0.0.0:4444"
conductor_admin_internal_port = 4444
"#;
    let cfg: PolicyConfig = toml::from_str(toml_str).unwrap();
    assert!(cfg.network.expose_conductor_externally);
    assert_eq!(cfg.network.conductor_external_bind, "0.0.0.0:4445");
    assert_eq!(cfg.network.conductor_internal_port, 4445);
    assert_eq!(cfg.network.conductor_admin_external_bind, "0.0.0.0:4444");
    assert_eq!(cfg.network.conductor_admin_internal_port, 4444);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib policy::config::tests::network_config_parses_admin_fields`
Expected: compile error — `no field 'conductor_admin_external_bind' on type 'NetworkConfig'`.

- [ ] **Step 3: Extend `NetworkConfig` struct**

In `elohim/elohim-storage/src/policy/config.rs`, replace the `NetworkConfig` struct (lines 48-53):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Master switch. When true, elohim-storage spawns TCP forwarders that
    /// bridge pod-network ports to the embedded conductor's localhost ports.
    /// The conductor itself stays on 127.0.0.1 (Holochain's safe default);
    /// elohim-storage owns the pod-network exposure point and is the auth
    /// boundary for external traffic.
    pub expose_conductor_externally: bool,

    /// Bind address for the app-WS forwarder (zome calls).
    pub conductor_external_bind: String,
    /// Upstream app-WS port on 127.0.0.1 that Holochain is listening on.
    pub conductor_internal_port: u16,

    /// Bind address for the admin-WS forwarder (register, install hApp, etc).
    /// On headless k8s services this port is what doorway pods actually reach.
    pub conductor_admin_external_bind: String,
    /// Upstream admin-WS port on 127.0.0.1 that Holochain is listening on.
    pub conductor_admin_internal_port: u16,
}
```

- [ ] **Step 4: Update the default `NetworkConfig` in evaluator.rs**

In `elohim/elohim-storage/src/policy/evaluator.rs:64` (inside `PolicyConfig::default()` or the evaluator default path — confirm with `grep -n 'expose_conductor_externally: false' src/policy/evaluator.rs`), extend the literal with the new fields:

```rust
network: NetworkConfig {
    expose_conductor_externally: false,
    conductor_external_bind: "0.0.0.0:4445".to_string(),
    conductor_internal_port: 4445,
    conductor_admin_external_bind: "0.0.0.0:4444".to_string(),
    conductor_admin_internal_port: 4444,
},
```

- [ ] **Step 5: Update the default `NetworkConfig` in heartbeat.rs**

Identical change at `elohim/elohim-storage/src/heartbeat.rs:385` — the existing test constructs a `NetworkConfig` literal that now needs the two new fields added with the same defaults.

- [ ] **Step 6: Update the existing `parses_example_config` test**

In `elohim/elohim-storage/src/policy/config.rs:82-85`, extend assertions:

```rust
assert!(!cfg.network.expose_conductor_externally);
assert_eq!(cfg.network.conductor_external_bind, "0.0.0.0:4445");
assert_eq!(cfg.network.conductor_internal_port, 4445);
assert_eq!(cfg.network.conductor_admin_external_bind, "0.0.0.0:4444");
assert_eq!(cfg.network.conductor_admin_internal_port, 4444);
```

- [ ] **Step 7: Update `peer-policy.example.toml`**

Replace `elohim/elohim-storage/config/peer-policy.example.toml` entirely:

```toml
[pool]
accept_general_traffic = "auto"
min_free_storage_pct = 20
require_conductor_healthy = true

[stewardship]
accept_new_reserves = "auto"
max_storage_pct = 80

[network]
# Master switch. When true, elohim-storage spawns two TCP forwarders that
# bridge pod-network ports to the embedded conductor's localhost ports.
# Intended for k8s deployments where the embedded conductor must be reachable
# to sibling pods (doorway) on the pod network, while keeping Holochain on
# its safe localhost default. Doorway is the external auth boundary.
expose_conductor_externally = false

# App WebSocket (zome calls). Corresponds to HOLOCHAIN_APP_URL in elohim-node.
conductor_external_bind = "0.0.0.0:4445"
conductor_internal_port = 4445

# Admin WebSocket (register agents, install hApps, list cells).
# Corresponds to HOLOCHAIN_ADMIN_URL in elohim-node.
conductor_admin_external_bind = "0.0.0.0:4444"
conductor_admin_internal_port = 4444
```

- [ ] **Step 8: Run policy tests to verify all pass**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib policy::`
Expected: both `parses_example_config` and `network_config_parses_admin_fields` PASS, along with any other policy tests.

- [ ] **Step 9: Run the full lib-and-bins test suite to catch any callers of the old shape**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins`
Expected: PASS. If a test fails because some other code constructs `NetworkConfig { ... }` as a literal without the new fields, add the two new fields with the defaults from Step 4.

- [ ] **Step 10: Commit**

```bash
git add elohim/elohim-storage/src/policy/config.rs \
        elohim/elohim-storage/src/policy/evaluator.rs \
        elohim/elohim-storage/src/heartbeat.rs \
        elohim/elohim-storage/config/peer-policy.example.toml
git commit -m "feat(storage): add admin-port fields to NetworkConfig

Prepares the policy layer to drive two forwarders (app + admin) instead
of one. No runtime behavior change yet — main.rs still spawns only the
app forwarder. That wiring lands in the next commit.

Part of Option B' — keeping Holochain on 127.0.0.1 and making
elohim-storage the pod-network exposure boundary."
```

### Task 2: Spawn the admin forwarder alongside the app forwarder

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs:434-443`
- Create: `elohim/elohim-storage/tests/forwarder_integration.rs` (new integration test)

- [ ] **Step 1: Write the failing integration test**

Create `elohim/elohim-storage/tests/forwarder_integration.rs`:

```rust
//! Integration test: when expose_conductor_externally is true, main.rs
//! should spin up TWO forwarders (app and admin), each bridging a
//! pod-network bind to the corresponding 127.0.0.1 upstream.
//!
//! We can't run main.rs directly — it depends on HTTP, Holochain, etc. —
//! so this test exercises the forwarder module at the same API
//! granularity main.rs uses it.

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use elohim_storage::forwarder::spawn_forwarder;

/// Stand up two echo upstreams (one per port), spawn two forwarders
/// pointed at them, and prove each forwarder forwards traffic to its
/// own upstream (not the other's).
#[tokio::test]
async fn two_forwarders_isolate_admin_and_app_traffic() {
    // Upstream 1 — "app" echoes "A" + client bytes
    let up_app = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_app_addr = up_app.local_addr().unwrap();
    tokio::spawn(run_prefix_echo(up_app, b"A:"));

    // Upstream 2 — "admin" echoes "B" + client bytes
    let up_adm = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_adm_addr = up_adm.local_addr().unwrap();
    tokio::spawn(run_prefix_echo(up_adm, b"B:"));

    // Forwarder 1 — front the app upstream on an ephemeral port
    let app_front = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let app_front_addr = app_front.local_addr().unwrap();
    drop(app_front); // spawn_forwarder will rebind
    spawn_forwarder(&app_front_addr.to_string(), up_app_addr.port())
        .await
        .unwrap();

    // Forwarder 2 — front the admin upstream on a different ephemeral port
    let adm_front = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adm_front_addr = adm_front.local_addr().unwrap();
    drop(adm_front);
    spawn_forwarder(&adm_front_addr.to_string(), up_adm_addr.port())
        .await
        .unwrap();

    // Give accept loops a tick to settle
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Hit the app front → should see "A:hello"
    let got_app = roundtrip(app_front_addr, b"hello").await;
    assert_eq!(&got_app, b"A:hello");

    // Hit the admin front → should see "B:hello"
    let got_adm = roundtrip(adm_front_addr, b"hello").await;
    assert_eq!(&got_adm, b"B:hello");
}

async fn run_prefix_echo(listener: TcpListener, prefix: &'static [u8]) {
    while let Ok((mut sock, _)) = listener.accept().await {
        let pfx = prefix;
        tokio::spawn(async move {
            let mut buf = [0u8; 64];
            if let Ok(n) = sock.read(&mut buf).await {
                let mut out = Vec::with_capacity(pfx.len() + n);
                out.extend_from_slice(pfx);
                out.extend_from_slice(&buf[..n]);
                let _ = sock.write_all(&out).await;
            }
        });
    }
}

async fn roundtrip(addr: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let mut s = TcpStream::connect(addr).await.unwrap();
    s.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len() + 8];
    let n = s.read(&mut buf).await.unwrap();
    buf.truncate(n);
    buf
}
```

> **Note on the port dance:** `TcpListener::bind("127.0.0.1:0")` picks a free port; we drop and immediately rebind via `spawn_forwarder`. There's a tiny TOCTOU window but it's acceptable for a unit-ish integration test. If flaky, pin to explicit ports in a range (the test is single-threaded by default in cargo).

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test forwarder_integration`
Expected: test COMPILES and PASSES (the forwarder module already supports this shape — Task 2's production code is actually in main.rs, not forwarder.rs). This test is a regression guard that `spawn_forwarder` remains callable twice without interference. If this test already passes, that's fine — we're encoding the invariant. Continue.

- [ ] **Step 3: Extend main.rs to spawn the admin forwarder**

In `elohim/elohim-storage/src/main.rs`, replace the existing `if policy_cfg.network.expose_conductor_externally { … }` block (around line 434) with:

```rust
                if policy_cfg.network.expose_conductor_externally {
                    // App WS forwarder — zome calls from peers (and doorway's
                    // TypedAppClient / conductor-normalizer path).
                    if let Err(e) = elohim_storage::forwarder::spawn_forwarder(
                        &policy_cfg.network.conductor_external_bind,
                        policy_cfg.network.conductor_internal_port,
                    )
                    .await
                    {
                        warn!("conductor app forwarder failed to start: {e}");
                    }

                    // Admin WS forwarder — register agents, install hApps,
                    // list cells. Doorway calls this on /auth/register.
                    if let Err(e) = elohim_storage::forwarder::spawn_forwarder(
                        &policy_cfg.network.conductor_admin_external_bind,
                        policy_cfg.network.conductor_admin_internal_port,
                    )
                    .await
                    {
                        warn!("conductor admin forwarder failed to start: {e}");
                    }
                }
```

- [ ] **Step 4: Run the build + full test suite**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins --tests`
Expected: all PASS, including `forwarder_integration::two_forwarders_isolate_admin_and_app_traffic`.

- [ ] **Step 5: Run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib --bins --tests -- -D warnings`
Expected: no warnings that aren't pre-existing. If the new code triggers a warning (e.g., redundant clone), fix it inline.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/main.rs \
        elohim/elohim-storage/tests/forwarder_integration.rs
git commit -m "feat(storage): spawn admin-WS forwarder alongside app-WS forwarder

When expose_conductor_externally is true, main.rs now starts two
tokio TCP forwarders — one for the app port (4445), one for the admin
port (4444). The conductor stays on 127.0.0.1 for both; elohim-storage
bridges pod-network traffic in.

Unblocks the consolidated-pod migration: doorway can now reach every
human's admin WS on pod-IP:4444 without Holochain having to expose
itself via danger_bind_addr or a socat sidecar.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 3: Push Phase A and verify the elohim-edge pipeline rebuilds cleanly

**Files:**
- Read-only: check pipeline build status

- [ ] **Step 1: Push to origin**

```bash
git push origin dev
```

Expected: pre-push gate passes. If it fails, fix the underlying issue and re-push (never `--no-verify`).

- [ ] **Step 2: Poll elohim-edge for the new build**

Fetch the job index (use the `pipeline-diagnostics` skill's address book):

```
WebFetch https://jenkins.ethosengine.com/job/elohim-edge/job/dev/
  prompt: "List the last 3 builds with build number, status, and timestamp."
```

Wait for a build newer than #899 to appear and complete (status other than "In Progress"). Expected: SUCCESS or UNSTABLE. If FAILED, pull the console log and fix before moving on.

- [ ] **Step 3: Correlate the SHA**

```
WebFetch https://jenkins.ethosengine.com/job/elohim-edge/job/dev/<N>/api/json?tree=changeSets[items[commitId,msg]]
  prompt: "List commit SHAs."
```

Confirm the two commits from Tasks 1 and 2 are in the changeset. If not, wait for the next build or investigate why they skipped.

---

## Phase B — Unblock Adam with the forwarder-based exposure

### Task 4: Drop the broken `host:` field and add peer-policy ConfigMap to Adam's manifest

**Files:**
- Modify: `genesis/orchestrator/manifests/humans/adam-firstman.yaml:25-34` (drop `host:`)
- Modify: `genesis/orchestrator/manifests/humans/adam-firstman.yaml` (add peer-policy ConfigMap + volume mount to elohim-node container)

- [ ] **Step 1: Drop the broken `host:` field from Adam's conductor config**

In `genesis/orchestrator/manifests/humans/adam-firstman.yaml`, replace the admin-interface block (around lines 25-34):

```yaml
  conductor-config.yaml: |
    # Holochain Conductor Configuration for Elohim Protocol
    # Conductor binds to 127.0.0.1:4444/4445 (Holochain's safe default).
    # elohim-storage's forwarder bridges pod-network:4444/4445 → 127.0.0.1,
    # so doorway reaches admin via the pod-IP without Holochain itself
    # exposing externally. See peer-policy ConfigMap in this manifest.
    admin_interfaces:
      - driver:
          type: websocket
          port: 4444
          allowed_origins: "*"
    network:
```

(The rest of the `network:` block — `bootstrap_url`, `signal_url`, `enable_mdns`, etc. — is unchanged.)

- [ ] **Step 2: Add a peer-policy ConfigMap to Adam's manifest**

At the top of `adam-firstman.yaml`, immediately before the existing `apiVersion: v1 kind: ConfigMap` for the conductor config, insert a new ConfigMap (preceded by `---`):

```yaml
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: elohim-adam-alpha-peer-policy
  namespace: elohim-alpha
  labels:
    app: elohim-node
    elohim-human: adam-firstman
    environment: alpha
    app.kubernetes.io/component: policy
    app.kubernetes.io/part-of: elohim
    app.kubernetes.io/managed-by: jenkins
data:
  peer-policy.toml: |
    [pool]
    accept_general_traffic = "auto"
    min_free_storage_pct = 20
    require_conductor_healthy = true

    [stewardship]
    accept_new_reserves = "auto"
    max_storage_pct = 80

    [network]
    # Spawns the two localhost→pod-network forwarders for conductor
    # admin (4444) and app (4445) WebSocket ports. Doorway reaches the
    # conductor exclusively through these forwarded ports; Holochain
    # itself stays on 127.0.0.1 (its safe default). elohim-storage is
    # the auth boundary for external traffic.
    expose_conductor_externally = true
    conductor_external_bind = "0.0.0.0:4445"
    conductor_internal_port = 4445
    conductor_admin_external_bind = "0.0.0.0:4444"
    conductor_admin_internal_port = 4444
```

- [ ] **Step 3: Mount the peer-policy ConfigMap into the elohim-node container**

In the `elohim-node` container spec (around line 152), after the existing `env:` block but keeping env intact, add an env var telling elohim-storage where to find the policy file:

```yaml
            # Path to the peer-policy mount — enables conductor forwarders.
            - name: ELOHIM_STORAGE_PEER_POLICY_PATH
              value: "/etc/elohim/peer-policy.toml"
```

Then, in the same container's `volumeMounts:` block (search for `- name: happ-volume` nearby — confirm with `grep -n 'volumeMounts:' adam-firstman.yaml`), add:

```yaml
            - name: peer-policy
              mountPath: /etc/elohim/peer-policy.toml
              subPath: peer-policy.toml
              readOnly: true
```

Finally, in the Pod's top-level `volumes:` list (search for `- name: happ-volume`), add:

```yaml
        - name: peer-policy
          configMap:
            name: elohim-adam-alpha-peer-policy
```

- [ ] **Step 4: Sanity-check the YAML parses**

Run: `python3 -c "import yaml; list(yaml.safe_load_all(open('genesis/orchestrator/manifests/humans/adam-firstman.yaml')))" && echo OK`
Expected: `OK` with no parse errors. If it errors, fix indentation before proceeding.

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/manifests/humans/adam-firstman.yaml
git commit -m "fix(manifests): replace broken host: field with peer-policy forwarder for Adam

The host: "0.0.0.0" field in the conductor config was silently dropped
(Holochain 0.6 InterfaceDriver::Websocket has no such field — serde
without deny_unknown_fields), so the conductor kept binding 127.0.0.1
and admin was unreachable. Dropping the field + mounting a peer-policy
ConfigMap with expose_conductor_externally=true wires elohim-storage
to spawn admin and app forwarders on 0.0.0.0:4444/4445, which IS what
doorway pods reach via the headless service.

Replaces the danger_bind_addr approach (rejected — we don't want
Holochain itself exposing externally; auth is at the storage boundary).

Part of 2026-04-21-consolidate-human-deployments plan."
```

### Task 5: Push Phase B and verify Adam's fingerprints collapse

**Files:** read-only.

- [ ] **Step 1: Push**

```bash
git push origin dev
```

- [ ] **Step 2: Wait for the elohim-holochain pipeline to redeploy Adam**

```
WebFetch https://jenkins.ethosengine.com/job/elohim-holochain/job/dev/
  prompt: "Last 3 builds with number, status, timestamp."
```

Wait for a build newer than #1122 to succeed.

- [ ] **Step 3: Wait for elohim-genesis to run A2O against the rebuilt alpha**

```
WebFetch https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/
  prompt: "Last 3 builds with number, status, timestamp."
```

Wait for a build newer than #936 to complete.

- [ ] **Step 4: Pull the new sprint-report**

```
WebFetch https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/<N>/artifact/genesis/a2o/reports/sprint-report.md
  prompt: "Return verbatim. Preserve summary table, all fingerprints, occurrence counts."
```

Expected fingerprints to have collapsed:
- `75ca9eb6e5dc` Adam admin :4444 refused: 8 → **0**
- `aac96b4f6151` login 401 Invalid credentials: 33 → **0** (seeder routes through Adam as genesis peer, so once his admin accepts, humans register → credentials work)
- `08442ee9ab1f` No stewardship allocations: 6 → **0** (same root — seeder now populates)

If `75ca9eb6e5dc` is still non-zero, the forwarder didn't come up. Before moving on, pull the elohim-node container logs via `kubectl logs elohim-adam-alpha-0 -n elohim-alpha -c elohim-node | grep forwarder` (user needs to run — Che has no kubectl). Expected log line: `peer-status forwarder: 0.0.0.0:4444 -> 127.0.0.1:4444`.

- [ ] **Step 5: Decision gate**

If Adam's fingerprints collapsed, proceed to Phase C. If not, STOP and investigate before doing any migration work — migrating the 5 legacy humans on top of a broken forwarder would compound the problem.

---

## Phase C — Build the consolidated deployment template

### Task 6: Extract `_edgenode-consolidated.template.yaml` from Adam's manifest

**Files:**
- Create: `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml`

- [ ] **Step 1: Identify the placeholder surface of the legacy template**

Run: `grep -E 'PLACEHOLDER|_PLACEHOLDER' genesis/orchestrator/manifests/humans/_edgenode-legacy.template.yaml | sort -u`
Expected output — document each placeholder so we can reproduce them in the new template. Typical set:
- `RESOURCE_PREFIX_PLACEHOLDER`
- `NAMESPACE_PLACEHOLDER`
- `HUMAN_ID_PLACEHOLDER`
- `HUMAN_LABEL_PLACEHOLDER`
- `ENV_PLACEHOLDER`
- `INSTANCE_PLACEHOLDER`
- `DEPLOY_VERSION_PLACEHOLDER`
- `HAPP_TAG_PLACEHOLDER`
- `STORAGE_TAG_PLACEHOLDER`
- `EDGENODE_TAG_PLACEHOLDER` (legacy only — will not appear in consolidated)
- `HAPP_INSTALLER_TAG_PLACEHOLDER` (legacy only — will not appear in consolidated)
- `BOOTSTRAP_URL_PLACEHOLDER`
- `SIGNAL_URL_PLACEHOLDER`
- `P2P_BOOTSTRAP_NODES_PLACEHOLDER`
- Resource sizing placeholders (memory/cpu) — name them per human:
  - `MEMORY_REQUEST_PLACEHOLDER`
  - `MEMORY_LIMIT_PLACEHOLDER`
  - `CPU_REQUEST_PLACEHOLDER`
  - `CPU_LIMIT_PLACEHOLDER`
- `DEVICE_ARCHETYPE_PLACEHOLDER`
- `HOUSEHOLD_ID_PLACEHOLDER`
- `NODE_ROLE_PLACEHOLDER` (archival, edge, etc.)
- `REGION_PLACEHOLDER`
- `AFFINITY_NODETYPE_PLACEHOLDER` (operations/edge/remote/performance)

- [ ] **Step 2: Create the consolidated template**

Copy `genesis/orchestrator/manifests/humans/adam-firstman.yaml` to `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml`.

```bash
cp genesis/orchestrator/manifests/humans/adam-firstman.yaml \
   genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
```

- [ ] **Step 3: Replace Adam-specific literals with placeholders**

In the new file, replace:
- `elohim-adam-alpha` → `RESOURCE_PREFIX_PLACEHOLDER`
- `elohim-alpha` (namespace context) → `NAMESPACE_PLACEHOLDER`
- `adam-firstman` (label) → `HUMAN_LABEL_PLACEHOLDER`
- `human-adam-firstman` (HUMAN_ID env) → `HUMAN_ID_PLACEHOLDER`
- `alpha` (env/instance context) → `ENV_PLACEHOLDER` / `INSTANCE_PLACEHOLDER` as appropriate
- Resource sizing (`memory: "2Gi"`, etc.) → `MEMORY_REQUEST_PLACEHOLDER`, `MEMORY_LIMIT_PLACEHOLDER`, `CPU_REQUEST_PLACEHOLDER`, `CPU_LIMIT_PLACEHOLDER`
- `family-node-base` (DEVICE_ARCHETYPE env) → `DEVICE_ARCHETYPE_PLACEHOLDER`
- `household-adam` (HOUSEHOLD_ID env) → `HOUSEHOLD_ID_PLACEHOLDER`
- `archival` (NODE_ROLE env) → `NODE_ROLE_PLACEHOLDER`
- `us-central` (REGION env) → `REGION_PLACEHOLDER`
- `remote` (node affinity) → `AFFINITY_NODETYPE_PLACEHOLDER`
- Image tags `STORAGE_TAG_PLACEHOLDER`, `HAPP_TAG_PLACEHOLDER`, `DEPLOY_VERSION_PLACEHOLDER` — already placeholders in Adam, keep.

Leave the peer-policy ConfigMap inline (same shape as Adam's, with `RESOURCE_PREFIX_PLACEHOLDER-peer-policy` as its own name).

Preserve the header comment from Adam's file but rewrite the first few lines:

```yaml
# Consolidated single-container pattern — shared by all consolidated humans.
#
# Per-human values are injected by deployHumanManifest in
# elohim/holochain/Jenkinsfile, driven by
# genesis/orchestrator/data/deployments.json. This template is NOT applied
# directly; it's rendered through sed into a per-human manifest before
# kubectl apply.
#
# Architecture:
#   Doorway Deployment → ClusterIP service → forwarder (0.0.0.0:4444/4445)
#                       → conductor localhost:4444/4445
#   P2P: elohim-node peers discover each other via headless service DNS.
#   Storage, conductor, happ installer are one container; cgroup-shared
#   memory pool; no socat sidecar.
#
# Ports:
#   - 4444: Conductor Admin WebSocket (exposed via forwarder)
#   - 4445: Conductor App WebSocket (exposed via forwarder)
#   - 8090: elohim-storage HTTP API
#   - 9876: P2P libp2p transport (cross-namespace via NetworkPolicy)
```

- [ ] **Step 4: Verify the template parses as valid YAML**

Run: `python3 -c "import yaml; list(yaml.safe_load_all(open('genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml')))" && echo OK`
Expected: `OK`.

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
git commit -m "feat(manifests): add consolidated-pattern edgenode template

Generalizes Adam's manifest into a parameterized template that the
5 legacy humans can migrate into by flipping deployments.json's
pattern field. Same placeholder surface as the legacy template plus
AFFINITY_NODETYPE_PLACEHOLDER and household-scoped env vars.

Part of 2026-04-21-consolidate-human-deployments plan."
```

### Task 7: Confirm Jenkinsfile can render the consolidated template

**Files:**
- Read-only: `elohim/holochain/Jenkinsfile`

- [ ] **Step 1: Trace which code path handles `pattern: consolidated`**

Run: `grep -n 'pattern\|template\|manifest\|consolidated\|legacy\|deployHumanManifest' elohim/holochain/Jenkinsfile | head -40`
Expected: find the function (likely `deployHumanManifest`) that either reads `h.template` or `h.manifest` from the deployments.json record. The existing Adam path uses `h.manifest`; legacy path uses `h.template`. We need `h.template` to work with the consolidated template path.

- [ ] **Step 2: If `deployHumanManifest` already handles both fields, no change needed**

If the function prefers `h.template` when present and the sed-rendering path is pattern-agnostic: no action. Move to Phase D.

- [ ] **Step 3: If the function assumes legacy-only for `h.template`, generalize it**

Extend the render logic so `pattern == 'consolidated'` + `template: .../_edgenode-consolidated.template.yaml` renders the new template. Tasks 9-13 test this path live.

- [ ] **Step 4: Commit if changes were needed**

```bash
git add elohim/holochain/Jenkinsfile
git commit -m "ci(holochain): render consolidated-pattern template from deployments.json

Teaches deployHumanManifest to sed-render the new
_edgenode-consolidated.template.yaml when a human's pattern is
consolidated and template is set (matches the existing legacy path).

Pre-req for Tasks 9-13 where each legacy human flips to consolidated."
```

(Skip if no change was needed.)

---

## Phase D — Migrate legacy humans one at a time

**Ordering rationale:** Least-critical → most-critical. Jessica is lowest-resource and purely device-steward — smallest blast radius. Matthew is last because he hosts the seeder (currently carrying a temp resource bump — any regression blocks content seeding for the whole cluster). Between them, frank/pete/timothy are ordered by resource profile and trust topology: frank (remote/WireGuard, isolated), pete (remote/WireGuard, faith cluster), timothy (chromebook, stewarded-child — bumped for seeding, higher scrutiny).

**Final order:** jessica → frank → pete → timothy → matthew.

### Per-migration template (used for each of Tasks 8-12)

Each migration follows this template:

- [ ] **Step 1: Flip `pattern` + `template` in deployments.json for `<human>`**

Edit `genesis/orchestrator/data/deployments.json`. Find the record matching `"name": "<human>"`. Change:

```json
"pattern": "legacy",
"template": "genesis/orchestrator/manifests/humans/_edgenode-legacy.template.yaml",
```

to:

```json
"pattern": "consolidated",
"template": "genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml",
```

Preserve every other field verbatim (resource overrides, `$comment`, affinity, etc.).

- [ ] **Step 2: Sanity-check deployments.json still parses**

Run: `python3 -c "import json; json.load(open('genesis/orchestrator/data/deployments.json'))" && echo OK`
Expected: `OK`.

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/data/deployments.json
git commit -m "feat(deploy): migrate <human> to consolidated single-container pattern

Flips pattern: legacy → consolidated. Next elohim-holochain deploy
will render _edgenode-consolidated.template.yaml for <human>, using
their existing resource overrides from this record.

Part of 2026-04-21-consolidate-human-deployments plan."
```

- [ ] **Step 4: Push**

```bash
git push origin dev
```

- [ ] **Step 5: Wait for elohim-holochain to redeploy and elohim-genesis to run A2O**

Same WebFetch workflow as Task 5 Steps 2-4. Get the new sprint-report.

- [ ] **Step 6: Verify no register fingerprint for `<human>`**

In the sprint-report's imagodei section, grep the scenario lists inside register-failure findings for the human's deployment hostname (e.g., `elohim-jessica-alpha-0`). Expected: absent. If present, something in the migration rendered wrong — see rollback step.

- [ ] **Step 7: Verify login fingerprint for `<human>`'s test credentials is absent**

In the sprint-report, check login-401 scenario lists for features that exercise this human (e.g., jessica → `fixture-humans.feature · Core family`). Expected: this human isn't in the failing list. If they are, investigate.

- [ ] **Step 8: Rollback if broken**

If sprint-report regresses (fingerprint COUNT increases vs. last green build for scenarios naming this human), revert:

```bash
git revert --no-edit HEAD
git push origin dev
```

Investigate (typical root causes: Jenkinsfile render path didn't pick up the consolidated template; resource sizing too low for elohim-node's cooperative cgroup; P2P_BOOTSTRAP_NODES placeholder not substituted) before retrying.

---

### Task 8: Migrate jessica (lowest-risk)

- [ ] Follow the per-migration template with `<human> = jessica`. Commit message format: `feat(deploy): migrate jessica to consolidated single-container pattern`.

### Task 9: Migrate frank

- [ ] Follow the per-migration template with `<human> = frank`. Only proceed if Task 8 verified cleanly. Commit message format: `feat(deploy): migrate frank to consolidated single-container pattern`.

### Task 10: Migrate pete

- [ ] Follow the per-migration template with `<human> = pete`. Only proceed if Task 9 verified cleanly. Commit message format: `feat(deploy): migrate pete to consolidated single-container pattern`.

### Task 11: Migrate timothy

- [ ] Follow the per-migration template with `<human> = timothy`. Only proceed if Task 10 verified cleanly.

Special attention: Timothy has a `$comment` about the chromebook floor and a `$chromebookFloor` record capturing the target-low-resource config. Preserve those fields unchanged — they're not instructions, they're posterity notes for when we restore chromebook-class profiling. Commit message format: `feat(deploy): migrate timothy to consolidated single-container pattern`.

### Task 12: Migrate matthew

- [ ] Follow the per-migration template with `<human> = matthew`. Only proceed if Tasks 8-11 all verified cleanly.

Special attention: Matthew has a TEMP BUMP `$comment` for seeder capacity (2Gi/8Gi, 1000m/3000m). The consolidated template uses a single cooperative memory pool — ensure the template's `memory: "MEMORY_REQUEST_PLACEHOLDER"` / `memory: "MEMORY_LIMIT_PLACEHOLDER"` paths receive the 2Gi/8Gi values, not the default 2Gi/6Gi from Adam. If the rendering pipeline doesn't already honor per-human `edgenodeMemoryRequest/Limit` overrides into the consolidated template's fields, map them explicitly in the Jenkinsfile. Commit message format: `feat(deploy): migrate matthew to consolidated single-container pattern`.

After Task 12 completes cleanly, every human is consolidated and the legacy pipeline path is unused.

---

## Phase E — Simplify and clean up

### Task 13: Simplify `computeConductorUrls` (all humans now consolidated)

**Files:**
- Modify: `elohim/holochain/Jenkinsfile:343-358`

- [ ] **Step 1: Replace the pattern-aware conditional with a single port**

In `elohim/holochain/Jenkinsfile`, replace the current `computeConductorUrls`:

```groovy
def computeConductorUrls(String targetEnv, List allHumans) {
    def envHumans = allHumans.findAll { it.env == targetEnv }
    return envHumans.collect { h ->
        def appPort = (h.pattern == 'consolidated') ? 4445 : 8445
        "ws://${h.resourcePrefix}-0.${h.resourcePrefix}-headless.elohim-${targetEnv}.svc.cluster.local:${appPort}"
    }.join(',')
}
```

with:

```groovy
def computeConductorUrls(String targetEnv, List allHumans) {
    def envHumans = allHumans.findAll { it.env == targetEnv }
    return envHumans.collect { h ->
        // All humans run the consolidated elohim-node pattern: conductor
        // binds 127.0.0.1:4444/4445 and elohim-storage's forwarders expose
        // them on 0.0.0.0:4444/4445. Doorway derives the admin URL as
        // app-port minus 1 → 4444.
        "ws://${h.resourcePrefix}-0.${h.resourcePrefix}-headless.elohim-${targetEnv}.svc.cluster.local:4445"
    }.join(',')
}
```

- [ ] **Step 2: Commit**

```bash
git add elohim/holochain/Jenkinsfile
git commit -m "refactor(holochain): simplify computeConductorUrls — all humans consolidated

The pattern-aware branch was a bridge for the migration. With all 6
humans on the consolidated pattern (Phase D of the plan), the URL is
uniformly ws://...:4445. Doorway derives admin as :4444 and hits the
forwarder spawned by elohim-storage."
```

### Task 14: Delete the legacy template and orphaned per-human yamls

**Files:**
- Delete: `genesis/orchestrator/manifests/humans/_edgenode-legacy.template.yaml`
- Delete: `genesis/orchestrator/manifests/humans/matthew-manager.yaml`
- Delete: `genesis/orchestrator/manifests/humans/jessica-spouse.yaml`
- Delete: `genesis/orchestrator/manifests/humans/pete-pastor.yaml`
- Delete: `genesis/orchestrator/manifests/humans/timothy-tutor.yaml`
- Delete: `genesis/orchestrator/manifests/humans/frank-farmer.yaml`

- [ ] **Step 1: Verify nothing else references the legacy template**

Run: `grep -rn '_edgenode-legacy\|matthew-manager\.yaml\|jessica-spouse\.yaml\|pete-pastor\.yaml\|timothy-tutor\.yaml\|frank-farmer\.yaml' --include='*.{groovy,json,yaml,yml,md,sh,ts,rs}' . 2>/dev/null | grep -v '\.git/' | head -20`
Expected: no hits outside the files being deleted themselves (and possibly the plan + any docs referencing the migration). If a Jenkinsfile or other config still points to one of these files, update the reference before deleting.

- [ ] **Step 2: Delete the files**

```bash
git rm genesis/orchestrator/manifests/humans/_edgenode-legacy.template.yaml \
       genesis/orchestrator/manifests/humans/matthew-manager.yaml \
       genesis/orchestrator/manifests/humans/jessica-spouse.yaml \
       genesis/orchestrator/manifests/humans/pete-pastor.yaml \
       genesis/orchestrator/manifests/humans/timothy-tutor.yaml \
       genesis/orchestrator/manifests/humans/frank-farmer.yaml
```

- [ ] **Step 3: Commit**

```bash
git commit -m "chore(manifests): remove legacy edgenode template and pre-consolidation yamls

All 6 humans run the consolidated elohim-node pattern now; the legacy
4-container template and its rendered per-human yamls are unreferenced."
```

### Task 15: Add an a2o regression scenario for conductor admin reachability

**Files:**
- Create: `genesis/a2o/features/deployment/conductor-admin-reachability.feature`
- Modify: `genesis/a2o/steps/ui/navigation.steps.ts` OR create `genesis/a2o/steps/conductor-admin.steps.ts` for the new step

- [ ] **Step 1: Write the feature file**

Create `genesis/a2o/features/deployment/conductor-admin-reachability.feature`:

```gherkin
@e2e @deployment @regression @conductor-admin
Feature: Conductor admin WebSocket is reachable through elohim-storage
  As a protocol operator
  I want every human's conductor admin interface reachable from doorway
  So that /auth/register and admin operations don't regress to "Connection refused"
  (Regression guard for sprint-report fingerprints 75ca9eb6e5dc and 68407ba1be5a.)

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Registration succeeds against each alpha conductor
    When a fresh human is registered on doorway "alpha"
    Then the doorway returns 200 and a valid agent identifier
    And no conductor admin connection in the cluster reports refused

  Scenario: Doorway can list cells via admin WS
    When doorway queries its admin-backed conductor-visibility endpoint
    Then the response enumerates at least 6 conductors
    And each conductor reports app_interfaces and admin_interfaces both reachable
```

- [ ] **Step 2: Add step definitions**

If steps don't exist yet, create `genesis/a2o/steps/conductor-admin.steps.ts`:

```typescript
import { strict as assert } from 'node:assert';
import { Given, When, Then } from '@cucumber/cucumber';
import { request } from 'undici';

import { E2EWorld } from '../src/framework/world.js';

When('a fresh human is registered on doorway {string}', async function (this: E2EWorld, doorwayId: string) {
  const doorway = this.getDoorway(doorwayId);
  const unique = `e2e-admin-reach-${Date.now()}`;
  const { statusCode, body } = await request(`${doorway.url}/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      identifier: `${unique}@test.elohim.host`,
      password: 'Test2026!',
      displayName: unique,
    }),
  });
  const text = await body.text();
  (this.contentIds as Map<string, string>).set('adminReach:status', String(statusCode));
  (this.contentIds as Map<string, string>).set('adminReach:body', text);
});

Then('the doorway returns 200 and a valid agent identifier', function (this: E2EWorld) {
  const status = this.contentIds.get('adminReach:status');
  const body = this.contentIds.get('adminReach:body');
  assert.equal(status, '200', `expected 200, got ${status}: ${body}`);
  const parsed = JSON.parse(body ?? '{}') as Record<string, unknown>;
  assert.ok(
    typeof parsed.agentPubKey === 'string' && parsed.agentPubKey.length > 10,
    `expected agentPubKey in register response, got: ${body}`
  );
});

Then('no conductor admin connection in the cluster reports refused', function (this: E2EWorld) {
  const body = this.contentIds.get('adminReach:body') ?? '';
  assert.ok(
    !/connect refused|Connection refused|os error 111/i.test(body),
    `register response still mentions a refused admin socket: ${body}`
  );
});

When('doorway queries its admin-backed conductor-visibility endpoint', async function (this: E2EWorld) {
  const doorway = this.getDoorway('alpha');
  const { statusCode, body } = await request(`${doorway.url}/admin/nodes`, {
    headers: this.adminHeaders ? this.adminHeaders() : {},
  });
  const text = await body.text();
  (this.contentIds as Map<string, string>).set('adminNodes:status', String(statusCode));
  (this.contentIds as Map<string, string>).set('adminNodes:body', text);
});

Then('the response enumerates at least {int} conductors', function (this: E2EWorld, min: number) {
  const status = this.contentIds.get('adminNodes:status');
  const body = this.contentIds.get('adminNodes:body');
  assert.equal(status, '200', `expected 200, got ${status}: ${body}`);
  const parsed = JSON.parse(body ?? '{}') as { nodes?: unknown[] };
  assert.ok(
    Array.isArray(parsed.nodes) && parsed.nodes.length >= min,
    `expected >= ${min} nodes, got: ${body}`
  );
});

Then('each conductor reports app_interfaces and admin_interfaces both reachable', function (this: E2EWorld) {
  const body = this.contentIds.get('adminNodes:body') ?? '{}';
  const parsed = JSON.parse(body) as { nodes?: { admin_ok?: boolean; app_ok?: boolean }[] };
  for (const n of parsed.nodes ?? []) {
    assert.ok(n.admin_ok, `node has admin unreachable: ${JSON.stringify(n)}`);
    assert.ok(n.app_ok, `node has app unreachable: ${JSON.stringify(n)}`);
  }
});
```

> **Field-name verification:** before shipping, run `grep -n 'adminHeaders\|getDoorway' genesis/a2o/src/framework/world.ts` to confirm the helpers exist with those names. If the project calls them differently (e.g., `adminAuthHeaders`), adjust the step file before Step 3.

- [ ] **Step 3: Run the a2o harness locally to syntax-check the steps**

Run: `cd genesis/a2o && npx tsc --noEmit`
Expected: exit 0. If it fails with missing types, rename the helper calls per the verification above.

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/features/deployment/conductor-admin-reachability.feature \
        genesis/a2o/steps/conductor-admin.steps.ts
git commit -m "test(a2o): regression scenarios for conductor admin reachability

Guards against re-introducing sprint-report fingerprints 75ca9eb6e5dc
(Adam admin :4444 refused) and 68407ba1be5a (legacy-human admin refused
via old port derivation). Asserts that /auth/register succeeds without
any admin connection surfacing 'refused', and that /admin/nodes reports
every human's conductor as both app- and admin-reachable.

Part of 2026-04-21-consolidate-human-deployments plan."
```

### Task 16: Final push and acceptance check

- [ ] **Step 1: Push**

```bash
git push origin dev
```

- [ ] **Step 2: Wait for elohim-genesis to run the new regression scenarios**

Pull sprint-report from the next build following the pipeline-diagnostics skill workflow.

- [ ] **Step 3: Acceptance**

Expected state:
- `Conductor admin WebSocket is reachable…` feature is in the `passed` column, not `failed` / `pending`
- `75ca9eb6e5dc`, `68407ba1be5a`, `08442ee9ab1f`, `aac96b4f6151` are all absent from the sprint-report
- Findings total ≤ 5 (remaining should be the three `@wip` pending-step findings plus whatever orthogonal defects remain — notably the `@browser-only` without playwright, the admin/cache/warm 202, and case-sensitivity fixture lookup — all outside this plan's scope)

If the admin-reachability feature fails, the forwarder is not spawning correctly on one or more humans. Check the elohim-node container logs for the forwarder startup line (`peer-status forwarder: 0.0.0.0:4444 -> 127.0.0.1:4444`) per human. Investigate before declaring the plan complete.

---

## Self-Review

**Spec coverage:** Every user requirement (Option B' Rust fix; migrate all 5 legacy humans; "getting this right" via per-step validation) maps to a task. Phase A covers Option B'; Phase B unblocks Adam (validates the Rust fix end-to-end before any migration); Phase C builds the reusable template; Phase D migrates one human at a time with explicit rollback; Phase E cleans up stale artifacts and adds regression guards.

**Placeholder scan:** No `TODO`, `TBD`, `implement later`, `fill in`, or `similar to Task N` placeholders remain. Every code block is concrete. The per-migration template is repeated by reference in Tasks 8-12 with `<human>` substitution rules spelled out explicitly.

**Type consistency:** `NetworkConfig` field names (`conductor_admin_external_bind`, `conductor_admin_internal_port`) are used identically in config.rs (Task 1), evaluator.rs (Task 1), heartbeat.rs (Task 1), peer-policy.example.toml (Task 1), and main.rs (Task 2). The manifest ConfigMap key `peer-policy.toml` and its mount path `/etc/elohim/peer-policy.toml` plus the env var `ELOHIM_STORAGE_PEER_POLICY_PATH` (which already exists in `config.rs:99` and `main.rs:221` — verified during fact-gathering) all agree in Phase B Task 4. `spawn_forwarder(bind, upstream_port)` signature in main.rs Task 2 matches the existing `forwarder.rs:33` signature exactly.

---

## Clean rebuild execution (2026-04-21)

After the Phase D incremental migrations surfaced two latent Jenkinsfile bugs (`eaddeff1` source-file picker, `c202d2b8` agent-side log read), the Adam-admin forwarder fix (`5f5fbb58`), jessica/frank migrations, and the batched pete+timothy+matthew migration, the cluster accumulated stale state from a sequence of partially-completed deploys: matthew's storage :8090 unreachable, timothy's chromebook-tier memory (1536Mi) OOMKilling under sync load, and a Jiva iSCSI write-loss (`pvc-7f1595cd`) during the node write-saturation incident that also disrupted the `ethosengine` build host.

Rather than continue incrementally patching, we performed a full k8s-level reset of `elohim-alpha`:

- **Deleted:** 6 per-human StatefulSets; 16 PVCs (12 current + 4 shared-edgenode orphans from the pre-per-human era); 2 ancient unlabeled PVCs on microk8s-hostpath (79-day orphans); 6 peer-policy ConfigMaps; MongoDB projection-cache PVC and scaled its Deployment to 0.
- **Preserved:** `elohim-alpha` namespace; per-human Services (ClusterIP + headless); TLS secrets; `cross-namespace-p2p-networkpolicy`; doorway + elohim-site + nats deployments (doorway will reconnect to fresh MongoDB when the pipeline brings it back).
- **Reclaimed:** ~185 GiB, most of it Jiva-backed (relieves iSCSI load on the host that was under writeback saturation).

Final tweak before re-deploy: `45235287` bumps timothy's `edgenodeMemoryLimit` from 1536Mi → 3Gi (with request 512Mi → 1Gi) per observed 1.5GB RSS + 3 OOMKilled restarts in 19 minutes during sustained content-serving load. Chromebook floor (384Mi/768Mi + 150m/500m) preserved in `$chromebookFloor` as the eventual restoration target.

Pipeline trigger comes via `[build:all]` on this commit. On recreation all 16 PVCs land on `openebs-hostpath` per the Jenkinsfile:460 sed rule (matthew's Jiva PVC `pvc-7f1595cd` stays deleted; jessica's other Jiva PVC likewise). Acceptance gate:

- `75ca9eb6e5dc` Adam admin refused → 0 (third consecutive fresh-trigger confirmation — done-stable)
- `conductor-admin-reachability.feature` (Task 15, `5f35a7a9`) passing
- `aac96b4f6151` login 401 cascade — expected to collapse now that matthew's fresh pod serves :8090 cleanly and the seeder can route through him

[build:all]
