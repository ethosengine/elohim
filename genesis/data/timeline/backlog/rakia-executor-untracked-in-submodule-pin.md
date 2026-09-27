---
id: "backlog-rakia-executor-untracked-in-submodule-pin"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim/rakia/rakia-executor (the compute-executor binary the delegated-compute leg runs) is UNTRACKED inside the rakia submodule — pin 13cb31d does not contain it, so CI and any fresh clone cannot build the executor the operations guide and worker image depend on"
slug: "rakia-executor-untracked-in-submodule-pin"
written: "2026-09-07"
author: "overnight shift 2026-09-07"
status: "open"
priority: "high"
jobs: [rakia, elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
tags: [rakia, submodule, delegated-compute, ci]
---

**Evidence (2026-09-07 05:4xZ, compute-leg prep):** `git -C elohim/rakia status` shows `rakia-executor/` untracked; the superproject pin is `13cb31dddb…` (heads/main). The binary at `/projects/.cargo-target-pool/family/dev/elohim__rakia/dev/debug/compute-executor` was built from that uncommitted work. `scripts/ci/build-compute-worker.sh` and `genesis/agentic/compute/operations.md` both require it. **Cure (operator-owned, the submodule is theirs):** commit `rakia-executor/` in the rakia repo, push, bump the pin in elohim. **Done when:** a fresh `git submodule update --init elohim/rakia` contains `rakia-executor/Cargo.toml` and `just gate rakia` builds `compute-executor`.

**Also needed (2026-09-08, sprint):** schema widening for `run.cargo.env` — the pinned
`elohim/rakia` submodule's build-manifest schema currently refuses `GateProject.run.cargo.env`
(`/gate/projects/elohim-storage/run/cargo must NOT have additional properties`), which is why a
per-project cargo resource cap (T8's storage gate `CARGO_BUILD_JOBS=1`) had to move to
`genesis/agentic/pool-policy.json`'s `cargo_env_overrides` instead of living on the manifest
project itself. The mirror shape to copy is already in
`genesis/orchestrator/manifest.schema.json:161-166`. See
`genesis/data/timeline/backlog/cargo-test-memory-shed-storage-gate.md` (2026-09-08 amendment)
for the full account.

**Also needed (2026-09-27, rakia leg shift, peer-executed-stage delegation to jessica):** seven follow-ups
from the first real delegated-stage runs, most already recorded as habit DELTAs on
`genesis/agentic/.epr-meta/measure-runs-on-a-peer.habit.md` but not yet backlog rows. Source for all:
`.claude/shifts/rakia-leg-2026-09-27.md`.

1. **`t2-receipt.sh` trusts mtime, not lane/author.** It accepts any `sprint-report-household-*.json` by
   mtime alone; a guest-run peer stage left two nobody(65534)-owned reports in `reports/` that could pass
   as the pre-push T2 receipt. Should require `env.lane=household` + a non-guest author/moored session.
   Shift record lines 108, 125.
2. **Stages should skip the berth explicitly, not error into it.** A guest's inner `just test mesh` prints
   `berth: store unavailable (EACCES /projects/.claude-config/berth/berth.lock) — lane proceeds unleased`;
   fine for a guest, but should be an explicit `BERTH_SKIP` for stages rather than an error line. Lines 36, 126.
3. **Stage spec §5.3 should record the per-feature write-set + the delegability refusal rule** (today
   `FIXTURE_WRITE_SETS` and `assessDelegability` exist in code with no spec-level record). Lines 39, 107, 127.
4. **An a2o tag `@owns-processes`** to replace the heuristic step-code scan (`assessDelegability` greps
   step modules for `fixture.processControl`/`requireProcessControl()`/`/proc/<pid>` reads) — a genesis/a2o
   change. Lines 106, 128.
5. **Real offload value needs a long guest-runnable measure-class lane, or rung A (adam).** Every lane
   delegable today (doorway-fixture-readiness, federation-version-convergence) is 12-15s of guest work
   saving ~0.2 min of dev-berth hold; process-owning lanes (the ~4 min serving-receipt lane) cannot
   delegate to an unprivileged guest at all. Lines 48-49, 119, 129.
6. **Recipe `edge-pipeline@1`'s Deploy stage should list `fleet:probe`; drift needs an observed
   per-stage capability signal, not just the declared `exercises:` binding.** The conductor roll gate
   (`scripts/ci/peer-roll-gate.sh`, curls storage `/p2p/status` + Prometheus) now runs inside "Deploy Edge
   Node - Alpha", but the recipe binds `fleet:probe` only to Dataplane Validation, so `jenkins-bridge drift`
   (`bridges/jenkins/src/drift.rs:186`) cannot see the capability move into an existing stage — stale
   binding, not a hidden capability (offer already discloses `fleet:probe`). Lines 72, 174.
7. **Mark "Quality Gate: Doorway" delegable and let the stage submit a compute task instead of running
   inline** (2702s in edge #1487, and the "gate time is duplicated" externality the offer already names).
   Blocked on a Jenkins-agent→household-provider network path (the offer is `crossesNetwork: false`; adam/
   rung A is operator-gated) and depended on the frame-truncation fix (now landed, round 2 of the shift).
   Lines 89-99.
