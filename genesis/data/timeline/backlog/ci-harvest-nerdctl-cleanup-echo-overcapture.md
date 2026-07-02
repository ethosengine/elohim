---
id: "backlog-ci-harvest-nerdctl-cleanup-echo-overcapture"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ci-harvest over-captures routine image-cleanup command echoes as failures — INFRASTRUCTURE taxonomy's bare `nerdctl` token matches `set -x` echoes of the cleanup stage"
slug: "ci-harvest-nerdctl-cleanup-echo-overcapture"
written: "2026-07-02"
author: "ci-failure-triage"
status: "backlog"
priority: "low"
ci_status: blocked
fingerprints: [e4cad4b435b1, b9ee178c936d, 310cd811389a, b0d29582b52f]
jobs: [elohim-edge]
relatedNodeIds: []
tags: [ci, ci-harvest, taxonomy, false-positive, classifier-precision, nerdctl, cleanup-stage, set-x-echo, elohim-edge, tooling]
cites:
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1137/
  - .claude/scripts/ci-harvest.py
  - .claude/data/failure-taxonomy.json
  - genesis/data/timeline/backlog/ci-edge-p2p-sim-docker-compose-missing.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# ci-harvest over-captures cleanup-stage command echoes as failures

ONE concern behind four fingerprints. The harvester filed four INFRASTRUCTURE
"failures" from `elohim-edge` #1137 that are not failures at all — they are the
`set -x` command echoes of the routine image-cleanup stage, whose `nerdctl rmi`
commands **succeeded**. This is a harvester classifier-precision gap (false
positive), not a CI failure. Canonicalized so the deterministic layers stop
re-firing on this signature class.

## The failure (what was fingerprinted)

Four ledger lines, all `elohim-edge`, category INFRASTRUCTURE, seen 1,
first/last build 1137:

```
e4cad4b435b1  + nerdctl -n k8s.io rmi elohim-doorway:#.0.0-dev-#
b9ee178c936d  + nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-doorway:#.0.0-dev-#
310cd811389a  + nerdctl -n k8s.io rmi elohim-storage:#.0.0-dev-#
b0d29582b52f  + nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-storage:#.0.0-dev-#
```

The `+ ` prefix is the bash `set -x` trace — these are echoed COMMANDS, not
error output. In the #1137 console each of the four is immediately followed by
its own success output, e.g. (log lines ~18904–18936):

```
+ nerdctl -n k8s.io rmi elohim-doorway:1.0.0-dev-f6d36262
Untagged: docker.io/library/elohim-doorway:1.0.0-dev-f6d36262@sha256:b00cd7b7…
Deleted: sha256:b05a96227958…
```

The commands ran and removed the images. Tellingly, the ONE cleanup `rmi` that
actually errored in #1137 —
`harbor.ethosengine.com/ethosengine/elohim-agent-sdk` with
`level=fatal msg="… no such image …"` then `+ true` (line ~18978) — was NOT
fingerprinted, because the cleanup step swallows errors with `|| true` and the
per-build console cap (`MAX_CONSOLE_FINDINGS_PER_BUILD = 4`) had already been
spent on the four succeeding echoes above it in the tail window.

## Verdict — infra false positive (harvester over-capture), NOT a CI failure

- The rmi commands succeeded (Untagged/Deleted). No image-in-use error; the
  cleanup stage did not fail (and structurally cannot fail the build — `|| true`).
- #1137 was UNSTABLE from two unrelated, already-known causes, both raised as
  **stage-level `unstable()`** calls (not JUnit failures):
  - "Quality Gate: Storage - failures (non-blocking while stabilizing): script
    returned exit code 1" → `[Pipeline] unstable` → "WARNING: Storage quality
    gate failed" (console ~13263). Known; fix queued at `f6c06a76e`.
  - "`./simulate.sh: line 44: docker-compose: command not found`" in the P2P
    Simulation Test stage (console ~15481). Pre-existing; already canonicalized
    in `ci-edge-p2p-sim-docker-compose-missing.md`.
