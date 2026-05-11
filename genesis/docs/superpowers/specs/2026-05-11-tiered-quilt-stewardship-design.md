# Tiered Quilt Stewardship — Design

**Version:** 0.1
**Status:** Draft (post-brainstorm, pre-implementation)
**Last updated:** 2026-05-11
**Owner:** Matthew (operator decisions); spec drafted in brainstorm session 2026-05-11
**Vocabulary:** `genesis/graphos/vocabulary.md` (quilt, pantry, stock, draw, shard, RS(N,K))

---

## 1. Why this spec exists

The Elohim Protocol's content-distribution substrate is a P2P S3-shaped problem.
Phase 11 gate #2 partially solved the "hot" plane via iroh-canonical `/blob` for
BLAKE3-capable callers. This spec maps the full temperature gradient — *drawn /
stocked-warm / stocked / shelved* — and names the stewardship policy that moves
quilts between classes while honoring a network-wide reach floor, emitting an
unbroken REA event chain, and remaining invisible to end users.

The capability bar for "why we are going through this trouble" is **a family
cluster that spans cities, states, and jurisdictions, more resilient than any
k8s cluster precisely because no single operator, datacenter, or trust root
binds it.** Grandma's family photo album survives a flood in one city, a power
outage in another, and a court order in a third — because the protocol's
substrate is structurally redundant and elohim-attested across geographic and
social boundaries. Tiered quilt stewardship is the layer that makes that
substrate self-tending without operator intervention.

### Capability bar (the "why")

- Grandma never sees a tier. Her elohim and the cluster handle it.
- The protocol matches *and exceeds* k8s-cluster storage resilience by virtue
  of spanning physical/jurisdictional/social boundaries — a property no
  centralized cluster can claim.
- Tier transitions are first-class REA events. Every move emits a signal that
  flows into shefa for fair-reciprocal cost-sharing math.
- Sccache, our live MinIO-backed build-cache substrate, is dogfooded as the
  first external-archive destination — and a new contributor's first cargo
  build participates in the cooperative compute substrate from day one.

### What this spec subsumes (and what it does not)

This spec **absorbs** the self-healing dataplane work-in-flight (Plans 2–5
of `2026-04-19-self-healing-p2p-dataplane-design.md`) as sub-systems of the
tiered-quilt narrative:

- Plan 2 (verification scanner)        → §6 BreachScanner
- Plan 3 (auto-recovery)               → §6 Restitutor
- Plan 4 (attested holdings, B2)       → §4 HoldingsAttestation
- Plan 5 (chaos demo on shem)          → §7 chaos demo + dogfood scenarios

This spec **composes on top** of Plan 1 (observable + diverse auto-distribute),
which is live-coding now. Plan 1's diverse-placement remains the foundation
the TierController respects when stocking. Plan 1 is not re-litigated here.

This spec **does not** address: the elohim-operator LLM logic on a dwelling
hub (it is referenced as a clean integration point at the driver layer, but
its policy reasoning is its own domain). Recovery seed/share crypto. The
generalized federated-dwelling driver beyond a named scaffold.

### Wave 0 substrate cleanup (in-scope corrections)

Two preexisting drift items are absorbed and must complete before tier work
lands:

1. **Dedupe `Attestation` entry type.** The `Attestation` entry type
   currently exists in BOTH the elohim DNA (`content_store_integrity:1052`)
   and the imagodei DNA (`imagodei_integrity:416`) with identical shape.
   Wave 0 removes the imagodei copy; the single source of truth is the
   elohim DNA. Any imagodei coordinator function that creates Attestation
   entries gets routed through elohim DNA's coordinator.
