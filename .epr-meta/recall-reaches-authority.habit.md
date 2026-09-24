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
status: red
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
DELTA 2026-09-22 (recall — Codex's trail sprint + capstone memory ceremony; NO status change — the
habit stays red): the six observations Codex left on 2026-09-13 (consolidation queue item 25) drove
four stations, landed on dev cd912bd40…dd75c16aa. S1 the entry reaches: declared multi-type globs,
short terms, stemming, a root-scope authority set passed through the declared-scope gate (CLAUDE.md
and .epr-meta/ are now DECLARED source roots, contract v13), a narrowed continuation on a budget cut,
a new passage.rs seam (rarity-weighted word-start scoring; term-density windows for unsectioned
files), proximity re-rank; the deterministic bank went 1/6 → 5/6 (q-hook-binary the named miss: a
filename hit breaks a tie on passage terms). S2 `open --purpose resume --habit <id>`: last delta BY
DATE (atoms are not reliably newest-first — a fresh reader caught the view labelling a 09-11 entry
"last" over 09-12; fixed with a direction-aware tie-break shared with the first screen), commits
since it, an implemented-but-unverified verdict naming the check to rerun. S3 prose that lags its
value: read/source name `key=N` prose contradicting the document's own value (live on
pool-policy.json max_concurrent_heavy 1 vs 2). S4 blind-reader findings carry a class and the loop
rules record rounds. Checks: recall suites 197/62/3/3/25/5/14, clippy clean, a2o ceremony profile
7 scenarios / 40 steps incl. the new resume scenario; `just gate memory-ceremony` native legs green,
the hook leg red ONLY on memory-index idempotence (the index projects past its hard 24,000 B bound on
dev and stranded memory differs per checkout — the ceremony's head-compaction batch). Fresh reader
(context-reset Sonnet, entry-only, judged by opus): resume question 2 screens 14,160 B, pool question
5 screens 14,259 B, 0 unmetered, 0 mistaken — folded on the plan (env
reader=fresh-reader-codex-trail-sonnet). Window: 9 of 14 journeys (32 folds) in 91 days, still failed
at hard 0.2 — it carries this sprint's own five pre-fix baseline misses; the standing reader's weekly
runs decide the flip, not this sprint. Frontier: search does not match an underscored key from its
words; the bank's q-hook-binary; S4 classes reach readers only once dispatch reads this branch's
agent package; the build-receipt artifact chain (Codex obs. 5) held.

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
DELTA 2026-09-12 (station 3 — the standing reader; GREEN preserved on the first rolling-window reading): the six
fresh-start questions are Intents in scope of the recipe (`recall-questions.json`, contract v12); `recall sample`
records a reader's journey as a FlowEvent (Consume; fulfills iff the receipt bytes carry every `reached_when.terms`;
a journey whose entry locates nothing is a MEASURED MISS — screens-to-shape 4, no receipt — never a refusal) and folds
metered/unmetered/screens with `env:journey=<event cid>`; `recall judge` lets a seat of a DIFFERENT tier record a
Verdict as a `verdict` note on the contract and fold `recall-mistaken-assertions@1` (a reader never judges its own
journey; every refusal keeps the `--json` envelope). Check 3 now reads `recall-journey-window-ceiling@1`
(`derive: rate-over-window`, journeys grouped by the journey slot, 91 days, hard 0.2, `skipped` under 3 journeys,
an unknown derive skips rather than passing 0). The weekly routine `.claude/workflows/recall-standing-reader.js`
samples every tier the actor sidecar saw in 30 days × the six questions and judges each by a rotated tier
(routine: operator /schedule pending). Dry-run pair: haiku-labelled sample of q-corrections reached
(memory-ceremony/SKILL.md:133-159, lens minimal, 2,152 metered B, 0 unmetered) → sonnet judge Verdict refuse,
mistaken 1 — an omission counted as a mistake against an account the controller wrote; the judge brief was corrected
the same day to count contradicted claims only, and the provenance is an observation on the contract. What the
instrument found first: under the deterministic first-candidate journey 4 of 6 bank questions locate no candidate
(`question_terms` drops "top"/"red"; no area stemming) and q-remine lands on converge/SKILL.md — recorded on the
question bank as the station-4 frontier (short register vocabulary, area stemming, reader-chosen candidate).
Intermediate reading FAILED at 1 of 5 journeys (exactly the hard 0.2 at-or-above); final reading this station:
1 of 7 journeys (9 folds) within hard 0.2; latest journey mistaken 0, screens-to-shape 4 (soft warn 3, hard 6);
unmetered 0. Thin window, honestly: three of the seven are pre-3.2 hand folds and three are deterministic misses of
one question — the first real weekly run decides. Frontiers: `finish` refuses a zero-receipt focused journey
(journey.rs); fold `occurred_at` is HEAD-dated while `now` is wall-clock; `Contract::question_bank()` ignores `--root`.
DELTA 2026-09-12 (final review round; GREEN preserved): the whole-branch review found the habit's own gate not running
this sprint's four test binaries (golden, lens, sample, report) — `just gate memory-ceremony` now runs them and the
manifest watches their sources; the tier gate closed `--force`/`--recursive` long flags, `submodule foreach`, and an
ABSENT policy row (deny, not skip); and a measured miss no longer folds as a clean journey: `recall-not-reached@1`
(1 when the FlowEvent fulfills nothing) is folded on every journey and consumed by the window bound, so the next
real weekly run reads the discovery gap the standing reader found (4 of 6 questions) instead of diluting it. Fresh
tree readings after the round: window 1 of 7 journeys within hard 0.2 (the new middot has no folds yet — the first
run with it decides), python 64/64, rust 147/0, goldens byte-identical.
RED 2026-09-12 (GREEN → RED on evidence, the first rolling-window reading on the shared checkout): check 3's window bound
reads `failed — 4 of 6 journeys (14 folds) in the last 91 days — has reached the hard watermark 0.2` against main's own
fold history: reader 1 and reader 2 (pre-fix, unmetered ~100 KB / mistaken 1), the haiku bootstrap sample (mistaken 1: it
chose the headline's bare `hold --scope --apply`), one more positive; readers 3 and 5 reached clean. The latest-journey
checks were green because each read only the newest fold; the window reads the quarter, and the quarter says most
journeys missed. This is the instrument doing its job, not a regression in the code that landed today. The register flips
because the covenant flips on evidence, never intention. Two artifacts to name, neither a reason to widen the bound:
the two bootstrap samples share the hand-written journey label `bootstrap-sample` and group as one journey; the pre-3.2
hand folds carry no `recall-not-reached@1`. Green again when the standing reader's real weekly runs (with the discovery
gap in station 4 closed: short register vocabulary, area stemming, a reader-chosen candidate) bring the 91-day rate
under 0.2 — and not before.
DELTA 2026-09-12 (index-as-measure vocabulary; NO status change — the habit stays red). Landed `elohim/epr-rea/src/index.rs`: `IndexMeasure` (a middot measure whose fold is a searchable shard: PinnedRef + chunk-rule CID + optional ModelPin + RankingMethod + ReachBound + SurfaceRule + Retention + fold-lag Bound), `FoldAttestation` (a custodian proves its own fold: Complete | Degraded | Failed), `ShardManifest`. Three invariants by construction: the private chain (AttentionTending) never enters a fold; Retention has no delete variant; is_complete() only for Complete. A ring BAND was drafted and withdrawn under the requisite-variety guidestar §3a (one framework = hold); ReachBound is one bound with a Sense. Gate `elohim-epr` green (fmt, clippy, 69 epr-rea unit tests incl. 7 new). Zero new DHT entry types. This is the vocabulary the recall executor's semantic route needs to print a method CID on every candidate (the contract's mempalace route is ranking null); the providers behind it are stations 4+ of the three-seams spec (`genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md`), unscheduled until station 3 lands.

