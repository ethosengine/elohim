---
id: "backlog-declared-head-blob-serves-unauthenticated-content-under-verified-head"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The read path serves an unauthenticated doc blob under a verified head — declared_head_blob gates on a string match, never a conductor-verified binding (story 1.4c)"
slug: "declared-head-blob-serves-unauthenticated-content-under-verified-head"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 review"
status: "open"
priority: "high"
jobs: [elohim]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:doorway-failover"
tags: [dataplane, head-adoption, doorway-failover, blob-serving, torn-row]
---

**The fact.** `SyncManager::declared_head_blob` (`elohim/elohim-storage/src/sync/mod.rs:253-266`) loads the
Automerge doc, checks only that the doc's own `head_action_hash` field equals the string passed in, and — if it
matches — returns whatever blob hash the SAME doc carries. Nothing established that this doc's blob field is the
one the SQL row's `declared_head_action_hash` was ever verified against by a conductor call; the check is a
string comparison between two fields the doc itself supplies. `HttpServer::declared_head_served_blob`
(`http.rs:10773-10807`) calls it with the row's `declared_head_action_hash` and, on a distinct + locally-present
answer, PREFERS it over the row's own `blob_hash` (`http.rs:10419-10434`) and writes it into the in-memory
`slug_index` cache (`http.rs:10428-10431`) so the O(1) fast path repeats the same answer. The verified alternative
already exists in the same file: `resolve_exact_candidate_blob` (`http.rs:8252-8274`) fetches the record for a
candidate action and runs it through `call_validate_carried_head_record` before trusting it — `declared_head_blob`
does neither.

**Evidence.** Test `serves_declared_head_blob_when_present_and_local` (`http.rs:20733-20760`) endorses exactly this
behavior as a feature: a row whose own `blob_hash` differs from the doc's blob is made to serve the doc's blob
solely because the doc's `head_action_hash` string matches. A torn or stale peer's doc — `(head B, blob A)`, where
A was never conductor-verified against B — reaches a healthy peer whose SQL row is entirely correct; that peer
will serve A under B anyway once its doc projection carries the same pair, because the gate only compares two
strings inside one untrusted document. `story-1.4a-design.md` §c.3
(`genesis/a2o/reports/recovery/serving-edge-20260919/story-1.4a-design.md:319-332`) documents the same doc-vs-SQL
gap for a different consumer; the codex review of the reverted 1.4b attempt independently names this exact
function as BLOCKER 3 (`codex-review-1.4b.result.md`: "sync/mod.rs checks only the doc's head-hash string, then
accepts its independently controlled blob... the read path then serves A under B").

**Why it matters.** This is not the torn-row-selection gap (`torn-row-never-selected-for-repair.md`, which is
about `classify_content_gap` never re-probing a torn SQL row). It is upstream and worse: even a peer whose SQL
row is fully correct and never torn can be made to serve wrong bytes under a correct, verified head, because the
serving preference trusts an unauthenticated doc field over the row it is supposed to only supplement. It is
reachable during any mixed-version roll or any window where a torn/old peer's CRDT doc reaches a healthy peer
before that peer's own SQL state is what's asked about — the healthy peer's correctness offers no protection.

**Smallest next step.** Either (a) verify the doc's blob against the conductor before serving it — reuse
`resolve_exact_candidate_blob`'s pattern rather than the raw doc read, or (b) stop consulting the doc for this
decision at all and serve only the SQL row's own verified pointer for declared rows, moving `distribution`/`cid`
concerns elsewhere. This is the same "what does the CRDT doc get to assert unilaterally" question raised by
BLOCKER 8 of the 1.4b review (a stale doc field surviving a head move) — a shared fix (pairing/verification at
the doc-read boundary) may resolve both. p2p-design-gate applies: this decides what "verified" means at a read
boundary, not a header tweak.

**Links.** Plan story: `genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md`
story 1.4c. Review: `genesis/a2o/reports/recovery/serving-edge-20260920/codex-review-1.4b.result.md` (BLOCKER 3).
Sibling, distinct mechanism: `torn-row-never-selected-for-repair.md` (SQL-row selection) and
`head-authority-carried-with-content-sync-unit.md` (carried-record adoption redesign). Habit:
`doorway/doorway-service/.epr-meta/doorway-failover.habit.md`.

## 2026-09-26 — fixed on `dev` by `06e9587f8` (story 1.4c); fleet reading owed after the push

The serve path no longer consults the sync doc's `blobHash` for a row with a verified declared head: the C3
doc-blob preference (`declared_head_served_blob`, `SyncManager::declared_head_blob`) is gone from `http.rs` and
`sync/mod.rs`, and `read_head_blob_hash` / `doc_head_action_hash` survive only as test helpers. A head moves only
with the pointer its own proven record names, and only once those bytes are held. The row keeps its status until
the fleet serves the same bytes from both doorways (seam 6 `OK`), which only a post-push reading can show.