2. **Rename `lamad_event_type` → `elohim_event_type`.** The field is
   currently named for the lamad pillar but used for non-lamad domains.
   Lamad is the LMS pillar; elohim is the protocol core. The field is
   renamed across schemas (`elohim/sdk/schemas/v1/EconomicEvent.schema.json`),
   Rust (`elohim/elohim-storage/src/db/models.rs` + DNA coordinators),
   generated TS (`elohim/sdk/storage-client-ts/src/generated/`), Angular
   consumers (the lamad pillar's `EconomicEventFactory` etc.), seed data,
   and tests. Existing values keep their strings (e.g. `content-view`).

These corrections are not punitive cleanups — they exist because the
tiered-quilt design itself adds eight new `elohim_event_type` values and
four new `Attestation` discriminators that must land on top of a clean,
non-conflicting substrate.

---

## 2. Architecture

### Three audiences, three surfaces

| Surface | Sees | Cares about |
|---|---|---|
| **End user (grandma)** | Silence. Optional ambient tile in shefa view: "compute contribution +1.6 GB-hours, served 23 draws this period." | Her elohim handles it. Responsiveness when she draws, fair share without thinking about it. |
| **App developer** | Tier-policy API on the storage SDK; manifest declarations; REA event subscription. | Tuning their app to the cost/availability curve they want. |
| **Operator / cluster admin / contributor** | Tier-state dashboard, placement-gap signals, capacity probes, breach/restitution attestations. | Keeping the cluster healthy; surfacing imbalances to shefa for fair-reciprocal accounting. |

### Pantry-temperature classes

Four classes name the REA resource state that an action verb produces:

| Class | Stewardship intent | Physical realization (driver-dependent) |
|---|---|---|
| `drawn` | Transient working copy from a draw; **no stewardship commitment** | Local cache; first to evict under pressure |
| `stocked-warm` | Active commitment + RAM-warm path (iroh-blobs) | Sub-100ms draw budget; SSD/RAM on a steward; max retention priority on a laptop |
| `stocked` | Active commitment + on-disk pantry | ~100ms–2s draw budget; persistent disk; medium retention priority on a laptop |
| `shelved` | Active commitment offloaded to durable/slow tier | Multi-second draw budget; HDD, external archive, or peer-cellar; lowest retention priority on a laptop |

The verbs that move quilts between classes:

- `stock` — deposit content into a pantry (entering `stocked` or `stocked-warm`)
- `draw` — retrieve content from a pantry (caller-side `drawn` working copy)
- `shelve` — route a stocked quilt to a cellar destination
- `promote` — warmer transition (e.g., `stocked` → `stocked-warm`)
- `demote` — cooler transition above floor
- `evict` — remove all stewardship (only legal at-or-below floor=`drawn`)
- `restitute` — reconstruct from K-of-N survivors after a breach

### Three truth layers

| Layer | Class | What lives here |
|---|---|---|
| **DHT (Category A)** | Notarized | `Commitment` entries for `action="custody-quilt"` — the stewardship floor |
| **DHT (Category B2)** | Agent-Scoped Attestation | `Attestation` entries for `tier-breach`, `tier-restitution`, `tier-holdings`, `tier-accounting`, `tier-self-degraded` |
| **libp2p (operational)** | Category C | EconomicEvent rows for every transition; inventory gossip deltas |
| **SQLite (projection)** | Category C | `quilt_tier_state`, `rea_commitments` view, extended `placement_gaps`, four small attestation projection tables |

No new DHT entry types are introduced. The four (eventually five) new
attestation discriminators all reuse the existing `Attestation` entry type
with `category="storage-stewardship"`.

### Controller approach

A peer-local `TierController` loop runs inside `elohim-storage`. Per quilt
held: it reads the contract floor (DHT projection), the local heuristic
signals (`BlobMetadata`, gossip projections, manifest hints, archetype bias),
and computes a *desired* class with `floor ≤ desired ≤ stocked-warm`. When
desired differs from observed, it stagger-waits its deterministic slot
(`blake3(cid || peer-id || epoch) % stagger_window`), re-checks reach floor,
and executes the transition through the appropriate driver. Every transition
emits an `EconomicEvent` row + an inventory gossip delta.

This is **Approach A** — contract-floored, archetype-tuned local heuristic.
It honors the three-truth-layer model, fits the archetype-tunable cadence
pattern, integrates cleanly with Plans 1–5, and the grandma-invisible
framing is natural because the controller is silent unless an operator
opens the dashboard.

Approach B (collective-attested transitions) was rejected because every
below-floor transition would couple to gossip round-trips. Approach C
(per-household centralized orchestrator) was rejected because it creates
a single point of failure and contradicts the P2P value pillar.

---

## 3. Components

### Inside `elohim-storage` (Rust)

| Component | Purpose | New? | Source files |
|---|---|---|---|
| `TierController` | Peer-local loop. Reads commitments + heuristics, emits transitions. | NEW | `src/tier/controller.rs` |
| `CommitmentFactory` | Creates `Commitment` entries on DHT when a peer accepts stewardship. Negotiates capability-attestation prerequisites. | NEW | `src/tier/commitment.rs` |
| `HeuristicClassifier` | Pure function: `floor + signals → desired_tier`. Easy to unit-test. | NEW | `src/tier/heuristic.rs` |
| `StorageBackend` (trait) + drivers | Per-archetype drivers: `LocalDeviceDriver`, `LocalHardwareTieredDriver`, `PeerCoordinatedDriver`, `FederatedDwellingDriver`, `ExternalArchiveDriver`. Realize the physical move. | NEW | `src/tier/drivers/*.rs` |
| `ShelfRouter` | Two-phase commit for moves to/from cellar destinations. Per-URI-scheme drivers (`peer-cellar://`, `external-archive://minio/`, etc.). | NEW | `src/tier/shelf/*.rs` |
| `BreachScanner` (absorbs Plan 2) | Two-pass scan: observation-gap (cheap) + direct draw probe (sampled). Issues `tier-breach` attestations. | Plan 2 | `src/tier/scanner.rs` |
| `Restitutor` (absorbs Plan 3) | Deterministic arbitration; reconstruct from K-of-N. Issues `tier-restitution` attestations. | Plan 3 | `src/tier/restitution.rs` |
| `HoldingsAttester` (absorbs Plan 4) | Periodic tier-holdings attestations. Feeds swarm-visible tier state. | Plan 4 | `src/tier/holdings.rs` |
| `AccountingAggregator` | Periodic aggregation of granular tier events into `tier-accounting` attestations. Cost-class weighted. | NEW | `src/tier/accounting.rs` |
| `TierStateProjection` | New SQLite table + read-API. | NEW | `src/db/quilt_tier_state.rs` + migration |
| `ReaEventEmitter` | Storage-side actor that writes `EconomicEvent` rows for every transition and gossips via inventory delta. | NEW | `src/tier/events.rs` |

### Outside `elohim-storage`

| Component | Purpose | Where |
|---|---|---|
| `quilt-s3-shim` (NEW Rust service) | S3-compatible HTTP front-end. Translates PUT/GET/DELETE onto storage-client's tier-aware `stock`/`draw`. sccache repoint target. | NEW crate `crates/quilt-s3-shim/` |
| Tier-state Angular component | Operator dashboard tile. Per-CID observed tier, floor, breach state. | `app/elohim-app/src/app/shefa/components/tier-state/` |
| App-manifest schema extension | New `quilt_policy` block. | `elohim/sdk/schemas/v1/app-manifest.schema.json` |
| Lamad manifest extension | New `elohim_event_type` values for the 10 tier-related events. | `elohim/sdk/domains/lamad/manifest.json` |
| Schemas (view + event) | `QuiltTierStateView`, `BreachAttestationView`, `RestitutionAttestationView`, `HoldingsAttestationView`, `AccountingAttestationView`. | `elohim/sdk/schemas/v1/views/` |

### Storage backend driver hierarchy

```
TierController (archetype-agnostic)
    │
    ▼
StorageBackend (trait)
    ├─ LocalDeviceDriver              ── one drive; tier = retention priority
    │                                    (laptop, browser, mobile, wearable)
    │                                    Bittorrent/Transmission analogy:
    │                                    tier moves priority weight, not bytes
    │
    ├─ LocalHardwareTieredDriver       ── real hardware tiers
    │                                    (steward node SSD + HDD)
    │
    ├─ PeerCoordinatedDriver           ── elohim-operator chooses peers
    │                                    in the local mesh; works on Ethernet,
    │                                    Wi-Fi, mesh radio — no k8s assumption
    │
    ├─ FederatedDwellingDriver         ── elohim-operator coordinates across
    │                                    multiple dwellings; the family-cluster
    │                                    spanning cities/states/jurisdictions
    │
    └─ ExternalArchiveDriver           ── external archives
                                         (MinIO, IPFS, Arweave, ...)
```

K8s PVC is one specific deployment, not a design primitive. If a hub happens
to run on k8s, `LocalHardwareTieredDriver` + `PeerCoordinatedDriver` may use
PVCs underneath, but the trait does not know that. A hub running on bare-metal
or on a Raspberry Pi cluster uses the same drivers with different bottom-layer
mechanics.

### Boundary discipline

- `TierController` never writes to DHT directly. It calls the elohim DNA
  coordinator zome (for `Commitment` and `Attestation` writes) or the
  existing post-commit signal projection (for storage-side reception).
  Honors `project_elohim_agent_sense_respond_architecture`.
- `ShelfRouter` is the only component that knows about external destinations.
  Adding IPFS/Arweave is a new driver, not a change to the controller.
- `HeuristicClassifier` is a pure function of observable signals. Swappable
  by archetype, easy to unit-test, easy to simulate at scale.

---

## 4. REA event catalog

### New `elohim_event_type` extensions (10 values)

| `elohim_event_type` | Action | Provider | Receiver | Resource | When emitted |
|---|---|---|---|---|---|
| `quilt-stocked` | `produce` | steward | `network` | quilt-cid at `stocked` | bytes settled on disk |
| `quilt-stocked-warm` | `produce` | steward | `network` | quilt-cid at `stocked-warm` | bytes promoted to RAM-warm path |
| `quilt-shelved` | `transfer` | steward | shelf-destination-agent | quilt-cid at `shelved` | bytes routed to cellar |
| `quilt-promoted` | `modify` | steward | `network` | quilt-cid (tier change) | warmer transition |
| `quilt-demoted` | `modify` | steward | `network` | quilt-cid (tier change) | cooler transition above floor |
| `quilt-evicted` | `consume` | steward | none | quilt-cid (removed) | bytes released; only at-or-below floor=`drawn` |
| `quilt-drawn` | `use` | steward | requesting-agent | quilt-cid (served) | served a draw request |
| `quilt-restituted` | `produce` | restitutor | `network` | quilt-cid at floor | reconstruction completed |
| `quilt-floor-committed` | `accept` | committer | `network` | commitment-hash | new ReaCommitment notarized |
| `quilt-floor-released` | `accept` | committer | `network` | commitment-hash | commitment expired or revoked |

### New optional columns on `economic_events`

```sql
ALTER TABLE economic_events
  ADD COLUMN tier_from TEXT,              -- {drawn, stocked, stocked-warm, shelved, null}
  ADD COLUMN tier_to TEXT,
  ADD COLUMN shelf_destination_uri TEXT,  -- only set when tier_to=shelved or tier_from=shelved
  ADD COLUMN driver_class TEXT,           -- {local-device, local-hardware-tiered,
                                          --  peer-coordinated, federated-dwelling, external-archive}
  ADD COLUMN cost_class TEXT;             -- driver-advertised cost weight for accounting math
```

### `custody-quilt` Commitment shape

Reuses the existing `Commitment` entry type (`elohim DNA content_store_integrity:1341`):

```rust
Commitment {
    action: "custody-quilt",
    provider: <agent_id>,
    receiver: "<household-id> | <collective-id> | network",
    resource_classified_as_json: serde_json::to_string(&QuiltCustodyClassification {
        cid: "bafkrei...",
        tier_floor: "stocked",           // minimum class peer commits to maintain
        shelf_destination: "peer-cellar://household/H" | "external-archive://...",
        diversity_role: Some("primary-warm" | "redundant" | "external-cold"),
    }),
    resource_quantity_value: Some(quilt_size_bytes as f64),
    resource_quantity_unit: Some("bytes"),
    has_beginning: <ISO timestamp>,
    has_end: Some(<ISO timestamp>),       // commitment duration
    clause_of: Some(<Agreement hash for the broader stewardship contract>),
    in_scope_of_json: serde_json::to_string(&[<household_id_or_collective_id>]),
    state: "active",
    ...
}
```

Naming-drift note: the DNA-side type is `Commitment`; the storage projection
in `elohim-storage/src/db/models.rs:1839` is `ReaCommitment`. The divergence
is preexisting; the spec documents the convention rather than renaming further.

### Attestation discriminators (five total under `category="storage-stewardship"`)

All five reuse the existing `Attestation` entry type (`elohim DNA
content_store_integrity:1052`). The `earned_via_json` field carries
type-specific payload.

#### `tier-breach`

```json
{
  "category": "storage-stewardship",
  "attestation_type": "tier-breach",
  "agent_id": "<witness_agent>",
  "earned_via_json": {
    "subject_cid": "...",
    "subject_agent": "...",
    "commitment_hash": "...",
    "observed_tier": "drawn",
    "tier_floor": "stocked",
    "probe_method": "holdings-attestation-gap" | "direct-draw-failure",
    "breach_window_start": "...",
    "breach_window_end": "...",
    "probe_result_json": "..."
  },
  "proof": "<signed by witness>"
}
```

#### `tier-restitution`

```json
{
  "category": "storage-stewardship",
  "attestation_type": "tier-restitution",
  "agent_id": "<restitutor_agent>",
  "earned_via_json": {
    "breach_attestation_hash": "...",
    "cid": "...",
    "restituted_at": "...",
    "reach_post_recovery": 5,
    "reconstruction_source_peers": ["...", "..."],
    "bytes_reconstructed": 1234567
  }
}
```

#### `tier-holdings` (periodic, Plan 4 absorbed)

```json
{
  "category": "storage-stewardship",
  "attestation_type": "tier-holdings",
  "agent_id": "<holder_agent>",
  "earned_via_json": {
    "period_start": "...",
    "period_end": "...",
    "holdings": [
      {"cid":"...", "tier":"stocked", "dwell_seconds":86400,
       "draws_served":23, "driver_class":"local-device", "cost_class":"long-term-personal"}
    ],
    "summary": {
      "total_gb_hours_warm": 0.2,
      "total_gb_hours_stocked": 1.4,
      "total_gb_hours_shelved": 8.0
    }
  }
}
```

#### `tier-accounting` (periodic aggregate, fair-reciprocal input)

```json
{
  "category": "storage-stewardship",
  "attestation_type": "tier-accounting",
  "agent_id": "<agent>",
  "earned_via_json": {
    "period_start": "...",
    "period_end": "...",
    "contribution": {
      "gb_hours_warm": 0.2,
      "gb_hours_stocked": 1.4,
      "gb_hours_shelved": 8.0,
      "draws_served": 23,
      "restitutions_made": 0,
      "weighted_total": 2.0
    },
    "consumption": {
      "gb_drawn": 3.2,
      "draws_made": 87
    },
    "net_position": -1.2
  }
}
```

#### `tier-self-degraded`

Issued by the steward itself when its driver cannot honor a commitment
(e.g., HDD unmounted, partition extending past retention window). This
preempts breach detection by external witnesses and shortens restitution
latency. Per `project_values_forward_disclosure_accountability` — visible
self-reporting is the protocol's preferred posture.

```json
{
  "category": "storage-stewardship",
  "attestation_type": "tier-self-degraded",
  "agent_id": "<self>",
  "earned_via_json": {
    "cid": "...",
    "commitment_hash": "...",
    "degradation_reason": "driver-failure" | "partition" | "capacity-pressure",
    "observed_tier": "drawn",
    "tier_floor": "stocked",
    "detected_at": "..."
  }
}
```

### Shefa signal mapping

| Shefa signal | Source | Cadence |
|---|---|---|
| `compute-contribution` (existing) | extends to read `tier-accounting` attestations | per accounting period |
| `storage-contribution` (new event type stream) | aggregated from `quilt-stocked` + `quilt-stocked-warm` events | continuous |
| `placement-gap` (existing `placement_gaps` table) | extended with `gap_kind="tier-below-floor"`, `"tier-breach-unresolved"`, `"tier-below-floor-prevented"` | scanner cadence |
| `restitution-required` (new) | derived from open `tier-breach` attestations without matching `tier-restitution` | scanner cadence |
| `reach-floor-honored` (new) | derived from active commitments + observed tier-holdings across distinct stewards | scanner cadence |
| `unresponsive-steward` (new) | derived from commitments-vs-served-draws over rolling window | accounting cadence |
| `commitment-expiring` (new) | derived from commitments where `has_end - now < threshold` | scanner cadence |

### App-manifest extension

```json
{
  "quilt_policy": {
    "default_tier_floor": "stocked",
    "shelve_after": "5m",
    "hold_warm_min": "0s",
    "prefer_destinations": [
      "external-archive://minio/sccache-elohim",
      "peer-cellar://household/{any}"
    ],
    "cost_class_hint": "ephemeral-build-cache"
  }
}
```

A grandmother's photo album would declare:

```json
{
  "quilt_policy": {
    "default_tier_floor": "stocked",
    "shelve_after": "30d",
    "hold_warm_min": "7d",
    "prefer_destinations": [
      "federated-dwelling://family/{family-id}",
      "peer-cellar://household/{any}",
      "external-archive://minio/family-cold"
    ],
    "cost_class_hint": "long-term-personal"
  }
}
```

---

## 5. Data flow — four loops

### Loop 1: The stock loop (entering stewardship)

```
App / agent action               elohim DNA                   elohim-storage
─────────────────────            ─────────                    ─────────────

"I want to steward CID-X    ───▶ create_commitment
 at ≥stocked-warm with           {action:"custody-quilt", ...}
 shelf_destination=peer-               │
 cellar://household/H"                 │ commits to DHT
                                       ▼
                                 post_commit signal ─────▶  ReaCommitmentCommitted
                                                            handler
                                                                  │
                                                                  ▼
                                                            rea_commitments row
                                                            (dht_anchor_hash NOT NULL)
                                                                  │
                                                                  ▼
                                                            TierController.wake_for_cid(X)
                                                                  │
                                                                  ▼
                                                            stock(X) via driver
                                                                  │
                                                                  ▼
                                                            quilt_tier_state(X) = "stocked-warm"
                                                            emit_event(elohim_event_type=
                                                                       "quilt-stocked-warm")
                                                                  │
                                                                  ▼
                                                            InventoryBroadcaster.delta(+X)
                                                            → libp2p gossip
```

### Loop 2: The controller loop (continuous tier management)

Per cadence interval (archetype-tuned: wearable=5min, mobile=2min,
edge=30s, hub=10s, archival=10s):

```
for each CID in quilt_tier_state:
    floor       = rea_commitments(CID).tier_floor              # DHT projection
    observed    = quilt_tier_state(CID).current_tier           # local
    last_drawn  = BlobMetadata(CID).last_accessed              # sled
    draw_rate   = decay-weighted hits over window              # computed
    peer_demand = aggregated peer_blob_inventory hits          # gossip projection
    manifest    = app_manifest.quilt_policy(CID)               # config
    archetype   = NODE_ARCHETYPE env                           # config

    desired = HeuristicClassifier.desired_tier(...)
              # pure function: floor ≤ result ≤ stocked-warm

    if desired ≠ observed:
        stagger = blake3(CID || peer || epoch) % stagger_window
        sleep(stagger_ms)
        if observed_swarm_reach(CID, after_my_transition) < floor:
            emit_signal("tier-below-floor-prevented", CID)
            continue                                            # suppress this transition

        execute_transition through driver
        emit_event(elohim_event_type=<one of quilt-* values>,
                   tier_from=observed, tier_to=desired,
                   driver_class=..., cost_class=...)
        update quilt_tier_state row
        InventoryBroadcaster.delta(...)
```

### Loop 3: The breach/restitution loop

```
BreachScanner.scan(now):
    # Pass 1 — observation gap (cheap)
    for each rea_commitment C where state="active" and tier_floor != "drawn":
        holdings = recent_holdings_attestations(C.cid, C.provider, window=2× scan_period)
        if holdings.is_empty:
            queue_probe(C, reason="silent-attester")
        elif latest(holdings).tier < C.tier_floor:
            elapsed = now - first_below_floor(holdings)
            if elapsed > breach_threshold_for_floor(C.tier_floor):
                issue_breach_attestation(probe_method="holdings-attestation-gap", ...)

    # Pass 2 — direct draw probe (expensive, sampled)
    for C in queued_probes ∪ stochastic_sample(active_commitments, p=archetype_sample_rate):
        result = direct_draw_probe(C.cid, C.provider, expect_tier=C.tier_floor)
        if result.failed or result.latency > sla_for(C.tier_floor):
            issue_breach_attestation(probe_method="direct-draw-failure", ...)

Restitutor.on_breach_witnessed(B):
    primary = argmin(p in candidate_restitutors,
                     blake3(B.hash || p.peer_id || epoch))
    if primary == self:
        run_primary_restitution(B)
    else:
        run_witness_role(B, primary)

run_primary_restitution(B):
    sources = discover_surviving_shards(B.subject_cid)   # K-of-N
    if len(sources) < K:
        emit_signal("reach-floor-unrecoverable", B)
        issue_attestation(attestation_type="tier-restitution-failed", ...)
        return
    reconstructed = rs_decode(sources)
    verify(blake3(reconstructed) == cid_to_blake3(B.subject_cid))
    original_commitment = fetch_commitment(B.commitment_hash)
    stock(B.subject_cid, tier=original_commitment.tier_floor)
    issue_attestation(attestation_type="tier-restitution", ...)
    emit_event(elohim_event_type="quilt-restituted", ...)
```

### Loop 4: The accounting loop

Archetype-tunable interval (default: daily):

```
AccountingAggregator.aggregate(agent, period):
    raw_events = SELECT * FROM economic_events
                  WHERE provider=agent
                    AND elohim_event_type LIKE 'quilt-%'
                    AND has_point_in_time BETWEEN period.start AND period.end
    summary = {
      gb_hours_warm:     Σ(size × tier_dwell_time at stocked-warm),
      gb_hours_stocked:  Σ(size × tier_dwell_time at stocked),
      gb_hours_shelved:  Σ(size × tier_dwell_time at shelved),
      draws_served:      count(quilt-drawn where provider=agent),
      restitutions_made: count(quilt-restituted where provider=agent),
      weighted_total:    Σ(gb_hours × cost_class_weight)
    }
    issue_attestation(attestation_type="tier-accounting",
                      earned_via_json=summary, ...)
    → DHT → shefa input signal → fair-reciprocal cost-sharing math
```

### Cross-flow: shefa planning signals

| Signal | Origin | Latency |
|---|---|---|
| Raw transition events | TierController operational | seconds |
| Placement-gap projection | BreachScanner + Plan 1 placement | minutes |
| Breach + restitution attestations | DHT-notarized | gossip latency (200ms–2s) |
| Aggregated accounting | Periodic AccountingAttestation | daily-ish |

Shefa's `EconomicEventBridgeService` (referenced at
`elohim DNA content_store_integrity:2648`) is the consumer surface.
Operator dashboard reads the projection tables; trust-as-efficiency signal
derives from accounting attestations.

