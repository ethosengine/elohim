# The REA Compute-Commitment Primitive

> **Canon status:** Gospel-tier substrate primitive. Read [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) first.

---

## §1 — The Shape

A reciprocal REA compute commitment is a **`Mishpat::Commitment` DHT entry** (existing entry type — Mishpat at ~11/~100 headroom; no new entry type needed) with a new action discriminator `delegates-compute`, between a **provider agent** and a **recipient agent**, scoped to a class of **`EconomicEvent`s** the recipient is authorized to emit, bounded by enforceable conditions, with reciprocal obligations on both sides.

The primitive is one shape. It recurs everywhere in the protocol where one agent commits compute, work, or standing to another agent under bounded conditions. Master the primitive once; the rest of the protocol inherits.

---

## §2 — Diagram

```
                                      Commitment
                                      ┌─────────────────────────────┐
                                      │ action: "delegates-compute" │
                                      │   scope: "<event-class>"    │
                                      │                             │
   ┌──────────────┐                   │ provider:  agent X          │   ┌──────────────────────┐
   │  Provider X  │ ────signed by──▶  │ recipient: agent Y          │ ◀─│    Recipient Y       │
   └──────────────┘                   │                             │   └──────────────────────┘
        ▲                             │ bounds: { … }               │            │
        │                             │ reciprocity: { … }          │            │
        │                             │ ttl: <rotatable>            │            │
        │                             └─────────────────────────────┘            │
        │                                                                        │
        │                                                                        ▼
        │                                                       EconomicEvent
        │                                                       ┌───────────────────────────┐
        │                                                       │ action: <within scope>    │
        │                                                       │                           │
        │                                                       │ performer: Y              │
        │                                                       │ target:    <outcome CID>  │
        │                                                       │                           │
        └──────────  back-ref  ── bounded_by: <Commitment CID> ◀│ (proves standing)         │
                                                                └───────────────────────────┘
```

Every event the recipient emits carries `bounded_by: <Commitment CID>` — a back-reference. The substrate validates an event by walking the back-reference to the Commitment, checking the Commitment is active and within bounds, and accepting/rejecting.

---

## §3 — Reciprocity Model

Reciprocity is real, not decorative. Every compute commitment names two-way obligation. If either side defaults, a `FeedbackSignal` accrues on-chain — the protocol witnesses the breach.

This is what distinguishes a reciprocal commitment from a unilateral grant (such as an X-API-Key, which has no return obligation and no on-chain accountability).

| Provider obligation | Recipient obligation |
| --- | --- |
| Hold custody of the bounding Commitment | Sign every in-scope `EconomicEvent` with own key |
| Acknowledge scope/reach escalations (soft-warn ceremony) | Reference `bounded_by: <Commitment CID>` in every event |
| Rotate the recipient's key on schedule | Stay within bounds — substrate enforces, agent helps |
| Revoke promptly if compromise or misbehavior detected | Refuse to sign outside bounds; emit feedback when must defer |

A provider who refuses to acknowledge escalations or rotate keys produces accumulating `non-reciprocation` signals — the protocol witnesses the failure as much as it would witness a recipient's bounds violation.

---

## §4 — Auditability Properties

This shape gives the substrate four properties that a unilateral X-API-Key bypass cannot:

### 1. Standing is checkable

Given any `EconomicEvent`, walk back through `bounded_by` to the Commitment. Verify it is still active, signed by a provider with standing in the relevant scope, and that the event falls within bounds. Yes/no answer from the DHT.

### 2. Revocation is real

If the provider revokes the Commitment, subsequent events from the recipient referencing it fail validation. No "rotate the API key everywhere" scramble; no synchronization race; the substrate refuses out-of-bounds events the moment the Commitment changes state.

### 3. The authority chain is itself notarized

Every link from "operator owns this resource" to "this CI agent may republish this EPR" is a chain of DHT-witnessable Commitments. No off-chain trust. A skeptical reviewer can walk the chain and confirm each step is real.

### 4. Reciprocity is observable

The provider's acknowledgements (e.g., for soft-warn ceremonies) are themselves DHT-resident Commitments. Default = silence; default emerges as a `non-reciprocation` FeedbackSignal pattern. The protocol can witness chronic non-reciprocation and surface it as a stewardship concern.

---

## §5 — Generalization Table

