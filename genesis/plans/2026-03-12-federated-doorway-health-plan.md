# Federated Doorway Health Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Public status page per doorway showing self-reported health, DHT peer attestations, and shefa compute metrics — federated peers keep each other honest.

**Architecture:** Add `HealthAttestation` entry type to infrastructure DNA. Doorways probe peers every 5 minutes via existing `/health` endpoint and publish attestations to DHT. Status page at `/status` serves server-rendered HTML combining self-heartbeats, peer attestations, and shefa compute. Operators see expanded view via JWT cookie.

**Tech Stack:** Holochain HDK (integrity + coordinator zomes), Rust (doorway-service with Askama templates), existing reqwest HTTP client, existing ZomeCaller infrastructure.

---

### Task 1: HealthAttestation Entry Type (Integrity Zome)

Add the new entry type, link type, and validation to the infrastructure DNA integrity zome.

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs`

**Step 1: Add HealthAttestation struct**

Add after the `DoorwayHeartbeatSummary` struct (after line ~130):

```rust
/// A peer doorway's observation of another doorway's health.
/// Published to DHT so all nodes see cross-validated health.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthAttestation {
    /// Doorway ID of the observer (must match author's DoorwayRegistration)
    pub attestor_doorway_id: String,
    /// Doorway ID of the subject being observed
    pub subject_doorway_id: String,
    /// Observed status: "online", "degraded", "unreachable"
    pub observed_status: String,
    /// Response time in milliseconds (None if unreachable)
    pub response_time_ms: Option<u32>,
    /// Whether the subject's conductor pool was healthy (from /health response)
    pub conductor_healthy: Option<bool>,
    /// When the probe happened (ISO 8601)
    pub timestamp: String,
}
```

**Step 2: Add to EntryTypes enum**

In the `EntryTypes` enum (line ~238), add variant:

```rust
    HealthAttestation(HealthAttestation),
```

**Step 3: Add DoorwayToAttestation link type**

In the `LinkTypes` enum (line ~252), add after `DoorwayToHeartbeat`:

```rust
    /// DoorwayRegistration (subject) -> HealthAttestation (peer observations about this doorway)
    DoorwayToAttestation,
```

**Step 4: Add validation function**

Add after `validate_doorway_heartbeat` (after line ~407):

```rust
const ATTESTATION_STATUSES: &[&str] = &["online", "degraded", "unreachable"];

