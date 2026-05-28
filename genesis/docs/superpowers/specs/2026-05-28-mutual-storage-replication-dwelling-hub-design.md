---
title: Mutual Storage Replication (Dwelling-Hub Tier) — First REA Compute-Commitment Instance
tier: superpowers/spec
status: Design (pre-implementation, post-brainstorm 2026-05-28)
created: 2026-05-28
authors: Matthew (operator decisions); spec drafted in brainstorm session 2026-05-28
pillar coupling: elohim (substrate primitive); infrastructure (storage tiering plane); imagodei (peer-household-hub binding); mishpat (Commitment authoring)
informed-by:
  - genesis/docs/superpowers/plans/2026-05-28-sprint1-zd-substrate-correct-deploy.md (Sprint 1 close-out — Z.D abandoned as first instance; storage replication recommended)
  - genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md (Sprint 2 — bounds_validator + signal_weight_registry primitives)
  - genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md (DistributionSummary, DistributionDetails, EprProjectionView, ProjectionCoverage already shipped)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md (quilt vocabulary; tiered storage; pre-implementation draft)
  - .claude/memory/project_hub_archetype_abstraction.md (Hub is abstract role, dwelling+collective+computed as kinds, stewards-not-members, peer↔hub encryption boundary)
  - .claude/memory/project_compute_commitment_first_instance_pivot.md (Sprint 1 pivot rationale; storage replication as next first-instance)
  - .claude/memory/project_household_resilience_unit.md (household as resilience primitive)
  - .claude/memory/project_three_layer_truth_model.md (DHT vs libp2p vs doorway scoping)
informs:
  - Follow-up sprint: collective_steward mode end-to-end + replicates-collective action
  - Follow-up sprint: encryption envelope + per-recipient key wrapping
  - Follow-up sprint: replicates-commons + commons-class content filter
  - Follow-up sprint: geographic-attestation infrastructure for fault-domain diversity
memory_anchors:
  - project_compute_commitment_first_instance_pivot
  - project_bounds_validator_pattern
  - project_rea_compute_commitment_primitive
  - project_hub_archetype_abstraction
  - project_signal_kind_extensible_protocol_class
---

# Mutual Storage Replication (Dwelling-Hub Tier) — First REA Compute-Commitment Instance

**Version:** 0.1
**Status:** Design (pre-implementation, post-brainstorm 2026-05-28)
**Owner:** Matthew (operator decisions); spec drafted in brainstorm session 2026-05-28

---

## 1. Why this spec exists

Sprints 1 and 2 (2026-05-28) landed the REA compute-commitment substrate primitives — `Mishpat::Commitment` with action discriminator, `bounds_validator::validate` with seven checks, `signal_weight_registry` with manifest-driven weights, `project_extension_signal` for standing-projection of string-named signal kinds, the donut-relevant standing pipeline — but did **not** prove the primitives on a real first instance. The Z.D substrate-correct-deploy framing was abandoned mid-Sprint 1 after a design conversation reframed deploy as authorship-delegation, not compute-delegation.

This sprint **proves the primitives** by shipping the first real first instance: **mutual storage replication between dwelling hubs** (households, in narrative). The user named three real REA compute-commitment use cases (mutual storage replication, doorway projection compute, distributed workloads); storage replication is the most ready and the most foundational — "social compute is your primary security."

### Architectural claim

Three load-bearing properties the substrate enforces:

1. **The graduation gradient.** Boundaries are intimate before institutional. Storage participation graduates from `free` (hyper-local self) → `dwelling` (intimate mutual aid) → `collective` (membership-mediated) → `commons` (universal substrate). Every hub contributes at all four tiers; the commons tier is what gives the substrate hyperscaler-class efficiency without hyperscaler-class capture.

2. **Donut economics, device-level.** A device's storage is allocated across the four tiers within a **constitutional donut**: DNA-locked FLOOR + CEILING constants form the donut walls; the elohim manifest declares specific ratios within the donut. `bounds_validator` clamps and enforces at every commitment author. No opt-out from any tier at the device level (the FLOOR is > 0 for dwelling and commons).

3. **Intent-first, observed-state-second.** Commitments express intent (notarized to DHT); existing inventory gossip + libp2p pull is observed state. Aggregate views juxtapose both: "Pledged 80 GB across commitments; actually hold 35 GB unique shard bytes (2.3x dedup multiplier)."

### What this sprint ships

| Layer | Deliverable |
|-------|-------------|
| Mishpat coordinator | New action `replicates-dwelling` with validator + integrity defense-in-depth |
| elohim-storage service | `replicates_dwelling_validator` (instance of bounds-validator-pattern) |
| elohim-storage substrate | `constitutional_ratio_registry`, `mutuality_audit_service`, `replication_prioritizer` |
| FeedbackSignal | New signal kind `reciprocity-imbalance` in elohim manifest signalKinds |
| Views | `PeerCapacityView` (per-device donut accounting); `HubCapacityView` (hub-aggregate paralleling `HubComputeAggregateView`); extensions to `DistributionSummary` + `DistributionDetails` + `HouseholdResilienceView` |
| DNA constants | Donut floor/ceiling in elohim-DNA |
| Operational table | `mutuality_audit_log` (Category C; sweep telemetry) |
| Routes | `GET /api/v1/peer/{peer_cid}/capacity`, `GET /api/v1/hub/{hub_id}/capacity`, `GET /api/v1/diagnostics/mutuality-audit?hub={hub_id}` |
| Tests | Unit + integration + sweettest + a2o coverage |

### What this sprint explicitly does NOT ship

- `provider_role=collective_steward` end-to-end: schema-reserved + validator stub; full membership-attestation chain in a follow-up sprint
- Encryption envelope + per-recipient key wrapping: separate sprint; commitments here are explicitly "storage availability, NOT read access"
- `replicates-collective` and `replicates-commons` actions: schema slots not yet authored
- Doorway-geography attestation infrastructure: fault-domain-diversity uses available signals; refines when geography lands
- Proof-of-storage cryptographic primitive: trust-and-debit suffices; future sprint
- Automatic shard re-balancing on `over_replicated` status: substrate signals waste, operator chooses to act
- Hub-stewardship constitutional access-path scenarios: `@wip` per `project_hub_archetype_abstraction` until simulation harness lands

---

## 2. Architectural shape

### 2.1 The graduation gradient

```
free (self) → dwelling (intimate mutual) → collective (membership mutual aid)
                                              ↓
                              commons (universal substrate, hyperscale efficiencies)
```

Each tier is a hub-kind expression of the same underlying substrate primitive:

