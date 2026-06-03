---
name: project_cite_seal_born_linked_enforcement
description: "The semantic-links discipline is enforced deterministically (seal keystone + postHook + ceremony + end-of-sprint), not documentation-only"
metadata: 
  node_type: memory
  type: project
  originSessionId: 7c732b67-b888-46d5-a52e-6372cedb7b53
---

The semantic-computable-links "born-linked" discipline is now **enforced deterministically**, not left to recall. An audit (2026-06-03) found it was documented-only at every point — `/brainstorm` and `/plan` described `cites:` but never ran the linking tool; the stasis-loop only drained legacy cites *reactively*. Closed with four wired points around one keystone:

1. **Keystone** — `cite-gen.py --seal <doc>` is the single composite: `assign-id → --into → --verify`, and it flags cites still on the **title-default desc** (the migration's weak default) as progressive-discovery debt. `--seal-all` is the corpus sweep (cheap needs-check skips already-sealed docs, ~0.1s clean).
2. **postHook** — `.claude/hooks/cite-seal-signal.py` (PostToolUse Edit|Write, registered in settings.json) nudges the moment a doc-root `.md` is written carrying un-sealed debt (legacy path-cite to an id-bearing doc, or no `id:` yet). Non-blocking, no slug-index build, **self-limiting** (silent once sealed).
3. **Ceremony POST-step** — `.claude/commands/brainstorm.md` + `plan.md` run `cite-gen --seal <new-doc>` right after writing it.
4. **End-of-sprint** — `/shift`'s decompose-self close (step 9) and the `memory-stasis-loop` `cites` dimension run `--seal-all`; pair with `cite-describe.py` (or the corpus-describe workflow) to author relationship hints for the title-default descs the seal flags.

**Why:** a sealed doc is content-addressed (slug+desc+fingerprint), so its cites survive the target relocating (e.g. into `held/` → `HELD-CITE` not `DEAD-CITE`) and carry a "what + why HERE" hint. That is what makes held↔live moves and bulk relocations free. The weak-desc → relationship-hint authoring is agentic (judgement), so the seal does the deterministic floor and *flags* the rest rather than faking it.

Builds on [[project_memory_cites_edge.md]] (the cites: edge + coherence hook) and [[feedback_build_move_safety_before_bulk_relocation.md]] (content-addressed links before bulk moves). Skill: `.claude/skills/semantic-links/SKILL.md`.
