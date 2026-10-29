---
id: "backlog-ci-compute-worker-test-reds-when-grant-expires"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "compute-worker.test.mjs 'the declared Adam 24Gi worker fits its current full envelope' turns red on the wall clock: it renders the live adam-compute-worker grant, and validateGrantDeclaration refuses an expired one — the orchestrator test lane went red at the G2 grant's expiry (2026-10-29T04:54:39Z) with no code change"
slug: "ci-compute-worker-test-reds-when-grant-expires"
written: "2026-10-29"
author: "overnight shift 2026-10-29 (closure-keyed-edge-build)"
status: "open"
priority: "high"
jobs: [elohim-orchestrator]
cluster: "arch-workspace-discipline-backlog"
tags: [ci, compute-worker, grant, flaky-by-clock, orchestrator]
---

**Observed 2026-10-29 ~05:10Z** on an untouched `origin/dev` (`e67c6cef8`) checkout and on the main
checkout alike: `node --test genesis/orchestrator/compute-worker.test.mjs` fails one case —

```
✖ the declared Adam 24Gi worker fits its current full envelope
  Error: grant requires fixed, finite, active issuance timestamps
      at insist (genesis/agentic/compute/grant-bootstrap.mjs:9:25)
      at validateGrantDeclaration (genesis/agentic/compute/grant-bootstrap.mjs:37:3)
      at renderEnabled (genesis/orchestrator/scripts/render-compute-worker.mjs:333:5)
```

The test renders the real `adam-compute-worker.json` declaration; `validateGrantDeclaration` refuses a
grant whose validity window has passed. The G2 grant expired at 2026-10-29T04:54:39Z with no
auto-renewal, so the test (and every `just gate orchestrator` / CI lane that runs the directory)
flipped red on the clock. The refusal itself is correct: an enabled worker with an expired grant
must not render. The defect is a test whose verdict depends on wall time against a live grant.

**Cure, two halves.** (1) The test must assert the *shape* (a worker that fits its envelope renders)
against a fixture grant with a fixed, future `validUntil` — or freeze the clock the validator reads —
so the suite cannot red on a date. (2) The live declaration needs a renewed grant through the grant
flow (`genesis/agentic/compute/measure.sh grant` / the provider's issuance), which is a provider act,
not a test edit; until then `adam-compute-worker.json` `enabled: true` describes a worker that cannot
render, and `deployments.json`'s "enabled with a grant" line should say "grant expired".

**Done when:** `node --test genesis/orchestrator/compute-worker.test.mjs` is green on a date past any
grant's expiry, and the declaration's grant status is read, not assumed, by whatever renders it.

**DELTA 2026-10-29 ~15:00Z (overnight shift, closure-keyed-edge-build).** Cured on dev by shem's lanes
push 5 (bf810489c "Renew adam's runner grant for a second seven-day window (steward-approved)" —
Matthew: "renew please"): the grant window is 2026-10-29T13:15:00Z..2026-11-05T13:15:00Z with identical
parties and bounds, the envelope test renders with runnerGrant disabled (off the clock), and
b68e348bd adds a named test. A renderer-degrade arm (omit the worker with a named
`COMPUTE-WORKER-GRANT-EXPIRED` line instead of failing adam's deploy) was drafted on this branch and
dropped before landing so two fixes to the same files would not collide. Still open from this atom:
whether an inactive window should degrade rather than refuse (the operator chose renewal), and that a
grant is a time-bomb for every gate unless its renewal is scheduled before `validUntil` — the next
expiry is 2026-11-05T13:15:00Z.

