---
id: "backlog-content-head-declined-declaration-has-no-retained-hydration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A T-2-declined ContentHeadDeclared leaves no retained work — discovery compares anchors, so a declined declaration reads InSync"
slug: "content-head-declined-declaration-has-no-retained-hydration"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 review"
status: "open"
priority: "medium"
jobs: [elohim]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [dataplane, head-adoption, projection-reconcile, content-head-declared, convergence]
---

**The fact.** `handle_authenticated_content_head_signal` (`elohim/elohim-storage/src/rea_projection.rs:740-789`)
is the async arm every complete-ordered `ContentHeadDeclared` signal is routed to
(`requires_authenticated_head_projection`, `rea_projection.rs:609-618`). Its own doc comment states the contract
plainly: "an unavailable, unattempted, failed, stale, or malformed answer writes nothing... an already-split row
awaits another exact signal or authenticated projection rather than being mutated further" (`:734-739`). Its sole
caller (`main.rs:1926-1939`) is fire-and-forget: on `Err` it logs `warn!("authenticated REA/content signal
deferred")` and drops the signal — there is no retry queue, no re-enqueue, nothing "deferred" actually happens.
Discovery cannot pick the row back up either: `classify_content_gap` (`p2p/projection_reconcile.rs:4454-4476`)
compares `local_anchors` against `peer_anchor` — a row whose anchor already equals the peer's classifies
`ContentGap::InSync` regardless of whether a declaration for it was ever hydrated. The reanchor sweep
(`services/reanchor_backfill.rs:257-314`) selects candidates from exactly two arms, "disjoint by construction
(`dht_anchor_hash IS NULL` vs `IS NOT NULL AND dht_anchor_state = 'dead'`)" — a row with a live, non-dead anchor
is invisible to it no matter how stale its declared-head content is.

**Evidence.** The event-driven fast path (`services/head_adoption_trigger.rs`) needs a `doc_hint`
(`retry_warranted`, `:268-293`; `decide`, `:491-505`) and refuses a non-canonical answer (`:511-512`: "canonical
== false → None — a fallback is not an authority"); its re-probe ladder is spent after 60s ("`the ladder is
spent — leave the id to the sweep`", `:336`). The pure-iroh transport path never raises it at all:
`p2p_iroh/sync_driver.rs::reverse_project` (`:266-291`) only calls `reverse_project_content_doc` — no reference
to `head_adoption_trigger` exists anywhere in that file. All three gaps (declined-with-no-retry, anchor-only
discovery, iroh-only silence) are independently confirmed by the codex review of the reverted 1.4b attempt as
its finding 9 (`codex-review-1.4b.result.md`): "a T-2-declined head gets no later doc change, its producer fill
fails, or it arrives only through iroh. The carried branch never rescues it... failure there is not durable retry
work."

**Why it matters.** A row can sit with a declined declaration indefinitely with nothing watching for it: no doc
change re-fires the trigger, the sweep's anchor comparison already reads it as converged, and an iroh-only peer
has no path to the trigger at all. This is a distinct mechanism from
`torn-row-never-selected-for-repair.md` — that item is about a torn SQL *pointer* never being selected for repair
once torn; this item is about a *declaration that was never applied in the first place* leaving no trace for
discovery to find, because discovery's only signal (anchor equality) was never touched by the failed hydration
attempt.

**Smallest next step.** Retain declined declarations as bounded, retryable hydration work independent of
inventory/anchor divergence — a small durable queue or a `content_head_pending` marker the reanchor-style sweep
can also drain — and wire the iroh path to the same trigger `head_adoption_trigger.rs` already offers the
libp2p/dual path. `story-1.4a-design.md` and the 1.4b review both treat this as part of the same "T-1/T-2
recovery is incomplete" cluster; scope it as its own slice rather than folding into either.

**Links.** Review: `genesis/a2o/reports/recovery/serving-edge-20260920/codex-review-1.4b.result.md` (finding 9).
Sibling, distinct mechanism: `torn-row-never-selected-for-repair.md`. Related design:
`head-authority-carried-with-content-sync-unit.md` (open items 3, 6, 7 name adjacent non-converging gaps in the
same subsystem). Habit: `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`.
