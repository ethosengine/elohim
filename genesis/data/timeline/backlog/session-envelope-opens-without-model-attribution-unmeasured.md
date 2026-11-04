---
id: "backlog-session-envelope-opens-without-model-attribution-unmeasured"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every Claude Code session envelope opens without a model, so every commit is stamped UNMEASURED and the live envelope cannot be corrected"
slug: "session-envelope-opens-without-model-attribution-unmeasured"
written: "2026-11-04"
author: "claude (row-5 precursor session a340d2e2)"
status: "backlog"
priority: "medium"
tags: [attribution, session-envelope, commit-trailers, claude-code, device-runtime, observability]
cites:
  - elohim/eprfs/device-runtime/src/envelope.rs
  - elohim/eprfs/device-runtime/src/commit_attribution.rs
  - genesis/agentic/agent-runtimes.json
---

# The envelope's model is never measured for Claude Code

## Observed (session a340d2e2, 2026-11-03..04)

- `epr session show --json` → `envelope.model: null` from SessionStart onward. Every commit's
  `prepare-commit-msg` printed `[commit-attribution] UNMEASURED: epr session: session model is
  unmeasured`. The commits carry a hand-written `Co-Authored-By` instead.
- `epr session open --model claude-opus-5-5` on the live session returns the existing envelope
  unchanged. `envelope::model()` is consulted only when an envelope is first opened: it takes the
  explicit `--model`, else the runtime's declared `model_env` variables.
- `genesis/agentic/agent-runtimes.json` declares no `model_env` for any runtime (claude-code,
  codex, gemini, antigravity). The SessionStart hook opens the envelope without `--model`.
- The berth mooring does carry the model: `berth moor --model claude-opus-5-5` recorded it in
  `berth.mooring.model`. So the device knows the model, but the envelope never learns it.

## Why it matters

CLAUDE.md requires an UNMEASURED warning to be diagnosed before publishing, and forbids amending
history to repair an omission. A batch landed from such a session is attributed only by hand, so
the native measure stays absent for every Claude session.

## Cure (pick by the session-discipline design, not here)

1. Give the envelope a one-time, owner-only "measure model" act: a live envelope whose model is
   `null` may record a model once, from `--model` or from the mooring. That is an observation
   arriving late, not a rewrite of a recorded one.
2. Or make the SessionStart hook pass the mounted model to `epr session open --model`, where the
   harness exposes it.
3. Declare `model_env` for the runtimes that export one.

Probe: a fresh session's first commit prints no UNMEASURED line, and `commit-trailers --json`
names the model.
