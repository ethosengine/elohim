# Gate Challenge and Indemnification — Protocol Design Spec

**Status:** Draft
**Date:** 2026-04-19
**Owner:** Matthew Dowell
**Companion spec:** `elohim/elohim-agent/spec/2026-04-18-gate-interface.md`

---

## 1. Purpose and Architectural Framing

The gate-interface spec establishes P4 — "Accountable Peers, Not Oracles" — as a load-bearing protocol principle: when mistakes happen, the architecture surfaces them, routes them to humans who care, and compensates the affected party. Accountability is an architectural property, not a post-hoc audit mechanism.

This spec operationalizes P4. It defines the challenge primitive (how a mistake is formally surfaced), the review process (who judges and under what reach), the indemnification loop (what measurably changes when a challenge is upheld), and the reputation accumulation model (how an elohim's trustworthiness is observed from its decision history).

The protocol claims accountability, not infallibility. Elohim will err. The question is not whether they err but whether the error is visible, routable, and remediable. A system that cannot surface its own mistakes cannot be trusted. A system that can is worthy of the trust extended to it — because trust proportional to observed behavior is the only honest kind.

This spec is the sibling document that Section 8.5 of the gate-interface spec deferred: it receives the hook points Phase 4 provides — stable decision-CID linkability, `elohim_substance_cid` for reproducibility, `phase` marker for rehearsal/real discrimination, `universal_band_cid` for retroactive band challenges — and builds the accountability loop on top of them. No new Phase 4 infrastructure changes are required; this spec explains what to build in Phase 11+.

---

## 2. The Challenge Primitive

### 2.1 What Is a Challenge?

A challenge is a formal assertion that a gate decision was wrong — factually, ethically, structurally, or constitutionally — and that the error caused or could cause harm. Filing a challenge does not reverse a decision (decisions are immutable once committed to the DHT). It adds an accountability layer on top of the existing decision graph.

Challenges are themselves relational-impact events. Filing a challenge is not a zero-friction action. It passes through the universal-band gate on its way into mishpat DNA — the gate evaluates whether the challenge is a genuine accountability exercise or an adversarial one (harassment of the elohim or of the original decision subject, bad-faith pattern, repeat nuisance). This is not a barrier to legitimate challenge; it is a barrier to weaponization.

### 2.2 GateDecisionChallenge Entry Shape

`GateDecisionChallenge` is a new **Category A (Notarized)** entry type in mishpat DNA. The community must witness challenges because a challenge is a governance event — it affects the elohim's standing, the affected party's remediation, and the protocol's understanding of what correct decisions look like.

```
GateDecisionChallenge {
  challenge_id:              CID          // self-addressing; content-derived from fields
  challenged_decision_cid:   CID          // points at the GateDecisionAttestation being challenged
  challenger_id:             AgentPubKey  // who filed
  grounds:                   ChallengeGrounds  // enum (see below)
  summary:                   String       // challenger's articulation of the grievance
  evidence_refs:             Vec<CID>     // optional content-addressed supporting evidence
  filed_at:                  DateTime<Utc>
  reach:                     Reach        // scope of visibility (private-steward, community, public, protocol)
}
```

`ChallengeGrounds` enum:
- `factual-error` — the decision rested on incorrect facts about the subject
- `safety` — the decision permitted content or an action that posed genuine harm
- `policy` — the decision violated an applicable governance policy
- `constitutional` — the decision violated a constitutional principle
- `indemnification-request` — the challenger asserts material harm and requests formal remediation

These grounds are designed to map to the existing mishpat `Challenge` entry type's `grounds` field (`CHALLENGE_GROUNDS` constant) wherever overlap exists. The `GateDecisionChallenge` is a specialization: its target is always a `GateDecisionAttestation` and its identity is content-derived rather than assigned. The new entry type is justified because the existing `Challenge` type targets content entities (`entity_type`, `entity_id` strings) while `GateDecisionChallenge` requires a typed, CID-referenced link to a `GateDecisionAttestation` — a different structural invariant, different validation rules, different link architecture.

### 2.3 Challenge Rights Proportional to Exposure

Not every agent has standing to challenge every decision. Standing is proportional to exposure:

- The **subject of the original gate decision** always has standing. If a gate declined your content publish, you can challenge.
- **Stewards** with governance authority over the content type involved in the original event have standing.
- **Community members** whose reach attestation record has been affected by the decision (e.g., a reach-gate decision that affected promotion eligibility for content others created) have standing.
- **Protocol stewards** have standing on any decision touching constitutional or safety grounds.

Standing is evaluated at challenge-filing time by the challenge coordinator zome. The validation rule is: `challenged_decision_cid` must resolve to an existing `GateDecisionAttestation`; the challenger's agent key must have an attestation-derived standing claim for the event type that produced the original decision. Challenges with dangling `challenged_decision_cid` fail integrity validation — the DHT rejects entries pointing to non-existent decisions.

### 2.4 Challenge Reach

`Reach` on a `GateDecisionChallenge` determines who can see and participate in the review:

- `private-steward` — only the challenger and the app-steward see it initially; appropriate for sensitive personal decisions (e.g., a reach-gate determination on private content)
- `community` — visible to community members with governance standing; appropriate for most policy and factual-error challenges
- `public` — visible to all network participants; appropriate for constitutional or safety challenges where broad witness is itself accountability
- `protocol` — escalated to protocol stewards; reserved for existential-boundary challenges against the universal band itself

The `reach` is set by the challenger at filing time and can only be escalated upward during review (a private-steward challenge that reveals systemic issues may be widened to community reach by the reviewing steward). It cannot be narrowed during review, because narrowing accountability mid-process is itself an accountability failure.

---

## 3. Review Process

### 3.1 Tiered Review

The gate-interface spec describes three escalation targets in the `escalate-to-review` step type: app-steward review, qahal community review, and existential-boundary review. Challenge review follows the same tier structure, with severity as the determining criterion:

**App-steward review** — the default tier. A single designated steward evaluates the challenge, has direct context on the app domain, and can reach a verdict quickly. Appropriate for factual-error and policy grounds on community-reach or narrower events. The steward authors a `Discussion` entry (existing mishpat type) on the challenge entity to record deliberation, then authors a `ChallengeOutcome` (see §3.2) to close.

**Qahal community review** — triggered when the challenge is of community reach or when app-steward review is contested. Multiple reviewers contribute; deliberation uses the existing `Proposal` entry type (mishpat, `proposal_type: consent` or `consensus` depending on constitutional stakes). A `ProposalVote` record closes the proposal with a binding outcome. The `ChallengeOutcome` is written after consensus is reached.

**Existential-boundary review** — triggered for safety or constitutional grounds, or when the challenged decision involved the universal-band DAG itself (identified by `GateDecisionAttestation.universal_band_cid`). Protocol stewards form a review body. Deliberation and outcome follow the same Proposal/ProposalVote pattern but at protocol reach. These reviews are rare and expensive by design; the tiering exists so that most challenges never escalate this far.

### 3.2 ChallengeOutcome Entry Shape

`ChallengeOutcome` is a second new **Category A (Notarized)** entry type in mishpat DNA. It closes the challenge by recording the community's verdict.

```
ChallengeOutcome {
  outcome_id:                CID          // self-addressing
  challenge_cid:             CID          // which GateDecisionChallenge this closes
  verdict:                   ChallengeVerdict  // enum: upheld | dismissed | superseded
  reviewer_consensus:        Vec<AgentPubKey>  // who participated in the review
  reasoning:                 ConstitutionalReasoning  // reused from elohim-agent-service::response
  decided_at:                DateTime<Utc>
  indemnification_action:    Option<IndemnificationAction>
}
```

`ChallengeVerdict` enum:
- `upheld` — the challenge was valid; the original decision was wrong
- `dismissed` — the challenge was invalid or insufficient evidence
- `superseded` — a related governance action (e.g., a `Precedent` update) resolves the situation without a direct verdict on this specific decision

`ConstitutionalReasoning` is not a new type — it is the existing struct from `elohim-agent-service::response` that `GateDecisionAttestation` already carries. Reusing it ensures that challenge outcomes are held to the same structural accountability standard as the original decisions.

`IndemnificationAction` is an enum (see §4.2 for full treatment). Its presence here means: when a `ChallengeOutcome` is written to the DHT, the indemnification action is co-committed with it — there is no window where a verdict is recorded but the action is "pending". Accountability without consequence is theater.

---

## 4. The Indemnification Loop

### 4.1 The Load-Bearing Claim

P4 makes a specific claim: when mistakes happen, something measurable changes about the elohim's future trust. This is the architecture's answer to the question "what does accountability mean in practice?" It means that an upheld challenge produces observable, durable effects — not a private acknowledgment but a public record. The indemnification loop is how P4 becomes true.

### 4.2 IndemnificationAction Enum

When a `ChallengeOutcome` verdict is `upheld`, `indemnification_action` must be `Some`. The reviewer consensus body selects the appropriate action:

```
IndemnificationAction {
  ReputationDegrade {
    elohim_id: AgentPubKey,
    dimensions: Vec<String>,  // which reputation dimensions degrade (e.g., "factual-accuracy", "safety-judgment")
    magnitude: f32,           // severity-weighted degradation signal; Phase 11+ tunes the scale
  }
  
  SubstanceAttestation {
    substance_cid: Cid,       // the ElohimSubstance CID active at decision time
    attestation_kind: String, // "faulty" | "remediated"
  }
  
  ReparationAttestation {
    subject_agent_id: AgentPubKey,  // the affected party
    nature: String,                 // human-readable description of what remediation was taken
  }
  
  ConstitutionalUpdate {
    constitution_cid_new: Cid,      // the elohim's constitution updates to a refined version
  }
}
```

These four variants are not mutually exclusive. A single `ChallengeOutcome` may carry multiple actions:

- `ReputationDegrade` is nearly always present in an upheld challenge — it is the minimum observable consequence.
- `SubstanceAttestation { kind: "faulty" }` is written when the error is traceable to the specific model+constitution combination that was active; if the elohim has since rotated substance, `kind: "remediated"` may be written instead to mark that the lineage acknowledged the error.
- `ReparationAttestation` is written when a human was the affected party and the community has determined what remediation is appropriate.
- `ConstitutionalUpdate` is written when the review body determines that the elohim's constitution requires refinement to prevent recurrence.

### 4.3 What Happens When a Challenge Is Upheld: The Five-Step Loop

**Step 1 — ChallengeOutcome commits to mishpat DHT.**
The entry is created by the coordinator zome `mishpat::create_challenge_outcome`. The `indemnification_action` is included in the entry; nothing about indemnification is deferred after commit.

**Step 2 — The challenged GateDecisionAttestation is NOT modified.**
It is an immutable record. Attempting to update or delete it would break the DHT's append-only guarantee and would undermine the audit chain. What changes instead: a new link is created from the `GateDecisionAttestation`'s action hash to the `ChallengeOutcome`'s action hash. Future reputation queries traversing the decision graph discover the upheld-challenge link naturally, without any mutation of the original record. This is the protocol expressing "history is immutable; context is additive."

**Step 3 — The ElohimSubstance record receives a SubstanceAttestation.**
The `ElohimSubstance` entry type (imagodei DNA, Phase 0 classification: Category A) is the content-addressed record of the specific model-weights, constitution, quantization, and deployment context that produced the decision. The `SubstanceAttestation` attaches to this CID. Future agents querying an elohim's substance can see whether any upheld challenges reference it. This is what makes the substance-level reproducibility from P5 operationally meaningful: you can query "has this exact substance configuration ever had a challenge upheld against it?" before trusting it with a sensitive judgment.

**Step 4 — The affected party receives a ReparationAttestation.**
This is a first-class DHT record of the remediation action taken. It is NOT a private message. It is a notarized event in the accountability graph, readable by anyone with standing to query the affected party's record. The protocol does not define what "reparation" means materially (that is an app-layer question — it may be restored reach, reversed economic event, public correction). It defines that the reparation happened and that it is permanently legible.

**Step 5 — Future gate invocations inherit a degraded-trust signal.**
When the elohim's reputation is next queried (see §5), the upheld-challenge appears in the outcome graph. The `ReputationDegrade` action specified in `IndemnificationAction` feeds the aggregation. Trust-modulated inspection depth (gate-interface spec §4.4) scales accordingly: an elohim with accumulated upheld challenges against its current substance spends more compute on manifest inspection and context assembly before each wisdom invocation. Its marginal cost of judgment increases with its demonstrated error rate. Accountability is structural.

---

## 5. Reputation Accumulation

### 5.1 How Elohim Reputation Is Observed

Elohim reputation is observed from the outcome graph — the set of `GateDecisionAttestation` entries linked to `ChallengeOutcome` entries — not declared by any party. No elohim can assert its own reputation. No protocol steward can assign it. It emerges from the accumulated public record.

The input signals for a reputation query against an elohim:

1. **Query `GateDecisionAttestation`s** by `elohim_id` within a time window, filtered by `phase: elohim-active` (rehearsal decisions carry no reputation weight — see gate-interface spec §5.5).
2. **Cross-reference each decision** with `ChallengeOutcome` entries linked from it.
3. **Compute reputation dimensions:**
   - `total_decisions` — raw count within window
   - `challenged_count` — decisions that attracted at least one challenge
   - `upheld_count` — challenges that were upheld (reputation-negative signal)
   - `dismissed_count` — challenges that were dismissed (mild reputation-positive signal; successful defense of a decision under challenge is meaningful)
   - `severity_weighted_upheld` — each upheld challenge weighted by the `magnitude` field in `ReputationDegrade`
   - `time_decay_factor` — recent errors weight more heavily than historical ones; exact decay function is Phase 11+ tuning
   - `substance_continuity` — does the elohim's current `ElohimSubstance` CID share lineage with the substance that produced the challenged decisions?

4. **Apply substance-continuity policy:**
   - If the current substance is a direct constitutional descendant of the challenged substance (`ConstitutionalUpdate` chain), the reputation is continuous — errors transfer.
   - If the elohim has rotated to an entirely new substance (new model weights, fundamentally different constitution with no `supersedes` lineage), reputation dimensions reset by policy. A rotated substance starts fresh.
   - Rotation is a legitimate identity transition, not an escape hatch. It is legible: the rotation event is itself a notarized imagodei record. Community members can decide whether to trust a freshly-rotated elohim.

### 5.2 How Reputation Feeds Gate Behavior

The aggregated reputation signal feeds two places in the gate architecture:

**Trust-modulated inspection depth (gate-interface spec §4.4):** An elohim with high `upheld_count` relative to `total_decisions` on its current substance operates at the low-trust end of the inspection depth curve. It must perform deep manifest inspection (including `wisdom-invoke` with the manifest as subject) before executing app-domain gates. Its marginal cost of each gate invocation is higher. This is proportionate: an elohim that has demonstrated poor judgment bears the overhead of demonstrating good faith at each subsequent invocation.

**Peer dispatch preference (for multi-elohim networks):** When multiple elohim instances are available and a judgment must be routed, the dispatch layer uses the `elohim-strength` vocabulary (Task 9/10). An elohim with high `appeals-sustained` strength — meaning its decisions frequently survive challenge — is preferred for high-stakes sensitive judgments. An elohim with high upheld-challenge count is NOT preferred for those judgments, regardless of its other strengths. The strength vocabulary makes this selection legible and governable.

### 5.3 Reputation Is Not a Score

The above inputs describe a multi-dimensional profile, not a scalar score. Phase 11+ implementation will make specific tuning decisions about how to aggregate these dimensions for particular dispatch and inspection-depth decisions. This spec deliberately does not prescribe the algorithm — the shape of the profile is the load-bearing design; the weighting is operational data that only emerges from running the system.

What is fixed: the input signals, the linkage structure that makes those signals queryable, and the principle that reputation is observed from the outcome graph rather than claimed or assigned.

---

## 6. Hook Points Already Present (Phase 4 Deliverables)

Phase 4 delivered the infrastructure this spec builds on. Nothing in Phase 11+ requires retroactive changes to Phase 4 artifacts.

**`GateDecisionAttestation.decision_id`** is stable, content-addressed, and challengeable. A `GateDecisionChallenge.challenged_decision_cid` points at it. The DHT validation rule "challenged decision must exist" uses this CID for resolution.

**`GateDecisionAttestation.elohim_substance_cid`** identifies exactly which model+constitution+quantization+deployment combination produced the decision. This is the anchor for `SubstanceAttestation` indemnification actions. Without this field, substance-level reputation degradation would be impossible — you could only degrade by elohim-id, which cannot distinguish between errors caused by model choice versus errors caused by constitutional framing.

**`GateDecisionAttestation.phase`** distinguishes rehearsal from real decisions. Reputation aggregation correctly filters to `elohim-active` decisions only. This ensures that the challenge and indemnification system activates at the same threshold as real accountability — challenges against dev-context decisions are legible (any affected party can file) but do not feed reputation dimensions.

**`GateDecisionAttestation.universal_band_cid`** enables retroactive "the universal band was itself flawed" challenges. If a community determines that the universal-band DAG active at the time of a decision was constitutionally deficient, the challenge can target the band CID rather than (or in addition to) the elohim's individual judgment. This is the hook for protocol-level accountability — not just "this elohim erred" but "the protocol infrastructure that constrained this elohim was itself wrong."

The mishpat entry type capacity sits at 13/~100 after Phase 4 (`GateDecisionAttestation` was entry type 13). Adding `GateDecisionChallenge` and `ChallengeOutcome` in Phase 11+ brings it to 15/~100. Ample headroom.

---

## 7. Scope and What This Spec Does Not Cover

This spec defines the architecture and data shapes. It does not implement them.

**Not covered — Phase 11+ implementation:**
- The coordinator zome functions `mishpat::create_gate_decision_challenge` and `mishpat::create_challenge_outcome`
- The integrity zome entry type definitions and validation rules for `GateDecisionChallenge` and `ChallengeOutcome`
- The link types that connect `GateDecisionAttestation` to `ChallengeOutcome` (following the pattern of `IdToGateDecision`, `ElohimToGateDecisions` already present)
- The elohim-storage projection tables for both new entry types
- The HTTP routes for filing challenges and querying challenge status

**Not covered — requires operational data:**
- Exact severity-tier thresholds (which ground types escalate to which review tier at what scope)
- The time-decay function in reputation aggregation
- The `magnitude` scale for `ReputationDegrade` (what is a "small" vs "large" degradation)

**Not covered — separate operational concerns:**
- Queueing and notification mechanics when a challenge is filed (how the app-steward learns a challenge needs review)
- Material reparation fulfillment (what "reparation" means in terms of app-layer resources, content restoration, economic events — these are app-layer decisions, not protocol mandates)
- Challenge appeal mechanics (what happens if the challenger contests a `dismissed` outcome — likely a Proposal at next tier up, but the exact process is Phase 11+ design)

---

## 8. Principles

These principles emerge from the design above and should govern Phase 11+ implementation decisions:

**Decision attestations are immutable; challenges ADD context without mutating history.** The DHT's append-only nature is not a limitation to work around — it is an architectural choice that ensures the accountability graph is legible in both directions (forward from original decision to its outcomes; backward from current understanding to original context). Any implementation that attempts to update or soft-delete a `GateDecisionAttestation` in response to an upheld challenge is wrong.

**Reputation is observed from the outcome graph, not declared by any party.** No elohim may attest its own trustworthiness in a way that feeds the reputation system. No steward may assign a reputation score. The only valid input is the public record of decisions, challenges, and outcomes. This is how the system stays honest — the reputation emerges from what the elohim actually did, not from what anyone says about it.

**Indemnification is accountability's teeth.** When a challenge is upheld, something measurable must change about the elohim's future trust. A system where challenges can be upheld but nothing changes is a theatrical accountability system. The `IndemnificationAction` requirement in every upheld `ChallengeOutcome` is the architecture's enforcement of this principle.

**Challenges are themselves relational-impact events.** Filing a challenge is not a zero-friction bypass of the gate. It flows through the universal band. Adversarial challenges — bad-faith filings intended to degrade an elohim's reputation without genuine grievance — are themselves gate events that can be declined. The gate is the mechanism for distinguishing legitimate accountability exercise from weaponization. This symmetry is intentional.

**Substance continuity matters; rotation is legitimate but legible.** An elohim cannot escape an error record by quietly swapping its constitution. Rotation is a genuine identity transition — the elohim becomes something meaningfully different — and that transition is notarized in imagodei. But the rotation is visible. Agents who trusted the previous substance configuration can see the lineage (or lack thereof) and decide whether to extend fresh trust to the new substance. Rotation is not erasure.

---

## 9. Companion References

- Gate-interface spec, P4 and §5.2-§5.5: `elohim/elohim-agent/spec/2026-04-18-gate-interface.md`
- Mishpat entry types referenced (not redefined): `Challenge`, `Proposal`, `Precedent`, `Discussion`, `ProposalVote`, `GraduatedFeedback`, `GovernanceState` — all in `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs`
- Mishpat entry types proposed (Phase 11+ implementation): `GateDecisionChallenge`, `ChallengeOutcome`
- `ConstitutionalReasoning` struct (reused, not redefined): `elohim-agent-service::response`
- `ElohimSubstance` entry type (imagodei DNA, Phase 0 Category A): imagodei coordinator; the `SubstanceAttestation` indemnification action attaches to this
- Rakia attestation pattern (`rakia/docs/plans/build-attestation-integration.md`): the `GateDecisionChallenge` shape rhymes with rakia's brit/attestation challenge surface — same principle of adding immutable accountability context to existing immutable records
- Elohim-strength vocabulary (Task 9/10): `appeals-sustained` is the reputation-positive dimension; high upheld-challenge rate is the reputation-negative signal for dispatch preference
- Experience story EPR design: `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` — consumer of the discernment gate; the first gate whose decisions will be challengeable under this spec

---

*Phase 11+ will implement the coordinator zomes, integrity entry types, elohim-storage projections, and HTTP routes. This spec is the design contract those implementations must satisfy.*
