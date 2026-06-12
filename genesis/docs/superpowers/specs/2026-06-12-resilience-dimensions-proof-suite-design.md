---
id: resilience-dimensions-proof-suite
status: Approved
---

# Resilience Dimensions Proof Suite — design

> **Status:** Approved (operator, 2026-06-12 ~04:00Z, overnight EPR-durability session).
> A two-layer test suite that proves every dimension of the designed P2P resilience
> surface — protection status, peer counts, commitment-backing, diversity,
> local/regional/global projection, the progressive icon, high availability —
> against the thresholds the substrate actually implements. Drawn from the
> graphos resiliency matrix sketches (`app/elohim-library/projects/graphos/src/
> imported/reference/resilience/`) and the felt-resilience design
> (`2026-05-29-durability-topology-felt-resilience.md`).

## Why

The dimensional vocabulary exists in three places that nothing pins together:
the felt-resilience spec's progressive icon (`●/◐/○` at ≥3 / 1–2 / 0 stewards),
`household_resilience.rs`'s compound thresholds (protected ← ≥3 households AND
≥2 online peers), and the rendered snapshot component's status classes. The
2026-06-12 session proved the cost of unpinned seams (12 dead cross-DNA bridge
sites; a tooltip of zeros nobody could interpret). This suite makes each
dimension's boundary an executable fact.

## Shape (approach A, approved)

- **Layer 1 — deterministic boundary matrix.** Extends
  `elohim/elohim-storage/tests/household_resilience.rs` (existing `test_pool()`
  harness + seed helpers). Pure SQL-fixture → `compute()`/`snapshot()` tests;
  green in CI immediately; no cluster dependency.
- **Layer 2 — E2E dimensional matrix.** One new feature,
  `genesis/a2o/features/resilience/resilience-dimensions.feature`, the single
  legible home for the matrix. Asserts the live API
  (`/api/v1/resilience/{id}/household`) and the rendered surfaces (icon classes,
  glyphs, tooltip text). HA is carried by cross-reference to the existing flow
  features (`peer-loss-failover`, `app-blob-heal-on-read`,
  `substrate-reconciliation`), never duplicated.

## The seven dimensions

### D1 — Protection-status ladder
Truth: `household_resilience.rs` Stage 4 —
`protected ← n≥3 && o≥2`; `partial ← n≥2 || o≥1`; else `at-risk`; missing
manifest → degenerate at-risk view (zeros, empty details).

Layer-1 edges (one test per row; h = distinct stewarding households with
junction rows, p = online peers within those households):

| h | p | expected |
|---|---|---|
| 0 | 0 | at-risk (and degenerate path when manifest absent) |
| 1 | 0 | at-risk |
| 2 | 0 | partial |
| 1 | 1 | partial |
| 3 | 1 | partial (NOT protected — peers short) |
| 2 | 2 | partial (NOT protected — households short) |
| 3 | 2 | protected |

Layer-2: three seeded contents land one in each status; assert API
`protectionStatus` + snapshot icon class `status-{at-risk,partial,protected}`.

### D2 — Peer counts
Truth: `count_online_peers_in_households` — counts `peer_statuses` rows with
status `online` OR `degraded`, only within stewarding households.
Layer-1 edges: offline rows don't count; degraded counts; peers in
non-stewarding households don't count; empty household set → 0.
Layer-2: tooltip "N peers online" + header `ConnectionIndicator` peerCount.

### D3 — Commitment-backing
Truth: snapshot() counts DISTINCT households over `rea_commitments` with
`action=provide`, `state=active`, `resource_classified_as = content:<reach>`,
joined through `humans.household_id`.
Layer-1 edges: `proposed` does NOT count; wrong scope does NOT count; two
provide rows in one household count once; provider without household_id does
not count.
Layer-2: `commitmentBackedCollectives ≥ 1` (`@wip` until Epic-B provide rows
exist outside test_util).

### D4 — Diversity score
Truth: `min(stewarding, max(commitment_backed, 1)) / 7`, clamped 0..1.
Layer-1 edges: 0 stewarding → 0.0; the `max(...,1)` floor means
commitment-backing zero does NOT zero the score (stewarding=3, cb=0 →
min(3, max(0,1)) = 1 → 1/7); stewarding=7, cb=7 → 1.0; stewarding>7 clamps
at 1.0.
Layer-2: percent renders in the context panel.

