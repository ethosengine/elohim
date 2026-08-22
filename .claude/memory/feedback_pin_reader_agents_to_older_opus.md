---
index: false
name: pin-reader-agents-to-older-opus
title: Pin legibility-judging agents to an explicit Opus ID
description: "Opus 5 handles complexity well but writes less accessibly — pin blind-reader to a full model ID, not the floating `opus` alias."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: c1280948-357d-4a93-9b91-d8d8273c0e3e
  modified: 2026-08-15T12:33:04.772Z
---

Operator observation (2026-08-15, corroborated by others): Opus 5 understands complexity well but is **not as accessible a writer**. For agents whose job is judging legibility — `blind-reader` above all — the `opus` alias is wrong: it floats to the newest Opus and silently re-calibrates the reviewer's prose taste.

**Why:** a legibility reviewer's value is its feel for what an unfamiliar reader can follow, and that is model-generation-specific — pin it. Reasoning-heavy agents (rust-architect, quality-architect) are unaffected; complexity is where Opus 5 is strongest.

**How to apply:** Claude Code subagent frontmatter accepts a full model ID, not just `sonnet`/`opus`/`haiku`/`fable`/`inherit`. Set `metadata.modelHints.claudeModel` in the package under `.epr-meta/elohim/packages/agents/` — never the `.claude`/`.codex` projection ([[feedback_managed_surface_edit_discipline]]) — then `pnpm run elohim-agent:packages:project`, `:runtime`, `:verify`. blind-reader is on `claude-opus-4-6`; `claude-opus-4-5` is the next step back. Contrast [[feedback_delegate_narrow_tasks_to_cheaper_tiers]] — that one is cost, this one is prose quality, and they can point opposite ways.