fn validate_health_attestation(attestation: &HealthAttestation) -> ExternResult<ValidateCallbackResult> {
    if attestation.attestor_doorway_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "attestor_doorway_id cannot be empty".to_string(),
        ));
    }

    if attestation.subject_doorway_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "subject_doorway_id cannot be empty".to_string(),
        ));
    }

    if attestation.attestor_doorway_id == attestation.subject_doorway_id {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot attest about yourself".to_string(),
        ));
    }

    if !ATTESTATION_STATUSES.contains(&attestation.observed_status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!(
                "Invalid observed_status '{}'. Must be one of: {:?}",
                attestation.observed_status, ATTESTATION_STATUSES
            ),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
```

**Step 5: Wire validation into validate() function**

In the `validate()` function's `FlatOp::StoreEntry` match (line ~293), add:

```rust
EntryTypes::HealthAttestation(attestation) => validate_health_attestation(&attestation),
```

**Step 6: Build and test**

```bash
cd elohim/holochain/dna/infrastructure
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test
```

Expected: Compiles clean, all existing tests pass.

**Step 7: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs
git commit -m "feat(infrastructure): add HealthAttestation entry type with validation"
```

---

### Task 2: HealthAttestation Coordinator Functions

Add zome functions to record and query attestations, plus post-commit signal emission.

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`

**Step 1: Add input/output structs**

Add after the existing `RecordSummaryInput` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordHealthAttestationInput {
    pub attestor_doorway_id: String,
    pub subject_doorway_id: String,
    pub observed_status: String,
    pub response_time_ms: Option<u32>,
    pub conductor_healthy: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAttestationOutput {
    pub action_hash: String,
    pub entry_hash: String,
    pub attestation: HealthAttestation,
    pub author: String,
}
```

**Step 2: Add HealthAttestationCommitted signal variant**

In the `InfrastructureSignal` enum, add:

```rust
    /// HealthAttestation was recorded (peer observed another doorway)
    HealthAttestationCommitted {
        action_hash: String,
        entry_hash: String,
        attestation: HealthAttestation,
        author: String,
    },
```

**Step 3: Add record_health_attestation function**

Follow the `record_heartbeat` pattern exactly:

```rust
#[hdk_extern]
pub fn record_health_attestation(input: RecordHealthAttestationInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Verify attestor is a registered doorway operator
    let attestor_doorway = get_doorway_by_id(input.attestor_doorway_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Attestor doorway '{}' not found", input.attestor_doorway_id)
        )))?;

    if attestor_doorway.doorway.operator_agent != agent_info.agent_initial_pubkey.to_string() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the doorway operator can record attestations".to_string()
        )));
    }

    // Verify subject doorway exists
    let subject_doorway = get_doorway_by_id(input.subject_doorway_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Subject doorway '{}' not found", input.subject_doorway_id)
        )))?;

    let attestation = HealthAttestation {
        attestor_doorway_id: input.attestor_doorway_id,
        subject_doorway_id: input.subject_doorway_id,
        observed_status: input.observed_status,
        response_time_ms: input.response_time_ms,
        conductor_healthy: input.conductor_healthy,
        timestamp,
    };

    let action_hash = create_entry(&EntryTypes::HealthAttestation(attestation))?;

    // Link from subject doorway to attestation (so we can query "what do peers say about X?")
    create_link(
        subject_doorway.action_hash,
        action_hash.clone(),
        LinkTypes::DoorwayToAttestation,
        (),
    )?;

    Ok(action_hash)
}
```

**Step 4: Add get_doorway_attestations query function**

```rust
#[hdk_extern]
pub fn get_doorway_attestations(doorway_id: String) -> ExternResult<Vec<HealthAttestationOutput>> {
    let doorway = get_doorway_by_id(doorway_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Doorway '{}' not found", doorway_id)
        )))?;

    let links = get_links(
        GetLinksInputBuilder::try_new(doorway.action_hash, LinkTypes::DoorwayToAttestation)?
            .build(),
    )?;

    let mut attestations = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(attestation) = record
                    .entry()
                    .to_app_option::<HealthAttestation>()
                    .ok()
                    .flatten()
                {
                    let entry_hash = record
                        .action()
                        .entry_hash()
                        .map(|h| h.to_string())
                        .unwrap_or_default();
                    let author = record.action().author().to_string();

                    attestations.push(HealthAttestationOutput {
                        action_hash: action_hash.to_string(),
                        entry_hash,
                        attestation,
                        author,
                    });
                }
            }
        }
    }

    Ok(attestations)
}
```

**Step 5: Add signal emission in post_commit**

In the `post_commit` function, add after the `DoorwayHeartbeatSummary` case:

```rust
} else if let Some(attestation) = record
    .entry()
    .to_app_option::<HealthAttestation>()
    .ok()
    .flatten()
{
    emit_signal(InfrastructureSignal::HealthAttestationCommitted {
        action_hash: action_hash.to_string(),
        entry_hash: entry_hash.to_string(),
        attestation,
        author: author.to_string(),
    })?;
```

**Step 6: Build and test**

```bash
cd elohim/holochain/dna/infrastructure
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test
```

Expected: Compiles clean, all existing tests pass.

**Step 7: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs
git commit -m "feat(infrastructure): add record/query coordinator functions for HealthAttestation"
```

---

### Task 3: Peer Probe Loop in Doorway Service

Extend the existing heartbeat task to probe federation peers every 5th cycle and publish attestations to the DHT.

**Files:**
- Modify: `doorway/doorway-service/src/services/federation.rs`

**Context:** The `spawn_heartbeat_task` function (line ~256) already loops every 60s, gathers metrics from AppState, and calls `record_heartbeat` via ZomeCaller. We add a probe counter so every 5th iteration (~5 minutes) it also probes peers.

**Step 1: Add probe counter and peer probing logic**

Inside `spawn_heartbeat_task`, before the loop:

```rust
let mut probe_counter: u32 = 0;
let probe_interval: u32 = 5; // Every 5th heartbeat (~5 minutes)
let http_client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()
    .unwrap_or_default();
```

After the existing heartbeat zome call succeeds, add:

```rust
// Probe federation peers every 5th heartbeat
probe_counter += 1;
if probe_counter >= probe_interval {
    probe_counter = 0;

    let peer_urls = get_peer_urls(&state.peer_url_list).await;
    for peer_url in &peer_urls {
        let probe_start = std::time::Instant::now();
        let health_url = format!("{}/health", peer_url.trim_end_matches('/'));

        let (observed_status, response_time_ms, conductor_healthy) =
            match http_client.get(&health_url).send().await {
                Ok(resp) => {
                    let elapsed = probe_start.elapsed().as_millis() as u32;
                    if resp.status().is_success() {
                        // Parse health response to check conductor
                        let conductor_ok = resp
                            .json::<serde_json::Value>()
                            .await
                            .ok()
                            .and_then(|v| v.get("conductor")?.get("connected")?.as_bool());
                        let status = if conductor_ok == Some(true) {
                            "online"
                        } else {
                            "degraded"
                        };
                        (status.to_string(), Some(elapsed), conductor_ok)
                    } else {
                        ("degraded".to_string(), Some(elapsed), None)
                    }
                }
                Err(_) => ("unreachable".to_string(), None, None),
            };

        // Resolve peer's doorway_id from peer cache
        let peer_id = {
            let cache = state.peer_cache.read().await;
            cache
                .iter()
                .find(|p| p.url.trim_end_matches('/') == peer_url.trim_end_matches('/'))
                .map(|p| p.id.clone())
        };

        if let Some(subject_id) = peer_id {
            let attestation_input = RecordHealthAttestationInput {
                attestor_doorway_id: config.doorway_id.clone(),
                subject_doorway_id: subject_id.clone(),
                observed_status: observed_status.clone(),
                response_time_ms,
                conductor_healthy,
            };

            match rmp_serde::to_vec(&attestation_input) {
                Ok(payload) => {
                    match zome_caller
                        .call_zome(
                            &config.infrastructure_role,
                            &config.zome_name,
                            "record_health_attestation",
                            payload,
                        )
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                peer = %subject_id,
                                status = %observed_status,
                                "Health attestation recorded"
                            );
                        }
                        Err(e) => {
                            warn!(
                                peer = %subject_id,
                                error = %e,
                                "Failed to record health attestation"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to serialize attestation input");
                }
            }
        } else {
            debug!(
                peer_url = %peer_url,
                "Skipping attestation — peer not in cache (no doorway_id)"
            );
        }
    }
}
```

**Step 2: Add the input struct import/definition**

At the top of federation.rs, add (or define locally):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecordHealthAttestationInput {
    attestor_doorway_id: String,
    subject_doorway_id: String,
    observed_status: String,
    response_time_ms: Option<u32>,
    conductor_healthy: Option<bool>,
}
```

**Step 3: Build and test**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build --release
RUSTFLAGS="" cargo test --lib --bins
RUSTFLAGS="" cargo clippy -- -D warnings
```

Expected: Compiles clean, existing 331+ tests pass, no clippy warnings.

**Step 4: Commit**

```bash
git add doorway/doorway-service/src/services/federation.rs
git commit -m "feat(doorway): probe federation peers every 5 minutes and publish DHT attestations"
```

---

### Task 4: Status JSON Endpoint with Federation Health

Extend the existing `/status` JSON response to include federation peer health from DHT attestations, and rename the current endpoint to `/status.json`.

**Files:**
- Modify: `doorway/doorway-service/src/routes/status.rs`
- Modify: `doorway/doorway-service/src/server/http.rs`

**Step 1: Add federation types to status.rs**

Add after the existing `Diagnostics` struct:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FederationHealthStats {
    pub enabled: bool,
    pub self_id: Option<String>,
    pub self_url: Option<String>,
    pub self_tier: Option<String>,
    pub self_uptime_7d: Option<f32>,
    pub peer_count: usize,
    pub peers: Vec<PeerHealthSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerHealthSummary {
    pub doorway_id: String,
    pub url: String,
    pub self_reported_status: Option<String>,
    pub peer_attestations: Vec<AttestationSummary>,
    pub peers_agree: String, // "3/4"
    pub consensus_status: String, // "online", "degraded", "unreachable", "unknown"
    pub uptime_7d: Option<f32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttestationSummary {
    pub attestor_id: String,
    pub observed_status: String,
    pub response_time_ms: Option<u32>,
    pub timestamp: String,
}
```

**Step 2: Add federation field to StatusResponse**

```rust
pub federation: FederationHealthStats,
```

**Step 3: Build federation stats in status_check()**

Before constructing StatusResponse, add:

```rust
// Gather federation health from peer cache + DHT attestations
let federation = {
    let peer_cache = state.peer_cache.read().await;
    let peer_count = peer_cache.len();
    let self_id = state.args.node_id.clone();
    let self_url = state.args.doorway_url.clone();

    let mut peers = Vec::new();
    for peer in peer_cache.iter() {
        // Query attestations about this peer from DHT (if zome_caller available)
        let attestations = if let Some(ref zome) = state.zome_caller {
            match rmp_serde::to_vec(&peer.id) {
                Ok(payload) => {
                    match zome
                        .call_zome("infrastructure", "infrastructure", "get_doorway_attestations", payload)
                        .await
                    {
                        Ok(result) => rmp_serde::from_slice::<Vec<HealthAttestationOutput>>(&result)
                            .unwrap_or_default(),
                        Err(_) => vec![],
                    }
                }
                Err(_) => vec![],
            }
        } else {
            vec![]
        };

        let total_attestors = attestations.len();
        let agreeing = attestations
            .iter()
            .filter(|a| a.attestation.observed_status == "online")
            .count();
        let peers_agree = format!("{}/{}", agreeing, total_attestors);
        let consensus_status = if total_attestors == 0 {
            "unknown".to_string()
        } else if agreeing as f64 / total_attestors as f64 > 0.5 {
            "online".to_string()
        } else {
            attestations
                .first()
                .map(|a| a.attestation.observed_status.clone())
                .unwrap_or_else(|| "unknown".to_string())
        };

        let attestation_summaries: Vec<AttestationSummary> = attestations
            .iter()
            .map(|a| AttestationSummary {
                attestor_id: a.attestation.attestor_doorway_id.clone(),
                observed_status: a.attestation.observed_status.clone(),
                response_time_ms: a.attestation.response_time_ms,
                timestamp: a.attestation.timestamp.clone(),
            })
            .collect();

        peers.push(PeerHealthSummary {
            doorway_id: peer.id.clone(),
            url: peer.url.clone(),
            self_reported_status: Some(peer.status.clone().unwrap_or_else(|| "unknown".to_string())),
            peer_attestations: attestation_summaries,
            peers_agree,
            consensus_status,
            uptime_7d: None, // TODO: compute from DoorwayHeartbeatSummary
        });
    }

    FederationHealthStats {
        enabled: !peer_cache.is_empty(),
        self_id: Some(self_id),
        self_url,
        self_tier: None, // TODO: from DHT registration
        self_uptime_7d: None, // TODO: from summaries
        peer_count,
        peers,
    }
};
```

**Step 4: Route /status.json and /status**

In `http.rs`, update the match arm:

```rust
(Method::GET, "/status.json") => to_boxed(routes::status_check(Arc::clone(&state)).await),
(Method::GET, "/status") => to_boxed(routes::status_page(Arc::clone(&state)).await),
```

The `status_page` handler will be added in Task 5.

**Step 5: Build and test**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build --release
RUSTFLAGS="" cargo test --lib --bins
```

**Step 6: Commit**

```bash
git add doorway/doorway-service/src/routes/status.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): add federation health stats to /status.json endpoint"
```

---

### Task 5: Server-Rendered Status Page

Add Askama HTML template and handler for the public `/status` page.

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml` (add askama dependency)
- Create: `doorway/doorway-service/templates/status.html`
- Modify: `doorway/doorway-service/src/routes/status.rs` (add status_page handler)

