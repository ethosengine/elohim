---
id: "backlog-lamad-search-facet-ui"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Lamad search UI: render the facets and highlights the service already computes"
slug: "lamad-search-facet-ui"
written: "2026-06-11"
author: "claude"
status: "backlog"
priority: "low"
tags: [lamad, search, frontend, ui, facets, discovery]
derived_from:
  - app/lamad/docs/IMPLEMENTATION_PLAN.md   # retired to git 2026-06-11
cites:
  - app/lamad/src/app/services/search.service.ts
  - app/lamad/src/app/models/search.model.ts
  - app/lamad/src/app/components/search/search.component.ts
  - elohim/sdk/domains/lamad/CLAUDE.md
---

# Lamad search UI: render facets and highlights the service already computes

## Layer declaration

This is a **lamad-bundle view-layer task** (`app/lamad/`, the reference client
consumer per `app/lamad/CLAUDE.md`). It assumes the lamad domain vocabulary
(`elohim/sdk/domains/lamad/` — content types, reach values) and consumes data the
bundle's own SearchService computes client-side. No substrate, DNA, or domain-manifest
change is involved — this does not pass through the p2p-design-gate because it creates
no new data entity; it renders an existing in-memory computation.

## What

The lamad search service computes a full faceted-search response that the search
component never renders. Close the gap:

1. **Facet sidebar** — render `SearchResults.facets`
   (`app/lamad/src/app/models/search.model.ts:178`; `SearchFacets`/`FacetCount`
   imported at `search.service.ts:19-20`) as clickable filter counts by content
   type, reach, trust, and tags, feeding back into `SearchQuery` filters.
2. **Highlight rendering** — render `SearchResult.highlights`
   (`search.model.ts:115`) and `matchedFields` (`search.model.ts:112`) as
   emphasized match snippets in result rows.

## Why

Phase 12 of the MVP plan ("Enhanced Search",
`app/lamad/docs/IMPLEMENTATION_PLAN.md`) landed in two halves: the service half
shipped — `search()` computes facets from all matching results before pagination
(`search.service.ts:104-105`) and returns them (`search.service.ts:129`), with the
docblock explicitly promising "Facet counts for filter UI" (`search.service.ts:51`)
— but the UI half never did. The plan's own Remaining Work tracked this as
Priority 5: "Search facet sidebar and highlight rendering," and it is the only
Remaining Work line item still verifiably open in the current tree. Faceted
narrowing (by type/reach/trust/tag) is the difference between search-as-lookup and
search-as-discovery, which matters in a content graph of thousands of nodes where
plain text relevance alone buries path-level and reach-scoped results.

## Readiness

High — this is pure view work over an existing, tested computation:

- `SearchService.search(query): Observable<SearchResults>` already returns
  `facets` and per-result `highlights`; `search.service.spec.ts` exists alongside.
- The consuming component is small and self-contained:
  `app/lamad/src/app/components/search/search.component.ts` is 211 lines with an
  inline template that renders results and the first 3 tags per result
  (`search.component.ts:43-44`) — and contains **zero** references to
  `facet`/`Facet`/`highlight` (verified by grep, 2026-06-11).
- Eyes-first rails apply: render before/after with `pnpm look` (a2o), and follow
  the bundle styling rails in `app/lamad/CLAUDE.md` (consume `--lamad-*` tokens,
  never define them in the bundle).

## Evidence of current state (verified 2026-06-11)

- `grep -c "facet\|Facet\|highlight" app/lamad/src/app/components/search/search.component.ts` → 0.
- `search.service.ts:104-105,129` — facets computed and returned on every search.
- `search.model.ts:112,115,178` — `matchedFields`, `highlights`, `facets` fields
  exist on the wire-facing result types.

OPEN QUESTION: should facet *counts by reach* render for reach values the current
human cannot access, or be suppressed? The service computes counts from all
client-visible results, so today this is moot (substrate already filters reads),
but a deliberate decision belongs in the implementation PR.