| Tier | Hub kind | Reciprocity shape | First-class action this sprint |
|------|----------|-------------------|--------------------------------|
| free | n/a (self-storage) | n/a | n/a |
| dwelling | `dwelling` | Bilateral mutual (steward_mutual) OR asymmetric (collective_steward) | `replicates-dwelling` (this sprint) |
| collective | `collective` | Membership-mediated | `replicates-collective` (follow-up sprint) |
| commons | n/a (no recipient hub) | Universal constitutional contribution | `replicates-commons` (follow-up sprint) |

The graduation is also the device-to-hub gradient:
- Tier 0 / Computed kind: phone-only, single-device participation, no stewardship role
- Tier 1 / dwelling-hub (light): laptop or NUC plugged in at home, intermittent or always-on
- Tier 2 / dwelling-hub (dedicated): rack-shelf NUC, always-on, primary household storage
- Tier 3 / collective-hub (institutional): server-class hardware in a church, refugee camp, or regional mutual-aid center

Elohim agents help the operator pick which hub role(s) a device should take on given capabilities + social commitments. **This management is substrate-correct discernment, not a config decision** — an elohim-mediated conversation, recorded as discernment-attestations against the device's manifest.

### 2.2 Hub is a role, not a notarized entity

Per shipped `HubComputeAggregateView` and `project_hub_archetype_abstraction`:

> Hub is a *role* (dial-up-by-capability), not a notarized entity — substrate stays kind-agnostic; kind classification (dwelling/collective/computed) happens at the projection layer.

`HubKind` enum (already shipped): `"dwelling" | "collective" | "computed"`. `hubId` defaults to `peer_id` for single-device-participants (Computed kind) before hub-binding tables distinguish.

`HouseholdHub` and `CollectiveHub` are intentionally **separate implementations** at the projection layer (governance considerations differ in shape, not in settings — per memory). The substrate stays kind-agnostic at the DHT layer.

### 2.3 Hub-aware vocabulary in spec vs narrative

| Substrate (schemas, structs, action names) | Narrative (a2o features, UI strings, docs) |
|---|---|
| `dwelling-hub` | "household", "family" |
| `provider_dwelling_hub_id` | "the household offering backup" |
| `recipient_dwelling_hub_id` | "grandma's household" |
| `steward_mutual` | "bilateral family-to-family backup" |
| `collective_steward` | "church-collective backing your family" |
| `HubCapacityView` | "Your network storage" |

Both layers carry meaning. Substrate vocabulary preserves the abstraction. Narrative vocabulary preserves accessibility.

### 2.4 Three load-bearing properties (reprise)

**Donut economics, device-level.** A device's storage is allocated across the four tiers within a constitutional donut. DNA-locked FLOOR + CEILING constants are the walls; the elohim manifest declares specific ratios within them.

**Mutuality bilateral-by-reference, not bilateral-by-signature.** Dwelling A authors `replicates-dwelling provider_role=steward_mutual` naming `recipient=B`. Dwelling B independently authors a counter-commitment naming A. Substrate runs a periodic `mutuality_audit_service` sweep; if only one direction is found after manifest-declared `grace_period_days`, substrate emits `reciprocity-imbalance` FeedbackSignal naming the breaching party.

**Intent-first, observed-state-second.** Commitments are authored to the DHT (intent); existing `peer_blob_inventory` gossip + libp2p pull catch up the data plane (observed state). Aggregate views juxtapose pledged vs actually held — the storage-premium efficiency multiplier is visible.

---

## 3. P2P Design Gate output

Per `.claude/skills/p2p-design-gate/SKILL.md`.

### Entity: `Mishpat::Commitment` action vocabulary extension (`replicates-dwelling`)

