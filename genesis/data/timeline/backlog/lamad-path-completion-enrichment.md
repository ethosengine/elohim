---
id: "backlog-lamad-path-completion-enrichment"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Path view enrichment friction — client-side O(N*M) completion intersection + unwired nested-path summaries"
slug: "lamad-path-completion-enrichment"
written: "2026-06-11"
author: "lamad island recompose (avodah authorship pass)"
status: "backlog"
priority: "medium"
tags: [lamad, path-service, performance, n-plus-one, projection, mastery]
cites:
  - app/lamad/src/app/services/path.service.ts
  - app/lamad/src/app/interfaces/agent.interface.ts
  - app/lamad/src/app/models/learning-path.model.ts
  - app/lamad/src/app/components/path-navigator/path-navigator.component.html
---

# Path view enrichment friction: completion intersection + nested-path summaries

**Layer declaration:** lamad is a design-domain lens over the shared EPR core
(`genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md`).
This entry assumes: (a) paths are EPR-composite content whose sections/steps
parse client-side (`parsePathView`,
`app/lamad/src/app/models/learning-path.model.ts:571`); (b) storage is a
projection layer, not truth (P1 — reconciliation controller), so "server-side
enrichment" here means doorway/elohim-storage view projection, NOT the 2024
recommendation of a direct `get_path_context` zome call. Completion/mastery
data is agent-scoped (gate class B) reached through the `LAMAD_AGENT` token.

Recomposed from the 2024-era `app/lamad/docs/UI-BLOCKERS.md` island ("Territory
Mastery" UI friction log). Of its three friction points, two are still real
(one got worse); one is obsolete.

## 1. Global completion is still an O(N*M) client-side intersection (REAL)

The "Mastered Elsewhere" feature checks every step's `resourceId` against the
agent's full completed-content set, client-side, on every path load. Verified
2026-06-11 in `app/lamad/src/app/services/path.service.ts`:

- Four pipelines fork-join `this.agentService.getCompletedContentIds(agentId)`
  (lines 370, 482, 544, 923) and intersect per-step via `.has()` (lines 386,
  498, 550, 1062, 1086).
- Chapter metrics recompute the same intersection per chapter
  (`calculateChapterMetrics`, lines ~979–1086).
- The contract is now token-mediated — `ILamadAgent.getCompletedContentIds():
  Observable<Set<string>>` (`app/lamad/src/app/interfaces/agent.interface.ts:43`)
  — which makes swapping in a backend-enriched implementation cheaper than in
  2024, but the computation itself never moved.

For paths with 100+ steps and agents with 1000+ completions this is repeated
fan-out work on every navigation. Fix home: a doorway/elohim-storage path-view
projection that returns per-step completion status (the projection layer
already joins content + agent progress for other views), with the lamad
`PathService` consuming it through the existing token seam. A p2p-design-gate
pass should classify the enriched view (likely operational/C — derived,
reconstructible).

## 2. Chapter metadata in flat paths (OBSOLETE — model evolved past it)

The 2024 friction ("PathChapter metadata calculated client-side for flat
paths; consider API returning a default chapter") was resolved by the
EPR-composite restructure: `parsePathView` now synthesizes chapters from
top-level body sections (`learning-path.model.ts:593-594`), flat paths get
`chapters: undefined` (line 620), and per-chapter `estimatedDuration` rides on
the section data itself (`sectionsToChapters`). The UI's both-structures
handling — the island's own fallback recommendation — is the landed design.
DEAD as written; no entry needed beyond this note.

## 3. Nested-path summaries: from N+1 to never-populated (REAL, regressed)

2024 friction: a step with `stepType: 'path'` forced a separate fetch of the
nested path's metadata to render the "Start Sub-Journey" card; recommendation
was a lightweight `nestedPathSummary` on `PathStep`. Verified 2026-06-11:

- The model grew the fields: `PathStepView.nestedPath?: PathView`
  (`learning-path.model.ts:264`) and `PathOverviewView.nestedPathSummaries?:
  PathIndexEntry[]` (line 296).
- But NO producer populates them: `path.service.ts` never references
  `nestedPath`/`PathIndexEntry`; the only nested-path logic in lamad source is
  graph-shaped (`path-graph.service.ts:77-146`, `nestedPathIds`), which
  explicitly does not expand nested paths.
- The consumer still exists and is now dead UI: the path-navigator template
  guards on `@if (stepView.nestedPath)`
  (`path-navigator.component.html:194-207`) and renders title/description/
  step-count badges + a "Start Sub-Journey" routerLink — none of which can
  ever appear.

So the bundle extraction carried the interface and the template but dropped
the wiring; the friction is no longer "N+1 queries", it's "feature silently
dark". Fix: populate `nestedPath`/`nestedPathSummaries` in `PathService`
step/overview composition (batch the lookups via the path index or fold them
into the same projection view as item 1).

## Readiness

Both live items share one seam (`PathService` composition + `LAMAD_AGENT`
token) and one fix home (a path-view projection). Doing them together is
coherent. Evidence step before implementation: a `pnpm look` render of a path
containing a `stepType: 'path'` step to confirm the sub-journey card is
absent in the running app.

OPEN QUESTION: does any seeded path in current seed data actually carry
`stepType: 'path'` steps? If none do, item 3 is unobservable until seed
content exercises it — the a2o scenario should seed one.
