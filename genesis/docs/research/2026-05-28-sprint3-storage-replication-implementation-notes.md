# Sprint 3 — Storage Replication (Dwelling-Hub Tier) Implementation Notes

**Status:** Landed 2026-05-28 (subagent-driven execution)
**Spec:** `genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md`
**Plan:** `genesis/docs/superpowers/plans/2026-05-28-mutual-storage-replication-dwelling-hub-plan.md`
**Branch:** `sprint/cross-pillar-cleanup`

## Commits

| Task | SHA | Subject |
|------|-----|---------|
| T1 | `d51993d90` | feat(elohim-integrity): DNA donut walls — constitutional storage-ratio constants |
| T2 | `69628028d` | feat(views): ReplicatesDwellingPayload schema + ts-rs + schema contract tests |
| T3 | `dfda90bd2` | feat(views): PeerCapacityView + HubCapacityView schemas + ts-rs |
| T3.1 | `3344acc9c` | fix(views): pub-use HubKind from hub_capacity for downstream import paths |
| T4 | `1b78909f0` | feat(views): light up storage-replication into existing topology view schemas |
| T5 | `78d0ea21b` | feat(manifest): elohim constitutionalRatios block + reciprocity-imbalance signal_kind |
| T6 | `7239f5613` | feat(storage): constitutional_ratio_registry — manifest-driven donut clamped to DNA walls |
| T7 | `e27ce61b3` | feat(bounds): ViolationKind::ConstitutionalRatioBreach variant |
| T8 | `993ddac9c` | feat(storage): replicates_dwelling_validator — first bounds-validator-pattern instance |
| T9 | `e0bb8b5a5` | feat(mishpat): replicates-dwelling Commitment action + validator + 6 unit tests |
| T10 | `87b1464d5` | feat(mishpat-integrity): defense-in-depth replicates-dwelling validation |
| T11 | `6d938d4a8` | feat(storage): mutuality_audit_log table + diesel model + CRUD |
| T12 | `cf0110c7a` | feat(storage): mutuality_audit_service — first per-scale aggregator instance |
| T13 | `048d323ca` | feat(storage): peer_capacity_service — PeerCapacityView computation |
| T14 | `3b0c3e8ef` | feat(storage): hub_capacity_service — aggregates PeerCapacityView per HubKind |
| T15 | `8ce097757` | feat(storage): compute projection_tier + over_replicated + faultDomainDiversity |
| T16 | `3ee99f90e` | feat(storage): replication_prioritizer — scores inventory advertisements vs commitments |
| T17 | `3a13d44be` | feat(storage): GET /peer/{cid}/capacity + /hub/{id}/capacity + diagnostics/mutuality-audit |
| T18 | `3464d8d15` | test(sweettest): two-conductor replicates-dwelling substrate-correct test |
| T19 | `7c9c09f2b` | test(a2o): household resiliency handshake + constitutional ratio + @wip disaster burst |
| T20 | (this commit) | docs(memory): Sprint 3 close-out + dwelling-hub replication pattern memory |

> Note: commit `1642c9669` ("feat(memory): self-coherence cleanup …") is interleaved in `dev..HEAD` but is **not** a Sprint 3 task — it is the operator's parallel memory-coherence workstream that landed on this branch during the sprint. See "Parallel-work note" below.

## What landed

- `Mishpat::Commitment` action `replicates-dwelling` end-to-end (coordinator full-validation T9 + integrity substring defense-in-depth T10). **Zero new DHT entry types.**
- DNA-locked donut walls (T1) + manifest-declared ratios (T5) + `constitutional_ratio_registry` clamping manifest→DNA walls (T6).
- `replicates_dwelling_validator` (T8) — **first concrete instance of the bounds-validator pattern**: schema check → donut check → delegate to `bounds_validator::validate`.
- `BoundsViolation::ConstitutionalRatioBreach` variant (T7).
- `mutuality_audit_log` table + CRUD (T11) + `mutuality_audit_service` (T12) — **first per-scale aggregator instance** (sweep → classify Matched/Pending/Breached → emit reciprocity-imbalance on breach → persist log row).
- `reciprocity-imbalance` FeedbackSignal kind registered in the elohim manifest (T5; debit_weight 8, decay_days 60).
- `PeerCapacityView` + `HubCapacityView` schemas + ts-rs (T3) and services (T13/T14); `HubCapacityView` mirrors `HubComputeAggregateView`.
- Topology view extensions (T4 schema/struct + T15 computation): `DistributionSummary.projectionTier` + `over_replicated`; `DistributionDetails.replicationCommitments` + `faultDomainDiversity`; `replica-peer.shardsHeld`/`shardsByEncoding`; `HouseholdResilienceView.commitmentBackedReplication`.
- `replication_prioritizer` (T16) — scores inventory-gossip advertisements vs active commitments (High/Skip this sprint; Medium reserved for commons tier).
- Three diagnostic HTTP routes (T17): `/api/v1/peer/{cid}/capacity`, `/api/v1/hub/{id}/capacity`, `/api/v1/diagnostics/mutuality-audit?hub=…`.
- Two-conductor sweettest (T18, `#[ignore]`-gated — runs in Jenkins with packed DNAs).
- Three a2o feature files (T19; one `@wip-collective-steward`).

## Adaptations during execution

