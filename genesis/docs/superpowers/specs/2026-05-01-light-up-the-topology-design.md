# Light Up the Topology — Operational Visibility Sprint Design

**Status:** Design (pre-implementation)
**Date:** 2026-05-01
**Predecessor:** [Light Up the Graph](2026-05-01-light-up-the-graph-design.md) (substrate orchestration; FeedbackSignal fan-out, reach-earning gate, Vouch primitive)
**Successor (planned):** Earning the Welcome (elohim-mediated discernment), VF-GraphQL projection at the shefa app layer

## Context

Phase 3.5 + Light Up the Graph shipped the trust-compute substrate live: FeedbackSignal arrival fans out, project_signal updates standing, the reach-earning gate is real, the Vouch primitive recovers Bob in the aunt-and-rage-bait integration test without mocks. The graph is built. The signals flow.

**The substrate is invisible.** A visitor cannot tell "this network actually works." A steward cannot see "my devices, my replication, my reciprocity." An operator cannot see their doorway's projection coverage. Without these views, the protocol's core resilience claim — *P2P hosting that doesn't go down* — is unverifiable from the surface.

This sprint surfaces five operational views that share one substrate query layer:

1. **Per-content distribution badge** — replicas + projectors + reach + health, visible to visitors with no auth (the trust signal that says "this network actually works")
2. **My cluster topology** — the steward's stewarded compute footprint, federated across their device bindings
3. **My peer topology** — cross-household replication / resilience graph
4. **My reciprocity aggregation** — committed-vs-delivered ledger over REA Commitments + EconomicEvents
5. **Doorway dashboard topology** — same substrate, viewed from doorway's operator vantage

Plus two substrate fixes that make the resilience demo land:

6. **GET-time peer-fallback for blobs** — page still loads when a peer goes offline
7. **On-connect replication kick** — bytes physically arrive when a peer comes online

## Sprint Goal

The demo lives or dies on one observable: **take a peer offline, replication count drops; bring it back online, count rises; page loads through it all.** Even visitors (no auth) see the count — that's the trust signal. Stewards see their multi-device cluster. Operators see their doorway. The same substrate composes all of it.

## Architecture

