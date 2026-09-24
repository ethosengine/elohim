---
epr-habit-version: 1
id: push-delivers-within-budget
invariant: >
  A push that plans work reaches every pipeline it planned, each finishing SUCCESS or UNSTABLE.
  At least 8 of the last 10 work-bearing orchestrator runs deliver, and the p90 wall clock of
  the delivered runs stays within 240 minutes. No pipeline is cut off by another pipeline's
  clock, and a push that changed nothing deployed does not roll the fleet.
status: red
active: true
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
DELTA 2026-09-24b (RED preserved): Lane A landed the readiness precondition + timed phases (b6f256b9a 5d44375b6; faces abd014123), Lane B run classes (58b6b5456 + rakia feat/step-class b081e9b, pin bump on pin/lane-B), B5 advisory guard (ac1e3a6cd), Lane D priced the stage: delivery-series --stages reads publish+verify p90 131.9 min against bound 20, 16.5 pipeline-h for 0 delivered (f8512d40f, NaN fix 9040b0eda); app #1726 failed like #1725. Live reading owed on the first push.
DELTA 2026-09-24: ACTIVE for the native-delivery sprint (plan genesis/docs/superpowers/plans/2026-09-24-native-delivery-sprint-plan.md) — app #1719–#1725 ≈12 pipeline-h, 0 delivered; the 7200 s readiness wait (a2d1d0975) is the app's cost; Lanes A/B/D/K serve this habit. Status stays RED until delivery-series reads within bounds.

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
DELTA 2026-09-23b (RED preserved; the budget held, delivery did not): app #1725 (from the 98ea2857b push) ran 2h09m — inside its own 180-minute budget — and finished FAILURE: the blob leg was green on the first offer (`forwarded_to_storage:true`, the 23a cure confirmed), the readiness deadline of 7200 s was then exhausted on one face, `catching-up` (matthew's storage shedding writes because its content cell stayed CellDisabled 3h20m after the edge roll's conductor restart), every host was left STALE and SSR delivery was refused; orchestrator #1897 closed FAILURE at 5h43m. The doorbell/RTT batch pushed at 00:40Z as b6f16b660 (#1898) rolls the edge again — that roll is matthew's second restart, so #1898's app leg is the next reading of this habit and of whether a second restart clears a cell stuck this long.
DELTA 2026-09-24c (RED preserved; the wait is re-aimed, not shortened): authorHeadOnce now asks each doorway's /health/serving before offering the head and iterates the AUTHOR loop serving-hosts-first (scripts/ci/doorway-serving-order.sh: shedding == false AND storageServing.status == serving; exit 0 always; the DECLARE_ONLY fan-out and verifyProjectedHeads keep the canonical list; one `author order` line per bundle in the log). Evidence, app #1725 (2026-09-23): the readiness wait sat on matthew (alpha.elohim.host) from 21:40:08Z to ~23:41:58Z while adam (elohim.host) had logged "apps enabled" at 22:48:27Z — about 53 minutes waiting on a host that could not author while another could. The 7200 s deadline itself is untouched; what changes is which host is offered it first. The next app leg after this lands is the reading: its author-order lines name which doorway was offered first and why.

CORRECTION 2026-09-24 (appended after the review of fb64ee72d; the 24c delta above is left as written). Two claims in 24c outran their evidence. (1) "a host that could not author while another could": nothing measured that adam could author. The cited datum is a conductor "apps enabled" log line at 22:48:27Z — not a /health/serving reading, not an authored head. The run's ONE measured reading of adam's author path is the fail-over itself (app #1725 console lines 6802-6815, ~23:42Z): the blob PUT to elohim.host was confirmed (forwarded_to_storage:true, peer-judged boots), then the head re-offer's readiness probe answered HTTP 503 catching-up (cause upstream, circuit closed, errorStreak 1 — the doorway's DEGRADING arm) and the run-scoped deadline, already 7219 s past its 21:40:08Z stamp, was reached at once; every bundle then read NO doorway could author. So adam refused the one PATCH it was offered, and whether it would have accepted one with a fresh clock at 22:48Z is UNMEASURED. What #1725 does show is narrower: the deadline was spent on one host while the other was never asked while there was time. (2) The predicate 24c names — shedding == false AND storageServing.status == serving — read 2 of the 5 arms of the doorway's own /health/serving 503 (routes/health.rs build_serving_response: shedding, degrading, rolesDiscovered == 0, warmupEmpty, storage refused/unreachable) and discarded the status code; adam's ~23:42Z answer above (circuit closed, errorStreak 1) is exactly the shape it would have called SERVING. doorway-serving-order.sh now requires HTTP 200 — the doorway's own verdict — AND the two body fields (which are stricter than the status for not-observed/not-configured), and its diagnostic prints the status and all five fields, so the author-order lines in the next app leg carry the doorway's verdict, not a subset of it. Still RED; the reading is unchanged: the next app leg's author-order lines.
