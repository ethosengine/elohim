---
id: "backlog-clusters-index"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Backlog Clusters — the index of subject-scoped idea sinks"
slug: "backlog-clusters-index"
written: "2026-08-04"
author: "claude (cluster discipline, operator-directed)"
status: "backlog"
priority: "medium"
tags: [backlog, clusters, index, provenance, research-mint-pass]
---

# Backlog Clusters — subject-scoped idea sinks

The backlog's job is to **graduate concerns into active sprints**, not to accumulate. Not every
entry will ever be acted on — so like concerns cluster into subject files, each the *single*
re-surfacing point for its subject: one ranked table, per-item graduation targets, items lifted
into shifts individually. A standalone entry is for an operationally-atomic concern (one bug, one
incident, one bounded task); anything with siblings belongs in a cluster. Research engagements
(surveys, retrospectives, confrontations) close with a **mint pass**: surviving take-items fold
into the matching cluster citing the survey's `epr:` slug — takes not worth a cluster row die
honestly in the survey prose.

**Re-surfacing:** groom this table when a cluster changes; `grep -c '^| [0-9]' <file>` counts a
cluster's open rows. Status lives in each cluster's frontmatter + per-row notes — this index is
the map, never the content.

| Cluster | Subject | Rows | Groomed | Notes |
|---|---|---|---|---|
| [arch-dataplane-refactor-backlog](epr:arch-dataplane-refactor-backlog) | Dataplane *internal* reshaping — reuse, IoC seams, `p2p/mod.rs` decomposition chain (10→12→15), dev QoL, head-plane scale | 16 | 2026-08-08 | Row 16 (composite-root head class) added from the 2026-08-08 sync-cost evidence; p2p-design-gate mandatory on pickup |
| [arch-workspace-discipline-backlog](epr:arch-workspace-discipline-backlog) | Crate/workspace discipline — lints, licensing, versioning, CI gates, extraction sequence (p2panda-derived) | 12 | 2026-08-06 | Items 2 + 9 need operator decisions (license policy; root LICENSE absent) |
| [arch-dataplane-borrows-backlog](epr:arch-dataplane-borrows-backlog) | *Externally-sourced* dataplane mechanisms from surveys (Holepunch/SSB/p2panda/sedimentree) — each p2p-design-gated | 9 | 2026-08-07 | Sibling of refactor cluster: borrows vs reshaping |
| [arch-confidentiality-plane-backlog](epr:arch-confidentiality-plane-backlog) | The unbuilt encryption layer (§3.13) — fail-closed classifier, KeyEnvelope, p2panda-encryption candidate, X25519 substrate, ciphertext relay | 7 | 2026-08-07 | #1 is immediate; #2's audit-check gates the design fork |
| [measure-family-borrows-backlog](epr:measure-family-borrows-backlog) | *Externally-sourced* observation procedures + their invariants, and the fold they ride — measure families over EPRs, unit-agnostic by design (Playnet-derived) | 11 | 2026-08-05 | Row 1 (harden `epr-rea`'s fold) is the prerequisite for every other row; composes with [middot](epr:middot-measure-primitive-design) |
| [design-legibility-borrows-backlog](epr:design-legibility-borrows-backlog) | *Externally-sourced* devices for making a quantity felt — palette registry, tension render, area-preserving surface (Playnet-derived) | 7 | 2026-08-05 | Render-half sibling of the measure cluster; rows 2–3 must land paired with their measure halves |
| [commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog) | How a holon holds standing — custody≠ownership (VF rights/custody split), steward-as-path, nested per-holon elohim ceilings, credential-as-lens | 9 | 2026-08-05 | Rows 1–2 are Playnet borrows; 3–8 our design frontier. Row 1 has **zero** implementation in-tree. **Row 2a (`subjectStanding`) + the row-2 carve-out are red-team-derived and operator-gated** — see the inalienable red-team spec |
| [arch-dataplane-sdk-proposal](epr:arch-dataplane-sdk-proposal) | Dataplane SDK surface (Artifact 2 of the 2026-06-11 review) | — | 2026-06-11 | Proposal-shaped, not a ranked table |
| [agentic-context-tooling-consolidation-queue](epr:agentic-context-tooling-consolidation-queue) | Agentic tooling consolidation | — | — | Pre-existing queue-shaped entry |

Candidates for future clustering (standalone entries with visible siblings): the `a2o-*` family;
the `alpha-*` incident family (some are chronicle-shaped, not backlog-shaped); the recovery/identity
`agent-peer-binding-*` pair.

## The provenance chain (storytelling as compression)

A concern that travels the whole pipeline leaves a walkable story, each hop content-addressed:

```
research (messy desk)          survey closes with a MINT PASS
   └─ cites →  cluster row     graduates when picked for work
        └─ cites →  spec/plan  (p2p-design-gated where it touches entities; decomposes to gaps)
             └─ cites →  code + a2o scenario   (shift lands it; story-harvest preserves constraints)
                  └─ cites →  chronicle entry  (historian compresses the arc at close)
```

Every hop cites *backward* via `epr:` slugs (content-addressed — survives file moves), so the
chronicle entry at the end is the **compressed story**: reading it and following cites reconstructs
the full why — which survey found it, which cluster held it, which spec shaped it, which commits
landed it, which scenario guards it. The row's `status` (the shared delivery-status axis) is the
chain's live position; `epr flow walk` renders the same chain as a valueflow. A row with no forward
cite is *waiting*, not lost; a spec citing no row is the smell (where did it come from?); a landed
change with no chronicle is a story not yet compressed. Enforcement is deliberately light: the
`.epr-meta` here nudges cluster-first at birth; the rest is convention until a measure earns its
headline token.
