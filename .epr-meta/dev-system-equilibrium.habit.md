---
epr-habit-version: 1
id: dev-system-equilibrium
invariant: >
  Every stock in the development system drains at least as fast as it
  fills — measured as rates over a declared window, never as a level.
  Equilibrium is drain >= inflow per stock; a check that reads green
  because it measured nothing is an over-claim, not a green.
status: red
active: false
checks:
  - "epr flow stocks --window <START..END> --per week --stock commitments --check --root /projects/elohim (exit 0 = every declared stock draining; fail-closed: refusals exit non-zero, never green-on-nothing)"
  - "a2o @concern:dev-system-equilibrium (genesis/a2o/features/devflow/run-plane.feature — 4 scenarios: filling exits non-zero, drain>=inflow exits zero, an unmeasurable window refuses rather than reporting equilibrium, and the @regression guard that a discharge counts as outflow and never as inflow. WIRED 2026-09-11: steps/devflow/run-plane.steps.ts drives them through the real `epr flow` CLI over a throwaway git fixture repo (mint instant = artifact commit date, discharge instant = HEAD commit provenance); runs in the mesh and default profiles, and scoped: `cd genesis/a2o && npx cucumber-js --config '' --require-module tsx --require steps/devflow/epr-cli.guard.ts --require steps/devflow/run-plane.steps.ts features/devflow/run-plane.feature --tags 'not @wip'` → 8 scenarios passed (these 4 + the 4 @concern:run-plane-note write-leg scenarios). The 4 @concern:run-plane-projection emitter scenarios stay @wip (they need scratch registers the run-projection.py emitter cannot yet be pointed at). Where `epr` is absent (the genesis CI image) the @requires:epr-cli guard reports SKIPPED, never a false red — not measured, not green.)"
guard: >
  Regression risks: (1) the outflow classifier keys on `fulfills` — a new
  discharge path that forgets the field silently under-drains (the level
  parity check with `epr flow status` is the tripwire); (2) discharge
  dedup is occurred_at-sorted, never append-order (the C2 channel,
  pinned by the_drain_is_dated_by_occurred_at_never_by_append_order);
  (3) greening this habit by widening the outflow arm instead of by
  actually draining commitments is the over-claim the invariant names.
refs:
  - "genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md"
  - "genesis/docs/superpowers/plans/2026-08-13-agentic-harness-borrows-implementation-plan.md"
  - "genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md — row 7"
  - "displaced: declarative-desired-state → genesis/data/timeline/backlog/declarative-desired-state-parked-habit.md (2026-08-13, operator-directed; returns when its brit/eprfs precondition greens)"
retire-when: >
  when every declared stock's drain>=inflow assertion runs inside the pre-push gate and has
  held a full quarter with no operator override. A system actually in equilibrium does not
  need a weekly reader.
