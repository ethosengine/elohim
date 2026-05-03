# Distribution + Resilience Surfaces — Coherence Design

**Date:** 2026-05-03
**Branch:** `feature/light-up-topology`
**Status:** Brainstormed — pending user review
**Supersedes:** an earlier draft of this file that proposed merging the two
into one widget. That draft mistook redundancy for two-dimensional truth;
this version treats distribution and resilience as orthogonal readings.

**Mission frame.** This is foundational substrate work. The Elohim Protocol
seeks to subsume Google, Facebook, and Amazon by reaching the Apple-mantra
bar of **"it just works"** peer-to-peer for *everyone*. That requires
foundational trust signals grandma can read ambiently — distribution and
resilience are two of those signals. Bar: **credible to a grandmother whose
family photos are on the line**, not "good enough for an internal demo".
Power has to extend safely from below (humans, households, collectives) to
the hubs; if the foundational surfaces fail, every downstream stewardship
flow inherits the failure.

## Two dimensions, not one

The protocol surfaces two distinct readings on every piece of content. They
are projections of the same DHT truth (REA commitments + economic events +
custodian state), but they answer different questions:

| Dimension | Question it answers | Today's widget | Today's data shape |
|---|---|---|---|
| **Distribution** | _Where does this content live right now? How widely is it spread? Who's projecting it? Where did this fetch come from?_ | `<app-distribution-badge>` (this sprint) | `DistributionSummary` (inline on EPR head) + lazy `DistributionDetails` |
| **Resilience** | _How safe is this? How much commitment backs it? What's missing from target? Will it survive losing peers?_ | `<elohim-resilience-snapshot>` (existing) | `ResilienceSnapshotView` (separate fetch) |

You can have:

- **High distribution + low resilience**: content reached many peers, but no
  REA commitments back it — could vanish on the next vacuum.
- **Low distribution + high resilience**: content sits in a few stewards who
  are deeply committed (private family archive on three home-servers with
  custodian commitments).
- **High both**: ideal state for grandma + podcaster.
- **Low both**: at-risk private content with single replica and no commitment.

These are not the same number expressed differently. They are two axes. The
surface needs both.

## How the two widgets serve the user-stories

| Moment | Distribution badge surfaces | Resilience snapshot surfaces | Pairing |
|---|---|---|---|
| **Grandma — family photos at rest** | "5 family devices · intimate reach" | "Safe — household-backed · 0 gaps" | **Both, side-by-side**: one tells where, the other tells safe. |
| **Podcaster — global reach** | "3 households (AU/TX/UK) · 3 doorway projectors · public reach" | "Stewarded by podcast-collective · 0 placement gaps" | **Distribution-led** with resilience as confirmation. |
| **Church steward — collective-stewarded site** | "4 households hosting · 2 projectors" | "4 stewards committed · protected · won't go down" | **Resilience-led**; distribution confirms it's actually live. |
| **Matthew — post-tornado recovery** | "Was on 5 devices (2 home-server, 2 laptop, 1 phone) · now 3 reachable" | "Was protected · placement gap opened · recoverable from peers" | **Both critical**: resilience tells what's recoverable; distribution tells where to recover from and what hardware to bring back. |
| _Future A — release a commitment_ | (no change) | `myStewardshipCommitment` + handoff trigger | **Resilience** (commitment-backed dimension) |
| _Future B — endangered → help_ | replicaCount drops below floor | `endangered.recommendedAction` + recruit trigger | **Resilience** (gap-closing) |
| _Future C — offline carry → REA credit_ | "You're carrying this · contributingToReplicaCount" | (when elevated) "Your REA commitment is now backing this" | **Distribution-side first** (you're a node), **crosses into resilience** when you elevate to a commitment with feedback exposure. |

The widgets are independent. A surface chooses to render one, the other, or
both, depending on which questions the user is asking at that moment.

## The actionable scope of this branch

This branch (light-up-the-topology) introduced:

1. `<app-distribution-badge>` — new widget, distribution dimension
2. `DistributionSummary` schema — new data shape, hydrated inline on EPR head
3. `DistributionService.getDetails(blobHash)` — lazy fetch of
   `DistributionDetails` (the deep tier)
4. T50 embed of `<app-distribution-badge>` into `concept-card`

The existing protocol already had:

5. `<elohim-resilience-snapshot>` — established widget, resilience dimension
6. `ResilienceSnapshotView` schema — collective-grain
7. `ResilienceService.getSnapshot(contentId)` — separate fetch
8. Existing embed of `<elohim-resilience-snapshot>` in content-viewer header

**Coherence work for this branch:**

A) **Rename for vocabulary consistency**: `<app-distribution-badge>` →
   `<elohim-distribution-badge>` and graduate the component into
   `elohim-library` alongside `<elohim-resilience-snapshot>`. Both are
   protocol-vocabulary widgets that should be reusable across the app, the
   doorway-app, and any future shell. They live where reusable
   protocol-grade widgets live.

