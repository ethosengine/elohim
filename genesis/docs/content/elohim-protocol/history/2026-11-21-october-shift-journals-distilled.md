---
title: "History: the October 2026 shift journals, distilled"
id: october-shift-journals-distilled
type: history-gotcha
status: Accepted
tier: history
created: 2026-11-21
topic: [shifts, landing, ci, measurement, worktree, household-mesh]
# This record DISTILLS sixteen agentic-developer shift journals (2026-10-03 .. 2026-10-31) that sat
# in .claude/shifts/ past the ~14-day working-state budget. That directory is gitignored by design
# (.claude/shifts/.epr-meta: "working state, never canon"), so the bodies were never in git and are
# not in the MemPalace shifts wing: this record and the homes it names are what survives.
distills:
  - .claude/shifts/2026-10-03T03-40-land-fork-ab31ecf2c-pin
  - .claude/shifts/2026-10-05T04-47-event-driven-head-delivery
  - .claude/shifts/2026-10-05T20-05-conductor-roll-wasm-retention
  - .claude/shifts/2026-10-07T02-45-land-release-retention-on-alpha
  - .claude/shifts/2026-10-09T06-30-land-gradient-sprint-on-dev
  - .claude/shifts/2026-10-10T01-50-steward-exile-probe
  - .claude/shifts/2026-10-12T04-10-roll-scheduler-latch-pin-edge-green
  - .claude/shifts/2026-10-13T04-00-idle-call-floor-matthew-and-gossip-round
  - .claude/shifts/2026-10-14T03-25-consent-demand-and-pin-backoff-landing
  - .claude/shifts/2026-10-15T04-15-integrate-1015-land-and-deploy
  - .claude/shifts/2026-10-16T03-40-commons-spaces-land-and-probe-b
  - .claude/shifts/2026-10-19T03-25-finish-integration-2026-10-18
  - .claude/shifts/2026-10-20T01-30-ci-runner-story-green
  - .claude/shifts/2026-10-23T15-20-integration-shakeout
  - .claude/shifts/2026-10-25T03-00-land-gate-evidence-reuse
  - .claude/shifts/2026-10-27T03-00-overnight-integration-pass
  # appended 2026-11-26 (stasis round 1): four journals the first pass missed
  - .claude/shifts/2026-10-24T-land-workspaces-cooperate-green
  - .claude/shifts/2026-10-26T04-35-land-slice-m-household-receipt
  - .claude/shifts/2026-10-27T03-10-devices-converse
  - .claude/shifts/2026-10-29T03-55-closure-keyed-edge-build
# Bidirectional: the canonical surface this record points back to (and which points here).
canonical:
  - genesis/docs/shifts/JOURNAL-TEMPLATE.md     # where every "Observed anti-patterns" table is born
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
memory_anchors:
  - integration-push-from-fresh-worktree
  - resumed-shift-check-for-stale-instance
  - read-the-pipelines-own-verdict
  - pre-push-serving-receipt-leg
  - decoy-git-env-paths-get-created
  - intentional-break-carries-its-why
---

# History: the October 2026 shift journals, distilled

> **Hot-context pointer (the one sentence to remember):**
> **Prove the judge before you trust the measure, and prepare the landing tree before the first push.**
> Across sixteen October shifts the costliest misreadings were the shift's *own* instruments (a measure
> keyed on the wrong build, a `pipefail` pipe that read false on success, a curl glob that never ran),
> and the costliest wall-clock was a fresh worktree that failed hours into a gate for a missing
> `node_modules`, dist or submodule. Everything else the journals found already has a durable home.

**Canonical:** each shift's "Observed anti-patterns" table follows
[`genesis/docs/shifts/JOURNAL-TEMPLATE.md`](../../../shifts/JOURNAL-TEMPLATE.md); check this record and
the [CI/orchestrator museum](2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md) before naming a
new AP. Committed sprint reports live under `genesis/docs/superpowers/sprints/`; the journals' open items
were filed as backlog rows (each verified present on 2026-11-21) under `genesis/data/timeline/backlog/`.

---

## Recurring shapes (the lessons worth re-reading)