- **Classification**: Notarized (A)
- **Justification**: A capacity commitment between two dwelling-hubs needs notarization — both parties (and the substrate's bounds_validator) must witness the symmetric pair; revocation must propagate; standing must debit on breach.
- **Content Address Strategy**: Content-Derived CID via Holochain ActionHash (inherited from existing `Mishpat::Commitment` entry type landed in Sprint 1).
- **Source of Truth**: Holochain DHT (existing `Mishpat::Commitment` entry type with new `action="replicates-dwelling"` discriminator). NO new entry type.
- **Coordinator Zome**: `mishpat::create_commitment` (already exists; dispatches on `action`)
- **Storage Projection**: existing `rea_commitments` SQLite table
- **HTTP Route**: `POST /api/v1/diagnostics/validate-replication-commitment` (Category C diagnostic only)
- **Anti-Pattern Check**: ✓ No new entry type (Mishpat at 11/~100; reusing existing); ✓ schema-first not route-first; ✓ ActionHash is identity; ✓ source of truth declared

### Entity: `replicates-dwelling.schema.json` payload

- **Classification**: Notarized (A) — payload of the Mishpat::Commitment entry
- **Source of Truth**: Holochain DHT (parent Commitment entry)
- **Coordinator validation**: `mishpat::validate_commitment_payload` → new branch `validate_replicates_dwelling`
- **HTTP Route**: none direct
- **Anti-Pattern Check**: ✓ no new entry type; ✓ schema declared first

### Entity: `PeerCapacityView`

- **Classification**: Operational (C)
- **Justification**: Recomputable at request time from `rea_commitments` filtered by provider, `peer_statuses` for raw capacity, elohim manifest for constitutional ratios. No persisted state.
- **Content Address Strategy**: keyed by `peer_cid` at request time
- **Source of Truth**: SQLite operational view, recomputable from DHT Commitments + `infrastructure:system-sample` graduation
- **Storage Projection**: none new; computed-on-read
- **HTTP Route**: `GET /api/v1/peer/{peer_cid}/capacity`
- **Anti-Pattern Check**: ✓ operational classification declared with rebuild strategy

### Entity: `HubCapacityView`

- **Classification**: Operational (C)
- **Justification**: Per-hub aggregate of member device `PeerCapacityView`s; mirrors `HubComputeAggregateView` shape exactly.
- **Source of Truth**: composed from `PeerCapacityView`s (per-device donut) + hub-membership graph (humans.household_id projection OR collective_participations)
- **HTTP Route**: `GET /api/v1/hub/{hub_id}/capacity`
- **Anti-Pattern Check**: ✓ pairs with existing `HubComputeAggregateView` pattern

### Entity: `DistributionSummary.projectionTier` (new field on existing schema)

- **Classification**: Operational (C) — additive extension; existing entity stays Category C
- **Justification**: Federation-level projection coverage classification; coarse heuristic from `projectorCount` + fault-domain-diversity.
- **HTTP Route**: rides on every existing `distribution: DistributionSummary` inline payload (no new route)
- **Anti-Pattern Check**: ✓ extends existing rather than parallel-view

### Entity: `mutuality_audit_log` (operational table)

- **Classification**: Operational (C) — sweep telemetry
- **Justification**: Records `mutuality_audit_service` sweep results; recomputable by re-running the sweep. No `dht_anchor_hash` — this is local audit log, not notarized.
- **Source of Truth**: local SQLite operational projection; rebuildable from sweep
- **Anti-Pattern Check**: ✓ source of truth declared; ✓ rebuild strategy named

### Entity: FeedbackSignal `reciprocity-imbalance` (new signal_kind extension)

- **Classification**: Notarized (A) — uses existing FeedbackSignal entry type with new `signal_kind` string per `project_signal_kind_extensible_protocol_class`
- **Source of Truth**: Holochain DHT (existing FeedbackSignal entry)
- **Coordinator Zome**: existing signal-emission path; new `signal_kind` registered in elohim manifest signalKinds + `signal_weight_registry`
- **Storage Projection**: existing `standing_view` via `project_extension_signal` (Sprint 2 T8)
- **Anti-Pattern Check**: ✓ no new entry type; ✓ standing impact via existing pipeline

### Entity: Constitutional ratio donut

- **Classification**: not an entity — protocol-level configuration
- **DNA-side**: hardcoded constants in elohim DNA (`COMMONS_MIN_FLOOR_PCT`, `COMMONS_MAX_CEILING_PCT`, `DWELLING_MIN_FLOOR_PCT`, `DWELLING_MAX_CEILING_PCT`, `FREE_MIN_FLOOR_PCT`, `FREE_MAX_CEILING_PCT`)
- **Manifest-side**: elohim domain manifest `constitutionalRatios` block within DNA-enforced walls
- **Validator**: `constitutional_ratio_registry::effective_ratios` clamps manifest to DNA walls; bounds_validator enforces on every replicates-* commitment author
- **Anti-Pattern Check**: ✓ no new entry type; ✓ donut shape prevents free-ride below floor + capture above ceiling

### Design constraints discovered

1. **Mutuality is bilateral-by-reference, not bilateral-by-signature.** Substrate finds the pair via DHT link traversal at audit time; neither party signs the other's payload (would require cross-agent signing primitives we don't have). Substrate emits `reciprocity-imbalance` FeedbackSignal if commitment exists in one direction without its counter past `grace_period_days`.
2. **Replication is intent-first, observed-state-second.** Commitment authored first (notarized intent); existing `peer_blob_inventory` gossip + libp2p pull catches up (observed state). Aggregate view juxtaposes both.
3. **Capacity sensing reuses existing `infrastructure:system-sample` observation pipeline.** No new sensors.
4. **Mishpat at 11/~100 entry types** — plenty of headroom but we're not using it. The action is discriminator-only on existing `Mishpat::Commitment`.
5. **Hub is a role, not a notarized entity.** Substrate stays kind-agnostic at DHT layer; kind classification (dwelling/collective/computed) happens at projection layer.
6. **Constitutional ratio donut is device-level**, not hub-level. A device may host multiple hub roles; donut walls apply to the device's total allocation.
7. **Encryption is a separate sprint.** Commitments here are explicitly "storage availability, NOT read access." The peer↔hub end-to-end encryption boundary terminates at the hub↔spoke edge per `project_hub_archetype_abstraction`; the encryption-layer sprint is the next-sprint pre-condition for production-readiness.

---

## 4. `replicates-dwelling` commitment payload

Lives at `elohim/sdk/schemas/v1/commitments/replicates-dwelling.schema.json`. Travels inside `Mishpat::Commitment.payload_json`.

```json
{
  "$id": "epr:schema:replicates-dwelling",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ReplicatesDwellingCommitment",
  "description": "Source of truth: Holochain DHT (Mishpat zome, existing Mishpat::Commitment entry type with action='replicates-dwelling'). Storage-availability commitment between two dwelling-hubs (households). Does NOT presuppose recipient can decrypt — that's the encryption-layer's concern (separate sprint).",
  "type": "object",
  "required": [
    "action",
    "provider_dwelling_hub_id",
    "recipient_dwelling_hub_id",
    "provider_role",
    "capacity_bytes",
    "scope_filter",
    "valid_from",
    "valid_until",
    "grace_period_days",
    "rotation_ttl_days",
    "ratio_attestation"
  ],
  "additionalProperties": false,
  "properties": {
    "action": { "const": "replicates-dwelling" },
    "provider_dwelling_hub_id": {
      "type": "string",
      "minLength": 1,
      "description": "Stable hub id of the dwelling-hub offering storage capacity. Defaults to peer_id for Computed-kind single-device participation."
    },
    "recipient_dwelling_hub_id": {
      "type": "string",
      "minLength": 1,
      "description": "Stable hub id of the dwelling-hub whose content is being backed."
    },
    "provider_role": {
      "type": "string",
      "enum": ["steward_mutual", "collective_steward"],
      "description": "'steward_mutual' = bilateral mutual aid between dwelling-hubs (counter-commitment expected within grace_period_days). 'collective_steward' = asymmetric resilience burst on behalf of a collective-hub the recipient is a member of (no counter-commitment; consideration is collective membership)."
    },
    "via_collective_hub_id": {
      "type": ["string", "null"],
      "description": "Required when provider_role='collective_steward'. Stable hub id of the collective-hub the provider is stewarding on behalf of. Provider must hold a valid 'attestation:collective-steward' for this collective; recipient must be a member."
    },
    "capacity_bytes": {
      "type": "integer",
      "minimum": 1,
      "description": "Bytes of provider storage committed to recipient's content. The cumulative-bytes ceiling enforced by bounds_validator's rate_within_limit check."
    },
    "scope_filter": {
      "type": "object",
      "additionalProperties": false,
      "description": "Curation policy for which of recipient's content qualifies for this commitment's shard slots. NOT a read-access gate — that's the encryption envelope layer (separate sprint).",
      "properties": {
        "epr_kinds": {
          "type": "array",
          "items": {
            "type": "string",
            "enum": ["Content", "Manifest", "Claim", "Observation", "EconomicEvent", "Commitment", "Attestation", "Delegation", "FeedbackSignal"]
          },
          "description": "Allow-list of EprKinds. Empty/absent = all kinds."
        },
        "bytes_per_blob_max": {
          "type": "integer",
          "minimum": 1,
          "description": "Per-blob size ceiling. Protects against single huge blobs eating the whole commitment budget."
        },
        "requires_attestations": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Optional. Only host shards for content carrying these attestation kinds. Abuse-resistance filter."
        },
        "kinds_excluded": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Optional deny-list."
        }
      }
    },
    "valid_from": { "type": "string", "minLength": 1, "description": "ISO 8601." },
    "valid_until": { "type": "string", "minLength": 1, "description": "ISO 8601." },
    "grace_period_days": {
      "type": "integer",
      "minimum": 1,
      "description": "Days the substrate waits for the counter-commitment before emitting reciprocity-imbalance signal. Only relevant when provider_role='steward_mutual'."
    },
    "rotation_ttl_days": {
      "type": "integer",
      "minimum": 1,
      "description": "Rotation TTL for the provider's signing key (bounds_validator key-rotation check)."
    },
    "ratio_attestation": {
      "type": "object",
      "required": ["commons_pct", "dwelling_pct", "collective_pct", "free_pct", "effective_ratio_cid"],
      "additionalProperties": false,
      "description": "Provider's claim of their device's donut ratios at author time. bounds_validator verifies (a) percentages sum to 100, (b) each within DNA floor/ceiling, (c) effective_ratio_cid matches current elohim manifest commit hash.",
      "properties": {
        "commons_pct":          { "type": "integer", "minimum": 0, "maximum": 100 },
        "dwelling_pct":         { "type": "integer", "minimum": 0, "maximum": 100 },
        "collective_pct":       { "type": "integer", "minimum": 0, "maximum": 100 },
        "free_pct":             { "type": "integer", "minimum": 0, "maximum": 100 },
        "effective_ratio_cid":  { "type": "string", "minLength": 1 }
      }
    }
  }
}
```