```
┌─── Holochain DHT (notarized truth) ─────────────────────────────────────┐
│                                                                          │
│   AgentPeerBinding (existing, Phase 2B Task A.2) — multi-device         │
│   Agent (existing, supersedes Human) — universal identity               │
│   FeedbackSignal · CustodianCommitment · REA Commitment · EconomicEvent │
│   Manifest · Vouch · Content · Path · Collective                        │
│                                                                          │
└─────────────────────────┬───────────────────────────────────────────────┘
                          │ post-commit signals
                          ▼
┌─── elohim-storage (libp2p data-ops + projection layer) ─────────────────┐
│                                                                          │
│   Existing tables (no schema changes):                                   │
│     custodian_blob_commitments  shard_locations  peer_status            │
│     peer_identity_bindings (binding projection, dht_anchor_hash)         │
│     rea_commitments  rea_economic_events  rea_projection                 │
│                                                                          │
│   NEW services (live aggregation; no materialized tables):               │
│     services/distribution_view.rs   compose Summary (inline) + Details  │
│     services/cluster_view.rs        federated steward cluster aggregator│
│     services/peer_topology_view.rs  federated cross-household graph     │
│     services/reciprocity_view.rs    SQL aggregation (no federation)     │
│                                                                          │
│   NEW libp2p protocol:                                                   │
│     /elohim/view-federation/1.0.0 — request-response codec             │
│       p2p/view_federation.rs (codec + handler)                          │
│                                                                          │
│   build_manifest() additions:                                            │
│     EXTEND existing EPR/content responses with                          │
│         distribution: Option<DistributionSummary>                        │
│     NEW  GET /api/v1/blob/{hash}/distribution/details (lazy)            │
│     NEW  GET /api/v1/cluster/me              (steward auth)             │
│     NEW  GET /api/v1/peer-topology/me        (steward auth)             │
│     NEW  GET /api/v1/reciprocity/me          (steward auth)             │
│                                                                          │
│   SUBSTRATE FIXES (independent of view layer; required for demo):       │
│     on-connect ListContent kick (cuts cold-peer gap from 60s+ to <10s)  │
│     GET-time peer-fallback for blobs (extends EPR cold-fetch pattern)   │
│                                                                          │
└─────────────────────────┬───────────────────────────────────────────────┘
                          │ HTTP via manifest-driven registry
                          ▼
┌─── doorway (web2 projection / single-target proxy) ─────────────────────┐
│                                                                          │
│   Existing manifest registry routes the four steward endpoints          │
│   automatically — no doorway code changes for those.                    │
│                                                                          │
│   NEW doorway-resident route (operator-only state, not in storage):      │
│     GET /admin/dashboard/topology  → DoorwayDashboardView               │
│       aggregates: federation peers, cache metrics, route registry,      │
│                   local view of connected storage stewards               │
│                                                                          │
└─────────────────────────┬───────────────────────────────────────────────┘
                          │ auth context propagated
                          ▼
┌─── elohim-app (Angular) ────────────────────────────────────────────────┐
│                                                                          │
│   Shared atomic components (elohim/components/):                         │
│     <distribution-badge>      reads parent payload, no fetch            │
│     <device-tile>             archetype-aware (per AgentPeerBinding)    │
│     <peer-household-card>     online/dark, reciprocity, hop hint        │
│     <commitment-bar>          committed-vs-delivered                    │
│     <diversity-hint>          reach-driven render                       │
│                                                                          │
│   Page-level surfaces (in their pillars):                                │
│     lamad/.../content-card           badge attaches inline              │
│     shefa/.../my-cluster             federated, polled on focus         │
│     shefa/.../peer-topology          federated, polled on focus         │
│     shefa/.../reciprocity-ledger     not federated                      │
│     doorway-app/.../operator-topology  operator surface                 │
│                                                                          │
│   Two-tier rendering: simple by default, [show details] reveals full    │
│   developer-grade decomposition. Same JSON payload underneath.           │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Key architectural decisions

1. **No new DHT entry types, no new SQLite tables.** All five views are live-aggregated queries over existing notarized + projected state. DNA capacity preserved (Lamad ~73/100, Mishpat 11/100).

2. **EPR-native composition for the badge case.** `distribution: DistributionSummary` rides on every existing EPR / ContentNode response. `<distribution-badge>` is presentation-only — no HTTP fetch on render. N+1 risk eliminated.

3. **Two-tier payload, single shape.** `DistributionSummary` (~100 bytes/item) is always inline. `DistributionDetails` (~1 KB) is lazy-fetched on tooltip-expand. Detail is a strict superset of summary.

4. **Distinct page-level endpoints for topology rollups.** Each fetched once per page visit, polled on focus (5s focused, 30s blurred).

5. **Multi-device humans are first-class** via existing AgentPeerBinding (notarized; Phase 2B Task A.2). The auth context resolves to a single `agent_cid`; bindings expand to a peer set. Cluster + peer-topology federate across the binding set. Reciprocity reads DHT-projected tables (no federation). Badge composes single-instance with bindings lookup (no federation).

6. **Federation is best-effort with cryptographic gating.** Each slice is signed by the responding peer's agent key, verifiable against the requesting agent's bindings. Offline peers return `online: false stale_since: <last_known>`; partial federation is the expected steady state.

7. **Diversity axis is reach-driven** (visitor-safe by design):
   - public reach → region/metro tier (CDN-flavored, anonymized)
   - collective reach → member count
   - intimate / household reach → device archetypes named (already inside the trust circle)
   - private → no diversity, just count
   Doorways are the geographic projection surface; visitors see doorway aggregation, not raw household placement.

8. **Device archetype is first-class; deployment topology is implementation detail.** Views render `node | desktop | mobile | steward` (per the AgentPeerBinding `DeviceArchetype` enum). No "shem-rack-02", no WireGuard, no node affinity in user-facing surfaces. k8s is a dev convenience, not a substrate concept. Story-driven device states (asleep, syncing, dark) are modeled in a2o.

9. **Two view tiers (simple / detailed) over one payload.** Simple is the default for everyday users (Google Drive-flavor: counts, friendly labels, status). Detailed is opt-in for elohim agents, developers, operators. Same JSON underneath.

10. **Doorway dashboard route lives in doorway, not storage.** It's about doorway's own operational state (cache, federation, DNS/TLS). The four steward routes live in `elohim-storage::build_manifest()` per `project_doorway_manifest_driven_routes`.

## P2P Design Gate Output

### Entity: AgentPeerBinding (existing, load-bearing)
- **Classification**: Notarized (A) — already exists, Phase 2B Task A.2
- **Address**: Content-Derived (CID via canonical_bytes signing body)
- **Source of Truth**: Holochain DHT (imagodei zome)
- **Coordinator Zome**: `imagodei::create_agent_peer_binding`, `get_agent_peer_bindings(agent_cid)`, `get_bindings_for_peer(peer_id)`
- **Storage Projection**: `peer_identity_bindings` (with `dht_anchor_hash`)
- **Pre-flight**: verify bindings are being created in seeder + runtime; if not, that's a substrate dep that has to land first
- **Anti-Pattern Check**: ✓ existing entry type reused; ✓ device_archetype carries the archetype taxonomy

### Entity: DistributionSummary (per-CID badge payload, inline)
- **Classification**: Operational (C) — materialized view derived from notarized sources
- **Justification**: Aggregates replicas (custodian + ShardLocation), projectors (rea_projection signal acks), reach class (FeedbackSignal-projected). Composed live at query time.
- **Address**: N/A — endpoint takes CID input; view itself not addressed
- **Source of Truth**: DHT (replicas, projectors, reach all DHT-notarized)
- **Coordinator Zome**: N/A (read-only projection)
- **Storage Projection**: Computed live from `custodian_blob_commitments`, `shard_locations`, `rea_projection` state, `content_store` entries, `peer_identity_bindings`
- **HTTP Route**: composed onto existing EPR / content responses; no standalone route for summary
- **Anti-Pattern Check**: ✓ no new entry type; ✓ source-of-truth declared per dimension; ✓ HTTP composition is thinnest projection layer

### Entity: DistributionDetails (lazy-fetched per-CID developer view)
- **Classification**: Operational (C)
- **Justification**: Same data sources as Summary; richer projection (full peer list, projector identities, placement gaps).
- **Source of Truth**: DHT
- **HTTP Route**: `GET /api/v1/blob/{hash}/distribution/details` (auth: optional — anonymous gets visitor-safe subset)
- **Reconstruction**: re-aggregate from sources

### Entity: MyClusterView (steward's federated compute footprint)
- **Classification**: Operational (C) — federated query result
- **Justification**: Snapshot of "the devices this steward operates" plus what each hosts. Composed via libp2p view-federation across the steward's AgentPeerBindings.
- **Source of Truth**: DHT (binding set notarized) + each device's live state (per-peer)
- **Coordinator Zome**: N/A
- **Storage Projection**: live aggregation, no materialization
- **HTTP Route**: `GET /api/v1/cluster/me` (auth: steward)
- **Anti-Pattern Check**: ✓ no standalone agent-state table; ✓ steward identity from auth, not stored; ✓ federation gated by DHT-anchored bindings

### Entity: PeerTopologyView (cross-household replication / resilience)
- **Classification**: Operational (C) — partially federated
- **Justification**: Edge-aggregation across the steward's bindings; each binding's storage instance reports its connected peer set, aggregator dedupes by household.
- **Source of Truth**: DHT (bindings, custodian commitments) + libp2p (per-peer connectivity)
- **HTTP Route**: `GET /api/v1/peer-topology/me` (auth: steward)
- **Anti-Pattern Check**: ✓ edge derivation from notarized commitments

### Entity: ReciprocityView (committed-vs-delivered)
- **Classification**: Operational (C)
- **Justification**: Pure SQL aggregation over DHT-projected REA Commitment + EconomicEvent tables. Filtered by the steward's binding agent set. No federation — DHT is authoritative; reads are cheap.
- **Source of Truth**: DHT — REA Commitment + EconomicEvent (existing in `content_store` zome, projected via `rea_projection.rs`)
- **Coordinator Zome**: existing (already shipped)
- **HTTP Route**: `GET /api/v1/reciprocity/me` (auth: steward)
- **Anti-Pattern Check**: ✓ reuses existing entry types and projection signals

### Entity: DoorwayDashboardView (operator vantage)
- **Classification**: Operational (C) — doorway-local aggregation
- **Justification**: Doorway-resident operational state (cache, federation, route registry, DNS/TLS). Doorway is web2 projection per `project_three_layer_truth_model`.
- **Source of Truth**: Mixed — peer connectivity (libp2p), cache metrics (operational counters), federation peers, route registry
- **HTTP Route**: `GET /admin/dashboard/topology` (auth: operator/admin)
- **Anti-Pattern Check**: ✓ doorway-resident operational state; ✓ federation queries reuse existing routes

### Entity: ViewFederationRequest / ViewFederationResponse (libp2p protocol)
- **Classification**: Operational (C) — protocol messages, not stored
- **Protocol**: `/elohim/view-federation/1.0.0` — request-response codec, msgpack-encoded (per existing protocol pattern)
- **Wire shape**:
  - Request: `{ view_kind: "cluster" | "peer-topology", agent_cid, request_id }`
  - Response: `{ view_kind, agent_cid, slice: ViewSlice, signed_by_responder_agent_key }`
- **Authz**: responder MUST be in the requester's binding set (verified against AgentPeerBinding signatures); responder signs slice with its agent key
- **Payload cap**: 256 KB (DOS protection at codec layer)
- **File**: `elohim/elohim-storage/src/p2p/view_federation.rs`

### Design Constraints Discovered

1. **Doorway-resident vs storage-resident split**: visitor-facing distribution endpoint composes onto existing EPR/content responses (storage-resident). Steward `/me` endpoints live in `elohim-storage::build_manifest()`. Doorway-only operator dashboard route lives in doorway (it's about doorway-local state).

2. **Steward auth context dependency**: `/cluster/me`, `/peer-topology/me`, `/reciprocity/me` need the authenticated `agent_cid` from the auth context. Doorway's `/auth/portal-host` flow (commit `ec0d370a`) provides session shape; verify the `agent_cid` is forwarded through the registry-routed proxy path.

3. **Federation protocol payload cap**: 256 KB per slice. ViewSlice for a single device cannot exceed this (typical: tens of KB).

4. **Live-vs-cached freshness tradeoff**: at alpha-cluster scale (~6 peers), live aggregation per request is fine. Beyond ~100 peers, a refresh-on-signal cache may be needed. Mark as future optimization; not in this sprint.

5. **Reach-driven diversity rendering** is a presentation concern. JSON includes both household identifiers and region/metro tier; the simple-tier render picks one based on reach class.

6. **Existing tables are sufficient**. No migrations for the projection layer. Operational counters for substrate fixes (kick fired, fallback success/fail) may be added as in-process metrics — not entities.

## Components

### New Rust files

**elohim-storage**
```
src/services/
  distribution_view.rs       compose DistributionSummary (inline) + DistributionDetails (lazy)
  cluster_view.rs            MyClusterView aggregator (federated)
  peer_topology_view.rs      PeerTopologyView aggregator (federated)
  reciprocity_view.rs        ReciprocityView aggregator over REA tables (no federation)

