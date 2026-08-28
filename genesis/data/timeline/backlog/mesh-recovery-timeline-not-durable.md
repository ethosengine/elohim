---
id: "backlog-mesh-recovery-timeline-not-durable"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Local recovery-timeline JSONL lives under $MESH_DIR (/tmp) and dies with the container — the a2o reports were moved in-repo for exactly this defect"
slug: "mesh-recovery-timeline-not-durable"
written: "2026-08-25"
author: "agentic-developer (shift 2026-08-25T0210-land-608a1ceff-iroh-dual-fleet)"
status: "done"
priority: "medium"
area: "mesh"
domain: "code"
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "habit:measure-honesty-local"
tags: [mesh, recovery, evidence, durability, container-restart, code-domain]
---

# Recovery measurements are the least durable evidence the household lane produces

`hc-mesh-recovery.sh` appends one JSONL row per run to `${MESH_DIR:-/tmp/elohim-local-mesh}/recovery-timeline.jsonl`
(plus `.live-proofs.jsonl`, `.spotcheck.jsonl`), and `genesis/scripts/recovery-timeline.py` reads it
back for the before/after deltas that `habits.yaml` cites. On 2026-08-25 ~00:48Z the devspace
container restarted; `/tmp` was wiped and the 13-row live series (warm/cold × homo-dual /
split-libp2p-iroh / harness-verify, 2026-08-24) vanished. Only the 8-row a2o fixture
(`genesis/a2o/scripts/__tests__/fixtures/recovery-timeline.jsonl`) survives in-repo. This is the same
defect the a2o sprint reports were moved out of `/tmp` for (spec
2026-08-22 verification-as-memoized-derivation, §S1/§S4: the lane with the highest discovery value
produced the least durable evidence).

Fix shape: write the recovery JSONL (and the quiesce records `hc-mesh-quiesce.sh` keeps beside it)
under `genesis/a2o/reports/recovery/` (gitignored like the sprint reports, but on the persistent
volume), keep `$MESH_DIR` as a symlink or secondary copy for the scripts that read it by that path,
and have `recovery-timeline.py` default to the in-repo location. `just mesh recovery-matrix` and
the transport self-awareness spec's before/after comparison then survive a restart.

## Cured (2026-08-28, M0 shift)

`hc-mesh-recovery.sh` writes `genesis/a2o/reports/recovery/recovery-timeline.jsonl` (gitignored, persistent
volume) and leaves `$MESH_DIR/recovery-timeline.jsonl` as a symlink to it (a legacy `/tmp` file is migrated
once); `hc-mesh-recovery-matrix.sh` reads the same path; `hc-mesh-quiesce.sh` does the same for
`quiesce-log.txt`; `recovery-timeline.py` defaults to the durable file (`RECOVERY_TIMELINE` overrides).
First durable record: warm jessica←matthew, libp2p, 62 s. Lane R rung R3.
