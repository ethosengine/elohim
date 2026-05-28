---
name: rea-compute-commitment-primitive
description: "Gospel-tier — reciprocal REA compute commitments are the substrate primitive for bounded authority delegation; one shape, instantiated everywhere (deploy, hosting, household chores, qahal moderation, content authorship, DePIN compute lending, recovery quorum); Z.D is the proving ground."
metadata: 
  node_type: memory
  type: project
  originSessionId: 0c3107ea-a896-4db5-ae15-c9e1d7921552
cites:
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
---

**Canon (in-tree, authoritative):** `genesis/docs/architecture/rea-compute-commitment-primitive.md`

The protocol's bounded-authority pattern is **one primitive** instantiated across many scopes. Master the primitive once; the rest of the protocol inherits the shape.

## Shape

A `Mishpat::Commitment` entry (existing DHT entry type; new action discriminator `delegates-compute`) between a **provider agent** and a **recipient agent**, scoped to a class of `EconomicEvent`s the recipient may emit, bounded by enforceable conditions, with reciprocal obligations on both sides.

Every event the recipient emits carries `bounded_by: <Commitment CID>` — back-reference. Substrate validates by walking the back-ref to the Commitment, checking it's active, and verifying bounds.

**Why:** This is the substrate's hedge against unilateral grants (X-API-Keys, "trust me" credentials). Standing is on-chain. Revocation is real. Reciprocity is observable.

**How to apply:** Whenever designing a feature where agent X commits compute, work, or standing to agent Y under bounded conditions — STOP. Don't invent a new auth pattern. Instantiate `delegates-compute` with appropriate scope and bounds.

## Reciprocity

Provider obligations | Recipient obligations
--- | ---
Key custody | Bounded event signing
Soft-warn acknowledgement on scope evolution | Back-reference Commitment in every event
Scheduled rotation | Stay within bounds; refuse out-of-bounds
Revocation on compromise | Emit feedback when must defer

Default on either side → `FeedbackSignal` accrues on-chain. Reciprocity is observable.

## Auditability properties

1. **Standing is checkable** — any event walks back to its Commitment for yes/no.
2. **Revocation is real** — revoke the Commitment; subsequent events fail validation. No "rotate everywhere" scramble.
3. **The authority chain is itself notarized** — operator → CI deploy agent is a chain of DHT-witnessable Commitments. No off-chain trust.
4. **Reciprocity is observable** — provider acknowledgements are DHT-resident; chronic non-reciprocation surfaces as signal pattern.

## Generalization table (the seven we've named)

Instance | Provider | Recipient | Event class | Bounds shape
--- | --- | --- | --- | ---
Deploy (Z.D) | operator steward | deploy-svc-agent | `republish-epr` | reach ceiling, rate, EPR scope, key rotation
Hosting projection | doorway operator | doorway-svc-agent | `serve-url-projection` | capacity, reach gates, URL prefix
Household chore | household member | another member | `chore-done` | scope (kitchen), period (week), chore type
Qahal moderation | qahal | moderator-agent | `moderation-action` | qahal scope, action types
Content authorship delegation | original author | co-steward | `publish-revision` | content CID lineage, branch policy
DePIN compute lending | node operator | requesting peer | `provide-cycles` | watts, wall-time, task class
Recovery delegation (graduated) | steward (pre-incident) | recovery quorum | `attest-recovery` | reach ceiling, quorum, time window

There are more rows we haven't written yet. They all inherit the same shape.

## Live spec — Z.D as proving ground

`genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` is the first concrete instance. Future spec authors should copy the pattern (§1 of that spec is gospel-tier; §2 is the deploy-specific instantiation).

## Anti-patterns this displaces

- X-API-Key admin authority (no on-chain standing; no revocation propagation)
- "Just rotate the key" recovery (no audit trail; everyone scrambles)
- Per-feature ad-hoc auth (re-derived every time; inconsistent surface)
- Unilateral grants without reciprocity (no observable default; no FeedbackSignal accrual)

## Cross-links

- [[project_compute_commitments_bounded]] — the parent intuition (compute commitments are bounded primitives)
- [[project_depin_contracts_are_policy]] — DePIN as policy-on-DHT (one row of the table)
- [[project_rea_prefix_redundant]] — REA is the pattern; resolve asymmetry by dropping the prefix, never adding it
- [[project_no_sovereignty_stewardship_over_ownership]] — stewarded compute resources, not "owned"
- [[project_signal_kind_extensible_protocol_class]] — extension path for low-trust signals, no new entry types
- [[project_socially_derived_security]] — recovery shape the deploy-agent rotation pattern mirrors
- [[project_graduated_recovery_authority]] — graduated trust circles instantiate the same primitive
- [[feedback_understand_orchestrator_substrate_before_changes]] — substrate-first design discipline this primitive depends on