src/api/
  distribution.rs            handler for GET /api/v1/blob/{hash}/distribution/details
  cluster.rs                 handler for GET /api/v1/cluster/me
  peer_topology.rs           handler for GET /api/v1/peer-topology/me
  reciprocity.rs             handler for GET /api/v1/reciprocity/me

src/p2p/
  view_federation.rs         request-response codec + per-slice signing
```

**doorway**
```
src/services/dashboard_topology.rs    aggregator over federation peers, cache metrics, route registry
src/routes/admin/dashboard_topology.rs handler for GET /admin/dashboard/topology
```

### Modified Rust files

```
elohim-storage/src/views.rs                     + DistributionSummary, DistributionDetails,
                                                  MyClusterView, PeerTopologyView, ReciprocityView,
                                                  + sub-types (ReplicaPeer, ProjectorIdentity,
                                                    DiversityHint, PeerHouseholdEdge, ViewSlice,
                                                    Freshness, etc) — all #[derive(TS, Serialize)]
                                                  with camelCase

elohim-storage/src/api/epr.rs                   extend EPR head response with
  (or epr_atom_protocol)                          `distribution: Option<DistributionSummary>`

elohim-storage/src/http.rs                      build_manifest() registers 4 new routes;
                                                  GET /blob/{hash} on local-miss now follows
                                                  EPR cold-fetch pattern (substrate fix)

elohim-storage/src/p2p/mod.rs                   ConnectionEstablished handler (~line 2117):
                                                  jittered immediate ListContent kick
                                                  (substrate fix); P2PCommand variants for
                                                  ViewFederate

elohim-storage/src/services/epr_store.rs        extract peer-fallback helper, share with
                                                  blob fallback path

elohim-storage/src/services/auth.rs (or         auth context resolution: agent_cid from
  wherever auth context lives)                    session/header → bindings lookup helper

doorway/doorway-service/src/server/http.rs      match arm for /admin/dashboard/topology

doorway/doorway-service/src/services/           existing services exposed via the new
  cache_metrics, federation, route_registry     aggregator (no behavior change, just surfaced)
```

### Schemas (schema-first IoC)

```
elohim/sdk/schemas/v1/views/
  distribution-summary.schema.json
  distribution-details.schema.json
  my-cluster-view.schema.json
  peer-topology-view.schema.json
  reciprocity-view.schema.json
  doorway-dashboard-view.schema.json
  view-slice.schema.json                        (federated slice base)
  freshness.schema.json                         (live | stale_<ms> | offline | cached_offline)
```

Plus contract test entries in `elohim/elohim-storage/tests/schema_contract.rs` and registration in `elohim/sdk/schemas/scripts/codegen-ts.mjs::INTERFACE_FILES`.

### Auto-generated TS types

`ts-rs` regenerates view types into `elohim/sdk/storage-client-ts/src/generated/`. DoorwayDashboardView lives in doorway-client-ts. Run `cargo test export_bindings` after Rust changes.

### New Angular shared atoms (elohim-app)

```
app/elohim-app/src/app/elohim/components/
  distribution-badge/                renders badge state, tooltip with simple/detail tiers
  device-tile/                       archetype-aware (node, desktop, mobile, steward)
  peer-household-card/               online/dark, reciprocity, hop hint
  commitment-bar/                    committed-vs-delivered visualization
  diversity-hint/                    reach-driven (region/metro for public,
                                       device-archetype for intimate, member-count for
                                       collective)
```

### New Angular page-level components + services

```
app/elohim-app/src/app/shefa/services/
  cluster.service.ts                 fetches MyClusterView, polls on focus
  peer-topology.service.ts           fetches PeerTopologyView, polls on focus
  reciprocity.service.ts             fetches ReciprocityView, polls on focus

app/elohim-app/src/app/shefa/pages/
  my-cluster/my-cluster.component.ts
  peer-topology/peer-topology.component.ts
  reciprocity-ledger/reciprocity-ledger.component.ts

app/elohim-app/src/app/elohim/services/
  distribution.service.ts            lazy-fetches DistributionDetails on tooltip-expand

doorway/doorway-app/src/app/...      new operator-topology pane on existing admin dashboard
```

### Modified Angular files

```
app/elohim-app/src/app/elohim/adapters/         ContentNode adapter surfaces
                                                  EPR envelope's `distribution` field

app/elohim-app/src/app/lamad/.../content-card   embed <distribution-badge>

app/elohim-app/src/app/app.routes.ts            +3 shefa routes
                                                  (/cluster, /peers, /reciprocity)

doorway-app routing                              + /admin/topology pane
```

### Substrate fixes (independent track in same sprint)

```
elohim-storage/src/p2p/mod.rs                  on-connect replication kick (jittered ListContent)
elohim-storage/src/http.rs                     GET-time peer-fallback for blobs
elohim-storage/src/services/epr_store.rs       shared fallback helper extraction
elohim-storage/tests/                          regression: filesystem-count parity post-replication
                                                 after kill-peer-bring-peer-back cycle
```

### What's NOT touched

- No DNA changes (no new entry types, no zome modifications)
- No new SQLite migrations (live aggregation over existing tables)
- No doorway proxy logic changes for steward routes (registry handles them)
- Sophia, holochain runtime, conductor: untouched

## Data Flow

### Flow 1 — Visitor loads a content page (no auth)

```
Browser ── GET /api/v1/content/{slug}
              │
              ▼
        elohim-storage::api::content
              │
              ├─ load ContentNode + EPR head
              ├─ compose distribution: DistributionSummary {
              │     replicaCount     = COUNT(custodian_blob_commitments
              │                              WHERE blob_hash = ?
              │                              AND status IN ('healthy','probing'))
              │     replicaTarget    = sharding policy → RS-target
              │     replicaHealth    = bucket(replicaCount / replicaTarget)
              │     projectorCount   = COUNT(rea_projection.projector_acks
              │                              WHERE cid = ?)
              │     reachClass       = from EPR envelope
              │     diversityHint    = match reachClass:
              │                          public → top-N region tier
              │                          collective → member-count
              │                          intimate → device archetypes named
              │                          private → none
              │     thisFetchSource  = "projected-via-doorway"
              │     lastVerifiedSec  = max(custodian.last_verified_at)
              │     myRole / reciprocityHint = None  (no auth)
              │ }
              ▼
        Response: { ContentNode, ..., distribution: DistributionSummary }
```

### Flow 2 — Steward loads same content page (authenticated)

```
Browser ── GET /api/v1/content/{slug}        (auth: agent_cid M)
              │
              ▼
        elohim-storage::api::content
              │
              ├─ resolve agent's binding set (single SQL):
              │     SELECT peer_id, device_archetype FROM peer_identity_bindings
              │     WHERE agent_cid = M AND superseded_by IS NULL
              │   → [P1 desktop, P2 node, P3 mobile]
              │
              ├─ compose distribution as Flow 1, plus steward fields:
              │     myRole = match (
              │       any(P1,P2,P3) ∈ custodian replica set     → 'replica'
              │       any(P1,P2,P3) ∈ rea_projection.acks       → '+projector'
              │       sole replica                                → 'sole-replica'
              │       neither                                     → 'not-hosting'
              │     )
              │     reciprocityHint = SUM(commitments-out-by-bindings)
              │                     − SUM(commitments-in-from-others)
              ▼
        Response: { ..., distribution: DistributionSummary{ ..., myRole, reciprocityHint } }
