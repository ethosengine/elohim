---
id: "backlog-search-reach-vocabulary-seam"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Content search has its own live site of the reach-vocabulary drift — schema-8 cast into geographic-8 with `as unknown as`"
slug: "search-reach-vocabulary-seam"
written: "2026-09-24"
author: "agent:implementer@claude-sonnet-5 (Lane S)"
status: "backlog"
priority: "medium"
domain: "D1/D2"
area: "app/lamad search service — reach type boundary between the wire view and the search model"
tags: [search, reach, vocabulary-drift, lamad, wire-boundary, unowned]
relatedNodeIds:
  - backlog-reach-vocabulary-frontend-strand
  - content-search-station-4
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - genesis/data/timeline/backlog/arch-frontend-bundle-seams-backlog.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - elohim/sdk/schemas/v1/views/content-search-view.schema.json
  - app/lamad/src/app/services/search.service.ts
  - app/lamad/src/app/models/search.model.ts
  - elohim/sdk/storage-client-ts/src/protocol-core.model.ts
shift_objective: |
  Give the content-search seam its own named resolution inside the reach-vocabulary
  reconciliation the frontend strand already tracks: either project `ContentSearchCandidateView.reach`
  (schema-8 `Reach`) into a real `LocalityLevel` at the search route (a lookup table, evidence-backed,
  not a type-system lie), or change `SearchResult.reach` to carry the schema-8 `Reach` type it
  actually holds and re-type every consumer that reads it as `ContentReach`. Done when
  `readCandidate` in `search.service.ts` no longer has `reach: candidate.reach as unknown as
  ContentReach`, and any ordinal-comparison consumer of `SearchResult.reach` (e.g. a
  `LOCALITY_LEVEL_VALUES` lookup) has a value it can place for every reach the search route can
  actually serve.
---

# Search has its own site of the known reach-vocabulary drift

**Chain:** search candidate leaves the storage peer → the lamad search page renders a result.

**Between (A→C):** `ContentSearchCandidateView.reach` (elohim/sdk/schemas/v1/views/content-search-view.schema.json:305-337,
`$ref` to `elohim/sdk/schemas/v1/enums/reach.schema.json` — the wire `Reach` schema-8 enum:
`private/self/intimate/trusted/familiar/community/public/commons`) → `SearchResult.reach`
(`app/lamad/src/app/models/search.model.ts:97`, typed `ContentReach`, which
`content-node.model.ts:319` and `trust-badge.model.ts:33` both alias to `LocalityLevel` — the
geographic-8 vocabulary in `elohim/sdk/storage-client-ts/src/protocol-core.model.ts:73-80`:
`private/invited/local/neighborhood/municipal/bioregional/regional/commons`).

**Missing node:** one of the two is not the reach the other means. `search.service.ts:242`
bridges them with a raw cast — `reach: candidate.reach as unknown as ContentReach` — inside
`readCandidate`. Only two of the eight schema-8 values (`private`, `commons`) are spelled the
same way in geographic-8; the other six (`self`, `intimate`, `trusted`, `familiar`, `community`,
`public`) are strings `LocalityLevel` never declares. **Probe:** any candidate the recipe serves
at `community`, `public`, `familiar`, `trusted`, `intimate`, or `self` becomes a `SearchResult.reach`
value that an ordinal-comparison consumer keyed on `LocalityLevel` — for example a
`LOCALITY_LEVEL_VALUES[result.reach]` lookup (`protocol-core.model.ts:86-95`) — cannot place; the
lookup returns `undefined` rather than throwing, so the failure is silent (a facet count that
never increments, a sort that treats the row as lowest-ordinal, or `NaN` propagating through a
comparison). `facets.reach` in the same wire view (`content-search-view.schema.json`) carries the
same schema-8 values into `byReach: FacetCount<ContentReach>[]` (`search.model.ts:222`) with the
identical un-cast type mismatch.

**Current state:** the cast is pre-existing in `transformContent` elsewhere in the lamad content
path (this is not a new pattern Lane S invented) and is now repeated in `readCandidate` for the
new content-search route landed in `c8f8fcad8`. It is unowned — no open item names it as the
concrete site of the reconciliation the frontend strand already tracks in general terms. The
`reach-vocabulary-frontend-strand.md` backlog entry (2026-06-11, still `backlog`) catalogs the
schema-8-vs-geographic-8 split across `protocol-core.model.ts`, `trust-badge.model.ts`, and the
doorway/steward `reach.rs` sites, and its slice-4 queue names the doorway residue and a
fixture-harness gap — but it does not yet carry the search route as one of its enumerated sites.
This atom exists so the search seam graduates alongside that strand's slice-4 queue rather than
being rediscovered independently later.