### Bounds check semantics

For `replicates-dwelling`, the substrate-wide checks delegate to Sprint 2's `bounds_validator::validate` with the following specializations:

| Check | Specialization for `replicates-dwelling` |
|-------|-----------------------------------------|
| `commitment_found` | Standard (CommitmentFetcher) |
| `not_revoked` | Standard |
| `active` | now ∈ [valid_from, valid_until] |
| `scope_includes_event` | event.epr_kind ∈ scope_filter.epr_kinds AND event.bytes ≤ scope_filter.bytes_per_blob_max AND event.recipient_dwelling_hub_id == commitment.recipient_dwelling_hub_id AND scope_filter.requires_attestations subset of event.attestations |
| `reach_ceiling_ok` | n/a for storage-availability commitments (handled inside scope_includes_event); always true |
| `rate_within_limit` | **cumulative_shard_bytes_held ≤ capacity_bytes** (NOT events-per-hour) |
| `key_rotation_current` | Standard |

The Sprint 2 `bounds_validator::validate` signature is unchanged; the per-instance `replicates_dwelling_validator` handles event projection + specialization.

### New `BoundsViolation` variant

`ConstitutionalRatioBreach` — emitted when `ratio_attestation` mismatches current `effective_ratios` OR adding this commitment's `capacity_bytes` to the provider's existing pledges would push any tier outside the donut walls.

---

## 5. Donut economics

### 5.1 DNA constants (immutable per protocol version)

Added to `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`:

```rust
pub const COMMONS_MIN_FLOOR_PCT: u8 = 10;   // Every device contributes ≥10% to commons
pub const COMMONS_MAX_CEILING_PCT: u8 = 60;
pub const DWELLING_MIN_FLOOR_PCT: u8 = 10;  // No opt-out from dwelling tier
pub const DWELLING_MAX_CEILING_PCT: u8 = 80;
pub const FREE_MIN_FLOOR_PCT: u8 = 5;       // Always reserve some for self
pub const FREE_MAX_CEILING_PCT: u8 = 70;
```

`COLLECTIVE_*` constants are absent in Sprint 3 — collective tier ships in a follow-up sprint; until then `collective_pct` lives unbounded between zero and the residual.

### 5.2 Manifest declaration

`elohim/sdk/domains/elohim/manifest.json` gains:

```json
"constitutionalRatios": {
  "description": "Protocol-version-stable ratios for the storage donut. Values clamped to DNA-enforced floor/ceiling at validation time. Sum must equal 100.",
  "version": 1,
  "commons_pct": 20,
  "dwelling_pct": 40,
  "collective_pct": 25,
  "free_pct": 15
}
```

Schema constraint added to `app-manifest.schema.json` — optional `constitutionalRatios` object with the same field shape.

### 5.3 Effective ratio computation

New service `elohim/elohim-storage/src/services/constitutional_ratio_registry.rs` mirrors `signal_weight_registry` shape:

```rust
pub struct EffectiveRatios {
    pub commons_pct: u8,
    pub dwelling_pct: u8,
    pub collective_pct: u8,
    pub free_pct: u8,
    pub manifest_cid: String,
}

pub fn effective_ratios() -> EffectiveRatios {
    let manifest = read_elohim_manifest();
    let commons  = clamp(manifest.commons_pct,  COMMONS_MIN_FLOOR_PCT,  COMMONS_MAX_CEILING_PCT);
    let dwelling = clamp(manifest.dwelling_pct, DWELLING_MIN_FLOOR_PCT, DWELLING_MAX_CEILING_PCT);
    let free     = clamp(manifest.free_pct,     FREE_MIN_FLOOR_PCT,     FREE_MAX_CEILING_PCT);
    let collective = 100 - commons - dwelling - free;
    EffectiveRatios { commons_pct: commons, dwelling_pct: dwelling, collective_pct: collective, free_pct: free, manifest_cid: current_manifest_cid() }
}
```

`OnceLock`-cached per process; rebuilt on manifest reload (out-of-Sprint-3 detail).

### 5.4 Donut check at commit author time

When a peer authors a new `replicates-dwelling` commitment with `capacity_bytes=X`:

1. Look up provider's current donut state from `PeerCapacityView`:
   - `total_raw_bytes` from latest `infrastructure:system-sample`
   - `pledged_dwelling_bytes`, `pledged_collective_bytes` from sum of active commitments per tier
   - `pledged_commons_bytes` is 0 in this sprint (replicates-commons authoring is follow-up); declared commons intent comes from `ratio_attestation.commons_pct`
2. Compute proposed-pct-per-tier with X included.
3. Apply `effective_ratios` as walls — **ceilings enforced via actual pledged commitments; floors enforced via declared ratio_attestation**:
   - **Ceiling checks** (pledged-based):
     - `dwelling_pct_pledged ≤ effective_ratios.dwelling_pct` → reject if breached
     - `collective_pct_pledged ≤ effective_ratios.collective_pct` → reject if breached (Sprint 3: collective always 0 since `replicates-collective` is follow-up; pass-through)
     - `free_pct_remaining ≥ FREE_MIN_FLOOR_PCT` → reject if pushed below
   - **Floor checks** (declared-based for Sprint 3):
     - `ratio_attestation.commons_pct ≥ effective_ratios.commons_pct` → reject if declaration below floor
     - `ratio_attestation.dwelling_pct ≥ DWELLING_MIN_FLOOR_PCT` → reject if declaration below floor
