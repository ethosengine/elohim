---
id: "backlog-hook-claude-md-structural-signal-times-out-on-every-structural-op"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "claude-md-structural-signal.py (PreToolUse:Bash, 2 s budget) takes ~3.4 s on every command with mv/mkdir/cp -r/rm -rf, so the harness kills it and its claude-md-structural-signal@1 folds are lost exactly when they matter"
slug: "hook-claude-md-structural-signal-times-out-on-every-structural-op"
written: "2026-11-08"
author: "artifact-store sprint 2 orchestrator (claude-opus-5-5, shem)"
status: "open"
priority: "medium"
tags: [hooks, algedonic, harness-exception, measure-loss]
---

**Observed 2026-11-08 ~14:20Z** as a harness exception (`PreToolUse:Bash timed out (2010 ms against 2000 ms)`).
Reproduced on shem: a non-structural command costs ~80 ms; a command containing `mv … ; mkdir -p …`
costs 3,349-3,512 ms (two runs, load ~6.7 under a storage gate).

**Cause.** `main()` calls `_obs.emit(...)` synchronously once per (structural path × enclosing CLAUDE.md),
and each emit waits on an `epr flow note` subprocess. The 2 s timeout in `.claude/settings.json` kills the
hook mid-loop, so the fold the hook exists to record never lands — the measure goes numb precisely on
structural edits.

**Cure (precedent 6610db2bb, `claude-md-drift-signal.py:122-136`).** Collect the notes and hand them off
without waiting: `_obs.emit_many(notes, wait=False)`. Re-time with a structural sample command
(expect ~80-100 ms) and confirm one fold per scope arrives (`epr flow report` for
`claude-md-structural-signal@1`).
