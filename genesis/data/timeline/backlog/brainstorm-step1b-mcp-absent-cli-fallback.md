---
id: "backlog-brainstorm-step1b-mcp-absent-cli-fallback"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brainstorm.md Step 1b assumes MemPalace MCP in the main session — document the CLI fallback"
slug: "brainstorm-step1b-mcp-absent-cli-fallback"
written: "2026-06-04"
author: "claude"
status: "refined"
priority: "low"
themes: [memory, brainstorm-seam, mempalace, front-fire, process-meta]
relatedNodeIds:
  - "genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md"
  - ".claude/commands/brainstorm.md"
---

Discovered during the PICKUP fire point brainstorm (2026-06-04, compaction-loop spec §4b): the MemPalace MCP
is wired per-subagent only, so `/brainstorm` Step 1b's `ToolSearch "select:mempalace_search,mempalace_check_duplicate"`
returns **no matching deferred tools** in a main-session context — the semantic lens silently degrades to
lexical-only without the operator knowing the instruction was unfollowable, not just stale.

**Fix shape (one paragraph in `.claude/commands/brainstorm.md` Step 1b):** when ToolSearch returns empty,
fall back to the CLI — `mempalace --palace <repo>/.mempalace/palace search "<topic>"` (~3 s, measured) —
and still apply the §4.4 staleness guard via `mempalace-currency.py --status`. The historian-dispatch
fallback already named in compaction-loop §4.1 remains the over-importing last resort.
