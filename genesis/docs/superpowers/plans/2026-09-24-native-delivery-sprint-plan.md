---
title: Native delivery sprint — reconcile the tree, split the App-delivery stage by concern, declare run classes, price the pipeline, grow the subconscious, and lay the brit/rakia rails
id: native-delivery-sprint-plan
status: Draft
class: process-meta
process_subdomain: ci
sprint: native-delivery-2026-09-24
serves: [push-delivers-within-budget, dataplane-convergence, pin-attestation, idle-is-free]
cites:
  - "evidence-ladder-push-left | the evidence ladder spec; its §8 prices the App-delivery incident (#1719–#1725) this plan drains | sha256:c8bdfe8ba6c28790 | path: genesis/docs/superpowers/specs/2026-08-10-evidence-ladder-push-left-design.md"
  - "algedonic-slice1-delivery-flow | algedonic slice-1 plan: concern-addressed findings this plan composes with (Lane D4), never forks | sha256:66054e651d33f3a4 | path: genesis/docs/superpowers/plans/2026-08-10-algedonic-slice1-delivery-flow-plan.md"
  - "algedonic-feedback-signal | algedonic feedback-signal spec; its §4b is canon for the Lane E sensor persona (honest absence over a guessed address) | sha256:d0b1b524dc7240fc | path: genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md"
  - "ci-detection-convergence-epr-meta-fold-plan | ci-detection convergence plan; its ci-trigger leg (Task 4) is the home Lane B's run classes extend | sha256:bef753c07ef436b5 | path: genesis/docs/superpowers/plans/2026-06-25-ci-detection-convergence-epr-meta-fold-plan.md"
  - "epr-app-deliverability-verdict-slice1-plan | EPR-app deliverability verdict (landed): the served-shell verdict Lane A's verify phases and Lane N2 reuse | sha256:607e73a127d15e4e | path: genesis/docs/superpowers/plans/2026-09-05-epr-app-deliverability-verdict-slice1-plan.md"
  - "submodule-pin-attestation-gate-design | pin-attestation gate spec: the rakia schema path, attested run kind and oracle Lanes B1/N1/F follow | sha256:79d9af05d5287df8 | path: genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md"
  - "runtime-artifacts-elected-content | elected-content spec; its §12.8 slice 2 is Lane N (app bundle as elected content) | sha256:eaa2716381075140 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
  - "workspace-berth-carrying-capacity-design | berth carrying-capacity spec: the workspace identity and lease discipline Lane F2's receipt attestation signs with | sha256:589932493289a667 | path: genesis/docs/superpowers/specs/2026-09-03-workspace-berth-carrying-capacity-design.md"
  - elohim/rakia/docs/specs/2026-04-12-rakia-design.md
  - genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - genesis/data/timeline/backlog/fleet-standing-celldisabled-one-third-party-cell.md
informed-by: [genesis/orchestrator/README.md, /CLAUDE.md]
derived_from: [runtime-upgrade-propagation, rakia-reach-as-promotion, eprfs-witnessed-interaction]
---

# Native delivery sprint

## Context

**The incident.** App builds #1719–#1725 spent ≈12 pipeline-hours and delivered nothing. The
"Publish and Verify App Delivery" stage waits up to 7200 s (`STAGE_CELL_READY_BUDGET_SECS`,
a2d1d0975) for the edge fleet's post-roll not-ready window, measured from inside the App
pipeline, because the App manifest `dependsOn: elohim-edge` (8ebce05a3, 09-14) dispatches it the
minute the roll ends. Evidence-ladder spec §8 (landed 2026-09-24, uncommitted) prices it and names
the four-layer VSM miss: durations scrubbed from fingerprints, an unaddressed red fingerprint open
15 days, the cost sentinel never built, six reviews of mechanism and none of placement.

**What origin/dev already carries (this checkout is 76 ahead / 61 behind, merge-base 7d27b04c7).**
The 61 commits are almost entirely this concern: `push-delivers-within-budget` habit (RED, live
series, budget arithmetic, BUDGET_EXHAUSTED findings), per-pipeline budgets (app 180 min,
orchestrator = chain sum 780), validate-only dispatch from the graph, content-keyed rolls, the
rakia oracle + attested-pin gate (rakia schema now accepts `cargo.env` and an `attested` run kind;
`concern_routes.py` and `ci_trigger.py` exist), and the **root cause of the two-hour window**: a
kitsune2 DHT-model rebuild on every conductor restart does ~10k full-table scans per DNA (56 min on
matthew, 6 h on eve); a sargable-arc + covering-index fix sits on the unpushed fork branch
`perf/k2-dht-model-sargable-arc` (e0bfc6c7a), measured ~20× on synthetic stores.
So: **Lane 0 is reconciliation, and every other lane composes with what landed.**

**The outcome we want** (operator, 2026-09-24): a native CI/CD experience that is useful, lowers
mental and reporting load, lets us validate on localdev → hybrid mesh → fleet without waiting on
Jenkins, mints "works on my machine" as a content-addressed attestation, and can be made
aesthetically pleasing at the end — built the right way once, benefiting the whole, not patched.
And a system that feels its own pain before the operator has to name it.

**Design rulings this plan makes** (defended in the lanes; the operator can veto in review):
1. No new Jenkins stages (root stage model 4386/4500 CPS). The split is *concern-scoped phases*
   with recorded timing; the native "build #" view is the walker, not BlueOcean.
2. Readiness is a *precondition that refuses in seconds*, never a wait. Waiting is
   measurement-by-blocking. Re-dispatch is an *event* (fleet became writable), not a poll inside
   the App pipeline.
3. Convergence is the peer's job (P1 reconciliation controller). CI authors once; the declare
   fan-out leaves the App pipeline and becomes a *measurement* in the dataplane lane.
4. Run classes are declared data on the manifest (`steps[].class`), read by both oracles; the
   schema path is the one the pin-attestation plan proved (rakia commit → pin bump → codegen).
5. Cost lands in the SessionStart headline through declared measures + bounds + the existing
   findings ledgers with a `concern:` address. No new register, ledger, or ranking script.
6. Protocol `Reach` is the only reach vocabulary; brit's `ReachLevel` becomes a derived view.
   One CID implementation (`elohim_epr::cid`) for every attestation.
7. The sensor persona senses *pre-measure* exhaust and may only emit registered sensors —
   never fixes, never reports.

**Source-of-truth declarations (for the P2P design gate).** Every "schema" this plan touches
outside Lane N is a *declaration file*, not a storage entity: the rakia `build-manifest` and
`release-manifest` schemas are the rakia-validated SOURCE (seam-registry C0: class-A
content-addressed bytes, anchoring deferred), `measures.yaml` / `policies.yaml` are dev-plane
private declarations (never imported or witnessed), a2o `.feature` files are stories, and
`deploy-intent.json` is an Ephemeral (C) CI artifact reconstructable from the build. The only
storage-entity change is Lane N, whose gate answers are written there. The one route this plan
adds (`GET /db/content/{id}/head` branch, N4) follows the existing `Content` + canonical-head
entry type; no route precedes a DHT design.

---

## Lanes, dependencies, parallelism

```
Lane 0  reconcile tree ──────────────────────────────┐ (serial; blocks everything)
                                                     ▼
 wave 1 (parallel, disjoint write-sets):
   A  stage phases + readiness precondition   [scripts/ci, root Jenkinsfile]
   B  run classes as declared data            [rakia schema, orchestrator/*.mjs+Jenkinsfile, edge Jenkinsfile params]
   C  T2 not-ready window (household a2o)     [genesis/a2o/features/dataplane/app-delivery-refuses-fast.*]
   D  price it                                [.claude/scripts/*, genesis/orchestrator/delivery-series.mjs, .claude/epr-meta/*.yaml]
   E  sensor persona (algedonic designer)     [.epr-meta/elohim/packages/agents/*, .claude/agents/*, commands]
   F1 reach + CID ruling (design)             [elohim/brit (submodule), rakia design spec §5 amendment]
   K  conductor window root-cause roll        [holochain-conductor fork + pin]  (operator-gated)
 wave 2:
   A3 event-driven re-dispatch        ← A1, A2, B (RUN_CLASS param)
   C2 T2 receipt contract             ← C1, A1
   F2 household receipt → attestation ← F1, Lane 0 (brit pin)
   F3 CI publishes build attestations ← F1
   N  app bundle as elected content   ← F1 ruling, C (household lane green)   [elohim-storage release_adoption, doorway bundle_heads, a2o delivery/*]
 wave 3:
   G  walker tier+cost columns, portal data contract ← D1, F2/F3
   A4 retire declare fan-out ← N stations 1-3 green on the household
```

