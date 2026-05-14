---
name: routing-layer-hygiene-is-librarian-scope
description: Agent-catalog and skill-catalog hygiene (description tightening, tools-mismatch, overlap disambiguation) is librarian's present-tense deliverable, not general-purpose. The audits that surface the drift (skill-audit, agent-audit) are explicitly librarian-driven per memory-kit CLAUDE.md.
type: feedback
---

When dispatching work to fix the always-loaded routing layer — skill descriptions, agent definitions, trigger overlap, frontmatter tools-mismatch — dispatch to **librarian**, not general-purpose or claude.

**Why:** Per `.claude/scripts/memory-kit/CLAUDE.md` (the memory-kit ownership table) and the librarian agent definition: librarian is the present-tense curator that drives `skill-audit` and `agent-audit`, and decides what to act on. The librarian "orchestrates the memkit toolkit with judgment about what matters" — and what matters in the routing layer is exactly the kind of present-tense hygiene the librarian role exists for.

**How to apply:**

- For agent-catalog or skill-catalog hygiene work (descriptions / overlap / tools-mismatch / drift): dispatch to `librarian` subagent.
- For working-memory hygiene (cleanup, dedupe, MEMORY.md tighten): same — librarian.
- For CLAUDE.md drift triage: same.
- General-purpose / claude is the wrong tier — they lack the librarian's calibrated heuristics about false-positives (e.g., `tools-mismatch` on Claude tools is a known false-positive class that librarian recognizes; general-purpose may strip tools the agent actually uses).
- The boundary holds even when the artifact being edited isn't `.claude/memory/`. The librarian's scope is *what gets routed at agent dispatch time* — that includes agent definitions and skill manifests, not just the topic files under `.claude/memory/`.

Pairs with [[feedback_first_memory_team_ceremony]], [[project_three_temporal_perspectives]].
