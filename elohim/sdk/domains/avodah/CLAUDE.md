---
id: avodah-domain-gospel
cites:
  - elohim-protocol-specification | the shared protocol substrate this work domain composes ON (ContentNode + REA + reach) — assumed, not redesigned | sha256:659b0d47078b298f | path: genesis/docs/content/elohim-protocol/protocol-specification.md
  - shefa-domain-gospel | the economic domain whose primitives avodah work creates value through (demonstrated work → stewardship-eligibility) | sha256:ca13d4bc043c03cb | path: elohim/sdk/domains/shefa/CLAUDE.md
---

# Avodah Domain — the Process Lens

The **avodah (work / service) design domain** is the protocol's **process lens**: *the process as canon*.
There is ONE shared core — EPR content atoms + REA events + governance/reach — and each design domain
reinterprets that core through a cohesive lens. Avodah's lens is **coordination-as-process**: who is doing
what, what work completes, what flows are planned, what risks are pooled. Think the Cybersyn control room —
a sensemaking surface over the protocol's own operations. The same `EconomicEvent` that shefa reads as value
flow and lamad reads as attributable contribution, avodah reads as **process telemetry**.

## One core, many lenses (where avodah sits)

lamad = learning/attribution over the nodes · shefa = authoring + value flow ("how do I author the node") ·
qahal = the steward social graph · mishpat = consensus, hygiene, and limits on the rest · imagodei = identity
ground · **avodah = the process view**. Lenses overlap **by design**: mutual risk-pooling appears here as
*process* (`CoveragePolicy`, `MemberRiskProfile`) and in shefa as *value* (mutual-credit, premium gates) —
same core primitive, two cohesive readings. Overlap between lenses is the model working, not duplication.

## Two strata in this domain

**1. The work-board vocabulary** (`manifest.json` — the app-visible demonstrator slice):
`work-story` and `work-project` are ContentNode + metadata + REA coupling — **no new primitives**; the slice
exists to prove the substrate needs no work-specific entry types. Its REA footprint flows into shefa's
accounting: `work-capacity` (consumed via `use`), `work-output` and `project-milestone` (produced via
`produce`), with recognitions `work-credit` and `stewardship-standing`. Work earns standing rather than
asserting it — claims carry validity horizons graded by negative observations (`task-completes-within-cadence`
P7D vs `cadence-overrun`; `project-delivers-stated-purpose` P90D vs `project-stalled`; `cadence-sustainable`
P30D vs `cadence-burnout`) — the same value+governance+feedback machinery every lens uses.

**2. The process wire types** (`types/` crate, wired into the `content_store` zome — notarized):
the control room's fuller vocabulary — service coordination (`ServiceRequest`/`ServiceOffer`/`ServiceMatch`),
flow planning (`FlowPlan`/`FlowBudget`/`FlowGoal`/`FlowMilestone`/`FlowProjection`/`FlowScenario`/
`RecurringPattern`), and mutual risk pooling (`CoveragePolicy`/`InsuranceClaim`/`MemberRiskProfile`/
`AdjustmentReasoning`). These are not misfiled economics — they are the **process reading** of coordination
whose value reading lives in shefa. The work-board renders none of them yet; they are the lens's substrate
ahead of its surfaces.

## Layer below (what this domain assumes)

Avodah composes ON the shared protocol substrate (the cited `elohim-protocol-specification`) and the **shefa**
economic primitives; it does not redesign either — the way a web developer builds *on* HTML without authoring
the W3C spec. When the substrate (or shefa) changes, this gospel drifts STALE: re-verify the lens's reading of
the changed primitive. Vocabulary + coupling are the source of truth in `manifest.json`; where this prose and
the manifest disagree, **the manifest wins**. Wire types live in `types/` (move all `#[derive(TS)]` types
atomically — a partial cross-crate move emits broken import paths). Any NEW process entity must pass the
`p2p-design-gate` (prove its source-of-truth category before designing a route).

## The lens hazard

Every lens risks confusing its reinterpretation with the core it senses. Avodah's form: **the control room is
a view over the events, not the events** — board state, flow projections, and matching dashboards are
projections of notarized process; the moment a view *owns* a number the substrate doesn't carry, the lens has
forked the truth. (Shefa's form of the same hazard: the CMS confusing its meta with the EPRs it authors.)

## See Also

- Domains pattern: `elohim/sdk/domains/CLAUDE.md`
- Substrate: `elohim-protocol-specification` (the layer below — assumed, not redesigned)
- Sibling: `shefa-domain-gospel` (the value reading of the same core; avodah work feeds its accounting)
- Rendered slice: `app/elohim-app/src/app/avodah/CLAUDE.md` (`avodah-pillar-gospel` — the work-board)
