---
id: "backlog-search-model-dead-ranking-api"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "search.model.ts still declares a client-side ranking policy the client no longer owns — zero consumers app-wide"
slug: "search-model-dead-ranking-api"
written: "2026-09-24"
author: "agent:implementer@claude-sonnet-5 (Lane S)"
status: "backlog"
priority: "low"
domain: "lamad"
area: "app/lamad/src/app/models/search.model.ts — client ranking helpers superseded by R-S6"
tags: [search, lamad, dead-code, backend-authoritative, ranking]
relatedNodeIds:
  - content-search-station-4
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - app/lamad/src/app/models/search.model.ts
  - app/lamad/src/app/services/search.service.ts
shift_objective: |
  Remove `SEARCH_FIELD_WEIGHTS`, `SEARCH_MATCH_BONUSES`, `extractSnippet`, and `highlightMatches`
  from `app/lamad/src/app/models/search.model.ts` once a fresh, repo-wide grep confirms zero
  consumers remain (the check must be re-run at removal time, not trusted from this atom — code
  moves between now and then). Done when the four symbols are gone and `just gate` for the lamad
  bundle stays green, closing out the last client-side ranking-policy residue from the
  backend-authoritative migration (R-S6, landed `c8f8fcad8`).
---

# A client-side ranking policy the client no longer owns

**Chain:** search recipe scores and orders results server-side (R-S6, ruling landed in `c8f8fcad8`
— "search is the peer's recipe-governed view — server order, server facets, provenance printed")
→ the lamad search page renders the server's order and snippets as given.

**Between (A→C):** the server-authoritative ranking pipeline → `search.model.ts`'s
`SEARCH_FIELD_WEIGHTS`, `SEARCH_MATCH_BONUSES`, `extractSnippet`, and `highlightMatches`, which
together are a full client-side scoring and snippet-highlighting policy for exactly the same
decision the backend now makes.

**Missing node:** nothing calls these four symbols. A repo-wide grep for each name across
`app/lamad/src`, `app/elohim-app/src`, and `genesis/a2o` returns only their own definitions in
`search.model.ts` — no import, no call site, no test. They are declared but dead: a ranking policy
still on paper after the authority that would have run it moved to the peer.

**Probe:** `grep -rn "SEARCH_FIELD_WEIGHTS\|SEARCH_MATCH_BONUSES\|extractSnippet\|highlightMatches"
app/lamad/src app/elohim-app/src genesis/a2o` returns four hits, all in
`search.model.ts` itself (lines 259, 268, 369, 396) — zero consumers anywhere else in the tree, as
of this writing.

**Current state:** left in place. Backend-authoritative (R-S6) landed the server-side recipe path
and the search service (`search.service.ts`) already renders server order/facets/snippets
directly — these four symbols are the pre-R-S6 client-ranking shape that the migration made
redundant but did not remove. Removal is deliberately deferred to "a later pass with a
grep-proven zero-consumer check" (this atom's own words) rather than done inline here, since a
dead-code removal earns its own small, isolated diff and its own fresh verification rather than
riding along with unrelated work.
