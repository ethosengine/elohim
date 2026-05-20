# Light-up-the-topology — Sprint Result

**Branch:** `sprint/light-up-topology-2026-05-20`
**Plan:** [genesis/docs/plans/2026-05-20-light-up-the-topology.md](../../plans/2026-05-20-light-up-the-topology.md)
**Period:** 2026-05-20 (single-day intensive)
**Approach:** Subagent-driven execution per [superpowers:subagent-driven-development](../../../.claude/skills/) — fresh implementer per task, two-stage review (spec + code-quality), continuous flow.

## What landed

Three independent visual surfaces that prove the P2P substrate is alive and stewarding content resiliently.

### Sub-project A — Compute Triptych (`/shefa/cluster`)

Per-device "free / used / stewarded" surface in `DeviceTileComponent`. Substrate input layer for the operator's parallel-sprint hub-aggregate UX.

| # | Commits | What |
|---|---------|------|
| A1 | `b7b53ff2e`, `6f81af8ad` | `aggregate_stewarded_bytes_by_peer(pool, peers) -> HashMap<peer_id, u64>` — SUMs custody-blob REA commitments per provider peer. f64 SUM (multi-TB precision), empty short-circuit. |
| A2 | `127f2fa72`, `6cb0b3e60` | `ComputeTriptych { free, used, stewarded: Option<u64> }` view + JSON schema; attached as `Option<ComputeTriptych>` on `DeviceSummary`. Hub-abstract conventions (`epr:schema:view:*` `$id`, compact nullable). |
| A3 | `ba84be8d1` | Wired into `aggregate_my_cluster_view` — both live and offline arms. Stewarded value resolves from REA ledger even when peer is offline. |
| A4 | `06307523e` | `ComputeTriptychGql` GraphQL field — all three fields stringified for JS Number precision safety (mirrors existing `DeviceSummaryGql` byte fields). |
| A5 | `bdbfc67c2` | `VIEWER_HUB_QUERY` extended; `ComputeTriptychGql` TS interface declared inline next to query. |
| A6 | `9e4bcb042`, `b4cf99ed8` | `DeviceTileComponent` renders triptych with `data-testid` attrs; conditional `@if (device.compute)` guard; adapter passthrough (`adaptDevice` was swallowing the field). |
| A7 | `e9782bbb5` | `@compute-triptych` a2o scenario + step defs in `topology.steps.ts`. Tagged `@wip` consistent with sibling M1 scenarios. |
| A8 | `c16801391` | Sub-project verify; one parity regression fixed (adapter must omit `compute` key when null, not emit `compute: null`). |

**Substrate flow proved end-to-end:** REA commitments (notarized) → `aggregate_stewarded_bytes_by_peer` → `DeviceSummary.compute` → `DeviceSummaryGql.compute` (stringified) → `VIEWER_HUB_QUERY` → `topology-graphql.adapter.adaptDevice` (passthrough) → `DeviceTileComponent` template.

### Sub-project B — Doorway-peers wiring (app shell)

Made the existing `DoorwayDashboardComponent` (already routed, but unreachable without typing the URL) discoverable from the main app navigation.

| # | Commits | What |
|---|---------|------|
| B1 | `6885c90a4` | `DoorwayLayoutComponent` — standalone OnPush, sidenav (Dashboard / Configuration) + router-outlet, mirrors `shefa-layout`. |
| B2 | `923c52f70` | Doorway routes nested as children under the new layout. Three routes preserved (`''`, `'elohim'`, `'config'`) with their `data` metadata. |
| B3 | `ce7369733` | `ElohimNavigator` already had a `doorwayApp` entry gated by `hasDoorwayCapableNode()`; added `isDevMode()` bypass so doorway is discoverable in dev without a registered node. Production gate preserved. |
| B4 | `282a1b368` | `doorway-dashboard-health` step defs implemented + supporting UI scope-expansion (capabilities badge + per-tab empty-state testids in `doorway-app`). One scenario un-`@wip`. |
| B5 | (verification) | Gates clean. Doorway-app vitest coverage thin (11 tests); capabilities-badge logic relies on a2o scenarios for verification. |

**Note:** Research at sprint kickoff was wrong that routes didn't exist — they did. Real gap was layout chrome + nav discoverability. Sub-project B scope shifted accordingly and stayed minimal.

