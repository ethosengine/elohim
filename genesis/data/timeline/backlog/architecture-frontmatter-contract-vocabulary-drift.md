---
id: "backlog-architecture-frontmatter-contract-vocabulary-drift"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Architecture frontmatter contract: declared tier:/status: vocabulary has drifted from practice"
slug: "architecture-frontmatter-contract-vocabulary-drift"
written: "2026-07-30"
author: "memory-ceremony"
status: "refined"
priority: "medium"
tags: [frontmatter-contract, architecture-index, vocabulary-drift, epistemic-status, gospel-currency]
cites:
  - genesis/docs/content/elohim-protocol/architecture/INDEX.md
  - genesis/docs/content/elohim-protocol/architecture/MAP.md
shift_objective: |
  Reconcile the architecture frontmatter contract's declared vocabulary with practice, on both
  axes at once. Decide (a) whether `reference` joins `architecture` as a declared `tier:`, and
  (b) what the `status:` contract actually is — a wider enum, an enum plus free-text qualifier,
  or a structured pair (lifecycle-state + `truth:` level). Then update INDEX's contract block and
  bring the outlier seeds into conformance. Done when INDEX's declared vocabulary admits every
  value actually in use, and a new seed can be authored correctly from the contract alone.
---

## What

`INDEX.md`'s frontmatter contract declares two vocabularies that its own directory does not follow.

**`status:`** — the contract template says:

```
status: <Draft | In-flight | Landed | Superseded>
```

Practice uses roughly **30 distinct values** across the architecture seeds. `In-flight` and
`Superseded` appear **zero** times. Real values include `Living document`, `reference`, `proposal`,
`Architecture principle`, `Architecture pattern`, `Design (canonical)`,
`Ratification-pending operational governance contract`, and several long free-text forms carrying an
embedded epistemic qualifier — `truth:VISION`, `truth:DERIVED`, `accepted — As-implemented
distillation`, `Landed (mechanical floor live in CI; single-DNA target HELD — not yet demonstrated)`.

**`tier:`** — the contract states every architecture doc MUST declare `tier: architecture`. Four do
not declare `tier:` at all: `2026-06-21-elohim-seam-map-concern-routing` and
`2026-07-12-substrate-trust-contract-runbook` (both `status: reference`), and
`2026-06-04-qahal-epr-household-lattice-design` and `2026-06-11-bloom-mastery-progression-design`
(both `status: Draft`).

## Why it is one decision, not two

Both axes drift for the same reason: the contract's vocabulary was fixed once and the corpus grew
past it. The `reference` docs are the clearest case — they are neither drafts nor landed specs;
they are operate-time and orientation surfaces, a *kind* the contract has no word for. That is the
same gap on both axes, so reconciling `tier:` without `status:` would just move the problem.

The `truth:` qualifier is the load-bearing part. A seed that declares `truth:VISION` describes a
mechanism that is **designed but not built**, and the 2026-07-30 ceremony found exactly this trap
live: `upgrade-revert-and-constitutional-consensus` is trivially misread as shipped capability by an
agent that reads the title and skips §11. Two clauses were added to INDEX's contract making
`status:` load-bearing and requiring the qualifier be carried across an `informed-by:` edge — but
those clauses now name a form the template three lines above them does not admit. Closing that is
this item.

## Constraint

This touches every seed's frontmatter, so it is a contract change requiring a decision before any
sweep — deliberately NOT done as part of a currency pass. Whatever is chosen must let a new seed be
authored correctly from the contract alone, which is the property the current contract has lost.

## Provenance

`/memory-ceremony` 2026-07-30, Phase-1b MAP/INDEX currency work. Related:
[[feedback_agent_prompts_no_process_status]] (gospel prose describes stable architecture — a
`status:` field is the sanctioned home for state, which is why getting its vocabulary right
matters), and [[feedback_verify_the_measure_before_the_ranking]].
