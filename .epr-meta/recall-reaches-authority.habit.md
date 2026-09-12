---
epr-habit-version: 1
id: recall-reaches-authority
invariant: >
  One governed journey serves every agent question — a ceremony's "understand this assertion
  and its changed evidence" and a session's "what is the shape of this concern and what do I
  do first" are the same experience through the same algorithm (recall-contract.json), bounded
  by the same mishpat (the recall-journey ceilings) and measured by the same middot
  (recall-metered-bytes, recall-unmetered-bytes, recall-mistaken-assertions,
  recall-screens-to-shape). A context-reset agent carrying a concrete question reaches the
  current authoritative source through the entry alone: the session records the agent's own
  question as its intent, every surface an agent must recall (docs, code, skills, hooks,
  commands, measures, memory) is inside the declared scope, the bytes it read are metered, the
  first screen fits its door — whole-scope: stale edges grouped by shared source so specs can be
  converged and a head chosen; focused: the area's habit, its last delta and the competing
  sources ranked with the commands to compare, choose and file the residue — and the agent concludes without a mistaken assertion. Bytes saved
  never substitute for a correct decision; an unmetered direct read is a defect of the entry,
  not of the agent.
status: green
active: false
checks:
  - "a2o @concern:recall-reaches-authority (genesis/a2o/features/devflow/ceremony-reconciliation.feature — a fresh agent opens with its own question, the orientation carries it, a passage under .claude/skills/ is readable and receipted, and finish reports the question with bounded bytes)"
  - "just gate memory-ceremony (the native recall executor, corrections view and footprint suites plus the hook, lens and cucumber legs)"
  - "epr flow report --bound recall-journey-window-ceiling (the rolling-quarter rate over recall-mistaken-assertions@1 and recall-unmetered-bytes@1, grouped into journeys by the env:journey slot sample/judge write on every fold — the fraction of the last window_days worth of journeys that were NOT clean); MAY LEGITIMATELY read `skipped` while fewer than 3 journeys exist in the window — that is not a defect in the entry, only an early standing reader"
  - "epr flow report --bound recall-metered-bytes-ceiling / recall-unmetered-bytes-ceiling / recall-mistaken-assertions-ceiling / recall-screens-to-shape-ceiling: the four per-journey ceilings, each read against the LATEST journey alone — a fold is written per journey by the reader's own honest account and by `recall measure --phase close`; no fold is `skipped`, never green"
guard: >
  Regression risks: (1) greening by widening source_roots to everything — the private recall
  store and other worktrees stay refused, and a scope that admits scratch is not a governed
  entry; (2) greening by shrinking the human view until the Linked choices are gone — the
  emitted next-choice commands are the progressive-discovery payload, never optional; (3) a
  fresh-reader observation taken by the agent that made the change is not a fresh reader.
refs:
  - ".claude/workflows/recall-standing-reader.js — the weekly standing reader: samples every reader tier the actor sidecar has seen in 30 days x the six-question bank, judged by a seat of a different tier each time; routine: operator /schedule pending"
  - "genesis/docs/superpowers/plans/2026-09-11-bounded-recall-mastery-sprint.md"
  - "genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md — the verdict rule this habit inherits from dev-system-equilibrium, where it never belonged"
  - "genesis/docs/analysis/2026-09-09-memory-ceremony-efficacy.md — the five fresh-start questions"
  - ".epr-meta/elohim/algorithms/recall-contract.json — the method every receipt pins"
retire-when: >
  when the fresh-reader observation has been green for a full quarter of ceremonies with
  metered bytes inside the SessionStart budget and no unmetered direct read — an entry that
  reliably serves the next agent no longer needs a reader watching it.