DELTA 2026-09-24 (governed discovery station 4 at L0 — the native semantic route; NO status change — the habit stays red on its window). Landed on dev 840120a86…285effca2 (plan `genesis/docs/superpowers/plans/2026-09-23-governed-discovery-station-4-native-semantic-provider-plan.md`; executor Opus 5.5 per task, controller Fable 5.1/Opus 5.5 on design fidelity and alignment review, one review + at most one fix round per task, rulings in the plan ledger). The contract's `semantic_provider` reads `semantic` (v15→v22 across the station; the bank re-pinned each time); the model is a Manifest EPR pinned by CID (`all-MiniLM-L6-v2`, Apache-2.0, 384 dims) with the embedding procedure pinned beside it and refused on any byte mismatch; the fold is a SQLite store declared as `recall-semantic-index@1` over the git-tracked authority layer (2,764 files as gitignore-style globs — a whole-tree fold made the 25-file lag bound unreachable; ruled at the real run), incremental, append-only attested with the session's participant ref, demotion never deletion, private chain refused by code; the `Semantic` provider prints its measure and model CID on every candidate and labels a stale fold; the focused first screen fuses local + semantic ranks under `rrf-v1` (recipe CID on the floor line, standing never summed, a fused candidate's passage follows the ranking producer); the FTS5/BM25 `lexical` provider is declared and reachable but OUT of the recipe by its own bank evidence (q-top-red 1→2 with it fused; habits-status.py outranks CLAUDE.md); the `index:` fold-lag headline replaced the `mempalace:` mine gate and the surfacing hook reads the native provider, spawning one detached fold when behind. Evidence: deterministic bank via `recall sample` 6 of 6 reached on the complete live fold (36,085 chunks, ~56 min release; baseline 5/6; semantic-only 5/6, lexical-absent 5/6 — the routes miss different questions, fusion closes it); palace removal test PASSED (every rank unchanged with `providers.mempalace` deleted); `recall-bank-reach@1` folded 1.0 with `env model=<cid>`, the first fitness evidence on the model artifact; the freshness loop observed end to end three times today (other sessions' commits → red at 70/63/64 behind → one attested fold → green). Fresh reader (context-reset haiku, entry-only, a question in none of the authority's words, whole-tree door): NOT reached, 3 mistaken (landed on CLAUDE.md's mesh fork-pin passage) — the window bound reads 10 of 21 journeys, red, and this reading belongs in it. Gates: `just gate elohim-epr` green; `just gate eprfs` red only on `flow_cites::live_corpus_migrate_reports_zero_pending` (another session's untracked memory files without `id:`), every recall suite green (recall 64, golden 3, lens 25, sample 14, report 108, index 10, fold/semantic/fusion/lexical/embedder green in the crate run); `just gate memory-ceremony` red only on the hook unittest `memory_index_projection_test` (another session's hand-added MEMORY.md lines), native legs green, a2o ceremony-reconciliation 7/7 and collective-memory 4/4 green (the only reds are environmental — another session's unslugged memory file, unsealed cited doc and held-feature moves). Frontiers: a free-form journey mints no judge-able event (only `sample` does) so the fresh reader's ruling could not fold — station 5's human lens should give every finished journey one; the live test is red on a machine with the model cache but no onnxruntime; the fold's `!` negation is any-negation-excludes; the register and the question bank are refused on first screens by rule and owed as surface negations in the next measure version; L2 (sqlite-vec, the storage-service mirror, a native runtime) is consolidation-queue item 26.

CORRECTION 2026-09-24 (appended after the whole-branch review; the delta above is left as written). Three claims in that delta were stale or incomplete. (1) The window bound read 10 of 21 journeys when the delta was drafted; at the close it read 12 of 39, and 25 of the added journeys were this station's own deterministic `sample` runs — self-runs the habit's guard (3) says must not green it. The window now counts deterministic sample journeys once per (question, method CID): reading 10 of 26 journeys (56 folds), 0.38 against hard 0.2 — still red, honestly. (2) The sixth reach (q-hook-binary) needed a change to `sample`'s reached check — identifier words, word-start, in order — under an unchanged method CID; the rule is now declared in the contract (`evaluation.reached_rule`, v23, bafkreic7rfsr4i2stqsimbssn5soxegr6dneptsyjbjfxxypmrlujqozbu), so the ruler is part of the method from here. Fusion put the right passage in front of the reader; the declared ruler is what recognises it. (3) "lexical-absent 5/6" should read "semantic-absent 5/6"; "every recall suite green" mixed the close-time memory-ceremony run with the earlier private-target run of the station's own suites — those six suites now run inside `just gate memory-ceremony`, which at the close of this correction reads red only on the environmental hook unittest memory_index_projection_test, the six station suites green inside it. Also landed in the same wave: the bank and the register are withheld from `search` and the surfacing hook (not only first screens); the fold now carries a truncation tally that reads unknown until the counting procedure ships (the tested patch moves the procedure pin, which refuses the live store and forces the full re-fold, so it rides with the next measure version); the live embedder test asserts the specific `unavailable` reason instead of panicking when the palace's Python runtime is absent; the answer names its embedding procedure. Named follow-ups: the native route owns its model directory (the palace's cache is today's fallback, so the palace removal test holds at the declaration level); the next measure version takes the surface negations and the 256-token chunk size in one re-fold.
