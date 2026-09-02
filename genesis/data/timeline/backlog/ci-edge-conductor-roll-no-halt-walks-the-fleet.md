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

Three DEPLOYMENT fingerprints landed from `elohim-edge/dev` #1414 (UNSTABLE, started
2026-09-02T10:27:44Z), each seen 1×, `first_build == last_build == 1414`:

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

### The proof that this is what happened, not a theory

- `ordered` is built non-genesis first, then genesis-non-matthew, then matthew last
  (`Jenkinsfile:758-761`). From `genesis/orchestrator/data/deployments.json` only adam and
  matthew are `genesisPeer: true`, so restricted to the seven live alpha peers `ordered` is
  exactly **jessica, james, gertrude, susan, eve, adam, matthew**.
- The escalation atom's independently measured per-node restart sequence (operator,
  on-cluster) is **jessica 09:28 → james 09:38 → gertrude 09:49 → susan 09:59 → eve 10:09
  → adam 10:19 → matthew 10:28** — the same order, at ~10-minute intervals. The interval
  is the phase's own `--timeout=600s`, spent once per peer.
- Artifact-level confirmation: **#1413 archived failed-rollout evidence bundles for four
  conductors in one run** — `rollout-evidence/elohim-alpha--statefulset--elohim-{jessica,
  gertrude,susan,eve}-alpha-conductor/` — plus `…--deployment--elohim-doorway-alpha-b`. Four
  consecutive conductor rollout failures, and the loop kept going every time. #1413 ended
  ABORTED (superseded — see `ci-orchestrator-supersede-aborts-in-flight-edge-rolls.md`),
  which is the only reason it stopped where it did.
- #1414 then re-entered the same loop at jessica (first in `ordered`), found it already
  CrashLoopBackOff, and produced these three fingerprints. **Every subsequent edge build
  re-enters the roll** — that is the recurrence shape.

So the ~10-minute cadence of the fleet-wide destruction was not ambient; it was this
pipeline walking its own peer list, paying one rollout timeout per node, rolling six
healthy conductors into a state whose first instance had already demonstrably crash-looped.
With a halt, the cost would have been jessica alone.

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

1. **#1414 archived no rollout-evidence bundle at all** — only `build.env` — although its log
   carries both the collector's readiness summary and the `expected artifact:` line, and
   #1413 archived the full set from the same `post { always { archiveArtifacts … } }`. So
   the first-natural-failed-rollout receipt that `task-rollout-evidence-capture.md` is
   waiting on is satisfied by **#1413**, not #1414, and the #1414 gap is unexplained. Do
   not close that atom's DoD on #1414.
2. `gate-runner.mjs --changed-file-list --names` prints *no* project for
   `elohim/holochain/Jenkinsfile` — this file has no local gate at all, so the CPS lint and
   the orchestrator node tests above are the whole of its pre-push coverage (museum
   #13/#17 family: a gate whose trigger does not cover the file the author edits).
3. Whether james failed its rollout in #1413 (it is in `ordered` and restarted at 09:38 per
   the atom, but has no evidence bundle) or briefly reported Ready before crash-looping.

## Done when

An `elohim-edge/dev` build whose conductor phase hits a rollout failure logs
`conductor phase: HALTED` and rolls **no** further peer; and these three fingerprints stop
recurring for ≥3 consecutive builds once the alpha conductors are recovered.
