# Federated Doorway Health — Design

## Goal

A public status page at each doorway (`/status`) showing self-reported health, peer attestations from the DHT, and shefa compute metrics. Federated peers keep each other honest by publishing what they observe — discrepancies are visible to everyone.

## Architecture

### Core Insight

We don't build new health metrics. Three existing systems already produce everything we need:

1. **Infrastructure DNA** — `DoorwayHeartbeat` (60s self-reports), `DoorwayHeartbeatSummary` (daily aggregates, kept forever), tier computation (Emerging/Established/Trusted/Anchor)
2. **Shefa compute** — `CustodianMetrics` (CPU, memory, storage, bandwidth, reputation), `EconomicEvents` (immutable audit trail of compute contribution with token issuance)
3. **Doorway `/health` endpoint** — public, lightweight, returns conductor/p2p/cache health

The one new piece: a `HealthAttestation` entry type in the infrastructure DNA where peer doorways publish "here's what I observed about doorway X."

### Trust Model

Self-reports + peer attestations + economic proof = federated honesty.

- **Self-reported heartbeats** — what a doorway says about itself (existing, every 60s)
- **Peer attestations** — what other doorways observe when they probe it (new, every 5 min)
- **Shefa compute metrics** — economic proof of actual work done (existing, hourly events)

When a doorway claims "I'm operational" but 3 out of 5 peers report "unreachable," the discrepancy is visible on every doorway's status page. No single operator controls the narrative.

## Data Flow

### Publishing Attestations

Doorway A's heartbeat loop already runs at 60s intervals. Every 5th iteration (~5 minutes), it also probes known federation peers:

```
Heartbeat loop (existing, 60s)
  → Self-report heartbeat to DHT (existing)
  → Every 5th cycle: probe each federation peer
    → HTTP GET peer's /health (existing endpoint, public)
    → Record response time + status + conductor health
    → Publish HealthAttestation to infrastructure DNA
    → DHT gossips attestation to all nodes
```

### New Infrastructure DNA Entry Type

```rust
HealthAttestation {
    attestor_doorway_id: String,     // Who observed (must match author's DoorwayRegistration)
    subject_doorway_id: String,      // Who was observed
    observed_status: String,         // "online" | "degraded" | "unreachable"
    response_time_ms: Option<u32>,   // How fast they responded
    conductor_healthy: Option<bool>, // Did /health show conductor connected?
    timestamp: String,               // When the probe happened
}
```

**Validation:** Attestor must be a registered doorway operator (same cryptographic author-check as `DoorwayHeartbeat`). You can't attest unless you're a real doorway.

**Retention:** Same pattern as heartbeats — detailed attestations kept 24h, daily summary kept forever. Summaries feed into long-term doorway reputation.

**Link type:** `DoorwayToAttestation` — from subject doorway's registration to attestations about it.

**Post-commit signal:** `HealthAttestationCommitted` — for projection into storage.

### Reading (Status Page)

```
GET /status page loads
  → Query infrastructure DNA for:
    1. Own DoorwayHeartbeat history (24h)
    2. Own DoorwayHeartbeatSummary (7-day rolling)
    3. HealthAttestations about self (what peers say about me)
    4. HealthAttestations about all known peers (network-wide view)
  → Query local shefa CustodianMetrics from storage (economic proof)
  → Render server-side HTML template
```

## Status Page Layout

Public, server-rendered HTML from doorway-service (Askama template). No JS dependencies. Fast load.

```
┌─────────────────────────────────────────────────┐
│  alpha.elohim.host                              │
│  Operational · 99.8% uptime (7d) · Anchor tier  │
├─────────────────────────────────────────────────┤
│                                                 │
│  ██████████████████████░░  7-day uptime bars    │
│  (green=up, yellow=degraded, red=down, gray=no data) │
│                                                 │
│  Response Time (24h)         Peers Agree: 4/5   │
│  p50: 42ms  p95: 180ms      Last attestation: 2m ago │
│                                                 │
├─────────────────────────────────────────────────┤
│  Components                                     │
│  ● Gateway         Operational                  │
│  ● Conductor Pool  Operational (3/3 workers)    │
│  ● Projection Cache  Operational (hit rate 94%) │
│  ● P2P Network     Operational (12 peers)       │
│  ● Storage         Operational                  │
├─────────────────────────────────────────────────┤
│  Federation Peers                               │
│                                                 │
│  beta.elohim.host    ● Operational  99.2% (7d)  │
│    Self: online · Peers: 3/4 agree · 38ms       │
│                                                 │
│  gamma.elohim.host   ● Degraded    94.1% (7d)   │
│    Self: online · Peers: 2/4 agree · 420ms  ⚠   │
│                                                 │
│  delta.elohim.host   ● Operational  99.9% (7d)  │
│    Self: online · Peers: 4/4 agree · 22ms       │
│                                                 │
├─────────────────────────────────────────────────┤
│  Compute Contribution (shefa)                   │
│  CPU: 4.2 hours · Storage: 128 GB·h · BW: 340 Mbps·h │
│  Tokens earned (24h): 1.84                      │
│  Steward tier: Guardian · Trust: 0.92           │
└─────────────────────────────────────────────────┘
```

### "Peers Agree" Indicator

The honesty signal. Compares self-reported status against peer attestations from the last probe cycle. Shows `N/M agree` where M is total attesting peers and N is how many saw the same status the doorway claims.

Discrepancies are always visible — a doorway cannot hide the fact that peers disagree with its self-report.

### Operator Expanded View

Authenticated operators (JWT cookie check in template) see additional sections on the same page:

- **Route Registry** — total routes, registered peers, source breakdown
- **Conductor Pool Detail** — per-worker health, cell count, memory pressure
- **Attestation Log** — raw attestation history ("beta probed us at 14:32, saw 42ms, reported online")
- **Peer Management** — add/remove federation peers, force refresh
- **Compute Event History** — full shefa economic event stream, token issuance breakdown

No separate route — same `/status` page with auth-gated expansion.

## Changes Required

### Infrastructure DNA

- `HealthAttestation` entry type + validation in integrity zome
- `record_health_attestation` + `get_attestations_for_doorway` coordinator functions
- `DoorwayToAttestation` link type
- `HealthAttestationCommitted` post-commit signal

### Doorway Service (Rust)

- **Peer probe loop** — piggyback on existing heartbeat task, probe federation peers every 5 minutes, publish attestations via zome call
- **`GET /status`** — server-side HTML template (Askama) combining self-heartbeats, peer attestations, shefa compute metrics
- **`GET /status.json`** — existing JSON status endpoint renamed for API consumers
- **Auth-gated template sections** for operators (JWT cookie check)

### Not Changed

- **doorway-app (Angular)** — operator dashboard already has its detailed view; status page is server-rendered
- **elohim-storage** — shefa compute metrics and CustodianMetrics already exist; status page reads them via existing API
- **No new database tables** — attestations live in the DHT
- **No new metrics collection** — reuses shefa compute and existing `/health`
- **No new probing infrastructure** — reuses existing `/health` endpoint

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| DHT attestations, not HTTP-only | Shared truth across all doorways. This is what Holochain is for. |
| Reuse shefa compute metrics | Economically grounded health data, not vanity metrics. Already collected. |
| Server-rendered HTML, no JS | Public page must load fast. No Angular bundle needed for a status page. |
| 5-minute probe interval | Balances freshness against DHT write volume. O(n^2) manageable at dozens of doorways. |
| Same page for public + operator | Simpler routing, auth-gated expansion via JWT cookie. |
| 24h detail + forever summaries | Matches existing heartbeat retention. Rolling 7-day view from summaries. |
