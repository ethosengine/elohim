---
name: feedback_inherit_substrate_ontology
title: Inherit substrate ontology, never duplicate it
description: "Inherit Meadows/Beer/Ashby/ValueFlows ontology from elohim-epr, epr-rea, elohim-compute before minting any record in a new crate — bites at every new type"
metadata:
  type: feedback
  originSessionId: 08dda108-5eac-4580-8178-d1bade78f0ab
  modified: 2026-09-02T18:42:00.412Z
---

Operator direction (2026-09-02, during the ark S0 build): be mindful to inherit the substrate's
properties as we continue — Donella Meadows (stocks, flows, limits), Stafford Beer (VSM, algedonic
signals), Ashby (requisite variety), Lynn Foster / ValueFlows REA (intent → commitment → event,
process, resource) — in lieu of any bespoke duplication of what could be inherited more simply.

**Where the substrate already speaks it (verified 2026-09-02):** `elohim/epr-rea` (`model::{Intent,
Commitment, Bound, Sense, LimitSource, Composition, Process, ProcessSpec, FlowEvent, PinnedRef,
AgentRef}`, `stock::{Stock, Window}`, `store::{FlowRecord, FlowStore, SidecarFlowStore}`, `fold`,
`actor`), `elohim/epr` (`algedonic::{AlgedonicSignal, AlgedonicEvidence}`, `witness::{WitnessedInteraction,
ReaVerb, Magnitude}`, `measure::{Quantity, MeasureKind, Interval, Confidence}`, `Reach`, `EprKind`),
`elohim/elohim-compute` (`Governor`, `Refusal`, `LimitOwner`).

**Why:** a limit is a `Bound` on a `Commitment`; a tally is a `Stock` with a `Window`; a
write-ahead decision is a VF `Intent`; a crash-loop give-up is algedonic `Breach` evidence whose
`bound_ref` is the self-contract's CID; an incident is a VF `Process`. Re-minting these as
`DeathTally`, `IntentAction`, `Incident`, `BoundedBy` drifts the ontology and doubles the surface
(the BritCid/BlobCid precedent; `elohim/.epr-meta` interface-first rule).

**How to apply:** before adding any record/enum in a new crate, grep the three substrate crates
for the concept; classify INHERIT (use the type) / SPECIALIZE (wrap, adding only what the substrate
lacks) / KEEP (novel — justify in a doc comment naming what was checked). Run this audit at
task-boundary time, not after a spool format freezes. See [[project_tevah_compute_envelope_canonized]],
[[project_rea_valueflows_are_our_workflow_layer]], [[project_epr_flow_valueflow_projection]].
