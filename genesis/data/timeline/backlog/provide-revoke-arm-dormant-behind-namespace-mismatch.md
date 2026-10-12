---
id: "backlog-provide-revoke-arm-dormant-behind-namespace-mismatch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The provide reconciler's revoke arm has never run, and every restart authored duplicate commitments: the node read its own rows under the transport id while they are written under the agent key"
slug: "provide-revoke-arm-dormant-behind-namespace-mismatch"
written: "2026-10-12"
author: "claude (idle call-floor sprint 2, operator-directed)"
status: "open"
priority: "medium"
severity: medium
nodes: [alpha, household]
relatedNodeIds:
  - "habit:idle-is-free"
tags: [elohim-storage, provide-reconcile, mishpat, commitments, performance, perf-io, perf-scale, idle-is-free, decision, open-question, cross-namespace-identity, trustful-self, friction-mechanical, plane-value, plane-custody, unit-item, phase-transition, lane-background]
cites:
  - elohim/elohim-storage/src/services/provide_reconcile.rs
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
  - genesis/data/timeline/backlog/provide-reconciler-actual-set-transport-id-mismatch.md
  - genesis/data/timeline/backlog/rea-inventory-advertises-rows-of-another-notary.md
---

**What.** `ProvideReconciler::reconcile_provides` (`elohim/elohim-storage/src/services/provide_reconcile.rs`) reads
this node's live `replicates-commons` rows from `mishpat_commitments` under the libp2p transport id (`12D3Koo...`),
which `src/main.rs` (about line 2407) passes as `self_cid` on a 60 s tick. `ConductorCommitmentAuthor::author_commons`
writes those rows under the Holochain agent key (`uhCAk...`). The read matched nothing. This is the same class of
mistake as the older `provide-reconciler-actual-set-transport-id-mismatch`: one identity namespace used to look up
rows written in another.

**Consequence 1: duplicate authoring (cured in code, not yet measured).** Because the read found no live rows, every
storage restart re-authored a notarized commitment for every desired scope. Measured by a read-only sqlite census on
2026-10-12: matthew holds 729 live, unrevoked mirrored rows for 154 scopes, up to 8 per scope, created between
2026-10-04 and 2026-10-12 at 20, 54, 66, 70, 95, 135, 135 and 154 rows per day; jessica holds 613 and james 202;
none is revoked on any of the three. `deterministic_valid_from` buckets to the UTC day, so same-day restarts share
an entry hash and the per-day row count understates the number of conductor writes. The cure is commit `3eb2b52c3`
on branch `perf/idle-cpu-residual-sprint`: the author arm now reads live rows under any of the author's own
agent-key candidates (session key, then cell key). It had not been measured on the household when this was written.

**Consequence 2: the revoke arm has never run (open, the subject of this entry).** The arm that revokes "live actual
rows with no desired key" lives in the same function and has never fired, for the same reason. The cure above
deliberately left it reading the transport namespace, and a unit test pins that
(`the_revoke_arm_issues_nothing_for_rows_under_the_agent_provider`). Waking it as written would be wrong three ways.

1. It sets `target_cid` to `row.dht_anchor_hash` (the action hash), falling back to `row.cid`. The revoke projection
   (`parse_revokes_commitment` in `src/mishpat_projection.rs`, through the signal handler in `src/signals.rs` near line
   1009, to `mishpat_commitments::set_revoked_at`) matches `WHERE cid = ?`, which is the entry hash. The revocation
   would be notarized and would never mark the row.
2. It does not consult the per-key latch, so it would re-issue a notarized revocation for every stranded row every
   60 s.
3. It is unbounded per tick.

Separately, `main.rs` skips the whole reconcile when the desired set is empty, so a node with nothing desired never
revokes anything.

**Standing damage, not cleaned.** About 574 excess content rows on matthew (one live commitment per scope would be
154 or 155), in similar proportion on the other peers. The same defect has been running on the alpha fleet on every
restart; the fleet's count was not checked. 392 of matthew's 730 mirror rows are `epr-app-deliverability-*` fixture
scopes whose pins are gone. Readers: the resilience card's commitment counter counts every active row
(`elohim/elohim-facings/src/folds/replication_commitment.rs`, line 214), so it is inflated; distinct-household
coverage stays de-duplicated; `aggregate_pledges_by_tier` reads only `replicates-dwelling` and is unaffected. Bounds
rate accounting is per commitment CID (`src/services/bounds_validator.rs`, line 271), so N live commitments for one
scope plausibly multiply the usable rate allowance. That last point is inferred from the code and not demonstrated.

**Related, left as is.** `ProvideReconciler::observe`, called from `P2PNode::run_provide_reconcile`
(`src/p2p/mod.rs`, about line 10187), has the same transport-namespace read. It only sets latch stages and never
authors, but `P2PNode` holds no conductor client, so its `needs_commitment` count is inflated.

**Relation to the habit.** `idle-is-free` says the cost of a quiet hour does not depend on how much history the node
holds. A restart that writes one notarized commitment per desired scope per day is a cost that grows with scopes and
with restarts, and it shows in the habit's `dna-actions-per-day` check for mishpat. Consequence 1 is the part of this
that reaches the habit; the revoke arm is a correctness and cleanup question that the habit does not bound.

**The decision.** Revocations are notarized, irreversible acts under steward keys, so this is the operator's to
decide, per network.

1. Leave the duplicates and stranded rows standing. Zero writes; the counters and plausibly the rate allowances stay
   inflated.
2. Fix the arm: target `row.cid`, consult the per-key latch, bound the number of revocations per tick, and let it
   revoke stranded scopes going forward.
3. As well as (2), run a bounded one-time owner-authored pass that keeps one terms-compatible commitment per scope
   and revokes the rest. On matthew this is about 574 `create_commitment` calls, each a new mishpat entry.
4. De-duplicate on the reader side. Zero conductor calls; it fixes presentation only, unless bounds accounting is
   corrected too.

Recommended first step: (2). Whether to run (3) is the operator's call per network (the household fixture and alpha
are different cases), after (2) has been measured.

**Done when.** The operator has chosen among the options, and for the chosen one a test shows a revocation marks the
row it targets (the entry hash case), is issued once per row, and is bounded per tick; or the choice is (1) and this
entry records it as a decision.