B) **Place the distribution badge next to the resilience snapshot in
   content-viewer header**, side-by-side. Same icon density, both visible at
   a glance. Each carries its own data; neither replaces the other.

C) **Concept-card** keeps its existing distribution badge embed (T50). Cheap
   inline, no separate fetch — appropriate for list density. The resilience
   snapshot can be added later if list-density resilience reads are needed.

D) **No conflation of data shapes**: `DistributionSummary` and
   `ResilienceSnapshotView` stay distinct schemas. `DistributionService` keeps
   its name (it serves the distribution dimension). `ResilienceService` keeps
   its name. They share underlying DHT primitives but not types.

## Component placement after the rename

```
elohim-library/projects/elohim-service/src/
  resilience/
    resilience-snapshot/                          (existing)
      resilience-snapshot.component.ts            <elohim-resilience-snapshot>
      ...
    resilience.service.ts                         (existing — getSnapshot)
  distribution/                                    (NEW — graduated from app)
    distribution-badge/
      distribution-badge.component.ts             <elohim-distribution-badge>
      distribution-badge.component.html
      distribution-badge.component.scss
      distribution-badge.component.spec.ts
    distribution.service.ts                        (NEW — getDetails lazy fetch)
    distribution.service.spec.ts
```

`app/elohim-app/src/app/elohim/components/distribution-badge/` is **moved**
into the library; `app/elohim-app/src/app/elohim/services/distribution.service.ts`
is **moved** with it. The app re-exports nothing — consumers import from
`@elohim/service/public-api` like they do for `<elohim-resilience-snapshot>`.

## Component contracts (unchanged in shape, just relocated)

```ts
// EXISTING — no change
@Component({ selector: 'elohim-resilience-snapshot' })
class ResilienceSnapshotComponent {
  @Input({ required: true }) snapshot!: ResilienceSnapshotView;
  @Input() density: ResilienceSnapshotDensity = 'icon';
}

// MOVED into library; selector renamed for consistency
@Component({ selector: 'elohim-distribution-badge' })
class DistributionBadgeComponent {
  @Input({ required: true }) summary!: DistributionSummary;
  @Input() blobHash?: string;       // enables lazy DistributionDetails fetch
}
```

No prop-merging, no input-overloading, no rendering-rules choreography
between the two. Each is single-purpose.

## Where the two widgets render together

| Surface | Distribution badge | Resilience snapshot |
|---|---|---|
| `content-viewer` header (next to reach pill) | yes — `[summary]="node.distribution" [blobHash]="node.blobCid"` | yes — `[snapshot]="resilienceSnapshot$ \| async"` |
| `concept-card` (related-concepts list, etc.) | yes — `[summary]="concept.distribution"` (inline, cheap) | not yet — list density doesn't need a separate fetch |
| Topology pages (`/shefa/cluster`, `/shefa/peers`, `/shefa/reciprocity`) | neither — those use device-tile / peer-household-card / commitment-bar / diversity-hint atoms |

The content-viewer header carries both because the resource page is the
moment a user is asking _both_ questions: where is this and is it safe.

## Future-moment data hooks (schema reservations)

Reserve optional fields in the appropriate dimension's schema. They're
`optional` and `additionalProperties: true` so absence is the default;
presence is the future-sprint payload.

**In `distribution-details.schema.json`** (distribution-side hooks):
- `offlineCarry?: { isCarrying, contributingToReplicaCount }` — moment C
  prelude (you are a node, before elevating to REA commitment)

**In `resilience-snapshot-view.schema.json`** (resilience-side hooks; in the
existing `details` sub-object):
- `myStewardshipCommitment?: { committedBytes, sinceDate, contractId }` —
  moment A (release flow)
- `endangered?: { reason, atRiskCount, recommendedAction }` — moment B
  (recruit-help)
- `recoveryHints?: { devicesBefore, devicesNow, hardwareGap, obligations }`
  — disaster recovery

Moment C **crosses dimensions**: distribution surfaces "you're carrying
this"; when the user elevates to a REA commitment with feedback exposure,
the resilience snapshot starts to include `myStewardshipCommitment` for that
content. The cross-dimension transition is the design intent — distribution
tells you you're a node, resilience tells you you're a steward.

## Substrate convergence (BACKLOG — separate spec)

The two pipelines (`compose_resilience_snapshot` and
`compose_distribution_summary`) share underlying DHT primitives. They could
share helper functions / cached aggregations. They should NOT collapse into
one projection — the schemas serve different dimensions and different
delivery models (inline-cheap vs separate-rich).

The backlog spec is about **shared internals at the substrate**, not about
unifying the surfaces. Logging:

> _Backlog: `2026-05-?-distribution-resilience-substrate-sharing-design.md` —
> identify shared SQL aggregations / index hits / per-CID cached primitives
> across the two compose paths. Each pipeline keeps its public contract; the
> internals can converge to reduce duplicated work._

## Migration plan (this branch)

