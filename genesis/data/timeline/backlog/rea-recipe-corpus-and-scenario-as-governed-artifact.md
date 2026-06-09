---
title: "REA recipe corpus + scenario-as-governed-artifact — the ex-ante / experiment layer"
created: 2026-06-09
domain: "design"
tags: [rea, valueflows, process-specification, recipe, intent, commitment, proposal, scenario, forecasting, beer, p2p-design-gate]
cites:
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md
  - genesis/docs/superpowers/specs/2026-06-09-coupling-delay-observed-governed-primitive-design.md
  - genesis/docs/superpowers/specs/2026-05-28-app-manifest-staged-intents-design.md
---

# Recipe corpus + scenario-as-governed-artifact

Two coupled breadcrumbs that gate the forward model
(`experimental-forecasting-flowchart-simulation-engine.md`) and its experiment loop. Both
are **design/authoring gaps, not missing primitives** — the REA/ValueFlows three-layer
ontology already names the slots.

**(a) Recipe corpus — designed throughput per process kind.**
- *Composes from:* the type-level **ProcessSpecification / RecipeFlow** slot (input→output
  ratios — "this kind of process turns 3 of A into 1 of C"). This is Beer's nameplate
  box-size at the template level, and the standing-capacity half of "relative box size".
- *Missing:* the corpus itself. The slot exists in the ontology but is thin/unpopulated, so
  forward dynamics are only as sharp as the recipes authored. Needs an authoring + curation
  surface. (The other half of box-capacity — an agent's *standing* bandwidth — is already
  supplied by `reach`/`standing`; recipe × reach is the full ex-ante box size.)

**(b) Scenario-as-governed-artifact — Beer's operations-room committee debating futures.**
- *Composes from:* the experiment loop is mechanically doable TODAY — run the existing
  projection over a counterfactual **Intent** set (Intent = a proposed flow not yet
  committed; the experimental knob). **Proposal** (a ValueFlows bundle of Intents) is the
  natural scenario container. Progression Intent → Commitment → Event is the native
  promote-or-discard path (author Intents → project forward → compare to the committed
  trajectory → promote the good ones to Commitments).
- *Missing:* promoting "scenario" from an **ephemeral private projection** to a **first-class,
  shareable, governed, comparable artifact** — who may author one, how two scenarios are
  compared, how a scenario ratifies into Commitments. Intent/Proposal is the substrate slot;
  making Scenario a governed object is the design choice.

**Preconditions:** needs its own **p2p-design-gate** — is `Scenario` a new entry type, a
`Proposal` subtype, or a tagged Intent-set? (Notarized-A vs derived-A2 vs operational-C.)
Reach-enum reconciliation (roadmap #13) sits underneath any standing-capacity wiring.

**Why these two together:** (a) gives the forward model resolution; (b) gives it an
experimental *interface*. Without (a) the simulation is structureless; without (b) every
forecast is a private guess instead of a debatable, ratifiable proposal — which is the
whole point of Beer's "make the flow visible and let distributed agents coordinate."
