# Peer-Stewarded Availability Signaling

**Date:** 2026-04-17
**Status:** Design
**Scope:** elohim-storage, elohim-node, doorway, Holochain infrastructure DNA

## Summary

Peers publish their availability state directly through a single `PeerStatus` surface stewarded by elohim-storage. Doorway becomes a pure subscriber that makes routing decisions from peer-authored state, distinguishing traffic addressed to a specific agent (which reaches its hosting peer regardless of pool membership) from general load-balanced traffic (which filters to peers currently advertising pool membership). Deployment wrappers (elohim-node, tauri, browser) package elohim-storage for their context and configure its capabilities without implementing them.

The design corrects a current inversion — doorway authoring heartbeats on behalf of conductors it proxies to — and lays the first concrete foundation for the Terrance-class thin-client case: a peer that belongs in the mesh but should not be asked to carry general traffic.

## Motivating cases

**Terrance (thin client).** A frequently-offline chromebook. His agent lives on his device; when he logs in, doorway must reach his conductor to serve his session. But doorway must never select his device as a general-purpose pool member when other peers could serve the request.

**Adam (bootstrapper node).** A capable Level-5 node in the alpha cluster, expected to contribute to doorway's general pool. His conductor binds localhost (per Holochain convention); today his `app_interface` on port 4445 is unreachable cross-pod because `attach_app_interface` always binds localhost. Adam sits outside the pool despite being a full node.

**Matthew (current anchor).** Works today because a `socat` sidecar bridges the pod's `0.0.0.0:8444` to the conductor's `127.0.0.1:4444`. The sidecar is a one-off pattern that would need to be reproduced for every deployment context unless generalized.

## Principles

1. **Stewardship, not sovereignty.** No component stands alone. Peers depend on wrappers, wrappers on orchestrators, orchestrators on operators, operators on households, households on the mesh. This design distributes stewardship across that web rather than concentrating authority in any one layer. Where the spec says "peer-stewarded," it means "the peer is the nearest authority on this piece of state, contributing it on behalf of the mesh — not deciding in isolation."

2. **Peers steward their status.** The peer is in direct contact with its lived resource reality (storage %, conductor health, network conditions). It is the best-positioned contributor of moment-to-moment availability signals. Other components — attestors, elohim-agents, orchestrators — contribute complementary views (observed health, rebalancing coordination, container shape), but none are closer to the peer's current state than the peer itself.

3. **Archetypes are priors, not prescriptions.** Device archetypes describe the envelope of what is possible given hardware. They seed defaults; they do not declare what a peer is actually doing. Lived state supersedes archetype whenever the two disagree.

4. **Wrappers package, workloads serve.** elohim-storage is identical across form factors. Deployment-specific concerns (orchestrator lifecycle translation, OS integration, config delivery) live in the wrapper layer (elohim-node, tauri, browser). Feature capabilities (forwarder, policy engine, heartbeat) live inside elohim-storage, gated by policy flags. Each wrapper configures the appropriate flags for its deployment context.

5. **Doorway listens, never pesters.** Doorway is a projection of peer-authored state. It subscribes to peer heartbeats and routes accordingly; it does not write availability claims on behalf of peers.

6. **One signal surface.** Lifecycle transitions and periodic capability snapshots ride the same `PeerStatus` entry, distinguished by a state-machine `status` field. Less cross-surface consistency risk, less DHT traffic.

## Architecture

### Stewardship boundaries

| Component | Stewards | Does not steward |
|-----------|----------|------------------|
| **elohim-storage** | `PeerStatus` authorship, policy evaluation, self-assessment, optional TCP forwarder, graceful-shutdown emission | Orchestrator protocol translation, container shape discovery |
| **elohim-node** (wrapper) | Container shape reporting to elohim-storage, orchestrator-lifecycle translation, config plumbing, operator-facing health surface | Feature implementation, peer-state authorship, policy decisions |
| **doorway** | Subscribing to `PeerStatus`, agent→hosting-peer resolution, route selection, doorway-internal service metrics (cache hits, routes served) | Authoring peer availability, tracking peer liveness beyond what peers publish |
| **elohim-agent co-stewardship channel** (future) | Signed directive records contributing to peer policy input | Replacing peer self-authorship as the primary write path |

The existing `DoorwayHeartbeat` entries that doorway authors today migrate to `PeerStatus` stewarded by elohim-storage. Doorway's service-level stats remain doorway-internal metrics, distinct from peer-availability signaling.

### `PeerStatus` surface