| Today | After |
|---|---|
| `app/elohim-app/src/app/elohim/components/distribution-badge/` | **moved** to `app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/` |
| `app/elohim-app/src/app/elohim/services/distribution.service.ts` | **moved** to `app/elohim-library/projects/elohim-service/src/distribution/distribution.service.ts` |
| `<app-distribution-badge>` selector | **renamed** `<elohim-distribution-badge>` |
| `concept-card` template `<app-distribution-badge>` | swap to `<elohim-distribution-badge>` (import from `@elohim/service/public-api`) |
| `content-viewer` header — only `<elohim-resilience-snapshot>` today | **add** `<elohim-distribution-badge>` next to it; both render in icon density |
| `content-viewer` test for `viewer-resilience-info` testid | unchanged on the resilience snapshot; new testid `viewer-distribution-info` for the new badge |
| `<elohim-resilience-snapshot>` component | **unchanged** — no input changes, no rendering changes |
| `ResilienceService` (in elohim-library) | **unchanged** |
| `DistributionService` | **moved** to library, name preserved |
| `app/elohim-library/projects/elohim-service/public-api.ts` | **export** the new badge + service |

## Test / a2o updates

1. **Concept-card T50 spec**: rename selector assertion from
   `<app-distribution-badge>` → `<elohim-distribution-badge>`.
2. **Content-viewer**: add a test asserting **both** widgets render in the
   header when both data sources are hydrated.
3. **`observable-distribution.feature`**: the two scenarios harvested in
   this branch reference `app-distribution-badge` — re-tag to
   `elohim-distribution-badge`.
4. **DistributionBadgeComponent spec**: moves into the library testing rig
   (Vitest equivalent in elohim-library); existing tests carry over.
5. **No new ResilienceSnapshotComponent test**: it's unchanged.
6. **Story-harvest**: scenarios already authored. Update selector names
   only.

## Architectural implications respected

- **No new entities, no new DHT entry types**.
- **No new substrate routes**.
- **Schema-first IoC**: distribution and resilience schemas stay separate;
  future-moment hooks are added as optional fields.
- **No data-shape merging**: each widget owns its dimension.
- **Vocabulary alignment**: both selectors use `elohim-` prefix (matches
  protocol-grade library widgets).
- **Single-responsibility**: each component is small, testable in isolation,
  and consumers compose them.
- **Three-layer truth model preserved** (per memory): both pipelines compose
  from DHT truth; library widgets render projections; doorway is web2
  projection but neither widget runs there.

## Out of scope (explicit)

- **Substrate-side sharing of compose internals** (separate backlog spec).
- **UI for moments A/B/C** (each gets its own sprint).
- **Trust-tab grid in content-viewer** (lines 656-727) — fed by the older
  rich `ResilienceView` (encoding strategy, shards, parity, steward
  allocations, storage commitments). Stays as-is in this branch; the trust
  tab's relationship to the unified two-widget header is part of the
  substrate-sharing backlog.
- **Topology pages** — unaffected; different atoms.
- **doorway-app**: doesn't render content cards or content-viewer; no
  consumer change there.

## Success criteria

- [ ] `<elohim-distribution-badge>` lives in elohim-library and is exported
      from `@elohim/service/public-api`.
- [ ] `<elohim-resilience-snapshot>` is unchanged.
- [ ] `concept-card` embeds `<elohim-distribution-badge>` (T50 retained,
      selector updated).
- [ ] `content-viewer` header renders BOTH `<elohim-distribution-badge>` and
      `<elohim-resilience-snapshot>` side-by-side.
- [ ] `app/elohim-app/src/app/elohim/components/distribution-badge/` and
      `app/elohim-app/src/app/elohim/services/distribution.service.ts` are
      removed (moved into library).
- [ ] All component tests pass; story-harvest scenarios re-tagged to
      `elohim-distribution-badge`.
- [ ] Backlog spec stub for substrate-side compose sharing is opened.

## Why this matters

The Elohim Protocol's mission is to subsume Google, Facebook, and Amazon —
to displace surveillance-and-extraction architectures with a peer-to-peer
substrate that reaches the Apple-mantra bar of **"it just works"** for
*everyone*, including grandma, the Canadian podcaster, and the church
family that just wants its website to stay up. That requires foundational
trust at the substrate so power can extend safely **from below** (humans,
households, collectives) to the hubs — never the other way around.

These two widgets are load-bearing primitives for that mission. Distribution
and resilience are two questions every human must be able to ask of every
piece of content they touch. Conflating them into one widget hides the fact
that they can answer differently — high reach without commitment is
fragile, narrow stewardship without reach is unread. The standard for
getting this right is not "good enough for an internal demo" — it is
**credible to a grandmother whose family photos are on the line**.

Keeping them as two single-purpose widgets, both in the protocol-vocabulary
library, side-by-side where it matters, is what gives grandma the right
confidence ambiently and gives Matthew two distinct readings during
recovery: where it was, and what was actually protected. Future moments
(release, recruit-help, offline-carry → REA-credit) build on this same
foundation. We are not laying out badges; we are laying the foundational
trust signals that the rest of the protocol's promises rest on.
