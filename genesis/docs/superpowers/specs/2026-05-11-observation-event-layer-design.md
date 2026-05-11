---
title: Observation/Event Layer — Witness Substrate for the Three-Layer Truth Model
status: Draft (awaiting user review)
created: 2026-05-11
authors: Matthew Dowell + Opus 4.7
pillar coupling: elohim (substrate primitive), infrastructure + lamad + mishpat + shefa + imagodei (manifest layers)
narrative spine: genesis/docs/content/elohim-protocol/observer-protocol.md (the elohim-observer epic)
related:
  - genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md (sibling spec; this is the operational half of the cut attestation drew from the notary side)
  - genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md (Track 2 substrate plane this spec extends)
  - genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md (Witness/Audit/Proof/Confirmation gradient — observations feed the Audit tier's re-execution requirement)
  - genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md (trust-then-verify pattern applied to diversity computation)
  - genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-wave-0-substrate-cleanup.md (Wave 0; this spec is a sibling to attestation consolidation under Wave 0)
memory_anchors:
  - project_three_layer_truth_model
  - project_iroh_phase11_all_backends_wired
  - project_iroh_phase11_sync_first_plane_landed
  - project_signal_kind_extensible_protocol_class
  - project_dht_vs_libp2p_scoping
  - project_doorway_single_target_no_fanout
  - project_inventory_exchange_not_byte_replication
  - project_memory_lifecycle_comet_shape
  - project_memory_classes
  - project_first_class_graph_pattern
  - project_compute_and_model_independent_diversity_surfaces
  - project_household_is_resilience_unit
  - project_social_compute_collective_is_stewardship_unit
  - project_node_metrics_vs_hub_aggregation_boundary
  - project_reach_earned_at_authoring
  - project_social_reach_nervous_system
  - project_collapse_bureaucracy_into_protocol
defers:
  - Polis-style opinion-clustering algorithms (higher-layer application that reads observation_diversity_summary)
  - Shamir share transport for recovery (per attestation consolidation §5, libp2p direct-message; not an observation primitive)
  - Sensor → observation gateway (camera/microphone → REA-story extraction is the elohim-observer hardware story, not substrate)
  - ATProto / ActivityPub federation of observations (Track 4 doorway projection, not substrate)
  - Mechanical privacy-switch hardware (substrate handles switch-state-as-absence; the switch itself is hardware)
---

## 1. Narrative spine — witness, not surveillance

> *"The Observer isn't a product — it's a protocol. A way of seeing that generates stories without storing surveillance."*  
> — `observer-protocol.md`, Part I

The elohim-observer epic names the protocol's posture toward observation: **ephemeral witness in service of flourishing, not durable surveillance in service of extraction.** Raw observations exist long enough for elohim agents to understand the context, then dissolve into structured REA story elements. The structured story persists; the surveillance does not.

This spec defines the substrate primitive that makes that posture executable. It is the operational half of the cut the Attestation Consolidation spec drew from the notary side: *attestations* are the durable validated recognitions on the DHT; *observations* are the ephemeral peer-witnessed evidence on the libp2p+iroh data plane. Together they implement the elohim-observer principle at protocol layer.

The Public Observer epic — Sarah's Tuesday at the Jefferson school board — shows the outcome: observations aggregate into legibility, communities become visible to themselves, and the resulting REA stories are the protocol's deliverable. Substrate is the camera; DHT is the memory; doorway is the projection. **The eyes that serve love** is a specification, and this is one layer of it.

## 2. Problem

The protocol today conflates two architectural layers across multiple DNAs:

| Today's primitive | Layer | Problem |
|---|---|---|
| `DoorwayHeartbeat` (infrastructure DNA) | DHT entry, link-pruned daily | 60s ticks bloat DHT; never used as authoritative |
| `HealthAttestation` (infrastructure DNA) | DHT entry, every health check | Per-check entries; flagged by attestation consolidation spec §2 |
| `FeedbackSignal` (elohim DNA) | DHT entry with extensible `signal_kind` | DHT-notarized for reach-coupling — STAYS as documented edge case |
| `OpinionStatement`, `*Vote` (mishpat DNA) | DHT entries | Already moved by attestation consolidation to Content+content_type |
| `peer_blob_inventory` (SQL) | libp2p gossip → SQL projection | Already-correct shape; pattern to generalize |
| `system_metrics`, `projection_events` (SQL) | per-node operational | Already-correct shape |
| `EconomicEvent` (elohim DNA) | DHT entry, every economic action | High-frequency operational verbs (`served-blob`, `consumed-compute`) would bloat DHT if every occurrence is notarized |

Three structural problems flow from this:

1. **Operational chatter on a notary substrate.** The DHT's gossip latency (200–2000ms) and entry budget (~3000 before degradation) are wrong for high-volume operational evidence. Every doorway heartbeat or blob fetch should not be a DHT entry.
2. **No unified observation primitive.** Each domain reinvents its own observation shape — different fields, different retention, different visibility model — fragmenting the substrate.
3. **No graduation pipeline.** When operational evidence does need notarization (e.g., "this doorway has been healthy for a period"), there is no protocol-of-record path from evidence to attestation. Each domain hand-codes its own.

The deeper architectural problem: **observations are being modeled as bespoke entry types per domain instead of as instances of a single substrate primitive that graduates to attestations or summary events under manifest-declared policy.** This is the same anti-pattern that the Attestation Consolidation spec corrected on the notary side and that `signal_kind` extensibility prevents at the feedback-signal layer.

## 3. The architectural cut

Two distinct layers, today conflated, are separated:

| Layer | Purpose | Architecture | Frequency | DHT? |
|---|---|---|---|---|
| **Observation** | Raw peer-witnessed evidence — heartbeats, blob-serves, opinion samples, audit logs, learning events, behavioral observations | libp2p+iroh data plane (Track 2 substrate), per-observer iroh-blob log, SQL projection | High volume, continuous | NO — operational only |
| **Event** (REA) | Authored fact-of-action — transfers, custody handoffs, stewardship grants, governance enactments, summary aggregations of operational activity | Existing `EconomicEvent` DHT entry, stake-class-gated by manifest | Low volume per-event-type; high-stakes events are authored directly, operational events graduate via summary | YES — Category A (notarized) |
| **Attestation** | Validated recognition — already specced | Existing `Content` entry with `content_type: "attestation:<subtype>"` | Low volume | YES — Category A (notarized) |

This matches the three-layer truth model (`project_three_layer_truth_model`): DHT=notary, libp2p=data-ops, doorway=web2 projection. The current protocol accidentally puts operational evidence on the DHT; this spec corrects it without introducing any new DHT entry type.

**Event vs Observation distinction** (per user lock-in):

- **Event** = authored fact-of-my-action. REA-shaped: provider + receiver + resource + quantity. Asserts state change. Two stake-classes per manifest declaration: `high` (notarized directly on DHT) or `operational` (graduated as summary EconomicEvent after observation aggregation).
- **Observation** = peer-witnessed evidence about someone else's action or the environment. Has observer + subject + payload. Lives on substrate only; never DHT-notarized as a raw fact. Aggregates to graduate attestations or summary events.

The cut sits as one of three substrate transport tracks per `2026-05-08-iroh-libp2p-complementarity.md`:

- **Track 1** (DHT notary) carries attestations and high-stakes authored events
- **Track 2** (substrate data plane, dual-stack libp2p+iroh) gains a new **Observation plane** alongside the existing ten (Blob/Gossip/Sync/EPR/EPR-atom/Shard/ViewFed/IdentityHandshake/Trust/Reach)
- **Track 4** (doorway projection) reads SQL summary surfaces for civic legibility per the Public Observer epic

## 4. The Observation primitive

### 4.1 Wire format

A single record shape carries all observations. MessagePack/CBOR over the new `observation-log` ALPN (parity-wired on libp2p + iroh, mirroring the Phase 11 plane-wiring pattern).

```rust
pub struct Observation {
    // Identity & ordering
    pub observer_cid: AgentCid,        // who witnessed (signs the record)
    pub log_cid: Cid,                  // root of observer's append-only iroh-blob log this entry lives in
    pub log_offset: u64,               // append-order position; (observer_cid, log_cid, log_offset) is the universal reference
    pub observed_at: i64,              // unix epoch seconds; observer's local clock
    pub seq: u64,                      // per-observer monotonic counter (gap-detect)

    // What's being observed
    pub observation_kind: String,      // manifest-declared, e.g. "infrastructure:doorway-heartbeat"
    pub subject_cid: Option<Cid>,      // the EPR being observed; None for environment-level observations
    pub subject_kind: Option<String>,  // "agent" | "content" | "doorway" | "hub" | "blob" | "computation" | "governance-action"
    pub payload_json: String,          // kind-specific JSON, validated against manifest schema at write

    // Inline diversity tags (observed-not-asserted at write; cheap-trust path)
    pub observer_household_cid: Option<Cid>,
    pub observer_collective_cid: Option<Cid>,
    pub observer_region: Option<String>,
    pub observer_archetype: Option<String>,         // consumer-grade | tier-1-hub | tier-3-hub | wearable
    pub observer_compute_class: Option<String>,     // per project_compute_and_model_independent_diversity_surfaces

    // Signature
    pub signature: Signature,            // observer signs the canonical encoding above
}
```

`payload_json` is pre-stringified per `feedback_serde_json_value_breaks_zome_boundary` — no `serde_json::Value` on wire boundaries.

### 4.2 Identity and reference

There is no standalone observation ID. The universal reference is `(observer_cid, log_cid, log_offset)`, serialized as `iroh://<observer_cid>@<log_cid>#<offset>`. This is the form Attestation `metadata.observation_refs` uses (per attestation consolidation §3.3).

The `log_cid` is content-addressed (BLAKE3 root of the iroh-blob). Appending to a log produces a new log_cid (Merkle-chained); cursor gossip propagates the new root. Iroh-blob's chunked verification provides per-row integrity; the per-row observer signature provides authorship.

### 4.3 Inline diversity tags

Observed-not-asserted at write. The substrate populates them from the observer's known state at append time:

- `observer_household_cid` from `humans.household_id` projection
- `observer_collective_cid` from `collective_memberships` projection
- `observer_region` from `collectives.region` projection
- `observer_archetype` derived from `peer_transport_manifest.capability_level`
- `observer_compute_class` from compute-report self-declaration

Inline tags enable cheap-trust queries (no joins). The verify path re-checks against authoritative substrate at audit time. Trust-then-verify per the trust-compute-gradient brainstorm §4.

### 4.4 SQL projection

Mirrors the struct one-to-one. Primary key `(observer_cid, log_cid, log_offset)`. Indexes:

- `(subject_cid, observation_kind, observed_at)` — evidence queries for attestation issuance
- `(observation_kind, observed_at)` — diversity rollups and graduation evaluation
- `(observer_cid, seq)` — gap-detect for partition recovery

Migration documents source-of-truth: `-- Source of truth: iroh-blob log (per-observer, content-addressed). Classification: C.`

## 5. Substrate plane — wire, gossip, log

The Observation plane is a new Track 2 plane sitting alongside the existing ten. Same dual-stack pattern: one wire spec, two ALPN registrations.

### 5.1 Three substrate moves per observation

```
┌────────────────────────────────────────────────────────────────┐
│ OBSERVER                                                       │
│                                                                │
│  1. Append row to local iroh-blob log                          │
│       observation-log/<observer_cid>                           │
│       BLAKE3-chunked; root advances to new log_cid             │
│                                                                │
│  2. Gossip cursor announcement (libp2p gossipsub)              │
│       topic: elohim/observations/<kind_namespace>              │
│       payload: { observer_cid, kind, log_cid, latest_offset,   │
│                  subject_cid?, observed_at_window }            │
│       size: ~200 bytes; high frequency tolerable               │
└────────────────────────────────────────────────────────────────┘
                            │
                            ▼  (peers subscribed to the topic)
┌────────────────────────────────────────────────────────────────┐
│ RECEIVER                                                       │
│                                                                │
│  3. Pull-fetch new segment via iroh-blob                       │
│       observation-log ALPN                                     │
│       fetch chunks (last_offset .. latest_offset)              │
│       verify BLAKE3 + per-row signatures                       │
│       project rows to local `observations` table               │
└────────────────────────────────────────────────────────────────┘
```

### 5.2 Why metadata-only gossip + pull-fetch

Per `project_inventory_exchange_not_byte_replication`: gossip lists are metadata; bytes mobilize via fetch. Per `project_doorway_single_target_no_fanout`: substrate doesn't fan out raw payloads. A 60-second doorway-heartbeat tick gossips ~200 bytes of cursor, not 200 bytes × N receivers of payload — receivers pull only when they care.

### 5.3 Topical subscription per kind_namespace

Each manifest-declared `observation_kind` belongs to a kind_namespace (default: the pillar). Each peer subscribes to namespaces by role:

| Peer role | Subscribes to |
|---|---|
| Doorway | `elohim/observations/infrastructure`, `elohim/observations/elohim` |
| Hub (tier-1+) | `elohim/observations/infrastructure`, `elohim/observations/shefa`, all hosted-content namespaces |
| Consumer-grade peer | namespaces relevant to its custodial role (lamad if hosting learner content, etc.) |
| Witness peer (volunteer high-diversity hub) | all namespaces — Ophanim role per epic Part II |

Subscription is policy-driven. The manifest declares default subscription matrices; operators can override.

### 5.4 Cross-stack peer-map carries the plane

Per the iroh-libp2p complementarity spec, every peer's `PeerTransportManifest` declares its supported planes. We extend the existing `IrohPlane` and `Libp2pPlane` enums with one variant: `Observation`. Phase 11 backend wiring pattern applies — one new `ObservationManagerBackend` neutral service, parity-tested across both transports.

### 5.5 Cursor tracking

Reuses the existing `projector_cursor` shape pattern (already shipping per migration `2026-04-25-010000_projector_cursor`). New table `observation_cursors` keyed by `(observer_cid, viewer_peer_id)`, tracking `last_projected_offset` and `last_seen_at`.

### 5.6 Backpressure

Producers self-rate-limit via gossipsub flow-control. Receivers can lag (`last_projected_offset` trails `latest_offset`) without breaking causality — the iroh log is the buffer. Under sustained pressure, receivers prioritize observation_kinds by retention_class: `wisdom > attestation-feeding > operational`. Pure operational chatter degrades first; high-value evidence keeps flowing.

### 5.7 Partition handling

When a peer comes back online, it reads its `observation_cursors`, asks gossip peers for current cursor announcements, computes deltas, pulls iroh-segments. No special partition protocol — the cursor model handles it because each peer's view is a deterministic function of `(peer_cursors × current_cursors)`.

## 6. Diversity model

The substrate measures diversity along five dimensions; attestation issuers query a single materialized view to decide whether evidence threshold is met.

### 6.1 Dimensions

| Dimension | Source of truth | Anti-Sybil weight |
|---|---|---|
| `household_id` | imagodei DNA (`humans.household_id` projection) | high — multi-peer-per-household is the protocol's primary Sybil surface |
| `collective_id` | imagodei DNA (`collectives` projection) | high — collective membership is attested |
| `region` | imagodei DNA (`collectives.region` projection) | medium — geographic federation insurance |
| `archetype` | substrate (`peer_transport_manifest.capability_level` → archetype derivation) | medium — hardware-class diversity resists hardware-monoculture capture |
| `agent_cid` | imagodei DNA (the canonical anchor) | low alone (trivially Sybilable), but the universal floor |
| `compute_class` | substrate (compute-report self-declaration with witness verification) | low-medium — independent axis from archetype |

### 6.2 Diversity summary view

```sql
CREATE VIEW observation_diversity_summary AS
SELECT
    subject_cid,
    observation_kind,
    COUNT(DISTINCT observer_cid)              AS distinct_agents,
    COUNT(DISTINCT observer_household_cid)    AS distinct_households,
    COUNT(DISTINCT observer_collective_cid)   AS distinct_collectives,
    COUNT(DISTINCT observer_region)           AS distinct_regions,
    COUNT(DISTINCT observer_archetype)        AS distinct_archetypes,
    COUNT(DISTINCT observer_compute_class)    AS distinct_compute_classes,
    MIN(observed_at) AS first_observed_at,
    MAX(observed_at) AS last_observed_at
FROM observations
GROUP BY subject_cid, observation_kind;
```

### 6.3 Trust-then-verify

- **Trust path** (cheap, default): query `observation_diversity_summary` directly using inline tags. Handles the 99% case where observers are not adversarial-suspected.
- **Verify path** (expensive, on-demand): point-in-time joins against `peer_identity_bindings`, `humans`, `collectives`, `peer_transport_manifest` at each observation's `observed_at`. Catches lying-by-self-declaration; logged in `audit_observations` table.

Policy decides when verify is required, expressed in the pillar manifest's `graduation_policy` — high-stakes graduations (e.g., compute-attestation Confirmation tier) demand verify; operational graduations skip it.

### 6.4 Threshold declaration

Per-observation_kind in pillar manifest:

```jsonc
{
  "observation_kind": "infrastructure:doorway-heartbeat",
  "diversity_threshold": {
    "distinct_households": 3,
    "distinct_regions": 2,
    "min_count": 5
  },
  "graduates_to": "attestation:doorway-health",
  "graduation_window_seconds": 3600
}
```

The Ophanim shape from the elohim-observer epic lives here: **multiple perspectives, no blind spots.** A doorway-health attestation backed by five observations from one household carries less weight than five observations from five households across three regions. The substrate makes diversity legible; policy decides what's enough.

## 7. Vocabulary and retention

### 7.1 Manifest-declared observation_kinds

Each pillar manifest declares its observation_kinds. The substrate validates the declared schema at write time. No new DHT entry types, no substrate releases for new kinds. Same pattern as `signal_kind` (per `project_signal_kind_extensible_protocol_class`) and the attestation consolidation `attestation_kind`.

```jsonc
// pillar-manifest.json (per pillar; same shape across all)
{
  "manifest_kind": "lamad",
  "observation_kinds": [
    {
      "kind": "lamad:content-viewed",
      "namespace": "elohim/observations/lamad",
      "schema": {
        "ref_cid": "Cid",
        "dwell_ms": "u64",
        "scroll_depth_pct": "u8",
        "session_id": "Cid?"
      },
      "retention_class": "contextual",
      "reach": "agent-private",
      "diversity_threshold": null,
      "graduates_to": null
    },
    {
      "kind": "lamad:mastery-check-result",
      "namespace": "elohim/observations/lamad",
      "schema": { "node_id": "Cid", "score": "f32", "hint_count": "u16" },
      "retention_class": "archival",
      "reach": "agent-private",
      "graduates_to": "attestation:mastery",
      "graduation_policy": "self-threshold"
    }
  ]
}
```

```jsonc
// infrastructure manifest
{
  "manifest_kind": "infrastructure",
  "observation_kinds": [
    {
      "kind": "infrastructure:doorway-heartbeat",
      "namespace": "elohim/observations/infrastructure",
      "schema": { "doorway_id": "Cid", "peer_count": "u32", "uptime_secs": "u64" },
      "retention_class": "operational",
      "reach": "community",
      "diversity_threshold": { "distinct_households": 3, "min_count": 5 },
      "graduates_to": "attestation:doorway-health",
      "graduation_window_seconds": 3600,
      "graduation_policy": "diversity-threshold"
    },
    {
      "kind": "infrastructure:blob-served",
      "namespace": "elohim/observations/infrastructure",
      "schema": { "blob_cid": "Cid", "bytes": "u64", "peer_cid": "Cid" },
      "retention_class": "operational",
      "reach": "community",
      "graduates_to": "event:served-blob-summary",
      "graduation_window_seconds": 3600,
      "graduation_policy": "summarize"
    }
  ]
}
```

### 7.2 Retention — comet shape per class

Per `project_memory_lifecycle_comet_shape` and `project_memory_classes`:

| retention_class | Hot (SQL) | Warm (SQL paged) | Cold (iroh-log only) | Memorialized |
|---|---|---|---|---|
| `operational` | 7 days | 90 days summarized | Pruned from iroh after graduation | n/a |
| `contextual` | 30 days | until consolidation event | log retained, SQL trimmed | n/a |
| `archival` | 90 days | indefinite | log retained | promoted to Attestation |
| `attestation-feeding` | until graduation window closes + 30d for audit | indefinite | log retained | Attestation evidence_refs anchor |
| `wisdom` | indefinite | indefinite | log retained | promoted to Content (knowledge node) |

The elohim-observer "ephemeral architecture" maps directly: operational observations dissolve after graduation; only the structured REA story (the graduated Attestation or summary Event) persists on the DHT. The substrate is the camera; the DHT is the memory.

### 7.3 Reach

Controls visibility, mirroring Content reach:

- `agent-private` — observer's log is encrypted to observer key; not gossiped; never crosses to peers (self-observation pattern)
- `household` — log gossiped within household namespace only
- `community` — gossiped to subscribed peers per topic
- `commons` — gossiped broadly; auditable by any peer
- `commons-attested` — gossiped commons-wide AND graduated observations land on DHT for cross-network anchoring

`agent-private` is the elohim-observer epic's privacy-switch state — observer running, but no peer sees raw data, only the structured story it produces.

## 8. Graduation paths

Two graduation paths convert observations into protocol-of-record artifacts. Both produce DHT entries that reference the observations as `evidence_observation_refs`. The observations themselves never go to DHT.

### 8.1 Path 1 — Observation → Attestation

Per the attestation consolidation spec §3.3. A graduation-evaluator runs per-pillar (as a tokio task inside elohim-storage). It polls `observation_diversity_summary` for kinds with `graduates_to: "attestation:<subtype>"`. When the diversity threshold is met within the window:

1. Selects the contributing observation rows (those that fed the summary)
2. Issues a `Content` entry with `content_type: "attestation:<subtype>"` via the existing coordinator zome
3. Populates `metadata.evidence_json.observation_refs` with `iroh://<observer_cid>@<log_cid>#<offset>` tuples
4. Populates `metadata.proof_evidence.class = "witness"` (default; can escalate per compute-attestation gradient)
5. Issues `AttestationToSubject` link to subject's EntryHash

The attestation now lives on DHT. The observations remain on substrate until their retention_class ages them out. An auditor following `observation_refs` re-fetches iroh-blob segments and re-verifies signatures — the audit-replay path the compute-attestation spec's Audit tier needs.

### 8.2 Path 2 — Observation → summary EconomicEvent

For operational event_kinds declared as `graduation_policy: "summarize"`, the evaluator emits a graduated EconomicEvent per peer per window:

```
manifest declares: blob-served operational; summarize hourly per provider
window closes at 2026-05-11T11:00Z
peer X served 1,247 blob-fetches in this window
   → EconomicEvent {
        action: "served-blob-summary",
        provider: peer_X,
        receiver: "many",
        resource: "blob-bytes",
        quantity: 18.4 GB,
        period_start: 10:00Z, period_end: 11:00Z,
        observation_refs: [iroh://...@...#..., × 1247],
        diversity_summary: { distinct_receivers: 89, distinct_collectives: 12 }
     }
```

One DHT entry replaces 1,247 — three orders of magnitude lower DHT write pressure, with provable evidence retained on substrate. The existing `EconomicEvent` entry type carries it; no new entry type, just a new action verb declared in the manifest.

### 8.3 Coordinator stake-class gate

`create_economic_event` in elohim DNA's content_store coordinator zome checks the action verb against the pillar manifest:

- `stake_class: high` — coordinator accepts direct authored events (transfers, custody handoffs, governance enactments)
- `stake_class: operational` — coordinator only accepts events with non-empty `observation_refs` and valid `graduation_policy` provenance

The manifest is the policy; the coordinator is the gate. Accidental high-volume DHT writes are prevented at validation.

### 8.4 Self-graduation vs evaluator-graduation

| `graduation_policy` | Who evaluates | Example |
|---|---|---|
| `self-threshold` | Observer evaluates their own observations | Lamad learner's own observations of practice attempts crossing mastery threshold |
| `diversity-threshold` | Any subscriber with diversity-summary access | Doorway-health attestation issued by infrastructure-witnesses |
| `summarize` | Observer (provider) batches own activity periodically | Blob-served hourly summary by hosting peer |

The manifest declares which is appropriate per kind.

## 9. Constitutional safeguards (witness, not surveillance)

The elohim-observer epic Part VIII names what observers cannot do, must do, and what humans control. These translate to substrate invariants enforced at validator, coordinator, and reach layers.

### 9.1 Observers cannot do

| Epic constraint | Substrate enforcement |
|---|---|
| Store video / raw sensor data beyond processing window | `retention_class: operational` defaults to 7-day hot, 90-day summary; iroh-log pruned after graduation. Raw sensor observations declared with shortest retention. |
| Share data without explicit consent | `reach` field on every observation_kind. `agent-private` observations are encrypted to observer key; never gossiped. Reach changes require manifest amendment (DHT-notarized). |
| Override physical privacy switches | Substrate has no path to compel observation. A peer signaled "private mode" by its user emits no observations. The substrate detects absence (no cursor advances) but cannot infer content. |
| Serve institutional power over individuals | Observation consumers are equal peers — no privileged read path. Doorway projection (Track 4) is opt-in and reach-bounded. |
| Generate surveillance profiles | Inline diversity tags can be queried; raw payloads cannot be cross-joined without explicit attestation-issuance authority. Verify-path queries are logged as `audit_observations`. |
| Enable stalking or harassment | `reach: agent-private` is the default for behavioral observations. Cross-agent observations require `reach: community` minimum + manifest declaration; declarations are reviewable. |

### 9.2 Observers must do

| Epic requirement | Substrate enforcement |
|---|---|
| Process locally on user-controlled hardware | Observations originate from the observer's elohim-node; no protocol pathway uploads raw observations to a central service. |
| Destroy raw data after story extraction | Retention policy + graduation pipeline. Once an observation graduates to Attestation, the raw log segment can be pruned per retention_class. |
| Respect graduated privacy by constitutional layer | The reach gradient (`agent-private` → `household` → `community` → `commons`) maps to the epic's individual/family/community/municipal/global hierarchy. |
| Preserve evidence only with consent or emergency | `retention_class: attestation-feeding` is the only path to long-term retention, and only for observations cited as evidence in an issued Attestation. |
| Generate REA valueflows transparently | The summary-EconomicEvent graduation is the protocol's primary REA-flow generation. Evidence_refs make the valueflow auditable. |
| Maintain cryptographic proof of operation | Per-row observer signatures + BLAKE3 chunked log + content-addressed log_cid + audit-replay path. |

### 9.3 Humans control

| Epic right | Substrate surface |
|---|---|
| When observation happens | Observer is a local capability; absent its activation, no observations are emitted. |
| What stories get shared | Reach is per-observation_kind declared, override-able per-instance via the observer's local reach policy. |
| How patterns get used | Attestation issuance from observations is gated by manifest-declared graduation policies; humans amend the manifest through governance. |
| Whether to participate | Observers are deployed by the human; withdrawal = stop the observer. Substrate handles absence gracefully. |
| Access to their own data | The observer's own iroh-log is locally readable always; SQL projection queryable through their elohim-node. |
| Deletion rights (forgotten when requested) | First-class governance flow — see §9.4. |

### 9.4 Forget as governance

Right-to-be-forgotten is **not unilateral pruning**. It flows through the protocol's existing feedback/governance loops on the EPR:

```
1. Subject (or observer self-requesting) issues a FeedbackSignal on the EPR
     signal_kind: "forget-request"
     target_cid: the subject EPR (or the attestation EPR carrying observation_refs)
     evidence_cid: optional — specific observations or attestation cited

2. Mishpat governance evaluator receives the signal via existing FeedbackSignal pipeline
   Evaluates against constitutional constraints declared in the pillar manifest:
     - graduated-harm tier of source observations (epic Part IV)
     - presence of overriding accountability claims (epic safeguards)
     - cited-evidence dependencies in active attestations

3. Evaluator produces a "forget-decision" attestation
     content_type: "attestation:forget-decision"
     subject: the original observation cluster or attestation
     outcome: granted | granted-with-redaction | refused-per-constraint
     reasoning: structured citation of the manifest constraint applied

4. On granted:
     observer's iroh-log publishes a redacted root (root-rewrite);
     downstream attestation pages a "redaction-applied" note in metadata.revocation;
     the attestation-of-record persists with degraded evidence_refs.

   On granted-with-redaction:
     PII fields stripped from payload_json but row retained.

   On refused:
     the decision is itself a public attestation explaining the constraint.
```

Three benefits: (a) right-to-be-forgotten is visible and reasoned-about, not silent erasure; (b) constitutional layers (graduated-harm protocol, Level 4 emergency preservation per epic Part IV) get a structured check; (c) downstream parties whose attestations cite the observations get notified through the existing FeedbackSignal pipeline.

Manifest additions for this flow:
- elohim manifest adds `signal_kind: "forget-request"`
- mishpat manifest adds `attestation:forget-decision` subtype

Both are manifest changes only — zero new entry types.

## 10. Migration plan — eight stages, pre-launch hard cutover

Matches the pacing of the attestation consolidation spec. Each stage is independently testable.

### Stage 1 — Manifest declarations

- Each pillar's manifest gains an `observation_kinds` array (`elohim/sdk/domains/<pillar>/manifest.json`)
- `elohim/sdk/domains/infrastructure/manifest.json` is **created** (today only `types/` subdir exists) — needed for `infrastructure:doorway-heartbeat`, `infrastructure:blob-served`, `infrastructure:system-sample` declarations
- `elohim/sdk/domains/mishpat/manifest.json` is **created** — needed for `attestation:forget-decision` subtype declaration
- elohim manifest gains a `forget-request` signal_kind
- The manifest schema (`elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`) is extended with the `observation_kinds` array definition
- `pnpm run lamad:codegen` regenerates `manifest-types.ts` to include `ObservationKindDeclaration`
- `pnpm run schema:validate` extended to validate observation_kind schemas

### Stage 2 — Wire format and ALPN

- New `crate::p2p::wire::Observation` MessagePack/CBOR frame in elohim-storage
- Two ALPN registrations: `elohim/observation/1` (libp2p) and `iroh-observation/1` (iroh)
- `PeerTransportManifest` gains `Plane::Observation` in both `IrohPlane` and `Libp2pPlane` enums
- Parity tests in `tests/bench_observation_plane.rs`

### Stage 3 — Storage tables

- New migration adds:
  - `observations` (per §4.4)
  - `observation_logs` (`observer_cid` PK, `latest_log_cid`, `latest_offset`, `retention_class`, `last_attested_at`)
  - `observation_cursors` (`(observer_cid, viewer_peer_id)` PK, `last_projected_offset`, `last_seen_at`)
  - `observation_diversity_summary` (view per §6.2)
  - `audit_observations` (operational, logs verify-path queries)
- All marked `-- Source of truth: iroh-blob log. Classification: C.`
- View types added to `elohim-storage/src/views.rs` with `#[derive(TS)]`
- `cargo test export_bindings` regenerates `@elohim/storage-client` types

### Stage 4 — Backend manager service

- `ObservationManagerBackend` neutral service (mirrors Phase 11 `SyncManagerBackend` pattern from `project_iroh_phase11_sync_first_plane_landed`)
- Reads from local iroh-log, writes to SQL projection, emits cursor-gossip via gossipsub
- Manages subscription matrix per peer role
- Both transports as parity backends

### Stage 5 — Graduation evaluator

- Per-pillar evaluator runs as a tokio task inside elohim-storage
- Polls `observation_diversity_summary`; matches against manifest `graduation_policy`
- Issues graduated artifacts via existing coordinators (Content for attestations, EconomicEvent for summary events)
- Coordinator stake-class gate added to `create_economic_event` in elohim DNA's content_store coordinator zome

### Stage 6 — DHT entry-type retirement

Hard cutover removal from DNAs (pre-launch — no backward-compat shim):

| DNA | Remove entry types | Replace with |
|---|---|---|
| infrastructure | `DoorwayHeartbeat`, `DoorwayHeartbeatSummary`, `HealthAttestation` | `infrastructure:doorway-heartbeat` observations + `attestation:doorway-health` graduations (via Content+content_type) |
| mishpat | (already handled by attestation spec) `OpinionStatement`, `ProposalVote`, `StatementVote` | child attestations of governance-action parent |
| elohim | `EconomicEvent` — KEEPS entry type; gains coordinator stake-class gate | manifest-validated stake-class per action verb |
| elohim | `FeedbackSignal` — STAYS on DHT (documented edge case: reach-coupling requires authoring-time notarization) | n/a |

### Stage 7 — HTTP API + storage-client

- New routes in doorway-service:
  - `GET /api/observations/by-subject/{cid}?kind={kind}`
  - `GET /api/observations/by-observer/{cid}?kind={kind}`
  - `GET /api/observations/diversity/{subject_cid}/{kind}`
  - `POST /api/observations` — internal-only; observers post their own rows; node signs and appends to local iroh-log
- TypeScript client in `@elohim/storage-client` via codegen

### Stage 8 — Existing-table reclassification

Pure-documentation stage. Existing operational SQL tables that already match the pattern get retro-classified as observation projections without schema change:

- `peer_blob_inventory` → reclass as `infrastructure:blob-served` + `infrastructure:blob-hosted` observations
- `system_metrics` → reclass as `infrastructure:system-sample` observations (per-node only per `project_node_metrics_vs_hub_aggregation_boundary`)
- `projection_events` → STAYS as is (already correctly operational; doc reference added)
- `peer_transport_manifest` → STAYS as is (canonical peer-identity surface, not an observation)

### Stage ordering

`1 → 2 → 3 → 4 → 5` in sequence (each strict dep on prior). Stage 6 follows 5 (need graduation working before retiring DHT entries). Stages 7 and 8 are parallel after 6.

## 11. Wave 0 integration

This spec is a sibling to the attestation consolidation under Wave 0 substrate cleanup. Sequencing:

1. Wave 0 stage A (attestation dedupe) lands first — establishes Content+content_type discriminator pattern
2. This spec's stages 1–6 land second — depends on attestation pattern for graduation Path 1
3. Stages 7–8 land in normal sprint cadence after Wave 0 closes

The two specs together complete the architectural cut between DHT-as-notary and substrate-as-data-ops. They share the manifest layer and the graduation pipeline; they retire overlapping DNA entry types; they collapse the protocol's bureaucratic surface into protocol primitives per `project_collapse_bureaucracy_into_protocol`.

## 12. Out of scope

- **Polis-style opinion-clustering algorithms.** Aggregation across observation rows for k-means/MDS clustering is a higher-layer application that reads `observation_diversity_summary`. The substrate just provides the rows.
- **Shamir share transport.** Per attestation consolidation spec §5, recovery shares move via libp2p direct-message; not an observation primitive.
- **Sensor → observation gateway.** The elohim-observer epic's camera/microphone → REA-story extraction pipeline produces observations as output, but the AI extraction layer is its own spec.
- **Inter-substrate federation (ATProto / ActivityPub).** Doorway projection of observations to federated protocols is Track 4 doorway work, not substrate.
- **Mechanical privacy-switch hardware.** The epic's physical privacy switches are a hardware story; this spec defines substrate behavior given switch state (no observation produced → no cursor advances), not the switch itself.

## 13. Open questions (deferred — may clarify in-flight)

1. **Graduation evaluator placement.** Runs inside elohim-storage as a tokio task. Should this be a separate elohim-graduator service for clearer ownership / restart isolation? Tracker for future split.
2. **Diversity-threshold tuning.** Initial thresholds in pillar manifests are best-guess; need calibration data from alpha cluster. Ship with conservative thresholds (3 households, 5 observations minimum) and adjust via manifest amendment after observing real distributions.
3. **`agent-private` observation encryption key.** Per-observer key. On device migration (multi-device humans per `project_multi_device_humans`), the observer's encryption key must travel with source-chain export. Intersects M5 auth-portal convergence sprint.
4. **Witness peer subscription model.** Ophanim role (hub peers subscribing to all observation namespaces for diversity-witnessing) is described but not policy-defined. Self-declaration in `peer_transport_manifest.role`? Operator-set?
5. **Compute cost of verify-path.** Substrate re-check of inline diversity tags requires point-in-time queries. Confirm `peer_identity_bindings` has the index shape to support this efficiently. If not, materialize a `diversity_state_at` snapshot table updated daily.

## 14. Success criteria

The spec is implemented when:

- A new observation is appended to an observer's iroh-blob log, cursor-gossiped, pull-fetched by subscribed peers, projected to SQL, queryable via the diversity summary view — **in under 5 seconds end-to-end on the alpha cluster.**
- A graduation evaluator detects threshold crossing and emits an Attestation (or summary EconomicEvent) within one graduation_window of threshold met.
- An auditor following `iroh://<observer_cid>@<log_cid>#<offset>` references in an Attestation's `metadata.evidence_json.observation_refs` can re-fetch and re-verify every cited observation deterministically.
- `DoorwayHeartbeat`, `DoorwayHeartbeatSummary`, and `HealthAttestation` are removed from the infrastructure DNA without functional regression — heartbeat liveness signals route through observations, doorway-health attestations are issued by the graduation evaluator.
- `peer_blob_inventory`, `system_metrics`, and `projection_events` carry doc-comment references to this spec confirming their reclassification.
- The `forget-request` → `attestation:forget-decision` flow round-trips through mishpat governance and produces a redacted iroh-log root.
- Diversity scores in `observation_diversity_summary` distinguish single-household observation pools from multi-household, multi-region pools.
- Manifest schema validation rejects observation_kind declarations missing required fields.
- Phase 11 backend-wiring tests pass for the Observation plane parity-tested across libp2p and iroh transports.

---

*The Observer Protocol exists to make communities visible to themselves. This spec exists to make the Observer Protocol executable at substrate layer.*
