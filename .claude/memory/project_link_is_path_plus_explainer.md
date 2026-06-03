---
id: project-link-is-path-plus-explainer
name: link-is-path-plus-explainer
description: "A \"link\" in the elohim traceability/memory graph = a path PLUS a 1-2 sentence plain-text explainer of what's at the end; bare paths do not count."
metadata: 
  node_type: memory
  type: project
  originSessionId: 6d96f5fe-2184-4539-9ce1-dd7cae8d9d43
cites:
  - placement | the placement contract whose stasis check enforces this path-plus-explainer rule on every traceability edge | sha256:f84d7cb16bea9379
---

A link is **a path + a 1–2 sentence plain-text explainer of what's at the end of that path** — never a bare path or reference.

**Why:** the explainer makes every edge self-describing, so an agent reads *what's there* and decides whether to follow the path — spending context only on relevant links. This IS the "progressively surfaced context" the whole memory-coherence effort was started to deliver. It is the atom of the story→archetype→scenario→glue→impl→spec→doc→CLAUDE.md traceability graph.

**How to apply:** every `cites:`/`canonical:`/traceability edge (frontmatter links, gap-item cites, code↔story/archetype/scenario/CLAUDE.md/doc links) carries path + explainer. The history records (path + one-sentence gotcha) are the template. The stasis check's traceability dimensions require the explainer (present, 1–2 sentences), not just a path — `placement-audit.py --stasis`, and the stasis contract in `genesis/docs/PLACEMENT.md`. The loop's "done" = composite stasis score ≥ 0.85 (±15% of the 1.0 benchmark) AND the traceability/test-tag/claude-coverage dimensions wired.
