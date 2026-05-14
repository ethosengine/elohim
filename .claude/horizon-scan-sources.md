# Horizon-scan sources

Canonical list for the cartographer's quarterly horizon scan (`/mem-horizon-scan`). The point isn't to read every source every cycle — it's to have a stable surface area so deltas across cycles are meaningful (this URL said X last quarter, says Y now).

When updating: add sources at the bottom; mark retired ones `(retired YYYY-MM-DD)` rather than deleting (preserves history of what we used to watch).

## Native Claude memory mechanisms

Where Anthropic's memory primitives evolve. Watch for: new memory features in Claude Code, changes to CLAUDE.md handling, "Memories" feature evolution, "Projects" with memory, offline consolidation primitives.

- **Claude Code documentation** — https://docs.anthropic.com/en/docs/claude-code (or its current canonical URL; check release notes index)
- **Anthropic news** — https://www.anthropic.com/news (filter for memory / Claude Code / agent-related posts)
- **Anthropic engineering blog** — when they publish on memory architecture, agentic loops, prompt caching, system-prompt evolution
- **Claude Code changelog/release-notes** — whichever feed lists Claude Code minor-version changes

## Substrate / MCP layer

Where the substrate we're built on evolves.

- **MemPalace repo** (whichever GitHub/source the project lives in — locate via `mempalace --version` or the install path); watch releases, RFCs, breaking changes
- **MCP servers in the ecosystem** related to memory (search modelcontextprotocol.io or the awesome-mcp lists for new memory servers)

## Alternative / adjacent architectures

What others are doing differently. Useful for "we built X; should we have built Y?" reflection.

- **MemGPT / Letta** — https://github.com/letta-ai/letta (formerly MemGPT). OS-style memory tiering; main read for "what does a tiered memory system look like?"
- **LangGraph memory** — https://langchain-ai.github.io/langgraph/concepts/memory/ (or current canonical URL). Practitioner shape for agentic memory.
- **AutoGen / ax / braintrust** memory docs — whichever framework's memory chapter is currently most active
- **Sleep-time-compute / dreaming patterns** — search "sleep-time compute LLM" or "dreaming LLM consolidation" — emerging primitive for offline memory consolidation

## Academic surface

What the literature is consolidating around. Quarterly is the right cadence for arxiv watch (papers compound slowly).

- **Arxiv** — search "memory-augmented language models" + "agent memory" + "long-context retrieval" + "consolidation" (last 90 days, filter to most-cited or institution-anchored)
- **Surveys** — when one publishes on agentic memory, agent-OS, or long-horizon LLM systems, that's high-signal
- **Workshop proceedings** at NeurIPS / ICLR / ACL on memory + agents

## Awesome lists / practitioner roundups

- `awesome-llm-memory` (or current canonical equivalent)
- `awesome-agent-orchestration` (memory chapters)
- Reddit `r/LocalLLaMA` (filter for memory threads with substantive discussion)

## Scoping discipline

The scan's job is NOT exhaustive coverage. The job is "is anything emerging that would change how we run our memory team?" Filter aggressively:

- **Skip**: vendor marketing, abstract surveys with no implementation, anything not adjacent to our substrate
- **Surface**: concrete primitives we don't have, comparable systems' lessons-learned, breaking changes in our substrate dependencies
- **Elevate**: anything that would change a tier in our LIFECYCLE.md, retire one of our roles, or replace a piece of our toolkit

## Output discipline

Per `.claude/skills/mem-horizon-scan/SKILL.md` — output is a single dated report at `.claude/memory-kit/horizon-scans/YYYY-MM-DD.md` with frontmatter for `scanned_at`, `next_recommended_scan`, `sources_checked`, plus body sections for `Horizon delta`, `Already-aligned`, `Elevation candidates`.