---

## 6. Error handling

### Breach-threshold table (archetype-tuned defaults)

| Floor | Wearable | Mobile | Edge | Hub | Archival |
|---|---|---|---|---|---|
| `stocked-warm` | not allowed | 5m | 2m | 30s | 30s |
| `stocked` | 30m | 15m | 5m | 2m | 2m |
| `shelved` | 24h | 12h | 4h | 1h | 1h |

`stocked-warm` floor is rejected by CommitmentFactory at creation time
for wearable archetypes.

### Failure classes + mitigations

| Class | Failure shape | Mitigation | Where |
|---|---|---|---|
| Synchronized eviction | Cascade below reach floor | Deterministic stagger + pre-transition reach probe | TierController |
| Cold-tier hoarder | Holds shelved to game accounting | Cost-class weighted accounting; draws-served separate line item | AccountingAggregator |
| Hot-tier freeloader | Draws heavily, contributes nothing | Net-position math; values-forward visible, not punitive | shefa signal layer |
| Malicious tier-misreporting | Asserts capability without earning | Earned capability attestations gate commitment acceptance (the graph pattern handles it; no special enforcement code) | CommitmentFactory + ambient elohim-attestation chain |
| Driver divergence | Local driver fails to honor commitment | Self-degraded attestation issued by the steward | TierController self-check |
| Restitution thrashing | Multiple peers racing to restitute | Plan 3 deterministic arbitration extended | Restitutor |
| Network partition during transition | Bytes in flight when link drops | Two-phase commit in ShelfRouter; local cleanup never before destination ack + retention window | ShelfRouter drivers |
| Commitment expiration without successor | Floor drops to zero on `has_end` | Pre-expiry renewal signal to shefa; recruitment-flavored | BreachScanner |
| Misreported cost class | Driver promises cheap, manifest demands expensive | Manifest governs; CommitmentFactory rejects mismatched negotiation | CommitmentFactory |