Write-set rule for a Workflow run: one lane per agent, the sets above are disjoint except the root
`Jenkinsfile` (owned by A; B's `RUN_CLASS` param is *declared* by A, *sent* by B) and
`elohim/holochain/Jenkinsfile` (B declares the param; nobody else touches it this sprint).

---

## Lane 0 — Reconcile the tree (serial, first, ~half a day)

Goal: one `dev` that carries origin/dev's delivery repairs, this checkout's 76 commits, and the
uncommitted §8/habit delta — with pins moved forward and gates green.

1. **Inventory uncommitted work before touching anything.** `git status` shows unrelated
   uncommitted edits in `elohim/elohim-storage/src/{db,http.rs,main.rs,p2p/projection_reconcile.rs,
   rea_projection.rs,services/*}` and `app/elohim-app/src/app/qahal/CLAUDE.md`. Read `berth ledger`
   to find the session that owns them; do not commit or stash another session's work. If unowned,
   move them to a scratch patch (`git diff > <scratchpad>/orphan-storage.patch`) and record it in
   the handoff. Commit our own two files: spec §8 + `dataplane-convergence.habit.md` + re-projected
   `genesis/manifests/habits.yaml` (pathspec commit, per push discipline).
2. **Merge origin/dev into dev** (merge, not rebase — 76 local commits include ledger churn).
   `git merge-tree` dry run shows 4 content conflicts + 1 file/dir: `.claude/data/deprecations.jsonl`
   (ledger: union both sides, keep fp uniqueness), `genesis/orchestrator/README.md`,
   `genesis/orchestrator/package.json`, `pnpm-lock.yaml` (regenerate with `pnpm install
   --lockfile-only`), and `genesis/orchestrator/.epr-meta` (origin turned the manifest file into a
   directory holding the two new habit atoms — take origin's directory, fold our manifest body in).
   Pins move to origin's: rakia d3329b2, brit 92f5faf, sophia 631f7f4 → `git submodule update
   --init elohim/rakia elohim/brit sophia`. The untracked `elohim/rakia/rakia-executor/` (a
   feedback-signal compute executor, uncommitted in the submodule) is either committed upstream in
   rakia or removed from the working tree — decide in step 1's inventory (recommend: commit it in
   rakia on its own branch; it is the compute-receipt executor Lane F3 will want).
3. **Gates:** `just gate genesis-orchestrator`, `pnpm rakia:schema:validate`, the rakia codegen
   gate, `node --test genesis/orchestrator/*.test.mjs`, `python3 .claude/scripts/habits-project.py
   --check`, `epr flow report --headline`.
4. **Re-baseline this plan** against the merged tree: re-read `push-delivers-within-budget`
   (its RED reading + DELTA 09-23a) and `pin-attestation`; strike any task below already landed.
5. **WIP fence (operator ruling 2026-09-24).** `push-delivers-within-budget` becomes
   `active: true`; `runtime-death-witnessed` steps back to `active: false` (its receipts are green
   and its next station is an ark S2 item); `dataplane-convergence` stays active. One-line deltas
   in both atoms, then `habits-project.py`.

Verify: merged `dev` builds the orchestrator tests green; `habits-status.py` renders both active
habits; `git log --oneline -1` on dev is a merge commit whose parents are the two heads.

**Lane 0b — second reconciliation (after wave 1 lands; before ANY push).** The closed shift
(`.claude/shifts/2026-09-24T04-30-land-batch-c-deliver-app.sprint-result.md`) landed batch C
(`ed1d7a9b5..2015f3e6c`) on origin/dev at 14:45Z; dev is now 36 ahead / 11 behind. Merge
`2015f3e6c` the same way (criss-cross expected on the ledgers, README, package.json, lockfile —
resolve as in Lane 0). **Pin hazard, verified 2026-09-24:** local dev carries the conductor
gitlink at `7e553f9c3` (d8b8aa19f, d3d7175ce) while origin/dev and the merge-base hold
`25dd2d0be`; the household deliverability lane passes on 25dd2d0be and missed doorway-B
convergence 3× on 7e553f9c3 (memory `project_conductor_pin_7e553f9c3_convergence_regression`).
The merge MUST land a `build(conductor): pin held at 25dd2d0be pending K0 bisect + window
measurement` commit so the next push does not roll the regressed pin. That push is batch C's
fresh trigger (orchestrator #1904 died before dispatching anything).

---

## Lane A — Concern-scoped phases + readiness precondition (scripts/ci, root Jenkinsfile)

Serves `push-delivers-within-budget` (the readiness wait is most of the app's 139.7 min observed
max) and `dataplane-convergence` (A4).

**A1 — `scripts/ci/fleet-write-readiness.sh`** (new; bash + coreutils only, per `scripts/ci/.epr-meta`).
Single-shot, one call per doorway URL from `resolveDoorwayEprUrls()`:
- `GET /health/serving` (doorway `routes/health.rs:813-882`): 503, `shedding`, `degrading`,
  `storageServing.status ∈ {refused, unreachable}` ⇒ NOT READY with the face named.
- A zero-byte content-addressed blob PUT re-ask (the side-effect-free probe `stage-spa-blob.sh`
  already uses after a broken PUT): a `503 {"status":"catching-up"}` or
  `forwarded_to_storage:false … timed out` ⇒ NOT READY (face = catching-up / storage-forward-timeout).
- Exit codes follow the house convention: 0 ready · 3 `FLEET-NOT-READY <host> face=<f>
  retryAfter=<s>` · 2 usage/unparseable. It never sleeps.
- Tests: `scripts/ci/tests/fleet-write-readiness.test.sh` on the existing deterministic harness
  (scripted curl, real node parser) — ready / each of three faces / mixed hosts / unparseable.

**A2 — phases with timing, no new stages** (`Jenkinsfile` helpers `stageAndVerifyAllBundles`,
`emitAppDeployJunit`).
- Phase 0 `readiness`: call A1. Not ready ⇒ record `outcomes["readiness|<host>"]`, emit junit
  case `readiness` with the face, write `deploy-intent.json` (commit, bundle CIDs, env, the face,
  `retryAfter`) as an archived artifact, set stage UNSTABLE via `unstable(...)`, **return** — no
  seed, no author, no wait. Ready ⇒ continue with `STAGE_CELL_READY_BUDGET_SECS=300` in `withEnv`
  (tail tolerance only; the 7200 default in the script stays for the household prologue's own
  override and is no longer the CI value).
- Phases named by concern in the junit report (`time=` currently hard-coded `"0"` at `:719`):
  `publish.seed`, `publish.author`, `verify.mounts`, `verify.projected`, `verify.shell`,
  `converge.declare` (until A4 removes it). Each records wall-clock from `System.currentTimeMillis()`
  in the outcomes map; classname stays `elohim-app.deploy.<env>` so ci-harvest's testReport read
  keeps working.
- Keep helpers heredoc-free; every new bash body is a script. Stage model byte count must not
  grow: verify with `.claude/hooks/jenkinsfile-method-size.py` in ARGV mode before commit.
- Fix the stale "Level 0" comments (`:554`, `:1761-1767`) while there.

**A3 — event-driven re-dispatch** (orchestrator; after B's `RUN_CLASS`).
- The orchestrator reads the app's `deploy-intent.json` artifact when the app result is UNSTABLE
  with a `readiness` case; it **does not advance the app baseline** (today UNSTABLE advances it —
  `recordPipelineResult :530-532`), so the next push re-dispatches, and it records
  `pendingDeploy: {commit, intentCid}` in the level checkpoint.
- The existing `cron('0 9 * * *')` + `timer-dispatch.mjs` gains a `deploy-pending` pass: if a
  pending intent exists and `fleet-write-readiness.sh` exits 0, dispatch app with
  `RUN_CLASS=deploy` (Lane B) and `DEPLOY_ONLY=true`; the already-built filter exempts it. Cadence
  every 30 min is enough (the window is 20–120 min today, minutes after Lane K).
- Tests: `genesis/orchestrator/pending-deploy.test.mjs` (intent present + ready ⇒ dispatched;
  intent present + not ready ⇒ held, no baseline advance; no intent ⇒ nothing).

**A4 — retire the declare fan-out from the App pipeline** (after N stations 1–3 green).
Delete the `DECLARE_ONLY` loop in `authorHeadOnce` (`DECLARE_MAX_ATTEMPTS=24`, ~36 min
worst-case). `verifyProjectedHeads` (400 s converge window) stays as the *measurement*, UNSTABLE
never FAILURE. Propagation is measured where it belongs: `@concern:federation-deploy` in the edge
Dataplane Validation lane. Habit delta on `dataplane-convergence`.

Verify (A): unit tests green; `node --test genesis/orchestrator/pipeline-budget.test.mjs`; one
`[build:app]` push during a known not-ready window shows the `readiness` case in the junit, stage
UNSTABLE in < 60 s, intent archived; one during a ready window delivers with phase timings > 0.

**Tasks**

- [x] **A1 — fleet-write-readiness.sh: single-shot readiness precondition per doorway, exit 0/3/2, never sleeps** (serves push-delivers-within-budget) — b6f256b9a; one face vocabulary abd014123
- [x] **A2 — App delivery in concern-scoped timed phases; readiness refuses UNSTABLE in seconds with deploy-intent.json** (serves push-delivers-within-budget) — 5d44375b6; faces abd014123
- [x] **A3 — event-driven re-dispatch: hold the app baseline on a readiness UNSTABLE, pendingDeploy, deploy-pending timer pass** (serves push-delivers-within-budget) — 7002b26ca; K follow-up capture-rollout-evidence --since-conductor 983efe6bb
- [ ] **A4 — retire the DECLARE_ONLY fan-out from the App pipeline; verifyProjectedHeads stays as the measurement** (serves dataplane-convergence, push-delivers-within-budget)

---

## Lane B — Run classes as declared data (rakia schema, orchestrator)

Serves `push-delivers-within-budget` (no-op rolls, telemetry never rides deploy) and
`pin-attestation` (uses its schema path).

Classes: `build | deploy | verify | measure | profile`. Semantics: a **run class** is derived, never
hand-picked: the class of a dispatch = the *maximum* class among its stale steps
(build > deploy > verify > measure > profile), and a class only executes steps at or below it.
`[edge:validate-only]` becomes the special case `verify`; bf898278f's `every { it == 'dataplane-validation' }`
becomes `every { stepClass(it) <= 'verify' }`.

- **B1 schema** (rakia, operator-owned): `steps[].class` enum, default `build`, in
  `elohim/rakia/schemas/v1/build-manifest.schema.json`; regen `generated_types.rs` (`Step.class`);
  mirror in `genesis/orchestrator/manifest.schema.json`; pin bump. Follow pin-attestation plan
  Tasks 2–3 to the letter (this is the same four-commit landing shape).
- **B2 manifests**: `elohim/holochain/build-manifest.json` — `dataplane-validation: verify`,
  `mesh-quiesce-measure: measure` (the `runMeshQuiesceMeasure` inside Build Edge Node Image),
  profiling/telemetry steps `profile`; `app/elohim-app/build-manifest.json` — `build-angular:
  build`, `publish-app-delivery: deploy`, `verify-app-delivery: verify`, `e2e-alpha: verify`.
- **B3 orchestrator**: `pipeline-registry.mjs` / `build-graph.groovy:751-766` / `getPipelineMetadata`
  carry step classes; `applyBuildGraphRouting` computes the class; `triggerPipeline` sends
  `RUN_CLASS` to **every** downstream (booleanParam→stringParam); commit tag `[run:verify|measure|profile]`
  generalizes `[edge:validate-only]` (parsed in a `@NonCPS` def — the Checkout block is at 10062/11000).
- **B4 downstream**: root and edge Jenkinsfiles declare `RUN_CLASS` (`params.RUN_CLASS ?: 'build'`)
  and gate stages with `classAllows('deploy')`-style defs; `shouldRunStep` learns the class. The
  App pipeline's Publish phase runs only for class ≥ deploy; SonarQube/telemetry only for `profile`
  unless changed files demand it.
- **B5 (from the 2026-09-24 handoff, one line, lands with B3):** `genesis/orchestrator/Jenkinsfile`
  stage `Brit Plan (advisory)` (~:2333) wraps its steps in `catchError(buildResult: null,
  stageResult: 'UNSTABLE') { … }` so a durable-task launch flake (`process apparently never
  started`, exit -2 — orchestrator #1904, 7 pipelines aborted before start) can no longer cascade
  to FAILURE with 0 dispatched. Advisory means advisory. Test: a case asserting the stage body is
  guarded.
- Tests: extend `validate-only-pipeline.test.mjs` → `run-class.test.mjs` (class derivation table,
  tag override, `[build:*]` still forces `build`), `graph-walker.test.mjs` (class exposed per step),
  a `rakia affected` oracle round-trip on a fixture repo (pattern in `gate-oracle.test.mjs`).

Verify (B): a push touching only `genesis/a2o/**` prints `RUN_CLASS=verify` for edge and app and
rolls no pod; `[run:measure]` on an empty commit runs the mesh-quiesce measure alone.

**Tasks**

- [x] **B1 — rakia schema steps[].class enum + generated Step.class + orchestrator mirror** (serves push-delivers-within-budget, pin-attestation) — rakia feat/step-class b081e9b, pin bump on pin/lane-B 9466cae71 (branches; pin not moved on dev); mirror class in ac1e3a6cd
- [x] **B2 — edge and app build manifests declare step run classes** (serves push-delivers-within-budget) — pin/lane-B 9466cae71 (branch, rides the rakia pin bump)
- [x] **B3 — orchestrator derives the run class and sends RUN_CLASS to every downstream; [run:*] commit tag** (serves push-delivers-within-budget) — 58b6b5456 1d29b1128
- [ ] **B4 — root and edge Jenkinsfiles gate stages by class (classAllows), shouldRunStep learns the class** (serves push-delivers-within-budget) — edge param declared in 58b6b5456; root App pipeline gating open
- [x] **B5 — orchestrator Brit Plan (advisory) stage guarded by catchError so a launch flake cannot abort dispatch** (serves push-delivers-within-budget) — ac1e3a6cd

---

## Lane C — The not-ready window, expressed on the household (a2o, T2)

Serves `push-delivers-within-budget` (proof before paying) and the evidence-ladder ascend-only rule.

- **C1 feature** `genesis/a2o/features/dataplane/app-delivery-refuses-fast.feature` — its OWN file
  (adding to `epr-app-deliverability.feature` changes the pre-push receipt contract).
  Tags `@e2e @dataplane @act:i @requires:owned-substrate @concern:push-delivers-within-budget`.
  Do **not** tag `@requires:deploy-churn` (Act I declares it unavailable; we *cause* the window).
  Stations:
  1. *A doorway that is shedding refuses the deploy in seconds, naming the face* — `PUT /admin/dev/shed`
     (fixture gate, loopback) on doorway A; run `fleet-write-readiness.sh` ⇒ exit 3 within 30 s,
     face=catching-up, retryAfter carried.
  2. *A restarting conductor is a window, not a failure* — `meshControl('conductors-restart')`
     (300 s step timeout, not the 180 s default); readiness exits 3 with face=cell-not-running or
     catching-up; then `pollUntil` caughtUp (`dataplane.steps.ts:447/1566`); readiness exits 0.
  3. *The same content-addressed offer delivers once the window closes* — `stageBundle()` with
     `STAGE_CELL_READY_BUDGET_SECS=60` and a per-scenario `STAGE_CELL_READY_STATE_DIR`; the head
     the doorway serves equals the offered hash (`bundle_heads` tick ≤ 30 s).
  4. *Nothing waited longer than it was told* — the stage's own timing lines are read back: no leg
     exceeded its declared budget (this is the household form of the cost bound).
  Steps: reuse `stageBundle`, `meshControl`, `restartStoryDoorway`, the dev-shed steps in
  `steps/federation/name-routing.steps.ts:656-726`, the caught-up Thens in
  `self-healing-flow-control.steps.ts`. New: `'the fleet write-readiness probe answers {word} within {int} seconds'`.
- **C2 receipt contract**: `genesis/orchestrator/scripts/t2-receipt.sh` `SERVING_RE` gains
  `scripts/ci/fleet-write-readiness.sh`, `scripts/ci/stage-spa-blob.sh`,
  `elohim-storage/src/services/release_adoption/`; `serving-receipt.mjs` accepts this feature's
  report as the receipt for those paths; `T2_RECEIPT=strict` for them (the ascend-only rule
  becomes admission).

Verify (C): `just test mesh features/dataplane/app-delivery-refuses-fast.feature` 4/4 twice;
pre-push on a stage-spa-blob.sh change prints the receipt line, not `NO-T2-RECEIPT`.

**Tasks**

- [x] **C1 — app-delivery-refuses-fast.feature: four household stations for the not-ready window** (serves push-delivers-within-budget) — be979acbc 5ec286ce3
- [x] **C2 — T2 receipt contract: t2-receipt.sh SERVING_RE and serving-receipt.mjs admit this feature as the receipt, strict** (serves push-delivers-within-budget) — bc84d1f9b (receipt line proven in a fixture repo; the live receipt waits on a household run)

---

## Lane D — Price it (cost in the headline; compose with algedonic slice 1 + push-delivers-within-budget)

- **D1 per-stage series**: `genesis/orchestrator/delivery-series.mjs --stages` reads
  `wfapi/describe` per build (reuse `pipeline-trajectory.mjs:getBuildStages`), emits per
  pipeline × stage p50/p90 and "cost per delivered bundle" (pipeline-hours ÷ bundles whose
  `verify.shell` passed). Declare `stage-wallclock@1` and `delivery-cost@1` in
  `.claude/epr-meta/measures.yaml`; bounds in `policies.yaml` (app `publish.*`+`verify.*` p90 ≤ 20
  min; cost-per-delivered ≤ 1 pipeline-hour); the harvest hook records them with
  `epr flow note --measure` so `epr flow report --headline` prints the line.
- **D2 ci-harvest**: keep `durationMillis` as a field on findings (never in `fingerprint()`), file
  `STAGE_OVER_BUDGET` findings with `concern` from `concern_routes` (the BUDGET_EXHAUSTED pattern
  at `:364`), and address the open `e22562ad0ec9` fingerprint with `concern: dataplane-convergence`
  (status triaged, pointing at this plan).
- **D3 `.epr-meta` inject rule** in `scripts/ci/.epr-meta` (lightest signal, per
  `elohim-epr-metafile`): a `*_SECS:-<n>` default > 900 in a `*.sh` write must cite the habit that
  prices it (`why:` names §8). Advisory, not deny.
- **D4 algedonic slice-1 leftovers** now unblocked by the merge: Task 5 (habits-status joins live
  pain per concern) — the headline shows *pain + last evidence + cost* on one row.

Verify (D): `node genesis/orchestrator/delivery-series.mjs --stages --json` on the last 10 runs
prints the readiness/publish/verify split; `epr flow report --headline` shows a `cost:` line;
`python3 .claude/scripts/_lib/__tests__/ci_harvest_stage_budget_test.py` passes.

**Tasks**

- [x] **D1 — delivery-series --stages: per-stage p50/p90 and cost per delivered bundle; stage-wallclock@1, delivery-cost@1 declared** (serves push-delivers-within-budget) — f8512d40f; NaN fix 9040b0eda
- [x] **D2 — ci-harvest STAGE_OVER_BUDGET findings addressed to their habit, durationMillis a field only** (serves push-delivers-within-budget) — 322c3d63a
- [x] **D3 — scripts/ci .epr-meta inject: a long *_SECS default names the habit that prices it** (serves push-delivers-within-budget) — 66994b31d
- [x] **D4 — habits-status joins pain, last evidence and cost on one row per active red** (serves push-delivers-within-budget) — 308638344

---

## Lane E — The sensor persona: `algedonic-designer` (the system's subconscious)

Operator intent (2026-09-24): "the emotional subconscious of the system — registers the pain
before we consciously understand it; it must have the senses for those primitive things that get
under the nerves; not reliant on deterministic things that hide pain because a measure was never
registered."

**Mandate.** Survey, occasionally and non-deterministically, for burden that no declared measure
sees; design and register the *sensor* (measure + bound + concern address + habit check or
`.epr-meta` rule) so the pain fires through existing channels. It never fixes, never files a
report, never guesses an address (§4b: honest absence over a guessed address).

**Nerve endings (pre-measure senses, raw exhaust — read, never declared):**
- *Waiting*: Jenkins stage `durationMillis`; `sleep`/`ATTEMPTS`/`_SECS` defaults in scripts;
  `pollUntil` budgets in a2o steps; berth `refuse` entries (a lease someone waited on).
- *Repetition*: ledger `seen` counts with `status: open` and no `concern`; the same fingerprint
  across ≥ 3 builds; the same command in ≥ 3 handoffs; `git reflog` churn on one path.
- *Effort with no fruit*: builds ABORTED/FAILURE with zero delivered artifacts; gate-skip
  no-measures; a2o runs with 0 receipts; T4 spend with no evidence row (§4 of the ladder).
- *Interruption*: RAM-guard sheds (`ram-guard status`), io-guard pauses, `signal: 15` in gate logs.
- *Contradiction*: green verdict with nothing delivered; a habit `green` with
  `observed_status: not-measured`; a spec claim whose cite is DEAD.
- *Apology and rant*: operator messages/handoffs containing "sorry", "still going", "again",
  "why do I have to"; agent transcripts with ≥ 3 retries of one tool call.
- *Numbness*: a signal channel that has been silent longer than its subject's change rate
  (e.g. no CI finding for a job that failed 5× — the harvester itself is the pain).

**Shape.** One agent package under `.epr-meta/elohim/packages/agents/algedonic-designer/`
(planted with `plant-eprfs-agent`; projection to `.claude/agents/algedonic-designer.md`), model
Opus, tools read-only except: `Write` to `measures.yaml`/`policies.yaml`/a `.epr-meta` rule/a
habit `checks:` line, and `epr flow note --measure`. Output contract: ≤ 3 sensors per firing, each
a 4-tuple (measure id@version · bound · concern address · reader = the surface that renders it),
plus one `epr flow note --kind observation` per sensor naming the raw sense that fired. Hysteresis:
a sense already covered by a declared measure is skipped (`concern_routes` + `measures.yaml` are
the "already conscious" check); one open sensor per (sense, subject).

**Cadence — a surprise auditor, not a fixed slot** (operator ruling 2026-09-24). (1) a
`/pain-sweep` command for on-demand; (2) a station in `/delivery-stasis` (runs when the conveyor
has room); (3) a **daily** scheduled cloud routine (`/schedule`) that rolls a die at start:
fire with probability 0.2 (≈1.4 firings/week, unpredictable day), cap 2 firings per rolling
7 days, floor 1 per 14 days so it can never go numb; on each firing it also draws a random
subset of the nerve endings (3 of 7) and a random look-back window (2–14 days), so the same
evidence is never surveyed the same way twice. The seed and the draw are written into the
firing's `epr flow note --kind observation` so a surprise is still auditable. (4) Never on a
hook, never at SessionStart.
**Created 2026-09-24:** cloud routine `trig_01Pm5R6UBgRUNFQoPDUWPNAR` (Default environment,
driver claude-opus-5-5, cron `17 9 * * *` UTC, no connectors), operator rulings: the write
channel is a `pain/<YYYYMMDD>-<seed>` branch pushed to origin (the firing record IS the commit,
subject carries `seed=` and `draw=`; an empty commit when nothing is minted); cap/floor are read
from `origin/pain/*` tip dates plus `algedonic-designer seed=` commits on dev; the
delivery-stasis pain-sweep station merges `pain/*` into dev locally (station edit owed: E4).
Its own success measure: `operator-surfaced-pain@1` — count of pains the operator had to name
in a week (source: messages/handoffs tagged by the sweep) — the number this persona drives to 0.

**First firing = self-test**: retro-mint the sensor that would have caught the 7200 s wait
(*waiting* + *effort with no fruit* on app #1719–#1725) and show it fires on the archived builds.

Verify (E): package round-trips byte-identical (plant skill fidelity gate); the self-test emits
`stage-wallclock@1` red on the archived runs; a second run over the same evidence emits nothing
(hysteresis).

**Tasks**

- [x] **E1 — plant the algedonic-designer persona package and its projection** (serves push-delivers-within-budget) — 6cb119bfa; success measure operator-surfaced-pain@1 4e5a6d3de
- [x] **E2 — cadence: /pain-sweep command, delivery-stasis pain-sweep station, daily surprise-auditor cloud routine** (serves push-delivers-within-budget) — 8c5679f9d; routine trig_01Pm5R6UBgRUNFQoPDUWPNAR
- [ ] **E3 — first firing self-test: stage-wallclock@1 fires red on archived app #1719-#1725 and stays quiet on a second pass** (serves push-delivers-within-budget)
- [x] **E4 — delivery-stasis pain-sweep station merges origin pain/* branches into dev locally** (serves push-delivers-within-budget) — 3d91ec75c

---

## Lane F — brit/rakia rails: "works on my machine" as a content-addressed claim

Serves `pin-attestation` and the rakia arc (reach IS promotion; attestation = governance).

- **F1 rulings (design, p2p-design-gate answered in the rakia spec amendment §5):**
  (a) protocol `Reach` (`elohim/epr/src/reach.rs`) is the single vocabulary; brit `ReachLevel
  {Unknown, Built, Deployed, Verified}` becomes a *derived view* (`Built→self`, `Verified` on the
  household → `trusted`, fleet-verified → `community`, prod → `public`); (b) every attestation
  CID comes from `elohim_epr::cid::compute_cid` — replace `BritCid` uses in
  `brit-epr/src/elohim/attestation/{build,deploy,validation}.rs` (a byte-identity test pins the
  claim that they already agree); (c) build attestations live in git notes-refs *and* are eligible
  content for the DHT later — no DNA change this sprint (`attestation:build-provenance` stays an
  idea; storage's release attestations keep reusing `device-health`).
- **F2 household receipt → attestation**: after `just test mesh`, `build-sprint-report.ts` calls
  `epr flow fulfill <report>` (today by hand) and `brit-build-ref validate put` a
  `ValidationAttestationContentNode` keyed by `env.sut` (the git-tree hash the report already
  carries), signed by the workspace identity the berth `moor` names. `serving-receipt.mjs` accepts
  the attestation (reach `trusted`) as the T2 receipt instead of file mtime.
- **F3 CI publishes `brit build put`** per pipeline (`BuildAttestationContentNode` with
  `manifest_cid`, `output_cid`, `build_duration_ms`, `agent_id` = the pipeline's deploy-service
  agent per Z.D) via `genesis/orchestrator/scripts/brit-helper.sh` (today it warns and exits 0).
  `delivery-series.mjs` gains `--from attestations` so the cost series can be read without the
  Jenkins API — the first step off Jenkins as the source of truth.
- **F4** decide the untracked `rakia-executor` (Lane 0 inventory) and, if kept, its
  `compute-receipt.schema.json` becomes the receipt shape F2 emits for a2o runs (one receipt type,
  not two).

Verify (F): `cargo test -p brit-epr` byte-identity test green; a `just test mesh` run leaves a
notes-ref attestation `brit-build-ref validate list` shows; pre-push reads it as the receipt.

**Tasks**

- [x] **F1 — reach and CID ruling: brit ReachLevel as a view over protocol Reach, attestation CIDs via elohim_epr** (serves pin-attestation) — brit feat/reach-derived-cid-epr 06f9f234a4 b9c27d5ba5 (branch; pin not moved)
- [x] **F2 — household receipt becomes a signed validation attestation the pre-push T2 reader admits** (serves pin-attestation) — 14730ccdd bc8826323 bc84d1f9b (real brit-build-ref → reader smoke green in a scratch repo; a `just test mesh` attestation waits on the mesh and brit-build-ref on PATH)
- [x] **F3 — CI publishes brit build attestations per pipeline; delivery-series --from attestations** (serves pin-attestation, push-delivers-within-budget) — 2eb1c8511 6be44104c + brit `feat/build-put-success-value` 22ec935c12 (branch; pin not moved; CI emits once brit-build-ref is on ci-builder)
- [ ] **F4 — decide the untracked rakia-executor; if kept its compute-receipt schema is the one receipt shape F2 emits** (serves pin-attestation)

---

## Lane K — Shrink the window at its root (conductor; operator-gated)

The 2-hour not-ready window is the kitsune2 DHT-model rebuild on restart (backlog
`fleet-standing-celldisabled-one-third-party-cell`, DELTA 09-22). Fix on fork branch
`perf/k2-dht-model-sargable-arc` (e0bfc6c7a, local only, off pin 25dd2d0be; 146+8 tests, clippy
clean, ~20× synthetic).

0. **K0 — measure before moving (from the 2026-09-24 handoff; quiet host, no other cargo/mesh):**
   (a) bisect the pin on the batch-C tree with the prebuilt `hc-fork-61565f320d0e` pair:
   `scratchpad/bisect-lane.sh 61565f320d0e mid` (script under the closed session's scratchpad
   `/tmp/claude-0/-projects-elohim/3a11e1a8-…/scratchpad/`; copy it into `genesis/local-dev/`
   with a header if it proves useful) — pass ⇒ suspect `0efa40939` (publish selects only locally
   validated ops), fail ⇒ suspect `ff2ea44c6` (iroh receive throttle); load, not the pin, is the
   demonstrated factor in the household misses. (b) measure the post-restart window on the
   persisted 2.3 GB matthew store for both pins with the direct-launch script
   (`scratchpad/measure-conductor-window.sh <checkout> <label>` on 25dd2d0be, then with
   `FORK_BIN_DIR=/projects/.claude-config/tools/hc-fork-7e553f9c30d5/bin`; hc_* exported on
   :9464–9466) — the number the CellDisabled backlog asks for, and the three-run split (WAL
   recovery vs DHT-model rebuild) the store-growth report names. Only after (a)+(b) does the
   gitlink move.
1. Publish the fork: fast-forward `elohim-0.7` → 7e553f9c3 and push `perf/k2-dht-model-sargable-arc`
   (both dry-run as plain fast-forwards). **Operator authorized 2026-09-24** ("if elohim-0.7 is
   useful then push it"); the controller's own push was refused by the harness classifier, so the
   operator runs the two push lines. Then open the branch's PR; the fork's own CI attests it (this
   is exactly what the attested-pin gate reads).
2. Move the `elohim/holochain-conductor` gitlink; `[build:conductor]`; the edge build consumes
   the tag through `CONDUCTOR_SOURCE_IMAGE`.
3. One staggered roll (content-keyed, so only conductors move); record per pod
   `conductor app is RUNNING again` − restart, as the arc doc's cycle-time row.
4. Structural follow-ups (don't hold `running_cells` hostage to join; one read per sector) stay
   in the arc as rung-3 items — captured, not planned here.
5. (added 2026-09-24, Lane H3) the same fork branch carries a second commit: WAL checkpointing
   + O(1) `storage_info`; the PR body names both findings and the three-run restart measurement
   that splits WAL recovery from the DHT-model rebuild.

Operator ruling 2026-09-24: **included, operator-gated** — agents prepare the fork PR, the pin
move commit and the receipt script; the operator triggers the fork push, `[build:conductor]` and
the roll. Everything in Lane A is correct either way; K is what makes the wait vanish for everyone.

**Tasks**

- [ ] **K0 — measure before moving: bisect the pin on the batch-C tree and measure the post-restart window for both pins** (serves push-delivers-within-budget)
- [ ] **K1 — publish the fork: fast-forward elohim-0.7, push perf/k2-dht-model-sargable-arc, open its PR (operator pushes)** (serves push-delivers-within-budget)
- [ ] **K2 — move the conductor gitlink to e0bfc6c7a and [build:conductor]** (serves push-delivers-within-budget) — prepared on pin/lane-K 62ffcfb2f (branch; dev holds 25dd2d0be)
- [ ] **K3 — one staggered roll with a per-pod restart-to-RUNNING cycle-time row** (serves push-delivers-within-budget) — receipt script 088ab0b6c prepared; roll is the operator's

---

## Lane N — App bundle as elected content (slice 2 of the elected-content spec, §12.8)

Serves `dataplane-convergence` (retires the per-host crutch `federation-deploy.feature` names)
and, structurally, `push-delivers-within-budget`: once peers *adopt* a release when they are
ready, the App pipeline no longer needs to wait for the fleet at all — step N6 removes the
app→edge `dependsOn` that created the coupling.

**P2P design gate (answered).** The app-bundle *release manifest* is **Notarized (A)**: a
`Content` version under the channel id `runtime:app-bundle:<network>:<channel>`, carried in
`metadata_json {"kind":"release-manifest"}` exactly as `release-ceremony.ts authorVersionFromManifest`
writes and `watch.rs::extract_release_body` reads. The browser/server zips are blob-plane bytes
addressed by CIDv1 inside the manifest (no entity of their own). The per-peer pointer write on the
slug row is **Ephemeral (C)**, reconstructable from the adopted manifest. The slug→channel binding
is Notarized (A): one `metadata_json.releaseChannel` field on the slug's own content version.
Entry type: existing `Content` + canonical-head election (`declare_canonical_content_head`
staging, `declare_earned_canonical_head`, `resolve_canonical_election`) — **no integrity change,
DNA hash unmoved**. Head plane: +1 head per (network, channel), two channels; ~5 deploys/day ×
one manifest version (all four artifacts ride ONE version) ≈ 1.3–1.8k versions/year under one
head — versions are not heads, quiesce delta nil. Identity: `blobCid = Cid::new_v1(0x55,
sha2-256)`; `sha256-<hex>` is the same digest in legacy dress (`blob_store.rs compute_addresses()`
returns both, stores ONE file); the manifest carries both, the vehicle writes `sha256-…` to the
row so `routes/apps`, SSR and `verify-served-shell.sh` stay untouched. Projection signal: the
adopting peer emits `StorageEvent::ContentUpdated{id: slug}` → doorway `bundle_heads.rs` dirty-slug
rerun (+30 s tick). No conductor signal needed.

**The one thing that would make this wrong**, and its cure: the slug row is today its own
elected head, so `head_adoption::try_adopt_canonical_head → pointer_heal_patch` would heal a
vehicle-written pointer back every sweep. Cure: a notarized binding + one predicate
`head_adoption::bound_release_channel(metadata_json)`; a bound slug returns `AdoptOutcome::Held`
before any pointer/head stamp (and `projection_reconcile.rs`'s stamp site skips it). A slug is
elected by exactly one mechanism, chosen in its own metadata. **Do not ship the vehicle without
this.** Metric arm `held_bound_to_release_channel`.

- **N1 class + schema** — `release_adoption/mod.rs`: `ArtifactClass::AppBundle` (`"app-bundle"`);
  `Artifact { app, kind }`; `AppliesTo { roles (default), apps: BTreeMap<slug, AppBinding{kinds,
  mount}> }`. `elohim/rakia/schemas/v1/release-manifest.schema.json` (rakia commit + pin bump,
  the 09-23 path): enum += `app-bundle`; `$defs.artifact` += `app`, `kind ∈ {browser, server}`;
  `appliesTo.required` via `if/then` by class; `$defs.appBinding`. No codegen exists for this
  type; `epr-release-package.ts ARTIFACT_CLASSES` += `'app-bundle'`, `--app-artifact
  <slug>:<kind>:<path>` (×4), `--applies-to-apps` derived. `serverBlobHash` rides as the `kind:
  server` artifact → prologue leg 4b (`hc-mesh-prologue.sh:369-402`) retires.
- **N2 verify** — `verify.rs`: `verify_shape` branch for AppBundle (apps non-empty, every
  artifact names a declared (app, kind), exactly one artifact per pair); `verify_envelope` skips
  the installed-reality/roles match for this class; new pure
  `verify_app_bundle_boots(manifest, entries)` calling `app_deliverability::judge_deliverability`
  on each browser zip → `RefusalReason::AppBundleCannotBoot` (terminal) / `AppBundleNotJudged`
  (transient); `watch.rs::check_channel` unzips staged browser artifacts into
  `VerifyInput.app_bundle_entries`; `apply.rs::staged_bundle_evidence` gets an `absent()` arm.
- **N3 vehicle** — `apply.rs::AppBundleVehicle{pool, ctx, events}` (`name "app_mount_pointers"`):
  per bound slug read the row (`content_diesel::get_by_id`), refuse `AppSlugRowAbsent`
  (transient) / `AppSlugNotBoundToChannel` (terminal, the mutual binding check), resolve
  browser/server `sha256-…`, idempotent no-op receipt when equal, write ALL slugs in ONE diesel
  transaction through `ContentProjectionPatch` (`blob_cid`/`blob_hash` + `content_size_bytes`,
  `metadata_json.serverBlobHash` → `server_blob_hash` via `server_bundle_from_metadata`),
  `partial_apply_refusal` on partial, emit `ContentUpdated` per slug, receipt
  `detail.apps{slug:{browser,server,mount}}`. Register in `main.rs:~5916`. Revert = same apply on
  the prior manifest. Plus the `head_adoption.rs` `Held` predicate above and a
  `release-ceremony.ts channel bind <slug> <channelId>` subcommand (merged-metadata
  `update_content`, no blob change).
- **N4 doorway** — `bundle_heads.rs` Converged channel: **no change** (reads the vehicle-written
  row; `ContentUpdated` already triggers `reconcile_pending_snapshot`). Candidate channel:
  storage `http.rs::handle_content_head` gains a branch — a bound slug resolves the election on
  its channel id; a staged candidate → `(app=slug, kind=browser)` artifact →
  `stagingCandidateBlobHash`; `parse_candidate_head` consumes it unchanged, so gamma serves the
  staged candidate with zero doorway edits.
- **N5 CI** — `scripts/ci/publish-app-release.sh <doorway> <manifest-out>`: deterministic zips
  (extract the zip function from `stage-spa-blob.sh` into `scripts/ci/lib/bundle-zip.sh`, shared
  with Lane A), `PUT /blob/{sha256}` ×4 to ONE doorway (other peers pull via
  `release_adoption/artifact_pull.rs`), `epr-release-package.ts --artifact-class app-bundle …
  --channel-id runtime:app-bundle:alpha:dev --soak-secs 60 --attestation-threshold 1`,
  idempotency by content (skip when the head's artifact CID set equals ours),
  `release-ceremony.ts publish --transport doorway` (new transport: `PATCH /db/content/{channelId}`
  metadata + `POST …/canonical-head`, the routes DECLARE_ONLY already uses; household default
  stays admin-WS). Auth: the existing `storage-api-key-admin` on one doorway for 4 PUT + 1 PATCH
  + 1 POST — bounded because staging never beats earned; the Z.D deploy-service agent replaces
  the key later (captured). `scripts/ci/verify-app-adoption.sh` polls `GET /db/p2p/adoption?peer=`
  per peer for `verdict.applied.releaseCid == cid`, 10 min bound, UNSTABLE on transient, FAILURE on
  `app_bundle_cannot_boot`. Modes: alpha follows `runtime:app-bundle:alpha:dev=canary` (alpha IS
  the canary), commons follows `elohim:commons=apply`; `deployments.json`
  `ELOHIM_RELEASE_CHANNELS` becomes a list (`parse_followed_channels` already accepts one; the
  orchestrator `runtime-config-render.test.mjs` single-channel assertion becomes a list rule).
  Soak probe for this class in `spawn_soak_observer`: re-judge + self-fetch `/apps/{hash}/index.html`
  → `release-soak` attestation (unchanged kind). Promote/revert = the existing verbs.
- **N6 household proof, then fleet** — new
  `genesis/a2o/features/delivery/app-bundle-elected-delivery.feature` (`@concern:runtime-upgrade-propagation
  @requires:household-nodes`; Background binds the slug and sets canary mode via `postFollow`/`setPeerMode`):
  1 publish at staging via one doorway (`appBundleCandidate()` beside `candidateHapp()`);
  2 every peer adopts — rows point at the release's browser+server bytes (4b retired);
  3 the doorway serves the elected head (reuse the deliverability "materialized that server
  pointer" step + served-shell-boot helpers); 4 canary attests, steward promotes
  (`ensureAttested`/`ensurePromoted`, channel parametrised); 5 revert by re-election
  (`ensureReverted`; every doorway serves the prior head); negative: a bundle without `main-*.js`
  is refused `app_bundle_cannot_boot` on every peer, no row moves. Fleet: the same class via a
  `verify`-class run after `deployments.json` follows the channel; receipt = `verify-app-adoption.sh`
  green on both alpha peers with no per-host PUT in the log. **Then** delete prologue leg 4b,
  `stageSpaBlobs`/`authorHeadOnce` (Lane A4), and the app→edge `dependsOn`.

Ordering inside N: N1 (schema, blocking) → N2/N3 (Rust, parallel with N5's packager work) → N6.
Overlaps flagged: `scripts/ci/stage-spa-blob.sh` zip extraction (Lane A owns the file; A does the
extraction), root `Jenkinsfile` (A owns; N5's phase deletion lands as A4), `deployments.json` and
`runtime-config-render.test.mjs` (Lane B's orchestrator agent). Old controllers refuse
`app-bundle` as `ManifestSchemaInvalid` — flip fleet follow sets only after the storage binary
carrying the class is deployed (C10 ordering).

Verify (N): `cargo test -p elohim-storage release_adoption::` (shape/envelope/boots/vehicle/held
predicate); household feature 6/6 twice; fleet `verify-app-adoption.sh` green; `federation-deploy`
scenario 2 still green with the crutch deleted.

**Tasks**

- [x] **N1 — ArtifactClass::AppBundle and the release-manifest schema app-bundle class** (serves dataplane-convergence) — ee5f9e54a, packager 31c0bfac6; rakia feat/app-bundle-class 720c132, pin bump on pin/lane-N 1b1002c84 (branches; pin not moved on dev)
- [x] **N2 — verify.rs AppBundle shape, envelope skip and verify_app_bundle_boots** (serves dataplane-convergence) — ee5f9e54a
- [x] **N3 — AppBundleVehicle writes every bound slug in one transaction, plus the head_adoption Held predicate** (serves dataplane-convergence) — ee5f9e54a; channel bind 31c0bfac6
- [x] **N4 — doorway candidate channel through storage handle_content_head for a bound slug** (serves dataplane-convergence) — ee5f9e54a
- [x] **N5 — CI publish-app-release.sh and verify-app-adoption.sh; channels followed as a list** (serves dataplane-convergence, push-delivers-within-budget) — 2cb1b6b2a, doorway transport 31c0bfac6; app-bundle soak-probe arm owed
- [ ] **N6 — household app-bundle-elected-delivery.feature, then fleet; retire prologue leg 4b and the app to edge dependsOn** (serves dataplane-convergence)

---

## Lane H — The store that grows (writer bounded at the writer; ruling 2026-09-24)

Surfaced by the operator from another session's report
(`genesis/docs/content/elohim-protocol/architecture/2026-09-24-conductor-store-growth-report.md`,
sniffer `app/elohim-app/scripts/conductor-store-sniff.py`): matthew's household conductor holds
2.3 GB after 42 hours; 27,317 of 30,787 lamad entries are `attestation:device-health` nodes
written by the three storage agents' doorway heartbeat (every doorway ever registered, every 150 s,
never deregistered; 179 subjects, 25,973 "unreachable" about doorways that lived four minutes);
the lamad WAL is 1.4 GB and never checkpoints. This is **causally upstream of the two-hour
window**: the restart rebuild's cost is (reads × ChainOp rows), and this writer is what inflates
the rows; WAL replay on restart is another slice of the same window. Serves `idle-is-free`
(its invariant verbatim: "work done on behalf of nobody is a defect") and, through Lane K,
`push-delivers-within-budget`.

**P2P design gate ruling.** A peer-health *sample* is **Ephemeral (C)**: it lives in the peer-meta
store / a storage-side table with a retention window and is never a notarized Content node. A
*state transition* (reachable↔unreachable, registered↔expired) is the only notarized event
(A), one per transition. Registrations expire: a doorway whose `active|<ts>` link is older than
N heartbeats leaves the probe roster. The report's option (c), with option (a) as its first step;
option (b) (update chains) is rejected because it still writes per probe. The content-type index
link per attestation is removed (a hot anchor that grows with time). This is a coordinator-only
change — DNA hash unmoved — plus doorway/storage code.

- **H0** commit the two files as they stand (pathspec; review the sniffer's key handling: it
  derives the db key from the passphrase like the fork does and must never print it).
- **H1 stop the bleed (immediate):** doorway heartbeat probes only the live roster (expire by
  `active|<ts>` age); the a2o serving receipt's scenario doorways deregister on teardown
  (`OwnedDoorwayPair` fixture); tests on both.
- **H2 the writer, redesigned:** samples → peer-meta/storage table with retention; notarize only
  transitions via the existing attestation coordinator; drop the per-attestation type-index link;
  a household-at-rest reading (`idle-is-free` check) before/after shows the idle create rate fall
  from ~648/h toward the ceiling.
- **H3 the fork (rides Lane K's branch as a second commit):** `PRAGMA wal_checkpoint(PASSIVE)` after
  each integration batch, `TRUNCATE` at idle/shutdown, `journal_size_limit`; `storage_info` reads
  `page_count × page_size` + WAL size (O(1)) instead of walking `dbstat` twice per DNA per minute.
- **H4 the sensor (algedonic-designer, real firing):** per-DNA growth budget as declared measures
  (`dna-actions-per-day@1`, `wal-main-ratio@1`, `idle-writes-per-hour@1`) with the sniffer `--json`
  as the household probe and conductor metrics as the fleet probe; bounds from the report
  (lamad ≤ 5k actions/day/peer idle; WAL/main ≤ 1; ≤ 100 links per subject anchor); concern
  address `idle-is-free`; reader = habits-status + headline. A limit raise is a design signal,
  never a capacity ask.

Write-set: `doorway/doorway-service/src/services/federation.rs` (+ hosts), the attestation
coordinator in `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs`,
`elohim/elohim-storage/src/services/attestation_projector.rs`, the a2o `OwnedDoorwayPair`
fixture, the conductor fork branch (H3), `measures.yaml`/`policies.yaml` (H4, append-only).
Overlaps: Lane N's soak attestations reuse `attestation:device-health` — bounded (1–3 per release)
and unaffected, but N must not add the type-index link H2 removes. Lane K's PR body gains the WAL
finding and H3's commit.

Verify (H): `cargo test -p doorway-service federation::` + the a2o receipt teardown test; the
sniffer on a stopped household after one idle hour shows lamad actions/day within the ceiling
and WAL/main ≤ 1 after H3; `idle-is-free` gains a dated delta with both readings.

**Tasks**

- [x] **H0 — conductor-store growth report and offline sniffer committed** (serves idle-is-free) — 553edab1f
- [x] **H1 — stop the bleed: heartbeat probes only the live roster, self-deregistration verb, a2o receipt doorways deregister on teardown** (serves idle-is-free) — 89d7c7e86 7be4d4e37 97977b60b; review fixes 78a46bd01 fc441e497
- [ ] **H2 — health samples leave the DHT: retention table, notarize only transitions, drop the type-index link** (serves idle-is-free)
- [ ] **H3 — fork WAL checkpointing and O(1) storage_info on the Lane K branch** (serves idle-is-free)
- [x] **H4 — store-growth sensors: dna-actions-per-day@1, wal-main-ratio@1, subject-anchor-links@1 with bounds** (serves idle-is-free) — 980ee25f6

---

## Lane G — The native "build #" view: walker columns + the portal's data contract

- **G1** `epr flow walk/status --json` gains `tier` (highest green rung + when, from a2o
  receipts and `@requires:` tags) and `cost` (from `stage-wallclock@1` folds and attestation
  `build_duration_ms`) per commitment; compiler-format lines `ERROR dangling-proof … / WARN orphan
  … / ERROR stale-claim …` (evidence-ladder increment 3). Rust in
  `elohim/eprfs/epr-cli/src/flow/walk.rs`; tests beside it.
- **G2** the portal's API *is* `walk --json` + `delivery-series --stages --json` + brit
  attestations. This sprint ships only a page on `reports:serve` (port 4201) that renders the
  pipeline graph with per-node tier and cost and links each node to its logs/artifacts —
  functional, not styled. The aesthetic pass (graphos, Library B) is captured as backlog with
  this data contract named as its input, so it is built on data, not on Jenkins.

Verify (G): `epr flow walk genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md
--json` shows tier/cost; the page renders the last 10 orchestrator runs with the readiness case
visible as its own node.

**Tasks**

- [x] **G1 — epr flow walk/status --json gains tier and cost per commitment; the cost: headline slot** (serves push-delivers-within-budget) — d964fbef2
- [x] **G2 — reports:serve page renders the pipeline graph with per-node tier and cost** (serves push-delivers-within-budget) — 73580ac63

---

## How this plan lands, and how it runs as a workflow

1. **Land it auditable (first execution step, before any lane):** copy this file to
   `genesis/docs/superpowers/plans/2026-09-24-native-delivery-sprint-plan.md` (frontmatter above
   already carries `status`, `class`, `process_subdomain`, `cites`, `serves`); `epr flow cites seal
   <plan>` (author `--desc` hints where it flags title-defaults); `epr flow project <plan>` → gap-items;
   `epr flow report placement --ledger | tail` shows them. Commit with the Lane 0 pathspec commit.
2. **Valueflow authoring:** one `epr flow claim --on <gap-id> --serves <habit>` per lane at
   pickup; `epr flow fulfill --report` at close with the lane's verify evidence; the harness
   witnesses agent edits (never run import/witness by hand).
3. **Who does what (operator ruling 2026-09-24, restating 2026-09-23): Opus 5.5 drives the
   bulk of the work; Fable is orchestrator, architect, planner and advisor only.** Every
   implementation task — Lane 0's inventory and merge, every lane, the landing of this plan, the
   habit deltas, the fleet-step preparation — runs as an Opus 5.5 agent (`model: opus`) on its
   lane's disjoint write-set, with the lane's *Verify* block as acceptance and the lane's habit
   as `--serves`. Fable's only work products are: design rulings when a lane hits a fork, review
   verdicts on each lane's diff (via `code-reviewer` + a fidelity read against the rulings),
   dispatch and sequencing, and the operator-facing summary. `blind-reader` on every new
   `.feature` (a2o `.epr-meta` requires it). A `Workflow` script runs Lane 0 first, then
   `parallel([A,B,C,D,E,F1,K-prep])`, then `parallel([A3,C2,F2,F3,N])`, then `[G, A4]` — each
   stage's agent reads only its lane section plus the *Design rulings* list, and returns a report
   Fable judges take/leave/reshape against the trajectory.
4. **One push per batch,** never mid-build; a `[build:app]` during a known window is the Lane A
   live check, a `[run:verify]` (Lane B) the no-roll check; Lane K's roll is the operator's.

## Rulings log — wave 1 (2026-09-24, controller)

Wave 1 landed on dev (A b6f256b9a 5d44375b6 · B 58b6b5456 1d29b1128 + rakia `feat/step-class`
b081e9b + `pin/lane-B` 9466cae71 · C be979acbc 5ec286ce3 · D f8512d40f 322c3d63a 66994b31d 308638344
· E 6cb119bfa 8c5679f9d 4e5a6d3de · F1 brit `feat/reach-derived-cid-epr` 06f9f234a4 b9c27d5ba5 · K
088ab0b6c + `pin/lane-K` 62ffcfb2f · H0/H1 553edab1f 89d7c7e86 7be4d4e37 97977b60b). Reviews: all
approve; the majors are fixed in wave 1.5 below. Rulings on the forks the lanes raised:

- **A** all five recommendations stand: `storage-refused` is NOT READY (a write that cannot be
  anchored does not start — and the fleet reads exactly that today, `serving_ok=false` ≈2.8 h after
  a doorway start: a fleet red for K0 to read, not a gate defect); transport error = not ready;
  exit 2 proceeds on the 300 s tail; sentinel-hash probe on CI, `READINESS_PROBE_HASH` on the
  household; one junit `readiness` case. **A adopts one face vocabulary** shared with Lane C:
  `cell-not-running | catching-up | storage-forward-timeout` for the plan's three faces, the rest
  as they are, and the host printed as the origin URL.
- **B** do not invent plan-named steps (`mesh-quiesce-measure`, `publish-app-delivery`, …) until
  the Jenkinsfile can honour them; every class below `deploy` dispatches validate-only; `[run:*]`
  selects every pipeline declaring a step at or below the class; cross-repo downstreams get
  `RUN_CLASS` only once they declare it; genesis lints stay `build`. Pre-existing red
  `validate-manifests.mjs` (mirror lacks `attested`) is fixed with B5.
- **C** shed for stations 3–4, real restart for station 2; served page within 30 s; the
  `features/dataplane/README.md` concern table gains `push-delivers-within-budget`.
- **D** the `cost:` headline slot is Rust (`HEADLINE_ORDER` in `flow/report.rs`) → Lane G1;
  `e22562ad0ec9` reopening on the next unsplit red is the honest signal; habits-status at ~6 s is
  acceptable off the SessionStart path; the `spread()` NaN is fixed in wave 1.5.
- **E** habit ids are declared concern addresses; `operator-surfaced-pain@1` gets its lens after
  the first folded week (hard 0, headline slot with G1); last-fired is read from the persona's own
  observation notes; the cloud routine is the controller's to create from the scratchpad spec.
- **F1** `Unknown→private`, `Deployed→intimate` in every scope; `BritCid` stays the field type
  (serde-transparent over the same `cid::Cid`); the engine keeps `BritCid::compute` under the
  parity test; the rakia spec lines "CI passed = trusted" are superseded (a pipeline's identity
  earns no reach; only verification of the running artifact does) — the amendment lands on its own
  rakia branch in F2. The dirty `elohim/brit` gitlink (HEAD on the feature branch) is restored to
  the pin by Lane 0b, branch kept.
- **K** the pin-move target is **e0bfc6c7a** (25dd2d0be + the sargable-arc fix alone), never
  7e553f9c3, which K0's bisect judges separately; the conductor manifest gains an `attested`
  step reading the fork's `pr_tests` check (B/F follow-up); `capture-rollout-evidence.sh` gains a
  read-only storage-pod log capture since the conductor's creation time (A3 agent).
- **H** the deregister verb over stable fixture ids; the unauthenticated admin verb is accepted
  now with admin-path auth for all `/admin/federation/*` routes decided together (backlog);
  partitioned-but-alive siblings stay skipped until H2; the peer-cache echo is H2's first named
  node.
- **Valueflow rail** refused every `epr flow claim` because the hand-written gap-items are not
  intents the sidecar knows; Lane 0b converts the lane items to checkbox tasks so `epr flow project`
  mints real gap-items, then claims are recorded retroactively per lane.

**Wave 1.5 (small fixes before Lane 0b):** B5 + mirror-schema `attested`/`class`; D
`spread()` NaN; A/C one face vocabulary + origin-URL host.

## Rulings log — wave 2 (2026-09-25, operator + controller)

- **Measure-class work never runs on the dev berth** (operator, 2026-09-25: "this LONG measured run…
  is exactly the kind of thing that holds up development"). The K0 A/B and the household proof held this
  workspace's mesh lease from 2026-09-24 20:16Z to 2026-09-25 ~13:00Z across two container restarts:
  measurement-by-blocking, the anti-pattern ruling 2 names, done to ourselves. Ruling: a run whose class
  is `measure` or `profile` (window/A-B measurements, soaks, proofs on grown stores) is dispatched to a
  **measurement peer** — a second stewarded device or devworkspace on the T3 hybrid rung (the P52s, or
  shem via `just dev conductor`), never the workspace a human is developing in. The dev berth runs only
  `verify`-class lanes, each bounded to its declared budget and refusing fast past it; the household
  proof on this workspace is that bound (≤ 30 min wall-clock or it stops and reports). Lane B's
  `RUN_CLASS ∈ {measure, profile}` therefore carries a *dispatch target*, and `berth` learns a
  `measure` resource whose lease on a dev workspace is itself a pain: the next `/pain-sweep` mints
  `dev-berth-held-by-measure@1` (mesh lease held by a measure-class task > 30 min on a workspace with a
  live human session). The measurement peer is the first job the ethosengine-collective seam below
  takes: a measure-class claim travels to the peer that has the stores, and its receipt travels back
  through brit. Until that peer exists, measure-class runs are queued, not run here.
- **K0 disposition**: e0bfc6c7a is the pin target on the household evidence (window 15–19 min → ~1 min;
  CPU and WAL unchanged). `pin/lane-K-on-dev` (52cc0f125) holds the move; landing + `[build:conductor]`
  + roll are the operator's. Two items leave K for the conductor arc: the per-hosted-agent publish
  selector/COUNT(*)/ScheduledFunction cost (steady-state 230–414 % CPU per conductor) and the WAL that
  checkpoints only in the first 90 s after boot (H3). The grown stores for reproducing both are archived
  at `/projects/.claude-config/k0-household-stores-20260925/`; the household was recast on e0bfc6c7a.

## Complementary work captured (backlog, not this sprint)

- **`ethosengine` as a Qahal collective — the collaboration level between devices** (operator,
  2026-09-24, from today's exhaust: two sessions committing to dev from one checkout, the same
  merge resolved twice, cargo without a claim, a push refused on another session's CID sync).
  Members = the operator's stewarded devices (Che workspace, local machine) + agent actors; commons
  store = claims (integrator lease, write-set, cargo, mesh, fleet pin), receipts, and the admission
  rule; a merge to `dev` becomes a governed reach elevation `self→trusted` (brit: merge consent read
  from the parent EPR; Z.D's authority-as-commitment one level down). First expression: claims as
  notes-refs on the shared remote (`refs/notes/ethosengine/claims/*`) written by `berth claim`,
  read by pre-push (refuse while another device holds the lease or an open claim overlaps the
  changed paths; TTL on claims); the qahal DNA hosts the same records later and the notes-refs become
  its projection. **Refined by the operator the same day:** every device and agent is *matthew's*
  stewarded device, so the level is born at reach `self` — a self-governed steward collective
  (one human, many hands): witness + claims, no consent ceremony; conflicts are the same person's
  two hands and the steward's own rule resolves them; agents' claims are bounded by the steward's
  commitments (Z.D). The **social boundary is a reach elevation**: the first *other* human's device
  claiming into the commons is the moment the collective exists and Qahal governance (membership,
  consent, redress) applies to the same record set. Home: imagodei/stewardship first (D2/D4), Qahal
  (D7) at the boundary. **Second refinement (operator, same day): the first expression is
  peer-native, not git notes.** Two workspaces (this Che workspace and the operator's P52s
  ThinkPad) run simultaneously as *peer-aware peer runtimes* — each a stewarded device of matthew
  joined to the same network (the T3 rung `just dev conductor alpha`, `local-conductor` set
  per-run). The story already exists and is HELD only on that capability:
  `genesis/a2o/held/features/auth/stewarded-device-sync.feature` (workspace agent W bound as a
  controller of matthew's identity head; transport binding; writes recognised at community reach
  under a `delegates-compute` grant; "a second household device makes the same handshake") — spec
  `2026-08-30-workspace-stewarded-device-peer-design.md`. The collaboration records ride the
  node-registry DNA the devices already join (`NodeRegistration` + `NodeHeartbeat` = the berth
  mooring made native). A claim on `dev` integration / a write-set / cargo / the fleet pin is
  **an REA Commitment** — the vocabulary `epr flow claim` already mints in the sidecar — notarized
  on the DHT between the devices (`CustodianAssignment` was checked and is content custody, the
  wrong shape for a lease); both devices see each other's claims through the DHT and pre-push reads
  the claim from the local peer. **Between clones the same record travels through brit, not git
  notes** (operator question, same day): brit is the covenant on git — EPR content nodes in
  `.git/brit/objects` with the SAME CID as their DHT twin (F1's parity test), synced as refs; a
  claim is one more `BritRefManager` family (`refs/brit/claim/<resource>`) beside build / deploy /
  validate / reach, and `reach compute` folds claims into the admission verdict ("may this push
  land on dev" is a reach computation). The CI runner is a brit peer (F3 already writes build
  attestations through it), never a runtime-less one. **Trajectory (operator):** VS Code (local) + OSS VS Code (Eclipse Che) →
  `lvi` (local: the runtime IS the peer, the editor attaches) → `lvi` on a peer through a doorway
  (Che rebuilt native); the peer-awareness is the same at every rung, lvi only makes the device a
  peer by construction (`project_lvi_devspace_peer_runtime`, spec 2026-07-20). Needs its own
  brainstorm + spec; the first collective-discipline work with a felt user, and the story that
  returns the held feature to the plate.

- rakia portal aesthetic pass (graphos) — input: Lane G data contract.
- kitsune2 structural fixes (running_cells not hostage to join; one read per sector) — arc rung 3.
- Reach vocabulary in `release_attestation` promotion thresholds — after F1.
- `attestation:build-provenance` DHT kind — after F3 evidence; moves the DNA hash.
- The orchestrator Checkout `script{}` diet (10062/11000) — before any further tag parsing.

## Verification (sprint-level, the DoD)

- `push-delivers-within-budget`: `delivery-series.mjs --window 10` reads ≥ 8/10 delivered, p90
  within budget, on live runs after Lanes A+B+K — the habit flips green on that reading, by hand.
- `dataplane-convergence`: A4 delta; `federation-deploy` scenario 2 still green live.
- Every lane's a2o feature green on the household twice; the T2 receipt admitted at pre-push.
- `epr flow report --headline` carries `cost:`; `habits-status.py` shows pain + cost per concern.
- The sensor's self-test fires on the archived builds and stays quiet on a second pass.