One DHT entry type authored by elohim-storage, approximately every 60 seconds and on transitions.

```rust
pub struct PeerStatus {
    pub peer_id: AgentPubKey,
    pub status: PeerLifecycleState,
    pub flags: PeerCapabilityFlags,
    pub archetype_class: Option<String>, // e.g. "home-nuc", "chromebook-edu"
    pub timestamp: Timestamp,
}

pub enum PeerLifecycleState {
    Starting,      // booting; not yet in pool; subscribers expect a concrete state soon
    Online,        // accepting traffic per current flag set
    Degraded,      // responsive but signaling reduced capability
    Maintenance,   // scheduled downtime / operator-marked; pool eviction
    Leaving,       // graceful shutdown announced; pool eviction; subscribers may retain cached state briefly
}

pub struct PeerCapabilityFlags {
    pub general_pool_member: bool,
    pub accepting_stewardship_reserves: bool,
    // Evolution hooks — see §Evolution
    // pub traffic_class: Option<PeerTrafficClass>,
    // pub current_load: Option<LoadMetric>,
}
```

Implicit `Offline` = no heartbeat within N cycles (safety net for ungraceful termination). Graceful peers announce `Leaving` before exit; ungraceful peers are evicted by timeout.

### Policy layer

elohim-storage evaluates `PeerCapabilityFlags` each heartbeat cycle by running its policy against live state.

**v1: explicit config with archetype defaults.** `peer-policy.toml` under elohim-storage's config path:

```toml
[pool]
accept_general_traffic = "auto"      # auto | true | false
min_free_storage_pct = 20
require_conductor_healthy = true

[stewardship]
accept_new_reserves = "auto"
max_storage_pct = 80

[network]
expose_conductor_externally = false  # TCP forwarder gate
conductor_external_bind = "0.0.0.0:4445"
```

When a flag is `"auto"`, the policy engine derives it from live state. Archetype provides the *threshold defaults* — Terrance's chromebook archetype supplies one set of numbers, Adam's node archetype supplies another — so peers rarely need to hand-author values.

**v2: auto-sensing.** At boot, elohim-storage queries its wrapper for container shape (memory, storage, CPU, network interfaces, archetype hint), classifies itself against the archetype library, loads the matching policy. Explicit TOML becomes an override path, used for failure-mode testing and deliberate operator tuning. The archetype library becomes a classifier, not a static lookup.

### Wrapper contract

elohim-node (and peer wrappers tauri, browser, mobile) provide a thin, orchestrator-generic contract to elohim-storage:

