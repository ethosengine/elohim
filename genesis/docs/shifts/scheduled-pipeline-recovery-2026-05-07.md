# Scheduled Pipeline Recovery Shift — 2026-05-07

**Shift type:** scheduled-pipeline-recovery
**Started:** 2026-05-07 06:26 UTC
**Ended:** 2026-05-07 06:35 UTC (bailed early — environmental block)
**Status:** **BLOCKED — no work executed**
**Operator action required:** see "Handoff" section.

---

## TL;DR

The shift could not begin survey because **Jenkins is unreachable from this
sandbox**. The shift kickoff prompt assumed `WebFetch` could reach
`https://jenkins.ethosengine.com/...`, but in this environment the egress
allowlist denies the host. Without survey data there is no responsible way
to fix or retrigger pipelines blind, so the shift bails immediately and
hands the budget back to the operator.

No code touched. No commits to dev. No empty-retrigger commits dispatched.
Only artifact: this report.

---

## Initial survey table

| Pipeline | Result | Build # | Notes |
|----------|--------|---------|-------|
| elohim-orchestrator/dev | UNKNOWN | — | egress block — see "Network reachability" |
| elohim (app) | UNKNOWN | — | last known dispatch was elohim/dev #1408 for SHA `1244f114` (per commit `742f6ef` message) — pod evicted by NoExecute taint |
| elohim-edge | UNKNOWN | — | egress block |
| elohim-holochain (DNA Lamad) | UNKNOWN | — | egress block |
| elohim-genesis | UNKNOWN | — | last retrigger commit `742f6ef` at 04:14 UTC; result unknown |
| elohim-sophia | UNKNOWN | — | egress block |
| elohim-epr | UNKNOWN | — | egress block |
| elohim-storybook | UNKNOWN | — | per commit `8649716b` (01:56 UTC) #25 was SUCCESS, #26 died at Checkout with infra signal-15; #27 retriggered via `[build:storybook]` — terminal state unknown |

All entries are UNKNOWN because **the survey step never executed**. See
"Network reachability" below.

---

## Network reachability — what was actually attempted

Three independent probes, all failed:

```text
$ curl -sI https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/api/json
HTTP/2 403
x-deny-reason: host_not_allowed
content-length: 21
content-type: text/plain

$ curl -sI https://jenkins.ethosengine.com/
HTTP/2 403
x-deny-reason: host_not_allowed

$ curl -sI https://example.com/
HTTP/2 403
date: Thu, 07 May 2026 06:27:10 GMT
server: Varnish
```

`WebFetch` returns the same 403 for every URL tried (Jenkins, example.com,
api.github.com). The only network channel that works is the GitHub MCP
server (`mcp__github__*`), which is whitelisted at the harness layer.

Conclusion: this sandbox has a strict egress allowlist that excludes
`jenkins.ethosengine.com`. The "anonymous WebFetch" path described in the
shift prompt was viable in earlier shift environments but is not viable
here.

### What GitHub MCP can and cannot substitute for

| Need | GitHub MCP coverage |
|------|---------------------|
| `lastBuild` result for orchestrator branch job | **No** — Jenkins posts statuses on PR head SHAs, not on dev pushes |
| Per-pipeline build number, timestamp, URL | **No** — same reason |
| consoleText for failing build | **No** — Jenkins is unreachable |
| Commit statuses on a PR head | **Yes** — `pull_request_read --method get_status` works |
| Recent commits on dev (to read prior shift retrigger commits) | **Yes** — `list_commits` works |

The MCP gives me enough to read history but not enough to **observe the
current state of any pipeline**. The shift's stop conditions
("UNSTABLE-or-better", "no progress for 30 minutes") are unobservable
from this position.

---

## Why I did not push speculative retriggers

Tempting fallback: push empty `[build:<key>]` commits for every pipeline
in scope and let the orchestrator decide. Rejected because:

1. **No feedback loop.** Without Jenkins read access I cannot tell if a
   retrigger went green, red, or stayed red — I'd be writing spam.
2. **Recent prior work already covers the obvious cases.** Commit
   `742f6ef` (04:14 UTC, ~2h ago) already retriggered `[build:app]` and
   `[build:genesis]`; commit `8649716` retriggered `[build:storybook]`.
   Those builds may still be in flight. Stacking another retrigger on top
   without knowing the outcome of the first is shift-anti-pattern (the
   "cascade-halt" gotcha in the prompt: "fixing one pipeline often
   unmasks downstream failures one layer at a time").
3. **Orchestrator dispatch budget is real.** Each empty commit dispatches
   the entire matched pipeline set. Doing this blind for 7 pipelines =
   ~7 × N runner-minutes consumed for zero diagnostic gain.
4. **Hard rule violation risk.** The prompt explicitly forbids
   "If a fix would touch shared infra (Jenkinsfile root, orchestrator,
   build-manifest.json), STOP and document in the report instead of
   pushing — that's outside auto-shift authority." A blind survey-less
   retrigger across all pipelines is closer to that class than to a
   targeted code fix.

---

## Handoff — what the operator needs to decide

Pick one of:

**A) Whitelist `jenkins.ethosengine.com` for shift sandboxes.**
Add the host to whatever allowlist gates `curl` / `WebFetch` egress in
the agentic-developer container image. The shift prompt's design assumes
this is reachable; right now it isn't. This is the "unblock the channel"
fix and the prompt becomes runnable as-written.

**B) Switch the shift channel from Jenkins to GitHub commit statuses.**
If Jenkins is going to stay denylisted, the orchestrator could be
configured to post per-pipeline GitHub commit statuses on dev pushes
(not just PR heads). Then `mcp__github__get_commit` could see them. This
is a one-time orchestrator change in `genesis/orchestrator/Jenkinsfile`
to extend the existing PR-head status posting to dev/main pushes.

**C) Provide a curated "starting failures" list as a shift parameter.**
If the operator already knows which pipelines are red, pass them in the
Objective so the shift can target consoleText-equivalent investigation
through a different read channel (e.g. last commit message, sprint-result
artifact, or a Jenkins-fetched log file mirrored to GitHub artifacts).

**D) Run the shift from an environment with Jenkins network access.**
The earlier shifts on 2026-05-06/07 (ending at commit `742f6ef`)
clearly had reachability — they cite specific build numbers (`#1407`,
`#1408`, `#883`, `#26`). Running this shift in that same environment
would just work.

Default recommendation: **A**, because it preserves the existing prompt
and CI architecture. **B** is a structural improvement worth doing
independently (it gives operators commit-level CI status visibility on
dev pushes, which is useful beyond shifts), but is more than this shift
should attempt.

---

## Final state table

| Pipeline | Result | Action taken | New result |
|----------|--------|---------------|-------------|
| (all)    | UNKNOWN | none — survey blocked | UNKNOWN |

---

## Time budget

- Allocated: ~90 min
- Consumed: ~9 min (survey attempts + report writing)
- Returned to budget pool: ~81 min

## Files changed

- `genesis/docs/shifts/scheduled-pipeline-recovery-2026-05-07.md` (this file, new)

## Branch

`shift/report-2026-05-07` (off dev `742f6ef`). Not opened as PR — no
code change to review, just the artifact. Operator can merge or close.
