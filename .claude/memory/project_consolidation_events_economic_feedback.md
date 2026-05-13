---
name: Consolidation events as first-class economic-social feedback
description: Consolidation/merge events are not janitorial — they emit structured signals to shefa (rewards/restitutions), qahal (governance), mishpat (restitution/reach), imagodei (relationship); discovering equivalence pays the discoverer + frees compute (shardable); discovering bad-propagation triggers restitution + reconciliation + reach-level drop; the substrate's nervous system runs partly on consolidation events
type: project
originSessionId: 10d85ef0-1979-4311-97e9-c2c209de48e2
---
A consolidation event (merge, dedup, equivalence-attestation) is **a first-class moment of judgment** with economic, social, and structural consequences. Treating it as a silent graph rewrite forfeits the protocol's most signal-rich operation.

Every consolidation event emits structured signals to four pillars:

**Positive feedback (discoverer + ecosystem):**
- *Shefa* — reward flows to the discoverer for the act of finding the connection (truth-finding is valuable work, deserving REA-event recognition)
- *Compute economics* — freed stewardship capacity (sharding becomes possible once N replicas can collapse to fewer, distributed by reach/governance/resiliency); the saved compute is itself an economic good redistributable across the household network
- *Relationship* — discoverer-agents accrue shared-insight history; trust accrues between agents who recognized the equivalence together

**Negative feedback (when consolidation reveals bad-propagation):**
- *Mishpat / restitution* — if X propagated content that turns out to be misinformation/malware/harm, X owes restitution to those who received it through them; non-optional
- *Reconciliation* — relationships strained by bad propagation need active repair; the protocol must offer the path, not just record the breach
- *Reach drop* — reach earned at authoring is reversible at consolidation; propagating-bad costs reach, and the cost lands at the moment of discovery
- *Quarantine* — the discovered-bad must be contained, not just relabeled; quarantine is structurally distinct from forget (forgetting a known-bad is irresponsible)

**Why this is huge:**

The protocol's living systems already require feedback to function (per "social reach is a sense-respond nervous system"). Consolidation events are among the richest signal-emission moments in the protocol because they simultaneously convey:
1. A truth-discovery (the graph fact `a = b` — or `a = bad`)
2. An economic shift (whose compute is freed, who pays restitution, what flows where)
3. A relationship update (who is now in shared-insight relation, whose trust took a hit)
4. A reach update (earned increments, reversal decrements)
5. A governance moment (who had authority over this consolidation, did they exercise it well)

Treating merge as a silent rewrite forfeits all five.

**Connections to existing project principles:**
- *Reach earned at authoring* — extended: reach is also *lost via discovered bad-propagation*; same accounting symmetric.
- *Social reach is a sense-respond nervous system* — consolidation events are the explicit back-prop signal class (restitution + quarantine + reach update).
- *Trust as efficiency signal* — consolidation events update the trust signal at every node touched by the consolidation, in both directions (discovered-good earns efficiency for adjacent stewards; discovered-bad costs them).
- *DePIN contracts are policy* — consolidation outcomes are governance events that feed REA commitments and obligations.
- *Placement signals are shefa inputs* — discovered-bad is a structural placement signal: this content does not belong here, redistribute or quarantine.
- *Three-layer truth model* — consolidation events get notarized (DHT for the truth-finding), distributed (libp2p for the consequences), projected (doorway for human-visible accounts of restitution/reach).
- *P2P inventory exchange ≠ byte replication* — consolidation lets the network replicate the *fact* of equivalence (lightweight) without re-replicating bytes (heavy); the equivalence attestation is the new shardable record.
- *Storage as actor vs forwarder* — forwarders take on responsibility when they forward; consolidation that reveals bad-content makes their accumulated forwarding into a restitution liability.
- *Bootstrap social → elohim-integrated security gradient* — at Stage 3, consolidation events are first-class enforcement signals; at Stage 1, they're advisory but observable.

**Why:** the protocol cannot achieve household-scale compute sustainability OR truth-discovery integrity without making consolidation events first-class. Silent merges miss every economic, governance, and relational consequence.

**How to apply:**
- Every consolidation primitive in the substrate (merge, dedup, equivalence-attestation, supersede) MUST emit structured events to shefa, qahal, mishpat, and imagodei, not just rewrite the graph.
- Restitution and reconciliation paths are required, not optional features. Discovering bad-propagation creates obligations; the protocol must offer them as first-class affordances.
- Reach accounting is bidirectional: earned at authoring AND adjusted at consolidation. The full history is visible to qahal/mishpat for audit.
- Quarantine is a primitive distinct from forget — known-bad is contained and tracked; forgotten content is genuinely released. Confusing them creates risk.
- This deserves its own downstream spec under elohim-protocol research, paired with the memory-lifecycle spec.

**Sources:** brainstorm 2026-05-10, emergent during merge-primitive design; load-bears the memory-lifecycle spec and extends multiple existing project principles into a single unified frame.