4. `ratio_attestation` block must match computed `effective_ratios` (per-tier values clamped to DNA walls). Mismatch → reject.

**Sprint 3 floor-vs-ceiling asymmetry — explicit design choice.** Until `replicates-commons` ships in the follow-up sprint, no peer can author commons commitments, so the commons floor cannot be satisfied by active pledges. The substrate accepts the peer's `ratio_attestation.commons_pct` declaration as a *commitment of intent* in lieu of an active replicates-commons commitment. This avoids a chicken-and-egg lockout (fresh peer authoring first replicates-dwelling would breach commons floor under a strict pledged-only rule).

The follow-up sprint that lands `replicates-commons` MUST upgrade the floor check to require backing pledges (commons declarations un-backed by active commitments fail bounds_validator). Mark this in the spec close-out and in `project_compute_commitment_first_instance_pivot` memory so the follow-up sprint doesn't ship without closing this gap.

`ConstitutionalRatioBreach` is the BoundsViolation variant; standing-debit via existing `bad-custody` FeedbackSignal weight (registered in Sprint 2).

---

## 6. Mutuality enforcement + grace period

### 6.1 `mutuality_audit_service`

New service in elohim-storage; runs periodically (default daily, manifest-tunable):

```rust
pub struct MutualityAuditService {
    pub pool: DbPool,
    pub hc_client: Arc<HcClient>,
    pub cadence: chrono::Duration,
}

impl MutualityAuditService {
    /// Walk every active replicates-dwelling commitment where provider_role=steward_mutual.
    /// For each:
    ///   1. Query DHT for the counter-commitment (recipient_dwelling_hub_id → provider_dwelling_hub_id).
    ///   2. Matched + active → reciprocity_status=Matched.
    ///   3. Missing AND days_since_authored ≤ grace_period_days → reciprocity_status=Pending.
    ///   4. Missing AND days_since_authored > grace_period_days → reciprocity_status=Breached;
    ///      emit FeedbackSignal{signal_kind: "reciprocity-imbalance", target: recipient_dwelling_hub_id}.
    ///   5. Update mutuality_audit_log row.
    /// provider_role=collective_steward commitments are skipped (no bilateral check).
    pub async fn run_sweep(&self) -> Result<SweepReport, StorageError> { ... }
}
```

Substrate-correct: reads DHT for counter-commitment lookup; emits FeedbackSignal via conductor. Idempotent (running twice produces the same audit log state).

### 6.2 `mutuality_audit_log` table

Migration:
```sql
-- Source of truth: local SQLite operational projection; rebuildable by re-running the sweep.
-- No dht_anchor_hash — this is sweep telemetry, not notarized.
CREATE TABLE mutuality_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    commitment_cid TEXT NOT NULL,
    provider_dwelling_hub_id TEXT NOT NULL,
    recipient_dwelling_hub_id TEXT NOT NULL,
    reciprocity_status TEXT NOT NULL,   -- 'Matched' | 'Pending' | 'Breached'
    days_since_authored INTEGER NOT NULL,
    grace_period_days INTEGER NOT NULL,
    signaled_at TEXT,                    -- ISO8601; NULL if no signal emitted this sweep
    swept_at TEXT NOT NULL
);
CREATE INDEX idx_mutuality_audit_commitment ON mutuality_audit_log(commitment_cid);
CREATE INDEX idx_mutuality_audit_recipient ON mutuality_audit_log(recipient_dwelling_hub_id);
```

### 6.3 `reciprocity-imbalance` signal_kind

Added to `elohim/sdk/domains/elohim/manifest.json` signalKinds (already extended by Sprint 2 to accept `debit_weight` + `decay_days`):

```json
"reciprocity-imbalance": {
  "description": "Provider-dwelling-hub authored a replicates-dwelling commitment in steward_mutual mode; counter-commitment from recipient never arrived within grace_period_days, or was unilaterally revoked. Substrate emits this signal naming the breaching party (the one who failed to author the counter). Standing-debit moderate.",
  "target_kinds": ["dwelling-hub", "agent"],
  "evidence_required": true,
  "standing_impact_allowed": ["consequential"],
  "debit_weight": 8,
  "decay_days": 60
}
```

Standing-projection via `project_extension_signal` (Sprint 2 T8) is automatic.

### 6.4 Late counter-arrival semantics

The substrate does NOT auto-rescind the signal — standing decay (`decay_days: 60`) handles gradual recovery. If a late counter is genuine, `mutuality_status` flips to `Matched` on the next sweep; no further signals emitted. Original signal continues to decay.

### 6.5 Elohim-operator-shaped substrate aggregator

The `mutuality_audit_service` is the **first concrete instance** of a per-scale aggregator shape that extends to higher tiers:
- Dwelling scale: `mutuality_audit_service` (this sprint)
- Collective scale: `collective_membership_audit_service` (follow-up sprint; member-attestation freshness, dues, governance participation)
- Commons scale: `commons_contribution_audit_service` (follow-up sprint; every device contributing to commons; free-rider detection)

All three feed FeedbackSignals through the same `signal_weight_registry` + `project_extension_signal` standing pipeline. **Same primitive, different scale.**

---

## 7. Views

### 7.1 `PeerCapacityView` (per-device donut accounting)

`elohim/sdk/schemas/v1/views/peer-capacity-view.schema.json`:

```json
{
  "$id": "epr:schema:view/peer-capacity",
  "title": "PeerCapacityView",
  "description": "Per-peer-device storage capacity rollup: raw hardware, free-for-self, pledged-per-tier (with multi-reach dedup at actual-bytes level), constitutional ratio compliance. Source of truth: computed projection from infrastructure:system-sample (raw capacity), Mishpat::Commitment entries (pledges), peer_blob_inventory (actually held). Operational Category C — no persisted entity; recomputable.",
  "type": "object",
  "required": ["peerCid", "computedAt", "totalRawBytes", "pledges", "actuallyHeld", "ratioCompliance"],
  "additionalProperties": false,
  "properties": {
    "peerCid": { "type": "string", "minLength": 1 },
    "computedAt": { "type": "string", "description": "ISO 8601." },
    "totalRawBytes": { "type": "integer", "minimum": 0 },
    "pledges": {
      "type": "object",
      "required": ["dwellingBytes", "collectiveBytes", "commonsBytes", "totalPledgedBytes"],
      "additionalProperties": false,
      "properties": {
        "dwellingBytes":   { "type": "integer", "minimum": 0 },
        "collectiveBytes": { "type": "integer", "minimum": 0 },
        "commonsBytes":    { "type": "integer", "minimum": 0 },
        "totalPledgedBytes": { "type": "integer", "minimum": 0, "description": "Sum of per-tier pledges. May exceed totalRawBytes when commitments overlap on shards." },
        "pledgesByRecipient": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["tier", "recipientHubId", "commitmentCid", "capacityBytes"],
            "additionalProperties": false,
            "properties": {
              "tier":            { "type": "string", "enum": ["dwelling", "collective", "commons"] },
              "recipientHubId":  { "type": "string" },
              "commitmentCid":   { "type": "string" },
              "capacityBytes":   { "type": "integer", "minimum": 0 },
              "providerRole":    { "type": "string", "enum": ["steward_mutual", "collective_steward"], "description": "Only present for dwelling tier." }
            }
          }
        }
      }
    },
    "actuallyHeld": {
      "type": "object",
      "required": ["uniqueShardBytes", "freeBytesRemaining", "fragmentationEstimate"],
      "additionalProperties": false,
      "properties": {
        "uniqueShardBytes":    { "type": "integer", "minimum": 0, "description": "Distinct shard bytes held; shard satisfying multiple commitments counts once." },
        "freeBytesRemaining":  { "type": "integer", "description": "totalRawBytes - uniqueShardBytes. Negative when peer is over-fetching." },
        "fragmentationEstimate": { "type": "number", "minimum": 0, "maximum": 1 }
      }
    },
    "ratioCompliance": {
      "type": "object",
      "required": ["effectiveRatios", "currentRatios", "compliantWithDonut", "violations"],
      "additionalProperties": false,
      "properties": {
        "effectiveRatios": {
          "type": "object",
          "required": ["commonsPct", "dwellingPct", "collectivePct", "freePct", "manifestCid"]
        },
        "currentRatios": {
          "type": "object",
          "required": ["commonsPct", "dwellingPct", "collectivePct", "freePct"]
        },
        "compliantWithDonut": { "type": "boolean" },
        "violations": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["tier", "violationKind", "currentPct", "boundPct"],
            "properties": {
              "tier":          { "type": "string", "enum": ["commons", "dwelling", "collective", "free"] },
              "violationKind": { "type": "string", "enum": ["below_floor", "above_ceiling", "below_manifest_target", "above_manifest_target"] },
              "currentPct":    { "type": "integer" },
              "boundPct":      { "type": "integer" }
            }
          }
        }
      }
    }
  }
}
```

Route: `GET /api/v1/peer/{peer_cid}/capacity`.

Authenticated to the peer's own agent + operator. Other peers querying receive a **redacted** view (just `totalRawBytes` + `ratioCompliance.compliantWithDonut`) — donut-compliance is public; per-recipient breakdown is private.

### 7.2 `HubCapacityView` (hub-aggregate, mirrors `HubComputeAggregateView`)

`elohim/sdk/schemas/v1/views/hub-capacity-view.schema.json`:

```json
{
  "$id": "epr:schema:view/hub-capacity",
  "title": "HubCapacityView",
  "description": "Hub-level storage-capacity aggregate. Sums per-device PeerCapacityView across all devices belonging to a hub. Hub is a *role* (per project_hub_archetype_abstraction); substrate stays kind-agnostic. Source of truth: hub-membership graph (notarized humans.household_id projection OR collective_participations) + per-device PeerCapacityView. Operational Category C — reconstructed per request; not persisted.",
  "type": "object",
  "required": ["hubId", "hubKind", "memberDeviceCount", "capacity"],
  "additionalProperties": false,
  "properties": {
    "hubId":             { "type": "string" },
    "hubKind":           { "type": "string", "enum": ["dwelling", "collective", "computed"] },
    "displayLabel":      { "type": ["string", "null"] },
    "memberDeviceCount": { "type": "integer", "minimum": 0 },
    "capacity": {
      "oneOf": [
        { "type": "null" },
        {
          "type": "object",
          "required": ["totalRawBytes", "pledges", "actuallyHeld", "ratioCompliance"],
          "description": "Aggregate across all member devices. Null when no devices have a current sample."
        }
      ]
    }
  }
}
```

Route: `GET /api/v1/hub/{hub_id}/capacity`.

### 7.3 Extensions to existing topology views

Per Section 6 of the brainstorm — no new parallel views. Three small extensions:

**`DistributionSummary` (light-up-topology, inline badge):**
- Add `projectionTier: "local" | "regional" | "global"` (federation-level coverage classification; coarse heuristic from `projectorCount` + fault-domain-diversity)
- Extend `replicaHealth` enum with `"over_replicated"` (waste-recovery signal)

**`DistributionDetails` (lazy-fetched):**
- Add `replicationCommitments` array (active replicates-* commitments whose recipient + scope_filter cover this content; tier + provider + role)
- Add `faultDomainDiversity` object (distinctHouseholdCount, distinctCollectiveCount, distinctRegionCount, singleFaultDomainRisk, faultModesEvaluated)
- Extend `replica-peer.schema.json` items with `shardsHeld` + `shardsByEncoding` (RS-aware shard count per peer)

**`HouseholdResilienceView`:**
- Add `commitmentBackedReplication` object (counts per tier + totalPledgedBytes; complements existing householdsStewarding count)

All extensions backwards-compatible (new fields, no removals).

### 7.4 Storage premium honest accounting

The honest dual-accounting `totalPledgedBytes: 80 GB / uniqueShardBytes: 35 GB / totalRawBytes: 100 GB` story makes the **2.3x effective pledged capacity per byte of raw disk** visible. This is the storage-premium efficiency mechanism — dedup-at-shard, multi-commitment-counts-same-bytes — that lets dwelling-hubs decompose hyperscaler power without burdening any one steward.

---

## 8. Replication data plane (existing primitives, no new wire protocol)

### 8.1 What already exists (no changes)

| Primitive | Source | Role |
|-----------|--------|------|
| `peer_blob_inventory` table | `db/diesel_schema.rs:1275` | Observed state per peer |
| `peer_inventory_cursor` | same | Per-peer gossip cursor |
| Gossipsub topic `elohim/inventory/blob` | `p2p/inventory_gossip.rs:25` | Periodic broadcast |
| `inventory_broadcaster.rs` | `p2p/inventory_broadcaster.rs` | Authoring-side |
| `blob_fetch.rs` | `p2p/blob_fetch.rs` | Fetcher-side |
| Reed-Solomon RS-4-7 | `sharding.rs` | Already shipped; threshold 64MB |
| `peer_identity_bindings` | DHT projection | peer_id ↔ agent_cid ↔ hub linkage |
| On-connect ListContent kick | Light-up-topology | Cold-peer gap 60s → <10s |
| GET-time peer-fallback for blobs | Light-up-topology | Page survives peer-offline |

