# Resilience-Snapshot Unification Design

**Date:** 2026-05-03
**Branch:** `feature/light-up-topology`
**Status:** Brainstormed — pending user review

## Vision context

`<elohim-resilience-snapshot>` is the surface where every piece of content the
user touches communicates its resilience, distribution, stewardship, and
recovery posture. It is not a passive status indicator — it is the operator
console for **stewardship participation** at the scale of one human
interacting with one piece of content.

This widget is **load-bearing for the protocol's core promises** — the same
surface that lets grandma never worry about losing the family photos lets
Matthew log in via doorway after a tornado and recover his stewarded life. It
is the foundation; getting it right enables the recovery epic and a
sequence of stewardship-flow surfaces that build on it.

## User-stories the widget serves

| Moment | Reads from data | What it tells the human |
|---|---|---|
| **Grandma — family photos at rest** | `summary.replicaCount`, `snapshot.stewardingCollectives`, `summary.reachClass=intimate` | "Safe — 5 family devices, just us" |
| **Podcaster — global reach** | `summary.replicaCount` (geo-spread), `summary.projectorCount`, `summary.diversityHint=region_metro`, `summary.reachClass=public` | "3 households (AU/TX/UK) hosting · 3 doorways projecting" |
| **Church steward — collective-stewarded site** | `snapshot.stewardingCollectives`, `snapshot.commitmentBackedCollectives`, `snapshot.protectionStatus` | "4 stewards committed · won't go down" |
| **Matthew — post-tornado recovery** | `snapshot.placementGaps`, `summary.replicaCount` (was→now), `details.replicaPeers` (devices), future `myStewardshipCommitment` | "Recoverable from 3 peers · bring back 1 home-server · obligations to honor: [list]" |
| _Future A — release a commitment_ | `myStewardshipCommitment` + handoff trigger | "Release stewardship of X to ___" |
| _Future B — endangered → help_ | `endangered.recommendedAction` + recruit trigger | "At-risk — recruit help from your collective" |
| _Future C — offline carry → REA credit_ | `offlineCarry.contributingToReplicaCount` + REA commit trigger | "You're carrying this. Pin → contribute → take feedback exposure for the value." |

The four normal-operation moments + disaster recovery are **in-scope for this
unification**. The three future moments (A/B/C) define the **schema-reserved
data hooks** so subsequent sprints can fill them without breaking the widget
contract.

## Why two systems exist today (and shouldn't)

The protocol has two parallel substrate pipelines computing the same
underlying truth at different grains:

| Pipeline | Grain | Delivery | Today's UI |
|---|---|---|---|
| `compose_resilience_snapshot` (older) | **Collective-aggregated** — stewardingCollectives, regionalDistribution, placementGaps, protectionStatus | Separate HTTP fetch per content (`getSnapshot(contentId)`) | `<elohim-resilience-snapshot>` (in elohim-library) |
| `compose_distribution_summary` (light-up-topology sprint) | **Replica/peer-grained** — replicaCount, projectorCount, peer archetypes, reachClass, diversityHint, thisFetchSource, lastVerifiedSeconds | **Inline** on every EPR head response (no extra round-trip) | `<app-distribution-badge>` (in elohim-app, this sprint) |

Both compose from the same DHT primitives (REA commitments + economic events).
The new pipeline is **finer-grained and cheaper**; the old pipeline is
**collective-grain and richer in placement-gap detail**. They are not two
features — they are two grains of one feature, currently surfaced as two
widgets that the user is supposed to think of as one thing.

The unification: **one widget, two data inputs, one user-story name.**

## Component contract

```ts
@Component({
  selector: 'elohim-resilience-snapshot',          // existing protocol vocabulary
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
class ResilienceSnapshotComponent {
  // Either or both — the component renders whichever is hydrated.
  @Input() snapshot?: ResilienceSnapshotView;     // collective-grain (separate fetch)
  @Input() summary?: DistributionSummary;          // replica-grain (inline on EPR head)
  @Input() blobHash?: string;                      // enables lazy DistributionDetails fetch
  @Input() density: 'icon' | 'context' | 'full' = 'icon';
}
```

