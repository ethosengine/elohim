---
name: feedback-limit-raises-are-design-signals-not-capacity
title: "Limit raises are design signals, not capacity"
description: "Operator 2026-09-13: a conductor limit raise is an emergent loop in a capacity costume — run a VSM design pass."
metadata: 
  node_type: memory
  title: "Limit raises are design signals, not capacity"
  type: feedback
  originSessionId: dcddc033-024b-4dfa-8d13-39fa5f75b9ef
  modified: 2026-09-13T13:27:30.462Z
---

**What the operator said (2026-09-13, morning after the pipeline-e2e shift):** "do those cpu limits seem a bit high
for a total genesis payload being synced around the network of 25MB?" then "add an epr-meta guard that fires on those
limits to consider what problem you're really trying to solve.. we're trying to make refinements and maximize
performance and resiliency.. those limits creeping up to patch over an emergent problem that needs a design pass"
and "that epr-meta guard should be pretty robust being vsm aware as well."

**What I had done:** drafted matthew 4000m→10000m and adam 8000m→12000m in deployments.json on the strength of a
live read (both conductors ~99% CFS-throttled, storage admission permits derived from the storage pod's CPU). The
tree already carried five prior raise stamps on those units, each saying "band-aid" and naming a targeted lever.
Reverted on the operator's question; the same hour Loki showed one leecher logging ~5.4M publish-queue-full
refusals/hour, each closing the peer connection (kitsune2_api transport.rs:244) — the storm the fleet-saturation
backlog (2026-09-11) already named.

**Why:** the corpus size is the oracle. When the work on the data is small and the burn is large, the burn is a
feedback loop, and a limit raise feeds the loop (Ashby: absorb variety at the operational unit instead of
attenuating it at the disturbance's source). In VSM terms the raise IS the algedonic signal and must reach the
design plane (S3/S4), not be answered by S1. Related: [[feedback_inherit_substrate_ontology]] (VSM/Ashby vocabulary
lives in elohim/epr/src/algedonic.rs and the 2026-08-10 algedonic spec), [[project_conductor_arc_resources]],
[[project_fleet_conductor_admission_ceiling]].

**How to apply:** before any `edgenodeCpuLimit`/`edgenodeMemoryLimit`/`limits:` raise, ask what disturbance the
limit would absorb and which seam we own attenuates it at the source (fork patch, boot profile, reconcile ramp,
sequenced roll). Count prior raise stamps on that unit; two or more means a design pass is due, not a third raise.
The compose-gate rule `resource-limit-raise-design-signal` (added 2026-09-13) fires an `ask` carrying those
questions; answer them in the design surface, not in the ack.
