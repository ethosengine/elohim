---
id: "backlog-observation-session-route-namespace-collision"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Separate the observation-session routes from the observation-layer routes under /api/v1/observations"
slug: "observation-session-route-namespace-collision"
written: "2026-09-24"
author: "agent:implementer@claude-opus-5-5 (Lane A, task A9)"
status: "backlog"
priority: "medium"
domain: "D2"
area: "elohim-storage http routing"
tags: [observation, routing, namespace, http, lane-a]
relatedNodeIds:
  - attention-witnessed-privately
  - observation-vocabulary-collision-disambiguate
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/api/observations.rs
  - genesis/data/timeline/backlog/observation-vocabulary-collision-disambiguate.md
shift_objective: |
  Give the observation-session API (begin / entries / report — a2o diagnostic capture) and the
  observation-layer API (write, stream, by-subject, by-observer, diversity — the substrate
  witness) disjoint route prefixes, declared in build_manifest(), so neither can shadow the
  other. Done when no path under one prefix can reach the other's handler, the exact-shape
  matcher is_observation_session_path is deleted, and the doorway route manifest lists both
  families distinctly.
---

# Two APIs share /api/v1/observations

**What happened.** `http.rs` matched `p.starts_with("/api/v1/observations")` and sent every
path under the prefix to the observation-*session* handler (`/begin`, `/{id}/entries`,
`/{id}/report`), ahead of the `/api/v1/` catch-all that dispatches to `api/observations.rs`.
As a result the observation-layer GET routes had never served. Nobody noticed, because the only
tests were DB-layer tests. Task A2 narrowed the arm to the three exact session shapes
(`is_observation_session_path`), and an `http.rs` unit test pins that the bare path is no
longer captured.

**Why that is not the end.** The fix is a matcher over shapes, and it still assumes the two
families never overlap. `/api/v1/observations/{id}/report` is a two-segment path under the
same prefix as `/api/v1/observations/stream`. A future layer route of the shape `{x}/entries`
or `{x}/report` would be captured silently again. The names also invite confusion: the
vocabulary collision atom (`observation-vocabulary-collision-disambiguate`) records that
"observation-session" (a2o diagnostic) and "observation-event" (substrate witness) are
different things that share a word.

**Direction.** Move the session family to its own prefix (for example
`/api/v1/observation-sessions/…`), keep the old paths as a short-lived redirect or refusal,
and declare both families in `build_manifest()` so the doorway routes them by declaration and
never by prefix accident. Then delete `is_observation_session_path`. Pick the new prefix together
with the vocabulary atom so the rename happens once.