### Rendering rules

- **`icon` density**: prefer `summary` (cheap, inline). If only `snapshot` is
  bound, render the existing dot-status icon. Tooltip surfaces both grains
  when both are available.
- **`context` density**: render a panel with two sections, each rendered only
  if its data source is hydrated:
  - **Replica detail** (from `summary` + lazy `details`): replicas, target,
    health, reach, projectors, diversity hint, this-fetch source, my-role.
  - **Collective rollup** (from `snapshot`): stewarding collectives,
    commitment-backed collectives, regional distribution, placement gaps.
- **`full` density**: same two sections, expanded; deep details (per-replica
  peer rows, per-projector identity rows, per-collective list) shown.
- **Lazy detail tier**: when `[blobHash]` is bound, on tooltip expand the
  component calls `ResilienceDetailService.getDetails(blobHash)` and shows
  per-replica/projector rows from `DistributionDetails`.

### Inputs are independently optional

A consumer can bind any subset:
- `[summary]` only — the cheap inline path (concept lists, content viewer
  header during normal navigation).
- `[snapshot]` only — legacy callers that already fetch the snapshot.
- `[summary] [snapshot]` — full-fidelity (e.g., trust tab in content-viewer).
- `[summary] [blobHash]` — cheap inline + on-demand deep tier.

## Migration plan (this branch)

| Today | After |
|---|---|
| `<app-distribution-badge>` element (in app/elohim) | **deleted** — `<elohim-resilience-snapshot>` everywhere |
| `DistributionBadgeComponent` + .html + .scss + .spec | **deleted** |
| `app/elohim-app/src/app/elohim/components/distribution-badge/` directory | **deleted** |
| `DistributionService` (lazy details fetch only) | **renamed** `ResilienceDetailService` — same one route, same lazy fetch |
| `ResilienceSnapshotComponent` (in elohim-library) | **enhanced** — accepts `[summary]` and `[blobHash]`, calls service for lazy details, renders new fields |
| `concept-card` T50 embed `<app-distribution-badge>` | swap to `<elohim-resilience-snapshot>` |
| `content-viewer` header `<elohim-resilience-snapshot>` | unchanged tag; gains new fields automatically when substrate hydrates `[summary]` |
| `viewer-resilience-info` testid | unchanged |

The `<app-distribution-badge>` widget was added in this branch (T37). It has
no external consumers. Deleting it cleanly is correct — no deprecation cycle
needed.

## Substrate convergence (BACKLOG — separate spec)

`compose_resilience_snapshot` should derive from `compose_distribution_summary`
+ collective aggregation. They share underlying primitives but currently run
as parallel pipelines. Logged as a follow-on substrate spec —
**not blocking** the finish-right of this branch:

> _Backlog: `2026-05-?-substrate-projection-convergence-design.md` — collapse
> the two compose paths into one canonical projection at the substrate;
> `ResilienceSnapshotView` becomes a derived rollup over
> `DistributionDetails.replicaPeers` grouped by collective membership._

## Future-moment data hooks (schemas reserved, no UI now)

To enable future sprints (A/B/C in the user-stories table) to land cleanly,
reserve optional fields in the schemas now. Each is `optional` so absence is
the normal-state default; presence is the future-sprint payload.

