---
id: elohim-domain-gospel
cites:
  - elohim-protocol-specification | the shared protocol substrate this cross-cutting domain declares coordination over — signal kinds and constitutional ratios spanning pillars | sha256:659b0d47078b298f | path: genesis/docs/content/elohim-protocol/protocol-specification.md
---

# Elohim Domain

The **elohim cross-cutting design domain** — substrate-level coordination primitives and the `signalKinds`
that span *all* pillars. Unlike the pillar domains (lamad/qahal/imagodei/shefa), elohim declares no content
types; it owns the cross-pillar coordination vocabulary.

## Layer below (what this domain assumes)

This domain sits CLOSEST to the shared protocol substrate (the cited `elohim-protocol-specification`) — it
declares the cross-pillar coordination the substrate exposes, but does not redesign the substrate itself. A
designer working a pillar domain assumes these declarations the way a web developer assumes HTML; when the
substrate changes, this gospel drifts STALE. Vocabulary lives in `manifest.json`.

## Owns

- `constitutionalRatios` — cross-pillar invariants
- `signalKinds` — substrate signals that span pillars (attention / compute / storage / bandwidth / …)
- `observation_kinds` — cross-pillar observation vocabulary

## See Also

- Domains pattern: `elohim/sdk/domains/CLAUDE.md`
- Substrate: `elohim-protocol-specification` (the layer below — assumed, not redesigned)