- **HubKind reuse (T3.1):** `infrastructure::HubKind` already existed; redeclaring would clobber the generated `HubKind.ts`. Reused it via `pub use` so downstream `elohim_views::hub_capacity::HubKind` resolves. Same variants + snake_case serde.
- **Tier/ProviderRole clobber avoidance (T4):** the plan's "new `Tier`" would clobber `peer_capacity::Tier`'s generated `.ts`; used a distinctly-named `CommitmentTier` (3 variants, matches schema) and reused `replicates_dwelling::ProviderRole`.
- **T4 constructor ripple:** new required fields on `DistributionSummary`/`DistributionDetails`/`HouseholdResilienceView`/`ReplicaPeer` forced updates at 16 constructor sites (2 services incl. `graph_views/shefa/distribution.rs`, + 4 test fns). All seeded with placeholder defaults; T15 replaced the computable ones.
- **CommitmentRecord ↔ payload_json gap (T12):** Sprint 2's `CommitmentRecord` has `bounds: Value` + top-level `provider`/`recipient`, no `payload_json`/`signed_at`. The audit service reads `provider_role`/`grace_period_days` from `bounds`, uses `c.provider`/`c.recipient`/`c.cid`, and uses `c.valid_from` as the authoring timestamp. (Option (b): no struct extension → no caller ripple.)
- **codegen-ts.mjs (T2):** added `commitments/` to the generateFromDir subdir loop; as a side-effect the two pre-existing Sprint-1 commitment schemas gained barrel exports (correct, not churn).

## Explicit Sprint-3 stubs (preserved; filled by follow-up sprints)

- `mutuality_audit_service::find_counter` → `Ok(None)` (bilateral counter lookup needs by-pair query / conductor bridge).
- `mutuality_audit_service::emit_reciprocity_imbalance` → log-only (FeedbackSignal emission via hc_client pending).
- `peer_capacity_service`: `query_latest_total_raw_bytes` / `aggregate_pledges_by_tier` / `compute_unique_shard_bytes` → 0/empty (real data-source queries pending).
- `hub_capacity_service::resolve_hub_members` → single-device fallback (hub_id IS peer_id).
- `DistributionDetails.replication_commitments` → empty Vec; `HouseholdResilienceView.commitment_backed_replication` → zeros (rea_commitments-by-content query pending).
- `projection_tier` distinct-regions input → 0 (coarse tier from projector_count; region-diversity refinement pending).

## Pre-existing inherited debt surfaced (NOT introduced by this sprint — outside scope, flagged for operator)

These exist on `origin/dev` independent of Sprint 3; this sprint made the `schema_contract` binary compile (T2 added the missing `EprPublishInput { event: None }` initializer), which then *exposed* latent runtime failures:

- `elohim-storage/tests/schema_contract.rs`: **4 failing tests** — `epr_publish_input_conforms` (republish `event` struct↔schema drift) + 3× `tending_policy_payload_*` (the `manifest-payloads/tending-policy.schema.json` `$ref`s `epr:schema:manifest:tending-policy-floor`, an `epr:` scheme the test harness registry can't resolve). All new Sprint-3 schema_contract tests pass; these 4 are unrelated.
- `elohim-storage/tests/reciprocity_view*`: pre-existing `bounded_by` compile error on `NewEconomicEvent` (confirmed predates sprint).
- `holochain/tests/sweettest/.../recognition_participation_via_route.rs`: pre-existing `role_name()`→`type_name()` compile error (the new T18 test sidesteps it via `cells().first()`).

Recommend a small follow-up to reconcile the republish `event` schema and register the `epr:` scheme in the schema_contract harness so the binary goes fully green.

## Parallel-work note (operator)

The operator developed a memory-coherence feature in parallel during this sprint. Effects on this branch: commit `1642c9669` (interleaved between T10 and T11) and uncommitted working-tree files (`.claude/hooks/memory-coherence-signal.py`, `.claude/memory-kit/*`, `.claude/scripts/memory-kit/memory-coherence-audit.py`, `genesis/docs/superpowers/specs/2026-05-28-in-flight-memory-coherence-design.md`, plus `doorway/.../storage_events_subscriber.rs` and `genesis/data/lamad/content/lamad-spa.json` modifications). Some of these were materialized into the working tree by a `git stash` mishap during T4 (see operator handoff note); **no data was lost** (all 13 stashes intact, every sprint commit verified to contain only its intended files). The operator's `Jenkinsfile` + `generate-pipeline-list.mjs` pipeline WIP was never staged by the sprint.

## Follow-up sprints unblocked

1. **Encryption envelope + key custody** — peer↔hub end-to-end; production-readiness precondition.
2. **`replicates-collective` action + collective-tier handshake** — `collective_steward` mode end-to-end (currently schema-reserved + explicitly rejected by `replicates_dwelling_validator`) + membership-attestation chain.
3. **`replicates-commons` action + commons-class filter** + close the floor-via-declaration gap (declaration must be backed by an active commitment).
4. **Conductor-bridge wiring** — `find_counter` by-pair query + `emit_reciprocity_imbalance` real FeedbackSignal; real data-source readers for `peer_capacity_service`; rea_commitments-by-content for `replication_commitments` / `commitment_backed_replication`.
5. **Doorway projection compute agreements** (second compute-commitment instance).
6. **Distributed workloads** (third compute-commitment instance).

## Manual operator-watch acceptance checklist

- [ ] `pnpm run hc:start:seed` brings up local stack with 2+ simulated dwelling-hubs
- [ ] `curl http://localhost:8090/api/v1/peer/<peer_cid>/capacity` → `compliantWithDonut: true` on a fresh peer
- [ ] Author a replicates-dwelling commitment via CLI; PeerCapacityView reflects pledge; mutuality_audit_log shows Pending
- [ ] Advance clock past grace_period; reciprocity-imbalance signal appears in stream (after conductor-bridge wiring)
- [ ] Author a ratio-breaching commitment; storage validator → 400 with ConstitutionalRatioBreach
- [ ] Jenkins: run the two-conductor sweettest (`--ignored`) with packed mishpat.dna
