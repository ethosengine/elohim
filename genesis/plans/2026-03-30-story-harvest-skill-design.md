# Story Harvest Skill — Design

**Date:** 2026-03-30
**Status:** Approved

## Problem

The most valuable regression stories come from the discovery process during development — failures that prompt designs, edge cases that surprise, specific parameter values that reveal constraints. These discoveries live in conversation context and developer memory. If not captured at the moment of insight, they're lost.

The Resilient Delivery epic is the canonical example: Matthew observed Evolution of Trust cause storage OOM under browser load. The specific parameters — 256MB container, 30 concurrent browsers, 40 files per app — revealed the constraint that P2P storage nodes cannot absorb CDN-pattern traffic. Those parameters now inform the NodeCapabilities presets (laptop: 200MB cache, home_node: 2GB, network_node: 10GB). But this knowledge only survived because Matthew was in the room. The next discovery might not.

## The Engineering Discovery Loop

The bridge engineering metaphor:

| Step | Bridge Engineering | Elohim Protocol |
|------|---|---|
| 1 | Test bridge to failure | Observe 502s on Evolution of Trust |
| 2 | Discover the constraint | "Storage with X memory under Y concurrent requests = OOM" |
| 3 | Document as specification | a2o scenario with specific parameters |
| 4 | Prove the constraint is handled | Regression test: "With cache enabled, same load is absorbed" |
| 5 | Future bridges designed against spec | Future sprints inherit scenario as guard rail |

The skill's job: prompt step 2-3 before the developer moves on.

## Design

### Trigger Points

- **finishing-a-development-branch**: Between Step 1 (tests pass) and Step 3 (present options)
- **systematic-debugging**: After root cause identified and fix verified
- **Manual**: `/story-harvest` at any time

### Discovery Categories

**Failure Regression** (`@regression`) — "Without X, Y breaks at these values." The original failure preserved as a regression anchor.

**Capability Proof** (`@wip`) — "With X enabled, the system handles Y." The bridge that holds.

**Observability Gate** (`@wip`) — "An operator can see/control Z." The diagnostic capability that enabled the discovery.

### Parameter Specificity as Value Signal

The highest-value stories carry specific operational parameters (memory, concurrency, cache size, timeout, peer count). These parameters do double duty:

1. They prove the constraint in the scenario
2. They inform operator presets, documentation, and recommended configurations

Example: "storage with 256MB container limit receiving 30 concurrent requests" creates the parameters that define why a laptop preset has 200MB cache budget while a home node has 2GB. After major releases, these parameter-bearing stories can be reviewed to update recommended settings.

The skill teaches the agent to look for parameter-bearing stories as highest value, but leaves the noise-vs-signal judgment to the skill's reasoning rather than imposing a hard gate.

### Process

1. **Gather context** — git log, diff stat, dev-intent.jsonl, conversation context
2. **Identify discoveries** — what failed? at what values? how is it handled now? what observability was needed?
3. **Assess value** — parameter-bearing? constraint boundary? observability enabling?
4. **Classify and scaffold** — Given/When/Then skeleton with parameters in comments
5. **Place scenarios** — existing or new feature file under genesis/a2o/features/
6. **Note operational parameters** — connect scenario back to configuration decisions it informs

### Integration

CLAUDE.md instruction added under Development Workflow, between Exploration Fallback and P2P Design Gate:

> "When using `finishing-a-development-branch`, invoke `story-harvest` between Step 1 (tests pass) and Step 3 (present options). When using `systematic-debugging` and a root cause is identified and fixed, invoke `story-harvest` before closing the debugging session."

### What It Does NOT Do

- Write full Gherkin step definitions (wiring is later work)
- Gate the finish workflow (advisory only)
- Replace story-first development (captures what story-first misses: constraints discovered during implementation)
- Generate noise (the skill teaches what to look for; the agent judges)

## Files

| File | Purpose |
|------|---------|
| `.claude/skills/story-harvest/SKILL.md` | The skill |
| `CLAUDE.md` (Development Workflow section) | Integration instructions |
