---
id: "backlog-qahal-governance-dimension-gap-ledger"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Qahal governance dimension: gap ledger of typed-but-unreachable surface"
slug: "qahal-governance-dimension-gap-ledger"
written: "2026-06-11"
author: "claude"
status: "backlog"
priority: "medium"
tags: [qahal, governance, routes, gap-ledger, mishpat-boundary, frontend]
derived_from:
  - app/elohim-app/src/app/qahal/QAHAL_API_SPECIFICATION_v1.0.md   # retired to git 2026-06-11
cites:
  - app/elohim-app/src/app/qahal/models/governance-deliberation.model.ts
  - app/elohim-app/src/app/qahal/community.routes.ts
  - app/elohim-app/src/app/qahal/components/context-menu-only/context-menu-only.component.ts
  - app/elohim-app/src/app/qahal/components/feedback-mechanism-gateway/feedback-mechanism-gateway.component.ts
  - app/elohim-app/src/app/qahal/components/graduated-feedback/graduated-feedback.component.ts
  - app/elohim-app/src/app/qahal/components/file-appeal/file-appeal.component.ts
  - app/elohim-app/src/app/qahal/components/respond-to-challenge/respond-to-challenge.component.ts
  - app/elohim-app/src/app/qahal/components/sensemaking-page/sensemaking-page.component.ts
  - app/elohim-app/src/app/qahal/services/bracket-synthesis.service.ts
  - app/elohim-library/projects/elohim-service/src/angular/services/governance-api.service.ts
  - app/elohim-app/src/app/elohim/services/governance.service.ts
  - app/lamad/src/app/lamad.routes.ts
  - mishpat-domain-gospel | the judgment-substrate boundary — every ledger item is qahal surface over mishpat mechanics, never a substrate respec | sha256:d3a335f06f37c884 | path: elohim/sdk/domains/mishpat/CLAUDE.md
  - qahal-domain-gospel | subject home whose mechanism ladder and vocabulary the reachable surface must compose with | sha256:002d11309d8d9620 | path: elohim/sdk/domains/qahal/CLAUDE.md
  - qahal-mvp-roadmap | Sprint-10 power-user panel horizon overlaps the alerting/queue end of this ledger — compose, do not duplicate | sha256:eb80fd03cf0c390d | path: genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md
shift_objective: |
  Close the reachable-surface gaps in the qahal governance dimension, smallest
  first: (1) mount FileAppealComponent and RespondToChallengeComponent from the
  challenge-detail flow — both are fully backed by GovernanceApiService and
  reachable by nothing; (2) decide the fate of the dead model surface
  (GOVERNANCE_ROUTES, GovernanceDeliberationService, GovernanceContextMenu,
  GovernanceHistoryView, FEEDBACK_SCALES): implement against the per-entity
  route pattern already tracked in the epr-routing captures, or delete to the
  retired spec's git history; (3) add a precedents browse route over the existing
  GovernanceApiService precedent reads. Respect the mishpat boundary: UI surface
  only, no substrate respecification.
---

# Qahal governance dimension: gap ledger of typed-but-unreachable surface

## Layer declaration

This is a **shell qahal-pillar view-layer ledger** (`app/elohim-app/src/app/qahal/`,
a consumer of the qahal subject home per the pillar gospel). Every item below is
UI/service reachability — no new data entities, so no p2p-design-gate passage is
required *for the ledger itself*; any item that grows into new entity design
(subscriptions, history snapshots) must pass the gate at pickup. **Mishpat
boundary**: precedents, SLA expectations, and challenge adjudication are qahal
*surface* over the mishpat judgment substrate — "qahal is the app/social governance
surface; mishpat is the judgment substrate it escalates into"
(`elohim/sdk/domains/mishpat/CLAUDE.md`). Items here render or route; they never
respecify substrate mechanics.

## Confirmed ledger (each verified 2026-06-11)

