---
epr-habit-version: 1
id: push-delivers-within-budget
invariant: >
  A push that plans work reaches every pipeline it planned, each finishing SUCCESS or UNSTABLE.
  At least 8 of the last 10 work-bearing orchestrator runs deliver, and the p90 wall clock of
  the delivered runs stays within 240 minutes. No pipeline is cut off by another pipeline's
  clock, and a push that changed nothing deployed does not roll the fleet.
status: red
active: false
checks:
  - "live series: `node genesis/orchestrator/delivery-series.mjs --window 10` reads each orchestrator run's archived actual-build-graph.json. Exit 0 within bounds, 1 outside, 2 not enough evidence (Jenkins unreadable or fewer than 3 work-bearing runs are never read as health)."
  - "budget arithmetic: `node --test genesis/orchestrator/pipeline-budget.test.mjs` checks that every dispatchable pipeline owns an options { timeout }, and that the orchestrator's limit covers the longest dependency chain of them."
  - "measurement liveness: `bash scripts/ci/fleet-quiesce-gate.test.sh` checks that an oversized /metrics body is still measured, and that a broken evaluator ends as GATE-DEFECT instead of polling blind."
  - "no-op rolls: `node --test genesis/orchestrator/validate-only-pipeline.test.mjs` checks that an edge selection with only dataplane-validation stale dispatches validate-only."
  - "ledger: `.claude/scripts/ci-harvest.py` files a BUDGET_EXHAUSTED finding for any build its own timeout killed (`python3 .claude/scripts/_lib/__tests__/ci_harvest_budget_exhausted_test.py`)."
refs:
  - "genesis/data/timeline/backlog/edge-quiesce-gate-timeout-aborts.md (the budget arithmetic, one layer down then up)"
  - "genesis/data/timeline/backlog/ci-edge-a2o-steps-glob-redeploys-fleet.md (no-op fleet rolls)"
  - "genesis/data/timeline/backlog/fleet-standing-celldisabled-one-third-party-cell.md (why every roll costs hours)"
  - "genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (change-class delivery vehicles)"
retire-when: >
  upgrades propagate peer to peer by change class without a CI fleet roll, so a push's delivery
  no longer waits on a serial pipeline chain. The orchestrator then only builds and publishes,
  and its wall clock stops being the delivery path this habit watches.
---
RED written 2026-09-22 on a live reading. `delivery-series.mjs --window 10` delivered 1 of 10 work-bearing
runs (#1883-#1892). Five of them timed out. The last delivered run was #1883 (190.4 min, edge + genesis).
No coupled push has reached app since 09-14, when app was ordered after edge inside a flat 240-minute budget.
Also, the Dataplane Validation quiesce gate had been measuring nothing for 45 minutes per edge run: its
/metrics body exceeded the 128 KiB env limit.

Branch `ci/wallclock-2026-09-22` (not yet on dev) carries the repairs:
- the gate reads bodies from files;
- each pipeline owns a budget, and the orchestrator's limit is the chain sum (780);
- an a2o-only change dispatches validate-only;
- ci-harvest files timeout aborts.

Stays RED until the live series reads within bounds after they land.

DELTA 2026-09-23a (RED preserved; the repairs ship): the four repairs above plus the content-keyed roll (a peer restarts only when what it consumes changed) and the per-level baseline checkpoint (328efba7b, already on dev) go to dev in one push from a clean worktree off origin/dev, together with the blob-PUT off-path cure. Landing-batch gate on the batch itself: doorway 1631/0, elohim-storage 4903/0 across 17 suites, orchestrator + budget + validate-only suites green, 10 Jenkinsfiles lint-clean, elohim-app 227 test files. Predicted dispatch (graph-walker): elohim, elohim-edge, elohim-holochain, elohim-orchestrator — the first coupled chain under per-pipeline budgets; the first edge deploy after this rolls storage once to record its input digest. Stays RED until `delivery-series.mjs --window 10` reads within bounds on live runs.