**Step 1: Add askama dependency**

In `Cargo.toml`, add:

```toml
askama = "0.12"
```

**Step 2: Create the HTML template**

Create `doorway/doorway-service/templates/status.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{{ self_id }} — Doorway Status</title>
  <style>
    :root {
      --bg: #0a0a0b;
      --surface: #141416;
      --border: #23232a;
      --text: #e4e4e7;
      --text-muted: #71717a;
      --green: #22c55e;
      --yellow: #eab308;
      --red: #ef4444;
      --blue: #3b82f6;
    }
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
      background: var(--bg);
      color: var(--text);
      line-height: 1.6;
      padding: 2rem;
      max-width: 720px;
      margin: 0 auto;
    }
    h1 { font-size: 1.5rem; font-weight: 600; }
    h2 { font-size: 1.1rem; font-weight: 500; margin-top: 2rem; margin-bottom: 0.75rem; color: var(--text-muted); }
    .header { display: flex; align-items: baseline; gap: 1rem; margin-bottom: 0.25rem; }
    .status-badge {
      display: inline-block;
      padding: 0.15rem 0.6rem;
      border-radius: 9999px;
      font-size: 0.8rem;
      font-weight: 500;
    }
    .status-online { background: rgba(34,197,94,0.15); color: var(--green); }
    .status-degraded { background: rgba(234,179,8,0.15); color: var(--yellow); }
    .status-unreachable, .status-offline { background: rgba(239,68,68,0.15); color: var(--red); }
    .status-unknown { background: rgba(113,113,122,0.15); color: var(--text-muted); }
    .meta { color: var(--text-muted); font-size: 0.85rem; margin-bottom: 2rem; }

    .uptime-bar { display: flex; gap: 2px; margin: 1rem 0; }
    .uptime-segment {
      flex: 1;
      height: 32px;
      border-radius: 3px;
      min-width: 3px;
    }
    .seg-up { background: var(--green); }
    .seg-degraded { background: var(--yellow); }
    .seg-down { background: var(--red); }
    .seg-nodata { background: var(--border); }
    .uptime-labels { display: flex; justify-content: space-between; font-size: 0.75rem; color: var(--text-muted); }

    .stats-row { display: flex; gap: 2rem; flex-wrap: wrap; margin: 1rem 0; }
    .stat { display: flex; flex-direction: column; }
    .stat-value { font-size: 1.4rem; font-weight: 600; }
    .stat-label { font-size: 0.75rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }

    .card {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 1rem;
      margin-bottom: 0.5rem;
    }
    .component-row {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0.5rem 0;
      border-bottom: 1px solid var(--border);
    }
    .component-row:last-child { border-bottom: none; }
    .dot {
      display: inline-block;
      width: 8px;
      height: 8px;
      border-radius: 50%;
      margin-right: 0.5rem;
    }
    .dot-green { background: var(--green); }
    .dot-yellow { background: var(--yellow); }
    .dot-red { background: var(--red); }
    .dot-gray { background: var(--text-muted); }

    .peer-card {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 1rem;
      margin-bottom: 0.5rem;
    }
    .peer-header { display: flex; justify-content: space-between; align-items: center; }
    .peer-name { font-weight: 500; }
    .peer-detail { font-size: 0.8rem; color: var(--text-muted); margin-top: 0.25rem; }
    .peer-agree { font-size: 0.8rem; }
    .agree-good { color: var(--green); }
    .agree-warn { color: var(--yellow); }
    .agree-bad { color: var(--red); }

    .shefa-card {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 1rem;
    }
    .shefa-row { display: flex; justify-content: space-between; padding: 0.3rem 0; font-size: 0.9rem; }
    .shefa-label { color: var(--text-muted); }

    .operator-section {
      margin-top: 2rem;
      padding-top: 1.5rem;
      border-top: 2px solid var(--border);
    }
    .operator-badge {
      display: inline-block;
      padding: 0.1rem 0.5rem;
      border-radius: 4px;
      font-size: 0.7rem;
      font-weight: 600;
      background: rgba(59,130,246,0.15);
      color: var(--blue);
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    footer {
      margin-top: 3rem;
      padding-top: 1rem;
      border-top: 1px solid var(--border);
      font-size: 0.75rem;
      color: var(--text-muted);
      text-align: center;
    }
    footer a { color: var(--blue); text-decoration: none; }
  </style>
</head>
<body>
  <div class="header">
    <h1>{{ self_id }}</h1>
    <span class="status-badge status-{{ overall_status }}">{{ overall_status_label }}</span>
  </div>
  <div class="meta">
    {% if let Some(ref tier) = self_tier %}{{ tier }} tier · {% endif %}
    {% if let Some(uptime) = self_uptime_7d %}{{ uptime }}% uptime (7d) · {% endif %}
    {{ version }}
  </div>

  <!-- 7-Day Uptime Bar -->
  <h2>Uptime</h2>
  <div class="uptime-bar">
    {% for seg in uptime_segments %}
    <div class="uptime-segment seg-{{ seg }}" title="{{ seg }}"></div>
    {% endfor %}
  </div>
  <div class="uptime-labels">
    <span>7 days ago</span>
    <span>Now</span>
  </div>

  <!-- Response Time & Peer Agreement -->
  <div class="stats-row">
    {% if let Some(p50) = response_p50 %}
    <div class="stat">
      <span class="stat-value">{{ p50 }}ms</span>
      <span class="stat-label">Response p50</span>
    </div>
    {% endif %}
    {% if let Some(p95) = response_p95 %}
    <div class="stat">
      <span class="stat-value">{{ p95 }}ms</span>
      <span class="stat-label">Response p95</span>
    </div>
    {% endif %}
    <div class="stat">
      <span class="stat-value">{{ peers_agree_summary }}</span>
      <span class="stat-label">Peers Agree</span>
    </div>
  </div>

  <!-- Components -->
  <h2>Components</h2>
  <div class="card">
    {% for comp in components %}
    <div class="component-row">
      <span><span class="dot dot-{{ comp.color }}"></span>{{ comp.name }}</span>
      <span style="font-size:0.85rem;color:var(--text-muted)">{{ comp.detail }}</span>
    </div>
    {% endfor %}
  </div>

  <!-- Federation Peers -->
  {% if !peers.is_empty() %}
  <h2>Federation Peers</h2>
  {% for peer in &peers %}
  <div class="peer-card">
    <div class="peer-header">
      <span class="peer-name">
        <span class="dot dot-{{ peer.dot_color }}"></span>
        {{ peer.doorway_id }}
      </span>
      <span class="peer-agree {% if peer.agree_ratio >= 0.8 %}agree-good{% elif peer.agree_ratio >= 0.5 %}agree-warn{% else %}agree-bad{% endif %}">
        {{ peer.peers_agree }}
      </span>
    </div>
    <div class="peer-detail">
      Self: {{ peer.self_reported }} · Consensus: {{ peer.consensus }} · {% if let Some(ms) = peer.avg_response_ms %}{{ ms }}ms{% else %}—{% endif %}
    </div>
  </div>
  {% endfor %}
  {% endif %}

  <!-- Shefa Compute -->
  {% if show_shefa %}
  <h2>Compute Contribution</h2>
  <div class="shefa-card">
    {% if let Some(ref shefa) = shefa %}
    <div class="shefa-row"><span class="shefa-label">CPU</span><span>{{ shefa.cpu_hours }} hours</span></div>
    <div class="shefa-row"><span class="shefa-label">Storage</span><span>{{ shefa.storage_gb_hours }} GB·h</span></div>
    <div class="shefa-row"><span class="shefa-label">Bandwidth</span><span>{{ shefa.bandwidth_mbps_hours }} Mbps·h</span></div>
    <div class="shefa-row"><span class="shefa-label">Tokens (24h)</span><span>{{ shefa.tokens_24h }}</span></div>
    <div class="shefa-row"><span class="shefa-label">Steward Tier</span><span>{{ shefa.steward_tier }}</span></div>
    <div class="shefa-row"><span class="shefa-label">Trust Score</span><span>{{ shefa.trust_score }}</span></div>
    {% endif %}
  </div>
  {% endif %}

  <!-- Operator-Only Sections -->
  {% if is_operator %}
  <div class="operator-section">
    <h2><span class="operator-badge">Operator</span> Detailed View</h2>

    <!-- Attestation Log -->
    <h2>Recent Attestations</h2>
    <div class="card">
      {% for att in attestation_log %}
      <div class="component-row">
        <span style="font-size:0.85rem">{{ att.attestor }} → {{ att.subject }}</span>
        <span style="font-size:0.85rem;color:var(--text-muted)">{{ att.status }} · {% if let Some(ms) = att.response_ms %}{{ ms }}ms{% else %}—{% endif %} · {{ att.time_ago }}</span>
      </div>
      {% endfor %}
      {% if attestation_log.is_empty() %}
      <div style="padding:0.5rem 0;color:var(--text-muted);font-size:0.85rem">No attestations yet</div>
      {% endif %}
    </div>

    <!-- Route Registry -->
    <h2>Route Registry</h2>
    <div class="card">
      <div class="component-row">
        <span>Total Routes</span>
        <span>{{ route_count }}</span>
      </div>
      <div class="component-row">
        <span>Storage Registered</span>
        <span><span class="dot {% if steward_registered %}dot-green{% else %}dot-red{% endif %}"></span>{{ steward_registered }}</span>
      </div>
    </div>
  </div>
  {% endif %}

  <footer>
    Powered by the <a href="https://elohim.host">Elohim Protocol</a> · Data from DHT peer attestations
  </footer>
</body>
</html>
```

