---
name: feedback-master-peer-compute-before-hardware
title: "Master peer compute before hardware"
description: "Operator 2026-10-29: elegant peer utilization (brit/lvi/peer/rakia subsuming GitHub, Nexus, Harbor, Jenkins) before any new node; bites when a capacity gap tempts a purchase."
metadata:
  node_type: memory
  title: Master peer compute before hardware
  cites:
    - genesis/docs/superpowers/specs/2026-10-29-closure-keyed-landing-plane-design.md
    - elohim/rakia/docs/plans/stage-2-canopy.md
  type: feedback
  originSessionId: dff40885-cdc7-4d21-a991-c99cd24d3e06
  modified: 2026-10-29T13:11:17.412Z
---

Operator, 2026-10-29 (overnight integration-capacity shift), on being told the media PC (Ryzen 5,
16 GB, no TLC drive yet) could carry gates: "whatever we can do to improve the elohim native
developer CI/CD design over peers to subsume github, nexus, harbor, jenkins.. is worth doing now
regardless of when I can supply the compute. We should master this rather than throwing hardware
at a problem that would just benefit from more elegant peer utilization of compute." And the
target story: "onboarding a new dev with one workstation natively (brit, lvi, peer) they could just
fork a lvi workspace, and they join the cluster.. and are available to run integration jobs if
they leave their peer on in the network."

**Why:** the measured bottleneck (grounding record `genesis/research/integration-capacity-grounding-2026-10-29.md`)
is a closure paid three times, not a shortage of cores; a new node widens nothing until the
landing plane keys work on closures and lets peers carry gates. This is rakia Stage 2 (Canopy)
stated as an onboarding moment.

**How to apply:** sequence design and sprints by what removes a payment or adds a peer rung
(closure-keyed builds, sibling-verdict reuse, the carry, device-written attestation, bundle custody),
never by what a purchase would enable. A hardware ask is read the way [[feedback_limit_raises_are_design_signals_not_capacity]]
reads a limit raise: a design signal first. Enlisted machines enter as enrolled devices
(`epr actor device enroll`, `epr peer serve`) and carry what their own guard lines admit;
they are never Jenkins agents. Related: [[feedback_local_mesh_first_cadence_and_measure_offload]],
[[project_rakia_as_artifact_store_custody_registry]].