```

### Flow 3 — Steward opens tooltip → expand → details

```
Browser ── GET /api/v1/blob/{hash}/distribution/details   (auth optional)
              │
              ▼
        elohim-storage::api::distribution
              │
              ├─ load DistributionDetails {
              │     replicaPeers: full list with archetype, last-seen, hop-est
              │     projectorIdentities: doorway hostnames, last signal_ack
              │     placementGaps: from custodian.placement_gaps
              │     recentProjectionEvents: rea_projection signal stream window
              │     // steward-only:
              │     reciprocityEdges: per-peer hostedByMe/hostedByThem counts
              │     commitmentReferences: matching CustodianCommitment CIDs
              │ }
```

### Flow 4 — Steward navigates to /cluster page (FEDERATED)

```
Browser ── GET /api/v1/cluster/me                       (auth: agent_cid M)
              │
              ▼
        elohim-storage::api::cluster
              │
              ├─ resolve bindings (local SQL): [P1, P2, P3]
              │
              ├─ for each binding:
              │     spawn libp2p view-federation query in parallel
              │     P2PCommand::ViewFederate {
              │         target: peer_id,
              │         req: ViewFederationRequest {
              │             view_kind: "cluster",
              │             agent_cid: M,
              │             request_id: uuid,
              │         },
              │         timeout: 3000ms,
              │     }
              │
              ▼
        ┌──────────── parallel libp2p RPC ────────────┐
        │                                              │
   ┌────▼─── P1 (desktop)                         ────▼─── P3 (mobile)
   │   storage instance                           │   STORAGE OFFLINE
   │   handles ViewFederationRequest              │   timeout @ 3s
   │   → builds local cluster slice:              │
   │     ViewSlice {                              │   → returns cached
   │       peer_id: P1,                           │     last-known slice
   │       archetype: desktop,                    │     with stale_since
   │       online: true,                          │
   │       storage_used_bytes: ...,               │
   │       hosting_count: ...,                    │
   │       projecting_count: ...,                 │
   │       beacon_age_ms: 0,                      │
   │       freshness: live,                       │
   │     }                                        │
   │   signs slice with P1's agent key            │
   │   returns ViewFederationResponse             │
   └────────────────────────────────────────────┘
              │
              ▼
        aggregator:
          - validates each response signed by responder's agent_key
          - validates responder is in M's binding set
          - merges slices; offline peer marked with stale_since
          - composes MyClusterView { devices, totals, freshness }
              │
              ▼
        Response → Browser renders <my-cluster> page
```

Best-effort: 3000ms per-peer timeout, parallel; offline peers mark `online:false stale_since:<lastSeen>`. Partial federation is the expected steady state.

### Flow 5 — Steward navigates to /peer-topology page (FEDERATED, partial)

```
Same shape as Flow 4, view_kind = "peer-topology". Each peer's slice includes
its own peer set:
  ViewSlice {
    peer_id: P1,
    connected_peer_households: [
      { household_id: adam-household,
        online: true,
        last_sync_sec: 12,
        my_cids_hosted_by_them: 7,
        their_cids_hosted_by_me: 12,
        ... },
      ...
    ],
  }

