---
id: "backlog-relayed-response-drops-holder-retry-after"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "build_relayed_response does not carry the holder's Retry-After to the client, so a client behind a fully-shedding federation learns nothing about when to come back"
slug: "relayed-response-drops-holder-retry-after"
written: "2026-09-20"
author: "serving-edge failover-balance-stream campaign, 2026-09-20 review"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:served-under-standing"
tags: [doorway, name-routing, retry-after, federation]
---

**The fact.** `build_relayed_response` (`doorway/doorway-service/src/services/name_routing.rs:1422-1458`)
restamps the holder's `content_type`, `cache-control`, `standing`, `bundle` and `receipt` headers verbatim
onto the response it hands back to the client, but has no arm for `Retry-After`. `HolderReply` does carry
`retry_after_secs: Option<u64>` (`:1122-1127`, landed in `8800f274c`, story 3.1) — the value is read and
used to feed `ShedMemory`'s internal demotion window — but that value never reaches the builder. When
every holder for a name is shedding and answers 503, the client-facing response the relaying doorway
returns carries no `Retry-After` of its own, regardless of what any holder named.

**Evidence.** `doorway/doorway-service/src/services/name_routing.rs:1422-1458` (the full body of
`build_relayed_response`, confirmed by direct read 2026-09-20 — no `Retry-After` header is set anywhere
in the function); `:1105-1140` (`HolderReply` struct, showing `retry_after_secs` is captured but only
consumed by `note_backpressure`/`ShedMemory`, not by the response builder); call sites
`doorway/doorway-service/src/server/http.rs:6939`, `:7299` (both construct the final client response via
this function). Found in review of `8800f274c` ("a holder that says it is busy is set aside for the time
it named — story 3.1").

**Why it matters.** Story 3.1 landed the selector-side use of a holder's declared window (ordering which
holder is dialed) but not the client-facing propagation of the same fact. A client that exhausts every
holder in one relay attempt gets a bare 503 with no window to wait on, and will poll on its own schedule
instead of the window the busiest holder actually named — the same information-loss class the campaign is
otherwise closing at the relay's internal-ordering layer.

**Smallest next step.** When every candidate in `relay_one_hop`'s slice has been tried and all answered
with backpressure, carry the smallest (or most recent) observed `retry_after_secs` onto the final
response's own `Retry-After` header before returning it to the client. This needs no new observation —
`RelayOutcome.shed_doorways` already has the values — and no new DHT entry type or route.

**Links.** Story: `genesis/a2o/reports/recovery/serving-edge-20260919/story-3.1-design.md` (the design this
extends). Habit: `doorway/doorway-service/.epr-meta/served-under-standing.habit.md`. Plan:
`genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md` story 3.1.
