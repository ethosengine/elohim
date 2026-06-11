---
id: shefa-pillar-gospel
cites:
  - shefa-domain-gospel | the subject SOURCE OF TRUTH this pillar consumes — REA primitives, stewardship-context metadata, cross-pillar coupling map (renders, never redefines) | sha256:ca13d4bc043c03cb | path: elohim/sdk/domains/shefa/CLAUDE.md
---

# Shefa Pillar — Economic Experience Layer

Shefa is the human experience of the Elohim Protocol's economic
infrastructure. It renders stewardship, banking, resource flows, and
compute sharing in ways humans can understand and interact with.

## Shefa is UX, Not Truth

The protocol primitives (economic events, commitments, agreements,
mutual credit) live on the Holochain DHT — distributed infrastructure
that no one can capture. Shefa services in this directory are the
**experience layer** that makes those primitives legible to humans.

The distinction matters: if an economic event is only recorded in an
Angular service's state, it can be lost, forged, or silently modified.
If it's notarized on the DHT and projected to storage with a
`dht_anchor_hash`, it's cryptographically provable. Shefa reads from
storage (fast), but writes should go through the conductor (truthful).

## Hazard: the tool's meta vs the EPRs it senses

Shefa is more than a UX layer — it is a **sensemaking tool OVER EPRs**, three fused surfaces on one
substrate: a **CMS** (the authoring lens — "how do I author the node": new doc / sheet / car / boat /
epr-app …), a **filesystem/namespace** for EPRs (organize, browse, name), and a **value-flow analytics**
view (the **R** in REA — resources, and how value flows through them). Think Google Drive × Mint/Monarch ×
Analytics, *for the protocol* — but a CMS whose content lives one layer DOWN: the EPRs are notarized on the
substrate; shefa authors *into* and projects *from* them, never owning them the way a normal CMS owns its
database. That inversion IS the hazard: **shefa confuses the meta its tool is concerned with (dashboards,
aggregations, the authoring chrome, the "filesystem" framing) with the underlying EPRs it makes sense
FROM.** The CMS instinct to own content is exactly the trap. When a shefa view feels like it *owns* a
number or a node, that is the smell — it should be reading the substrate, not authoring it. (The
sensemaking-tool form of "storage is projection, not truth"; every lens carries its own form of this hazard —
see the lens model in `2026-06-11-subject-routing-locus-graph-design.md` §2b.1. Shefa's own-session cleanup
must hold this line.)

## Subject home & citation discipline (this pillar is a CONSUMER)

This Angular pillar does not OWN the shefa subject — it consumes it. The protocol primitives
(`EconomicEvent`/`Agreement`/`Commitment`/`Resource`), the `stewardship-context` metadata, and the
cross-pillar coupling map are the source of truth at the cited subject home `shefa-domain-gospel`
(`elohim/sdk/domains/shefa/`). The cite is content-addressed: a change at the subject home drifts this gospel
STALE for re-verification.

**Where code citations to the subject belong:**
- `generated/` is DERIVED from the subject home — never hand-edit; regenerate with `pnpm run shefa:codegen`.
- When code encodes affinity accrual or demurrage, cite that earned standing requires demonstrated mastery +
  sustained curation, never attention or consumption — leave a `// subject: shefa-domain-gospel` breadcrumb.
- When wiring cross-pillar value flows (mastery-achieved → stewardship-eligibility, stewardship-allocated →
  steward-recognition), breadcrumb the coupling site so subject changes find dependents.
- When building resource-nature checks (rivalry/excludability/depletability/fungibility/circularity), cite the
  REA-extension classification the subject home owns.

## Service Categories

### API Services (thin HTTP clients to storage projections)
Services like `EconomicEventsApiService`, `ExchangeApiService`,
`FlowPlanningApiService` read from elohim-storage's HTTP API. These
are reading the **projection** of DHT truth — fast and queryable,
but not the source of truth.

For writes, these services should call through to the Holochain
conductor (via HolochainClientService zome calls), which writes to
the DHT and projects back to storage via post-commit signals. Direct
storage writes bypass the notary and create un-notarized records
(dht_anchor_hash = null).

### Composition Services (app-level logic)
Services like `InsuranceMutualService`, `BudgetReconciliationService`
compose multiple protocol primitives into domain-specific workflows.
These belong in the app, not the SDK — they're how this particular
app interprets the protocol, not the protocol itself.

### Transition State
Some services currently POST directly to elohim-storage. As the
conductor-first pattern is wired up, these should migrate to:
1. Write via conductor zome call
2. Post-commit signal projects to storage
3. Read from storage HTTP API (unchanged)