---
DELTA 2026-09-09 (collective memory design; RED preserved): existing unified-memory-loop design revised with governed shared assertions, ephemeral context provenance, local-to-repository graduation, EPRFS integration ownership and migration/retirement conditions. Independent bounded architecture review GREEN; implementation, paired run economics and network enforcement remain unproven. Evidence: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md#2026-09-09--collective-memory-and-progressive-discovery.
DELTA 2026-09-09 (ceremony sprint planned; RED preserved): six delivery stations define purpose-bearing entry, assertion views, linked repair/review, context-reset continuity, governed alternatives and final-revision fresh-agent acceptance. Two seam reviews corrected provider-scope and acceptance-ordering claims; implementation and current native state remain unverified. Plan: genesis/docs/superpowers/plans/2026-09-09-purpose-preserving-memory-ceremony.md.
DELTA 2026-09-09 (Sacred Attention foundation; RED preserved): governed retrieval v2 and narrow household continuity independently accepted;29 recall tests and1919 package checks pass; MAP domain placement adopted with proposed status preserved. Native stale edges147→28 (119 initial reverified), no holds; real conflicts/missing evidence/incomplete reviews and7 older dead citations remain. MemPalace fresh. Full purpose-guided ceremony frontend and total attention/token efficiency remain unproven. Evidence: genesis/data/timeline/chronicle/2026-09-09-sacred-attention-foundation.md.
DELTA 2026-09-09 (repeat memory ceremony; RED preserved): 3/3 remaining witnessed corrections reconciled with exact evidence; independent diff and fresh readers GREEN, including category membership. Fixed recall source bytes 18,900→19,890; broad signing search exceeded packet budget, total token efficiency unproven. MAP navigation repaired without claiming domain currency; native doorway acceptance remains unestablished/revalidation-required and 144 stale edges remain. Chronicle: genesis/data/timeline/chronicle/2026-09-09-substrate-currency-repeat-reconciliation.md.
DELTA 2026-09-09 (reconciliation and recall rails; RED preserved): exact-CID intake closes 4 corrections with current evidence and retains 3; 26 focused tests passed, independent reader recovered current authority from 18,900 source bytes, native projection preserved all six assertions with 4,262 output bytes. Approved roadmap correction adopted; ceremony permits successive useful batches and in-flight classification on existing sources. Evidence: genesis/data/timeline/chronicle/2026-09-09-reconciliation-recall-rails.md. Sustained stock drain, total token savings, and classification retrieval efficacy remain unproven.
DELTA 2026-09-09 (bounded memory ceremony; RED preserved): ten forensic note bodies and all 76 feedback files preserved; generated head 25,958→23,772B and unloaded 9→0; diff review GREEN, fresh-reader binary ambiguity repaired, semantic refresh fresh and same-query first hit now current evidence routing. Reader file bytes increased, so token savings and sustained stasis remain unproven. Roadmap adoption pending; chronicle genesis/data/timeline/chronicle/2026-09-09-substrate-currency-evidence-recall.md names unresolved corrections and intake defects.
DELTA 2026-09-09 (acceptance-aware reconciliation implemented; RED preserved): final eprfs, genesis-a2o and gherkin gates EXIT=0; 19 new native integration tests and 3 executable devflow scenarios passed; 1851 package checks passed. Independent technical verdict approved; appointed experiential verdict is a separate record linked from genesis/docs/superpowers/plans/acceptance-aware-reconciliation/task-4-report.md and acceptance-report.json. Bounded doorway scope readout retains six acceptance-unestablished assertions, including one with production and technical approval; correction recorded for ceremony intake. Local accountable appointment is not peer authority, and this delivery is not a measured stock-drain rate.
DELTA 2026-09-09 (native process grounding; RED preserved): source review plus successful `epr flow context`/`status` probes identified unfiltered open intents beside admitted fulfillment, historical-discharge filtering, and implicit chronology/context-selection rules; evidence and bounded reconciliation/search/algorithm iteration targets are in genesis/docs/analysis/2026-09-09-native-process-reconciliation-grounding.md. This is diagnostic evidence, not a measured improvement in stock drain or a feature completion.
DELTA 2026-09-05 (concurrent attribution station 2 verified; supersedes review HOLD below, no status flip): just gate elohim-epr EXIT=0, just gate eprfs EXIT=0, sidecar-disabled tests EXIT=0; operator-selected gpt-5.6-sol independently approved with no actionable findings and recorded its native verdict. Station 2 fulfilled against genesis/docs/superpowers/plans/concurrent-agent-attribution/task-2-report.md. Native concurrent history and claim idempotence proven; peer witnessing and real harness lifecycle bindings remain later work. No commits or pushes. One discharged commitment, not an equilibrium rate.
DELTA 2026-09-05 (concurrent attribution station 2, review HOLD; no status flip): cooperating native actor/flow authors coordinate create/read/check/append; interrupted evidence is preserved, attribution stays nonblocking, and explicit unlock fixes inherited-descriptor lifetime. just gate elohim-epr EXIT=0; just gate eprfs EXIT=0; sidecar-disabled tests EXIT=0; berth released. Independent reviewer unavailable at capacity; alternate Claude dispatch denied before launch pending disclosure consent. Commitment undischarged. Evidence: genesis/docs/superpowers/plans/concurrent-agent-attribution/task-2-report.md. Passing implementation checks, not an equilibrium rate or witnessed cross-harness completion.
DELTA 2026-09-05 (concurrent attribution station 1 verified; supersedes the HOLD below, no status flip): cargo berth acquired/released; just gate eprfs EXIT=0 (fmt, workspace clippy and tests); independent review approved and the existing commitment fulfilled. Exact claim references now survive governance, claim, note and report authoring without new caller steps. Plan expanded to six stations with cold-start valueflow discovery and verifiable care/accountability after failure; concurrent persistence and real harness integration remain unclaimed. Evidence: genesis/docs/superpowers/plans/concurrent-agent-attribution/task-1-report.md. One completed implementation event, not an equilibrium rate.
DELTA 2026-09-05 (concurrent attribution station 1, HOLD; no status flip): spec and five-station plan authored against existing EPR/REA primitives; exact ActorClaim references retained by governance and flow verbs with no new caller ritual. rustfmt and scoped diff checks EXIT=0; just gate eprfs NOT RUN because the live cargo-berth holder refused the prerequisite claim (EXIT=3). Integration tests authored but unexecuted; commitment remains undischarged. Evidence: genesis/docs/superpowers/plans/concurrent-agent-attribution/task-1-report.md. An implementation event, not evidence of equilibrium or concurrent runtime safety.
DELTA 2026-09-05 (valueflow authoring surface closed through its own verbs; no status
flip): the plan's 11 tasks were claimed and fulfilled with `epr flow claim` / `fulfill --on`,
lifting commitments outflow 0.429 -> 2.000/day on the 2026-08-29..09-05 window (level 616,
inflow 5.571/day, still FILLING). Task-level fulfilment is now a real drain path; the rate
is still net positive, so RED stands. An event, not a rate.
DELTA 2026-09-05 (first task-level fulfilments via epr flow claim/fulfill;
event, not a rate; no status flip): valueflow-authoring Task 11 dogfooded
the new verbs against the Holochain Evolution Epic MVP plan — decompose
re-ran (20 gap-items), then three landed epic tasks were claimed and
fulfilled as agent:implementer@claude-opus-5 (Task 16 roster check,
825a090df + fix 4425bb6fb; Task 17 constitution_root, 10cb3dc00; Task 18
export_held_records, 4fe69b918); no tool:decompose-claim commitment held
any of the three intents, so --supersede was never exercised. Stock
reading over 2026-08-29..2026-09-05 --per day --stock commitments,
identical window/flags before and after: BEFORE level 614, inflow
3.286/day, outflow 0.000/day, FILLING; AFTER level 616, inflow 4.000/day,
outflow 0.429/day, FILLING. Outflow moved (0.000 -> 0.429/day, 3 consume
events newly witnessed in the window) — this is the event this habit
exists to witness becoming visible, not evidence of a sustained rate:
three fulfilments in one authoring session, not a drain the stock can
count on. Habit remains RED; equilibrium is unproven and the check stays
FILLING.
DELTA 2026-08-16 (developer CLI drain; no status flip): the committed
command stock fell 362→322 while its public surface converged to eight
root verbs; 32 local gates now have one manifest-declared detector and
executor, eliminating the pre-push two-map inflow. This is a local tooling
outflow, not evidence that the commitments stock itself is draining;
habit remains RED on its existing measured rate.
DELTA 2026-08-14 (leg 2 close, run #1349 banked): OUTFLOW MOVED A SECOND
TIME — 2.000/wk -> 3.000/wk on a genuinely NEW consume event (level
559->558, consumed 19->20, fulfill ledger 'fulfilled (new): 1'), not the
re-discharge that held the reading flat at #1348. Sequence since birth:
0.000/wk -> 2.000 (#1345, first ever) -> 3.000 (#1349). That is the
event-becoming-a-rate this habit exists to witness. Verdict stays
FILLING and the habit stays RED — inflow 23.000/wk against 3.000/wk
drain is still a filling stock, and the fail-open defect below is
unfixed in epr itself.
DELTA 2026-08-14 (leg 2): the check FAILS OPEN and its green cannot be
trusted yet — REPRODUCED: with git unreadable (the dubious-ownership
state a fresh container starts in), `epr flow stocks` reads inflow 0.0
while level+outflow survive, so outflow >= inflow holds trivially and
the verdict reads DRAINING. It asserts "drained" precisely when blind,
and did so in the wild (the 17:05Z SessionStart fold cached draining for
a stock filling at 23/wk). Control vs HOME=/nonexistent, identical
window/stock/binary and byte-identical flows.jsonl (2502991): inflow
23.0/filling vs 0.0/draining. This — not the window definition — is
what disqualifies equilibrium as a stasis TERMINATION criterion, since
this invariant's own words ("a check that reads green because it
measured nothing is an over-claim") name exactly this failure.
Mitigated fail-closed in run-projection.py (git_readable gate, honest
absence); the typed refusal still owed in stocks.rs, so any direct
caller of `epr flow stocks --check` remains exposed —
backlog/equilibrium-inflow-fails-open-to-false-draining.md. Outflow did
NOT move a second time at banked run #1347: still 2.000/wk from the same
two events (ch04's recovery was a re-discharge of an already-drained
commitment, correctly not counted as outflow). CHECK=1, FILLING.
DELTA 2026-08-14: FIRST OUTFLOW — banked run #1345 (validate-only,
quiesce-gated) fulfilled 2 commitments (ch11 pull-queue-retires
first-ever green + 1); 2026-08-08..2026-08-15 --per week reads
outflow 2.000/wk (was 0.000 since birth), level 561->559, verdict
still FILLING (inflow 23.000/wk, net +21), CHECK=1 — the drain arm
is witnessed live, the equilibrium target stands.
DELTA 2026-08-13: T8 close-the-loop — a2o scenarios landed
(@concern:run-plane-note, @concern:run-plane-projection,
@concern:dev-system-equilibrium; step-def wiring NOT written — all 11
scenarios dry-run undefined and the feature carries @wip, so the story
is authored and blind-read but not yet executing), all tree gates green
(a2o LINT/FMT/TSC/GHERKIN/UNIT=0, 180 unit tests; eprfs workspace
FMT/CLIPPY/TEST=0), final CHECK=1 (FILLING at 22.000/wk inflow vs
0.000/wk outflow, level 560, unchanged from the T2 red).
RED WRITTEN 2026-08-13 (T2, commit cae50c1, the unwired->red first_move
completed same day as admission): live reading over
2026-08-06..2026-08-13 --per week: commitments stock level 560,
inflow 22.000/wk, outflow 0.000/wk, net +22.000/wk, verdict FILLING,
CHECK=1. Level independently agrees with `epr flow status`
unfulfilled (total): 560 — same fulfills-keyed predicate, two readers.
Turnover NaN (honest absence, not +inf). Outflow arm v1 = fulfillment
discharge only (Dismiss is a regression marker here, NOT a dismissal
— counting it would double-drain; printed on every render). Rides
Stock{level,inflow,outflow} + MeasureKind::Rate{per} (measure-family
rows 12-14); 28 tests; fail-closed on MismatchedPeriods / empty
window / NaN rate.

DELTA 2026-09-09 (purpose-preserving ceremony, no status flip): first live reader preserved uncertainty but native context exceeded the view budget; paging correction is source-reviewed with 46 Python tests and 1,919 package checks green, final native build/experiential acceptance pending shared Cargo lease. One roadmap acceptance-state concern corrected and independently source/coherence reviewed GREEN; exact closure and observed rework are recorded in genesis/data/timeline/chronicle/2026-09-09-substrate-currency-purpose-preserving-ceremony.md. This is an observed repair and incomplete journey, not a measured equilibrium rate.

DELTA 2026-09-09 (corrected ceremony completion, RED preserved): final native gate 341 tests, integrated gate 51 Python tests and 5 scenarios/22 steps GREEN; appointed fresh reset-to-completion acceptance approved at bafyreihf4cg4k4gokeaslhp7qnvxzd5zboxrvdllhsixw4h7g2ysk7qal4 after preserved changes-requested evidence and targeted repairs. Exact contract dependency repaired/reviewed, runtime neighbor remains stale. Final finish reused pertinent evidence with zero source reread versus prior 9,220B; total economy and population coherence unproven. Operator-authorized Cargo capacity 2 with incumbent-preserving transitions and live guards. Evidence: genesis/data/timeline/chronicle/2026-09-09-substrate-currency-purpose-preserving-ceremony.md.

DELTA 2026-09-09 (collective memory integration, RED preserved): repository-local collective declaration, governed contribution/projection/feedback and paired burden lenses integrated; final native 353 tests, recall 60 tests, measurement 9 tests, 9 scenarios/39 steps and 1,919 package checks GREEN. Two isolated readers preserved contested standing; explicit method-adoption reset reused three valid receipts with zero source-passage rereads and exposed/fixed shared remember. Sampled concern cohort grew +8 files/+28,880B/+147 lines; no net cleanup or token-savings claim. Native graduation attempt stopped at missing review; tested reach enforcement is scenario evidence only. Evidence and remaining migration: genesis/docs/superpowers/plans/collective-memory-integration/task-3-report.md. No equilibrium rate proof or status flip.

DELTA 2026-09-10 (collective memory task 3 closed; provenance plan planted; RED preserved): second-seat review of task-3-report.md at head 2cea494ee returned changes-requested (verdict bafyreigqm7zcuicu56jmkwnqxorq4ae5juxnekmysxoglsail5rlwtn7lq); re-ran measurement 9 tests, recall 60 tests and collective-memory 4 scenarios/17 steps GREEN against frozen binary a354c33a…; package fixtures for memory-kit and memory-ceremony carried stale embedded descriptions (1913/6) and are re-verified 1919/0 after a package-first sync; memkit-retention --status now reports HELD instead of advertising a prune --apply refuses. Follow-on plan genesis/docs/superpowers/plans/2026-09-10-agent-provenance-and-collective-affiliations.md decomposed to 4 OPEN gaps (signed persona claims + did:key; identity chain of layer heads; plural affiliations replacing the steward field, child collectives, locality rename, distinct-steward graduate gate; held crossing story). Stories genesis/a2o/features/devflow/agent-provenance.feature (@wip, 21 scenarios) and collective-crossing.feature (@requires:household-nodes @wip @design, 7 scenarios) blind-read READY after three and two fresh-reader rounds; gherkin gate parsed 211 features. Actor log measured 9 model spellings over ~4 families across 21 role@model strings, so the hard layer is a controlled vocabulary. No implementation, no rate proof, no status flip.

DELTA 2026-09-10 (memory-kit replacement stations 1+2 APPROVED; RED preserved): every kit threshold is a declared bound (34 measures, 31 lens/ceiling rows with provenance, binding-local, compare direction) evaluated by native `epr flow report` into three-valued folds, plural recipes by CID with a declared default, drift hooks append structured observations, accumulated counts derive from folds (distinct subjects since reset, zero-value heals honoured), SessionStart headline fully native with no fallback; kit accumulators backfilled (90 folds, 163 dead paths exposed: kit cleanup 250 vs native 96/94 over real files). Placement ledger native and byte-identical to the kit on 709 files, checkbox stations minted natively from plans, scope report item-for-item with the kit plus next-action pointer, gap-items relocated to a tracked pre-image (193 files) with nested sidecars still ignored. Evidence: native gate 545 tests/45 suites EXIT=0 (binary 97af15e9…), hooks 29, package verifier 1998 (audits absorbed, 2 kit scripts deleted), 9 independent review rounds (verdicts on gaps #1 and #2). Kit remains the producer until station six; no rate proof, no status flip. Reports: genesis/docs/superpowers/plans/memory-kit-replacement/.

DELTA 2026-09-11 (memory-kit replacement COMPLETE, all 7 gaps approved; RED preserved): `.claude/scripts/memory-kit/` and `.claude/memory-kit/` are deleted (10 scripts, 174 report-tier files, 42 dated dirs); 39 files relocated (3 horizon scans → genesis/docs/analysis/horizon-scans, 24 balance sheets → .eprfs/status/balance, 7 lenses → .epr-meta/elohim/lenses/{gospel,delivery,prior-art} with declared measure rows); native parity fold `epr flow report parity` 53/53 passed, zero gating references; SessionStart headline five native slots, bridge empty; recall executor, cite writer, placement/scope/gap/stasis renderings, footprint lens, retraction path all native; gospel projections carry no kit command. Evidence: native gate 691 tests / 54 suites EXIT=0 (binary e30ec71c…), hooks 34, package verifier 1972 (88 packages), both ceremony profiles green, 18 independent review rounds across 7 gaps. Pre-registered fresh-start observation: two isolated after-readers reached authority 5/5 with 0 and 1 mistaken assertions (baseline before-reader failed to locate authority); tooling-scoped bytes 18,178 and 24,524 vs baselines 42,149/58,162, but honest totals with direct reads outside source_roots 85.2 KB and 49.1 KB; the byte leg of the conjunctive rule fails for one reader, so no flip. Defects filed for the follow-on: recall search discovery exhausts its budget at repository scope; .claude/epr-meta and gap-items sit outside source_roots; no native verb addresses a gap id; ~a quarter of attempts unmetered. Reports: genesis/docs/superpowers/plans/memory-kit-replacement/.
DELTA 2026-09-11 (a2o check WIRED; RED preserved): run-plane.feature's 4 equilibrium scenarios + 4 write-leg scenarios now execute against the real `epr flow` CLI (steps/devflow/run-plane.steps.ts, throwaway git fixture per scenario; mutation-tested — deleting the discharge flips the drain scenario to EXIT=1) — 8/8 passed, verified independently by the orchestrating session; the shared @requires:epr-cli guard (steps/devflow/epr-cli.guard.ts) turns the genesis-CI `spawnSync epr ENOENT` class into SKIPPED for every devflow feature carrying the tag. Live reading 2026-09-05..2026-09-12: level 1003, inflow 177/wk, outflow 66/wk, FILLING, CHECK=1 — the stock is still filling, so RED stands on the measure, not on the apparatus.