**Step 3: Add the template struct and handler**

In `status.rs`, add:

```rust
use askama::Template;

#[derive(Template)]
#[template(path = "status.html")]
struct StatusPageTemplate {
    self_id: String,
    version: String,
    overall_status: String,
    overall_status_label: String,
    self_tier: Option<String>,
    self_uptime_7d: Option<f32>,
    uptime_segments: Vec<String>, // "up", "degraded", "down", "nodata" x 168 (7 days hourly)
    response_p50: Option<u32>,
    response_p95: Option<u32>,
    peers_agree_summary: String,
    components: Vec<ComponentStatus>,
    peers: Vec<PeerStatus>,
    show_shefa: bool,
    shefa: Option<ShefaDisplay>,
    is_operator: bool,
    attestation_log: Vec<AttestationLogEntry>,
    route_count: usize,
    steward_registered: bool,
}

struct ComponentStatus {
    name: String,
    color: String, // "green", "yellow", "red", "gray"
    detail: String,
}

struct PeerStatus {
    doorway_id: String,
    dot_color: String,
    self_reported: String,
    consensus: String,
    peers_agree: String,
    agree_ratio: f64,
    avg_response_ms: Option<u32>,
}

struct ShefaDisplay {
    cpu_hours: String,
    storage_gb_hours: String,
    bandwidth_mbps_hours: String,
    tokens_24h: String,
    steward_tier: String,
    trust_score: String,
}

struct AttestationLogEntry {
    attestor: String,
    subject: String,
    status: String,
    response_ms: Option<u32>,
    time_ago: String,
}

pub async fn status_page(state: Arc<AppState>) -> Response<Full<Bytes>> {
    // Reuse status_check logic to gather all data
    let json_status = build_status_data(&state).await;

    // Build component list from status data
    let components = vec![
        ComponentStatus {
            name: "Gateway".to_string(),
            color: "green".to_string(),
            detail: "Operational".to_string(),
        },
        ComponentStatus {
            name: "Conductor Pool".to_string(),
            color: if json_status.conductor.connected { "green" } else { "red" }.to_string(),
            detail: format!(
                "{} ({}/{} workers)",
                if json_status.conductor.connected { "Operational" } else { "Disconnected" },
                json_status.conductor.connected_workers,
                json_status.conductor.total_workers,
            ),
        },
        ComponentStatus {
            name: "Projection Cache".to_string(),
            color: "green".to_string(),
            detail: format!("hit rate {:.0}%", json_status.cache.hit_rate * 100.0),
        },
        ComponentStatus {
            name: "Storage".to_string(),
            color: if json_status.storage.reachable { "green" } else { "red" }.to_string(),
            detail: if json_status.storage.reachable { "Operational" } else { "Unreachable" }.to_string(),
        },
    ];

    // Build peer list from federation stats
    let peers: Vec<PeerStatus> = json_status
        .federation
        .peers
        .iter()
        .map(|p| {
            let total: usize = p.peer_attestations.len();
            let online = p
                .peer_attestations
                .iter()
                .filter(|a| a.observed_status == "online")
                .count();
            let ratio = if total > 0 { online as f64 / total as f64 } else { 0.0 };
            let avg_ms = if !p.peer_attestations.is_empty() {
                let sum: u32 = p.peer_attestations.iter().filter_map(|a| a.response_time_ms).sum();
                let count = p.peer_attestations.iter().filter(|a| a.response_time_ms.is_some()).count();
                if count > 0 { Some(sum / count as u32) } else { None }
            } else {
                None
            };

            PeerStatus {
                doorway_id: p.doorway_id.clone(),
                dot_color: match p.consensus_status.as_str() {
                    "online" => "green",
                    "degraded" => "yellow",
                    "unreachable" => "red",
                    _ => "gray",
                }
                .to_string(),
                self_reported: p.self_reported_status.clone().unwrap_or_else(|| "unknown".to_string()),
                consensus: p.consensus_status.clone(),
                peers_agree: p.peers_agree.clone(),
                agree_ratio: ratio,
                avg_response_ms: avg_ms,
            }
        })
        .collect();

    // Check if requester is operator (JWT cookie)
    let is_operator = false; // TODO: extract from request cookies

    // TODO: uptime_segments from DoorwayHeartbeatSummary (168 hourly segments for 7 days)
    let uptime_segments: Vec<String> = (0..168).map(|_| "nodata".to_string()).collect();

    let template = StatusPageTemplate {
        self_id: json_status.node_id.clone(),
        version: json_status.version.to_string(),
        overall_status: json_status.diagnostics.status.clone(),
        overall_status_label: json_status.diagnostics.status.clone(),
        self_tier: json_status.federation.self_tier.clone(),
        self_uptime_7d: json_status.federation.self_uptime_7d,
        uptime_segments,
        response_p50: None, // TODO: from attestation response times
        response_p95: None,
        peers_agree_summary: format!("{} peers", json_status.federation.peer_count),
        components,
        peers,
        show_shefa: false, // TODO: query shefa CustodianMetrics from storage
        shefa: None,
        is_operator,
        attestation_log: vec![],
        route_count: state.route_registry.route_count().await,
        steward_registered: state.route_registry.steward_registered().await,
    };

    match template.render() {
        Ok(html) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Full::new(Bytes::from(html)))
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from(format!("Template error: {}", e))))
            .unwrap(),
    }
}
```