### Capability attestations subsume misreporting concerns

A peer does not simply assert `stocked-warm` capability. Their elohim accrues
an `attestation_type="storage-capability"` chain over time, witnessed by
peers who actually drew from them at warm-class latencies — same shape as
content reach being earned at authoring (`project_reach_earned_at_authoring`,
`project_social_reach_nervous_system`).

- A peer with no earned `storage-capability ≥ stocked-warm` attestations
  **cannot accept** a `tier_floor="stocked-warm"` commitment. The
  CommitmentFactory negotiation refuses.
- A peer asserting capability without the earned chain gets ignored by
  other peers' routing decisions — same way unearned reach gets ignored.
- Direct draw probes are observational, not adversarial. The probe verifies
  behavior matches earned capability; it does not police a claim.
- Repeated probe failure → existing capability attestations decay. The
  peer's effective capability drops; they can no longer accept those
  commitments. No punitive mechanism — natural lifecycle of an attestation
  losing witnesses.

### Cascading-breach handling

When `len(sources) < K`:

- Emit `placement-gap` with `gap_kind="tier-breach-unrecoverable"` and
  `severity="critical"`.
- Issue `Attestation` of type `tier-restitution-failed` so the failure is
  itself notarized — the protocol does not hide failure.
- Signal flows to shefa as a recruitment crisis. Per
  `project_values_forward_disclosure_accountability`: protocol surfaces
  the failure clearly, in the values-forward way.