### 1. The judge was wrong, not the system (3 shifts)
- **10-05 wasm-retention:** the roll measure read `lastBuild`, which the validate-only sibling displaced
  within minutes; it printed 0 while the console showed 7. *Key a roll measure on the build number.*
- **10-10 steward-exile-probe:** under `pipefail`, `printf "$console" | grep -q` read false when grep exited
  on its first match; the judge was patched mid-shift to here-strings and the patch was disclosed.
- **10-12 roll-scheduler-latch:** the measure script never executed (a curl glob), so the "baseline 0" was an
  artifact; separately a bare-word console witness (`[Ff]ormatter`) refused on a benign tooling notice
  (fixed 70ed2957f). *Run the measure's pieces once by hand at kickoff; anchor error patterns.*

### 2. `pipefail` plus an early-exiting consumer (2 shifts, code-held)
`printf … | head -n1` on a >64 KB report returned 141 (SIGPIPE) and stopped the coordinator-swap driver
at its last peer (DNA #1484; fixed 3f73f504e with a large-report test). The 10-10 `grep -q` judge above is
the same shape. Any `set -o pipefail` script that pipes unbounded output into `head`, `grep -q` or `cut`
can read success as failure.

### 3. The landing tree was not prepared (4 shifts; now a preflight)
10-14, 10-15, 10-16 and 10-27 ("Lesson, again") each lost gate hours to a fresh worktree missing
`pnpm install`, element dists, the full sophia UMD or initialised submodules; a household lane in such a
tree writes an EMPTY-SELECTION receipt and exits 0. Crystallized in memory
`integration-push-from-fresh-worktree` and enforced by `gate-preflight.mjs`.

### 4. Lessons with no other home (one occurrence each, kept here)
- **Never edit a shell script while bash runs it** (10-20): editing `hc-mesh.sh` mid `just mesh build` gave a
  syntax error at the old byte offset.
- **Signal handlers must follow a re-minted bridge** (10-07): after a conductor restart, storage adopted a new
  websocket but its signal handlers stayed on the dead one, so nothing projected until storage restarted
  (fixed 166f7a54c).
- **A `--verify` gate must regenerate, not read a gitignored cache** (10-07; fixed 4e02329d5).
- **A measurement can harm the instrument** (10-10): four-day Loki regexp queries OOMed Loki; use
  three-hour windows (dataplane-convergence DELTA 10-10b).
- **Capture a receipt's SUT identity at launch** (10-15): a report written after a mid-lane merge carried the
  post-merge identity for older bytes. Never edit the tree while a receipt lane runs.
- **Update a cache only after the awaited write returns** (10-13, four Codex rounds on slice-hash and
  peer-meta caches).
- **A household pass is not a fleet pass when boot paths differ** (10-05): the standing coordinator pass
  proven on an embedded-conductor household was inert on alpha's external conductors.
- **TaskStop on a backgrounded `git push` leaves its pre-push gate children running** (10-16).

Already crystallized elsewhere, so not repeated here: two instances after a session refresh
(`resumed-shift-check-for-stale-instance`), a 4 h gate losing the `dev` ref race
(`arch-workspace-discipline-backlog`), `[skip ci]` plus an empty baseline dropping a range (fixed
f257e1074), `core.worktree` re-pointed through an inherited `GIT_DIR` (`decoy-git-env-paths-get-created`),
a whole-push toolchain export leaking into WASM gates (`manual-cargo-in-this-workspace`), host-green is not
CI-green (`include_str!` target absent from the image stage; backlog
`storage-image-include-targets-unchecked-locally`), and measurement-by-deploy (root `CLAUDE.md`).

---

## What each shift settled (one line, with its durable home)

| Shift | Outcome | Durable home |
|---|---|---|
| 10-03 fork ab31ecf2c pin | pin landed; hosted-app errors stop only the primary app's roll (616768fb0) | backlog gap 3 (CellMissing set) |
| 10-05 event-driven head delivery | household head delivery 50.7 s → 1.2 s; fleet leg owed | `conductor-verified-head-moves-read-unverified` |
| 10-05 conductor roll / wasm retention | 7/7 conductors on pin conductor-3c1e8052504f | dataplane-convergence DELTA 2026-10-05d |
| 10-07 release retention on alpha | `release-retention` GREEN; own-unnamed bytes let go at 64/pass | DELTA 2026-10-07b; `alpha-adam-peer-meta-store-disk-io-error`, `ci-app-full-clone-under-degraded-github-egress` |
| 10-09 gradient sprint on dev | landed and on the fleet; three habit readings | sprint `2026-10-08-gradient-sprint.md` |
| 10-10 steward exile probe | seam 7 in substrate smoke; fork cause unread | `steward-key-chain-fork-warrant-exile`, `ci-seam-smoke-change-dispatches-no-pipeline` |
| 10-12 roll-scheduler latch pin | conductor-96bcd2bee215 on the fleet; validate-only reds were born-red scenarios | sprint `2026-10-11-idle-cpu-residual-sprint.md`; `ci-edge-inventory-convergence-caughtup-sawtooth-flap` |
| 10-13 idle call floor | call leg met; CPU leg red (~21 s/min vs 10) | sprint `2026-10-12-idle-call-floor-sprint.md`; `conductor-gossip-round-op-store-cost` |
| 10-14 consent demand + pin backoff | A landed (household); B parked | `retired-pin-readmit-backoff-parked-after-six-reviews` |
| 10-15 integrate 10-15 | alpha at 89c41e6 (edge #1632) | memory `integration-push-from-fresh-worktree` |
| 10-16 commons spaces + probe B | measure 0 → 4; probe B measured on a scratch fork | spec `2026-10-16-commons-reach-spaces-physical-holon-design.md`; sprint `2026-10-16-commons-spaces-probe-a-report.md` |
| 10-19 finish integration 10-18 | pin, SSR self-heal, plane write-guard landed; hosted provisioning held | memory `shem-wave6-unlanded-after-power-loss` |
| 10-20 CI-runner story green | 4/5 scenarios stable; scenario 2 held on `@requires:compute-executor` | `rakia-executor-untracked-in-submodule-pin`, `alpha-doorways-record-no-hosting-promise`, `hosting-conductor-keystore-listing-overflows-lair-frame` |
| 10-23 integration shakeout | the `[skip ci]` range walked after the baseline fix | `integration-sweep-2026-10-23` |
| 10-25 gate evidence reuse | landed by deliberate bypass, with the WHY at the head (b2130d38b) | backlog `gate-merge-gradient-reading-2026-10-25`; memory `intentional-break-carries-its-why` |
| 10-27 overnight integration pass | trains 2–4 landed; two ports deferred | `local-view-delivery-deferred-steward-closure-conflict`, `chrome-trust-edge-deferred-egress-census-site` |

A seventeenth file, `2026-10-29-continuation.md`, was empty and carried nothing.

### Appended 2026-11-26: four journals the first pass missed

| Shift | Outcome | Lesson (one line) |
|---|---|---|
| 10-24 land workspaces-cooperate green | DONE at #2138 + #2140 | A partial worktree install and absent sophia/rakia submodules read as eleven code reds; prepare the tree first (memory `integration-push-from-fresh-worktree`). |
| 10-26 land slice M household receipt | slice M on dev as ddee90f30; gate reuse went live | A 401 "credential drift" meant the apex fixture human (adam) was never cast; the receipt ran on a separate household fixture with adam joined rather than recasting the canonical one. |
| 10-27 devices converse | first device-to-device code handoff over berth | **Split-generation stranding**: a release declaring a new bin (`bin-launch`) projected before the follow that carried it stranded every release under the old launcher's `verified()`; `epr device arrive` cannot self-cure — bootstrap must use `admissible()` (fix carried by shem as 9d5e27da2). A fold watcher keyed on action filtered `say` rows as cites. |
| 10-29 closure-keyed edge build | stalled on an unsheddable workload; closed 11-01 | **An empty `[build:edge]` commit measures nothing**: the graph-walker derives run class from changed steps, an empty changeset derives `verify`, and edge runs verify as validate-only; waves coalesce per branch head. |