In `distribution-details.schema.json`:
```json
{
  "myStewardshipCommitment": {
    "type": "object",
    "description": "Future moment A: present when the viewer is the steward of this content. Schema kept open during reservation.",
    "additionalProperties": true
  },
  "endangered": {
    "type": "object",
    "description": "Future moment B: present when replicaCount is below floor and a recruit-help signal is recommended. Schema kept open.",
    "additionalProperties": true
  },
  "offlineCarry": {
    "type": "object",
    "description": "Future moment C: present when the viewer is carrying this content offline; signals an open opportunity to elevate to REA-committed stewardship with reciprocal feedback exposure.",
    "additionalProperties": true
  },
  "recoveryHints": {
    "type": "object",
    "description": "Disaster-recovery preview: device set before/after, hardware gap, stewardship obligations to reconstitute. Surfaced when the viewer is in recovery context.",
    "additionalProperties": true
  }
}
```

These do NOT add Rust struct fields yet (the schemas allow extension);
substrate work for each lands with the corresponding future sprint.

## Architectural implications respected

- **No new entities, no new DHT entry types** — both pipelines already exist;
  this is consolidation.
- **No new routes** — the existing `/api/v1/blob/{hash}/distribution/details`
  route powers the lazy detail tier; the existing per-content snapshot route
  remains for legacy callers.
- **Substrate is unchanged in this sprint** — only the UI consolidation and
  one TS service rename.
- **Schema-first IoC** — schemas reserve future fields; component contract
  declares which inputs power which moments.
- **Vocabulary alignment** — `resilience-snapshot` is the established
  user-facing name (per memory: "let's think about what we named it before");
  it wins over the brand-new sprint name `distribution-badge`.

## Test / a2o updates

1. **Concept-card T50 spec**: assert `<elohim-resilience-snapshot>` (not
   `<app-distribution-badge>`) when `concept.distribution` is hydrated.
2. **Content-viewer**: existing `<elohim-resilience-snapshot>` header keeps
   its testid; gains rendering of new fields when `summary` is bound. Add
   spec covering both data sources hydrating together.
3. **`observable-distribution.feature`**: existing scenarios already use
   `elohim-resilience-snapshot` — correct as-is. Two scenarios harvested in
   this branch reference `app-distribution-badge` — re-tag to the unified
   element.
4. **ResilienceSnapshotComponent spec (elohim-library)**: extend to cover
   the new inputs and the lazy-details-fetch flow.
5. **Story-harvest**: add a recovery-preview scenario that asserts the
   widget surfaces replica + collective + (future) recovery-hint fields,
   marked `@wip` for the recovery-epic sprint that fills `recoveryHints`.

## Out of scope (explicit)

- Substrate convergence of the two compose pipelines (separate backlog
  spec).
- UI for moments A/B/C (each gets its own sprint that builds on the
  reserved schema fields).
- The trust-tab grid in content-viewer (lines 656-727) — fed by the older
  rich `ResilienceView` shape (encoding strategy, shards, parity, steward
  allocations, storage commitments). Stays as-is in this branch; convergence
  with the unified widget is part of the substrate-convergence backlog.
- Any change to topology pages (cluster / peers / reciprocity) — those use
  different atoms (device-tile / peer-household-card / commitment-bar /
  diversity-hint), not the snapshot widget.

## Success criteria

- [ ] One widget element (`<elohim-resilience-snapshot>`) renders on every
      content surface that previously had either `<app-distribution-badge>`
      or `<elohim-resilience-snapshot>`.
- [ ] `<app-distribution-badge>` element + component + directory removed
      from the codebase.
- [ ] The widget renders meaningfully with `[summary]` only, `[snapshot]`
      only, or both bound; lazy `[blobHash]` triggers details fetch.
- [ ] Concept-card and content-viewer both show the widget when their
      respective data is hydrated.
- [ ] All component-level tests pass.
- [ ] Story-harvest scenarios re-target the unified element name.
- [ ] Backlog spec for substrate convergence opened (placeholder file with
      the convergence sketch).

## Why this matters

This is the foundation of the protocol's promise to a human. Grandma's
"safe — 5 family devices" and Matthew's "recoverable from 3 peers, bring
back 1 home-server" are the same widget reading the same DHT truth at
different moments of need. The protocol is not credible until that surface
is one thing.