**Step 4: Refactor status_check to share data gathering**

Extract the data-gathering logic from `status_check` into a shared `build_status_data()` function that both `status_check` (JSON) and `status_page` (HTML) call.

**Step 5: Build and test**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build --release
RUSTFLAGS="" cargo test --lib --bins
```

**Step 6: Commit**

```bash
git add doorway/doorway-service/Cargo.toml doorway/doorway-service/templates/status.html doorway/doorway-service/src/routes/status.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): add server-rendered /status page with Askama template"
```

---

### Task 6: Operator Auth-Gated Expansion

Wire JWT cookie checking so operators see the expanded status page.

**Files:**
- Modify: `doorway/doorway-service/src/routes/status.rs`

**Step 1: Extract JWT from request**

The `status_page` handler needs access to the original request to check cookies. Update the function signature to accept the request:

```rust
pub async fn status_page(req: Request<Incoming>, state: Arc<AppState>) -> Response<Full<Bytes>> {
```

**Step 2: Check for operator JWT**

Add cookie extraction logic:

```rust
let is_operator = req
    .headers()
    .get("cookie")
    .and_then(|v| v.to_str().ok())
    .and_then(|cookies| {
        cookies.split(';').find_map(|c| {
            let c = c.trim();
            if c.starts_with("doorway_token=") {
                Some(c.trim_start_matches("doorway_token=").to_string())
            } else {
                None
            }
        })
    })
    .map(|token| {
        // Validate JWT using existing auth module
        state.auth.validate_operator_token(&token).is_ok()
    })
    .unwrap_or(false);
```

**Step 3: When operator, populate attestation log**

```rust
let attestation_log = if is_operator {
    // Query all recent attestations (about self and peers)
    // from DHT via zome_caller
    // ... (follow same zome call pattern as federation stats)
    vec![] // TODO: populate from DHT query
} else {
    vec![]
};
```

**Step 4: Update http.rs match arm to pass request**

```rust
(Method::GET, "/status") => to_boxed(routes::status_page(req, Arc::clone(&state)).await),
```

**Step 5: Build and test**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build --release
RUSTFLAGS="" cargo test --lib --bins
```

**Step 6: Commit**

```bash
git add doorway/doorway-service/src/routes/status.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): gate operator sections on status page via JWT cookie"
```

---

### Task 7: Integration Wiring and Smoke Test

Connect all the pieces: ensure the heartbeat loop calls the new zome function, status page queries attestations, and the whole flow works end-to-end.

**Files:**
- Modify: `doorway/doorway-service/src/main.rs` (if needed for new imports)
- Test all pieces together

**Step 1: Verify DNA builds with new entry type**

```bash
cd elohim/holochain/dna/infrastructure
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

Expected: `Finished release [optimized]`

**Step 2: Verify doorway-service builds with all changes**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build --release 2>&1 | tail -5
```

Expected: `Finished release [optimized]`

**Step 3: Run all doorway tests**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10
```

Expected: `test result: ok. 331+ passed; 0 failed`

**Step 4: Run clippy**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo clippy -- -D warnings
```

Expected: No warnings.

**Step 5: Run cargo fmt**

```bash
cd doorway/doorway-service
cargo fmt --check
```

Expected: No formatting issues.

**Step 6: Verify DNA tests**

```bash
cd elohim/holochain/dna/infrastructure
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -10
```

**Step 7: Commit any remaining fixes**

```bash
git add -A
git commit -m "chore(doorway): integration fixes for federated health status"
```
