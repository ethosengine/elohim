---
title: "History/ADR: Landing the EPR substrate by waves — audit-as-truth, stubs that hold the seam"
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [epr, substrate, waves, audit-as-truth, stubs, federation]
# Sibling of 2026-06-01-dht-is-a-notary-not-a-byte-store.md: that record owns substrate
# PLACEMENT (where bytes live); this owns phase-EXECUTION (how a multi-phase substrate
# landed). Raw plan bodies retire to git.
distills:
  - genesis/docs/superpowers/plans/2026-04-21-elohim-epr-codec-crate-plan.md
  - genesis/docs/superpowers/plans/2026-04-22-elohim-epr-storage-foundation-plan.md
  - genesis/docs/superpowers/plans/2026-04-23-epr-phase-2c-libp2p-federation-plan.md
  - genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md
  - genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md
  - genesis/docs/superpowers/plans/2026-05-16-epr-foundation-closure.md
canonical:
  - ../../../superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md   # the successor substrate
  - 2026-06-01-dht-is-a-notary-not-a-byte-store.md                                       # sibling: substrate PLACEMENT
memory_anchors:
  - project_epr_substrate_vs_vf_graphql
  - project_principle_p1_reconciliation_controller
  - project_three_layer_truth_model
  - project_first_class_graph_pattern
---

# History/ADR: Landing the EPR substrate by waves — audit-as-truth, stubs that hold the seam

> **One-sentence lesson:** A large multi-phase Rust substrate (the EPR codec → storage → federation →
> manifest-resolver → gradient-signal stack) was delivered NOT by executing one plan box-by-box, but by
> a wave-sequenced fleet of sub-plans whose *real* state was reconciled by a periodic Wave-0 audit
> against live code — so plan checkboxes are tracking-debt, never ground truth.

**What was built/attempted (the arc).** Over 2026-04-21 → 2026-05-16 the EPR (Elohim Protocol Record)
foundation shipped in phases: Phase 1 codec (`elohim/epr/` crate — 12 modules:
cbor/cid/envelope/proof/reach/signature/validation + `@elohim/epr` TS package, cross-language parity via
committed fixtures); Phase 2A storage foundation (`db/epr_atoms.rs`, 6 REST routes, view schemas,
contract tests); Phase 2C libp2p federation (`/elohim/epr-atom/1.0.0` protocol, golden vectors); Phase
2B identity+projector+signal; Phase 3 manifest-resolver (standing-aware code paths returning
`Standing::Unknown` until lit); Phase 3.5 trust-compute gradient
(`FeedbackSignal`/`AttentionTending`/`CollectiveFilterPattern`, sealed-against-self predecessor records,
+3 DNA entry types); Phase 4 projector controller (`db/projection_events.rs`); and the Wave-2 runtime
tail (`record_predecessor` on the Announce path, IntegrityNotify KeyRotation/KeyRevocation arms in
`p2p/recovery_rotation.rs`).

**What superseded it / where it went.** The whole foundation was a *substrate handoff*. It is now LIVE
code (`elohim/epr/`, `elohim-storage/src/db/epr_atoms.rs` + `projection_events.rs`,
`p2p/epr_atom_protocol.rs` + `feedback_signal.rs` + `recovery_rotation.rs`) and its successor — the
**Graph-Native Projection Substrate** sprint (2026-05-16, design + plan in-tree) — explicitly builds on
these phases (Phase 3.7+4 folded). The remaining `@wip` a2o scenarios were dispositioned (lift /
defer-with-evidence) and the deferrals routed to graph-native.

**WHY we turned (the three load-bearing patterns to keep).** (1) **Audit-as-truth over
checkbox-as-truth** — three plans (codec/2A/2C) still show 0/117, 0/82, 10/64 ticked yet are confirmed
LANDED by the Wave-0 audit and the live crate; on a multi-agent wave fleet, a periodic re-audit against
code is the reconciliation controller, and unticked boxes are NOT a signal of incomplete work. (2)
**Stubs that hold the seam** — Phase 2A shipped `FederatedEprStore` as a `LocalEprStore`-delegating stub
and Phase 3 shipped `Standing`-typed signatures returning `Unknown`; the route/function contract froze
early so later phases (2C federation, 3.5 gradient) swapped the implementation at construction time
without touching callers. (3) **Parallel-shapes-then-reconcile** — the legacy `EprHead` (~500B) and the
generalized `Envelope` were kept deliberately parallel through 2A/2C rather than force-merged;
reconciliation was scheduled as its own concern, not smuggled into delivery.

**Watch-out for future planners.** When you inherit a "LANDED" substrate, trust the audit + the live
module list, not the plan's checkboxes — and do not "re-open" a phase because its plan looks unfinished.
Conversely, when YOU run a wave fleet: write the trait/signature seam first (it's what lets stubs land
safely), and schedule an explicit audit step to convert checkbox-fiction into recorded truth, because no
one will tick boxes for code confirmed retroactively.

## Bidirectional links

- **This record → canonical:** [graph-native projection substrate](../../../superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md) (the successor that builds on these phases).
- **Sibling:** [dht-is-a-notary-not-a-byte-store](2026-06-01-dht-is-a-notary-not-a-byte-store.md) owns substrate PLACEMENT; this owns phase EXECUTION.
- **Distilled-from (raw bodies in git history):** the EPR codec/storage/federation/gradient/master/closure plans (linked in frontmatter).
