---
id: "backlog-ci-edge-conductor-roll-no-halt-walks-the-fleet"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The staggered conductor roll had no halt — a failed conductor rollout neither stopped the roll nor paid the stagger soak, so edge #1413 walked the alpha fleet in `ordered` sequence at one 600s rollout timeout per peer and turned one node's DNA-drift reinstall into a seven-node source-chain loss"
slug: "ci-edge-conductor-roll-no-halt-walks-the-fleet"
written: "2026-09-02"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: blocked
fingerprints: [ca8705d89578, e63e3dcd0771, d89606ca5cde]
jobs: [elohim-edge]
relatedNodeIds: []
tags: [ci, elohim-edge, deploy, conductor, staggered-roll, rung-3, blast-radius, halt-on-failure, fail-fast, rollout-evidence, alpha, source-chain-loss, dna-drift, shared-artifact-failure]
cites:
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1413/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1414/
  - elohim/holochain/Jenkinsfile
  - scripts/ci/capture-rollout-evidence.sh
  - genesis/orchestrator/data/deployments.json
  - genesis/data/timeline/backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md
  - genesis/data/timeline/backlog/task-rollout-evidence-capture.md
  - genesis/data/timeline/backlog/ci-orchestrator-supersede-aborts-in-flight-edge-rolls.md
  - genesis/data/timeline/backlog/staggered-conductor-fleet-restarts.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# One call site, three fingerprints, seven torn source chains

## The failure

