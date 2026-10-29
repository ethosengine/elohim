---
title: "Handoff — the closure-keyed landing, after the overnight shift of 2026-10-29"
id: closure-keyed-landing-handoff-20261029
date: 2026-10-29
status: handoff
author: "Claude Fable 5.1 (overnight shift closure-keyed-edge-build), for the operator (steward of record)"
habits: [push-delivers-within-budget, sprint-lands-each-closure-once, push-gate-pays-once, devices-share-one-berth]
cites:
  - genesis/docs/superpowers/specs/2026-10-29-closure-keyed-landing-plane-design.md
  - genesis/research/integration-capacity-grounding-2026-10-29.md
  - genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md
  - genesis/data/timeline/backlog/agent-presence-listens-to-the-space-by-default.md
  - genesis/data/timeline/backlog/ci-compute-worker-test-reds-when-grant-expires.md
---

# Handoff — the closure-keyed landing

**What this work is.** The operator asked, on the evening of 2026-10-28, to start from the limit canon
(limitarianism, carrying capacity, the VSM limits), measure the hard device limits and the modeled
fixture limits, locate the integration bottleneck, and plan how to widen it without adding nodes —
and, as the bonus, to clarify the primitives (epr, epr-rea, eprfs, ark, berth, brit) into a design
and implementation plan in rakia. Then: "go ahead and plan and execute the sprint … treat it as your
overnight /shift."

**Start here.** Paste into a new session on ethosengine:

