---
name: Agentic context graph model
description: Architectural model for context across the dev lifecycle — skill-graph + typed baton + velocity-tiered memory + localized drift hooks + consolidation harvest; confirmed by industry convergence and Anthropic Dreams (2026-04-21 beta)
type: project
originSessionId: 10d85ef0-1979-4311-97e9-c2c209de48e2
---
The model crystallized 2026-05-10 across a multi-probe brainstorm. Five load-bearing pieces, all in dialogue:

1. **Skills as a graph, not a flat bag.** Each skill is a node with `consumes:` / `produces:` declared in frontmatter. Edges between skills carry typed artifacts. Industry surveys (Anthropic Skills, OpenAI Codex Skills, Microsoft Agent Framework) all treat skills as a flat bag today; nobody has shipped typed inter-skill handoff. Our repo's prose already names skill positions (e.g. p2p-design-gate "sits between brainstorming step 2 and 3"); making it machine-readable is the missing step.

2. **Edges typed by validation surface, weighted by drift tolerance.** Three tiers: *contract* (schema/serde, doorway↔storage wire — hooks fire loud, refresh per-commit); *interface* (skill I/O, route manifests — soft check); *narrative* (manifesto↔design↔scenario — gentle reminder, refresh per-sprint). Industry convergence (Temporal+OpenAI, LangGraph, Microsoft Agent Framework) is "tight typing at machine boundaries, loose context at human boundaries, with durable journaling so the loop can recover when an edge slips" — verbatim our model.

3. **Velocity-tiered memory with validity intervals (not just TTLs).** Manifesto rare / contract per-commit / iteration per-task. Zep's insight: "Alice owned the budget until February" is structurally different from "Alice owns the budget" with TTL — facts close their interval rather than expiring. The "memory says X exists but X was renamed" failure has an industry name: *Stale Memory Override*.

4. **Localized drift hooks as consolidation triggers + handoff validators.** Today's SessionStart hook is the un-positioned, un-weighted prototype — fires for every conversation regardless of what node we're at. Localizing by edge + weighting by tier turns one always-on hook into many sharp position-aware ones with no extra cognitive load.

5. **Consolidation harvest — the field's named-fragile open problem.** GAM/TiMem call episodic→semantic promotion structurally fragile. Anthropic shipped **Dreams** (research preview, beta `dreaming-2026-04-21`) as the primitive: input memory store + up to 100 session transcripts + instructions string → new output memory store (input never modified). Dedupes, replaces stale, surfaces new insights. Operator-gated by design (output is separate; review or discard). Almost exactly the consolidation pattern we sketched, productized.

**Why:** field consensus is that consolidation is unsolved at the skill-graph layer specifically; the elohim repo has the structural pre-work others lack — manifesto, schemas, scenarios, validation surfaces, dated specs, MEMORY.md tier already roughly stratified. We're not inventing structure; we're declaring what's already there.

**How to apply:**
- When designing new skills, declare `consumes:` / `produces:` / `position:` in frontmatter — the graph compiles itself.
- When specs accumulate without consolidating into MEMORY.md or manifesto principles, that's a harvest signal — repetition across the episodic corpus is the promotion criterion.
- Treat localized drift hooks as cheap — one per (node, edge, weight) tuple; not one global hook trying to fire everywhere.
- Memory invalidation: when a referent renames or deletes, close the interval (mark validity ended) rather than just deleting the entry — preserves the trajectory.
- Dreams primitive is a candidate consolidation engine (requires beta access via https://claude.com/form/claude-managed-agents). Local equivalent works fine: any LLM + structured input + reviewable output store achieves the same shape.

**Trajectory corpus in this repo:**
- Episodic (high volume, dated): `genesis/docs/superpowers/specs/2026-MM-DD-*.md`, `genesis/docs/plans/`, `.claude/data/dev-intent.jsonl`, sprint-result.md artifacts, git commit messages, conversation transcripts
- Semantic (mid-velocity, principle-shaped): `MEMORY.md` topic files, `.claude/skills/*/SKILL.md` descriptions, `CLAUDE.md`
- Manifesto (rare, vision-shaped): `genesis/docs/content/elohim-protocol/`

**Sources:** brainstorm conversation 2026-05-10; three research probes (industry RAG/graph survey, Claude Code/peer agent codebase exploration, agent memory systems); Anthropic Dreams documentation (https://platform.claude.com/docs/en/managed-agents/dreams).
