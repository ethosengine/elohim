---
status: Draft
---

# Records Lifecycle — Part D Substrate Gaps Plan (Revised After Phase 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This plan is **operator-led** — intended to be executed inline with human review at each subsection. Findings from Phase 1 ([phase2-findings-synthesis.md](./2026-05-24-records-lifecycle-phase2-findings-synthesis.md)) expanded the gap list from 10 to 19 (Gap 1+2 merged into D.1; 10 new gaps from Phase 1). Each subsection now resolves the operator-decisions surfaced by Phase 1 alongside the spec content.

**Goal:** Land the Part D subsection content in [`records-lifecycle-design.md`](../../content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md) for all 19 substrate-lifecycle gaps, in the wave order Phase 2 synthesis identified, with operator decisions resolved per subsection.

**Architecture:** Five execution waves (A through E) ordered by prerequisite dependency. Wave A unblocks everything else (schema-first IoC governance + substrate-floor validator backfill + missing view schemas + observation spec implementation). Waves B and C are substrate-primitive and lifecycle-operation work. Waves D and E cover patterns/interop and validator/policy.

**Tech Stack:** Markdown spec editing of `genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md`. Spec subsections reference (but do not implement) Holochain DNA Rust changes, Diesel migrations, pillar manifest JSON, view schemas, codegen artifacts. Actual implementation is a follow-up sprint plan; this plan completes the *spec*.

---

## Architectural context (carried from Phase 1)

This plan completes the lifecycle wiring identified during the brainstorming conversation that produced [`records-lifecycle-design.md`](../../content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md). Phase 1 dispatched 7 rust-architect agents to write the primitive walkthroughs (Part A.2–A.8) plus surface concerns. Those concerns revealed:

- **Substrate vocabulary surfaces have drifted independently.** 4 of 8 application archetypes reference vocabulary (action verbs, signal_kinds, attestation subtypes, resource classifications) that fails substrate validation today. The schema-first IoC discipline hasn't been applied uniformly.
- **Substrate-floor enforcement is aspirational, not actual.** Attestation validator Floor F2/F4/F6 are ACCEPT-all stubs. Retention-class validation isn't wired. LINK_ARCHITECTURE.md deprecation checklist is incomplete.
- **Event-sourcing at scale needs snapshot primitives.** Balance materialization, graduation throughput, standing re-derivation all face read-side cost growth that requires a checkpoint/aggregate primitive.
- **Hard-cutover retirements are migration landmines.** Gap 5 (StewardedResource consolidation) and Gap 6 (DoorwayHeartbeat retirement per observation spec Stage 6) both need derived views wired BEFORE the entry type is retired.

The synthesis document is the single source of truth for the meta-patterns and the operator-decision queue. This plan operationalizes those findings into spec subsection drafts.

## Operator-decision resolution discipline

Each gap subsection has zero or more operator decisions that must resolve before the subsection's spec content is written. The pattern per task:

1. **Read the relevant findings** from Phase 2 synthesis
2. **Resolve the operator decisions** (interactively with operator) and record the decision in the spec subsection
3. **Write the spec subsection content** with decisions baked in
4. **Commit**

Architecture-shaping decisions (10 of them, per synthesis) are flagged per subsection below. Implementation-detail decisions can be resolved subsection-by-subsection without blocking the wave.

## File structure

Part D edits land in a single file:

- Modify: `genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md` — replace stubbed Part D subsections (D.1–D.9 today) with full design blocks for all 19 subsections (D.1–D.19, where D.1 covers merged Gaps 1+2).

The spec file itself is the deliverable. Implementation (actual Rust + Diesel + manifest + codegen changes) is a follow-up sprint plan.

## Subsection ↔ Gap mapping

| Subsection | Gap | Wave | Status |
|---|---|---|---|
| D.1 | Gaps 1+2 (subordination links + parent_epr_cid) | B | reframed + addenda |
| D.2 | Gap 3 (surface re-elevation) | C | reframed |
| D.3 | Gap 4 (submerge canonical signal) | C | expanded |
| D.4 | Gap 5 (EconomicResource consolidation) | B | reframed + prerequisite |
| D.5 | Gap 6 (Observation spec implementation) | A | unchanged + addendum |
| D.6 | Gap 7 (elohim-authoring pattern) | D | unchanged + addenda |
| D.7 | Gap 8 (dissolution semantics) | C | unchanged + addendum |
| D.8 | Gap 9 (bridge pattern) | D | unchanged + addenda |
| D.9 | Gap 10 (reach-mutation Events) | C | unchanged + addendum |
| D.10 | Gap 11 (schema-first IoC governance) | A | NEW |
| D.11 | Gap 12 (substrate-floor validator backfill) | A | NEW |
| D.12 | Gap 13 (checkpoint/snapshot primitive) | B | NEW |
| D.13 | Gap 14 (missing view schemas) | A | NEW |
| D.14 | Gap 15 (standing-curve view + policy) | E | NEW |
| D.15 | Gap 16 (cross-DNA coordination patterns) | D | NEW |
| D.16 | Gap 17 (multi-oracle confirmation-class) | D | NEW |
| D.17 | Gap 18 (recurring Commitment stagger) | D | NEW |
| D.18 | Gap 19 (FeedbackSignal signal_class field) | D | NEW |
| D.19 | Gap 20 (agent-private EPR-mediated key recovery) | D | NEW (REFRAMED) |
| D.20 | Gap 21 (Layered Commons + elohim-mediated Global Commons) | D | NEW |

---

# Wave A — Prerequisites (unblocks everything else)

Wave A closes the substrate-debt that's causing application archetypes to fail validation. Until these land, downstream gaps pay a higher cost because they ship against vocabulary that hasn't been declared. Wave A is the highest-leverage move surfaced by Phase 1.

## Task A1: D.10 Schema-First IoC Governance (Gap 11) ⭐ HIGH IMPACT

**Files:**
- Modify: `genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md` D.10 (new subsection — does not currently exist)
- Reference reads: `feedback_schema_first_ioc` memory (project_schema_first_ioc); CI scripts at `pnpm run schema:codegen:ts`, `pnpm run lamad:codegen`

**Operator decisions to resolve:**
- None (this is a meta-gap; the design follows from the schema-first IoC pattern already established)

**Coverage:**
- **Motivation**: Phase 1 found 4 application archetypes (Mint, Photos, Meta, Patreon) referencing vocabulary that fails substrate validation today because Rust whitelist + JSON schema + pillar manifest + codegen output are drifting independently. Plus codegen bugs like the `$ref` sentinel in `ATTESTATION_KINDS`.
- **Design**: A unified CI gate (`pnpm run schema:check-extensibility-vocabulary`) that for every extensible vocabulary (action verbs, signal_kinds, observation_kinds, attestation_kinds, resource_classifications, content_types) validates:
  1. Every value in the Rust whitelist appears in the JSON schema enum
  2. Every value in the JSON schema enum appears in at least one pillar manifest declaration
  3. Every manifest-declared value is reflected in the codegen output
  4. No literal codegen artifacts (`$ref`, partial paths) leak into whitelists
