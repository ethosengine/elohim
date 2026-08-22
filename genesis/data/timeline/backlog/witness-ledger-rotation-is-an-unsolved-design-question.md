---
id: "backlog-witness-ledger-rotation-design"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "How to rotate or compress a witness ledger is an unsolved design question — governance-findings is 2.8MB and unbounded"
slug: "witness-ledger-rotation-is-an-unsolved-design-question"
written: "2026-08-05"
author: "operator + Claude Opus 5 (worktree commit triage)"
status: "backlog"
priority: "medium"
domain: D-governance
tags: [epr-meta, witness-ledger, governance-findings, retention, repo-weight, design-question]
cites:
  - .claude/data/governance-findings.jsonl
  - .claude/scripts/_lib/epr_meta.py
  - genesis/data/timeline/backlog/epr-meta-unregistered-validators.md
---

# Witness-ledger rotation is a design question, not a chore

`.claude/data/governance-findings.jsonl` is the `.epr-meta` compose-gate's witness ledger: every
time a rule fires on a write, the appender records the decision. It is **3,493 bytes at HEAD and
2,798,458 bytes in the worktree** — a single uncommitted delta of +1,954 rows, of which **1,760 are
`decision: permit`** (an advisory fired and allowed the write) and only 33 are `refuse`.

`epr_meta.py` has a `FINDINGS_LEDGER_REL` appender and **no rotation, cap, or prune anywhere**. At
this growth rate the ledger dominates repo weight within weeks. The delta was held back from the
2026-08-05 commit sweep for exactly this reason.

## Why this is not "just add logrotate"

Operator, 2026-08-05: *"how to rotate/compress a witness ledger is actually a design decision that
we haven't gotten to."*

A witness ledger is not application logging. Its value is that it can be **replayed and verified** —
"this write was permitted under this rule at this version." Any retention scheme is a claim about
what the commons is still able to prove later. The open questions:

- **What is the witness FOR?** Proving a specific past write was governed, or measuring aggregate
  rule pressure? The first needs per-row durability; the second only needs counts.
- **Is a compacted ledger still a witness?** If 1,760 `permit` rows collapse to a count plus a
  digest, the artifact stops being evidence about any individual write. That may be fine — but it
  is a governance decision about the standard of proof, not a storage tweak.
- **Who is the audience?** If it is only the local drift gate, it need not be in git at all. If it
  is peers verifying that a doorway's governance actually ran, it belongs on the substrate as
  content-addressed blobs, not as a file in the monorepo — the k8s-is-not-the-architecture lesson
  applied to audit trails.
- **Asymmetric retention.** `refuse` and `ask` rows are rare and load-bearing; `permit` rows are
  bulk and individually near-worthless. Retention probably differs by decision class — but that
  asymmetry needs stating as policy before it is coded.
- **Where does the compaction boundary live?** Time window, row count, byte ceiling, or a
  ratification event (rotate at each `dev -> main`)? The last is the most protocol-shaped: the
  witness for a ratified period is sealed and the live ledger restarts.

## What to decide

Write the retention **policy** first (a `.claude/epr-meta/policies.yaml` row, so the rule that
governs the witness is itself witnessed), then implement the appender change to honor it. Until
then the ledger keeps accumulating in the worktree uncommitted, which is a stable holding pattern
but not a resolution.

## Signals to watch

- Worktree delta size at each commit sweep (2.79 MB as of 2026-08-05).
- `permit` : `refuse` ratio (currently ~53:1) — if advisories dominate this hard, the injects may
  be too broad, which is a separate finding about rule tuning.

---

## Measurement, 2026-08-22 — and the rule this class was missing

**The growth is now three-fold and the read path still does not exist.**

| observed | size | source |
|---|---|---|
| 2026-08-05 | 2,798,458 B | this row, at authoring |
| (undated) | ~4.3 MB | `genesis/manifests/habits.yaml:1691` — *"written with zero readers, which needs a read path or a stop-write"* |
| **2026-08-22** | **7,806,004 B** | measured this session |

A repo-wide grep for a read path (`read`/`load`/`open(`/`json.loads` against the filename) returns
**nothing**. Every reference is a writer (`_lib/epr_meta.py:1566,1587`; `hooks/epr-meta-git-gate.py:12`;
`package-projections.mjs:50,382`), a `"ledger":` declaration under `.epr-meta/elohim/packages/`, or prose.
Three independent surfaces have now flagged it — this row, the habits register, and the
2026-08-22 artifact subtraction audit — and it has grown through all three.

**Why deleting it does not close this row.** The file returns on the next hook fire. The stop-write
touches ~30 package declarations plus a projected hook sitting behind a pre-push projection-drift gate
(`.husky/pre-push.bash:136-147`), so it is a scoped change, not a `git rm`.

**The rule the class was missing** (named 2026-08-22, from the operator's observation that Jenkins has
both halves and we use only one):

> Every retained artifact needs TWO things — an **automatic forgetting rule** (a clock, a count, or a
> comet tier) and a **PIN that survives it, released by a DECISION rather than by time.**

This ledger has neither, which is why "rotate or compress" read as an open design question rather than
a policy choice: the question was never *how* to rotate, it was that no artifact class in this repo had
a declared forgetting rule paired with a decision-gated pin. The shape already exists four times over
and was nowhere named — Jenkins `buildDiscarder(logRotator(numToKeepStr))` runs at 20/100/50/30/30
across five pipelines with **no** `keepLog` pin anywhere in the repo; `memkit-retention.py` runs a
HEAD/TAIL/CORE comet whose spine is permanent but *automatic*, not decision-gated; `.epr-meta`
`retire-when:` is a decision-gated pin for **rules**; `habits.yaml` `best_observed:` is one for a
habit's high-water mark. **Nothing pins a result or a ledger row.**

So the concrete shape of the fix for this row: a forgetting rule (rows older than N, or beyond the
newest N per fingerprint), plus a pin for any row that is standing evidence for an open question —
released when that question closes, not when a counter rolls. A `decision: permit` row with no open
question behind it is exactly what the forgetting rule should take, and it is 1,760 of the 1,954 rows
this row was authored about.

**Still an operator decision** (unchanged from the original framing, now with the cost priced):
stop-write versus keep-writing-and-forget-on-a-rule. The audit that re-measured this recommends
scoping the stop-write separately from any deletion, because deleting first guarantees the file
returns before the scoped change lands.