---
DELTA 2026-09-11 (station 1 — reader lens; GREEN preserved, one more reader-sample): governed-discovery stations 0–1
landed on branch sprint/2026-09-11-governed-discovery (recall.rs split into eight seams under an 1,800-line ceiling with a
declared seam rule; Provider trait; contract as ProcessSpec+Bounds v10; reader lens from the actor sidecar with a declared
lens_table v11; render per lens with the honesty floor line and the content floor; print-never-sum asserted). Fresh reader 5
(sonnet claim → lens `simple`, binary 51bcb6c7…): authority reached, shape on screen 1, 6 operations, 16,116 metered bytes,
0 unmetered, 0 mistaken — all four recall-journey ceilings pass. Defect surfaced and queued (Task 1.2 fix round 2): at `simple`
a read/source/history rendered no primary payload and no `lens:` line, so the reader had to widen to `detail`; ruling: an
operation's own result and the lens line render at every lens, density bounds only candidate lists. Sequence on one question:
14 screens (Fable bootstrap) → 12 (reader 1, not reached) → 14 (reader 2, not reached) → 3 (reader 3) → 6 (reader 5 at simple).
GREEN 2026-09-11 (RED → GREEN on evidence, one reader-sample): contract v9, binary sha256 37cae28e…
(gate-built and installed at /opt/rust/cargo/bin/epr). Checks: (1) a2o @concern:recall-reaches-authority
passes inside `cucumber-js --profile ceremony` 6/6 scenarios, 34/34 steps; (2) `just gate memory-ceremony`
EXIT=0 — 51/9/10 native recall/corrections/footprint tests, 34 hook tests, both cucumber profiles;
(3) fresh-reader 3 (context-reset Sonnet, entry-only, question "what commands re-mine the MemPalace index
and when may the marker be stamped"): authority reached at .claude/skills/memory-ceremony/SKILL.md:192-207
in 3 operations, shape on screen 1, metered 8,058 B over 2 files, unmetered 0, mistaken assertions 0 —
all four recall-journey ceilings pass (folds on the plan, env reader=fresh-reader-3-sonnet). Sequence
that earned it on the same question: orchestrator bootstrap 14 screens/~100 KB unmetered/1 mistaken →
reader 1 (v6) zero candidates → reader 2 (v8) candidate present, passage not located → reader 3 (v9)
passage located on screen 1. What changed (native seat rounds 1-5): `open --need` is the intent; `.claude/`
in source_roots; two-line refusals with remedies; compact human view (12.6 KB → 4.5 KB); help answers on
stdout; focused first screen with habit lines + ranked candidates; `recall` headline slot replaces `memkit`;
UTF-8-safe metadata read; body-term discovery with a declared body_scan_bytes=65536 budget and
occurrence/rarity-weighted ranking; section hit annotation + located `read` choices; `finish` on a focused
journey without a concern edge. Integration seat: 393→122 kit-script mentions, all remaining historical;
package verifier 1972/0. Bounded by: ONE reader-sample on ONE question; retire-when needs a quarter.
Held frontiers on the plan: reader-context presets (lens per reader, imagodei↔sophia negotiation); the
focused door still prints empty ceremony blocks; the authoritative doc ranks second on lexical evidence
(an authority decision); six specs with unquoted `cites:` entries are unreadable to discovery.
RED WRITTEN 2026-09-11 (born red; the orchestrator's own bootstrapping into this sprint is the
first journey sample — 14 probe rounds and ~100 KB of unmetered direct reads before the concern's
shape was nameable, 1 mistaken assertion (looked for a memory habit that did not exist), 62,939
metered bytes from the test-drive baseline; folded on the four recall-journey measures, evidence from the memory-kit replacement's station six and the
2026-09-11 test-drive): two isolated after-readers reached authority 5/5 with 0 and 1 mistaken
assertions, tooling-scoped bytes 18,178 and 24,524, but honest totals with direct reads outside
source_roots were 85.2 KB and 49.1 KB — the byte leg fails for one reader because `.claude/` is
outside the entry's scope. Test-drive session `test-drive-20260911`: `open --need` dropped the
question (intent = recipe default), `search`/`read` under `.claude/skills` refused, three refusals
each re-printed ~900 B of orientation, the human `open` view was 12.6 KB of inline JSON. The a2o
scenario is not yet written; the gate is green on the pre-sprint binary.
DELTA 2026-09-12 (station 2 — the bootstrapping head; GREEN preserved, two bootstrap reader-samples): the SessionStart
head is `open --purpose bootstrap`, rendered as a ProjectionRequest (top red → its check → the atom's last delta → read-only
handed choices, then concern edges); `load-project-context.py` and `run-projection.py` are thin projections of that one view
at `minimal`/`simple` (BOOTSTRAP block ≤2,000 B inside additionalContext; `--event session` a no-op), so the first screen an
agent sees is the journey it continues, not a second instrument. Bootstrap readers folded on `recall-screens-to-shape@1`
(reader=sonnet-bootstrap: 1 screen, 0 mistaken; reader=haiku-bootstrap: 1 screen, 1 mistaken — it chose the headline's bare
`hold --scope --apply`; frontier: report.rs hands that mutation bare, outside this sprint). Harness capability tiering landed
alongside (Task 2.4, operator-directed after a shared-checkout hard reset): `.claude/hooks/capability-tier-gate.py`
(PreToolUse Bash) denies destructive git below the declared tier — reset --hard/--merge/--keep, checkout/restore discards,
clean -f, push --force/--delete/+ref, branch -D, switch -C, rebase, filter-branch, reflog expire, gc --prune, update-ref,
stash drop/clear, worktree remove --force, rm -r ., symbolic-ref writes — reading the tier from harness env or the actor
sidecar (unknown = deny) against one pinned policy row `destructive-git-requires-tier@1` (tamper = fail-closed). Tokenised
classification, deny-on-ambiguity scoped to destructive inner text, wrappers/exec/xargs/line-continuation/$PWD targets
covered: 56 gate tests, hook suite 106/108 (the memory-index idempotence red is a worktree-ledger artifact). Five review
rounds to the cap; residuals ruled out of class (interpreter strings, script files, encodings, remote exec — evasion, not tiering).
DELTA 2026-09-12 (station 3 — the standing reader; GREEN preserved, check 3 re-pointed): `derive: rate-over-window`
lands in report.rs beside `count-since-reset` — the fraction of a rolling `window_days` (default 91, declared 91 on
`recall-journey-window-ceiling@1`) worth of folds across recall-mistaken-assertions@1 and recall-unmetered-bytes@1
whose value is positive, `skipped` (never a zero) under 3 folds in the window. 3 new `flow_report.rs` tests green
(99/99 in the file; 0.2 exactly at the hard watermark reads `passed … within` under the default `above` comparator;
2 folds skips; a fold outside the window is excluded from both the count and the rate). Check 3 now reads the window
bound, naming the four per-journey ceilings as the still-live per-journey reading.
DELTA 2026-09-12 (station 3, fix round 1 — GREEN preserved, denominator made true): the bound now reads JOURNEYS, as
declared, rather than an approximating fold count — `sample`/`judge` write one more `env:journey=<FlowEvent cid>` slot
per fold (`note::JOURNEY_ENV_KEY`, one constant for both the writer and `evaluate_rate_over_window`'s reader), folds
sharing it are one journey, a fold with no such slot is its own singleton journey. `< 3 journeys` (not folds) skips —
guarded by a new test where 4 folds forming only 2 journeys still skips. `window_cutoff` now falls back to the
default 91 days rather than panicking on an unrepresentable declared `window_days` (`Duration::try_seconds` +
`checked_sub_signed`, never the raw `as i64` cast), naming the fallback in the summary when it fires. `now` threads
through `ReportOptions::with_now` (one pinned-clock test added) instead of `evaluate` reading `Utc::now()` inline; the
HEAD-date-vs-wall-clock skew of `occurred_at` stays a named, undone frontier. An unknown `derive:` (`Bound::
unknown_derive`, distinct from "no `derive:` declared") now resolves to `skipped — unknown derive <name> (binary
older than the declared measure)` rather than silently falling through to a plain bound's `passed … 0`. Check 3 split
into two bullets: the window bound (may legitimately read `skipped` under 3 journeys) and the four per-journey
ceilings (still `no fold is skipped, never green`).
