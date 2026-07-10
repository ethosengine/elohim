---
id: mishpat-domain-gospel
cites:
  - elohim-protocol-specification | the shared protocol substrate this governance domain validates decisions over — qahal escalates into mishpat judgment | sha256:659b0d47078b298f | path: genesis/docs/content/elohim-protocol/protocol-specification.md
---

# Mishpat Domain

The **mishpat (judgment / governance) design domain** — the validation layer: proposals, challenges,
gate-decisions, deliberation, attestations. Mishpat is the notarized *validator* of community decisions —
qahal is the app/social governance surface; mishpat is the judgment substrate it escalates into.

## Layer below (what this domain assumes)

Mishpat composes ON the shared protocol substrate (the cited `elohim-protocol-specification`) — it adds
governance-actions, attestations, and observation kinds, but does not redesign the substrate. Its
plans/specs work the layer above; a substrate change drifts this gospel STALE. Vocabulary lives in
`manifest.json`; the running implementation is the mishpat DNA.

## Owns

- `governance-actions` — the validated decision/challenge/appeal verbs
- `attestations` — judgment-bearing claims
- `observation_kinds` — governance observation vocabulary

## See Also

- Domains pattern: `elohim/sdk/domains/CLAUDE.md`
- Implementation: `elohim/holochain/dna/mishpat/` (under `holochain-integrity-layer-gospel`)
- Substrate: `elohim-protocol-specification` (the layer below — assumed, not redesigned)
