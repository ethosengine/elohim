# Conductor Agent-Info Substrate Gossip (Step Zero)

**Status**: Design — pre-implementation
**Date**: 2026-05-28
**Related**: `genesis/docs/superpowers/sprints/2026-05-27-federation-wiring-audit.md` (audit that surfaced the gap), commit `91f300663` (Phase 1: per-human primary doorway routing in `elohim/holochain/Jenkinsfile`)
**Memory**: `project_multi_doorway_human_registration`, `project_iroh_phase11_all_backends_wired`, `project_three_layer_truth_model`

## Problem

The federation-wiring-audit shift landed Phase 1: per-human primary doorway routing. Family-three personas (matthew/jessica/james) get `signal.doorway-alpha.elohim.host`; the 11 remote personas (adam included) get `signal.elohim.host`. Each Holochain conductor registers itself at exactly one signal URL — the cluster's signaling mesh is now split along the doorway-A / doorway-B boundary.

The kitsune2 networking layer (Holochain 0.7.0-dev.5, pinned via `holochain_types = 0.7.0-dev.5`) accepts only one `bootstrap_url` + one `signal_url` per conductor. There is no schema for multi-bootstrap. Without an additional substrate channel that propagates peer information across the libp2p mesh, the two halves of the cluster will not naturally discover each other.

The seeder (`genesis/seeder/src/seed-accounts.ts`) splits its writes hash-mod across the full `SEEDER_TARGET_PEERS` set — currently ~11-14 storage URLs in alpha — and trusts the substrate to fan out content from each primary landing peer. Under Phase 1 without this design, Matthew's writes land at `elohim-matthew-alpha` but never reach `elohim-adam-alpha` via Holochain DHT gossip (matthew's conductor doesn't know adam's `AgentInfoSigned`, so kitsune2 has no peer to gossip with). The seeder's contract — "split the payload, let the substrate sync" — breaks.

## Solution

A new gossip topic, `elohim/conductor/agent-info/v1`, published over the existing `DualGossipPublisher` (libp2p + iroh, byte-identical). Every `elohim-storage` pod periodically publishes its embedded conductor's `AgentInfoSigned` JSON strings (obtained via `admin_ws.agent_info(None)`); every other pod subscribes, verifies, and injects via `admin_ws.add_agent_info(...)`. The conductor's own peer cache is the destination store; the substrate is purely the transport.

