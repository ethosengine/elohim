---
name: agent-prompts-no-process-status
description: "Agent prompts (and skill prompts, and any gospel-tier always-loaded surface) describe stable architecture and patterns, not sprint progress. Temporal state belongs in memory entries and chronicles, which link forward and stay current."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: ba3b92b8-8e5a-46b4-aa04-6dbcdeab5119
---

**Rule**: Agent prompts (`.claude/agents/*.md`), skill prompts (`.claude/skills/*/SKILL.md`), and any gospel-tier always-loaded substrate must not contain process-status phrasing — "Phase N closed", "prereq #1 CLOSED; gates #2–#N remaining", "in-flight graduation work", "next milestone is X", "currently sprinting Y". Describe the architecture and the stable patterns; let memory entries, chronicles, and sprint-results carry the temporal state.

**Why**: Agent and skill prompts are always-loaded gospel. Anything time-sensitive in them decays the moment substrate moves — a phase closes, a gate count changes, a sprint completes — and the prompt's now-stale claims actively mislead subsequent sessions. The Run #6 memory ceremony first added "Phase 11 prereq #1 closed; cutover gates #2–#10 remaining" to rust-architect.md as part of an iroh-awareness update; the operator caught it as drift-bait. Memory entries (`.claude/memory/project_iroh_phase11_*.md`) and chronicles ARE the temporal record — they're journals. Agent prompts pointing to them via `[[wiki-link]]` stays current because the link resolves to whatever the journal currently says.

**How to apply**: When writing or editing an agent/skill prompt, name the architecture and the stable invariants. Where temporal context matters (a parallel-stack window, an in-flight migration), describe the *shape* — "two parallel transport stacks, `TransportBackend` selects at runtime" — not the current *position* — "we're 80% through cutover, 6 gates remain." Link to memory entries that carry the temporal state. Specifically forbidden phrasings: "currently", "in flight", "as of [date]", "closed", "remaining", explicit gate/milestone counts. Allowed: "during the cutover window, both stacks must work in parallel" (describes a discipline that's true whenever it's true).

Inverse: memory entries SHOULD carry process status. They're frozen snapshots of state at a moment, and the chronicle layer carries forward-pointers to the next one.

Related: [[project_three_temporal_perspectives]] (gospel/canonical/working memory have different decay rates); [[feedback_first_memory_team_ceremony]] (cross-substrate coherence is the ceremony's job).
