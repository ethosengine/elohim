---
id: project-dwelling-hub-replication-pattern
name: dwelling-hub-replication-pattern
description: First concrete instance of the REA compute-commitment primitive — mutual storage replication between dwelling-hubs (households). Three load-bearing properties: donut economics (device-level), bilateral-by-reference mutuality (with grace-period soft-warn), intent-first observed-state-second. Hub-aware substrate vocabulary; encryption-decoupled commitments. Pattern extends to collective + commons tiers.
metadata:
  type: project
cites:
  - genesis/docs/superpowers/plans/2026-05-28-mutual-storage-replication-dwelling-hub-plan.md
  - genesis/docs/research/2026-05-28-sprint3-storage-replication-implementation-notes.md
---

The Sprint 3 shape (landed 2026-05-28, plan `genesis/docs/superpowers/plans/2026-05-28-mutual-storage-replication-dwelling-hub-plan.md`, close-out `genesis/docs/research/2026-05-28-sprint3-storage-replication-implementation-notes.md`):

**`replicates-dwelling` action on `Mishpat::Commitment`** — payload schema fields: `provider_dwelling_hub_id`, `recipient_dwelling_hub_id`, `provider_role: steward_mutual | collective_steward`, `via_collective_hub_id?`, `capacity_bytes`, `scope_filter`, `valid_from/until`, `grace_period_days`, `rotation_ttl_days`, `ratio_attestation`. **No new DHT entry type.**

**Donut economics, device-level:** DNA-locked floor + ceiling constants (`content_store_integrity`) form the donut walls; elohim manifest declares specific ratios within them (`constitutional_ratio_registry` clamps manifest→walls, collective is the residual). `replicates_dwelling_validator` enforces at every commitment author. Sprint 3 design choice: ceilings enforced via active pledges; floors via declared `ratio_attestation` (floor-via-declaration). The follow-up sprint that lands `replicates-commons` MUST close this gap (declaration → backing-pledge requirement).

**Mutuality bilateral-by-reference, not by-signature:** A authors commitment naming B; B independently authors a counter. `mutuality_audit_service` runs a sweep; if the counter is missing past `grace_period_days`, it emits a `reciprocity-imbalance` FeedbackSignal naming the breaching party. Standing-debit via existing `signal_weight_registry` (weight 8, decay 60d). Breach never contaminates attribution (see [[project_compute_commitments_bounded]]).

**Hub-aware vocabulary:** substrate uses `dwelling-hub` (matches shipped `HubKind::Dwelling`); narrative uses "household"/"family"/"grandma"/"church" (a2o features keep substrate jargon out of human copy). `provider_role=steward_mutual` is bilateral peer-to-peer; `provider_role=collective_steward` is asymmetric (a collective backs member households) — schema-reserved + explicitly rejected by the storage validator in Sprint 3, end-to-end in the `replicates-collective` follow-up.

**Intent-first, observed-state-second:** commitments authored first (notarized intent); existing inventory_gossip + libp2p pull catch up; `replication_prioritizer::score_advertised_blob` scores incoming inventory advertisements against active commitments (High/Skip in Sprint 3; Medium reserved for commons tier) to decide what the local peer fetches. No new wire protocol.

**Dual-accounting (storage premium):** `PeerCapacityView.pledges.totalPledgedBytes` vs `actuallyHeld.uniqueShardBytes` shows the dedup multiplier (~2.3x for typical multi-reach content). Makes hyperscale-without-capture honest. `HubCapacityView` mirrors `HubComputeAggregateView` shape (Hub is a role, not a notarized entity — see [[project_hub_archetype_abstraction]]); substrate stays kind-agnostic, hub classification at the projection layer.

**When to apply this pattern:**
- Every per-instance bounds-validator (collective tier, commons tier, doorway projection compute, distributed workloads) MUST delegate substrate-wide concerns to `bounds_validator::validate` and only add (a) schema validation of the action-specific payload, (b) action-discriminator check, (c) projection to `EventForValidation`, (d) any instance-specific enforcement (e.g. the donut check in `replicates_dwelling_validator`). See [[project_bounds_validator_pattern]].
- Every per-scale audit aggregator (collective_membership_audit_service, commons_contribution_audit_service) MUST mirror `mutuality_audit_service`: walk active commitments, classify status, emit a FeedbackSignal on breach, persist an operational log row. The service shape is mock-testable; production conductor-bridge wiring (counter lookup + signal emission) is a documented follow-up.

**Execution lesson (subagent-driven):** the plan's stale file paths (`distribution.rs`/`household_resilience.rs` → actually all in `infrastructure.rs`) and the `CommitmentRecord` shape gap (no `payload_json`/`signed_at`; read from `bounds`/`valid_from`) were caught by the orchestrator reading real signatures before each dispatch. Type-clobber hazards (two `Tier`/`ProviderRole`/`HubKind` exporting the same `.ts`) were resolved by reuse/rename, not redeclaration.

**Related:**
- [[project_bounds_validator_pattern]] — the substrate primitive this proves a first instance of
- [[project_rea_compute_commitment_primitive]] — gospel-tier shape
- [[project_compute_commitment_first_instance_pivot]] — why deploy was abandoned; storage replication chosen
- [[project_hub_archetype_abstraction]] — Hub role; dwelling/collective/computed; stewards-not-members
- [[project_signal_kind_extensible_protocol_class]] — reciprocity-imbalance uses this pattern
- [[project_compute_commitments_bounded]] — breach never contaminates attribution
- [[feedback_a2o_narrative_is_opus_work]] — a2o features authored by Opus
- [[feedback_diesel_migration_timestamp_collision]] — migration slot pattern (used HHMMSS 100000)
