---
name: No /generalize-permissions gate in /shift kickoff
description: Auto mode handles permission prompts inline; pre-shift palette pass is no longer required and adds friction
type: feedback
originSessionId: 9224e3af-0b7a-4f15-b700-1fc37bfe9b39
---
Do not run `/generalize-permissions` as the first step of `/shift` kickoff. Skip straight to Objective interview/composition.

**Why:** Claude Code's auto mode handles permission prompts inline at command time, so the up-front bulk-collapse pass adds friction without buying anything. The standalone `/generalize-permissions` command still exists for hygiene runs after a shift's wishlist accumulates.

**How to apply:** When `/shift` is invoked, go directly to Objective composition (interview, JSON, palette gap-check, readiness, journal init). Treat `/generalize-permissions` as user-driven hygiene only. The change is also reflected in `.claude/skills/agentic-developer/SKILL.md`, `.claude/commands/shift.md`, and `.claude/skills/generalize-permissions/SKILL.md`.
