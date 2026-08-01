---
id: "backlog-rea-heal-classify-write-toctou-transactionalize"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "REA heal classify-then-write TOCTOU — a same-id live signal in the heal window can be misclassified Refreshed; transactionalize read+upsert"
slug: "rea-heal-classify-write-toctou-transactionalize"
written: "2026-08-01"
---

# REA heal classify-then-write TOCTOU — transactionalize read+upsert

**Status:** open · bounded code fix
**Found:** 2026-08-01, post-push parallel review of `98a3c5f1e` (REA divergence adjudication)
**Owner surface:** `elohim/elohim-storage/src/p2p/projection_reconcile.rs` (`heal_one`, ~:2136-2181) + `db/rea_commitments.rs` (`upsert_with_anchor`, ~:1169)

## Finding

`heal_one` reads the pre-write anchor (`get_commitment`) and then calls
`upsert_with_anchor` as separate unguarded statements. The live
`ReaProjectionSignal` handler (`rea_projection.rs:562`) writes the same table
via the same function on its own task — not covered by the reconcile
single-flight guard. A signal landing in the window can move the row to a
fresher anchor `V`; heal's write then silently clobbers `V` while the
classification (computed from the stale pre-read) counts the row
`Refreshed`/`divergent_refused`.

**Bounded impact:** one-sweep misclassification, self-correcting — discovery
recomputes divergence against peers every sweep, so a genuinely divergent row
re-enters `divergent_anchor` next cycle (converged flips back false). Not a
permanent gauge lie; the race window is a handful of DB statements and needs a
same-id signal to land inside it.

## Fix direction

Make read-classify-write atomic: either wrap `get_commitment` + upsert in one
`conn.transaction(...)` in `heal_one`, or have `upsert_with_anchor` return the
previous anchor observed inside its own transaction and classify from that
return value. Prefer the second (removes the double-read entirely).

Clean-verified in the same review: empty/backward anchor semantics
(`Advanced` never falsely reduces actionable), the exhausted/refreshed
partition invariant (disjoint subsets of `divergent_ids`), and the
`saturating_sub` underflow backstop in `publish_sweep`.
