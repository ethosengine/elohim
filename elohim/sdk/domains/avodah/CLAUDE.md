---
id: avodah-domain-gospel
cites:
  - elohim-protocol-specification | the shared protocol substrate this work domain composes ON (ContentNode + REA + reach) — assumed, not redesigned | sha256:659b0d47078b298f | path: genesis/docs/content/elohim-protocol/protocol-specification.md
  - shefa-domain-gospel | the economic domain whose primitives avodah work creates value through (demonstrated work → stewardship-eligibility) | sha256:ca13d4bc043c03cb | path: elohim/sdk/domains/shefa/CLAUDE.md
---

# Avodah Domain

The **avodah (work) design domain** — work management as protocol *composition*. Avodah introduces no new
primitives: `work-story` and `work-project` are `ContentNode` + metadata + REA events. It is the action
half of the lamad↔avodah loop, and the place where value in shefa is created.

## Layer below (what this domain assumes)

Avodah is a **design domain** — it composes ON the shared protocol substrate (the cited
`elohim-protocol-specification`) and the **shefa** economic primitives; it does not redesign either. Its
plans/specs work the layer above, the way a web developer builds *on* HTML without authoring the W3C spec.
When the substrate (or shefa) changes, this gospel drifts STALE — the signal to re-verify the assumptions
the work rested on. Vocabulary + coupling live in `manifest.json`; types in `types/`.

## Vocabulary

- `work-story` — a unit of contribution (ContentNode + work metadata + REA events)
- `work-project` — a coordinated body of work

Avodah is a D1 demonstrator: it proves the protocol needs no work-specific primitives. Mastery (lamad)
gates capability; demonstrated work (avodah) produces stewardship-eligibility value (shefa).

## See Also

- Domains pattern: `elohim/sdk/domains/CLAUDE.md`
- Substrate: `elohim-protocol-specification` (the layer below — assumed, not redesigned)
