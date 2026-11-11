---
id: "backlog-absolutes-guard-hook-near-its-2s-budget"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "absolutes-guard-signal.py takes about 1.2 s on a prose document against its 2 s PostToolUse budget, roughly ten times any other Edit|Write hook, and times out under build load"
slug: "absolutes-guard-hook-near-its-2s-budget"
written: "2026-11-09"
author: "artifact-store sprint 2 orchestrator (claude-opus-5-5, shem)"
status: "open"
priority: "medium"
jobs: [agent-harness, hooks]
cluster: "hook-budgets"
tags: [hooks, latency, algedonic, absolutes-guard, budget]
---

**Observed 2026-11-09 ~04:30Z.** Writing
`genesis/docs/superpowers/sprints/2026-11-07-artifact-store/RESULT.md` (about 7 KB of prose)
raised `harness exception: PostToolUse:Write timed out ×1`. The transcript's `hook_cancelled`
attachment names only the event, `PostToolUse:Write`, not the hook. The machine was running two
builds at the time: the device-arrival worker compiling brit, and a pre-push gate.

Each of the 17 `Edit|Write` PostToolUse hooks was replayed on the same payload, idle:

| hook | budget | ms |
|---|---|---|
| `absolutes-guard-signal.py` | 2 s | **1201** |
| `blind-reader-review.py` | 3 s | 277 |
| `sovereignty-guard-signal.py` | 2 s | 158 |
| every other hook | 2–10 s | 44–144 |

At 60% of its budget when idle, absolutes-guard is the only hook that crosses on a loaded host.
Documents that state outcomes ("never", "always", "every") are exactly where it does the most
work.

**Sibling, same class (2026-11-08, sprint journal only until now):** `claude-md-structural-signal.py`
took about 3.4 s on structural operations against its 2 s budget (artifact-store STATE.md, Radicle survey
landing, 14:25Z).

**Cure:** profile `.claude/hooks/absolutes-guard-signal.py` (the `_lib.absolutes` classifier,
the frame atoms, and its `fcntl` ledger lock) and bring it under ~300 ms. If its work cannot
shrink, raise its budget to match it. A budget under the hook's own idle cost is a recurring
timeout. Owner: whoever holds `.claude/hooks/` (outside the artifact-store sprint's write-set).