### Recognition flow on restitution

Per `project_consolidation_events_economic_feedback` and
`project_ungrudging_service`: the breach window's recognition shifts to
whoever did the recovery work; the original provider remains in good
standing for their other commitments; no punitive event.

```
post_commit(RestitutionAttestation):
    emit_event(elohim_event_type="recognition-transfer",
               provider=SubjectAgent,
               receiver=RestitutorAgent,
               resource_classified_as=tier-recognition-shift,
               note="breach window {window} recognition transferred")
```

### HTTP responses (grandma surface)

The HTTP API for `/blob/{hash}` does **not** expose tier mechanics:

- **200 OK** with bytes — the protocol found them somewhere, regardless of tier
- **202 Accepted** with `Retry-After: <seconds>` — tier transition in progress
- **503 Service Unavailable** — reach floor unrecoverable (rare; UX message
  is "your protocol is reorganizing your family cluster, try again later")

Operator endpoints (`/api/v1/quilts/{cid}/...`) expose tier mechanics for
dashboards.

---

## 7. Testing strategy

### Band 1: Unit (per crate, deterministic)

| What | Where | Coverage |
|---|---|---|
| `HeuristicClassifier` pure function | `elohim-storage/src/tier/heuristic_test.rs` | 100% |
| Stagger arbitration determinism | `elohim-storage/src/tier/stagger_test.rs` | golden tests |
| `LocalDeviceDriver` priority math | `elohim-storage/src/tier/drivers/local_device_test.rs` | per-tier weight + eviction order |
| `LocalHardwareTieredDriver` device selection | `elohim-storage/src/tier/drivers/local_hardware_tiered_test.rs` | device routing + degraded path |
| `ExternalArchiveDriver` two-phase commit | `elohim-storage/src/tier/drivers/external_archive_test.rs` | partition during phase 1 + retention |
| Manifest hint resolution | `elohim-storage/src/tier/manifest_test.rs` | precedence: app → archetype → policy.toml |
| `quilt_tier_state` reconstruction on boot | `elohim-storage/tests/tier_state_reconstruction.rs` | rebuild from BlobStore + BlobMetadata |

