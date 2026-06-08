---
id: "backlog-ci-genesis-pin-step-cucumber-parse-abort"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis E2E aborts whole-suite — unescaped / in acquisition-pin cucumber expressions (museum trap #7)"
slug: "ci-genesis-pin-step-cucumber-parse-abort"
written: "2026-06-08"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [8b5167c0c57c]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, elohim-genesis, e2e, cucumber, gherkin-parse-abort, museum-trap-7, test-code-fix]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1110/
  - genesis/a2o/steps/delivery/acquisition-pins.steps.ts
  - genesis/a2o/features/delivery/acquisition-pins.feature
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
  - genesis/a2o/CLAUDE.md
---

# Genesis E2E parse-abort — unescaped `/` in the pin step expressions

## The failure

Harvested fingerprint `8b5167c0c57c` (genesis #1110, UNSTABLE, seen 1, first/last
build 1110) flagged this line:

```
⏳ waiting for doorway rollout to serve 200 (conductor-connected) at https://doorway-alpha.elohim.host/health ...
```

**That flagged line is a red herring** — the *very next* log line (genesis #1110
line 2509) is `✅ doorway serving 200 + conductor-connected at public /health
(stable ×2 after ~80s)`. The doorway rollout SUCCEEDED. The "⏳ waiting" echo is a
normal progress line the harvester's classifier mistook for a failure signature.

The genuine cause of the UNSTABLE is in **both** E2E stages (API at line 2612,
BROWSER at line 2650):

```
Error: This Cucumber Expression has a problem at column 30:
I POST a pin for {string} to /api/v1/pins
                             ^
Alternative may not be empty.
If you did not mean to use an alternative you can use '\/' to escape the '/'
```

## Verdict

**real — test-code bug, museum trap #7 (Cucumber/Gherkin parse aborts the WHOLE
E2E run).** Not a flake, not infra. The unescaped `/` in a cucumber expression is
read as an empty alternation; the step-file load throws → both E2E stages abort
→ the whole feature glob is lost → UNSTABLE with a blank cucumber report. This is
exactly the museum trap #7 row AND the `genesis/a2o/CLAUDE.md` watch-out ("A
gherkin parse error aborts the WHOLE E2E run, not one scenario … READ THE RAW E2E
LOG FIRST"). Citing the museum, not re-deriving.

## Root cause

Commit `0b12eecc6` ("test(acquisition): a2o pin scenarios + two-node
byte-arrival e2e (spec §11)") introduced three new step expressions in
`genesis/a2o/steps/delivery/acquisition-pins.steps.ts` whose literal text
contains unescaped `/` (`@cucumber/cucumber-expressions@18.0.1` treats `/` as the
alternation operator):

- `'I POST a pin for {string} to /api/v1/pins'` (col 30 — the CI error)
- `'GET /api/v1/pins lists one active pin for {string}'`
- `'I POST a pin with kind {string} for {string} to /api/v1/pins'`

The first throws on load; the other two would throw the same way once the first
is fixed (all three are the SAME concern — one commit, one root symptom). Verified
locally that the unescaped form fails at column 30 (matching CI) and the escaped
form parses cleanly under the same library version.

## Current decision

**FIXED — landed, awaiting disappearance.** Bounded test-code fix per the museum's
own prescribed escape (`\/`). The fix surface is the step file only; the feature
file lines (`acquisition-pins.feature:17,22`) stay literal `/api/v1/pins` because
an escaped `\/` in the expression matches a literal `/` in the gherkin text.

This concern carries no NEW museum-worthy lesson — it is a clean instance of the
already-recorded trap #7, and the `gherkin-prepush-lint` follow-up that would
catch it pre-push already exists as a named backlog item in the museum record and
`a2o/CLAUDE.md`. So `decompose_on_confirm: true`: once the genesis green streak
confirms the parse error is gone, the harvester decomposes ledger line + this
backlog entry automatically (no graduation needed).

## Fix trail

- `genesis/a2o/steps/delivery/acquisition-pins.steps.ts` — escaped the `/` to
  `\/` in all three pin step expressions (lines ~83, ~101, ~127).
- **Local verification:** built every string step expression in the file under
  `@cucumber/cucumber-expressions@18.0.1` (the CI library version) — all 10 parse
  cleanly; the pre-fix unescaped form reproduces the exact `column 30` error.
  Prettier-clean.
- Ledger: `8b5167c0c57c` → `status: triaged`, `triaged_at_build: 1110`,
  `decompose_on_confirm: true`. Confirmation = genesis green-streak ≥3 with the
  E2E parse error gone (commit-only; integrator pushes — the genesis build is the
  CI validator).

## Note for the harvester / next sentinel run

The flagged line ("⏳ waiting for doorway rollout") is a classifier false-positive
on a progress echo whose success line immediately follows. If this exact line
fingerprints again on a genesis build whose `/health` succeeds, it is NOT a
doorway concern — read the E2E stages for the real cause. (Not promoting a
harvester-classifier change here; one occurrence isn't a pattern yet.)