### D5 — Projection: local / regional / global
Truth: `compute_regional_distribution` — per stewarding household (deduped),
bucket by (viewer_region, steward_region): (None,None)→unknown,
(None,Some)→global, (Some,None)→unknown, equal→local, differ→regional.
Viewer region resolves `viewer_household_id → collectives.region`. Missing
manifest → all-zero distribution; snapshot() error-fallback sets
`unknown = households_stewarding`.
Layer-1 edges: all five match arms + the dedupe (two peers, one household,
one bucket increment) + both fallbacks.
Layer-2: "Geographic distribution" line vs seeded `collectives.region`
(`@wip` — no region rows seeded on alpha today).

### D6 — Progressive icon (two vocabularies pinned together)
The felt-resilience icon (`●/◐/○` at ≥3 / 1–2 / 0 stewards,
`EprRelationshipCardComponent` ← `ResilienceService.getContentResilience`) and
the snapshot status classes are DIFFERENT threshold vocabularies. Layer-2 pins
both on the same seeded contents so divergence is visible: a 2-household
content shows `◐` AND `status-partial`; a 3-household+2-peer content shows `●`
AND `status-protected`. (Angular unit specs already cover class mapping;
the E2E rows prove the live join.)

### D8 — Storage aggregation triptych: free / used / committed
(Added same-session by operator request — "an agent lens on the primitives.")
Truth lives in two seams:
- **`cluster_view::compose_totals`** — the device triptych:
  `used`/`total` = SUM over device summaries (devices reporting `None` are
  skipped, not zeroed); `external_committed` = SUM of
  `rea_commitments.resource_quantity_value` over `action='custody-blob'`
  rows whose provider is one of MY bound peer ids, clamped at 0; empty
  bindings short-circuit to 0. Free is derived downstream as total − used.
- **`peer_capacity_service::compute_peer_capacity`** — pledged-vs-held:
  per-tier pledge sums, `free_bytes_remaining = total_raw − unique_shard`
  (may go negative — honest over-hold), and the saturate-never-wrap pct
  guard (already well-tested: 7 in-module tests incl. the 2026-06-04
  wrap-finding regression).
Layer-1 edges (new, in `cluster_view.rs` in-module tests): device sums skip
None; committed=0 without bindings; only custody-blob counts (provide
excluded); only my peers count; multi-peer sums; negative SUM clamps to 0
(never-wrap doctrine).
Layer-2: cluster page triptych renders used/total/committed for the
logged-in steward (`@wip` — identity-bound; committed-accounting readers
are the felt-resilience runway item 2).

### D7 — High availability (cross-reference)
Carried by: `federation/peer-loss-failover.feature` (reads keep serving,
returning peer re-syncs), `resilience/app-blob-heal-on-read.feature`
(race-fetch heal + serve-blob REA event), `resilience/substrate-reconciliation.feature`
(scale-down names who's unreachable, never fatal),
`federation/peer-recovery.feature` (wipe drill). The matrix feature carries a
comment-block index pointing at them — no duplicated scenarios.

## Tagging & env honesty

- Feature-level: `@e2e @resilience`.
- `@browser-only` on icon/tooltip rows; API rows run headless.
- Household floor by default (M/J/J mesh); `@requires:shem` only for breadth.
- `@wip` rows map 1:1 to the named data gaps proven 2026-06-12: humans
  junction unpopulated (D1/D2 E2E), provide rows test_util-only (D3),
  no region rows (D5). **Un-wipping these rows IS workstream D's acceptance
  gate** — the suite is the executable form of that backlog.

## Verification

- Layer 1: `cargo test --test household_resilience` (storage CI; container
  needs /tmp target-dir + ambient WASM RUSTFLAGS quirk-handling per memory).
- Layer 2: gherkin parse-validation before commit (a parse error aborts the
  whole E2E run); step shapes reuse `observable-distribution`'s existing
  `/api/v1/resilience` + browser-step patterns.
- No new DHT entry types, routes, or models — pure test surface over existing
  substrate (p2p-design-gate not triggered).

## Out of scope

- Filling the data gaps themselves (workstream D / Epic B / formation).
- The committed-accounting + wisdom-input gaps (felt-resilience runway items
  2–3).
- New UI surfaces (posture views have no consuming component yet — D-future).