### Band 2: Integration (multi-peer, real DHT + libp2p)

Reuses the sweettest harness (per `2026-04-22-sweettest-integration-layer-design.md`):

| Scenario | Peers | Verifies |
|---|---|---|
| `tier_commitment_lifecycle.rs` | 3 | Commitment created → projected → transitions emit → accounting rolls up |
| `tier_breach_detection.rs` | 4 | Steward drops below floor → scanner observes via holdings gap → breach attested |
| `tier_restitution_arbitration.rs` | 5 | Deterministic arbitration picks one primary; others fall back to witness role |
| `tier_cascading_breach.rs` | 6 (only K-1 survivors) | Cascading breach → `tier-restitution-failed` + critical placement-gap |
| `tier_capability_attestation_gate.rs` | 3 (wearable, mobile, hub) | Wearable rejects `stocked-warm` floor; hub accepts |
| `tier_two_phase_commit_partition.rs` | 2 + simulated external | Partition during shelve → no local data loss → completes on reconnect |

### Band 3: Chaos demo on shem (absorbs Plan 5)

Live multi-node topology per `project_shem_is_p2p_live_canvas`:
Matthew/Jessica/Timothy on household cluster; others on shem.

#### Existing Plan 5 scenarios (extended with tier assertions)

1. Steward offline → reach breached → restitution → reach restored. **ASSERT**:
   `tier-breach` + `tier-restitution` chain visible in shefa.
2. Partition splits household from family-cluster. **ASSERT**: no spurious
   cross-partition restitution.
