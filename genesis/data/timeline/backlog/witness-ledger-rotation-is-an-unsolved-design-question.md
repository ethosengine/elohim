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
