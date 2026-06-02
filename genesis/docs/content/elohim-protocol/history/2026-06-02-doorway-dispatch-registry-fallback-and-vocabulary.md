---
title: "History/ADR: Doorway dispatch — registry-as-universal-fallback + storage vocabulary"
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [doorway, dispatch, routing, blob, quilt, pantry, vocabulary]
# DISTILLS two landed sprints (registry-fallback dispatch fix + vocabulary cleanup).
# Routing truth now lives as live classify_dispatch + green contract tests; vocabulary
# truth lives in genesis/graphos/vocabulary.md. Raw plan bodies retire to git.
distills:
  - genesis/docs/superpowers/plans/2026-04-28-doorway-blob-registry-routing.md
  - genesis/docs/superpowers/plans/2026-04-30-vocabulary-cleanup-sprint-kickoff.md
canonical:
  - ../../../../../doorway/doorway-service/src/server/CLAUDE.md   # the dispatch-contract surface
memory_anchors:
  - project_doorway_manifest_driven_routes
  - project_doorway_single_target_no_fanout
  - project_quilt_pantry_vocabulary
  - feedback_head_vs_get_blob_asymmetry
---

# Doorway dispatch: registry-as-universal-fallback + storage vocabulary (2026-04-28 → 2026-04-30)

> **One-sentence lesson:** A dispatch that enumerates path-prefixes is a standing regression vector — it
> silently misclassifies every future manifest path family the day it ships. Make the RouteRegistry the
> *unconditional* fallback; never reintroduce a prefix guard in the wildcard dispatch arm.

**What was built.** The recurring `/blob/<hash>` thumbnail regression was traced to a hand-maintained
prefix guard in the doorway dispatch tail (`p.starts_with("/api/v1/") || p.starts_with("/account/")` at
`http.rs:1504`). Every time elohim-storage's manifest added a new top-level path family
(`blob_proxy → /blob/`, `stream_proxy → /stream/`), those requests skipped the RouteRegistry entirely
and fell into the SPA bootstrap, so thumbnails rendered as HTML. The fix extracted the dispatch decision
into a unit-testable `classify_dispatch` helper returning a `Disposition` enum, replaced the
prefix-gated arm + GET catch-all + default 404 with a single wildcard arm delegating to it, and pinned
the contract with four `dispatch_classification_tests`. Two days later a vocabulary-cleanup sprint
piggybacked: it migrated ~11 TypeScript callers off legacy `/store/<hash>` and `/api/blob/<hash>` to
canonical `/blob/<hash>`, deleted the dead legacy dispatch arms, and introduced the
`quilt`/`pantry`/`stock`/`draw`/`RS(N,K)` design register (rejecting `weave` for a Moss `@theweave/api`
identifier collision, and `lattice` for the holonic-governance collision).

**What superseded it / where the truth now lives.** Both plans landed (`8022ff361`, `51adf5118`,
`4c1580133`). The routing truth is the live `classify_dispatch` + `Disposition` in
`doorway/doorway-service/src/server/http.rs` and its green contract tests — verified behavior remembered
AS tests, not a parked plan. `classify_dispatch` has since become load-bearing: extended for SSR
(`classify_dispatch_returns_ssr_route_when_render_spec_set`) and the `root_app_slug` arm retired when
`ROOT_APP_SLUG` gave way to urlPath=/ projection (`2ac1431fa`). The vocabulary truth lives in
`genesis/graphos/vocabulary.md` (Status: Active reference).

**Why we turned.** A dispatch that enumerates path-prefixes is a standing regression vector: it silently
misclassifies every future manifest path family the day it ships, surfacing weeks later as a
user-visible content bug (the Jan/Mar/Apr `/blob` whipsaw). Making the registry the unconditional
fallback honors the `doorway/CLAUDE.md` contract — "adding a new endpoint to elohim-storage
automatically makes it routable through doorway, no doorway code change" — converting an open-ended bug
class into a closed one. The vocabulary turn recognized that `blob/store/s3` framing is hostile to the
protocol's actual shape (peers stewarding shards in pantries).

**Watch-out for future planners.**
1. Never reintroduce a path-prefix check in the wildcard dispatch arm — the registry already knows its
   own prefixes; a prefix guard there is the exact regression this work eliminated.
2. `Disposition::RegistryUnhandled` 404s today; `BlobProxy`/`StreamProxy`/`ZomeCall`/`AgentProxy` target
   dispatch is the intended extension point and must plug in there, not via a new top-level arm.
3. Vocabulary boundary rule is load-bearing: `quilt`/`pantry`/`stock`/`draw` apply to design language,
   signals/events, and NEW invented identifiers — NOT to wire-level HTTP routes (`/blob/{hash}`),
   content addresses (`sha256-{hex}`), or existing Rust types (`BlobStore`); renaming those breaks
   external legibility.
4. `weave` is permanently off the table (Moss collision); `lattice` is taken by holonic governance.

## Bidirectional links

- **This record → canonical:** [doorway server dispatch-contract CLAUDE](../../../../../doorway/doorway-service/src/server/CLAUDE.md) (the exact spot the regression recurs).
- **Distilled-from (raw bodies in git history):** doorway-blob-registry-routing + vocabulary-cleanup-sprint-kickoff (linked in frontmatter).
