---
name: bounds-validator-pattern
description: Single substrate-side bounds_validator::validate function that every per-instance per-row-of-the-table validator delegates to. Walks bounded_by → Commitment → 7 checks (commitment_found, not_revoked, active window, scope_includes_event, reach_ceiling_ok, rate_within_limit, key_rotation_current). CommitmentFetcher + RateHistory traits enable mocking without conductor.
metadata:
  type: project
cites:
  - genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md
  - elohim/elohim-storage/src/services/bounds_validator.rs
  - elohim/sdk/domains/elohim/manifest.json
---

When implementing a per-instance validator (Sprint 1's republish_epr_validator, Sprint 3's serve_url_projection_validator, Sprints 5a-e's per-row validators), DELEGATE to `services::bounds_validator::validate` for the substrate-wide concerns. Per-instance validators only handle (1) schema validation of the action's specific payload, (2) action-discriminator check, and (3) construction of an `EventForValidation` projection. The substrate-wide concerns — Commitment fetch, active-window, scope-includes-event, reach-ceiling, rate-limit, key-rotation, revoked — all live in one function.

**Why:** revocation propagation and rate-limit discipline must be uniform across all 7 rows of the gospel-tier generalization table. One implementation; one place to fix bugs; one place to audit. Per `[[project_rea_compute_commitment_primitive]]` §4 auditability properties.

**How to apply:**
1. Build your per-instance validator at `services/<instance>_validator.rs`.
2. Schema-validate the event payload against `elohim/sdk/schemas/v1/economic-events/<instance>.schema.json`.
3. Convert your view to `EventForValidation { action, performer, bounded_by, target_epr_id, reach, signed_at }`.
4. Call `bounds_validator::validate(&event, fetcher, rate_history).await`.
5. On `BoundsViolation`, emit the appropriate FeedbackSignal — `rate-limit-exceeded` for that kind, `bad-custody` for revoked/expired, `reach-escalation-pending` for ReachCeilingExceeded, etc. Weights live in `elohim/sdk/domains/elohim/manifest.json` signalKinds entries; the standing pipeline calls `project_extension_signal` to apply them.

**Reach hierarchy (per `reach_rank` in bounds_validator.rs):** private=0, self=1, intimate=2, trusted=3, familiar=4, community=5, public=6, commons=7. **`commons` is the MOST permissive** — counter-intuitive but correct (commons = no restriction, available to all). A reach_ceiling of `commons` allows all lower reaches; `reach=public` against `ceiling=commons` PASSES.

**Reference:** `elohim/elohim-storage/src/services/bounds_validator.rs`. First instance: Sprint 1's `republish_epr_validator`. Plan: `genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md`.

**Related:** `[[project_rea_compute_commitment_primitive]]` (gospel-tier shape), `[[project_signal_kind_extensible_protocol_class]]` (signal_weight_registry uses this extension pattern), `[[project_canonical_wire_shape_newtype_pattern]]` (CommitmentCid and AgentCid are newtype-hardening candidates in a follow-up).
