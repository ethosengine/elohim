---
title: "Retrospective — the 2026-10-12..18 marathon: 55 granted hours, 260 commits, and a delivery ledger that read 0 of 0"
id: marathon-retrospective-20261018
date: 2026-10-18
revised: 2026-10-19
status: complete
author: "claude-fable-5-1 (controller of the 10-18 close), with read-only evidence from claude-haiku (sprint-doc and CI readers) and claude-sonnet (ci-investigator); for the operator, human:matthew"
habits: [dev-system-equilibrium, push-delivers-within-budget, dataplane-convergence, blob-durability]
cites:
  - "autonomous-15h-sprint-20261018 | Autonomous sprint 2026-10-18 | sha256:2de6651e4b512861 | path: genesis/docs/superpowers/sprints/2026-10-18-autonomous-15h-sprint.md"
  - "idle-cpu-residual-sprint-20261011 | Idle-CPU residual sprint 2026-10-11 | sha256:d079e41a35954c3f | path: genesis/docs/superpowers/sprints/2026-10-11-idle-cpu-residual-sprint.md"
  - "idle-call-floor-sprint-20261012 | Idle call floor sprint 2026-10-12 | sha256:a33864f5354b676d | path: genesis/docs/superpowers/sprints/2026-10-12-idle-call-floor-sprint.md"
  - "commons-spaces-probe-a-report | 2026-10-16-commons-spaces-probe-a-report | sha256:6cfcd8332e25a77d | path: genesis/docs/superpowers/sprints/2026-10-16-commons-spaces-probe-a-report.md"
  - genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - elohim/elohim-storage/.epr-meta/blob-durability.habit.md
  - genesis/data/timeline/backlog/conductor-startup-lifecycle-ownership.md
  - genesis/data/timeline/backlog/arch-device-recognition-backlog.md
  - genesis/data/timeline/backlog/steward-key-chain-fork-warrant-exile.md
  - genesis/data/timeline/backlog/doorway-ssr-late-adoption-tick-silent.md
  - genesis/data/timeline/backlog/mesh-prologue-cast-and-env-gaps.md
  - genesis/a2o/features/resilience/chain-fork-lost-tail.feature
---

# Retrospective: the 2026-10-12..18 marathon

An agile-style retro over the whole granted arc: the shem workspace's ~40 h orchestrated sprint
(2026-10-12 to 2026-10-17 22:43Z, 242 commits) and this workspace's 15 h autonomous close
(2026-10-18 02:38Z to 16:53Z, 18 commits), merged on one branch. The operator was away for all
of it. Measurements are from the tree (`63e4acbfc..e11669f9b`), `epr flow stocks`, Jenkins, and
Prometheus; every number names its source so it can be re-read.

## 1. The frame