- **Failure mode if not wired**: Application archetypes ship referencing vocabulary that fails Floor 1 validation. Surfaces as 503/401 cascades downstream (per `feedback_schema_data_enum_drift_cascade`).
- **Manifest declaration**: each pillar manifest gains a `vocabulary_declarations:` section that the CI gate reads
- **Code anchors**:
  - `elohim/sdk/schemas/scripts/` — new `check-extensibility-vocabulary.mjs` script
  - `package.json` root + per-pillar — new `schema:check-extensibility-vocabulary` npm script
  - `.husky/pre-push` — add to the changed-projects detection
  - `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs` — `$ref` sentinel bug fix (Phase 1 finding)
- **Test surface**:
  - Property: every value in each whitelist exists in the corresponding schema enum
  - Property: every schema enum value has at least one manifest declaration
  - Regression: the `$ref` sentinel cannot reappear

- [ ] **Step 1: Draft subsection D.10 in records-lifecycle-design.md after the existing D.9 stub**
- [ ] **Step 2: Operator review of D.10 content**
- [ ] **Step 3: Commit D.10**: `git commit -m "spec(architecture): records-lifecycle D.10 — schema-first IoC governance (Gap 11)"`

## Task A2: D.11 Substrate-Floor Validator Backfill (Gap 12) ⭐ HIGH IMPACT

**Files:**
- Modify: records-lifecycle-design.md D.11 (new)
- Reference reads: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs` (Floors F2/F4/F6 ACCEPT-all stubs marked TODO(C.3) + Floor 5 temporal TODO Task C.2); `LINK_ARCHITECTURE.md` deprecation checklist (incomplete); observation spec §7.3 (retention class validation)

**Operator decisions to resolve:**
- None (each floor's enforcement is already specified in the relevant canonical spec; this gap just executes them)

**Coverage:**
- **Motivation**: Phase 1 found that several substrate-floor invariants are aspirational documentation, not enforced code. ACCEPT-all stubs let arbitrary capability claims through; retention-class validation can't reject `lamad:sensor-biometric` with `retention_class: wisdom`; ~50 `*By{Attribute}` link types violate DHT-as-notary but haven't been retired.
- **Design — three coordinated backfills**:
  1. **Attestation validator floors**: close Floor F2 (steward authorization for `attestation:mastery`), F4 (issuer eligibility for `attestation:content-quality`), F6 (subject domain match) per attestation-consolidation spec §4.2; close Floor 5 temporal completeness (Task C.2 — pass the op's Action timestamp through for strict `closes_at` enforcement)
  2. **Manifest schema validator extension**: reject `retention_class: wisdom | archival | attestation-feeding` on `observation_kind` schemas whose `subject_kind = environment | sensor` (per observation spec §9.1 constitutional constraint)
  3. **LINK_ARCHITECTURE.md deprecation sweep**: formally retire the ~50 `*By{Attribute}` query-index link types that violate DHT-as-notary (move query indices to SQL projection); reclaim slots toward the 256-cap
- **Sequencing**: these are independent backfills but should land together so the substrate-floor enforcement story moves from aspirational to actual in one coherent sprint
- **Code anchors**:
  - `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs` — Floors F2/F4/F6 + Floor 5 TODOs
  - `elohim/sdk/schemas/scripts/validate-observation-kinds.mjs` (or similar) — new retention-class manifest validator
  - `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — LinkTypes enum retirement of `*By*` variants
  - `elohim/holochain/dna/LINK_ARCHITECTURE.md` — update deprecation checklist
- **Test surface**:
  - Integrity zome: invalid attestation issuance rejected at write time
  - Manifest validation: sensor observation with `retention_class: wisdom` rejected
  - Link types: retired `*By*` variants no longer accepted

- [ ] **Step 1: Draft subsection D.11**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.11**: `git commit -m "spec(architecture): records-lifecycle D.11 — substrate-floor validator backfill (Gap 12)"`

## Task A3: D.13 Missing View Schemas + Enum Reconciliation (Gap 14)

**Files:**
- Modify: records-lifecycle-design.md D.13 (new)
- Reference reads: `elohim/sdk/schemas/v1/views/` (existing schemas + the missing `economic-resource-view.schema.json`); `attestation-view.schema.json` (`proofClass` enum drift); `p2p/feedback-signal.schema.json` (`signalKind` enum drift)

**Operator decisions to resolve:**
- None (schema authorship is mechanical; the decisions about enum canonical values are made downstream in Task B-series subsections that touch those vocabularies)

**Coverage:**
- **Motivation**: Phase 1 found `economic-resource-view.schema.json` doesn't exist; `proofClass` enum in `attestation-view.schema.json` differs from validator; `forget-request` is in `SIGNAL_KINDS` whitelist but missing from `p2p/feedback-signal.schema.json` enum. Each drift produces silent cascade failures (per `feedback_schema_data_enum_drift_cascade`).
- **Design**: three coordinated schema reconciliations:
  1. Author `economic-resource-view.schema.json` per the 10-rule conventions at `elohim/sdk/schemas/v1/views/CONVENTIONS.md`
  2. Reconcile `proofClass` enum across `attestation-view.schema.json`, `attestation_validator.rs`, and `2026-05-01-computation-attestation-graduated-rigor-design.md` (canonical: `witness | audit | proof | confirmation`)
  3. Add `forget-request` to `p2p/feedback-signal.schema.json` `signalKind` enum (currently has 5 values; needs the 6th to match whitelist)
- **Code anchors**:
  - `elohim/sdk/schemas/v1/views/economic-resource-view.schema.json` — new file
  - `elohim/sdk/schemas/v1/views/attestation-view.schema.json` — enum reconciliation
  - `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json` — enum addition
  - `elohim/elohim-storage/tests/schema_contract.rs` — add ResourceView contract test
  - `elohim/sdk/schemas/scripts/codegen-ts.mjs` — add ResourceView to INTERFACE_FILES

- [ ] **Step 1: Draft subsection D.13**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.13**: `git commit -m "spec(architecture): records-lifecycle D.13 — missing view schemas + enum reconciliation (Gap 14)"`

## Task A4: D.5 Observation Spec Implementation + Stage 6 (Gap 6)

**Files:**
- Modify: records-lifecycle-design.md D.5 (existing stub, replace with full subsection)
- Reference reads: `genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md` end-to-end

**Operator decisions to resolve:**
- None (the observation spec already has its own staged implementation plan; this subsection just references it as a prerequisite chain)