aggregator: union peer sets across M's bindings; dedupe by household_id;
            edge-counts summed across bindings. Resilience-cliff calc:
            for each foreign household, identify "sole-external-replica"
            CIDs (CIDs hosted only on M's bindings + that one household).
```

### Flow 6 — Steward navigates to /reciprocity page (NOT FEDERATED)

```
Browser ── GET /api/v1/reciprocity/me                   (auth: agent_cid M)
              │
              ▼
        elohim-storage::api::reciprocity
              │
              ├─ resolve bindings: [P1, P2, P3]
              │
              ├─ pure SQL aggregation over DHT-projected tables:
              │     SELECT
              │       counterparty_agent_cid,
              │       SUM(committed) AS committed,
              │       SUM(delivered) AS delivered
              │     FROM rea_commitments c
              │     LEFT JOIN rea_economic_events e ON e.commitment_id = c.id
              │     WHERE c.committer_agent_cid IN (M's bindings)
              │        OR c.beneficiary_agent_cid IN (M's bindings)
              │     GROUP BY counterparty
              │
              ▼
        Response → ReciprocityView { inflow, outflow, capacity }
```

REA Commitments + EconomicEvents are DHT-resident and gossipped; any peer with the projection answers fully. DHT is authoritative; reads are cheap.

### Flow 7 — Operator dashboard (doorway-resident, not federated)

```
Browser ── GET /admin/dashboard/topology                (auth: operator)
              │
              ▼
        doorway::routes::admin::dashboard_topology
              │
              ├─ aggregate from doorway-local services:
              │     federation_peers (existing service)
              │     route_registry (existing)
              │     cache_metrics (existing)
              │     connected_storage_stewards (libp2p peer list — doorway sees its
              │                                  own stewards, not the human's bindings)
              ▼
        Response → DoorwayDashboardView
```

### Flow 8 — Substrate fix: peer goes offline → page still loads

```
Visitor browser ── GET /blob/{hash}
                       │
                       ▼
                 doorway → forward to storage (single-target, manifest-routed)
                       │
                       ▼
                 elohim-storage::http::handle_blob   (EXTENDED on local miss)
                       │
                       ├─ blob_store.exists(hash)?
                       │     YES → return bytes from local pantry
                       │     NO  → (NEW PATH — extends EPR cold-fetch pattern)
                       │           ├─ kad.get_providers(content_routing_key(hash))
                       │           ├─ for each provider in best-effort order:
                       │           │     issue libp2p shard_protocol::Get { hash }
                       │           │     timeout 1.5s
                       │           │     on Data response → verify hash → write
                       │           │     pantry → return
                       │           ├─ all failed → 502 with structured error
```

### Flow 9 — Substrate fix: peer comes online → bytes arrive in seconds

```
swarm event: ConnectionEstablished { peer_id }
                       │
                       ▼
            handler in p2p/mod.rs (EXTENDED)
                       │
                       ├─ existing: trust + identity handshakes
                       ├─ NEW: jittered (0–2s random delay)
                       │       schedule single ListContent { peer_id, limit: 5000 }
                       │       through normal request-response path
                       │       (skip if peer disconnected during jitter)
                       │       (global cap: 16 in-flight kicks)
                       │
                       ▼
            response: ContentList → existing discover() path → gap_queue
                       │
                       ▼
            within 5s: drain_gap_queue → GetContent → Get(blob_hash) → bytes flow
```

Net effect: cold-peer-to-first-byte latency drops from up-to-65s to <10s.

## Error Handling

### Federation errors

- **Timeout**: 3000ms per-peer wallclock; exceeded → mark slice `online: false, stale_since: <last_known_beacon>`. If recent (<5 min), use cached slice. Never error the whole response.
- **Signature mismatch**: responder's signature does not validate against the AgentPeerBinding's recorded agent_cid → reject the slice, log a warning, mark device `unverifiable` in response. Do not federate again to that peer this request.
- **Responder not in binding set** (defensive): responder peer_id must match a binding entry. Mismatch → reject as above.
- **Partial success**: at least one slice → 200 with per-slice `freshness`. All offline → 200 with `freshness: all_offline`, body composed from binding metadata only. Never 5xx.
- **Federation aggregator panic**: wrapped in `tokio::spawn`; panic in one slice handler does not bring down the request.

### Auth / identity resolution errors

- **Missing auth on `/me` endpoints**: 401 with `{ reason: 'auth_required' }`. No anonymous fallback.
- **Auth context resolves to no Agent**: 500 with `{ reason: 'auth_resolution_failed' }`. Log full context.
- **Bindings lookup empty** (fresh agent): 200 with empty-state view + `reason: 'no_bindings_yet'`.
- **Superseded bindings filter**: query uses `superseded_by IS NULL`; defensive double-check before federation.

### Substrate fix errors (peer-fallback)

- **Kad providers empty**: 404 with `{ reason: 'unavailable_no_providers' }`.
- **All providers timeout**: 502 with `{ reason: 'all_providers_timed_out', tried: N }`.
- **Provider returns mismatched bytes**: discard, try next, increment metric.
- **Pantry write fails after fetch**: serve bytes anyway; do not 5xx.

### Substrate fix errors (on-connect kick)

- **Kick fails**: log at `info!`, do nothing else. Existing 60s loop catches up.
- **Jitter outlives connection**: check `is_connected` before firing.
- **Thundering herd**: jitter + global cap of 16 in-flight kicks.

### View composition errors

- **DHT projection lag**: aggregation may be momentarily stale; response carries `lastVerifiedSeconds`.
- **Missing rea_projection state for CID**: `projectorCount: 0`. Honest answer.
- **Inconsistent custodian + shard_locations**: aggregation queries the union (any-evidence). Marked in code comment + spec.

### Failure-mode invariant

A 200 OK on a federated view means *some* truthful state was assembled. Per-slice `freshness` annotations carry the honest information. Consumers must render `freshness` in the simple-tier view. Hiding stale data behind a 200 without freshness annotation would be lying.

### HTTP semantics summary

| Outcome | Status | Body shape |
|---|---|---|
| Federation partial success | 200 | view + per-slice `freshness` |
| Federation total miss (all offline) | 200 | view from binding metadata + `freshness: all_offline` |
| Auth missing on `/me` route | 401 | `{ reason }` |
| Auth resolves but no agent | 500 | `{ reason }` |
| Bindings empty | 200 | empty-state view |
| Blob unavailable (no providers) | 404 | `{ reason: 'unavailable_no_providers' }` |
| Blob unavailable (timeout) | 502 | `{ reason: 'all_providers_timed_out' }` |
| Federation signature reject | 200 | slice marked `unverifiable` |
| Cache write fails post-fetch | 200 | bytes returned anyway |
| Manifest absent | 200 | falls through to default policy |

## Testing

### Unit tests (Rust, in elohim-storage)

**`services/distribution_view.rs`** — composes summary correctly for all reach classes; diversity hint dispatches by reach; `myRole` resolution (4 cases × authenticated agent has 1, 2, 3 bindings); `reciprocityHint` math; `lastVerifiedSeconds` reflects most recent custodian probe; empty bindings → public fields render, steward fields null; `thisFetchSource` correctly identifies projected vs peer-direct vs local-pantry.

**`services/cluster_view.rs` + federation aggregator** — 3 bindings all online → 3 live slices; 1 offline → marked `stale_since`; 1 mismatched signature → `unverifiable`, others render normally; all timeout → 200 with bindings-only metadata + `freshness: all_offline`; slice handler signs with peer's agent key; verifier validates; aggregator serialization round-trip (msgpack).

**`p2p/view_federation.rs` codec** — request/response framing; payload size cap (256 KB); malformed request rejected; unknown view_kind rejected.

**`services/peer_topology_view.rs`** — federated edge dedup; edge math symmetry; offline household correctly marked; resilience-cliff calculation (sole-external-replica detection).

**`services/reciprocity_view.rs`** — SQL aggregation; filters by binding agent set; over-delivered flagged; honored % calculated correctly; capacity-available math.

**Substrate fixes** — on-connect kick (jitter respected; skipped on mid-jitter disconnect; global cap not exceeded; failure logged); blob fallback (no providers → 404 structured; first provider succeeds → 200 hash-verified; all timeout → 502 with `tried: N`; wrong bytes → discard + try next).

### Schema contract tests

Add cases for all new views + sub-types in `elohim-storage/tests/schema_contract.rs`. Test asserts `serde_json::to_value(rust_struct) == json_schema_validation_passes(...)`.

```
pnpm run schema:codegen:ts
pnpm run schema:validate
```

`INTERFACE_FILES` extended in `elohim/sdk/schemas/scripts/codegen-ts.mjs`.

### Integration tests (alpha cluster — Jenkins regression)

Per `feedback_shift_measure_jenkins`: shift measures live in Jenkins.

**Test A — Multi-device cluster view federation**
```
Setup: 6-peer alpha cluster up; matthew has 3 AgentPeerBindings
       (P1 desktop, P2 node, P3 mobile)
Step 1: GET /api/v1/cluster/me as matthew → expect 3 devices, all freshness=live
Step 2: SIGSTOP matthew's mobile pod (P3)
Step 3: wait > federation_timeout (3s) + 1s
Step 4: GET /api/v1/cluster/me → expect P3 freshness=offline,
        P1+P2 still live
Step 5: SIGCONT mobile pod
Step 6: wait < 60s
Step 7: GET /api/v1/cluster/me → expect all 3 freshness=live again
```

**Test B — Resilient delivery (the demo)**
```
Setup: alpha cluster running; seed CID X with replication target 4
Step 1: assert distribution: replicaCount=4, replicaHealth=healthy
Step 2: SIGSTOP 2 peers hosting X
Step 3: GET /blob/X (no auth) → expect 200, bytes match expected hash
        (peer-fallback path triggered)
Step 4: assert distribution: replicaCount=2, replicaHealth=at-risk
Step 5: SIGCONT 2 peers
Step 6: wait < 30s
Step 7: assert distribution: replicaCount=4, replicaHealth=healthy
```

**Test C — Cold peer to first byte**
```
Setup: alpha cluster running with seeded content; 7th peer fresh
Step 1: start 7th peer (raspberry-pi-4 archetype)
Step 2: timestamp = on libp2p ConnectionEstablished
Step 3: poll filesystem of 7th peer; first blob arrival timestamp
Step 4: assert (arrival - timestamp) < 10s
```

These three are the load-bearing demo measures.

### Frontend tests (Cypress / BDD)

```
genesis/a2o/features/topology/
  visitor-sees-replica-count.feature
  steward-sees-multi-device-cluster.feature
  badge-tooltip-simple-detail-toggle.feature
  reciprocity-view-renders-inflow-outflow.feature
  doorway-dashboard-renders-operator-vantage.feature
```

Page object selectors keyed by `data-testid` per `page-model` skill.

Critical scenarios:
- Visitor loads content page → distribution badge shows N replicas without auth, no PII
- Steward loads same page → tooltip shows myRole + reciprocityHint
- Steward expands tooltip → details lazy-fetched, full peer list rendered
- Steward navigates to /cluster → all devices shown; offline device greyed with `stale_since`
- Steward toggles `[show details]` → developer-tier renders
- Doorway operator loads /admin/topology → operator-only fields visible

### A2O scenario harvest

Per `story-harvest` skill, this sprint discovers parameter-bearing engineering constraints worth preserving:

- Federation timeout = 3000ms
- Replication target per blob (sharding policy)
- Cold-peer-to-first-byte latency target (10s)
- Bindings filter: `superseded_by IS NULL`
- view-federation/1.0.0 payload cap = 256 KB

Harvested scenarios go in `genesis/a2o/features/topology/` paired with implementation.

### Pre-flight verification (must run before sprint kickoff)

1. **AgentPeerBindings being created**: check `genesis/seeder/src/seed-agent-bindings.ts` (or equivalent) is running on `pnpm hc:start:seed`. If not, that has to land first.
2. **Auth context propagation**: verify the `agent_cid` is forwarded through the manifest-routed proxy path to storage handlers.
3. **rea_projection signal stream populated**: verify projector acks are landing.

Pre-flight failures → fix or descope before scope-locking.

#### Pre-flight verification — T00 results (static, 2026-05-01)

**Status: DONE_WITH_CONCERNS — gaps logged for T01-T03 to resolve.**

Static verification performed in Eclipse Che (no live Holochain stack available). Steps requiring `pnpm run hc:start:seed`, `curl /api/v1/agents`, or `sqlite3 ~/.elohim-storage/storage.db` are deferred to Jenkins or local-dev runtime verification.

**What is in place (static evidence):**

- DNA layer (imagodei DNA): coordinator `create_agent_peer_binding` and queries `get_agent_peer_bindings` / `get_bindings_for_peer` exist at `elohim/holochain/dna/imagodei/zomes/imagodei/src/agent_peer_binding.rs`. Integrity entry + validator at `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/agent_peer_binding.rs`. Sweettest coverage at `elohim/holochain/tests/sweettest/src/tests/imagodei_peer_binding.rs`.
- DnaSignal projection: `ImagodeiSignal::AgentPeerBindingCreated` is emitted on commit. `HolochainAppSignalStream` (Task A.11) translates this to a storage projection event.
- Schemas (notarized): `elohim/sdk/schemas/v1/objects/agent-peer-binding.schema.json`, `elohim/sdk/schemas/v1/views/agent-peer-binding-view.schema.json`, `elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json`, `elohim/sdk/schemas/v1/enums/device-archetype.schema.json`.
- TS bindings generated: `elohim/sdk/storage-client-ts/src/generated/AgentPeerBindingView.ts`, `app/elohim-app/src/app/generated/agent-peer-binding-view.ts`.
- Storage projection table: migration `elohim/elohim-storage/migrations/2026-04-24-235000_peer_identity_bindings/up.sql`. Diesel schema and CRUD at `elohim/elohim-storage/src/db/peer_identity_bindings.rs`.

**Gap 1 (BLOCKING for runtime): no AgentPeerBinding seeder.** Verbatim grep of `genesis/seeder/src/` for `AgentPeerBinding | agent_peer_binding | create_agent_peer_binding | peer_identity_binding` returned zero matches. No seeder file exists. The two device-related files (`genesis/seeder/src/generate-devices-json.ts`, `genesis/seeder/src/validate-devices.ts`) operate on the device *catalog* (`genesis/data/devices/*.md` -> `devices.json`) — a content-side reference, not an AgentPeerBinding writer. `seed.ts` does not call any binding seeder. **`pnpm run hc:start:seed` will produce zero AgentPeerBindings today.** This must be addressed by T01-T03 before Phases 4 and 7+ can demonstrate multi-device behaviour.

**Gap 2 (schema drift): projection table missing columns the wire view exposes.** The `peer_identity_bindings` projection table columns are: `peer_id, agent_cid, dht_anchor_hash, valid_from, valid_until, observed_at, source` (PRIMARY KEY `(peer_id, dht_anchor_hash)`). The DHT entry and `AgentPeerBindingView` (wire) carry two additional fields the projection drops: `device_archetype` and `superseded_by`. Step 3 of the kickoff prompt assumed these columns exist (`SELECT agent_cid, peer_id, device_archetype FROM peer_identity_bindings WHERE superseded_by IS NULL ...`). Today that SQL would fail with `no such column`. The DNA-side data is fine; the projection just drops it. T01-T03 should add an ALTER TABLE migration so subsequent phases can filter on archetype and current-binding-only.

**Runtime verification still required (defer to Jenkins / local-dev):**

- Step 2: `pnpm run hc:start:seed` followed by `curl http://localhost:8090/api/v1/agents | jq '.[] | select(.bindings != null) | {id, bindings: .bindings | length}'` — expect at least one agent with 2+ bindings (multi-device demo). Cannot run from Eclipse Che.
- Step 3: `sqlite3 ~/.elohim-storage/storage.db "SELECT agent_cid, peer_id, device_archetype FROM peer_identity_bindings WHERE superseded_by IS NULL LIMIT 10;"` — blocked on Gap 2 (column not present); blocked on Gap 1 (no rows would exist anyway). Re-run after T01-T03 land.
- Pre-flight item 2 (auth `agent_cid` propagation): not in T00 scope, separate task.
- Pre-flight item 3 (`rea_projection` stream populated): not in T00 scope, separate task.

#### Pre-flight verification — T01 results (static, 2026-05-01)

**Status: DONE — outcome = "not wired" through the proxy. Adds a T28 prerequisite for T22 (`bindings_resolver`).**

Static verification only (Eclipse Che, no live stack). Step 2 (live `curl /api/v1/health` round-trip) defers to Jenkins / local-dev. Step 1 (grep) plus targeted reads of the JWT, storage_proxy, and account-handler files are sufficient for the design decision.

**Step 1 grep result.** `grep -rn "x-agent-cid\|X-Agent-Cid\|agent_cid"` against `doorway/doorway-service/src/` returned **zero matches**. The same grep against `elohim/elohim-storage/src/` returned 41 matches — all referring to `agent_cid` as a *DHT content-address field* on entries (`AgentPeerBinding.agent_cid`, `EprHead.agent_cid`, `SignalEmitIntent.agent_cid`, etc.), never as an HTTP header. **No code anywhere reads, writes, or forwards an `X-Agent-Cid` header.**

**What the JWT carries.** `doorway/doorway-service/src/auth/jwt.rs:22-58` defines `Claims` with `human_id`, `agent_pub_key` (Holochain pubkey hex string, line 25), `identifier`, `permission_level`, `session_id`, `doorway_id/url`, `conductor_id`, `installed_app_id`, `is_steward`, `has_local_conductor`. **There is no `agent_cid` claim.** The closest identifier is `agent_pub_key` — the Holochain agent pubkey, which is *not* the same as the imagodei DHT `agent_cid` that `AgentPeerBindingView` records.

**What the proxy forwards.** `doorway/doorway-service/src/routes/storage_proxy.rs::forward_to_storage` (lines 64-179) — the canonical handler for every registry-routed request — explicitly forwards exactly three headers: `Content-Type` (line 102-106), `Authorization` (line 108-112), and `X-Observation-Id` (line 115-119). It does NOT decode the JWT, does NOT extract any claim, and does NOT inject any `X-Agent-*` header. The same is true of `forward_blob_to_storage` (lines 203-341) which forwards only the `Authorization` header (line 262-270). **Storage receives the bearer token but no extracted identity.**

**What storage expects.** `elohim/elohim-storage/src/api/account.rs:945-971` — `extract_agent_key()` — reads `X-Agent-Id` from the request, falling back to the active local session's `agent_pub_key` for Tauri-direct mode. The doc comment at line 948 even says *"doorway JWT middleware injects this after validation"* — but **doorway does not.** Storage's CORS allow-headers list (`elohim/elohim-storage/src/http.rs:998`) includes `X-Agent-Id`, but no doorway middleware writes it.

**The one place the header *is* set.** Two bespoke handlers in `doorway/doorway-service/src/routes/auth_routes.rs` — `handle_portal_host` at line 3561 and `probe_first_portal_host` at line 3623 — each manually set `X-Agent-Id: claims.agent_pub_key` for their hand-rolled `reqwest` calls to `/api/v1/account/portal-hosts`. **This is per-handler boilerplate, not middleware.** Routes that flow through the registry-routed proxy path (the path Phase 5 view-federation handlers will use) get nothing.

**Outcome: not wired.** The agent identity required by Phase 4's `bindings_resolver` (T22) is not propagated by the proxy. Two distinct gaps:

1. **Header name + identifier-kind mismatch.** Even if the proxy did forward, the available identifier is `agent_pub_key` (Holochain pubkey), and storage's `extract_agent_key` reads `X-Agent-Id`. The imagodei DHT key on `AgentPeerBinding.agent_cid` is a *different identifier* (a CID derived from the imagodei human entry, not the agent pubkey). T22 must clarify which it needs and either (a) wire `X-Agent-Id` and use `agent_pub_key → AgentPeerBinding` lookup, or (b) extend `Claims` and the JWT mint path with a real `agent_cid` claim and add `X-Agent-Cid` middleware.
2. **Proxy never injects the header.** Whatever choice (a) or (b) above lands on, `storage_proxy::forward_to_storage` and `forward_blob_to_storage` need a JWT-decode + header-inject step. The cleanest spot is a single helper applied uniformly so we don't repeat the `auth_routes.rs:3561/3623` per-handler pattern across every view-federation route.

**Adds a new prerequisite to T22.** Phase 4 task T22 (`bindings_resolver`) cannot resolve "the calling agent's bindings" until this is wired. **Suggested rewording for T28's task list (Phase 4):**

> T28a (NEW, prerequisite for T22): wire JWT-derived agent identity through `storage_proxy::forward_to_storage` and `forward_blob_to_storage`. Decode the bearer JWT once, inject `X-Agent-Id: claims.agent_pub_key` (or `X-Agent-Cid` if a CID claim is added) on every forwarded request, mirror the existing `auth_routes.rs:3561/3623` pattern but in the shared forwarder. Update `extract_agent_key` doc comment in `elohim/elohim-storage/src/api/account.rs:948` to reflect the actual injection point. Decide explicitly whether T22 needs `agent_pub_key` (matches existing JWT) or `agent_cid` (requires Claims extension + JWT mint path change in `auth_routes.rs` token-issue handlers).

Until T28a lands, T22 should accept the calling agent identifier as an explicit handler argument (not from the request) so unit tests are unblocked, and the integration-test pass (Phase 10, T54+) is what validates header propagation against a live doorway.

#### Pre-flight verification — T02 results (static, 2026-05-01)

**Status: DONE_WITH_CONCERNS — outcome = "no table, no writer, no helper data". This is the largest pre-flight gap so far. Phase 4's T23 (`compose_distribution_summary`) cannot be implemented as written; it requires either a new table + writer pair (substrate work) or a fundamental rework of how projector ack state is sourced.**

Static verification only (Eclipse Che, no live SQLite). The kickoff prompt's `sqlite3 ... SELECT COUNT(*) FROM rea_projection ...` cannot run against a live DB; static schema review against `migrations/` and a writer search across `src/` are sufficient to determine the design state.

**Step 1 — schema review.** `grep -rni "create table"` across `elohim/elohim-storage/migrations/` enumerates every table that exists on a fresh storage.db. **There is no `rea_projection` table.** The closest neighbours are:

- `projector_cursor` (migration `2026-04-25-010000_projector_cursor`, columns `pillar`, `kind`, `last_epr_cid`, `last_issued_at`, `updated_at`, PK `(pillar, kind)`) — a Category-C operational watermark table; tracks "how far has the projector advanced" per (pillar, kind), not per content unit.
- `epr_atoms` (migration `2026-04-22-050000_add_epr_tables`) — the canonical projection of EPR atoms (CID-keyed); sibling tables `epr_coupling`, `epr_claims`, `epr_supersedence`. None carry projector ack state.
- `agreements`, `rea_commitments`, `economic_events` (migration `2026-01-08-000000_initial`) — the three REA tables that `src/rea_projection.rs` *does* write to.

Verbatim grep across `elohim/`: `projector_acks`, `ack_count`, `replica_peers`, `replica_count`, `projector_peer_id` are **not present in any migration, schema file, Rust struct, or test fixture in the entire repo** (the one match for `replica_count` is `total_replica_count` on a derived view struct in `views.rs:3172`, unrelated to a column).

**Step 2 — writer search.** `src/rea_projection.rs` is **the signal-handler module**, not a backing store for a `rea_projection` table. The module name is misleading-by-coincidence with the kickoff prompt's expected table name. What it actually does (lines 124-206): receives `ReaProjectionSignal::{AgreementCommitted | ReaCommitmentCommitted | ReaEconomicEventCommitted}` from the conductor's post-commit hook and upserts into the corresponding REA table (`agreements`, `rea_commitments`, `economic_events`) with `dht_anchor_hash` set. **It does not emit a per-blob projector-ack record anywhere.** It also has a TODO at line 19-24 noting that it isn't even wired into the conductor's signal-receive loop yet — meaning even those three REA tables likely don't see projection writes outside dedicated tests.

There is no other module in `src/` that writes to a `rea_projection` table. The grep `rea_projection` returns exactly one hit across all of `elohim/elohim-storage/`: the `pub mod rea_projection;` declaration in `src/lib.rs:62`.

**Step 3 — what T23 assumes.** Plan T23 (`distribution_view::compose_summary`, lines 2935-3160) implements `load_projector_acks` as:

```rust
use crate::db::diesel_schema::rea_projection::dsl as r;
Ok(r::rea_projection
    .filter(r::cid.eq(blob_hash))
    .filter(r::ack_count.gt(0))
    .select(r::projector_peer_id)
    .load::<String>(conn)?)
```

That is, the plan expects a `rea_projection` table with at minimum these columns:

| Plan T23 expects | Actual |
|---|---|
| `cid: TEXT` (blob hash, content CID) | absent |
| `ack_count: INTEGER` | absent |
| `projector_peer_id: TEXT` | absent |

Plan T23 also assumes two further tables that this static pass surfaced as similarly absent — flagged here for the controller, even though they are out of T02's stated scope:

- `custodian_blob_commitments` (`load_replica_peer_ids`, `compute_reciprocity_hint`): expected columns `blob_hash`, `status` (`'healthy'|'probing'|...`), `custodian_id`, `committed_bytes`, `beneficiary_peer`. **Not in any migration.** `grep -rn "custodian_blob_commitments"` in `migrations/` returns zero.
- `content_store` (`load_reach_class`): expected columns `blob_hash`, `reach_class`. **Not in any migration.** `grep -rn "content_store"` in `migrations/` returns zero.

Together these are three load-bearing tables that T23 reads from and that do not exist. T02's specific charter is `rea_projection` only, but the controller should know the gap is wider before scoping a remediation task.

**Outcome: no writer (and no table).** This is the third pre-flight outcome class beyond Gap-1 (no seeder) and Gap-2 (schema drift): "no projection at all". The plan's kickoff text anticipated this exact path — *"If 0 → projector signals aren't landing; T20 (`compose_distribution_summary`) should default `projectorCount = 0` and the demo will render 'peer-only' badges."* The substrate state is one step further than that: not zero rows, but zero tables.

**What T23 needs that isn't there.** Two distinct things, in priority order:

1. **A table to hold per-blob projector acks.** A Category-C operational table keyed by `(blob_hash, projector_peer_id)` with at least an `ack_count` (or `acked_at` timestamp + a count derived by query). Schema-first: the column names in the kickoff prompt (`projector_acks`, `ack_count`) and the plan T23 query (`cid`, `ack_count`, `projector_peer_id`) **diverge** — the migration that adds this table needs to pick one and update T23 if it isn't `(cid, ack_count, projector_peer_id)`. The kickoff prompt's `projector_acks IS NOT NULL` phrasing implies a JSON-list column, while T23's `r::projector_peer_id` and `r::ack_count` imply normalized FK rows. The latter is the one the implementation actually consumes; recommend the migration follow it.
2. **A writer that populates it.** The current `rea_projection.rs` handler only writes the three REA tables, all of which are CID/id-keyed REA entities, not blob-hash-keyed projection acks. A new pathway is needed: either an additional signal variant `BlobProjectionAcked { blob_hash, projector_peer_id }` from a coordinator that watches for "this peer has projected the EPR for this blob", or — simpler — a doorway-side writer that marks an ack when the doorway successfully projects/serves a blob through its registry-routed path.

**Suggested remediation pattern for the plan (parallel to T03a/b for AgentPeerBinding):**

> T03c (NEW): migration adding `rea_projection` table with columns matching T23's query (`cid TEXT`, `projector_peer_id TEXT`, `ack_count INTEGER`, `last_acked_at TEXT`, PK `(cid, projector_peer_id)`); add Diesel schema entry; wire the writer either (a) into `rea_projection.rs` as a fourth signal variant once the underlying coordinator exists, or (b) as a doorway-side write at successful blob fetch time. Decision point: where the ack semantically originates (DHT-notarized vs. operational best-effort).
>
> Pending T03c, T23's Phase-4 implementation should **degrade gracefully**: if the table is absent (no migration applied, table-not-found error), `projector_count` defaults to 0 and `MyRole` cannot include the `Projector` axis (only `Replica | NotHosting | SoleReplica`). The simple-tier render then shows the "peer-only" badge described in the kickoff prompt's expected-zero-rows path. This matches the plan's stated graceful-degradation contract and unblocks the rest of Phase 4 from a substrate dep.

**Adds a sub-task to T34 (operator dashboard).** Per kickoff prompt step 2: when the table eventually does exist, the operator topology view should expose `rea_projection` row counts (per-cid and per-projector_peer_id histograms) as a debug pane so live "are projector signals landing?" can be answered without sqlite shell access. This is a Phase 5 dashboard add, not Phase 0 work; logged here for inclusion when T34 is detailed.

**Runtime verification still required (defer to Jenkins / local-dev once T03c lands):** `sqlite3 ~/.elohim-storage/storage.db "SELECT COUNT(*) FROM rea_projection WHERE ack_count > 0;"` — must run against a seeded + signal-wired stack. Until T03c, the query fails with `no such table: rea_projection`. After T03c with no writer wired: returns 0, and T23 graceful-degradation kicks in. After T03c with writer wired: positive count is the green light to drop the graceful-degradation branch.

### CI quality gates (per CLAUDE.md)

```
RUSTFLAGS="" cargo build --release            # both elohim-storage and doorway
RUSTFLAGS="" cargo test --lib --bins
RUSTFLAGS="" cargo clippy -- -D warnings
cargo fmt --check
pnpm run schema:validate
pnpm run schema:codegen:ts
pnpm run lint
pnpm exec vitest run
```

Pre-push hook auto-detects changed projects.

## Constitutional Guardrails

- **No sovereignty, no ownership** (per `project_no_sovereignty_stewardship_over_ownership`): views render "stewards" + "stewarded compute" + "hosted by"; no "owns", "owner", "your data" framing in user-visible copy.

- **Visitor privacy at the surface**: no household-level identifiers in public-reach distribution data. Diversity axis aggregates to region/metro for public reach. Household names appear only when reach class is intimate or smaller (already inside the trust circle).

- **Federation gated by DHT-notarized bindings**: a peer cannot answer a federated query for an `agent_cid` it does not have a valid AgentPeerBinding for. Signature verification against the binding's signed body is mandatory.

- **Best-effort federation, honest freshness**: 200 OK on a federated view means *some* truthful state was assembled. `freshness` annotations are mandatory in the simple tier render. Hiding stale data without annotation would be lying.

- **DHT remains canonical**: REA Commitment + EconomicEvent + AgentPeerBinding are DHT-resident sources of truth. Storage projections are read-optimized views. The substrate self-heals.

- **Doorway is single-target dispatch** (per `project_doorway_single_target_no_fanout`): no blob fan-out at doorway. Resilience and peer-fallback live in the substrate. Doorway projects + caches; the network moves bytes.

- **Diversity axis is reach-driven**: the simple tier never renders household-level granularity for public reach. Detail tier may, but only to authenticated stewards.

- **Device archetype is first-class**: views render `node | desktop | mobile | steward`. Deployment topology (k8s, WireGuard, node affinity) is implementation detail and never appears in user-visible surfaces.

## Open Questions Deferred

- **Per-archetype federation timeout tuning** (mobile may need >3s on poor networks): single value this sprint; env-overridable for tuning.
- **Refresh-on-signal cache** for federated views: not needed at alpha-cluster scale; future optimization.
- **`gate_decisions` audit table** for the reach-earning gate: still ephemeral.
- **Hot-reload of manifest-driven policies**: registry rebuild on restart is sufficient.
- **Cross-doorway federation of distribution metadata** (federation-flavored interop): out of scope; doorway's web2 projection is local-only this sprint.
- **AT Proto / ActivityPub flavor projection** (per `project_doorway_is_federation_surface_atproto`): separate sprint.

## Out of Scope

- Shamir-split socially-derived recovery (per user direction: "we don't need Shamir recovery yet, just the auth flow")
- Elohim-mediated discernment matchmaking (deferred to Earning the Welcome)
- VF-GraphQL projection at the shefa app layer (still 🔴 per `project_epr_substrate_vs_vf_graphql`)
- Real CSPRNG-backed Laplace noise in standing aggregator (deterministic stub remains)
- Window-over-window standing trend computation
- FeedbackSignal zome relocation from content_store to mishpat (governance refactor sprint)
- New survey contentFormat (fold onto sophia-quiz-json discovery mode)
- Full M5 auth-portal convergence (lightweight peer-join-as-host only)
- record_predecessor + persistent sealing keys (Light Up T22 carryovers — orthogonal; predecessor graph not load-bearing for the operational view)

## References

- Predecessor: [Light Up the Graph](2026-05-01-light-up-the-graph-design.md)
- Substrate brainstorm: [Trust-compute gradient](2026-04-30-trust-compute-gradient-brainstorm.md) — §2.8 floor classes, §6.4 collective wisdom
- Memory: `project_first_class_graph_pattern` (graph topology as primitive)
- Memory: `project_three_layer_truth_model` (DHT / libp2p / doorway split)
- Memory: `project_principle_p1_reconciliation_controller` (eager reconciliation)
- Memory: `project_doorway_single_target_no_fanout` (no blob fan-out at doorway)
- Memory: `project_doorway_manifest_driven_routes` (manifest registry pattern)
- Memory: `project_multi_device_humans` (don't assume one pod per human)
- Memory: `project_multi_doorway_human_registration` (multi-registration is the resiliency pattern)
- Memory: `project_household_horizontal_scaling` (more blades = more elohim-node instances)
- Memory: `project_inventory_exchange_not_byte_replication` (gossip ≠ bytes)
- Memory: `project_storage_actor_vs_forwarder_patterns` (forwarder pattern)
- Memory: `project_signal_kind_extensible_protocol_class`
- Memory: `feedback_shift_measure_jenkins` (Jenkins MCP for measures)
- Memory: `feedback_schema_first_ioc` (schema-first IoC)
- Memory: `feedback_a2o_is_human_experience_not_dev_bugs`
