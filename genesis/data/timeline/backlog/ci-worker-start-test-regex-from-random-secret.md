---
id: "backlog-ci-worker-start-test-regex-from-random-secret"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "worker-start.test.mjs 'wrapper exposes only the no-cast health feature…' builds a RegExp from a random base64 secret; a secret starting with '+' (or holding '++', '*', '?') throws 'Nothing to repeat' and reds the orchestrator gate with no code change"
slug: "ci-worker-start-test-regex-from-random-secret"
written: "2026-11-08"
author: "artifact-store sprint 2 orchestrator (claude-opus-5-5, shem)"
status: "resolved"
priority: "medium"
jobs: [elohim-orchestrator]
cluster: "arch-workspace-discipline-backlog"
tags: [ci, flaky-by-randomness, orchestrator, compute-worker]
---

**Observed 2026-11-08 12:31Z** in `just gate orchestrator` on `artifact-store/land-f2` (79acec4eb),
a closure that touches nothing under `genesis/agentic/compute/`:

```
✖ wrapper exposes only the no-cast health feature and restores the same owned topology
  SyntaxError: Invalid regular expression: /+EAmQ/k3FArckYOakBNzBCfvRZ6y+Ry9QMaMma+0GAc=/: Nothing to repeat
      at TestContext.<anonymous> (genesis/agentic/compute/worker-start.test.mjs:588:39)
```

`worker-start.test.mjs:588-589` asserts the generated `mesh-api-key-admin` and
`mesh-compute-local-token` secrets never reach stdout/stderr with
`assert.doesNotMatch(run.stdout, new RegExp(secret))`. The secret is random base64, whose alphabet
includes `+` and `/`; a leading `+` (or `++`) is not a valid pattern, so the assertion throws instead
of testing. Roughly one fixture draw in tens reds the gate, and the run spent 49 more minutes after
the red (evidence: `genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/gate-L4-land-f2.log`).

**Cure.** Assert by substring, never by pattern: `assert.ok(!run.stdout.includes(secret))` (same for
stderr). The same shape elsewhere: `grep -rn 'new RegExp(secret' genesis/` before closing.

**Recurred 2026-11-08 ~14:10Z** in the L3 combined gate on `artifact-store/h` (86e1395e0): the secret
held `M++OF` → `Invalid regular expression … Nothing to repeat` at `worker-start.test.mjs:588`. The
orchestrator rerun alone was green (`gate-L3-orchestrator-rerun.log`, EXIT=0). Two reds in one day.

**Resolved 2026-11-08 by ethosengine 39e897c1b** (on origin/dev): the test asserts that a secret never leaks by literal containment (`includes`), not by a RegExp built from the random secret.