> Carry on with the closure-keyed landing; read
> genesis/docs/superpowers/sprints/2026-10-29-closure-keyed-landing-handoff.md first, then the design
> §5. Land the two branches in order (gamma onto dev by fast-forward; alpha as the PR's build-only
> wave, then onto dev), read the PR's edge log for `GATE_ONLY — strict cargo quality gates and local
> builds; publish, deploy and dataplane validation disabled`, and write the first DELTA on
> push-delivers-within-budget from the first dev wave whose closures are unchanged.

## What landed where (nothing on dev yet at the time of writing)

| Branch | Commits (origin/dev e67c6cef8 + …) | State |
|---|---|---|
| `shift/2026-10-29-integration-sensors` (lane γ) | c1e0fa48b gate cost vector + ledger union + stage-wallclock fold + berth fold/reap + prose corrections; ee7a54d63 deployments comment (no path token); f4809efab presence backlog atom; bd6fec8f2 revert of the grant-fix (see below) | `just gate orchestrator` 202/202, evidence recorded; push to dev REFUSED by the berth — shem's device holds `dev-integration` (its "lanes" push 5, live) |
| `shift/2026-10-29-integration-capacity` (lane α, on top of γ) | design spec; grounding record; mint-pass rows 54–56; the fork's native-dev-tooling plan; ccb9143ab closure-keyed edge build (adversarial review findings 1–7 fixed); the fixture-scratch fix | all static gates green locally (orchestrator 202/202 before the revert; library 39/39; storybook; cps-scope; 27-case shell suite; rakia validator 24/24; method-size hook: edge pipeline 50,685 B / model 3,400, 19 stages, 18 when); the branch push waits on shem's renewal landing (below) |

**The one thing both branches wait on.** adam's compute-worker `runnerGrant` expired at
2026-10-29T04:54:39Z, which reds `compute-worker.test.mjs` in every orchestrator gate and would fail the
next dev edge deploy at adam's render (`elohim/holochain/Jenkinsfile:1992`). I first landed a renderer
degrade (omit the worker with a named line) on γ; shem's session then said on the channel that its
lanes push 5 (bf810489c) carries the grant RENEWAL the operator approved ("renew please": window
2026-10-29T13:15Z..2026-11-05T13:15Z) plus an off-clock test — so I reverted mine (bd6fec8f2) to avoid
two fixes to the same files. Until shem lands, any push of these branches fails that one test.
**Sequence:** wait for `berth status` → `dev-integration free` → `git fetch` → rebase γ onto origin/dev →
`GATE_EVIDENCE=reuse git push origin HEAD:dev` from the γ worktree → rebase α onto the new dev →
push the α branch → `gh pr create --base dev` (body: /var/tmp/shift-20261029/alpha/pr-body.md, or the
design §5) → read the PR edge log → land α on dev by fast-forward.

## Fresh-worktree prerequisites (cost me three push attempts)

- `git submodule update --init --depth 1 elohim/rakia elohim/brit sophia` — the registry tests load
  every build-manifest.json.
- `pnpm --filter @elohim/storage-client build` — the elohim-library gate imports it.
- `pnpm --filter "./app/elohim-elements/**" build` — the storybook gate imports `elohim-core/register`
  and `elohim-imagodei/register`.
- A hermetic test that scratches inside the tree (the closure fixture did, now fixed) poisons
  graph-walker's manifest discovery when its cleanup is interrupted; `scripts/ci/.closure-test.*` or
  `genesis/orchestrator/.alpha3-test.*` left behind = 13 selection-test reds.
- The hook ledger `.claude/data/governance-findings.jsonl` goes dirty during a gate; `git checkout --`
  it before a rebase.
- A CI-script or Jenkinsfile edit re-keys every Rust gate (storage, eprfs, doorway, steward-node): a push
  of lane α runs for an hour or more; set `GATE_EVIDENCE=reuse` only after one full green run.

## Decisions taken on standing evidence (journal D-1 … D-9)

D-1 scope = sprint-1 α + γ; lane β (Phase V/L hook split) deferred to the morning as governance-class.
D-4 keep `nerdctl build`, closure as image-config labels read through the config blob (annotations do
not survive `nerdctl push`). D-5 gate-only keeps local builds so a PR exercises derivation + oracle +
builder before dev. D-6/D-7 the expired grant: degrade landed, then reverted for shem's renewal.
D-8 build the worktree prerequisites instead of bypassing the hook. D-9 yield the lease queue to shem,
say so on the channel.

## Operator items (host/cluster/human)

1. Four shem `elohim-node` storage containers were crash-looping at probe time (design §10 row 9). **Cause found by shem's session, 2026-10-29 ~17:40Z:** adam's `storage-data` PVC (20Gi, `shem-zfs`) hit 100% on 10-28 ~18:41Z from superseded sled blobs (sync apply rewrites the doc every minute); `elohim-node` crash-loops on EDQUOT since 21:16Z, so elohim.host answers 503; gertrude/eve/susan on shem show the same; edge #1757's 80Gi grow was refused (storage class not expandable). **Operator:** raise the ZFS quota (`zfs set refquota=… quota=none` on the k8s datasets, per the 2026-10-08 entry) and recycle the pods; then the superseded-blob retention is a storage-side defect to file.
2. The DIMM: 2,044 uncorrectable ECC errors on `mc#0 csrow#3 channel#0` at 2026-10-28 ~19:52Z; swap
   still on the worn QLC md1 (design §10 row 10).
3. The jenkins offer's standing (`jenkins-bridge offer status`) — S2d′'s `observe` refuses while Proposed.
4. Two unmoored residents in this workspace (T3 join-alpha conductor 4.6 GB @128 % CPU; campaign-1.4
   seed-row storage 7.5 GB) — left running; moor or stop.
5. The memory entry `feedback_master_peer_compute_before_hardware.md` awaits the standing human's claim
   (the attribution hook names the command).
6. The operator's presence direction is recorded as a backlog atom; its slice (listen by default, collision
   reach from the covenant write-set) wants a design ruling before implementation.

## Measure and the judge (frozen at kickoff)

`node /projects/elohim/.claude/shifts/measure-edge-build-stages.mjs` — minutes of `Quality Gate: Doorway`
+ `Build Doorway` + `Build Storage` on the newest completed elohim-edge/dev build that executed them.
Baseline 50.6 (edge #1753). Target ≤ 15, two consecutive, one from a fresh trigger. No measurement moved
tonight: nothing reached dev.
