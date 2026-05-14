---
name: mem-horizon-scan
description: Quarterly horizon-scan of the LLM-memory landscape for the cartographer's future-projection role. WebFetches canonical sources at .claude/horizon-scan-sources.md to surface what's emerging externally that might change how our memory team operates. Use when "run a horizon scan", "check the state of LLM memory practice", or when the latest scan in .claude/memory-kit/horizon-scans/ is >90 days old. Auto-triggered by cartographer at memory-ceremony Wave 1 when stale.
---

# /mem-horizon-scan — Quarterly external-state check for the memory team

The memory team's three temporal-perspective agents (librarian/historian/cartographer) are inward-tilted. They tend the substrate we have. This skill is cartographer's outward-tilted complement — a quarterly read on how OTHER people are building LLM memory systems, to catch primitives we don't have, breaking changes in our dependencies, and shifts in the field that might want to reshape our practice.

**When to invoke**:
- Cadence: every ~90 days. The latest report at `.claude/memory-kit/horizon-scans/` has a `next_recommended_scan` field; trigger when current date ≥ that field.
- Operator: when "is anything new in the memory space?" or before a major refactor that might be solved cleaner elsewhere.
- Auto: cartographer's Wave 1 freshness check at memory-ceremony invocation — if latest report is missing or stale, dispatch this skill before Wave 1 surface.

**Owner**: cartographer (Opus tier, future-projection role). The scan needs judgment about what's signal vs vendor noise.

## Sources

Canonical list at `.claude/horizon-scan-sources.md`. Read that file FIRST. It enumerates:
- Native Claude memory (docs.anthropic.com, news, engineering blog, changelog)
- Substrate (MemPalace, related MCP servers)
- Alternative architectures (MemGPT/Letta, LangGraph memory, AutoGen/ax/braintrust)
- Academic surface (arxiv on memory-augmented LMs, agent memory, consolidation)
- Awesome lists / practitioner roundups

## Workflow

1. **Check freshness**. Read `.claude/memory-kit/horizon-scans/` — what's the latest report's `scanned_at`? If <90 days ago and not operator-invoked, **bail with "next scan due [date]"** — don't redo work.

2. **Read the sources file** at `.claude/horizon-scan-sources.md`. Identifies what to watch.

3. **WebFetch each canonical source**. Apply scoping discipline (per the sources file):
   - **Skip**: vendor marketing, abstract surveys, anything not adjacent to our substrate
   - **Surface**: concrete primitives we don't have, comparable systems' lessons-learned, breaking changes in our dependencies
   - **Elevate**: anything that would change a tier in LIFECYCLE.md, retire one of our roles, or replace a piece of our toolkit

4. **Compare against our state**. For each finding, ask: "do we already have this?" Read `.claude/scripts/memory-kit/LIFECYCLE.md`, `.claude/agents/*.md`, MEMORY.md briefly. Most findings should land in "already-aligned" — that's a healthy outcome, it means our design is in the field's mainstream.

5. **Write the dated report** at `.claude/memory-kit/horizon-scans/YYYY-MM-DD.md`:

```yaml
---
scanned_at: YYYY-MM-DD
prior_scan: YYYY-MM-DD (or "none — bootstrap")
next_recommended_scan: YYYY-MM-DD (+90 days)
sources_checked:
  - { url: "...", retrieved_at: "...", status: "ok|404|skipped — reason" }
  - ...
sources_skipped:
  - { url: "...", reason: "stale, vendor-only, not adjacent" }
---

# Horizon scan — YYYY-MM-DD

## Summary (≤4 sentences — this becomes the chronicle pointer)

[What's the field doing? Anything we should elevate? Any breaking changes coming?]

## Horizon delta — what's new since [prior_scan or "field overview" for bootstrap]

For each finding:
- **Source**: URL + retrieval date
- **Finding**: 1-2 sentence summary
- **Implication for us**: how this might shape ceremony/roles/substrate
- **Disposition**: ELEVATE / WATCH / SKIP

## Already-aligned

Things that emerged in the field that we already have. Healthy signal — design is in the mainstream.

## Elevation candidates

The ELEVATE items concentrated for cartographer-handoff. Each should turn into either a backlog Objective ("evaluate primitive X") or a memory entry capturing what we learned.

## Substrate / dependency changes

Breaking changes in MemPalace, MCP, Claude Code that affect our toolkit. Highest urgency category.

## Sources visited
[reproducible audit trail]
```

6. **Surface the summary into the active ceremony** if one is running. The cartographer's Wave 1 output prepends the report's "Summary" section as "Horizon delta since [prior_scan]". The chronicle entry written at Wave 6 includes a one-line pointer with date + summary one-liner.

7. **Update MEMORY.md** with a one-line index entry pointing to the latest scan (replaces prior — only the latest needs an index entry; the others are archived chronologically).

## Constraints

- Quarterly cadence is the discipline. Don't run more than every 90 days unless operator explicitly invokes.
- The job is filtering, not summarization. Aggressive skipping is correct.
- WebFetch is rate-limited and may fail; record failures in `sources_checked` with status. A partial scan is better than no scan.
- Don't trust vendor blog posts as primary evidence — cross-reference with the underlying repo, docs, or paper.
- The chronicle's pointer-with-summary discipline: the chronicle entry should carry enough that a future operator can decide if they want to read the full report — without reading it.

## Output discipline

Single dated report + chronicle pointer + MEMORY.md index update. Don't write more than that. The whole point is bounded artifact.

## Related

- `.claude/agents/cartographer.md` — cartographer agent (horizon-scan responsibility section)
- `.claude/horizon-scan-sources.md` — canonical sources
- `.claude/memory-kit/horizon-scans/` — dated reports
- `.claude/skills/memory-ceremony/SKILL.md` — Wave 1 invokes this skill via freshness check
- `.claude/scripts/memory-kit/LIFECYCLE.md` — what the scan is comparing our state against
- `genesis/data/timeline/CONVENTIONS.md` — chronicle frontmatter (pointer field convention)