Three DEPLOYMENT fingerprints landed from `elohim-edge/dev` #1414 (started
2026-09-02T10:27:44Z), each seen 1×, `first_build == last_build == 1414`. **#1414 was still
`IN_PROGRESS` when this entry was written** (verified 12:02Z, ~40.7 min into "Deploy Edge
Node - Alpha" alone) — its `UNSTABLE` is Jenkins' worst-verdict-so-far, not a final result:

```
ca8705d89578  error: timed out waiting for the condition
e63e3dcd0771  Actual pod readiness for statefulset/elohim-jessica-alpha-conductor in elohim-alpha:
              0/1 pods Ready — elohim-jessica-alpha-conductor-0=Running/NotReady[node=ethosengine;
              containers=happ-fetcher=true/Completed;elohim-conductor=false/CrashLoopBackOff;ws-proxy=true/;]
d89606ca5cde  ERROR: statefulset/elohim-jessica-alpha-conductor rollout failed with exit 1;
              evidence capture exit 0, expected artifact:
              rollout-evidence/elohim-alpha--statefulset--elohim-jessica-alpha-conductor/
```

**All three are one call site.** `waitForRolloutWithEvidence()`
(`elohim/holochain/Jenkinsfile:380-395`) emits them in sequence for a single workload:
`ca8705d8` is `kubectl rollout status --timeout=600s`'s own stderr (line 382), `e63e3dcd`
is the readiness summary that `scripts/ci/capture-rollout-evidence.sh` tees on line 387,
and `d89606ca` is the wrapper's propagating `error` on line 393. They are not three
concerns and should never be triaged as three. Note what `evidence capture exit 0` means
here: the collector **succeeded** — this is the rollout-evidence machinery working, the
opposite polarity of museum trap #14.

## Verdict — real, and repo-side

Not a flake (deterministic: the workload cannot become Ready). Not purely infra either.
The *substrate damage* is operator-owned and already canonicalized in
`alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md` (integrity DNA hashes
moved → `happ_manager` reinstalled under the standing `ALLOW_DNA_REINSTALL=true` → a
non-atomic `uninstall_app` tore the authored DBs → conductors panic `CellWithoutGenesis`).
Applying the museum gate: this is **not** a listed trap and the root cause is **not** novel.

What *is* new, and what this entry owns, is the **propagation**: how one node's fault
became seven. That lives in the repo, in the rung-3 staggered conductor phase.

## Root cause — the roll had no halt, and its safety pause was conditioned on success

`deployHumansInParallel()` (`elohim/holochain/Jenkinsfile:725`) runs two phases. The
**storage** phase is parallel and per-peer-independent; its documented
UNSTABLE-not-FAILURE, continue-on-failure posture is well argued (a single peer's
resource floor is a per-peer test failure — the 2026-05-22 james OOM rationale in the
function's doc comment). The **rung-3 conductor** phase inherited that posture, and there
it does not hold:

1. A conductor only rolls when a **shared** artifact moved — `resolveConductorWorkloadImage`
   holds the live image unless the hApp digest/conductor pin moves ("hApp bytes must reach
   the conductors"). So the first peer whose conductor will not come Ready is a **leading
   indicator for every peer behind it**, not an isolated per-peer fault.
2. `deployHumanConductor()` ends in `waitForRolloutWithEvidence(..., '600s')`, which
   `error`s on failure. The caller's `catchError` swallowed it, marked the build UNSTABLE,
   **and the `for (h in ordered)` loop moved straight to the next peer.**
3. Compounding: the stagger soak sat *after* the throw point —
   `def changed = deployHumanConductor(...); if (changed) { sh "sleep ${soakSecs}" }`.
   On failure `changed` was never assigned, so the sleep never ran. **A failing peer
   advanced to the next one faster than a healthy one** — the pacing safety inverted
   exactly when it was needed.

### The proof that the loop does not halt

- `ordered` is built non-genesis first, then genesis-non-matthew, then matthew last
  (`Jenkinsfile:758-761`). From `genesis/orchestrator/data/deployments.json` only adam and
  matthew are `genesisPeer: true`, so restricted to the seven live alpha peers `ordered` is
  exactly **jessica, james, gertrude, susan, eve, adam, matthew**.
- **Direct, in-build proof (#1414).** After jessica's conductor rollout failed, the phase
  attempted james (failed, `elohim-conductor=false/CrashLoopBackOff`), then gertrude
  (failed, `elohim-conductor=false/` on node `shem`), then susan — whose
  `kubectl rollout status` had already printed the same 3× `Waiting for 1 pods to be
  ready...` pattern that preceded every prior timeout. Four peers attempted, three
  confirmed failed, in `ordered` sequence, in one build. That settles the defect on its own
  and is independent of everything below.
- Artifact-level corroboration (#1413): it archived failed-rollout evidence bundles for
  four conductors in one run — `rollout-evidence/elohim-alpha--statefulset--elohim-{jessica,
  gertrude,susan,eve}-alpha-conductor/` — plus `…--deployment--elohim-doorway-alpha-b`.
  #1413 ended ABORTED (superseded — see
  `ci-orchestrator-supersede-aborts-in-flight-edge-rolls.md`), which is the only reason it
  stopped where it did; james/adam/matthew's waits there were cancelled mid-flight, so their
  #1413 outcomes are inconclusive rather than passing.
- Onset is bounded by the builds: **#1412 (06:48–08:29Z) had NO conductor rollout failure at
  all** — every conductor apply reported `configured` and every rollout completed clean;
  only the two doorway Deployments timed out. #1413 carries the first conductor timeouts.
  So the conductor fleet went unhealthy between #1412's end and #1413's conductor phase.
- The escalation atom's independently measured per-node restart sequence (operator,
  on-cluster) is **jessica 09:28 → james 09:38 → gertrude 09:49 → susan 09:59 → eve 10:09
  → adam 10:19 → matthew 10:28** — the same order as `ordered`, at ~10-minute intervals
  equal to the phase's own `--timeout=600s`. That correspondence is why this entry exists.
  See the open question below on how far it can be pushed.
- **Every subsequent edge build re-enters the loop** — that is the recurrence shape.

### What the loop costs, in both readings

Whether a given build *rolls* the conductors or merely *waits* on already-broken ones, the
no-halt behaviour costs up to `600s × remaining peers`. #1414 is the pure-witness case and
had already spent **40.7 minutes** in "Deploy Edge Node - Alpha" without reaching eve, adam
or matthew. So the halt is worth landing on wall-clock grounds alone, before any
blast-radius argument.

So the ~10-minute cadence is, at minimum, this pipeline walking its own peer list and paying
one rollout timeout per node. **How much of the fleet-wide damage the loop caused rather
than merely witnessed is NOT settled** — see open question 1. The claim this entry stands
on is the narrow one: the loop does not stop, and with a halt each build's attempt surface
would have been one peer instead of the whole roster.

## Current decision

**Mitigation landed in-tree; the fingerprints stay BLOCKED on an operator move.** The
ledger entries are `blocked`, not `triaged`, on purpose: the propagation is fixed but the
*occurrence* is not, so these three will recur on the next edge roll. Marking them
`triaged` would tell the stasis sweep the fix failed when it in fact never addressed
occurrence. **What unblocks them:** the operator recovery on the escalation atom's
decision 4 (destructive, per-node `databases/conductor/` clear, after the DNA-hash pin
`0927123e7` rolls), plus decisions 2–3 (`happ_manager` must not read a standing
`ALLOW_DNA_REINSTALL=true` as migration intent; the supervisor must distinguish a dead
child from a slow one) — both in flight in the working tree at time of writing and owned
elsewhere, deliberately untouched here.

The rung-3 phase now halts on the first
conductor rollout failure; skipped peers keep their RUNNING conductor (the safe state) and
are named in the log. The build still goes UNSTABLE, so the CI signal is unchanged. An
operator escape hatch — `CONDUCTOR_ROLL_CONTINUE_ON_FAILURE=1` — preserves the old
behaviour deliberately rather than by default.

The halt bounds the damage of each recurrence to one peer; it does not prevent the red.

Deliberately **not** changed: deploy ordering, timeouts, restart budgets, manifests, the
storage phase's parallel per-peer posture, or the junit shape.

## Fix trail

- `elohim/holochain/Jenkinsfile` — rung-3 staggered conductor phase: pessimistic
  `rollState.halted` marker (map mutation, matching the function's existing
  capture-by-reference idiom for `outcomes`), cleared only on a clean return from
  `deployHumanConductor`; remaining peers skipped with an explicit log line;
  `CONDUCTOR_ROLL_CONTINUE_ON_FAILURE=1` escape hatch. ~30 lines, one file.
- Local verification (no build can be triggered from here — Jenkins MCP is anonymous):
  `genesis/orchestrator/scripts/check-jenkinsfile-method-size.sh` → 0 failed, and the
  edge `pipeline{}` block is **byte-identical at 62328** (the edit lives in a top-level
  helper, outside the CPS dispatch method — no movement toward the 64KB ceiling);
  `node --test jenkinsfile-cps-scope.test.mjs validate-only-pipeline.test.mjs
  runtime-config-render.test.mjs` → 20/20 pass.

## Open, recorded rather than guessed

1. **Did the conductor phase ROLL these pods, or only wait on them?** For **#1414 the answer
   is: only waited.** Its changeset touches nothing under
   `genesis/orchestrator/manifests/humans/`, `elohim/holochain/dna/`, `elohim/holochain-conductor`
   or `elohim/tx5`; the console prints
   `conductor image …: holding harbor…/elohim-storage-iroh:1.0.0-dev-4a81a749` (only the
   *storage* image moved, to `…-7513654f`); and `_edgenode-conductor.template.yaml:72-77`
   deliberately keeps the git-SHA-varying `app.kubernetes.io/version` on OBJECT metadata,
   never on `.spec.template`, so an ordinary commit yields `configured` with **no new
   revision and no restart**. #1414 is therefore a witness, not a cause.
   **#1413 is the open half.** The atom's per-node restart times fall inside #1413's window
   and match `ordered`, and a moved hApp digest *is* a documented conductor-roll trigger
   ("hApp bytes must reach the conductors"), which the wave-4 integrity-hash move would have
   fired — but nobody has read #1413's own `conductor image …:` lines to confirm it rolled
   rather than waited. Until someone does, treat "the loop caused the fleet-wide tear" as
   **unproven** and "the loop never halts" as proven. Cheapest next read: grep #1413's
   console for `conductor image` and for `ROLL requested` / `FIRST ROLLOUT` / `holding`.
2. `gate-runner.mjs --changed-file-list --names` prints *no* project for
   `elohim/holochain/Jenkinsfile` — this file has no local gate at all, so the CPS lint and
   the orchestrator node tests above are the whole of its pre-push coverage (museum
   #13/#17 family: a gate whose trigger does not cover the file the author edits).
3. Whether the conductor `previous.log` tails inside #1413's archived bundles carry the
   `CellWithoutGenesis` panic the atom names. They are fetchable now (#1413 is finished) and
   would tie the CI evidence to the substrate diagnosis without any cluster access.

**Retracted 2026-09-02, same day, before anyone acted on it:** an earlier revision of this
entry claimed #1414 archived no `rollout-evidence` bundle and called that an unexplained
measurement gap. It is not a gap — `archiveArtifacts` lives in the pipeline-level
`post { always { … } }`, and #1414 had not finished its deploy stage, so `post` had not run.
The absence was simply a still-running build.

## Done when

An `elohim-edge/dev` build whose conductor phase hits a rollout failure logs
`conductor phase: HALTED` and rolls **no** further peer; and these three fingerprints stop
recurring for ≥3 consecutive builds once the alpha conductors are recovered.
