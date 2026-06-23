---
title: "Operational Facing — the weave/tiers/PVC lens (dual-projected: typed view + Prometheus gauges)"
id: operational-weave-facing-lens-design
status: Draft
class: protocol-canonical
domain: D5
topic: [operational, weave, tiers, pvc, facings, lens, placement, custodian, metrics, dataplane]
refines:
  - genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
informed-by:
  - genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
cites:
  - resilience-facings-select-fold-aggregate-design | the select→fold→aggregate lens framework (§11) this facing is a child of — its materialized-relation + pure-fold + typed-view substrate | sha256:93279fd25a0600d1 | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - lens-complete-epr-resolution-four-leg-coupling-design | the four-leg coupling law + §2 projection law this lens facing ultimately descends from | sha256:79f821217c1c8e11 | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
requires_env: [household-nodes, observability]
---

# Operational Facing — weave / tiers / PVC

> **One-line:** the operator debug Lens is not a new subsystem — it is the **operational fold** of the
> select→fold→aggregate pipeline, selecting the holder-relation plus two operational siblings
> (shard-placement, custodian-capacity), aggregating along a *different* ladder (per-shard → per-node →
> per-cluster weave), and projecting **twice**: a typed JSON view *and* Prometheus gauges from the same
> pure folds.

## Provenance
Surfaced 2026-06-19 from the operator's dimensional-handling framing ("developer-operational:
weave/tiers/PVC/storage"); refines `2026-06-19-resilience-facings-select-fold-aggregate-design.md` (the
pipeline) and `2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md` §6 (Operational-C verdict).

## Materialized relations it SELECTS
Reuses the framework's `HolderRow` for hub/region grouping, plus two per-facing sibling rows (recipe step 1;
not a god-struct), because `HolderRow` carries no shard geometry or bytes and operational aggregates per-shard/per-node:
- **`ShardPlacementRow`** from `shard_locations.rs` — `(shard_hash, agent_id, status, last_verified)`. ⚠ the
  `peer_id` column *holds* `agent_cid` (misnamed; same trap as `household_resilience.rs:74`); join VALUE stays
  `agent_cid`. RS geometry from `shard_manifests` (`RS(N,K)`).
- **`CustodianCapacityRow`** from `custodian_metrics` + `system_metrics` — `(agent_id, free, used, stewarded)`;
  reuses the existing `ComputeTriptych` view.

Placement gaps reuse `placement_gaps`/`PlacementGapView` as-is. All three are Category-C, rebuildable from
shard-protocol acks / gossip inventory.

## The FOLDS (pure fns; no `conn`)
- `rs_coverage(placements, manifest) -> f32` — achieved shards / `rs_data_shards`; reuses
  `PlacementGapView.contract_coverage`.
- `placement_gap_count(gaps) -> usize` and `gaps_by_kind` — `distinct_count_by` / `bucket_by`.
- `tier_occupancy(holders) -> RiskTierDistribution` and `region_occupancy -> RegionalDistributionView` —
  `bucket_by(hub)` then bucket-by-tier/region (the deduped re-use of the `intra_hub_peers` combinator).
- `node_capacity(custodians) -> ComputeTriptych` and cluster `aggregate(triptychs)` — `free/used/stewarded` rollup.

## Typed VIEW + HTTP surface (all GET / extend; no new POST)
New cluster-scoped **`WeaveView`** (`#[derive(TS)]`, camelCase: `placementGapCount`, `rsCoverageHistogram`,
`tierOccupancy`, `regionOccupancy`, `clusterCapacity: ComputeTriptych`, `measuredAt`) on a new
`GET /api/v1/weave` (+ `is_service_path` arm — the doorway-shadow trap). Wire the **already-built-but-dead**
`peer_capacity` handler (`api/peer_capacity.rs`, currently unrouted). **Second projection:** register storage
gauges in the existing `/metrics` (today only memory/smaps/corpus): a storage-side adapter calls the *same
pure folds*, then `GAUGE.set()`s `elohim_placement_gap_count`, `elohim_rs_coverage`, `elohim_tier_occupancy`,
custodian free/used/stewarded. **Never `set()` inside a fold** — the fold returns numbers, the adapter emits
both wire shapes.

## Aggregation levels
**per-shard → per-node → per-cluster (the weave)** — a *different* ladder than resiliency's
per-content→per-household, proving the pipeline generalizes across ladders. Verdict-rollup (`aggregate`) is
hand-written per level (genericity stops at the relation layer).

## P2P Design Gate output
**Operational-C, zero new DHT types, zero new identity** — inherits lens §6 / qahal-lattice §7. Every relation
reconstructs from `shard_locations` + `custodian_metrics`/`system_metrics` + gossip inventory; no
`dht_anchor_hash`, no table. `agent_cid` is the sole join key. No exception.

## Slices (sequence)
**Blocked-until (lens, not fold):** the lit lens is gated on real loader/observability work that does not
exist today — **zero storage Prometheus gauges are registered** (`/metrics` carries only memory/smaps/corpus),
the placement/capacity loaders aren't written, and the `peer_capacity` handler is dead (unrouted). The DB-free
proof fold below lands any time; the lens lights only after those loaders + gauge registration land. Charter,
not a near-done lens. (Also `requires_env: observability` — the gauge half needs the metrics pipeline.)

- **Slice 1 (proof-gate, DB-free).** Light ONE metric end-to-end: `placement_gap_count`. Hand-build
  `Vec<PlacementGapView>` → pure fold → assert count → assert the `/metrics` gauge value (the
  `build_felt_status` pattern; no DB, so NULL-`agent_pub_key` and no-shard-seed gaps cannot block it).
  `upsert_gap` is the cleanest real write path for later.
- **Slice 2.** `rs_coverage` fold + `WeaveView` field + ts-rs codegen + the gauge.
- **Slice 3.** `node_capacity` / cluster `ComputeTriptych` rollup; wire `peer_capacity`.
- **Slice 4.** `tier_occupancy` + `region_occupancy`; full `WeaveView` + `GET /api/v1/weave`.

## Non-goals / operator-owned
- v1 folds only from **existing tables**. **Pantry/storage-temperature occupancy** has no source table → non-goal.
  **PVC/disk-pressure** stays CI/hook-side (`pool-policy.json`) — not a fold.
- Does **not** fabricate `shard_locations` rows, build the boot self-session, or deploy/reseed —
  operator/security-owned. Lighting metrics on **live alpha** (needs real seed paths) is distinct from the
  in-repo green proof and is deferred/operator-owned.
- Does **not** adopt Cozo, a generic untyped envelope, or a `Lens` trait.
