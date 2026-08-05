---
id: "backlog-mempalace-currency-records-mine-while-lock-blocked"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "mempalace-currency --remine records a mine and prints 'fresh ✅' even when every operation was lock-blocked"
slug: "mempalace-currency-records-mine-while-lock-blocked"
written: "2026-07-30"
author: "memory-ceremony"
status: "wip"
priority: "medium"
verified_by: |
  memory-ceremony 2026-07-30 — fixed and verified by reproduction. Held the palace with a
  concurrent `mempalace ... mine` and ran `--remine`: it printed ABORTED, exited 1, and left
  .mempalace/.last-mine byte-identical (1785419850.737828 before and after). Pre-fix the same
  condition recorded the mine and printed `fresh ✅` at exit 0. Clean-path run still records
  and exits 0. Landed on feat/angular22-node24 in 72c285a8e; not yet delivery-verified,
  so the status stays `wip` — only /deliver mints active.*/stable.
tags: [false-green, mempalace, memory-kit, tooling, evidence-discipline]
cites:
  - .claude/scripts/memory-kit/mempalace-currency.py
  - .claude/scripts/memory-kit/placement-audit.py
shift_objective: |
  Make mempalace-currency --remine fail loudly instead of falsely green. If any sync/mine
  subprocess reports "palace ... is held by PID <n>", do NOT write the freshness stamp and do
  NOT print "fresh ✅": exit non-zero with a message naming the holding PID, or block on the
  lock and retry. Freshness must be recorded only on evidence that files were actually walked.
  Done when a --remine launched while another mine holds the palace exits non-zero, leaves the
  previous mine date in place, and the SessionStart headline still reads "re-mine due".
---

## What

`.claude/scripts/memory-kit/mempalace-currency.py --remine` records a successful mine and reports
`fresh ✅` **regardless of whether any mining happened**. Observed 2026-07-30 during a
`/memory-ceremony` Phase 4d: a second invocation collided with an in-progress mine, printed

```
mempalace: palace /projects/elohim/.mempalace/palace is held by PID 489309 ...
```

once per operation (4×), and then still emitted:

```
mempalace-currency: recorded mine @ 2026-07-30
mempalace: fresh ✅ (mined 2026-07-30)
EXIT=0
```

The SessionStart headline immediately flipped from `⚠ 145 surface file(s) changed since last mine`
to `mempalace: fresh ✅`, on a run that walked zero files.

## Why it matters

The freshness stamp is what the `memory-stasis-loop` gate and the ceremony's Phase-4d exit criteria
read. A stamp written without evidence means the index can silently fall behind the front-link
while every dashboard reads green — and the failure is self-concealing, because the next run also
reads "fresh" and skips the work.

This is a sibling of the `replant.mjs` defect closed on the same branch, where the tool reported
"planted, verify clean" while leaving real verify failures. Same lesson, different tool: **a tool
that reports green on blocked work is worse than one that fails**, because it removes the signal
that would have triggered a retry.

Note the ceremony's own end state was fine — a later lock-free run walked 441 files across three
corpora and found them all already filed, so the index genuinely is current. The defect is the
*reporting*, not the index.

## Resolution (2026-07-30)

`remine()` now captures each step's output and exit status via `_run_step()` and treats the
`is held by PID` marker — or any non-zero child exit — as a hard failure: it skips the
`record()` write, prints an ABORTED line naming the cause, and returns 1, so both the caller and
the SessionStart headline keep showing `re-mine due`. `main()` propagates that through
`sys.exit()`. A `--remine --wait` variant retries every 30s (12 attempts) instead of bailing, for
unattended callers that would rather block than skip.

The root cause was `check=False` on every subprocess with output uninspected, followed by an
unconditional `record(root)` — success was assumed rather than observed.

Related: [[feedback_verify_the_measure_before_the_ranking]] (verify the measure, not the label),
and the memory-kit gotcha "a 'landed' / checkbox / ✅ claim is NOT done — 'done' is EARNED by the
verification gate, never self-asserted."