The kitsune2 signal URL is REGISTRATION-only. Outbound connections to a known peer use that peer's claimed URLs (carried in their `AgentInfoSigned`). Once matthew's conductor has adam's agent_info in its cache, matthew's outbound kitsune2 gossip reaches adam via `signal.elohim.host` (publicly reachable from matthew's pod even though matthew registered on `signal.doorway-alpha.elohim.host`). The two halves of the cluster talk to each other through both signal servers without the signal servers federating.

## Constraints and verified facts

1. **`holochain_client = 0.9.0-dev.12`** (Cargo.lock resolved). Exposes:
   - `admin_ws.agent_info(dna_hashes: Option<Vec<DnaHash>>) -> ConductorApiResult<Vec<String>>` at `admin_websocket.rs:536`
   - `admin_ws.add_agent_info(agent_infos: Vec<String>) -> ConductorApiResult<()>` at `:548`
   - Symmetric JSON-string round-trip (kitsune2 v2 format). Substrate transports opaque blobs; conductor validates signatures + dedupes on ingest.
2. **`DualGossipPublisher`** is the established broadcast primitive (`elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md`). Twelve publish sites today. Step zero is catalog entry #13. Pattern is proven.
3. **libp2p mesh is already full-mesh across doorway-A / doorway-B.** `computeP2PBootstrapNodes` in `elohim/holochain/Jenkinsfile` gives every peer a full multiaddr set within its namespace plus one bridge per other namespace. The transport for this gossip exists without any P2P-topology change.
4. **Iroh subscriber-side wiring gap (acknowledged)**. Per the dual-publish catalog's "Subscriber-Side Parity" note, the iroh subscriber side is publish-only today; the libp2p subscriber side is fully wired. Step zero subscribes via libp2p only. The iroh subscriber wiring lands behind Plan 4 Task 8 (cross-stack soak) and adds redundancy when it lands, not new capability — the libp2p mesh covers the same membership.

## P2P Design Gate Output

### Entity: `ConductorAgentInfo` (gossip wire payload — the only new shape this design introduces)

- **Classification**: Operational (C).
- **Justification**: In-flight gossip envelope, never stored. Receivers decode, verify, inject into the conductor's existing peer cache, then drop. Lost messages are reconstructed by the next 60s heartbeat. No persistence beyond the conductor's own internal store.
- **Content Address Strategy**: Routing topic_id is `BLAKE3("elohim/conductor/agent-info/v1")[..32]` per dual-publish convention. Wire payload has no identity — it's a transport message, not a stored entity.
- **Source of Truth**: The publishing peer's embedded conductor, queried via `admin_ws.agent_info(None)`. Substrate is replication, not authority.
- **Coordinator Zome**: None. Lives in `elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs` (Rust service layer), parallel to `identity_binding_gossip.rs`.
- **Storage Projection**: None. The destination store is the embedded conductor's own peer cache, written via `admin_ws.add_agent_info`. No diesel table, no migration, no view.
- **HTTP Route**: None. The conductor's existing `agent_info` admin RPC is the local read interface; localhost-only by design.
- **Anti-Pattern Check**: All three forbidden alternatives explicitly ruled out — (a) creating a new "peer manifest" DHT entry type duplicates `AgentInfoSigned` and wastes Lamad's ~73/~100 headroom on a no-information-added entry; (b) a diesel shadow table is the "Standalone table for agent state" anti-pattern; (c) slug addressing rejected — topic_id is content-derived (BLAKE3 over topic name).

### Existing primitives propagated, not redesigned

- `AgentInfoSigned` — Holochain primitive, JSON-string encoded in kitsune2 v2. Already signed, self-verifying, has TTL (~20 min). Governed by Holochain's own design rules.
- Conductor peer cache — owned by Holochain. Written via admin RPC. Not under this design's authority.

## Architecture

```
                ┌────────────────────────────────────────────────────────┐
                │                  elohim-storage pod                    │
                │                                                        │
   admin_ws ───►│  AgentInfoPublisher    AgentInfoSubscriber             │
   :4444        │       (60s tick)         (bounded queue +              │
                │           │               rate-limited worker)         │
                │           ▼                                            │
                │   DualGossipPublisher           ▲                      │
                │      ├──libp2p─────►       ◄────libp2p──┐              │
                │      └──iroh────►         (iroh subscribers            │
                │                            deferred to Plan 4 T8)      │
                │                                                        │
                │    topic: elohim/conductor/agent-info/v1               │
                └───────────┼──────────────────────────────────────────┘
                            │
                            ▼ libp2p mesh (full-mesh A↔B per
                              computeP2PBootstrapNodes)
                            │
                ┌───────────┴──────────────────────────────────────────┐
                │              every OTHER elohim-storage pod          │
                │  Subscriber edge → bounded queue → worker tick →     │
                │  verify + batch → admin_ws.add_agent_info(Vec<String>)│
                └──────────────────────────────────────────────────────┘
```

Two logical components per pod (publisher, subscriber), three internal tokio tasks (publisher, subscriber-edge, subscriber-worker). One new module. One catalog row. No conductor-config changes, no DNA changes, no diesel migrations, no HTTP routes.

## Components

### Module structure

New file: `elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs` (~150 lines). Module placement parallels `identity_binding_gossip.rs` so the existing wiring conventions apply.

### Wire payload

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConductorAgentInfo {
    /// Opaque kitsune2 agent_info JSON string. Round-tripped publisher→subscriber.
    /// Receiver passes directly to admin_ws.add_agent_info. The conductor itself
    /// does signature verification + dedup. Edge handler does only cheap
    /// structural checks (non-empty, well-formed JSON envelope).
    pub agent_info_json: String,
    /// Microsecond timestamp at publish; for observability + last-seen dedup
    /// (subscriber drops older published_at for the same peer key).
    pub published_at: i64,
}

pub const CONDUCTOR_AGENT_INFO_TOPIC: &str = "elohim/conductor/agent-info/v1";
```

Wire size: typical kitsune2 agent_info JSON is ~400-600 bytes, plus ~30 bytes of envelope = ~500-700 bytes per message. 14 peers × ~4 own-agent entries × 1 msg/min = ~56 messages/min cluster-wide = ~30 KB/min total gossip bandwidth. Negligible.

### Publisher

Spawned after `happ_manager::wait_for_ready` returns. Ticks every `interval` (default 60s, well under kitsune2's ~20min TTL).

```rust
pub fn spawn_agent_info_publisher(
    admin_ws: Arc<AdminWebsocket>,
    publisher: Arc<DualGossipPublisher>,
    interval: Duration,
    shutdown: CancellationToken,
) -> JoinHandle<()>;

async fn publish_once(admin_ws: &AdminWebsocket, publisher: &DualGossipPublisher) -> Result<()> {
    // None = all DNAs this conductor is a member of
    let agent_info_strings = admin_ws.agent_info(None).await?;
    // Filter to OWN agents only — agent_info() returns the full peer cache
    // including entries learned via gossip. Without this filter, every pod
    // re-publishes everyone's info (amplification + confusion).
    let own_keys: HashSet<AgentPubKey> = admin_ws.list_cell_ids().await?
        .into_iter().map(|cid| cid.agent_pub_key().clone()).collect();
    let now = now_micros();
    for json in agent_info_strings {
        if !json_mentions_any_key(&json, &own_keys) { continue; }
        let msg = ConductorAgentInfo { agent_info_json: json, published_at: now };
        publisher.publish(CONDUCTOR_AGENT_INFO_TOPIC, msg.to_bytes()?).await?;
    }
    Ok(())
}
```

`json_mentions_any_key` does a substring check against the kitsune2 v2 serialization of each agent pubkey. The kitsune2 v2 agent_info JSON envelope is documented in the `holochain_types` crate; implementation-time work confirms the exact field name (likely `agent` or `space` per kitsune2 conventions). Substring match is cheap and adequate given the small own-keys set (~4 entries per pod). If kitsune2 v2 field naming proves ambiguous, fall back to a structured `serde_json::Value` parse on each entry — still O(N) where N is small.

**Seed-time race fix**: in addition to the heartbeat tick, the publisher exposes a `publish_now()` API. Called once from `main.rs` after `wait_for_ready` returns (one-shot first publish to bound the cold-start window). Optional follow-on: trigger publish_now on first inbound `/account/import` POST. Default to first-publish-on-spawn only; add post-import trigger as a tuning option if seed-window soak shows it's needed.

### Subscriber (pull-based)

Two tasks: a cheap gossip-edge handler that never blocks on admin RPC, and a worker that owns its own cadence + rate.

```rust
pub fn spawn_agent_info_subscriber(
    admin_ws: Arc<AdminWebsocket>,
    mut receiver: GossipReceiver,
    cfg: SubscriberConfig,  // batch_interval, queue_capacity, max_rate_per_sec, max_batch
    shutdown: CancellationToken,
) -> (JoinHandle<()>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel::<ConductorAgentInfo>(cfg.queue_capacity);

    // Edge handler — lightweight, never blocks on admin RPC.
    // Bounded channel: try_send fails when full → drop (next heartbeat re-delivers).
    let edge = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                Some(bytes) = receiver.recv(CONDUCTOR_AGENT_INFO_TOPIC) => {
                    let Ok(msg) = ConductorAgentInfo::from_bytes(&bytes) else { continue };
                    if msg.verify_structural().is_err() { continue }
                    let _ = tx.try_send(msg);
                }
            }
        }
    });

    // Worker — owns its own tick + rate.
    let worker = tokio::spawn(async move {
        let mut tick = tokio::time::interval(cfg.batch_interval);
        let mut last_seen: HashMap<String, i64> = HashMap::new();
        let mut limiter = RateLimiter::new(cfg.max_rate_per_sec);
        let mut rx = rx;
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = tick.tick() => {
                    let mut batch = Vec::with_capacity(cfg.max_batch);
                    while batch.len() < cfg.max_batch {
                        match rx.try_recv() {
                            Ok(msg) => {
                                if let Some(json) = verify_and_dedupe(msg, &mut last_seen) {
                                    batch.push(json);
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    if batch.is_empty() { continue }
                    limiter.acquire(batch.len() as u32).await;
                    if let Err(e) = admin_ws.add_agent_info(batch).await {
                        warn!(error = %e, "agent_info batch ingest failed; heartbeat will retry");
                    }
                }
            }
        }
    });
    (edge, worker)
}
```

### Pull-based discipline (load-bearing)

- **Bounded inbound queue = automatic backpressure.** Channel-full drops new gossip rather than backing up the gossipsub task or memory-bloating. Drops are safe because the 60s heartbeat re-delivers everything — at most one heartbeat window of latency on convergence.
- **Worker owns its own cadence and rate.** Per-pod tick (default 200ms) + per-pod rate limit (default 20/sec on chromebook-class, 200/sec on performance-class). A slow pod consumes slower — it does not back-pressure publishers or other subscribers.
- **Batched admin RPC.** One `add_agent_info(Vec<String>)` per tick, not one per gossip message — both honors HC's vec-accepting API and amortizes RPC overhead.
- **Per-archetype tuning** via env vars (`AGENT_INFO_MAX_RATE_PER_SEC`, `AGENT_INFO_BATCH_INTERVAL_MS`, `AGENT_INFO_QUEUE_CAPACITY`, `AGENT_INFO_MAX_BATCH`). Defaults per device archetype, operator-overridable. Matches the existing `policy.toml` cadence-archetype pattern.

### Wiring — three small edits

| File | Change |
|---|---|
| `elohim/elohim-storage/src/p2p/topics.rs` | Add `pub const CONDUCTOR_AGENT_INFO: &str = "elohim/conductor/agent-info/v1";` |
| `elohim/elohim-storage/src/main.rs` | After `admin_ws = manager.wait_for_ready(...)` returns (around L495): spawn subscriber immediately; spawn publisher with `publish_now()` then enter heartbeat loop. Pass the existing shutdown token + the existing `DualGossipPublisher` Arc. |
| `elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md` | Add row #13: topic, producer call site, payload type, `to_vec_named` flavor. |

## Data flow

### Cold start (per pod)

Subscriber spawns immediately (no precondition). May fill its inbound queue from cluster heartbeats before its own conductor is ready; worker drains at its own pace, drops are safe.

Publisher spawns after `wait_for_ready` returns. First action: one-shot `publish_now()` (the seed-time race fix). Then 60s tick loop.

### First receive on a cold cluster (timeline)

```
T+0s    Seeder POSTs Matthew's AccountPackage → elohim-matthew-alpha (hash-mod primary).
T+0s    elohim-matthew-alpha commits → DHT entries in matthew's source chain.
T+0s    Matthew's publish_now() fires (cold-start trigger). Publishes 4 ConductorAgentInfo
        envelopes (one per DNA: lamad, imagodei, infrastructure, mishpat).
T+~50ms libp2p-gossipsub fans out. All 13 other pods receive.
T+~50ms Adam's edge handler decodes, try_sends each into adam's bounded queue.
T+~200ms Adam's worker tick. Drains 4, verify_and_dedupe, calls add_agent_info([4]).
T+~250ms Adam's conductor now has matthew's agent_info in its peer cache.
T+~250ms Adam's kitsune2 sees new peer; schedules outbound gossip using matthew's
        claimed URL (signal.doorway-alpha.elohim.host — reachable from adam's pod).
T+500ms-2s WebRTC handshake completes; signal.doorway-alpha facilitates SDP exchange.
T+1-3s   Adam's conductor pulls matthew's freshly-committed DHT entries via kitsune2.
T+3-5s   Cluster-wide DHT replication of Matthew's package converges.
```

Worst case: 5 seconds from "Matthew writes" to "Adam has the data". Without step zero: never.

### Steady state

Each pod publishes every 60s. Each other pod's worker batches into one admin RPC per ~200ms tick. Cache stays fresh well within the kitsune2 ~20min TTL.

### Doorway-A signal server goes down mid-session

- Matthew/jessica/james lose ability to NEWLY register peer connections via signal-A.
- Every peer in the cluster already has matthew/jessica/james agent_info in cache (substrate gossip).
- Existing WebRTC connections persist (they don't route through the signal server post-handshake).
- Heartbeat continues from matthew etc.; receivers continue to re-inject (idempotent, no-op on dedup).
- Steady-state DHT replication continues.
- Broken: NEW peers joining the cluster cannot reach matthew/jessica/james for first contact until either doorway-A recovers OR a future revision publishes multi-URL agent_info that includes redundant signal endpoints.

### Pod restart

Cold start runs again. Subscriber immediately receives steady-state heartbeats from the cluster. Publisher restarts on `wait_for_ready`; FIRST publish (publish_now) immediately announces the pod to the cluster.

### The seeder's perspective

The seeder has two write shapes against the substrate, and step-zero substrate gossip closes the cross-doorway sync gap for one in full and for the other only partially:

**Content writes (`seed-accounts.ts`):** hash-mod split across `SEEDER_TARGET_PEERS`. Each AccountPackage lands at one primary peer. Holochain DHT gossip — now cross-doorway-capable — handles fanout. The "split the payload, let the substrate sync" contract holds end-to-end. No seeder code change.

**Projection-commitment writes (`seed-projections.ts`):** single-POST against one `DOORWAY_URL`. Author-side projection path is identical to content (local conductor commits → post-commit → `ProjectionSignal::ReaCommitmentCommitted` → in-process subscriber → `rea_commitments` upsert → SSE `projection.registered` → `EprRouter.replace_all`). DHT gossip still propagates the entry to other clusters' conductors. But because Holochain's `post_commit` fires only on local commits (per `dna/CLAUDE.md` gospel), remote peers' `rea_projection` signal subscriber never observes the entry, the remote `rea_commitments` table stays empty, and the remote doorway's `EprRouter` never learns about the projection. Step-zero substrate gossip is **necessary** for cross-doorway projection availability (no DHT propagation without it) but **not sufficient** — see "Sibling Federation Gaps" in the implementation plan for the concurrent work (F1: receiver-side projection, F2: bi-doorway seeding) that closes the EPR-delivery story end-to-end.

## Error handling

| Failure | Detection | Recovery |
|---|---|---|
| admin_ws disconnect | `add_agent_info` returns WebsocketError | Worker logs `warn`, drops batch, next tick retries. Reconnect handled by holochain_client. |
| Cell not yet active on publisher first tick | By construction can't happen — publisher waits for `wait_for_ready` | N/A |
| Subscriber starts before any publishers | Inbound queue stays empty | Benign — first heartbeat fills it |
| Gossip publish fails (mesh partition, no peers) | `DualGossipPublisher::publish` returns error | Heartbeat retries every 60s |
| Inbound queue full (slow worker, fast publishers) | `try_send` returns `TrySendError::Full` | Drop silently; heartbeat re-delivers. Operator-visible metric: queue-full count. If steady-elevated → tune `QUEUE_CAPACITY` or `MAX_RATE_PER_SEC`. |
| Malformed payload | `from_bytes` returns error in edge handler | Drop, log `debug`. Other messages unaffected. |
| add_agent_info rejection (sig invalid / expired) | RPC returns `ConductorApiError::Conductor(...)` | Worker logs `debug`, drops the entry. Publisher will re-publish (potentially with refreshed agent_info) on next heartbeat. |
| Worker task panics | tokio task abort | Module exposes `WatchdogHandle`; spawn caller polls for terminated tasks and re-spawns. Mirrors `services/federation::spawn_peer_discovery_task` pattern. |
| Pod-wide outbound mesh isolation | Existing libp2p mesh-health metrics | Leaf-consumer behavior; resumes when mesh recovers |

We do NOT persist agent_infos to local sqlite. The conductor's own peer cache is the persistent store (kitsune2 manages persistence). Restart cycles re-warm via gossip within one heartbeat window. Persisting would be the design-gate-flagged "Standalone table for agent state" anti-pattern.

## Testing

### Unit tests (in-module)

1. `ConductorAgentInfo` round-trip — `to_bytes`/`from_bytes` parity; structural validation rejects empty fields.
2. `verify_and_dedupe` semantics — last_seen map drops older `published_at` for the same peer key; updated entries replace; malformed JSON rejected.
3. Publisher "skip non-self entries" filter — given a conductor cache containing N agents (mix of self + foreign), publisher only emits the N_self subset matching `list_cell_ids`.
4. Worker rate limiter — under burst of 100 inbound messages with `max_rate_per_sec=10`, worker takes ≥9 seconds to drain (within ±10% tolerance). Bounded queue absorbs burst without blocking edge handler.
5. Worker batching — 10 inbound messages within one batch_interval → exactly one `add_agent_info` call with all 10 as a Vec.

### Cross-stack integration test (Phase 11 `tests/iroh_*` umbrella)

`tests/iroh_agent_info_gossip_parity.rs` — uses `multi_stack_fixture` two-node setup:
1. Spawn two elohim-storage stacks (libp2p+iroh both).
2. Mock admin_ws via a fake responder that records `add_agent_info` calls.
3. Node A publishes a fixture envelope via DualGossipPublisher.
4. Node B's subscriber receives, worker fires, mock admin records the inject.
5. Assert: JSON string node A published is byte-identical to what node B's admin_ws received.

Mirrors `iroh_gossip_parity.rs` from the Phase 11 catalog.

### A2O scenarios (`genesis/a2o/features/federation/cross-mesh-discovery.feature`)

```gherkin
Feature: Cross-mesh DHT discovery survives the doorway-A / doorway-B partition

  Background:
    Given the alpha cluster has 14 humans deployed with per-human primary doorway routing
    And matthew/jessica/james are registered with signal.doorway-alpha.elohim.host
    And adam plus 10 others are registered with signal.elohim.host

  Scenario: Seeded content lands on one peer and reaches the cross-mesh half
    Given the seeder writes Matthew's AccountPackage to elohim-matthew-alpha (hash-mod primary)
    When the seeder completes the Matthew package import
    Then within 30 seconds adam's conductor can DHT-resolve Matthew's ContributorPresence
    And within 30 seconds pete's conductor can DHT-resolve Matthew's identity binding

  Scenario: Signal-server-A goes down mid-session and the cross-mesh half stays reachable
    Given the cluster is in steady state with all conductor peer caches warm
    When signal.doorway-alpha.elohim.host becomes unreachable
    Then existing inter-peer DHT operations continue to complete for at least 5 minutes
    And adam can still DHT-resolve content authored by matthew via cached peer info
```

### Soak (24-hour Phase 11 prereq #6 parity)

Monitor `add_agent_info`-call rate, queue-full count, dropped-message count, conductor peer-cache size against a control pod (one pod without substrate gossip enabled). Acceptance: cache-size variance < 5% between control and treatment, queue-full count = 0 on chromebook-class with default config, no admin RPC backup observable.

## Rollout

1. Land the module + unit tests + cross-stack parity test as a single PR. Behind an env-var feature flag `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP` (default `false`).
2. Enable on matthew + adam (the genesis pair) for one full deploy cycle. Watch metrics for the cluster halves' first cross-mesh DHT lookup latencies.
3. Enable cluster-wide once metrics are clean.
4. Drop the feature flag after one stable week.

The flag deliberately defaults `false` for a deploy cycle so partial rollout is observable — peers without the flag are unaffected (no publisher = nothing to inject; no subscriber = no foreign agent_info injected), and the cluster continues to function in its pre-step-zero shape on those pods.

## What this design explicitly does NOT do (out of scope; named for future-phase pointers)

- **Cold-start with signal_url down**: requires substrate-level WebRTC signaling (ICE candidate exchange over gossip). Phase 12 work; not addressed here.
- **Cross-cluster reach** (peer on cluster-A discovers peer on cluster-B without operator FEDERATION_PEERS wiring): doorway-side federation discovery is a separate sequenced item in the federation-wiring-audit sprint result.
- **Multi-URL agent_info** (peer publishes both signal-A and signal-B as fallback reach paths): requires either an upstream HC schema extension or a kitsune2 URL-list serialization change. Phase 12+.
- **Session/recovery doorway-agnostic refactor**: `local_sessions.doorway_url` is `TEXT NOT NULL` today; making sessions multi-doorway is a separate, larger lift outside this design's scope (audit's section 2e).
- **Replacing the doorway signal server with a substrate-native signaler**: the operator's "ideally bootstrap is only that" framing. This design keeps the doorway signal server in the steady-state path; the substrate channel parallels it rather than replacing it. Phase 12+.
- **Iroh subscriber-side wiring**: deferred behind Plan 4 Task 8. Adds redundancy, not new capability — the libp2p mesh already covers cluster membership.

## Memory pointer

When this design lands, update `project_multi_doorway_human_registration` — the current-state section should reflect that the conductor peer-cache layer is now substrate-warmed (the first of the three blocking layers from the federation-wiring-audit) and the remaining two layers (doorway-federation bidirectional reconciliation, session.doorway_url single-pin) are unchanged.