### Sub-project C — Resilience tooltip + placement-gaps (`DistributionBadgeComponent`)

Polymorphic hub abstraction (`HubKind { Dwelling, Collective, Computed }`) sitting on the loadbearing notarized DHT foundation. Substrate stays kind-agnostic; UI resolves labels.

| # | Commits | What |
|---|---------|------|
| C1 | `8f6189206` | `PlacementGapRow { kind: PlacementGapKind::{HubDiversity, ReplicaCount, ReachClass}, contentId, shortfall: { target, observed }, remediation }` view + schema. Replaced `Vec<JsonVal>` loose shape on `DistributionDetails`. |
| C2 | `11903ab16` | `/api/v1/resilience/{id}/hub` returning `ResilienceHubView { contentId, hubs: HubSummary[] }`. Real hub classification chain: `peer_blob_inventory.peer_id → peer_identity_bindings.agent_cid → humans.agent_pub_key → household_id (Dwelling) / collective_participations (Collective) / Computed`. |
| C3 | `2647661e2` | Hover tooltip + placement-gaps row on `DistributionBadgeComponent` in `elohim-library`. Hub-kind labels mapped from `diversityHint`. |
| C4 | `18b16be46` | `load_placement_gaps_for` no longer stubbed — emits real `HubDiversity` gap rows by calling `hub_summary` and comparing observed-vs-target hub count. Target=2 constant with `TODO(reach-aware-targeting)` for proportional scaling by reach class. |
| C5 | `4cfa7324e` | 3 of 6 stubbed resilience step defs fully implemented (resilience-icon class, tooltip household count, signals-card gap count). 3 left as documented TODOs against surfaces not yet built (signals-gap-click → `/shefa/recruit`, doorway admin content-list per-row snapshots). |
| C6 | `f0306783b` | One scenario un-`@wip`ed (`Content-viewer resilience tooltip is live`). Eight scenarios honestly kept `@wip` — their steps reference surfaces not yet built. |
| C7 | (verification) | Gates clean. |

### Sprint cleanup

| Commit | What |
|---|------|
| `37a5539fe` | `chore(sprint)`: codegen drift (ComputeTriptych.ts doc-comment regen), captured `[[project_hub_compute_aggregate_primary]]` memory, cleared two pre-existing clippy items (`needless_return` at api/mod.rs, `items_after_test_module` `#[allow]` at views.rs) that would have blocked pre-push. |

## Substrate framing (carried throughout)

The plan opened with a **P2P design-gate declaration**: all new artifacts are **Category-C operational projections** sitting on the loadbearing notarized DHT foundation. Zero new DHT entry types, zero new diesel tables, one new HTTP route (`/api/v1/resilience/{id}/hub`) that is also Category-C projection.

**Hub is a role, not a notarized entity** — dial-up-by-capability ([[project_hub_archetype_abstraction]], [[project_hub_optional_floor]]). Polymorphic `HubKind { Dwelling, Collective, Computed }` resolves at the projection layer; substrate stays kind-agnostic so future hub kinds slot in without schema change.

## Mid-sprint reframing — hub aggregate is primary

During A1 dispatch the operator surfaced a critical UX insight ([[project_hub_compute_aggregate_primary]]): the human-visible surface should be the **hub aggregate** (sum of member-device capacities, progressive disclosure by driver capability — grandma / default / power-user), NOT the per-device triptych. Sliding a blade into the rack at home jumps the hub from "5GB / 15GB" to "5GB / 100GB" — the human's experience of "my hub" is steady; capacity changes underneath.

**Sprint adaptation:** A1–A5 substrate landed unchanged (the hub aggregate derives from the per-device truth). A6–A8 UX-side work was treated as **drill-down view**, not primary surface. The primary `/shefa/cluster` hub-aggregate component is the deliverable of the operator's parallel sprint on capability profiles + element contracts (interleaved on this branch — commits `eed9b6cb3` through `4fadb3821`, separate sprint result document).

## Adaptations from the plan

The plan was research-grounded but reality differed in places:

