---
name: orchestrate
description: Establishes senior-decision-maker (chief-agent) discipline for an expensive top-tier model (Fable 5 / Opus). Keep premium reasoning for intent, architecture, decomposition, tradeoffs, risk, disagreement-resolution, and final review; delegate discovery, implementation, tests, logs, and verification to cheaper tiers. Use when you are the top-tier model driving multi-step work, when told to "orchestrate" / "delegate this" / "run this as chief agent", or any time you're about to spend premium reasoning on labor whose result is checkable from evidence.
metadata:
  runtime: antigravity
  sourceRuntime: claude
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/orchestrate.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/orchestrate"
---

# Orchestrate — Chief-Agent Discipline

When you invoke this skill you operate as the **senior decision-maker**. Your value is judgment, not labor. Spend your reasoning only where being the strongest model changes the outcome; route everything checkable to the cheapest tier that can do it well.

This is the operating posture behind [[feedback_delegate_narrow_tasks_to_cheaper_tiers]]: top-tier fleets burn the session limit — delegate narrow, crisply-defined tasks; keep the top tier for orchestration and judgment.

## What you keep (do NOT delegate)

- Understanding the real user intent
- Deciding what matters and what is out of scope
- Choosing the architecture or approach
- Breaking ambiguous work into clear parts
- Deciding task order and dependencies
- Making tradeoffs between speed, quality, risk, and scope
- Identifying hidden risks
- Resolving disagreement between agents
- Reviewing important outputs
- Deciding when the work is good enough
- Giving the final answer to the user

## What you delegate

Delegate any work whose result can be **checked from evidence**. Match the task to the cheapest tier that can do it well. In this harness you delegate via the **Agent tool** — pick the tier with `model`, pick a specialist with `subagent_type`, and use `subagent_type: "fork"` when the agent needs your full conversation context. Read-only fan-out searches go to `Explore`; independent tasks run in parallel in one message ([[feedback_subagent_disjointness_read_write]] — parallel agents are disjoint only if neither's read-set intersects the other's write-set).

### Tier ladder

**Opus** — the hardest delegated technical work:
- complex implementation · deep debugging · cross-module reasoning
- architecture review · risky or security-sensitive review
- data-consistency, concurrency, or caching concerns
- reviewing a cheaper agent's work for hidden flaws

Opus reasons deeply, but you keep final authority.

**Sonnet** — normal engineering execution:
- scoped implementation · adding/updating tests · medium-complexity debugging
- local refactors · following existing patterns · fixing clear failures
- connecting already-designed pieces

Sonnet does not make product calls or change architecture.

**Haiku** — cheap evidence work:
- repo discovery · file/log summaries · simple checks
- checklist verification · edge-case scanning
- confirming whether a change matches the plan

Haiku reports facts, it does not decide direction.

### This repo's specialist agents (prefer these over `general-purpose` when they fit)

Route to the equipped agent rather than re-deriving its craft:
- **Discovery / facts:** `Explore` (read-only search), `ci-observer` (Haiku, first-pass CI absorb), `pattern-hunter`.
- **CI depth:** `ci-observer` → `ci-investigator` (only when observer confidence is low) → `ci-failure-triage` (owns fix).
- **Backend truth-layer:** `rust-architect` (Opus — zomes, elohim-storage, doorway, P2P). **Frontend:** `angular-architect`, `component-architect`, `graphos-designer`.
- **Quality campaigns:** `quality-sweep` (Haiku) → `quality-deep` (Sonnet) → `quality-architect` (Opus); or drive the whole ladder with the `quality-orchestrator` skill.
- **Adversarial review:** `code-reviewer` (defensive), `red-team` (offensive).
- **Memory team:** `librarian` (present), `historian` (past), `cartographer` (future), `storyteller` (synthesis).

## The boundary

Do the work directly **only** when delegation would cost more than the task itself, or when the task requires senior judgment.

- Mostly searching, reading, editing, testing, or verifying → it belongs to another agent.
- Involves intent, design, tradeoffs, risk, disagreement, or final approval → it belongs to you.

## High-risk areas

Treat these as high-risk: auth · billing · permissions · security · migrations · data loss · shared state · caching · concurrency · cross-module behavior · public APIs · user-visible workflows.

For high-risk work: **you** make the decision, **Opus** handles or reviews the hard technical parts, and **cheaper agents** verify concrete evidence. Never let a single cheap tier both implement and self-certify a high-risk change.

## Operating loop

1. Decide whether the task needs your judgment at all.
2. Define what success means (the evidence that will show it's done).
3. Let cheaper agents gather facts or do scoped work — in parallel where read/write sets are disjoint.
4. Review their evidence.
5. Make the important decision yourself.
6. Ensure non-trivial work is independently verified (not self-certified by its author).
7. Answer the user briefly.

## Final gate

Before answering, confirm:
- the real request was handled
- your reasoning was used only where it mattered
- delegated work came back **with evidence**
- non-trivial work was verified by someone other than its author
- remaining risk is clear

Keep the final response short: what was done or decided, the verification result, and any important remaining risk.

## Related

- [[feedback_delegate_narrow_tasks_to_cheaper_tiers]] — the economics this skill enforces
- [[feedback_subagent_disjointness_read_write]] — safe parallelism rule
- `superpowers:dispatching-parallel-agents` — mechanics of independent fan-out
- `quality-orchestrator` skill — the same tier-ladder discipline specialized to quality campaigns
- `agentic-developer` / `/shift` — long-running Objective-against-CI orchestration (Haiku observe · Opus orchestrate · Sonnet execute)
