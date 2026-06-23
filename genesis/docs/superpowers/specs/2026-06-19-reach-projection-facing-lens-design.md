---
title: "Reach / Projection Facing — the collective-distributive lens over the edge + reach relations"
id: reach-projection-facing-lens-design
status: Draft
class: protocol-canonical
domain: D5
topic: [reach, projection, facings, lens, peer-topology, reciprocation, distribution, dataplane]
refines:
  - genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
informed-by:
  - genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
cites:
  - resilience-facings-select-fold-aggregate-design | the select→fold→aggregate lens framework (§11) this facing is a child of — its materialized-relation + pure-fold + typed-view substrate | sha256:93279fd25a0600d1 | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - lens-complete-epr-resolution-four-leg-coupling-design | the four-leg coupling law + §2 projection law this lens facing ultimately descends from | sha256:79f821217c1c8e11 | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
requires_env: [household-nodes]
---

# Reach / Projection Facing — collective-distributive lens (doorway-facing)

> **One-line:** the reach/projection lens folds the *collective-distributive* facing —
> reciprocating-collectives, reach-class replica distribution, and peer-topology edges with their
> resilience cliffs — over operational projections, as the doorway-facing complement to the proven
> resiliency facing. It is one child of the lens framework (`2026-06-19-resilience-facings…` §11):
> loaders are storage-side, folds are pure functions in `elohim-facings`.

## Provenance
Refines the canonical facings architecture (`2026-06-19-resilience-facings-select-fold-aggregate-design.md`)
and the four-leg lens spec (`2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md`); the
resiliency facing (`household_resilience::snapshot`/`compute`) is the reference implementation this conforms to.

## Materialized relations (SELECT)
Two new materialized relations in `elohim-facings/src/relation.rs`, both Category-C operational projections:
- **`EdgeRow`** — one row per (peer ↔ household) edge: `agent_id` (the per-edge agent; join VALUE is
  `agent_cid`, never a transport id), `hub_id: Option<String>` (the grouping key), `online: bool`
  (liveness measured-at-T), `their_cids_hosted_by_me: i64`, `my_cids_hosted_by_them: i64`. Loader
  `load_edge_relation` (storage-side) joins `peer_identity_bindings` (libp2p peer_id ↔ agent_cid) →
  `humans` → `peer_blob_inventory`, as `peer_topology_view.rs` does today.
- **`ReachRow`** — one row per content CID: `content_cid`, `reach` (the DNA-notarized 8-value band),
  `replica_count: u32`, `region: Option<String>`. Loader `load_reach_relation` joins `content.reach`
  to `peer_blob_inventory`/`economic_events` for replication accounting.

Per-facing tuples only — no `Relations` god-struct.

## Folds (pure fns over the relation)
Each `&[Row] → View`, DB-free, unit-testable with hand-built `Vec<Row>`, composing the shared
`bucket_by`/`distinct_count_by` combinators:
- **`fold_reciprocation(edges) -> u32`** — count of `online` edges; `households_reciprocating` =
  `distinct_count_by(edges.filter(online), hub_id)` (replaces `network_posture.rs`'s separate query).
- **`fold_net_diff(edge) -> i64`** — `their_cids_hosted_by_me − my_cids_hosted_by_them` (give/take balance).
- **`fold_reach_distribution(reach_rows) -> Vec<ReachBandHealth>`** — `bucket_by` reach band, then
  `replica_health_for(count, target)` per bucket (lifted from `distribution_view.rs`).
- **`fold_resilience_cliffs(edges) -> Vec<ResilienceCliff>`** — `bucket_by(hub_id)` over sole-replica
  CIDs (`replica_count == 1`), counting per household.
- **`fold_edge_criticality(edges, cliffs) -> …`** — joins cliff verdicts onto edges to populate
  `isCriticalForMe`/`iAmCriticalForThem`, closing the today-`None` gap at `peer_topology_view.rs`.

## Typed VIEW + HTTP surface
Views stay in `elohim-views` (`#[derive(TS)]`, camelCase): `PeerTopologyView`, `PeerHouseholdEdge`,
`NetworkPostureView`, `DistributionDetails`/`DistributionSummary`. **Extend existing routes, no new POST:**
`GET /api/v1/peer-topology`, `GET /api/v1/network/posture`, `GET /blob/{hash}/distribution`. Each new
route needs an `is_service_path` arm (the doorway-shadow trap, `project_doorway_main_route_needs_is_service_path`).

## Aggregation levels
per-content → per-household via `bucket_by(hub_id)`; per-household → per-dashboard is a hand-written
`aggregate()` (genericity stops at the relation layer): `NetworkPostureView` is the edge-relation
dashboard rollup, `DistributionSummary` the reach-relation rollup.

## P2P Design Gate
**Operational (Category C), zero new DHT entry types.** Every relation projects already-notarized data:
`AgentPeerBinding` (A), `content.reach` (A, DNA-notarized), `peer_blob_inventory` (C). Identity is
`agent_cid` (`uhCAk…`), never slug/transport-id. **Flagged exception (data, not fold):** `AgentPeerBinding`
is self-asserted/unsigned (`STAGE1_SIGNATURE_SENTINEL`) — reach/reciprocation edges are NOT yet
economically attributable; the lens classifies, it does not attest.

## Slices (sequence)
**Blocked-until (lens, not fold):** the lens lights only once `load_edge_relation` + `load_reach_relation`
exist (the loaders don't today) and the reach-distribution fold has a reconciled reach vocabulary
(3 drifted enums, backlog item 13). The DB-free *proof fold* below can land any time; the *lit lens* is
gated on that loader/view/vocab work — it is a charter, not a near-done lens.

1. **Proof slice (build_felt_status pattern):** light `reciprocationCount` end-to-end — write
   `fold_reciprocation` + its unit test over hand-built `Vec<EdgeRow>` first (online/offline/null-hub),
   then `load_edge_relation`, then the `GET /api/v1/peer-topology` adapter; gate cutover on byte-identical JSON.
2. `fold_net_diff` + `fold_edge_criticality` (closes the `None` criticality gap).
3. `fold_reach_distribution` over `ReachRow` (lights `DistributionSummary` reach bands) — needs the reconciled vocab.
4. `fold_resilience_cliffs` made pure; `households_reciprocating` migrated off its separate query.

## Non-goals / operator-owned
- **Reach-vocabulary reconciliation** (backlog item 13): three drifted enums (`epr/src/reach.rs`,
  `epr_kind.rs`, `ReachClass`) — loader/schema work, not fold work.
- **HTTP-side reach enforcement** (`http-reach-enforcement-gap.md`): operator/doorway-owned; this lens classifies only.
- **Heuristic CID accounting** (`local_blob_count_heuristic`, hard-zero `storage_pressure`): loader fidelity, fixed in `load_*_relation`.
- **The `Lens` trait, the `dyn` registry, a facing×leg matrix** — excluded per the framework's no-over-reify constraint.
