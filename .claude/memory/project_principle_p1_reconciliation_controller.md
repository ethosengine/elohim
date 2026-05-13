---
name: Principle P1 — elohim-storage as reconciliation controller
description: Load-bearing architectural principle unifying EPR Phase 2B design and (claimed) future phases; k8s controller-manifest pattern applied to the three-layer truth model
type: project
originSessionId: 8c8c7e97-f63b-4df5-ae26-36e0fb18bcf7
---
The Holochain DHT is the authoritative manifest for identity/key/governance state. The libp2p/elohim-storage layer is a reconciliation controller over that manifest — k8s-controller-over-manifest pattern. Observed state changes → controller reconciles → no hesitation, no lazy acceptance.

**Why:** Named by the user during Phase 2B brainstorm (2026-04-24) as a reframe of "lazy mark-stale vs eager sweep." The user made the analogy explicit: "DHT is the manifest, the libp2p layer needs to react to that authority." This collapsed 8 separately-scoped coupling decisions into applications of one principle, which made items 5–7 (signal harness, write-through flag, Kad/gossip fanout) tractable in the same spec rather than deferred.

**How to apply:** When designing any state-synchronization layer between Holochain DHT entries and elohim-storage operational state, ask "what is the controller observing, and what does it reconcile to match?" Reject lazy-acceptance designs for integrity-critical state (revocations, rotations, bindings). Eager reconciliation is the correctness guarantee. Sweeps must be index-bounded, not table-scanning, and must be observable (reconciliation-lag metrics are first-class outputs). This generalizes beyond 2B — any future subsystem straddling DHT (A-notarized) and libp2p (C-operational) layers gets designed as one more controller loop, not a new paradigm.

Codified in `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md` §2 as Principle P1. The spec's Invariants I1–I9 (§5) are the concrete contract this principle implies for the projector specifically.