- **Shape report at boot:** available memory, storage, CPU class, network interface list, optional archetype hint.
- **Lifecycle translation:** whatever stop/start/drain signal the orchestrator speaks (k8s `SIGTERM`, elohim-operator's future equivalent, tauri window-close) → elohim-storage state transition.
- **Config plumbing:** orchestrator-provided config → `peer-policy.toml` path elohim-storage reads.
- **Health surface:** liveness/readiness in whatever protocol the current orchestrator uses.

k8s is developer scaffolding today; elohim-operator is the long-term orchestrator. Framing the wrapper contract in generic terms means elohim-node can be reimplemented against elohim-operator without touching elohim-storage.

### TCP forwarder capability (inside elohim-storage)

elohim-storage ships an optional TCP forwarder. When `[network].expose_conductor_externally = true`, after `attach_app_interface` attaches the conductor to localhost, elohim-storage spawns a tokio listener on the configured external bind address that pipes connections to `127.0.0.1:<conductor-port>`.

- Holochain convention (conductor stays on localhost, `danger_bind_addr` not used) is preserved.
- Same mechanism serves every wrapper. Browser wrapper config leaves it off (and could not activate it anyway — graceful no-op). k8s wrapper turns it on with `0.0.0.0` bind. Tauri can expose to the local network on user opt-in.
- Matthew's `socat` sidecar is retired in the same change — both adam and matthew use the forwarder.
- Capability stays under peer stewardship: if policy says `accept_general_traffic = false`, the forwarder may still be enabled (the peer's agent traffic still needs to reach the conductor), but the peer publishes `general_pool_member: false` and doorway respects it.

### Doorway routing

Doorway subscribes to `PeerStatus`. When a zome call arrives:

1. **Parse for addressee agent.** If the call targets a specific agent (capability-grant flow, direct message, session-scoped call), extract the addressee pubkey.
2. **Agent-addressed path.** Resolve addressee → hosting peer(s) via DHT lookup (new zome function; link structure already present in infrastructure DNA). Route to that peer regardless of `general_pool_member`. Terrance's chromebook receives his session traffic this way.
3. **General-pool path.** If no addressee, filter subscribed peers to `general_pool_member: true AND status IN (Online, Degraded)`, apply current selection logic (round-robin today, capacity-weighted when (iii) activates).
4. **No silent fallthrough.** If agent-addressed lookup fails (no hosting peer online), return a clear error; do not fall back to the general pool (would silently route addressee-scoped traffic to a stranger).

The conductor pool doorway maintains today (`CONDUCTOR_URLS` env var) is replaced by the live `PeerStatus` subscription. The env var remains as a bootstrap hint for initial connection, not a routing truth.

### elohim-agent co-stewardship channel (design hook only)

The design reserves a distinct write path for elohim-agents to contribute directive records (e.g., "please accept these reserves for mesh rebalancing"). This is co-stewardship, not override: the peer remains the primary author of its state, and agent directives are a second, audited, signed contribution that the policy engine weighs alongside live state. Implementation is out of v1 scope, but the `PeerStatus` schema and policy engine are shaped to accept directive-sourced contributions without a breaking change.

## Evolution

The design lights up (ii) and (iii) without schema churn:

| Story | v1 state | Path to completion |
|-------|----------|-------------------|
| **(ii) Traffic classes** | `general_pool_member: bool` | Promote to `PeerTrafficClass` enum on `PeerCapabilityFlags`, reusing the `DoorwayTier` shape already on `DoorwayRegistration`. Doorway routing gains a class-filter before pool selection. |
| **(iii) Numeric capacity** | Not published | Activate the `bandwidth_mbps` field already on `DoorwayRegistration` in the wire type, add `current_load` to `PeerCapabilityFlags`, enable weighted selection in doorway's router. |

Both changes are additive; v1 consumers of `PeerStatus` continue working through the transition.

## Out of v1 scope

- Numeric capacity publication and backpressure-aware routing (story (iii)).
- elohim-agent co-stewardship directive channel implementation (design hook only).
- Attesting peer claims — v1 trusts peers to contribute their status honestly; attestation arrives with the federated-doorway-health work (plan 2026-03-12) already in flight.
- Terrance's end-to-end thin-client session experience — v1 delivers the mechanism; the consuming feature is a separate story.
- Browser-context peer participation (service-worker-zip case).

## Migration plan (high level)

1. Introduce `PeerStatus` DHT entry type in infrastructure DNA alongside existing `DoorwayHeartbeat` (additive, no break).
2. Implement policy engine + heartbeat task in elohim-storage; every peer publishes `PeerStatus`.
3. Implement TCP forwarder capability in elohim-storage behind policy flag.
4. Update elohim-node wrapper: shape report, config plumbing, lifecycle translation. Turn on forwarder for adam. Retire matthew's socat sidecar in the same deploy.
5. Doorway subscribes to `PeerStatus`, implements agent-addressed routing and general-pool filtering. Retain `CONDUCTOR_URLS` as bootstrap hint only.
6. Deprecate doorway's direct heartbeat writes; remove after subscribers cut over.
7. Archetype library → policy default generator (fulfills 2026-04-13-device-archetypes-plan dependency).

Detailed step-by-step execution belongs in the implementation plan that follows this spec.

## Open questions deferred to the plan

- Exact shape of the agent→hosting-peer DHT lookup zome function (existing link structure to build on; signature needs working out).
- Heartbeat cadence tuning — 60s today for doorway; whether peers in `Starting` or `Leaving` need faster cycles.
- Back-compat for in-flight zome calls when a peer transitions to `Leaving` mid-request.
- Whether `Maintenance` should distinguish operator-scheduled from peer-self-declared.

## References

- Existing peer status architecture: `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs` (DoorwayHeartbeat, HealthAttestation)
- Doorway status route: `doorway/doorway-service/src/routes/status.rs`
- Doorway route registry: `doorway/doorway-service/src/services/route_registry.rs`
- Device archetype plan: `genesis/plans/2026-04-13-device-archetypes-plan.md`
- Federated doorway health plan: `genesis/plans/2026-03-12-federated-doorway-health-plan.md`
- Holochain `AttachAppInterface` / `danger_bind_addr` — [docs.rs](https://docs.rs/holochain_conductor_api/latest/holochain_conductor_api/enum.AdminRequest.html)
- Holochain reverse-proxy convention — [forum.holochain.org](https://forum.holochain.org/t/does-holochain-expect-a-reverse-proxy/3025)
