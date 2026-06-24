---
title: Blob Custody Reconciliation — Phase 3 — Salvage Placement (strategy seam; XOR-distance MVP)
id: blob-custody-phase3-xor-salvage-placement-design
status: Draft
class: protocol-canonical
domain: D5
topic: [blob, custody, placement, placement-strategy, xor-distance, salvage, byte-mobility, reconciliation, resilience, intentional-replication]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-02-blob-custody-reconciliation-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
refines:
  - genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md
cites:
  - genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md
  - blob-custody-reconciliation-design | The canonical architecture seed (manifest/reality/diff trinity); Phase 3 fills the Good-Samaritan salvage door it deferred | sha256:b5a567ba337539a2 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-02-blob-custody-reconciliation-design.md
  - genesis/research/holepunch-p2p-dataplane-cross-pollination-2026-06-24.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-01-light-up-the-topology-design.md
requires_env: [household-nodes]
---

# Blob Custody Reconciliation — Phase 3 — Salvage Placement (strategy seam; XOR-distance MVP)

**Predecessor:** [Phase 2 Redesign — Light Up the Topology](2026-05-02-blob-custody-reconciliation-design.md)
**Canonical seed:** [Blob Custody Reconciliation — Design](../../content/elohim-protocol/architecture/2026-05-02-blob-custody-reconciliation-design.md)
**External prior art:** [Holepunch blind-peering XOR mirror-selection](../../../research/holepunch-p2p-dataplane-cross-pollination-2026-06-24.md) (green-team borrow #3)

## Why this exists (the door Phase 2 left open)

Phase 2 built the manifest / reality / diff trinity and made replicas *grow toward a target after a peer connects* — but only for commitments **already authored upstream** naming `provider == self`. It deliberately deferred autonomous placement, leaving an explicit door:

> *"Door open for Good-Samaritan salvage. When a custody commitment goes unhonored, the placement-gap signal is consumed by this sprint only as a topology UI badge. A future sprint may add an opt-in salvage path: a peer with spare capacity sees the gap and (per consent policy) commits as a new custodian, healing the network without centralized coordination."* — Phase 2 §"Door open", and §"What's NOT in scope": *"Good-Samaritan salvage … Future sprint; the placement-gap signal feeds that future flow but this sprint emits, doesn't consume."*

The live operational wound this targets: **inventory count ≈ 3430 / blob bytes = 0 after 36h** — the dead fan-out. Inventory gossip moves *metadata*; nothing autonomously decides a *new* peer should hold an under-replicated blob, so bytes never spread beyond their seeded custodian.

**Salvage needs a placement *decision function*; this phase lands the seam for it and ships XOR-distance as the MVP strategy.** Without *some* deterministic answer, "any peer with spare capacity sees the gap and commits" is a thundering herd needing coordination. A deterministic strategy makes it coordination-free: of all opt-in always-on peers, **exactly the N closest to the blob self-select** to adopt custody — every peer computes the same closest-set independently, no coordination round-trip.

**XOR-distance is the MVP strategy, chosen for developer convenience — explicitly NOT asserted as the final, purposeful placement policy.** It is deterministic and coordination-free (exactly what gets bytes moving with zero new machinery), but it is *uniform spread* — blind to the things intentional replication actually weighs (household/failure-domain diversity, affinity, capacity/standing, governance intent; see §"Intentional placement"). The **durable artifact is the placement-strategy seam** (§1); XOR is the first plug-in behind it, and the more intentional strategies slot in *additively* without reworking salvage, the commitment author, or the byte path. We seal the seam, not the heuristic.

> Borrowed line: *"Holepunch trusts the writer's key; Elohim trusts the network's validation."* We adopt blind-peering's **mirror-selection heuristic** (transport/placement, Class C) — never its trust model. The placement *intent* a salvager produces is a notarized `custody-blob` commitment (Class A); a placement strategy only **ranks who**, it never **decides truth**.

## P2P Design Gate result (passed — see gate output)

| Artifact | Class | Entry type | Notes |
|---|---|---|---|
| Holder selection (XOR rank) | **C** (derived, ephemeral) | none | pure, recomputed every pass; never persisted |
| Salvage byte copy + inventory row | **C** (operational) | none | already exists (`BlobStore`, `peer_blob_inventory`) |
| Placement intent (salvage `custody-blob` commitment) | **A** (notarized) | **reuse `Commitment`** (`action="custody-blob"`) | authored via the conductor commitment-create path; NOT a local SQL insert |
| Salvage-capacity opt-in advertisement | **C** (operational) | none | gossiped like inventory; populates the candidate pool |

**No new DHT entry type.** **No new HTTP route** required for the decision core. Identity namespace is **`agent_cid`** throughout (commitments + `shard_locations` already use it; salvage standing is about a *steward*, not a transport session). Cross-namespace string-XOR is the documented all-zeros trap — the pool is carried in `agent_cid`; libp2p inventory ids resolve to `agent_cid` via `peer_transport_manifest` before they enter the metric.

## Design

### 1. The placement-strategy seam (the durable artifact) — XOR is the MVP plug-in

The load-bearing abstraction this phase lands is a **pluggable placement strategy**, not XOR. The candidate type carries the context richer strategies will need, even though the MVP ignores most of it — that is the forward-compatibility move that keeps the door open *in code*:

```rust
/// A candidate holder. Carries the context intentional strategies weigh.
/// The MVP XorDistanceStrategy reads only `agent_cid`; a diversity-aware
/// strategy reads `household_id`; a standing/capacity strategy reads
/// `spare_bytes` / standing. New strategies extend behavior WITHOUT
/// changing this type or salvage_pass.
pub struct PlacementCandidate {
    pub agent_cid: String,            // canonical namespace — the only field XOR uses
    pub household_id: Option<String>, // failure-domain diversity (intentional strategies)
    pub archetype: Option<String>,    // always-on class
    pub spare_bytes: Option<u64>,     // capacity-weighting (intentional strategies)
}

/// Deterministically rank candidate holders for a blob. Pure: same inputs →
/// same output on every peer (coordination-free self-selection depends on it).
pub trait PlacementStrategy {
    fn rank(&self, blob_marker: &str, candidates: &[PlacementCandidate], target_n: usize) -> Vec<String>;
}
```

**MVP strategy — `XorDistanceStrategy`.** Map both sides into one 256-bit space, then XOR:

- `key(blob)   = sha256(blob_marker_bytes)`  — the blob marker is the existing `sha256-<hex>` (legacy) or its wrapping CID; the digest is the same 32 bytes either way, so distance is identical (naming-coherence only).
- `key(agent)  = sha256(agent_cid_utf8_bytes)` — `agent_cid` (`uhCAk…`) is not a fixed-width digest, so hash it into the space (standard Kademlia "hash the key into the ID space"). Uniform, deterministic.
- `distance(blob, agent) = key(blob) XOR key(agent)` — big-endian 256-bit compare; ascending; tie-break by `agent_cid` string so the ordering is total.

It reads **only** `agent_cid` from each candidate — by construction blind to household, capacity, affinity, standing. That blindness is the cost of MVP convenience and the entire reason the seam exists (§"Intentional placement").

**The test contract (binding on ANY strategy):** determinism, total order (stable tie-break), `len() == min(target_n, candidates.len())`, no duplicates, empty-in → empty-out, and **agreement** — two peers given the same `(blob, candidate set)` produce the same closest-N. Agreement is what makes self-selection coordination-free; an intentional strategy must either *preserve* determinism-agreement OR move to a coordinated authoring model — the named future fork (§"Intentional placement").

### 2. `salvage_pass` — the new self-selection branch (additive)

A **new sibling** of `reconcile_pass` (NOT a signature change to it — keeps Phase-2 callers untouched). Same trait-injected testability pattern (`LocalBlobStore`, `FetchKicker` → add `CommitmentAuthor`).

For each distinct blob referenced by an active `custody-blob` commitment:

```
target_n  = cfg.salvage_target_replicas            # default = min_replicas_for_eviction (2)
honored   = COUNT(DISTINCT provider) of custody-blob commitments for this blob
            whose provider has a FRESH peer_blob_inventory row for it
            (provider is actually hosting), resolved to agent_cid
if honored >= target_n:            continue          # already resilient
if NOT self_salvage_enabled:       continue          # opt-in consent gate (imago-dei floor)
if self_cid already a provider for this blob: continue
closest = strategy.rank(blob_marker, salvage_pool, target_n)   # MVP strategy = XorDistanceStrategy; pool carries candidates incl. self if opted-in
if self_cid NOT in closest:        continue          # someone closer owns it; coordination-free
# self is opt-in, under target, closest, and not yet a holder → adopt:
author a custody-blob commitment (provider = self_cid,
                                  receiver = <content steward = receiver of an existing commitment for this blob>,
                                  resource_classified_as = [blob_marker])
```

On the **next** reconcile pass the existing Phase-2 provider-role branch sees `provider == self`, the blob missing locally, and fetches it via `blob_fetch::race_fetch` → `serve-blob` event → replica count rises. **Salvage authors intent; Phase-2 moves bytes.** No new fetch path.

`SalvageOutcome { blobs_examined, under_replicated, commitments_authored, skipped_not_closest, skipped_opted_out }` for metrics + tests.

### 3. Consent — the opt-in capacity gate (imago-dei floor)

A peer is **never** silently conscripted (`feedback-identity-sovereignty-ontology-guard`; reach authorization is consent-first). Two gates, both required for self to enter the pool:

1. **Local opt-in:** `salvage_capacity_enabled` config flag (default **false** — opt-in, not opt-out). Honors the hub-optional floor: salvage is an *enhancement* a node offers, never a precondition.
2. **Advertised capacity:** an opt-in node periodically gossips a `SalvageCapacityAd { agent_cid, spare_bytes, archetype, seq, signature }` (Class C operational, like inventory, structural-non-empty sig at Stage 1). Receivers project it into a `salvage_capacity` reality table; the **candidate pool = fresh, opted-in entries** (TTL-aged like inventory). Only always-on archetypes (`node`/`steward`) advertise by default.

The pool is `agent_cid`-keyed by construction → the XOR metric never crosses namespaces.

### 4. Whole-blob first (granularity)

This increment selects N holders **per whole blob**, matching Phase-2's whole-blob model end-to-end. Per-shard salvage (spread RS(4,7) shards across distinct holders, composed with the existing household/archetype diversity selector in `peer_selection.rs`) is a **named follow-on**, out of scope here. `PlacementStrategy::rank` is shard-agnostic (it ranks holders for a key) so the graduation needs no rewrite — only a per-shard caller, and (per §"Intentional placement") a diversity-aware strategy plugged into the same seam.

## Intentional placement (the door left open — NOT sealed by the MVP)

XOR-distance is *uniform spread* across the keyspace; **purposeful replication weighs more.** The strategy seam keeps each of these additive — this phase ships none of them, and must foreclose none of them. They are the reason the MVP is a *strategy behind a seam*, not a hardwired function.

- **Failure-domain / household diversity (highest priority).** Resilience is household-to-household, not peer-to-peer (`project_household_is_resilience_unit`). **XOR-closest can silently co-locate multiple replicas in one household — a single household loss then takes them all.** This is the MVP's sharpest blind spot. An intentional strategy spreads across distinct households / failure domains, composing with the existing `peer_selection.rs` household/archetype diversity multi-pass. (Until it ships, operators should be aware that whole-blob salvage targets ≥2 *peers*, not provably ≥2 *households*.)
- **Affinity / relationship-following placement.** "Replication follows relationship" — prefer holders who actually care about the content (affinity-weighted), not uniform-random.
- **Capacity- / standing-weighted.** Prefer holders with real spare capacity and good custodial standing (REA standing) over the geometrically nearest. `PlacementCandidate.spare_bytes` is already carried for this.
- **Governance- / reach-directed.** A qahal or content steward may direct placement (reach-scoped content → reach-authorized holders).
- **Geographic / latency-aware** placement for read-locality.

**The one named future fork (recorded, not decided):** several of these break the *determinism* XOR relies on for coordination-free self-selection. The seam therefore also leaves open the **authoring model**: keep self-selection (intentional strategies must stay deterministic-agreeing across peers) **vs** move to coordinated/governed authoring (a planner, the content steward, or a quorum authors placement intentionally). This phase commits to neither — it ships the deterministic-self-selection MVP and records the fork so the more intentional design can pick it up without unwinding a baked-in assumption.

## Threat model & guards

- **Conscription:** opt-in flag default false + advertised consent; no peer is drafted. (imago-dei floor.)
- **Thundering herd:** deterministic closest-N means only the intended holders adopt; no coordination, no storm. A peer that drops out → next pass another peer becomes closest and adopts (self-healing).
- **Salvage flapping:** authoring is idempotent (skip if self already a provider); honored-count uses fresh inventory so a just-authored commitment whose bytes haven't landed still counts self once it holds them. Re-author cooldown mirrors `placement_gap_cooldown_seconds`.
- **Lying capacity ads:** a peer can over-advertise spare capacity; the cost is a wasted adoption that ages out — same falsifiability as Phase-2's lying inventory (the lie collapses; no durable harm). Stage-2 signs ads.
- **Cross-namespace empty join:** pool is `agent_cid`; libp2p inventory resolves through `peer_transport_manifest`. Never raw-string-compare across namespaces (the all-zeros incident).

## What's NOT in scope

- **Per-shard XOR spread** (RS shards across distinct households) — follow-on; needs composition with `peer_selection.rs` diversity.
- **Signed capacity ads / signed gossip** — Stage-2, per the security gradient.
- **Distributed-introducer signaling** and **per-block verified streaming** — the other two Holepunch TOP-3 borrows (survey #1, #2); separate specs.
- **Tier-awareness** — `BlobHint.tier` is wired-but-always-`None`; salvage ships tier-blind; the tier axis (how-warm) is orthogonal to placement (where) and layers later.
- **Removing the survey's parity attribution confusion** — the `parity_shard_count:0` is in the operator-gated *seed* path, not `shard_manifest_backfill`; the wound is placement, not parity. Do not "fix parity."

## Migration / tasks (decomposed → gap-items)

| # | Task | Files | State |
|---|---|---|---|
| P3-1 | `PlacementStrategy` trait + `PlacementCandidate` (the seam) + `XorDistanceStrategy` MVP impl + unit tests (the strategy test contract) | `src/reconcile/placement.rs` (new) | OPEN |
| P3-2 | `salvage_pass` + `CommitmentAuthor` trait + `SalvageOutcome` + unit tests (under-replication, closest, opt-out, already-provider, idempotent) — strategy injected | `src/reconcile/custody.rs` | OPEN |
| P3-3 | `salvage_capacity` reality table (migration) + `SalvageCapacityAd` wire + projection writer | `src/db/`, `src/p2p/inventory_gossip.rs` (sibling) | OPEN |
| P3-4 | Capacity-ad broadcast scheduler (opt-in nodes, archetype-tunable cadence) | `src/p2p/inventory_broadcaster.rs` | OPEN |
| P3-5 | Production `CommitmentAuthor` (conductor commitment-create, notarized) | `src/services/conductor_writes.rs` | OPEN |
| P3-6 | Wire `salvage_pass` into the reconcile task (pool from `salvage_capacity`, resolve libp2p→agent_cid via `peer_transport_manifest`); config knobs (`salvage_capacity_enabled`, `salvage_target_replicas`, `salvage_recheck_seconds`) | `src/p2p/mod.rs`, `src/config.rs` | OPEN |
| P3-7 | a2o scenario: under-replicated blob + opt-in closest peer → adopts → replica rises (Jenkins multi-peer) | `genesis/a2o/features/` | OPEN |
| P3-8 | **(future / door-open)** Intentional placement strategies behind the seam — household/failure-domain diversity FIRST, then affinity/standing/governance; resolve the deterministic-self-selection vs coordinated-authoring fork | `src/reconcile/placement.rs`, `src/services/peer_selection.rs` | DEFERRED |

P3-1 and P3-2 are the **verified decision core** (fully unit-testable today, no network). P3-3–P3-6 light it up. P3-7 proves it cross-peer. **P3-8 is the door this spec deliberately leaves open** — the MVP must not foreclose it (the seam in P3-1 is exactly what keeps it cheap).

## Related

- Phase 2 — the trinity + the deferred door this fills.
- `project_inventory_exchange_not_byte_replication` — the failure mode (inventory without bytes) salvage cures.
- `project_placement_signals_are_shefa_inputs` — placement-gap as economic signal; salvage is the recovery actor that consumes it.
- `project_rea_compute_commitment_primitive` — `custody-blob` is one instantiation of the bounded-reciprocity commitment primitive.
- `project_hub_optional_floor` — salvage is an enhancement, never a participation gate (opt-in default false).
- `project_holepunch_cross_pollination` — borrow #3 (XOR mirror-selection); adopt transport, never attestation.
- `project_household_is_resilience_unit` — why diversity-aware placement (P3-8) outranks XOR uniformity; resilience is household-to-household.
- Backlog: `intentional-placement-strategy-beyond-xor.md` — the P3-8 follow-on (diversity/affinity/standing strategies + the authoring-model fork).
