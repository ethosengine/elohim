---
name: project_rea_valueflows_are_our_workflow_layer
description: Saga + habits + commitment:claim over EPR recipes ARE our platform-agnostic workflows — peer-native vs Linear tickets; the gap is projective (nothing re-reads it mid-run).
title: REA valueflows over EPRs are our workflow layer
metadata: 
  node_type: memory
  type: project
  originSessionId: 0a373f14-82a7-46f3-9198-a252328c16b5
  modified: 2026-08-13T13:44:04.096Z
---

Operator framing, 2026-08-13. Every team publishing on long-horizon agent runs improvised a
workflow container under time pressure — OpenAI took Linear tickets (Symphony), Anthropic a
feature-list JSON and later lock files in `current_tasks/`, Arize an in-RAM todo list. Each is
bound to its container. **Ours is REA valueflows over EPRs**: recipes as ValueFlows
ProcessSpecifications (`.claude/epr-meta/recipes.yaml`), `commitment:claim:<gap>` state,
the resiliency-SAGA's chapter/frontier ordering, admission-controlled by `habits.yaml`
(max 12, max 2 active, `unwired` as a declared state). Verified live 2026-08-13:
`epr flow status` → 556 active commitments, 539 unfulfilled, 410 sealed / 108 stale edges.

**Why:** this is the platform-agnostic, peer-native version of the thing the field keeps
re-inventing vendor-bound. It is checkable rather than merely instructive (a walk can call an
edge stale; a prompt template cannot), it carries provenance rather than a vendor audit log,
and any agent that can read the repo can read it. `habits.yaml`'s WIP fence and `unwired`
state have **no analog anywhere in the corpus**.

**How to apply:** when a task smells like "we need a plan file / ticket board / progress
tracker," the answer is almost always the existing commitment graph plus `habits.yaml`, not a
new register. The real gap is **projective**: Symphony reconciles ticket state around every
turn (only Arize re-injects state into every model call); we render habits once at session
start — so a mid-run correction dies at compaction unless it is written to the commitment
graph or `habits.yaml`. Treat conversation as history by definition. One plane-typing caution
(Codex-verified): Symphony's claim states are ephemeral scheduler reservations, our
commitments are durable economic promises — join the planes, don't conflate them.
Related: [[reference_horizon_scans]], [[feedback-decide-clear-calls-not-over-ask]],
[[feedback-subagent-disjointness-read-write]]. Full trace:
`genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md`.