3. Wearable churns. **ASSERT**: no `tier-breach` from wearable lifecycle.

#### New tier-specific scenarios

4. **New developer first cargo** (the dogfood, **Wave 7**)

```gherkin
Feature: New developer's first cargo build creates a quilt
  Scenario: First cargo build produces a quilt-stocked event and accrues recognition
    Given I open a fresh Che workspace
    And my pod boots with sccache-credentials mounted
    And SCCACHE_ENDPOINT points at the quilt-s3-shim
    When I run `cargo build` for the first time
    Then sccache writes compilation outputs through quilt-s3-shim
    And the shim emits quilt-stocked events with my agent_id as provider
    And the manifest hint shelve_after=5m demotes my entries within minutes
    And my entries route to external-archive://minio/sccache-elohim
    When a teammate later runs `cargo build` against the same dependencies
    Then their cache hit is a draw from my earlier stock
    And a quilt-drawn event records my contribution as draws_served
    And within one accounting period, an Attestation tier-accounting summarizes
      my compute-contribution in gb-hours
    And my shefa view shows the ambient compute-contribution tile
    But at no point am I asked to think about tiers
```

5. **Grandma's family cluster survives city-scale outage**

```gherkin
Feature: Family-cluster resilience across cities and states
  Scenario: Grandma's photo album survives concurrent outages
    Given grandma's family cluster steward peers in Brooklyn, Austin, Portland
    And her photo album quilts have tier_floor=stocked, RS(8,4)
    When the Brooklyn dwelling loses power for 4 hours
    And simultaneously the Austin household's wifi goes down for 2 hours
    Then grandma's photo draws succeed from Portland
    And no breach attestation fires (reach floor still honored: 4 of 8 shards survive)
    When Brooklyn comes back online
    Then auto-recovery is not triggered (no breach occurred)
    And the grandma surface shows nothing — silence is the correct UX
```

6. **Hoarder vs steward** (REA-math sanity check)

```gherkin
Feature: Cost-class weighted accounting handles hoarders
  Scenario: A peer who shelves everything earns less than one who serves draws
    Given peer A holds 100 GB shelved, serves 0 draws
    And peer B holds 50 GB stocked, serves 1000 draws
    When the accounting period closes
    Then peer A's weighted_total < peer B's weighted_total
    And peer B's draws_served line item is non-zero in shefa
    And no punitive event fires against peer A
    And peer A's net_position is positive but small
```

7. **Self-degraded driver report** (transparent failure)

```gherkin
Feature: Steward self-reports when its driver can't honor a commitment
  Scenario: HDD unmounts mid-operation; steward issues self-degraded attestation
    Given a steward node with LocalHardwareTieredDriver routing shelved to /dev/sda2
    And /dev/sda2 unmounts unexpectedly
    When the TierController self-check runs
    Then a tier-self-degraded attestation is issued by the steward itself
    And the breach detection skips holdings-gap pass (already attested)
    And restitution begins immediately
    And the original steward is not penalized in trust — the failure was disclosed
```

### CI integration

- Pre-push: Band 1 unit tests via `cargo test --lib --bins` (RUSTFLAGS="" override).
- Per-commit Jenkins: Band 2 sweettest (5–10 min per scenario, parallel-able).
- Daily Jenkins: Band 3 chaos demo on shem (nightly).
- Pre-merge to `dev`: Bands 1+2 required green; Band 3 reported but not gating.

---

## 8. Dogfood story + delivery waves

### The new-developer onboarding moment (concrete)

```
Hour 0    Developer opens devspace.elohim.host. Pod boots; sccache-credentials
          Secret auto-mounted; SCCACHE_ENDPOINT points at quilt-s3-shim
          ClusterIP. A custody-quilt commitment is auto-created for this
          dev's agent: tier_floor=stocked, retention=7d,
          manifest_hint=ephemeral-build-cache.

Hour 0+5  Developer runs `cargo build` (first one). sccache misses on every
          entry; cargo compiles; sccache writes outputs via S3 PUT to
          quilt-s3-shim. Shim calls storage-client.stock(cid,
          classification=ephemeral-build-cache). TierController stocks at
          "stocked"; emits quilt-stocked events; InventoryBroadcaster
          gossips deltas.

Hour 0+12 TierController next cycle: 5min shelve_after threshold passed.
          desired=shelved. ShelfRouter.move(cid,
          external-archive://minio/sccache-elohim/...). Emits
          quilt-shelved; local bytes evicted after retention window
          (two-phase commit complete).

Hour 1    Re-runs cargo. sccache S3 GETs entries; shim draws from MinIO.
          quilt-drawn events fire with this developer as receiver.

Hour 6    Teammate two timezones over runs `cargo build`. Their cache hits
          come from entries this developer stocked earlier. quilt-drawn
          events fire with new-dev as PROVIDER, teammate as RECEIVER.
          draws_served counter ticks up. Ungrudging service: neither party
          knows or needs to know who served whom.

Day 1     Accounting period closes. AccountingAggregator rolls up. Issues
          tier-accounting attestation:
            gb_hours_stocked=0.2, gb_hours_shelved=1.4
            draws_served=23, draws_made=87
            weighted_total: small but positive
          Shefa surfaces an ambient tile in new-dev's view: "compute
          contribution: +1.6 GB-hours, served 23 draws." That's the only
          thing the developer sees of the substrate. No tier vocabulary.
          No CID. No transition events. Just contribution.
```

### Delivery waves