**(a) Entity-scoped governance routes — minting helpers are dead code.**
`GOVERNANCE_ROUTES` (`governance-deliberation.model.ts:819-850`) builds
`/lamad/{entityType}:{entityId}/governance/{view}` URLs, but has **zero importers**
across app/elohim-app, app/lamad, and app/elohim-library (grep verified — so no
dead links render today; the builders themselves are unreachable). No router
registers the pattern: `community.routes.ts` registers only
`/community/governance/{sensemaking, challenges, challenges/new, challenges/:id,
disposition, proxy-votes}`; the lamad bundle (`lamad.routes.ts`) has no governance
path and its `**` wildcard (line 208) would swallow a minted URL. Note: each
builder carries a `route-literal-ok` comment — "claims-minting migration tracked
in epr-routing captures" — so a per-entity route decision already has a tracked
home; compose with it rather than re-deciding here.

**(b) Context menu — contract typed, never served.** `GovernanceContextMenu`
(`governance-deliberation.model.ts:52-79`: status summary, permission-aware
`availableActions`, `alerts`, `fullViewRoute`) has no producer; `getContextMenu`
exists only on the unimplemented `GovernanceDeliberationService` interface (line
861). What's real: `ContextMenuOnlyComponent` — the level-0 kebab menu mounted by
the gateway — is **presentation-only by design** ("No backend wiring in this
component", its own header) emitting flag/challenge events upward. The spec's
"never more than one click away" status-badge-with-alerts contract is unbuilt.

**(c) History view — types only.** `GovernanceHistoryView`/`HistoryTab`
(`governance-deliberation.model.ts:673-701`) have zero usages outside the model;
`getHistoryView` (line 902) lives on `GovernanceDeliberationService`, which has
**no implementing class anywhere** (grep verified). No component, no route.

**(d) FEEDBACK_SCALES — doubly dead.** Of the 15 `FeedbackContext` values, 4 have
populated scales (accuracy, usefulness, proposal-position, label-agreement) and 11
are empty-array stubs (`governance-deliberation.model.ts:404-414`). And the table
has zero consumers: the working `GraduatedFeedbackComponent` defines its own local
5-value `FeedbackContext` (`graduated-feedback.component.ts:629`) and its own
scales record (line 324), recording via `GovernanceApiService.recordSignal`
(line 504). The model's scale vocabulary and the shipped component diverged.

**(e) Sensemaking — implemented, differently shaped (NOT a gap).**
`/community/governance/sensemaking` is routed (query-param entity addressing, not
the spec's per-entity path). `sensemaking-page` calls
`GovernanceApiService.getClusters` (line 287 of the component; service line 461);
`contribute-statement` calls `submitStatement` (line 361) and `voteOnStatement`
(line 334) — both exist verbatim on the service (lines 417, 423).
`BracketSynthesisService` goes beyond the spec: it synthesizes bridging statements
into ranked-choice proposals (Layer A → Layer B). Record only the route-shape
divergence; do not re-spec.

**(f) Appeals/respond — backed but unreachable.** `FileAppealComponent` calls
`GovernanceApiService.fileAppeal` (`file-appeal.component.ts:187`);
`RespondToChallengeComponent` calls `respondToChallenge`
(`respond-to-challenge.component.ts:277`); the service implements the full write
path (`governance-api.service.ts:362-405`). But both components are exported
(`qahal/index.ts:31-32`) and **mounted by no route or parent** —
`challenge-detail.component.ts` does not reference either (grep verified). The
elohim-pillar `GovernanceService` lacking appeal/respond methods is moot: the
write path lives in the library service. The fix is a mount, not a service.

**(g) Subscriptions + global routes.** `subscribeToEntity`/`AlertType` exist only
in the model (`governance-deliberation.model.ts:103, 928`) — no implementation.
Precedent **reads exist** (`governance-api.service.ts:117-133` →
`/api/v1/governance/precedents`; elohim-pillar `governance.service.ts:389-411`)
but **no UI route** renders them anywhere. `SLADashboard`/sla-dashboard/
elohim-oversight: zero hits in any app or library source; nearest living relative
is `getChallengesNearingDeadline` (`governance.service.ts:466`).

## Composition note

The qahal MVP roadmap's Sprint 10 power-user panel suite — including a "feedback
queue inbox" (`genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md:625`) — overlaps
the alerting/queue end of this surface. Any pickup of (b) or (g) should compose
with that panel horizon, not duplicate it.

OPEN QUESTION: items (b), (c), and the 11 stub scales in (d) predate the
server-side MechanismSelection projection the gateway now uses — decide per item
whether to implement or retire to the spec's git history before investing.