| | shem workspace | this workspace |
|---|---|---|
| Window | 2026-10-12 → 10-17 22:43Z (last commit 15f4e9bcf) | 2026-10-18 02:38Z → 16:53Z |
| Grant | orchestrated sprint, operator away | "15 hours no interruptions", push + fleet granted |
| Commits | 242 (16 merges) | 18 (incl. the merge of shem's branch) |
| Record of itself | none on dev, but not lost: an 834-line journal (cycles C1–C16+) at `.worktrees/marathon-1016/genesis/local-dev/marathon-2026-10-16/STATE.md`, the charter in the workspace's plans dir, and an unpushed final wave (`land-1017`, 7 commits). *Corrected 2026-10-19; the first version said "none … died with the workspace".* | plan file Journal (hour-by-hour) + the 10-18 sprint record |
| How it ended | **a power loss.** The household's conductors wrote their last log line at 00:37:37.755Z on 10-18, under SQLite `database is locked` contention; the journal's last write is ≈00:32Z; node `shem` went Ready → Unknown between 00:27Z and 01:27Z (Prometheus `kube_node_status_condition`, hourly step) | planned close at h13.25; one wakeup to push, Jenkins still busy, batch 3 left local |

The "crashed with ~15 h to go" the operator reported is now dated: the node lost power just
after 00:37:37Z, two hours after shem's last commit (22:43Z). Shem's workspace came back on
10-18 with its journal, its unlanded wave and a rescue snapshot of the uncommitted main tree;
§10 records what was recovered and landed.

## 2. The numbers

**Throughput** (`git`, `63e4acbfc..e11669f9b`): 243 non-merge commits in 7 days (19 / 23 / 50 /
58 / 32 / 44 / 17 per day), 622 files, +63 578 / −9 597 lines. By type: 90 fix, 55 docs, 40
chore, 19 feat, 14 perf, 7 ci, 6 style, 5 test. 95 test or feature files touched, 21 `.feature`.
By area: genesis/docs 175 files, genesis/a2o 57, elohim-storage 57, genesis/data 49,
elohim-app 45, doorway-service 20.

**Habit register** (`genesis/manifests/habits.yaml` history):

| | start (63e4acbfc) | end (e11669f9b) |
|---|---|---|
| green / red / unwired | 15 / 26 / 5 of 46 | 14 / 28 / 5 of 47 |
| flips in window | 2 cures (hosted-human-lifecycle 10-15, blob-durability 10-18) · 3 regressions (hosted-human-lifecycle 10-15, consuming-apps-serve-never-control 10-15, blob-durability 10-17) · 1 born red (graduates-onto-own-key 10-16) | |
| evidence | 77 dated DELTA lines across 27 of 47 atoms | |

Both cures were re-cures of habits that regressed inside the same window. Net: one habit
went green → red, one was born red.

**Backlog rows** (`genesis/data/timeline/backlog/*.md` frontmatter `status:`): 703 → 718. 15
born, 0 status changes, 0 removed. 632 of 719 read as open under a hand-made rule, because
the directory uses 36 distinct status strings.

**Commitments** (`epr flow stocks --window 2026-10-11..2026-10-18 --per week`):

```
inflow:   0.000 per Week
outflow:  0.000 per Week
verdict:  REFUSED  (no events of this stock's unit fell inside the declared window)
```

Level 1542 (1724 produced − 182 consumed, all history). Per week since 09-06: 218/48, 15/0,
152/65, 0/1, 0/0, 0/0. The last dated commitment event is 2026-09-27. In the whole marathon
range the tree gained **2** `fulfills:` edges and **0** commits touching the flow sidecar or a
`recipes.yaml`.

**CI** (ci-observer + ci-investigator, Jenkins API and console logs):

| build | result | reading |
|---|---|---|
| orchestrator #2090 (batch 1) | UNSTABLE 2h49m | Reconcile Build Graph UNSTABLE, same in #2089 before the push: pre-existing |
| edge #1654 (batch 1, build+deploy) | UNSTABLE 1h49m | Deploy Edge Node - Alpha: 4/7 peer rollouts timed out at 600 s (adam, gertrude, susan, eve: all shem-pinned, pods `Running/Terminating(Ready=False)`), doorway-alpha-b (shem) likewise; matthew/jessica/james rolled on the same image. **Env-red: node shem.** #1650 before the sprint was 7/7 |
| edge #1655 (validate-only) | FAILURE 50m | Dataplane Validation fleet caught-up deadline, as #1652/#1653 before the push: pre-existing env-red |
| DNA #1502 | SUCCESS 1h12m | green |
| edge #1656 (batch 2) | UNSTABLE 1h14m | same four shem peers + doorway-b |
| orchestrator #2091 / edge #1657 (batch 2) | building at close | |
| genesis #1654, #1661 (10-17, before both pushes) | UNSTABLE | "could not register a hosted human on alpha: 503" — conductor zome timeouts on alpha; pre-existing |

No code-red on either batch. Four of the harvester's "new" fingerprints on 10-18 are one
cause: a node nobody had a sensor for.

**Cost** (session headline, `epr flow report --headline`): stage-wallclock 122.5 min against a
hard watermark of 20; delivery-cost 2.23 pipeline-hours against 1; operator-surfaced-pain 2.

## 3. Timeline, UTC

- **10-12 → 10-17** shem: idle-CPU residual (scheduler latch D1 on the fork, rolled via edge
  #1608; D2 not built), idle-call floor ("done-when NOT met", 4 of 5 callers cured), commons
  clone-space probes A and B, hosted-is-a-declared-stage, graduates-onto-own-key, 60-row
  deprecation drain, 26 habit-measurement commits.
- **10-17 22:43** shem's last commit. **10-18 00:27 → 01:27** node `shem` Ready → Unknown.
- **02:06** merge here (d4c33bd7f). **02:38** sprint start. **03:01** §A readiness complete in
  24 min against a 2.5 h estimate (toolchain, fork pair, storage/doorway/wasm rebuilds, preflight all ok).
- **03:09 → 04:27** push batch 1 refused three times: missing user-site `pytest` (wiped `$HOME`),
  a hand-appended line in the projected `MEMORY.md`, then three tree gates from the merged state
  (un-migrated cites, jest counting a stale `.worktrees/`, stale codegen). ≈1.5 h and ≈75 min of
  gate wall-clock.
- **~04:30 → 11:02** session suspended; background completions never re-invoked it. **≈6.5 h lost.**
- **11:36 → 11:42** A/B on 85 hosted cells: pin 43.4 / 25.6 / 43.4 s, fork fix 13.2 / 43.1 s. Same
  distribution → the conductor fork has no measurable effect here; gitlink does not move. Logs
  show storage finds the cells at +10 s and answers ready at +43 s: the window is storage's
  20 s probe tick, mishpat one tick later.
- **11:45** push batch 1 landed (gate 1125 s). **12:46** edge #1654 Deploy Alpha UNSTABLE (shem peers).
- **12:25 → 14:05** presence-recovery seam: Opus implements, Codex SEND-BACK (4 scheduling
  findings), Opus fixes + module move, Codex SEND-BACK (narrower), Opus fixes, Codex
  LAND-WITH-NITS. Proof on the household: windows 10.0 / 6.9 / 9.6 s (was 43 / 26 / 43).
- **15:38** push batch 2 landed (gate 1479 s). **15:48** blob-durability red → green on its own
  tag lane (10 passed, 0 failed), stale-fixture red. **16:53** close; batch 3 (3 commits) left local.

## 4. What went well

- **The household mesh did its job as the proving ground.** Readiness in 24 min; receipt lane
  7/7; a plausible cure (serial conductor startup) was killed by a 50-minute A/B instead of
  being pinned to the fleet; the real cause was read from logs and cured with a measured
  4× reduction. Fleet confirmed, never discovered.
- **Review discipline converted criticism into tests.** Five Codex rounds, two SEND-BACKs,
  four scheduling findings became four schedule tests through the real `ProbeGate`,
  mutation-checked. The seam landed in its own module (`services/presence_recovery.rs`) with
  declared bounds, not magic numbers.
- **Flips carried evidence or did not happen.** No pin move on a zero-effect A/B; blob-durability
  flipped only on its own `@concern` lane with a ruling note; five habits got dated deltas with
  run ids and stayed red.
- **Found defects were fixed in flight, not filed:** the prologue's silent custody skip became a
  named refusal; the convergence habit's broken check line; jest's `.worktrees/` leak; the
  un-migrated cites; the wrong 10-17b delta claim (storage never retried `enable_app`).
- **The plan file survived three compactions** and the close summary was reconstructable from
  its Journal alone.
- **Rails held under autonomy.** Four classifier/hook denials (settings self-edit, merge without
  review, heredoc journal, stash drop), zero retries or dodges, all recorded as ceiling.

## 5. What did not go well

1. **0 of 0, and it would read 0 of 0 for any sprint run the way CLAUDE.md says to run one.**
   The ledger's commitments are two things, and the work is neither. (a) 4 940 gap lines minted
   from 297 spec and plan documents (`.eprfs/status/gap-items/`): 4 214 OPEN, 716 CLAIMED, none
   DONE; 3 636 of them sit in documents whose status is `draft`, and 3 661 were born in May and
   June. No open gap describes the marathon's cure, the custody fixture, or the idle-CPU work.
   (b) one `a2o:scenario-green` commitment per feature file, minted by `epr flow project` and
   discharged by the first green household run — once. Tonight's receipt lane dry-runs as
   "already fulfilled: 1"; the blob-durability lane as "skipped (pending): 4"; 19 of the 21
   feature files the marathon touched were never minted because nothing has projected since
   September (577 dated mints in September, 2 in October). The habit register, which CLAUDE.md
   names as the unit of work, mints nothing; `claim --serves <habit>` has been used 0 times in
   7 092 flow events. So the ledger measures the discharge of spring design-doc gaps plus
   first-time scenario greens, and the agents follow the register. Both controllers stayed on
   goal and the instrument could not see it. This is the finding the rest of the retro hangs from.
2. **The register moved backwards.** 15 → 14 green while 3 habits regressed inside the window
   and 15 backlog rows were born against 0 closed. Discovery outran discharge by 2.3× (commitments,
   4-week), 2.7× (habits, 90-day) and 8× (backlog rows, 90-day).
3. **6.5 of 15 hours were lost to a suspended session** with no fallback wakeup. The rule
   existed in no skill; it existed only after the loss.
4. **Three push refusals before the first landing.** Two were environment (the wiped `$HOME`
   took `pytest`), one was a hand edit of a generated file, and the per-project tree gates were
   only read at the end of a 50-minute run. A local `just gate` plus the parity check would have
   caught all of them in minutes.
5. **Shem's record was not on dev.** *(Corrected 2026-10-19; the first version said shem left
   no record.)* The journal and charter survived the power loss, but in the workspace's
   gitignored campaign dir and config dir. Nobody could read them until the node came back, and
   its final wave (7 commits) and 245 uncommitted files never reached dev. A controller reading
   only the repo concluded "no record", and the retro repeated it. S1 below stands; its reason
   is reach, not loss.
6. **One node, four fingerprints, no sensor.** `shem` hosts four of seven alpha peers, doorway-b
   and a workspace. Its loss showed up as two UNSTABLE deploys, two UNSTABLE genesis runs and
   four "new" CI findings over 16 hours, and nothing said "node shem Unknown".
7. **Implementation started before the measurement returned.** The fork branch
   (`fix/startup-per-app-publication`, 12 commits, 2 reviews) cost ≈3 h of Opus and Codex before
   the A/B showed no effect at this household's size. The plan ruled measure-first; the
   dispatch did not wait for it. The branch stays as a frontier for the 100-cell shape, but the
   order was wrong.
8. **Closure is not countable.** 36 status strings in the backlog; `retire-when:` on every
   habit and one retirement in 90 days.

## 6. Puzzles (open, not actions yet)

- Shem's household saw station-2 windows past 120 s; this one saw 43 s on the pin and 10 s
  after the cure. Store size and ~100 hosted cells are the suspected difference; it cannot be
  re-measured while shem is down.
- ~~Does the "15 h to go" crash equal the node outage?~~ **Resolved 2026-10-19:** yes. The
  operator reported a power loss, and the household's conductor log ends at 00:37:37.755Z (§1).
- Who reads a sprint record? Six were written this month. The ratios above did not move.

## 7. Root causes

- **Mint > discharge is structural, not a mood.** Birth has a gate (the compose gate, frontmatter,
  cites); closure has none. `fix-what-you-find` and the story-maintainer rule oblige recording;
  nothing obliges retiring. `retire-when:` is written and never read.
- **The ledger's unit and the work's unit were never joined.** Commitments come from spec/plan
  gap lines and from feature files; work comes from red habits. The one join that exists
  (`claim --serves <habit-id>`) is optional and unused; the register has no `commitments:` field
  and mints nothing. A sprint that does exactly what the gospel says, moving the top red with
  proof, discharges nothing by construction.
- **Where the join exists it is one-shot and manual.** A scenario-green commitment discharges on
  its first green run and never again, so a mature suite reads 0 outflow forever; mints are
  dated at `epr flow project` time, which is a hand-run verb last run in September, so the 21
  features the marathon wrote or changed had nothing to discharge into.
- **Gap lines in drafts are counted as promises.** 74% of open commitments live in `draft`
  documents and 74% were born in May–June; a draft retiring to `held/` or being superseded does
  not retire its gaps. The stock is a library of spring design notes read as a backlog.
- **Lost time was a missing heartbeat, not a slow agent.** The harness re-invokes on completion
  only while the session is live. A suspended session loses wall-clock silently.
- **Alpha has a single point of infrastructure with no node-level sensor.** Four peers and a
  doorway on one host; the sensors watch pods and stages, so the node's failure arrives as
  four unrelated-looking fingerprints.

## 8. Actions

Owners are named; "me" is the controller of the next granted sprint in this workspace.
Each item has a done-when that can be read without asking anyone.

**Start**

| # | action | owner | done-when |
|---|---|---|---|
| S1 | A granted sprint writes its record in the repo at hour 0 (`genesis/docs/superpowers/sprints/<date>-<name>.md`) and mirrors the Journal into it at close; the plan file is a working copy, never the only copy | every sprint controller; the `agentic-developer`/`delivery-stasis` skill text names it | the next sprint's record exists before its first commit |
| S2 | Join the ledger to the register, in the register's favour: a red habit mints ONE commitment ("<habit> to green with proof"), discharged by its evidence-backed flip and re-opened by a regression (dismiss); deltas become notes on it. Gap lines in `draft` docs demote to intents (only `active`/`approved`/`in-flight` docs mint); a doc moving to `held/` or `superseded` retires its gaps; scenario-green discharges per measured window, not once. Then "x of y" is 14 of 47 and the marathon reads 2 fulfils, 3 dismissals, 1 mint, which is what happened | operator rules the design (it changes what a commitment IS); me: a spec under `genesis/docs/superpowers/specs/` and the `recipes.yaml` change; `epr flow project` runs in the sprint close so the window is dated | `epr flow stocks --window <sprint> --per week` is not REFUSED for the next sprint, and its fulfils equal the register's green flips in that window |
| S3 | A node sensor: `kube_node_status_condition{status="unknown"}==1` sustained 10 min fires one fingerprint naming the node, routed to the runtime ledger | `algedonic-designer` dispatch; operator confirms the reader | the next node loss is one finding, not four |
| S4 | `habits-status.py` prints each habit's `retire-when:` and whether its evidence exists; a quarterly retire pass | me (script); operator + librarian (pass) | at least one habit retires on evidence by 2027-01-18 |
| S5 | Local pre-push dry run: `just gate` on changed projects plus the MEMORY.md parity test before the 20–50 min hook; `pytest` provisioned by an on-demand devfile command | me (script); operator (devfile) | the next sprint's first push is not refused |
| S6 | Normalize the backlog `status:` vocabulary to a declared set and lint it at write time | librarian ceremony | `status` values ≤ 8 distinct strings |

**Stop**

- X1 Dispatching an implementer for a measured red before the measurement returns. The fork branch is the cost.
- X2 Hand-editing projected surfaces (`MEMORY.md`, `habits.yaml`); regenerate.
- X3 Ending a waiting turn without `ScheduleWakeup` (≤1800 s), journal stamped from `date -u`.

**Continue**

- C1 Household-first proof; A/B before any pin move; flips only with run ids.
- C2 Codex review rounds that end in mutation-checked tests, with maintainability in the brief.
- C3 The plan-file Journal with real clock stamps and one line per ruling.

**Operator-only** (unchanged from the sprint record, plus two): node `shem`'s health and the
Terminating pods on it; `git stash drop stash@{0}`; the fork pin decision; the allow-rules
script; three memory entries to claim; the first-device approver rule. ~~`git push origin dev`
for batch 3~~ (done: batch 3 and this document reached dev in the 2d1edf1ee lineage). Four more
from §10: decision D1 (how a forked key returns), Adam's operator key on the household, the
F1/F2 auth follow-ups, and re-triggering CI for b1e42ad82 once Jenkins is back.

## 9. Velocity, for planning the next one

At marathon pace the system cures about two habits a week and regresses about one. With
births frozen that is a 6–14 month horizon for 28 reds; births are not frozen, and should
not be, because the register grows when the system learns where it is blind. So a sprint
is not a step toward "closing the backlog". It is: one or two named reds moved with proof,
one to three commitments fulfilled in the ledger's own unit, and a ratio that does not get
worse. Judged that way, this marathon moved two reds with proof, fulfilled nothing the
ledger could see, and let the ratio slip by one. The next one should be measured before it
starts, on exactly those three lines.

## 10. Addendum, 2026-10-19: the integration pass, wave 6 landed, and what verification found

Written by the controller of the 10-18/19 integration pass on shem's recovered workspace
(claude-opus-5-5, for human:matthew). The operator's sequence was integration pass, then land
wave 6, then verify, then revisit this retro on the result. The campaign journal is
`genesis/local-dev/integration-2026-10-18/STATE.md` (gitignored, on this workspace's PVC).

**Integration pass.** Every stranded branch and worktree was read against dev and given a
verdict:
- **Still needed, next to land:** retired-pin-readmit-backoff; plane-violation-write-guard and
  hosted-provisioning-stop-damage (both need a rebase; the second is security); the SSR phase
  and ux-walk commits on perf/call-floor-2; brit-daily-driver.
- **Parked by ruling:** provide-revoke-arm.
- **Already on dev in another form:** vite-8 and agent-a37d55a.
- **Needs its owner:** devfile-shem, the 245-file rescue branch, and wt-sw-preload.
- **Authz gap found on dev:** eight doorway operator actuators accepted callers below Admin
  (provision, deprovision, force-graduation, the federation actuators, both refreshes). The fix
  branch rode with wave 6, by the operator's choice.

**Landed:** `origin/dev 2d1edf1ee..b1e42ad82`, 14 commits through the full pre-push gate with
no bypass:
- wave 6: the pool-batch-on-empty-primary fix, `excludeStates` on commitments with its SDK sync,
  and habit deltas
- the operator-surface authz guard
- one prologue fix found during verification

The serving receipt was `epr-app-deliverability` at 135/135
(`sprint-report-household-20261019T001729Z-2355e99d`).

**What verification found.**

1. **The power loss forked a chain, and the household exiled james from lamad.**
   - At the cold restart (22:11Z), matthew and jessica authored chain-fork warrants against
     james's agent for one action pair (22:17Z).
   - From then on they dropped all of james's lamad traffic (221 069 of 601 060 messages, every
     one on lamad) and left him out of their lamad peer stores.
   - Deliverability could not pass, and a 30-minute "settle" wait measured nothing.
   - Mechanism, consistent but not yet proven: the conductor commits at SQLite `synchronous=Normal`
     in WAL mode, where a published action can roll back on a power loss but not on a process
     crash.
   - Under the current pin, no admin call lifts the warrant's block, so the operator authorized a
     full re-cast of the household's identities. James's databases were preserved first for
     dating the fork.
   - It is the alpha adam/eve exile class (dataplane-convergence DELTA 10-10a/b) reproduced by an
     honest crash.
   - Recorded in `backlog/steward-key-chain-fork-warrant-exile` (reworked into C1a–C7 plus
     decision D1), with a reproducible drill in `a2o/features/resilience/chain-fork-lost-tail.feature`.
2. **The fresh household exposed three things a long-lived household had hidden.**
   - (a) This landing's operator-binding seeder needs `OPERATOR_STEWARD_STORAGE_URL`, which the
     prologue never passed. Fixed in b1e42ad82.
   - (b) The same seeder now needs a real login for every operator, and the household does not
     cast Adam, the apex primary. A fresh household therefore seeds no bindings at all. That is an
     operator decision, recorded in `mesh-prologue-cast-and-env-gaps`.
   - (c) A doorway that boots before its SSR heads are declared never adopts them: warmup reports
     complete while `servedBundleHeads` stays `[]`. It predates this landing: restarting each
     doorway on the same binary gave failover 41/41. Recorded in
     `backlog/doorway-ssr-late-adoption-tick-silent`.
   - Also surfaced: the 15 h sprint's first-device-no-approver finding, which now fails channel
     isolation and version convergence on any freshly cast household.
3. **CI could not be read.**
   - Jenkins was down at landing time (`jenkins.ethosengine.com` 503), so no pipeline verdict
     exists for b1e42ad82. Its webhook may have been lost.
   - The fleet did not move: doorway-alpha was degraded before the push, with its storage-A
     circuit open after 210 errors and shedding, while elohim.host was healthy.

**What this changes in the reading above.**
- §5.5 and §6 are corrected in place.
- The lesson of §5.6 ("one node, four fingerprints, no sensor") repeats at the household scale.
  An exiled peer reads exactly like a slow one from the storage side: pending grows, stuck sweeps
  climb. The first `/db/p2p/conductor-diagnostics` read would have named it in seconds.
- Under X3, waiting on "churn" without reading blocked-per-DNA counts is the household version
  of the suspended-session loss.
- Add to Start:

| # | action | owner | done-when |
|---|---|---|---|
| S7 | Name exile where it bites: `just mesh status` and preflight print `EXILED <peer> <dna>` from blocked÷received per DNA, and the deliverability pointer step names an exiled peer instead of `<200>` | me (C2b of the exile item; no decision needed) | chain-fork-lost-tail scenario 3 passes |
| S8 | Rule D1 (re-key with lineage, recommended, or operator unblock), so a forked key has a way back other than a re-cast | operator | the ruling is recorded in the exile item and its scenario 8 is rewritten or kept accordingly |
| S9 | Commit authored actions durably before publishing (`synchronous=Full`, or a narrower authoring sync), measured for disk-write cost on the household first | me (measure), operator (pin/config roll) | chain-fork-lost-tail scenario 6 passes |

**Numbers at this revision.** The register reads 14 green / 29 red / 5 unwired of 48. The
operator surface brought one habit, born red; no flips were claimed in this pass. The landing
fulfilled nothing in the ledger's unit, which is §5.1 again. It did discharge the operator's
three named steps, and it turned one stranded wave into dev. Velocity (§9) is unchanged:
nothing in this pass moved a red with proof.

## 11. Addendum, 2026-10-19 (later): the overnight shift, scored on §9's three lines

Written by the controller of the overnight shift `2026-10-19T03-25-finish-integration-2026-10-18` (claude-opus-5-5, for human:matthew). Its objective was to land the stranded branches that §10 judged still needed, each with a fresh household receipt.

**What landed.** Three branches reached dev through the full pre-push gate, with no bypass. Each has a household receipt on its own source:

| branch | dev | gate | receipt |
|---|---|---|---|
| retired-pin readmit backoff | `d37d467e4` | storage tests | deliverability 135/135 |
| SSR self-heal + `c9f9f8ea0` | `c41e075a8` | doorway 1816/0 | deliverability 135/135 |
| plane-violation write guard | `7d0e3089e` | storage 5692/0 | deliverability 135/135, name-routing 25/25, app-delivery-refuses-fast 50/50 |

The household ran the write guard in refuse mode and it refused nothing. CI deployed all three to the alpha fleet (edge #1660, #1662 and #1664), including adam-alpha, which serves elohim.host. Hosted-provisioning stop-damage is gated (1927/0 and 1930/0) but held for the operator: `backlog/hosted-provisioning-stop-damage-held-for-operator`. Every other open branch has a next act and an owner in `backlog/stranded-branches-after-2026-10-18-power-loss`.

**§9's three lines.**
1. **Reds moved with proof: 0.** dataplane-convergence stays red; DELTA 10-19a records the landings and claims no flip. The register reads 14 green / 29 red / 5 unwired of 48, the same as in §10.
2. **Commitments fulfilled in the ledger's unit: 0.** `epr flow stocks --window 2026-10-18..2026-10-20` reports 0 produced and 0 consumed. This is §5.1 again. This time the register agrees with the ledger: there was no flip for S2 to count.
3. **Ratio: worse by two.** Two backlog rows were born (the hosted hold and the stranded-branch table) and none were closed.

**The one red CI kept showing.** Every validate-only run (edge #1661, #1663, #1665) failed on one step: elohim.host never reports caught up, at `divergentAnchor=151` across five hours. Its public `/db/p2p/conductor-diagnostics` names the cause:
- doorway-alpha's conductor has blocked all 4 034 344 messages from adam's conductor.
- Four of adam's six peers have sent it nothing.

That is DELTA 10-10a's chain-fork exile, still in force nine days on. No push cures it, and waiting does not either. It reaches the fleet through D1 (S8). The shift first reported it as prod fleet state and asked the operator to read it. That was wrong: the read is public HTTP, and this shift has since taken it (`backlog/ci-edge-inventory-convergence-caughtup-sawtooth-flap`, SEEN AGAIN 2026-10-19).

**What this means for the next sprint.** The landings removed obstacles; they did not cure anything. The reds nearest a proven flip are blocked by named decisions, not by unlanded code. The exile on alpha (dataplane-convergence and every validate-only run) waits on D1/S8. Fresh-household version convergence waits on the Adam binding (§10, 2b). The ledger will read 0 of 0 for any sprint until S2 joins it to the register. So the next sprint's highest-leverage hour is the operator's rulings on D1, S2 and the Adam binding, not more agent hours.
