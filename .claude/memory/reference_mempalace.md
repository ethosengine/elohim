---
name: MemPalace — proposed substrate for historian / librarian role
description: Python CLI + library + MCP server with ChromaDB vector store and SQLite temporal entity-relationship graph. Wings/Rooms/Drawers metaphor; per-agent diaries; auto-save hooks at Claude Code compaction boundaries. Benchmark 96.6% R@5 on LongMemEval.
type: reference
originSessionId: b5ef4833-2583-4482-b36e-b595da75dafe
---
**URL**: https://github.com/mempalace/mempalace

**What it is**: A layered memory system — CLI tool (`mempalace`), Python library, MCP server (29 tools), and Claude Code auto-save hooks. Stores verbatim text (no summarization) and retrieves via semantic search + temporal graph.

**Architecture metaphor**:
- **Wings** = people/projects/agents (scoping containers; each specialist agent gets its own)
- **Rooms** = topics
- **Drawers** = individual content pieces

**Storage**: Pluggable vector store (ChromaDB default) + SQLite temporal entity-relationship graph with validity windows. Local by default; nothing leaves the machine.

**Why it matters to us**: Named as the proposed substrate for the historian role in `genesis/docs/superpowers/specs/2026-05-13-historian-and-epic-timeline.md`. Maps near 1:1 onto the spec's open questions:

- **Pattern signature schema** (open question) → vector embeddings + temporal graph
- **Historian cadence** → auto-save at compaction + on-demand `wake-up`
- **Epic-graph traversal** → temporal validity windows on entity relationships
- **Per-agent diaries** → matches `[Elohim as specialist subagents]` pattern; each elohim wing
- **Archive surface** → `sweep` indexes `.claude/archive/` content idempotently

**Boundary** (use when):
- Adopt for **history-perspective** tooling (the historian agent, archive indexing, pattern recognition). Belongs to that slice of the three temporal perspectives.
- Do NOT absorb into `memory-kit`. Memory-kit serves the *development-present* slice and stays stdlib + markdown for git-trackability and human-readability.
- Acceptable dependency surface for the future historian wing; would be wrong for memory-kit.

**Open questions before adopting**:
1. Storage location — `.claude-config/` vs new directory; memory data isn't versioned content
2. Embedding model choice — verify the local-inference path actually doesn't phone home
3. Whether to pilot first (small test against `.claude/archive/`) before committing to integration