- Neither UNSTABLE source is the cleanup stage. The cleanup echoes are pure
  noise the harvester mis-filed.

## Root cause (why the harvester filed them)

Two mechanisms compose:

1. **Fall-through to console classification.** `collect_build_findings`
   (`ci-harvest.py`) tries the JUnit testReport first (step 1); a stage-level
   `unstable()` produces no FAILED/REGRESSION cases, so #1137 had nothing there
   and fell through to the console-tail taxonomy scan (step 2).
2. **A tool-name token with no error context.** The INFRASTRUCTURE category in
   `failure-taxonomy.json` is `"search": "hApp.*not found|nerdctl|denied"`. The
   bare `nerdctl` alternative matches EVERY line containing the string
   `nerdctl` — including `set -x` command echoes of the cleanup stage, which run
   on essentially every build. On an UNSTABLE build with no test report, the
   scan then harvests up to `MAX_CONSOLE_FINDINGS_PER_BUILD` (4) of those echoes
   as findings — exactly the four doorway/storage lines seen here (the four that
   fell inside the 60 KB console-tail window before the cap was hit).

So the trigger is: UNSTABLE-via-stage-`unstable()` + empty test report + a
loose tool-name token that matches routine command echoes. It will recur on any
future UNSTABLE `elohim-edge` build that has no failing test cases.

## Museum gate

Related to museum trap #1 ("lossy measure" family — a build's result is read
imprecisely) but the opposite polarity: this is a **false positive** (capturing
non-failures) rather than the false-negative NOT_BUILT-reads-as-0 trap. It is a
harvester-classifier precision bug in the sentinel arm itself, not a
CI/orchestrator pipeline anti-pattern, so it does NOT graduate into the
`2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md` record on this
first occurrence (seen 1, one build). If the "tool-name token matches `set -x`
echo" shape recurs across ≥3 shifts or bites another taxonomy category, it earns
a museum row then (extend that record — never fork a second lessons doc).

## Current decision

`blocked` — on a bounded harvester-precision fix that is deferred out of this
run by dispatch scope ("canonicalization + ledger status only"). The four ledger
lines are marked `status: blocked` so they persist and DEDUPE on recurrence
(bump `seen`, no new triage dispatch) rather than re-firing the sentinel. What
unblocks: land the classifier-precision fix below (a follow-up tooling change),
then these entries can be closed. The stasis sweep re-checks `blocked` entries.

## Proposed fix (bounded, deferred — NOT landed this run)

Two candidate fixes, both small and low-risk. Preferred is the second (fixes the
whole class, not just `nerdctl`):

1. **Narrow (data only, `failure-taxonomy.json`).** Replace the bare `nerdctl`
   token with an error-context-bearing pattern, e.g.
   `nerdctl.*(error|fatal|denied|no such|failed)`, or drop `nerdctl` entirely
   (real push/pull auth failures are already covered by the `denied`
   alternative and by non-echo error output). Blast radius: one JSON value.

2. **Robust (classifier, `ci-harvest.py`).** In `collect_build_findings` step 2,
   skip lines that are `set -x` command echoes (match `^\s*\+ `) before regex
   classification. A `set -x` echo is the command about to run, never its
   result — errors always appear on subsequent non-`+` lines — so excluding
   echoes from failure classification is sound and fixes the whole "tool-name
   token matches a command echo" class across every category, not just
   INFRASTRUCTURE/`nerdctl`. Add a unit test with a synthetic cleanup-echo
   console fixture asserting zero findings.

## Fix trail

No fix landed this run (dispatch scoped to canonicalization + ledger status).
Ledger annotated:
- `e4cad4b435b1`, `b9ee178c936d`, `310cd811389a`, `b0d29582b52f` →
  `status: blocked`, `backlog: ci-harvest-nerdctl-cleanup-echo-overcapture`,
  with an inline `note` recording the false-positive verdict for the
  deterministic layers.
