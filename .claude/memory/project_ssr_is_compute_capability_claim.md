---
name: SSR is a compute-shape capability claim, not a deployment detail
description: A doorway with SSR enabled makes a real compute claim (V8 + bundle + render CPU); should be feature-gated locally and advertised through peer compute-reporting so peers can match SSR-eligible content to SSR-capable doorways
type: project
originSessionId: cf962313-d70a-459d-acb7-925c8f19e9e1
---
A doorway running server-side rendering is materially different from one that doesn't: ~200MB cold-start working set parsing the 51MB Angular bundle, per-render CPU cost, V8 isolate maintained per replica. Today (2026-05-08) every alpha doorway carries the runtime regardless of whether the operator opted in, and the substrate has no way to know which peers' doorways can SSR which content.

**Why:** P2P design-gate audit of the doorway-ssr-deliver shift surfaced this as a substrate-level gap. SSR is the same kind of diversity surface as `project_compute_and_model_independent_diversity_surfaces` flags — it belongs in peer compute-reporting, not just doorway image config. Without it, the substrate can't route an SSR-eligible request preferentially to a doorway that can serve it; it just fires-and-forgets and lets the receiving doorway fall through to CSR if it can't render. That's fine for correctness, wasteful for substrate-level matchmaking.

**How to apply:**
- Treat any future SSR work (bundle changes, new framework adapter, per-domain SSR opt-in) as a capability-claim change, not just a deploy-config change.
- When a doorway runs SSR, its peer-status / compute-report should declare it: which route families, what bundle version, concurrency budget.
- elohim-storage matchmaking should treat SSR like model-availability — a discriminator when choosing a peer for content known to be SSR-eligible (per its build_manifest annotation).
- Don't mint a new DHT entry type for SSR capability — extend existing peer-status / capability gossip (lamad is ~73/~100, mishpat ~11/~100, no headroom for vanity entries).
- Brainstorm prompt for the cohesive design: `.claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/brainstorm-prompt-followup.md` (covers feature-gate + capability-advertisement + auth threading as one design, not three).