### 8.2 What Sprint 3 changes (three additions, all read-side)

1. **`replication_prioritizer` service** — scores incoming inventory gossip against the local peer's active `replicates-*` commitments:
   - HIGH: blob's recipient (derivable from peer_identity_bindings → hub lookup) matches an active commitment recipient AND scope_filter accepts blob's epr_kind + size
   - MEDIUM: commons-tier eligible (deferred; this sprint only implements dwelling-tier path)
   - SKIP: no matching commitment

2. **`unique_shard_accounting` query helper** — used by `peer_capacity_service` to compute `uniqueShardBytes` (cross-references `peer_blob_inventory` with sharding service's blob-to-shard derivation).

3. **`shard_recovery_orchestrator` hook** — when `DistributionSummary.replicaHealth` flips to `at_risk` for content covered by a local commitment, emit `infrastructure:replication-shortfall` observation (graduates via existing observation pipeline; triggers proactive shard-fetch).

### 8.3 Explicit non-changes

- No new libp2p protocol
- No new gossipsub topic
- No new blob storage interface
- No proof-of-storage primitive
- No automatic shard re-balancing

### 8.4 Dataflow walkthrough

Dwelling-hub H1 authors blob B (encrypted via next-sprint envelope; assume done) with `reachClass=household`. RS-4-7 → 11 shards.

1. H1's node: `blob_store.put` → shard_service encodes → 11 shard CIDs land in local pantry → `inventory_broadcaster` gossips.
2. Peer P2 (holds active `replicates-dwelling provider_role=steward_mutual` naming H1, capacity 50GB, scope accepts Content kind):
   - Inventory subscriber receives gossip with H1's shard CIDs.
   - `replication_prioritizer` scores HIGH (H1 is commitment recipient; blob within scope).
   - P2 fetches via existing `blob_fetch.rs`.
   - `peer_blob_inventory` updates; `peer_capacity_service` next-query reflects new uniqueShardBytes.
3. Reader grandma (encryption layer next sprint): queries `GET /api/v1/content/{B}/resiliency` → `DistributionDetails` shows replicaPeers including P2 + `replicationCommitments` proving authorization → fetches → encryption-layer-next-sprint unwraps key → grandma reads plaintext.
4. Resilience visibility for H1: `GET /api/v1/content/{B}/resiliency` shows `protectionStatus: protected, currentShardsActive: 11/11, commitmentBackedReplication.dwellingCommitments: 3, singleFaultDomainRisk: false`.
5. Disaster (hurricane, 8 of 11 shards lost): `protectionStatus` flips to `at_risk`; `shard_recovery_orchestrator` emits `replication-shortfall` observation. A collective-hub in another region with `provider_role=collective_steward` commitment (future sprint) absorbs burst, fetches 8 missing shards. H1 returns to `protected` within hours.

---

## 9. Testing strategy

### 9.1 Unit tests (host-target, fast)

**Mishpat coordinator `validate_replicates_dwelling`**:
- Well-formed steward_mutual payload validates
- Well-formed collective_steward payload validates (with via_collective_hub_id present)
- Each required field missing → reject (parameterized)
- Wrong action discriminator → reject
- collective_steward without via_collective_hub_id → reject
- scope_filter with unknown epr_kinds → reject
- ratio_attestation pct sum ≠ 100 → reject
- ratio_attestation values outside DNA floor/ceiling → reject

**Mishpat integrity `validate_replicates_dwelling_entry`** (substring-heuristic per Sprint 1):
- Empty action/payload → reject
- Non-object metadata → reject
- action=replicates-dwelling with empty recipient_dwelling_hub_id → reject

**elohim-storage `replicates_dwelling_validator::validate`**:
- Schema violation rejected before bounds check (cheap-first)
- Bounds violation propagates from bounds_validator
- ConstitutionalRatioBreach when ratio_attestation mismatch
- provider_role=collective_steward returns CollectiveStewardModeNotYetSupported (no silent pass-through)

**`constitutional_ratio_registry::effective_ratios`**:
- Manifest values within walls pass through unchanged
- Below floor clamped to floor
- Above ceiling clamped to ceiling
- Sum equals 100 (collective_pct as residual)

**`mutuality_audit_service::run_sweep`** (HcClient + DbPool mocks):
- Matched: no signal
- Pending (within grace): no signal
- Breached: reciprocity-imbalance emitted naming recipient
- Stewarded-mode skipped from bilateral check

**`replication_prioritizer::score_advertised_blob`**:
- Blob from recipient + within scope → HIGH
- Commons-eligible → MEDIUM (deferred path, validates the return type)
- No matching commitment → SKIP

### 9.2 Integration tests

**`replicates_dwelling_integration.rs`**:
- Donut compliance: peer with 100GB raw + multi-tier pledges within walls passes; pushing past ceiling rejects
- Multi-reach blob accounting: blob with `reach={dwelling, collective}` counts toward both tier budgets at view level, single-counts in uniqueShardBytes
- mutuality_audit_log table populated across one sweep

**`peer_capacity_view_integration.rs`**:
- Empty state → zeroed view with four tier slots
- Realistic state (3 dwelling + 1 collective + 1 commons) → correct rollups
- Over-pledge → negative freeBytesRemaining without crash

**`distribution_view_extensions_integration.rs`**:
- New projectionTier computed for 1/3/6 projector counts
- New over_replicated variant fires when active shards > 2× minShardsForRecovery
- singleFaultDomainRisk true when all shards in one hub; false when spread

**`hub_capacity_view_integration.rs`**:
- Single-device dwelling-hub: HubCapacityView mirrors PeerCapacityView
- Multi-device dwelling-hub: capacity aggregates across devices

### 9.3 Sweettest (two-conductor)

**`replicates_dwelling_substrate_correct_test.rs`**:
- Two agents on two conductors; A is dwelling-hub H1 steward; B is dwelling-hub H2 steward
- A authors `replicates-dwelling provider_role=steward_mutual` naming H2 with capacity 50GB
- `exchange_peer_info + await_consistency`
- B authors counter-commitment naming H1 within grace_period; `mutuality_audit_service.run_sweep` shows Matched
- Authoring fails when ratio_attestation breaches DNA floor
- A tries to author second commitment that would push dwelling_pct above ceiling → ConstitutionalRatioBreach

**`replicates_dwelling_disaster_burst_test.rs`** (deferred, `#[ignore]`):
- Documents end-to-end collective-steward burst flow that substrate is structurally ready for
- Reactivated when collective_steward mode lands

### 9.4 A2o scenarios (Opus narrative work per `feedback_a2o_narrative_is_opus_work`)

**`genesis/a2o/features/storage/household-resiliency-handshake.feature`** — two households commit to back each other; bilateral counter arrives within grace; counter never arrives → reciprocity-imbalance signal fires.

**`genesis/a2o/features/storage/constitutional-ratio-enforcement.feature`** — household authors first commitment compliantly; tries to author breaching commitment; operator views PeerCapacityView with 2.3x storage premium visible.

**`genesis/a2o/features/storage/disaster-burst-resilience.feature`** (`@wip-collective-steward`) — forward-looking; documents the disaster-burst flow.

Narrative copy: "household", "family", "grandma", "neighborhood", "church" — never "Mishpat", "Qahal", "dwelling-hub" in feature copy (substrate names stay in stepdefs; narrative stays accessible).

### 9.5 Manual operator-watch acceptance

After landing, operator confirms:
- `pnpm run hc:start:seed` brings up local stack with 2+ simulated dwelling-hubs
- `curl http://localhost:8090/api/v1/peer/<peer_cid>/capacity | jq .ratioCompliance` shows `compliant=true`
- Manual `replicates-dwelling` author via CLI; PeerCapacityView reflects; mutuality_audit log shows Pending
- Stop counterparty node; advance clock past grace_period; observe reciprocity-imbalance signal in stream
- Author deliberately ratio-breaching commitment; observe 400 with ConstitutionalRatioBreach

### 9.6 Coverage bar

| Layer | Bar |
|-------|-----|
| Unit | ≥90% lines on validator code, registry, audit service |
| Integration | All 4 stories pass |
| Sweettest | 1 test pass; 1 ignored with documented reason |
| A2o | 3 features author + parse cleanly; first two run green |
| Manual | All 5 signals confirmed by operator |

---

## 10. Out of scope (with explicit follow-up sprint references)

| Out of scope | Sprint that lands it |
|--------------|---------------------|
| `provider_role=collective_steward` end-to-end | Follow-up: collective-steward attestation chain + replicates-collective action |
| Encryption envelope + per-recipient key wrapping | Follow-up: encryption-envelope-and-key-custody sprint (Sprint N+1) |
| `replicates-collective` action | Follow-up: collective-tier sprint |
| `replicates-commons` action + commons-class content filter | Follow-up: commons-tier sprint |
| Doorway-geography attestation infrastructure | Follow-up: when geographic-attestation infrastructure lands; fault-domain-diversity refines |
| Proof-of-storage cryptographic primitive | Future: trust-and-debit suffices for now |
| Automatic shard re-balancing on `over_replicated` | Future: substrate signals; operator chooses |
| Hub-stewardship constitutional access-path scenarios | `@wip` per memory until simulation harness lands |
| Seeder-based substrate-correct content publish (retiring Z.1 stage-spa-blob) | Separate small sprint; unrelated to this one |

---

## 11. Follow-up sprints implied

After this sprint lands:

1. **Encryption envelope + key custody** — peer↔hub end-to-end encryption boundary; key custody substrate; reach-as-key-wrap-set semantics. **Pre-condition for any production-readiness of this sprint's work.**
2. **`replicates-collective` action + collective-tier handshake** — membership-attestation chain; collective_steward mode end-to-end; collective_membership_audit_service.
3. **`replicates-commons` action + commons-class filter** — universal contribution; commons_contribution_audit_service (free-rider detection).
4. **Doorway projection compute agreements** — second compute-commitment instance per `project_compute_commitment_first_instance_pivot`.
5. **Distributed workloads** — third compute-commitment instance per same memory.
6. **Seeder-based content publish** — retire `stageSpaBlobs` Z.1 anti-pattern; unrelated to this sprint.

---

## 12. Acceptance signals (sprint-complete bar)

The sprint is "done" when:

1. Two dwelling-hubs on local hc-stack can author `replicates-dwelling steward_mutual` commitments to each other; mutuality_audit_service shows Matched within one sweep
2. Authoring a commitment that breaches the DNA floor (e.g., dropping commons below 10%) returns 400 with ConstitutionalRatioBreach + manifest_cid in the error
3. `GET /api/v1/peer/<peer_cid>/capacity` returns honest dual-accounting (pledged vs uniqueShardBytes) with at least one peer demonstrating >1x multiplier
4. Stopping a counterparty's commitment authoring past grace_period emits `reciprocity-imbalance` signal; standing_view debit visible
5. `DistributionSummary` on every EPR/content response carries `projectionTier`; existing callers unbroken
6. `HouseholdResilienceView` carries `commitmentBackedReplication`; existing callers unbroken
7. Three a2o features author and the first two run green
8. Sweettest two-conductor test passes; disaster-burst test ignored with documented reason
9. Memory entries land: `project_dwelling_hub_replication_pattern` (the pattern this sprint proves) + close-out notes referencing this spec

---

## 13. Memory references

- `[[project_compute_commitment_first_instance_pivot]]` — Sprint 1 close-out + storage replication as next first instance
- `[[project_bounds_validator_pattern]]` — substrate primitive landed Sprint 2
- `[[project_rea_compute_commitment_primitive]]` — gospel-tier shape this proves a first instance of
- `[[project_hub_archetype_abstraction]]` — Hub is role; dwelling/collective/computed separation; stewards-not-members; encryption boundary
- `[[project_signal_kind_extensible_protocol_class]]` — signal_kind extension pattern (reciprocity-imbalance uses this)
- `[[project_canonical_wire_shape_newtype_pattern]]` — DwellingHubId, PeerCid newtype-hardening candidates in follow-up
- `[[project_substrate_scale_ceiling]]` — hub-and-spoke scaling math
- `[[project_three_layer_truth_model]]` — DHT vs libp2p vs doorway scoping

---

## 14. Done when (operator-felt)

A grandma whose grandkid stewards a dwelling-hub in another region can say:

> "My family's photos are safe even if our house burns down. My grandkid's hub keeps a copy. So does the church's hub three states over. The protocol watches it for us — when something drifts off-track, it tells us, and our elohim help figure out what to do. We're not trusting any company. We're trusting each other, and the substrate."

The substrate is honest at every scale, the donut is enforced, the audit-service fires when reciprocity slips, and the operator UI shows the storage-premium multiplier that makes the whole thing economically viable. **That's the first proven instance of the REA compute-commitment primitive.**