- **Schema home for `ComputeTriptych`** — plan said `elohim-storage::views`; reality is `elohim-views::infrastructure` (dependency direction). Implementer adapted on first dispatch.
- **`DeviceSummary` schema is inline**, not a separate `device-summary.schema.json` — plan assumed a standalone file.
- **Doorway routes already existed** — research said "no route consumes it"; reality is the route exists, the gap is layout chrome + nav discoverability.
- **Hub classification chain works against real binding tables** — C2 didn't have to stub everything to `Computed`; the `humans.household_id` + `collective_participations` joins make real Dwelling/Collective resolution possible today.
- **Synchronous diesel** — A1's plan assumed `diesel_async`; codebase reality is sync diesel. Implementer noted and adapted.
- **Six DeviceSummary constructor sites** — exhaustive constructors required `compute: None` insertions across `graph_views/shefa/cluster.rs`, `services/cluster_view.rs` (×2), `tests/schema_contract.rs`.

## Follow-ups

1. **Hub-aggregate UX surface** (operator's parallel sprint) — primary `/shefa/cluster` view rendering the aggregate, with progressive disclosure by capability tier. Per-device triptych this sprint built becomes the drill-down view.
2. **Reach-aware gap targeting** — `TARGET_HUBS = 2` constant in `load_placement_gaps_for` should scale by declared reach class (commons / regional / household / none). TODO breadcrumb in place.
3. **Surfaces-not-yet-built** (8 scenarios stay `@wip`):
   - Shefa signals-card gap-click → `/shefa/recruit` route
   - Doorway admin content list per-row resilience-snapshot mount + `[data-testid="content-row"]`
   - Cluster page offline-device freshness display
   - Peer-topology resilience-cliff warning
   - Concept card distribution-badge integration (5 undefined steps)
   - Distribution-badge defers details-fetch until tooltip opens (lazy-load lifecycle)
4. **`formatBytes` consolidation** — code review flagged 5 private implementations across the codebase (different signatures). A shared util/pipe would dedupe.
5. **Hub-rich tooltip labels** — `DistributionBadgeComponent` tooltip currently derives hub-count label from `diversityHint`; when `ResilienceHubView` fetch lands on the badge it'll produce more accurate "N households / collectives / hubs" labels per actual kind mix.
6. **GraphQL/HTTP precision asymmetry** — HTTP wire ships `ComputeTriptych` as JSON integer; GraphQL ships `ComputeTriptychGql` as String (JS-safe). Documented on the struct doc comment. Worth deciding if this asymmetry stays or harmonizes.
7. **doorway-app vitest coverage** — capabilities-badge logic landed without unit tests; a2o scenario is the only coverage. Backfill if doorway-app grows.

## Verification

All pre-push gates green (sprint scope):
- `cargo test --lib`: 1122 passed, 0 failed
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo test --test schema_contract`: 111+ passed (depending on test snapshot; +4 from C1 + 4 from C2)
- `cargo test --test compute_triptych`: 6 passed
- `cargo test --test cluster_view`: 6 passed
- `cargo test --test resilience_hub`: 5 passed
- `cargo test --test distribution_view`: 18 passed
- elohim-views, storage-client-ts, elohim-library, elohim-app: all build clean
- elohim-app vitest: 7472+ passed
- a2o cucumber dry-run @resilience-p1: all step bindings resolve
- a2o cucumber dry-run @doorway-dashboard: all 26 step bindings resolve

**Live-stack verification deferred** — Eclipse Che workspace can't boot the Holochain + storage + doorway stack. Operator runs the visual end-to-end check locally or via Jenkins.

## Stats

- **Sprint duration:** single day, intensive
- **Tasks completed:** 21/21
- **Commits (topology scope):** 28 across A/B/C + cleanup
- **Subagent dispatches:** 38 (implementers + spec reviewers + code reviewers + verifiers + fixes)
- **Pre-existing issues cleaned up:** 2 (clippy)
- **Memory entries captured:** 1 ([[project_hub_compute_aggregate_primary]])

## Memory deltas

- **New:** `[[project_hub_compute_aggregate_primary]]` — hub-as-pool aggregating member-device capacities; "5GB/15GB → 5GB/100GB when blade joins"; progressive disclosure by driver capability; per-device triptych is drill-down substrate.

## Branch state

`sprint/light-up-topology-2026-05-20` is 28 commits ahead of `dev` (this sprint) plus the operator's parallel-sprint commits on `elohim-core` + `elohim-elements` (capability profile + element contract substrate). Both sprints are intended to merge together.