**Coverage:**
- **Motivation**: Records-lifecycle wires graduation from Observation to Event/Attestation; the Observation tier itself must be implemented before that wiring is meaningful. Stage 6 (retire `DoorwayHeartbeat`, `DoorwayHeartbeatSummary`, `HealthAttestation`) is the specific cleanup that disambiguates EPR-vs-Observation at the boundary.
- **Sequencing within the observation spec**:
  - Stage 1: Manifest declarations
  - Stage 2: Wire format + ALPN
  - Stage 3: Storage tables
  - Stage 4: ObservationManagerBackend service
  - Stage 5: Graduation evaluator
  - Stage 6: Retire DoorwayHeartbeat/DoorwayHeartbeatSummary/HealthAttestation
  - Stage 7: HTTP API + storage-client
  - Stage 8: Existing-table reclassification
- **Hard-cutover discipline (Phase 1 addendum)**: Stage 5 (graduation evaluator) must work end-to-end with green tests BEFORE Stage 6 retirement. Otherwise heartbeat consumers go dark.
- **Code anchors**: (references the observation spec's own code anchors; does not duplicate)

- [ ] **Step 1: Draft subsection D.5 (replace existing stub)**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.5**: `git commit -m "spec(architecture): records-lifecycle D.5 — Observation spec implementation prerequisite (Gap 6)"`

## Wave A checkpoint

After D.10 + D.11 + D.13 + D.5 land, the substrate-vocabulary surfaces are governed, the floor enforcement is actual, the missing schemas exist, and the observation prerequisite chain is documented. Wave B (substrate primitives) can now proceed without re-deriving these foundations.

- [ ] **Wave A coherence pass**: skim Part D D.5, D.10, D.11, D.13 for cross-references and consistency

```bash
git log --oneline | head -8  # verify 4 commits for Wave A
```

---

# Wave B — Substrate Primitives

Wave B specifies the core lifecycle-primitive additions: subordination links + field, Resource consolidation, and the checkpoint/snapshot primitive that resolves event-sourcing read-side cost.

## Task B1: D.1 Subordination Architecture (Gaps 1+2)

**Files:**
- Modify: records-lifecycle-design.md D.1 (existing stub, replace)
- Reference reads: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` (LinkTypes enum + EconomicEvent + EconomicResource structs); existing `ContentToResource` link type (Phase 1 found this overlaps)

**Operator decisions to resolve:**
1. **`ContentToResource` vs `EprToResource` — replace or coexist?** Both point at the same relationship from the same source type. Replace = clean but breaks existing callers; coexist = drift risk where query authors don't know which to use.
2. **Canonical projection target for link adjacencies — CozoDB graph_views or Diesel adjacency tables?** Both exist (`graph_views/` module + planned `epr_event_edges` / `epr_resource_edges` tables). Designate one as canonical; the other becomes a derived/secondary view.

**Coverage:**
- **Motivation**: Subordinate Events and Resources need a parent EPR so their custody and gossip cost shed under the parent's reach scope.
- **Design — link types**: `EprToEvent` and `EprToResource` variants added to `LinkTypes` enum
- **Design — field**: `parent_epr_cid: Option<String>` added to both `EconomicEvent` and `EconomicResource` structs; `Option` for backward-compatibility with existing parentless entries
- **Coordinator functions**: `create_event_under_epr(parent_epr_cid, event_data)` and `create_resource_under_epr(parent_epr_cid, resource_data)`
- **Validation rules**: integrity zome rejects link creation when parent EPR is `closed` (interlocks with Gap 8 dissolution); rejects when authoring agent has no relationship to parent (custody check via Membership link)
- **SQL adjacency tables (Phase 1 addendum)**: Diesel migration creates `epr_event_edges` and `epr_resource_edges` projection tables; field + link + adjacency table ship as a triple (Phase 1 finding: field-without-link is incomplete graph binding; link-without-projection is unqueryable at render time)
- **ContentToResource resolution (Operator decision 1)**: record the decision in the spec; if "replace", include migration sub-task that retires `ContentToResource` from the LinkTypes enum
- **Canonical projection target (Operator decision 2)**: record the decision; the other backend becomes a derived/secondary view
- **Manifest declaration**: which `content_type` parent EPRs accept which child Event/Resource shapes
- **Query patterns**: `list_events_under_parent(parent_epr_cid) → Vec<EconomicEventView>`; same for Resources
- **Migration**: pre-launch hard cutover; no shim
- **Code anchors**:
  - `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — LinkTypes + struct field additions
  - `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — new coordinator functions
  - `elohim/elohim-storage/migrations/<date>-epr-adjacency-tables/up.sql` — new Diesel migration
  - `elohim/elohim-storage/src/graph_views/` — CozoDB integration (if canonical)
  - `elohim/elohim-storage/src/views.rs` — extend Event + Resource view types with `parent_epr_cid`
  - `elohim/sdk/domains/*/manifest.json` — accepted-child declarations

- [ ] **Step 1: Resolve operator decisions 1 and 2 interactively**
- [ ] **Step 2: Draft subsection D.1 with decisions baked in**
- [ ] **Step 3: Operator review**
- [ ] **Step 4: Commit D.1**: `git commit -m "spec(architecture): records-lifecycle D.1 — subordination architecture (Gaps 1+2) + adjacency tables"`

## Task B2: D.4 EconomicResource Consolidation (Gap 5)

**Files:**
- Modify: records-lifecycle-design.md D.4 (existing stub, replace)
- Reference reads: `EconomicResource` AND `StewardedResource` structs in integrity zome

**Operator decisions to resolve:**
3. **`governed_by` field landing** — StewardedResource has it (references collective governance handle); EconomicResource has no analog. Options: add `governed_by: Option<String>` field; route through a new Commitment+link pattern; or fold into `resource_classified_as` metadata.
4. **`data_quality` field landing** — StewardedResource has `data_quality: enum { measured, estimated, manual, mixed }`. Monarch data-confidence view depends on it. Same options as above.

**Coverage:**
- **Motivation**: REA discipline says one canonical "resource with state" type. Current substrate has two (EconomicResource + StewardedResource) — duplication that confuses callers.
- **Design**: retire `StewardedResource` as distinct entry type; fold its fields into `EconomicResource` via `resource_classified_as` discrimination (e.g., `"stewarded-physical"`, `"stewarded-digital"`)
- **Field migration mapping (with operator-decided landings for governed_by + data_quality)**: enumerate exactly which StewardedResource fields go where in EconomicResource
- **Derived views (hard-cutover prerequisite per Phase 1)**: derived views for `total_capacity_value`, `total_used_value`, `available_value`, `allocations_json`, `recent_usage_json`, `trends_json` MUST be implemented and green-tested BEFORE the retirement commit. The capacity-planning + household-resilience + node-stewardship dashboards depend on these.
- **Manifest declaration**: pillar manifests declare stewardship classifications (`stewarded-physical`, `stewarded-digital`, `stewarded-compute`, etc.)
- **Migration strategy**: pre-launch hard cutover; all existing StewardedResource callers migrate to EconomicResource with classification set; one Diesel migration drops the old table; one TS codegen pass refreshes types
- **Budget win**: -1 variant in elohim DNA EntryTypes enum
- **Code anchors**: integrity zome + coordinator + migration script + derived-view services

- [ ] **Step 1: Resolve operator decisions 3 and 4 (governed_by + data_quality landings)**
- [ ] **Step 2: Draft subsection D.4 with decisions baked in + derived-view prerequisite explicit**
- [ ] **Step 3: Operator review**
- [ ] **Step 4: Commit D.4**: `git commit -m "spec(architecture): records-lifecycle D.4 — EconomicResource consolidation (Gap 5) + derived-view prerequisite"`

## Task B3: D.12 Checkpoint / Snapshot / Aggregate-Subordination Primitive (Gap 13) ⭐

**Files:**
- Modify: records-lifecycle-design.md D.12 (new subsection)

**Operator decisions to resolve:**
5. **Signal-Aggregate Commitment threshold** — when does a signal stream become eligible for subordination? Time-based (signals older than 90 days), count-based (more than N signals on a single `target_cid`), or standing-curve-based (after the standing-curve has crystallized)?
6. **Balance checkpoint trigger** — automated on row-count thresholds, periodic schedule, or manifest-declared per-resource policy?

**Coverage:**
- **Motivation**: Phase 1 found multiple read-side cost-growth problems on event-sourced primitives (balance materialization on 10yr Resource history; standing-curve re-derivation at hub scale; signal-aggregate DHT budget pressure). A unified checkpoint/aggregate-subordination primitive is the release valve.
- **Design — two new Commitment action verbs**:
  - `Commitment(action="checkpoint", subject_cid, period_start, period_end, projection_summary)` — authorizes a read-side balance snapshot. The Commitment carries the summary; subsequent reads against the same Resource can shortcut by treating events older than `period_end` as already-summed.
  - `Commitment(action="aggregate-subordinate", target_cid, signal_kind, window, aggregate_metrics)` — subordinates a window of FeedbackSignals under the Commitment + custody-quilt for cold archive. Per A.7 Phase 1 finding, this is the DHT budget release valve for signal-dense content.
- **Manifest declaration**: per-pillar policies for when to checkpoint and when to aggregate-subordinate
- **Coordinator functions**: `create_checkpoint_commitment`, `create_aggregate_subordination_commitment`
- **Validation rules**: checkpoint requires authorship from current custodian or stewardship-elohim; aggregate-subordination respects reach scope
- **Code anchors**:
  - `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — REA_ACTIONS additions (`checkpoint`, `aggregate-subordinate`)
  - `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — new coordinator functions
  - `elohim/elohim-storage/src/services/checkpoint_service.rs` (planned) — read-side shortcut logic
  - `elohim/sdk/domains/*/manifest.json` — per-pillar checkpoint policies

- [ ] **Step 1: Resolve operator decisions 5 and 6**
- [ ] **Step 2: Draft subsection D.12**
- [ ] **Step 3: Operator review**
- [ ] **Step 4: Commit D.12**: `git commit -m "spec(architecture): records-lifecycle D.12 — checkpoint/snapshot/aggregate-subordination primitive (Gap 13)"`

## Wave B checkpoint

After D.1 + D.4 + D.12 land, the substrate primitives have their connective tissue: subordination links + parent_epr_cid, single-canonical-Resource via consolidation, and the checkpoint/aggregate-subordination release valve.

```bash
git log --oneline | head -7  # 4 Wave A + 3 Wave B = 7 commits
```

---

# Wave C — Lifecycle Operations

Wave C specifies the lifecycle-transition operations: surface re-elevation, submerge canonical signal, dissolution semantics, reach-mutation.

## Task C1: D.2 Surface Re-elevation (Gap 3)

**Files:**
- Modify: records-lifecycle-design.md D.2 (existing stub, replace)

**Operator decisions to resolve:**
7. **CID continuity mechanism** — `update_entry` on the existing entry (new action hash, same CID — Holochain native) OR coordinator-side state machine that tracks lifecycle state via Events (never mutating entry). Phase 1 (A.3) flagged this as unresolved tension between immutability and CID-continuity claim.
8. **Surface authorship authority** — custodian's lineage only? Custodian OR elohim with active stewardship commitment? Mishpat governance for high-stakes surfaces?

**Coverage:**
- **Motivation**: The resell-the-couch case — a Resource demoted to subordinate or shelved needs a path back to active EPR-tier status
- **Design (depending on operator decision 7)**:
  - If `update_entry`: surface authors `update_entry(resource_cid, updated_resource)` with new `parent_epr_cid` and `lifecycle_state="active"`; CID stable; new action hash records the transition
  - If coordinator state machine: surface authors `Event(action="surface", subject=resource_cid)`; ReconcileController updates SQL projection's `lifecycle_state`; entry itself never mutates; current state is derived from event history
- **Authorship validation (depending on decision 8)**: integrity zome enforces the authority model
- **Manifest declaration**: `action: "surface"` verb in elohim manifest with stake-class
- **Custody transfer**: surface may include a new `parent_epr_cid` (transfer custody) or `None` (Resource becomes free)
- **Code anchors**:
  - `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — `surface_resource` coordinator function

- [ ] **Step 1: Resolve operator decisions 7 and 8**
- [ ] **Step 2: Draft subsection D.2 with decisions baked in**
- [ ] **Step 3: Operator review**
- [ ] **Step 4: Commit D.2**: `git commit -m "spec(architecture): records-lifecycle D.2 — surface re-elevation (Gap 3)"`

## Task C2: D.3 Submerge Canonical Signal (Gap 4)

**Files:**
- Modify: records-lifecycle-design.md D.3 (existing stub, replace)
- Reference reads: `2026-05-10-memory-lifecycle-design.md` + `2026-05-11-tiered-quilt-stewardship-design.md`

**Operator decisions to resolve:**
9. **`shelf_destination` vocabulary expansion** — Phase 1 found memory-lifecycle's 7 socio-institutional submerge destinations (personal subconscious, therapist collective, research observatory, government-encrypted evidence store, cultural archive, lineage archive, peer cellar) don't map to tiered-quilt's infrastructure-only `shelf_destination` (`peer-cellar://`, `external-archive://minio/`). Options: extend the schema to cover socio-institutional destinations; add a parallel field for socio-institutional routing; or designate that socio-institutional destinations route via a different Commitment action verb.

**Coverage:**
- **Motivation**: Memory-lifecycle and tiered-quilt have parallel vocabularies for "moved from active to cold-archive." Reconcile behind a single canonical authoring event.
- **Design — canonical authoring form**: `Commitment(action="custody-quilt", tier_floor="shelved")` authored by an elohim-agent
- **ReconcileController fan-out**: when the Commitment lands, ReconcileController projects it as both:
  - The `submerge` lifecycle operation in memory subsystem (per memory-lifecycle spec)
  - The `quilt-demoted` storage-tier transition (per tiered-quilt spec)
- **Both downstream specs gain amendment notes** pointing to D.3 as the canonical authoring event
- **`shelf_destination` vocabulary expansion (Operator decision 9)**: record the decision; if extending schema, specify the values for socio-institutional destinations
- **Cancellation flow (addendum from Phase 1 A.5 concerns)**: who authors cancellation, what happens to in-progress fulfillment Events, whether cancellation of custody-quilt triggers steward handoff
- **Code anchors**:
  - `elohim/elohim-storage/src/services/reconcile_controller.rs` (planned) — fan-out projection
  - `elohim/sdk/schemas/v1/views/commitment-view.schema.json` — `shelf_destination` enum expansion

- [ ] **Step 1: Resolve operator decision 9**
- [ ] **Step 2: Draft subsection D.3 with decision + cancellation flow**
- [ ] **Step 3: Operator review**
- [ ] **Step 4: Commit D.3**: `git commit -m "spec(architecture): records-lifecycle D.3 — submerge canonical signal (Gap 4) + shelf_destination expansion + cancellation flow"`

## Task C3: D.7 Dissolution Semantics (Gap 8)

**Files:**
- Modify: records-lifecycle-design.md D.7 (existing stub, replace)

**Operator decisions to resolve:**
- None at architecture-shaping level; implementation-detail decisions per the synthesis (custody Commitment lifecycle when CID dissolves) baked into the subsection

**Coverage:**
- **Motivation**: When something gets thrown away, that's a substrate event; how it's thrown away matters
- **Design — close/revive lifecycle**:
  - `Event(action="dispose")` transitions subject Resource or EPR to `lifecycle_state: "closed"`
  - Future-Event validation fails when targeting a closed entry (substrate-floor invariant)
  - `Event(action="revive")` reopens (for genuine mistakes / accidental disposals)
- **Field projection**: `lifecycle_state: enum { active, subordinate, shelved, closed }` on Event and Resource view schemas. Derived from event history in the projection; not stored on the entry itself (mass-balance discipline).
- **Validation invariant** (substrate-floor): integrity zome rejects new Events whose `subject_cid` resolves to a closed Resource/EPR, unless the new Event is `action="revive"`
- **Custody Commitment lifecycle (Phase 1 addendum)**: when subject CID transitions to closed, outstanding custody Commitments on that CID auto-fulfill or cancel; BreachScanner skips closed CIDs (no `tier-breach` Attestations on dissolved content). Sequencing: mishpat governance authors closure, ReconcileController fans out to close custody Commitments.
- **Manifest declaration**: `dispose`, `close-account`, `close-organization`, `revive` action verbs in elohim manifest
- **Cradle-to-cradle hook (deferred)**: `Event(action="dispose")` MAY include `disposition_kind` (`recycled` / `landfill` / `transferred-to-charity` / `sold-on-resell`) for future cradle-to-cradle accounting
- **Code anchors**: integrity zome validation + coordinator + view schemas

- [ ] **Step 1: Draft subsection D.7**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.7**: `git commit -m "spec(architecture): records-lifecycle D.7 — dissolution semantics (Gap 8) + custody Commitment lifecycle"`

## Task C4: D.9 Reach-Mutation Events (Gap 10)

**Files:**
- Modify: records-lifecycle-design.md D.9 (existing stub, replace)

**Operator decisions to resolve:**
- None at architecture-shaping level

**Coverage:**
- **Motivation**: Reach is the substrate's nervous system; when reach changes, that mutation should itself be auditable
- **Design — three new action verbs**: `grant-reach`, `revoke-reach`, `reclassify-reach`. Each authored as an Event whose subject is the EPR/Resource being mutated.
- **Validation**: reach-change validation in integrity zome; checks current standing of authoring agent + reach-grant chain (can you grant reach you don't have?); council-arbitration for `commons` and `commons-attested` elevations
- **Manifest declaration**: action verbs in elohim manifest with stake-class `high` (reach matters — not graduatable from observations)
- **Audit trail**: reach-history is a derived view over reach-mutation Events on a given EPR/Resource
- **Interlock with Gap 13 (D.12 aggregate-subordination)**: reach-mutation is the trigger that may also cause Signal-Aggregate Commitment to land (when a post's reach widens then contracts, its accumulated signals may aggregate-subordinate)
- **Code anchors**: integrity zome validators + coordinator + reach-state derived view

- [ ] **Step 1: Draft subsection D.9**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.9**: `git commit -m "spec(architecture): records-lifecycle D.9 — reach-mutation Events (Gap 10)"`

## Wave C checkpoint

After D.2 + D.3 + D.7 + D.9 land, the lifecycle gradient has its full set of transitions: surface re-elevation, submerge canonical, dissolution, reach-mutation.

```bash
git log --oneline | head -11  # 4 Wave A + 3 Wave B + 4 Wave C = 11 commits
```

---

# Wave D — Patterns + Interop

Wave D specifies the patterns and interop layer: elohim-authoring specialization, bridge pattern for legacy, cross-DNA coordination, multi-oracle confirmation, recurring stagger, FeedbackSignal class isolation, agent-private key derivation.

## Task D1: D.6 Elohim-Authoring Pattern (Gap 7)

**Operator decisions to resolve:**
10. **Graduation-evaluator architecture** — single tokio task per pillar (current) vs sharded (by kind_namespace, by provider, by EPR) vs separate elohim-graduator service. Rate-ceiling per-subject/period model. Hub-failover for community-scoped Attestations when issuing hub offline.

**Coverage:**
- **Motivation**: The records-lifecycle's value-prop unlock — elohim narrating mundane care into REA — requires domain-specialized agents
- **Design**: domain-elohim subscribes to specific `observation_kind` namespaces per manifest; runs as tokio task (or separate worker per operator decision); evaluates graduation policies per its domain; authors Events / Attestations / Subordination links on behalf of the household
- **Authority**: domain-elohim authors with stewardship-commitment Attestation; household authorizes at setup; revocation closes the Commitment
- **Floor-permissive**: humans can author the same Events directly
- **Graduation evaluator architecture (Operator decision 10)**: record the architectural decision
- **Bridge stewardship-elohim parallel-author fallback (Phase 1 addendum)**: when a bridge stewardship-elohim is offline, no new Observations enter the graduation pipeline. Specify if/how a backup stewardship-elohim can take over.
- **Code anchors**: `app/elohim-app/src/app/elohim/elohim-agents/` (planned TS services); `elohim-storage/src/services/graduation_evaluator.rs` (extended); pillar manifests

- [ ] **Step 1: Resolve operator decision 10**
- [ ] **Step 2: Draft subsection D.6**
- [ ] **Step 3: Operator review**
- [ ] **Step 4: Commit D.6**: `git commit -m "spec(architecture): records-lifecycle D.6 — elohim-authoring pattern (Gap 7) + graduation-evaluator architecture"`

## Task D2: D.8 Bridge Pattern (Gap 9)

**Operator decisions to resolve:**
- None at architecture-shaping level (implementation details — PII removal, KYC migration — baked into the subsection)

**Coverage:**
- **Motivation**: Legacy systems exist outside substrate; bridges enable parallel operation + subsumption-by-merit; cash-out is bidirectional
- **Bridges as substrate-native Collectives (REFRAMED per operator)**: bridges aren't passthrough adapters — they're substrate-native Collective EPRs whose stewardship-elohim agents handle legacy-protocol translation as a service. Plaid Inc, Stripe Inc, banking-API providers, KYC vendors each have a Collective EPR with their own employees as Memberships; their commercial revenue flows into their Bridge Commons.
- **Design**: per-vendor `bridges/<vendor>/` crate (speaks vendor API on one side; authors substrate-native Observations + Events under stewardship-elohim signature on the other); HTTP routes via doorway-service for OAuth/webhook ingress; bidirectional
- **Reference pattern**: `bridges/valueflows/`
- **Countersignature continues to work through bridges (A-2 resolution)**: bridge-stewardship-elohim signs receiver-side on behalf of not-yet-substrate-native participants (stub-EPR with subsumption-claim path; when the actual party joins substrate, they attest/claim the stub-EPR's history). This preserves the bilateral-transfer invariant.
- **Fee mechanics defer to D.20**: the layered-Commons fee-split (Bridge Commons + Global Commons) is specified in D.20. D.8 specifies the bridge-translation machinery only; D.20 specifies how value flows into the Bridge Commons.
- **Backfill pattern**: legacy-data import → batch-graduated Events under stewardship-elohim with `observation_kind: "bridge:backfill"`
- **Cash-out**: bridge-collective authors `Event(action="cash-out", provider=bridge_commons, receiver=legacy-USD-bank-account)` to convert Commons-held value to legacy money. Substrate doesn't subsidize legacy translation — bridges do, at their own cost, recouped via Commons fee.
- **PII removal addendum (Phase 1 A.6)**: define how `attestation:forget-decision` flow interacts with bridge-authored Attestation `metadata_json` containing PII (e.g., KYC name/DOB fragments). Coordinator-side scrub of PII fields on forget-decision; storage projection drops PII columns
- **KYC bridge migration addendum (Phase 1)**: when household migrates from one KYC bridge to another, existing credentials stay valid (append-only DHT) but old bridge can no longer issue new ones; specify the credential-transfer flow
- **Manifest declaration**: per-pillar bridge kind acceptance + per-bridge fee schedules (feed D.20)
- **Code anchors**: `bridges/<vendor>/`; `doorway/doorway-service/src/handlers/bridges/`; pillar manifests; bridge-collective Membership patterns; stewardship-elohim signing patterns

- [ ] **Step 1: Draft subsection D.8**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.8**: `git commit -m "spec(architecture): records-lifecycle D.8 — bridge pattern (Gap 9) as substrate-native Collectives + PII + KYC migration"`

## Task D3: D.15 Cross-DNA Coordination Patterns (Gap 16) [new]

**Operator decisions to resolve:**
- None at architecture-shaping level

**Coverage:**
- **Motivation**: Phase 1 found cross-DNA link creation (e.g., `GovernanceActionChild` from elohim → mishpat) requires `call(CallTargetCell::OtherRole(...))` bridge pattern that is not documented. Hub-graduation failover for community-scoped Attestations is undefined. Meta archetype's "Membership / Relationship link" zome ownership is ambiguous.
- **Design — three coordinated patterns**:
  1. **Cross-DNA link creation pattern**: standardized `call(CallTargetCell::OtherRole("target_dna"), "create_link_in_dna", payload)` shape; documented with worked example for `GovernanceActionChild`
  2. **Hub-graduation failover**: when issuing hub goes offline mid-window, a secondary hub takes over; specifies the failover detection (peer-heartbeat absence) + the takeover authority (next-hub-by-standing)
  3. **Social-graph zome ownership**: Meta archetype's friend/follow primitives — decision: do they reuse imagodei's `AgentToRelationship` or lamad's `ContentToRelated` or get new social-graph link types in imagodei zome?
- **Code anchors**: DNA boundaries (`elohim/holochain/dna/elohim/`, `imagodei/`, `mishpat/`); hub-failover detection in `peer_transport_manifest` watcher

- [ ] **Step 1: Draft subsection D.15**
- [ ] **Step 2: Operator review (the social-graph zome ownership decision deserves attention)**
- [ ] **Step 3: Commit D.15**: `git commit -m "spec(architecture): records-lifecycle D.15 — cross-DNA coordination patterns (Gap 16)"`

## Task D4: D.16 Multi-Oracle Confirmation-Class Attestation (Gap 17) [new]

**Operator decisions to resolve:**
- None at architecture-shaping level

**Coverage:**
- **Motivation**: Phase 1 A.6 found that `attestation:price-feed` from a single oracle-elohim is a centralized trust chokepoint that the entire Monarch dashboard depends on. The `proof_evidence.class = confirmation` tier exists but the multi-attestor chain format is not specified anywhere.
- **Design — multi-attestor confirmation pattern**:
  - `proof_evidence.confirmer_signatures: Vec<Signature>` (each confirmer signs the same fact)
  - Manifest-declared minimum confirmer count + diversity requirements (e.g., 3+ confirmers, distinct collectives)
  - Validator floor: confirmation-class attestations rejected without minimum confirmer count + diversity met
  - Applies to: `attestation:price-feed`, AWS-compute `attestation:computation` verification, any high-stakes commons-reach attestation
- **Code anchors**: `attestation_validator.rs` (Floor 8 extension); `attestation-view.schema.json` (confirmer_signatures field); pillar manifests (confirmer requirements per subtype)

- [ ] **Step 1: Draft subsection D.16**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.16**: `git commit -m "spec(architecture): records-lifecycle D.16 — multi-oracle confirmation-class attestation (Gap 17)"`

## Task D5: D.17 Recurring Commitment Scheduler Stagger (Gap 18) [new]

**Operator decisions to resolve:**
- None at architecture-shaping level

**Coverage:**
- **Motivation**: Phase 1 A.5 found that 1M-patron Patreon-shape recurring billing creates thundering-herd on creator's projection node on first-of-month. No stagger discipline exists in the Commitment scheduler today (TierController has one).
- **Design**: analog of TierController's `blake3(cid || peer-id || epoch) % stagger_window`, applied to Commitment fulfillment:
  - `stagger_seconds = blake3(commitment_id || agent_id || billing_epoch) % BILLING_STAGGER_WINDOW`
  - Scheduler delays fulfillment by `stagger_seconds` within the billing window
  - Per-pillar manifest declares `BILLING_STAGGER_WINDOW_SECONDS` (default e.g. 3600 — 1 hour)
- **Code anchors**: `elohim-storage/src/services/commitment_scheduler.rs` (planned); pillar manifests

- [ ] **Step 1: Draft subsection D.17**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.17**: `git commit -m "spec(architecture): records-lifecycle D.17 — recurring Commitment scheduler stagger (Gap 18)"`

## Task D6: D.18 FeedbackSignal signal_class Field (Gap 19) [new]

**Operator decisions to resolve:**
- None at architecture-shaping level

**Coverage:**
- **Motivation**: Phase 1 A.7 found that `debit-firm` quarantine on a bad-compute provider would violate compute/care isolation silently — the `SIGNAL_KINDS` whitelist doesn't distinguish what kind of standing is being debited.
- **Design**: add `signal_class: enum { care, compute, governance, trust }` field on FeedbackSignal; manifest-declared `signal_kind → signal_class` mapping; standing-curve respects class isolation (care debits affect care standing only; compute debits affect compute standing only; etc.)
- **Code anchors**: `feedback_signal.rs` integrity zome; `p2p/feedback-signal.schema.json` view schema; pillar manifests with signal_kind→signal_class declarations

- [ ] **Step 1: Draft subsection D.18**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.18**: `git commit -m "spec(architecture): records-lifecycle D.18 — FeedbackSignal signal_class field (Gap 19)"`

## Task D7: D.19 agent-private EPR-Mediated Key Recovery (Gap 20) [new — REFRAMED]

**Operator decisions to resolve:**
- RESOLVED: EPR-mediated trusted-circle recovery is primary; cryptographic Shamir is an opt-in add-on. The substrate provides both; manifest declares which a household uses.

**Coverage:**
- **Motivation**: Phase 1 A.4 (and the observation spec open question #3) — when a human migrates to a new device, the observer's `agent-private` encryption key must be recoverable without putting the burden on the human to remember a master passphrase. Per `project_socially_derived_security` + `project_recovery_grandma_standard`: relationships are the primary security primitive; cryptography enforces what relationships authorize.
- **Design — primary path (EPR-mediated)**:
  - Each agent has a `trusted-circle` Membership EPR (intimate household + collective stewards)
  - Recovery flow: human requests recovery → trusted-circle members each author `Attestation(content_type="attestation:key-recovery-authorization", subject=agent_cid)` → when N members have authored (manifest-declared threshold), substrate-floor invariant re-derives the agent's `agent-private` key from the social-attestation chain
  - "Log in with help from your people" applied to encryption keys
- **Design — optional add-on (Shamir secret-sharing)**:
  - Manifest opt-in: households can layer Shamir-N-of-M cryptographic split — each trusted-circle member additionally holds a Shamir share
  - Recovery requires BOTH the EPR-attestation threshold AND the cryptographic share threshold
  - **Defense in depth where external threat surface warrants it**, not as a wealth-class concern. The substrate's friction-gradient (per `project_friction_gradient_limitarianism` + D.20 Global Commons ratchet) prevents accumulation classes from forming structurally; there is no "high net worth" stratum on this protocol. The Shamir layer addresses *external adversarial threat* that makes social-attestation recovery insufficient on its own:
    - Journalists protecting sources from state subpoena
    - Dissidents under state threat
    - Intimate-partner-violence survivors where the trusted-circle pattern itself can be compromised by an abuser within the social graph
    - Households stewarding strategic public-good Resources where the threat surface extends beyond household-scale relationships
- **Manifest declaration**: per-household policy declares `recovery_mode: "epr_mediated" | "epr_mediated_with_shamir"`; `trusted_circle_threshold: N`; `shamir_threshold: { n, m }` (if applicable)
- **Validation**: integrity zome rejects key-recovery without sufficient trusted-circle attestations; coordinator-side flow handles Shamir share verification when manifest opts in
- **Code anchors**: `imagodei` DNA recovery flow; source-chain export logic; multi-device pairing flow; trusted-circle Membership EPR pattern; `attestation:key-recovery-authorization` subtype declaration

- [ ] **Step 1: Draft subsection D.19**
- [ ] **Step 2: Operator review**
- [ ] **Step 3: Commit D.19**: `git commit -m "spec(architecture): records-lifecycle D.19 — agent-private EPR-mediated key recovery (Gap 20)"`

## Task D8: D.20 Layered Commons + elohim-mediated Global Commons (Gap 21) [new]

**Operator decisions to resolve:**
- RESOLVED: Layered Commons (Bridge + Collective + Global) with elohim-mediated Global Commons stewardship per `project_elohim_councils_capture_apex`.

**Coverage:**
- **Motivation**: Every value flow through the substrate ratchets a manifest-declared slice into one or more Commons-held Resources at different governance scopes. **This is where the elohim-apex thinking gets cashed out as actual value flow** — the network self-funds its development, infrastructure, and anti-concentration enforcement through substrate-native fee flows mediated by elohim councils. Not a tax (no enforcement layer); a manifest-declared substrate invariant.
- **Design — three Commons scopes**:
  - **Bridge Commons**: revenue for bridge-collectives (Plaid Inc, Stripe Inc as Collective EPRs) that translate legacy protocols. Their own stewards govern.
  - **Collective Commons**: scope at the Collective level (multi-family pools, credit-union-style mutual aid, organization shared funds). Each Collective EPR has a Commons-Resource governed by its own stewards.
  - **Global Commons**: protocol-wide, **elohim-mediated** per the apex-elohim governance pattern. Funds substrate development, new bridges, public goods, anti-concentration redistribution.
- **Fee mechanics**: each Event(action="transfer") may declare a manifest-defined fee split (e.g., 99% to receiver, 0.5% Bridge Commons, 0.5% Global Commons). Fee flows are themselves Events authored alongside the main transfer. Substrate-floor validation: sum of fee splits + receiver share = 100% of original transfer (mass-conservation).
- **Commons as composable EPRs (no new entry types)**:
  - Each Commons = a `Content` EPR (`content_type: "commons"`) with a Membership of stewards
  - Commons-held value = `EconomicResource` with `parent_epr_cid = commons_epr_cid` and `resource_classified_as = "commons-credit"` (or `currency-USD` for stablecoin-denominated Commons)
  - Commons stewards (elohim agents for Global Commons; collective members for Bridge/Collective) author allocation Events from the Commons
- **Global Commons elohim governance**: the apex elohim councils participate as Memberships of the Global Commons EPR. Allocation Events (funding decisions) require quorum of council attestations per the apex-elohim pattern. **This is the substrate's self-funding mechanism that cannot be captured by self-interest** because elohim councils don't accumulate (per `project_elohim_councils_capture_apex`).
- **Anti-concentration ratchet**: manifest-declared friction-gradient — as an EPR accumulates large balances, a larger fraction of incoming Events ratchets into Commons (per `project_friction_gradient_limitarianism`). Substrate-floor mechanism for limitarianism.
- **Cash-out path**: bridge-collectives can author `Event(action="cash-out", provider=bridge_commons, receiver=legacy-USD-bank, observation_refs=[bank-transfer-confirmation])`. Substrate doesn't subsidize legacy translation — bridges do, at their own cost.
- **Manifest declaration**: per-pillar fee splits, per-bridge fee schedules, per-Collective Commons policies, Global Commons allocation rules
- **Code anchors**:
  - `elohim/sdk/domains/elohim/manifest.json` — Global Commons declarations + apex-elohim council Membership
  - `elohim/sdk/domains/*/manifest.json` — per-pillar fee schedules
  - `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — fee-split validation (mass-conservation check)
  - `elohim/elohim-storage/src/services/commons_steward_service.rs` (planned) — commons stewardship surface

- [ ] **Step 1: Draft subsection D.20**
- [ ] **Step 2: Operator review (apex-elohim council governance is substrate-shaping; careful review)**
- [ ] **Step 3: Commit D.20**: `git commit -m "spec(architecture): records-lifecycle D.20 — layered Commons + elohim-mediated Global Commons (Gap 21)"`

## Wave D checkpoint

After D.6 + D.8 + D.15 + D.16 + D.17 + D.18 + D.19 + D.20 land, the pattern + interop layer is specified.

```bash
git log --oneline | head -19  # 4 + 3 + 4 + 8 = 19 commits
```

---

# Wave E — Validator + Policy

Wave E specifies the standing-curve view and policy declarations.

## Task E1: D.14 Standing-Curve View + Policy Declaration (Gap 15) [new]

**Operator decisions to resolve:**
11. **Standing-curve view update frequency** — real-time (re-derive on every signal arrival) vs eventual-consistency (staleness SLA like 60 seconds)

**Coverage:**
- **Motivation**: Phase 1 A.7 found `standing_scores` is queried as a SQL view/table but the view definition logic is not documented anywhere. The standing-curve policy (decay rate, vouch recovery fraction, debit weights) needs manifest declaration so collectives can tune it.
- **Design — view contract**:
  ```sql
  CREATE VIEW standing_scores AS
  SELECT
    author_id,
    signal_class,
    SUM(impact_value) AS raw_score,
    -- decay-adjusted score per manifest-declared decay_rate
    -- vouch-recovery offset per manifest-declared recovery_fraction
    ...
  FROM feedback_signals
  GROUP BY author_id, signal_class;
  ```
- **Manifest-declared standing-curve policy**:
  ```json
  "standing_curve": {
    "decay_rate_per_day": 0.01,
    "vouch_recovery_fraction": 0.5,
    "debit_soft_weight": -1,
    "debit_firm_weight": -10,
    "update_frequency": "real-time" | "eventual-60s"
  }
  ```
- **Update frequency (Operator decision 11)**: record decision; if eventual-consistency, specify staleness SLA + how feed-ranking handles potentially-stale standing
- **Standing-stewardship-elohim**: the agent that re-derives the view per policy
- **Code anchors**: `elohim-storage/src/db/views/standing_scores.sql` (new); standing-stewardship-elohim service; pillar manifests

- [ ] **Step 1: Resolve operator decision 11**
- [ ] **Step 2: Draft subsection D.14**
- [ ] **Step 3: Operator review**
- [ ] **Step 4: Commit D.14**: `git commit -m "spec(architecture): records-lifecycle D.14 — standing-curve view + policy declaration (Gap 15)"`

---

# Final Task: Part D Coherence Pass + Commit

**Files:**
- Read-only: records-lifecycle-design.md Part D in full

- [ ] **Step 1: Skim each subsection D.1–D.19 for coherence**

```bash
sed -n '/^# Part D /,/^# Part E /p' genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md | less
```

Verify:
- No "TBD", "stubbed", "TODO", or placeholder text remains
- All code anchors point to real file paths (or are explicitly marked "planned")
- Cross-references between subsections are consistent (e.g., D.7 dissolution validation references D.1 subordination invariant; D.10 schema-first IoC references the vocabulary surfaces touched by D.4 + D.16 + D.18)
- Vocabulary is consistent across subsections (`subordinate` not `pooled`; `closed` not `dissolved`; `surface` not `re-elevate`; `signal_class` not `signal_category`)
- Operator decisions are recorded in their respective subsections (not just resolved in conversation)

- [ ] **Step 2: Fix any drift found inline**

- [ ] **Step 3: Final commit**: `git commit -m "spec(architecture): records-lifecycle Part D — coherence pass + Phase 3 complete"`

## Self-Review

**1. Spec coverage:** All 19 substrate-lifecycle gaps from Phase 2 synthesis are addressed in D.1–D.19. ✓

**2. Placeholder scan:** No "TBD"; all design blocks specify concrete entry types, link types, fields, coordinator functions, validation rules, code anchors. ✓

**3. Type consistency:** `EconomicEvent`, `EconomicResource`, `Content`, `Commitment`, `Attestation`, `FeedbackSignal` are the only DHT entry types named; `StewardedResource` retirement is consistent across D.4 and dependent subsections; `parent_epr_cid: Option<String>` is the consistent field name; `lifecycle_state` enum is consistent (`active | subordinate | shelved | closed`); `signal_class` enum is consistent (`care | compute | governance | trust`). ✓

## Execution handoff

This plan is **operator-led** — the design decisions inside each subsection (especially the 11 architecture-shaping operator decisions) need interactive review with the human operator, not autonomous agent work. Execute inline in the current session using `superpowers:executing-plans`. Each task ends in a commit so we have rollback points.

**Wave dispatch:** Wave A first (unblocks everything). After Wave A coherence pass, decide whether to:
- (a) Continue into Wave B inline, OR
- (b) Pause to dispatch Wave 2 (application archetype full-drafts) — now safe because Wave A has closed the vocabulary-drift gaps

Once Part D lands, the records-lifecycle spec is content-complete except for Part C placeholders (deferred per operator direction) and any final coherence pass across Parts A, B, C, D, E.