| Wave | Sub-plan | Deliverable | Depends on |
|---|---|---|---|
| **0** | `tiered-quilt-substrate-cleanup` | Dedupe Attestation (elohim DNA single source); rename `lamad_event_type` → `elohim_event_type` across schemas/Rust/TS/Angular/seed/tests | none |
| **1** | `tiered-quilt-commitment-factory` | CommitmentFactory + ReaCommitment projection + `custody-quilt` action + new event-type values (10) + `tier_*` columns | Wave 0 |
| **2** | `tiered-quilt-controller-and-drivers` | TierController + HeuristicClassifier + LocalDeviceDriver (laptop archetype first) + ReaEventEmitter + quilt_tier_state projection | Wave 1 |
| **3** | `tiered-quilt-shelfrouter-minio` | ShelfRouter + ExternalArchiveDriver + MinIO scheme + two-phase commit + quilt-s3-shim crate + sccache repoint | Wave 2 |
| **4** | `tiered-quilt-breach-scanner` (Plan 2) | BreachScanner with both passes + breach attestations + projection | Wave 2 |
| **5** | `tiered-quilt-restitution` (Plan 3) | Restitutor + arbitration extended to tier transitions + tier-restitution attestations + recognition-transfer events | Wave 4 |
| **6** | `tiered-quilt-holdings-and-accounting` (Plan 4) | HoldingsAttester + AccountingAggregator + tier-holdings + tier-accounting + shefa mappings + ambient contribution tile | Wave 5 |
| **7** | `tiered-quilt-chaos-and-dogfood` (Plan 5) | All chaos demo scenarios + new-developer-first-cargo a2o + grandma resilience + hoarder math + self-degraded a2o | Wave 6 |

### Cross-cutting tracks (parallel, not waves)

- **Drivers backlog**: `LocalHardwareTieredDriver`, `PeerCoordinatedDriver`,
  `FederatedDwellingDriver` — each scoped sub-plans, each can start once
  Wave 2 lands.
- **Manifest schema extensions**: `quilt_policy` block evolves across waves.
- **Doc + memory updates**: MEMORY.md cleanup; Garage→MinIO substrate
  memory correction; vocabulary.md cross-links.
- **a2o narrative authoring** (Opus work, per
  `feedback_a2o_narrative_is_opus_work`): story-harvest per wave.

### Dependency graph

```
Wave 0 ──┬──▶ Wave 1 ──▶ Wave 2 ──┬──▶ Wave 3 (shim)
         │                        │
         │                        ├──▶ Wave 4 ──▶ Wave 5 ──▶ Wave 6 ──▶ Wave 7
         │                        │
         │                        └──▶ (drivers backlog, parallel)
```

---

## 9. Decisions still required from operator

Before the delivery master writes sub-plans:

1. **Wave 0 scope confirmation.** Dedupe + rename is large. Bundle as one
   wave, or split into 0a (dedupe) and 0b (rename)?
2. **Archetype default catalog.** Who curates `wearable / mobile / edge /
   hub / archival` defaults for breach thresholds, scan cadence, driver
   lineup? Spec proposes initial values; operator signs off.
3. **Cost-class weight table.** Spec proposes ratios (warm:1.0, stocked:0.4,
   shelved-local:0.1, shelved-external:0.05). Operator may want different
   starting weights.
4. **Trust integration depth.** Wave 5/6 wires capability attestations to
   routing priority. How aggressively does trust-as-efficiency activate?
   Tunable or hard switch?
5. **MinIO bucket lifecycle policy.** Today's `sccache-elohim` bucket has
   no TTL. Wave 3 introduces shelve_after-driven cellaring; the bucket
   fills indefinitely unless lifecycle rules are added. Who owns that
   policy decision?
6. **DNA capacity check.** elohim DNA's `content_store_integrity` zome
   has ~70+ entry types in the visible enum. Tiered-quilt adds zero new
   entry types but extensions to `Commitment` (`action="custody-quilt"`)
   and `Attestation` (`category="storage-stewardship"`) should be audited
   for downstream impact before Wave 1.

---

## 10. Cross-references

- Vocabulary: `genesis/graphos/vocabulary.md`
- Self-healing dataplane (absorbed Plans 2–5):
  `genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md`
- Sweettest harness:
  `genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md`
- MinIO substrate (runbook, corrects Garage memory):
  `genesis/manifests/RUNBOOK-minio-sccache-2026-05-09.md`
- EPR substrate framing:
  `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`
- Three-truth-layer model: memory anchor `project_three_layer_truth_model`
- DePIN contracts as policy: memory anchor `project_depin_contracts_are_policy`
- Placement signals as shefa inputs: memory anchor `project_placement_signals_are_shefa_inputs`
- Trust as efficiency signal: memory anchor `project_trust_as_efficiency_signal`
- Reach earned at authoring: memory anchor `project_reach_earned_at_authoring`
- Values-forward disclosure: memory anchor `project_values_forward_disclosure_accountability`
- Ungrudging service: memory anchor `project_ungrudging_service`
- Family cluster as capability bar: memory anchor `project_household_fabric` +
  `project_subsume_g_f_a_via_it_just_works`

---

## 11. Open questions deferred to follow-on specs

- **Federated-dwelling driver design**: how does elohim-operator coordinate
  PVC-equivalent placement across multiple dwellings spanning jurisdictions?
  Named here as a clean integration point; the operator's policy reasoning
  is its own domain.
- **Peer-cellar URI resolution**: `peer-cellar://household/{H}` resolves to
  *which* peers in household H? Spec assumes Plan 1's diverse-placement
  selection extends; the exact match algorithm is in Plan 1's sub-plan.
- **IPFS / Arweave external-archive drivers**: scoped as backlog after
  MinIO ships in Wave 3.
- **Capability-attestation decay rates**: spec asserts attestations decay;
  the specific decay function is a separate signal-design question.
- **Recovery seed integration**: socially-derived security (memory anchor
  `project_socially_derived_security`) for the cold-tier identity case
  (e.g., if an external-archive bucket's credentials are lost) is out of
  scope here; deferred to recovery-protocol work.
