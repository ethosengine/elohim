# Distribution + Resilience Substrate Sharing — Backlog Stub

**Date:** 2026-05-03  
**Phase:** 10 (post light-up-the-topology)  
**Linked Design:** [Distribution + Resilience Surfaces — Coherence Design](./2026-05-03-distribution-resilience-coherence-design.md)

## Overview

The light-up-the-topology sprint (Phase 7–9) completed the first dimension of
operator visibility: the coherence design landing distribution and resilience
widgets side-by-side in content-viewer header.

This backlog stub captures the next phase of work: **substrate-sharing metrics
and incentives**. Once operators can *see* distribution and resilience state
ambiently, the protocol needs mechanisms for:

1. **Peer-side action loops** — When a peer sees they're carrying unique
   replicas (resilience cliff), what actionable signals do they get?

2. **REA-backed placement gates** — When a household has satisfied their
   stewardship commitments, how do they opt out of further distribution without
   breaking reach guarantees for content they've stewarded?

3. **Collective economic signals** — When placement gaps open, how do
   collectives (via shefa coordinators) decide whether to recruit new stewards
   or reduce reach?

## Scope Boundaries

**IN Scope (for Phase 10 planning):**
- `PlacementGap` as a first-class signal type (currently projection-only)
- Signal aggregation in shefa coordinator layers
- User-facing prompts when peers detect resilience cliffs
- Policy-driven handoff mechanisms (REA commitment completion → distribution opt-out)

**OUT of Scope (defer to later phase):**
- Complete shefa recruitment surfaces (those are big)
- Cross-household resilience optimization (cadence, coordination)
- Payment/incentive mechanisms for elevated stewards
- Doorway-side availability prediction

## Related Memory

- [Placement signals are shefa inputs](../../memory/project_placement_signals_are_shefa_inputs.md)
- [DePIN contracts are policy](../../memory/project_depin_contracts_are_policy.md)
- [Household horizontal scaling](../../memory/project_household_horizontal_scaling.md)

## Next Steps

This stub is a placeholder. Open the next feature-planning session with:

1. Read the coherence design above (distribution ≠ resilience)
2. Run `/p2p-design-gate` on PlacementGap entity classification
3. Brainstorm shefa coordinator action loops that consume PlacementGap signals
4. Capture scenarios in `genesis/a2o/features/shefa/substrate-sharing-incentives.feature`