The same `delegates-compute` shape, with different scopes and bounds, models nearly every act of bounded authority delegation in the protocol. Future spec authors should copy this pattern rather than re-derive it.

| Instance | Provider | Recipient | Event class | Bounds (examples) |
| --- | --- | --- | --- | --- |
| **Deploy (Z.D)** | operator steward | deploy-svc-agent | `republish-epr` | reach ceiling, rate/hr, EPR scope, key rotation TTL |
| **Hosting projection** | doorway operator | doorway-svc-agent | `serve-url-projection` | doorway capacity, reach gates, URL-prefix scope |
| **Household chore stewardship** | household member | another member | `chore-done` | scope (kitchen, yard), period (week), chore type |
| **Qahal moderation** | qahal collective | moderator-agent | `moderation-action` | qahal scope, action types, target reach class |
| **Content authorship delegation** | original author | co-steward | `publish-revision` | content CID lineage, branch policy, scope |
| **Compute lending (DePIN)** | node operator | requesting peer | `provide-cycles` | watts, wall-time, task class |
| **Recovery delegation (graduated)** | steward (pre-incident) | recovery quorum | `attest-recovery` | reach ceiling, quorum threshold, time window |
| **Guardianship (ward → agent)** | guardian-steward | ward's agent | `act-on-behalf` | scope (school, health, social), age-bounded, capacity-conditional |
| **End-of-life succession** | original steward | executor | `transition-stewardship` | testamentary, time-locked, witness-required |

There are more rows the protocol has yet to encounter. They all inherit the same shape.

---

## §6 — Anti-Patterns This Displaces

The primitive's existence is a refusal of several common patterns. Spec authors must not reinvent these in new clothing:

| Anti-pattern | Why it fails | Replacement |
| --- | --- | --- |
| **X-API-Key admin grant** | No on-chain standing; no revocation propagation; no audit trail. | `delegates-compute` Commitment with bounds; substrate validates. |
| **"Just rotate the key" recovery** | No audit trail; everyone scrambles; race between revocation and acceptance. | Steward revokes the Commitment via Mishpat; substrate refuses subsequent events. |
| **Per-feature ad-hoc auth** | Re-derived every time; inconsistent surface; impossible to reason about authority as a whole. | One primitive, instantiated per scope; consistent across the protocol. |
| **Unilateral grants without reciprocity** | No observable default; no FeedbackSignal accrual; provider can ignore obligations without consequence. | Reciprocity table in every Commitment; non-reciprocation produces signals. |
| **Anonymous publish** | Untraceable; no standing check possible; first link of the chain is missing. | Every action has a `bounded_by` reference, even for first-publish (bootstrap Commitment with `epr_scope: ["*"]`). |
| **"Trust the operator"** | Operator becomes single point of failure and capture vector. | Operator's authority is itself bounded by their `delegates-compute` from the community/qahal that constituted them. |

---

## §7 — Concrete Instance Index

The first concrete instance of this primitive is **Z.D** — substrate-correct deploy via deploy-service-agent operating under operator-steward's `delegates-compute` Commitment.

Spec: `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md`

Z.D's §2 walks through the primitive for one concrete scope (`republish-epr`). Reading Z.D §2 alongside this canon is the recommended path for understanding how the primitive translates into Rust code, JSON Schema, and Holochain zome functions.

Future concrete instances (per the §5 table) will get their own specs as they land. Each spec should:

1. Cite this canon doc in its §0 References section.
2. Name the provider, recipient, event class, and bounds for its instance.
3. Define the JSON Schema for its event payload under `elohim/sdk/schemas/v1/economic-events/<action>.schema.json`.
4. Define the bounds-validator extension (or reuse the generic one if bounds shape matches).
5. Add a2o scenarios mirroring Z.D's pattern.

This consistency means an operator who understands one instance understands them all.

---

## §8 — Implementation Reference

After Z.D Phase 1 lands, the implementation lives at:

| Surface | Path |
| --- | --- |
| Integrity validator for `delegates-compute` action | `elohim/holochain/dna/mishpat/zomes/integrity/commitments/src/delegates_compute.rs` |
| Coordinator function | `elohim/holochain/dna/mishpat/zomes/coordinator/commitments/src/delegates_compute.rs` |
| Bounds validator service | `elohim/elohim-storage/src/services/bounds_validator.rs` |
| JSON Schema (delegates-compute payload) | `elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json` |
| JSON Schema (republish-epr event) | `elohim/sdk/schemas/v1/economic-events/republish-epr.schema.json` |
| Storage event emission | `elohim/elohim-storage/src/services/rea_commitment_service.rs` |
| SSE event types | `elohim/elohim-storage/src/sse.rs` (`delegates-compute.registered`, `delegates-compute.revoked`) |
| Doorway subscriber | `doorway/doorway-service/src/projection/storage_events_subscriber.rs` |

### §8.1 — Bounds Validator Pattern

Every per-instance validator **delegates** substrate-wide concerns to a single `bounds_validator::validate` function. Per-instance validators only handle: (1) schema validation of the action's specific event payload, (2) action-discriminator check, and (3) construction of an `EventForValidation` projection. The substrate-wide checks — Commitment fetch, not-revoked, active window, scope-includes-event, reach-ceiling, rate-within-limit, key-rotation-current — all live in one function (7 checks). This is load-bearing for §4's auditability promise: revocation propagation and rate-limit discipline must be uniform across every row of the §5 table. One implementation; one place to fix bugs; one place to audit.

`CommitmentFetcher` and `RateHistory` are traits, enabling mocking without a live conductor — run bounds-validator tests without a full Holochain setup.

**Applying the pattern to a new instance:**

1. Build your per-instance validator at `services/<instance>_validator.rs`.
2. Schema-validate the event payload against `elohim/sdk/schemas/v1/economic-events/<instance>.schema.json`.
3. Convert your view to `EventForValidation { action, performer, bounded_by, target_epr_id, reach, signed_at }`.
4. Call `bounds_validator::validate(&event, fetcher, rate_history).await`.
5. On `BoundsViolation`, emit the appropriate `FeedbackSignal` — `rate-limit-exceeded`, `bad-custody` (revoked/expired), or `reach-escalation-pending` (ReachCeilingExceeded). Signal weights live in `elohim/sdk/domains/elohim/manifest.json`; the standing pipeline applies them via `project_extension_signal`.

**Reach hierarchy (counter-intuitive):** `private=0 < self=1 < intimate=2 < trusted=3 < familiar=4 < community=5 < public=6 < commons=7`. A ceiling of `commons` (7) is the **most permissive** — it permits everything, including `public`. Do not confuse "commons" with "restricted"; in the protocol, commons means unrestricted.

When extending the primitive to a new concrete instance, the typical work is:

1. Add a new action discriminator to the Mishpat zome's integrity validator (a few lines).
2. Add a JSON Schema for the new event payload.
3. Either reuse the generic bounds validator or extend it (most new instances reuse).
4. Add doorway/elohim-storage handlers for the new event type if needed.
5. Update the §5 generalization table here in canon to list the new instance.

Five steps, copy-pattern from Z.D. The substrate work is one-time; the application of the primitive scales linearly with no per-instance reinvention.

---

## §9 — References

### Canon (this directory)

- [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) — why this primitive exists and what it serves.
- [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) — how the primitive instantiates across human life-stage capacities.

### Specs

- `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` — Z.D, the first concrete instance.
- `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` — the EPR substrate the primitive operates on.
- `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` — recovery-quorum delegation, another concrete instance.

### Memory anchors (agent-side)

- `project_rea_compute_commitment_primitive` — agent-side condensed form.
- `project_compute_commitments_bounded` — the precursor intuition.
- `project_depin_contracts_are_policy` — the DePIN row of the §5 table.
- `project_no_sovereignty_stewardship_over_ownership` — the canon doc this canon assumes.
- `project_signal_kind_extensible_protocol_class` — the extension path for FeedbackSignal kinds without new entry types.

---

## §10 — Closing Note

This primitive is what makes the protocol's "stewardship over sovereignty" claim mechanically real. Every authority is a Commitment. Every action references its Commitment. Standing is checkable. Revocation is real. Reciprocity is observable. The chain is itself notarized.

Without this primitive, "stewardship" is rhetoric. With it, stewardship is the substrate.

Use this primitive. Do not reinvent it. Do not bypass it. Do not pretend it doesn't apply to your particular case. If you find a case where the primitive seems inadequate, that is the moment to extend the canon — not the moment to introduce a one-off.
