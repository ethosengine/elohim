# EPR Foundation Wave 0 — Post-Consolidation Re-Audit Results

**Date:** 2026-05-15
**Audited against:** dev @ 7a57fb57fd86d7c5b553f9178efc5b958fb6d91b
**Consolidation reference:** 34fcf1070 + a01e274e3
**Cross-sprint binding:** M4 brainstorm landed at /projects/elohim/genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md (D1=sibling RecoveryFlowProjector, D2=duality)

---

## Preliminary: Plan file gap

The EPR delivery master (`2026-05-11-epr-delivery-master.md`) references four sub-plan files by name:
- `2026-04-24-epr-phase-2b-plan.md` — **does not exist** in the filesystem
- `2026-04-30-epr-phase-3-manifest-resolver-plan.md` — **does not exist**
- `2026-05-01-light-up-the-graph-plan.md` — **does not exist** (design doc exists at `genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md`)

The P3.5 plan (`2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md`) and the W2A/W2B plans do exist.
The attestation batch kickoff prompts (`genesis/docs/plans/2026-04-24-epr-phase-2b-brainstorm-kickoff-prompt.md`, batch-b, batch-c, batch-d) serve as the P2B record. All audit slices below were run against the surviving plan files plus the kickoff prompts and the codebase itself.

---

## Audit Slice 1 — Bespoke entry type references

### Scope

The consolidation (Stage C) removed 22+ bespoke attestation entry types distributed across four DNAs:
- **mishpat**: `GateDecisionAttestation`, `ProposalVote`, `StatementVote`, `GovernanceReaction`, `Proposal`, `Challenge`, `GateDecisionChallenge` (15→8 entry types)
- **infrastructure**: `HealthAttestation`, `DoorwayHeartbeatSummary`
- **elohim**: 2 vestigial entry types (audited in Stage C.4)
- **imagodei**: 5 safe-removal entry types (Stage C.2)

### Grep results across surviving EPR plan files

```
rg "RevocationAttestation|KeyRevocationAttestation|GovernanceReaction|ProposalVote|StatementVote|GateDecisionAttestation|HealthAttestation|DoorwayHeartbeat"
  genesis/docs/superpowers/plans/ genesis/docs/plans/ --glob "*.md"
```

Matches found only in:
1. `2026-05-11-epr-delivery-master.md:32,53,239,242,244` — references `RevocationAttestation` and `KeyRevocation` as **IntegrityNotify pipeline signal kinds** (not as DHT entry types)
2. `2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md:5,332,350` — same; references `RevocationAttestation` as a **wire signal kind**, not as a bespoke entry type
3. `2026-04-26-epr-phase-2b-batch-d-execution-kickoff-prompt.md:14,46` — references `RevocationAttestation`, `AgentPeerBinding` as **integrity exception routing** bypass kinds in `is_integrity_kind()` hardcode

### Classification

| Plan reference | Original context | Post-consolidation status | Rationale |
|---|---|---|---|
| `epr-delivery-master.md` — `RevocationAttestation` in IntegrityNotify W2B row | Wire signal kind for recovery | **Irrelevant-post-consolidation as entry type; valid as signal kind** | `RevocationAttestation` is now a Category C DNA signal (per `revocation-attestation.schema.json`), not a bespoke entry type. The plans correctly refer to it as a signal kind. No entry type assumption survives. |
| `epr-w2b-integrity-notify-keyrotation-plan.md` — `RevocationAttestation` deferred per D3 | IntegrityNotify consumer | **Genuinely-irrelevant-post-consolidation** — deferred to graph-native sprint per master D3 | The signal exists; the consumer wiring is deferred. No orphaned task. |
| `2026-04-26-epr-phase-2b-batch-d-execution-kickoff-prompt.md` — `is_integrity_kind()` hardcode for `RevocationAttestation` | EPR Batch D integrity exception routing | **Landed-against-consolidation** — `is_integrity_kind()` hardcode is a transport-layer classifier, not a DHT entry type. The function routes by wire-signal kind string. The bespoke entry type removal does not change this classification. | Verify at `elohim/elohim-storage/src/epr_atom_service.rs` lines 74, 284+ — `handle_integrity_notify` match arms remain valid against signal kind strings. |

### Result

**0 orphaned tasks.** No EPR sub-plan task was conditional on a bespoke attestation entry type that is now gone. The plan references to `RevocationAttestation` are uniformly references to the DNA signal kind (which still exists as `revocation-attestation.schema.json`) — not to a DHT entry type. The `is_integrity_kind()` bypass routes by signal-kind string and is consolidation-neutral.

---

## Audit Slice 2 — Legacy HTTP route references

### Scope

Stage E consolidated 25+ legacy attestation routes into 8 unified routes. The 8 consolidated routes are:

| Unified route | Purpose |
|---|---|
| `GET /api/v1/attestations/unified` | List by subjectCid / issuerCid / kind |
| `GET /api/v1/attestations/unified/{id}` | Fetch single attestation projection |
| `POST /api/v1/attestations/unified/{id}/revoke` | Record revocation |
| `POST /api/v1/attestations/unified` | Upsert projection row (admin/backfill) |
| `GET /api/v1/governance-actions` | List by subjectCid or openOnly |
| `GET /api/v1/governance-actions/{id}` | Full detail (parent + votes + tally) |
| `GET /api/v1/governance-actions/{id}/tally` | Tally only |
| `POST /api/v1/governance-actions/{id}/vote` | Record a vote |

Source: `elohim/elohim-storage/src/api/attestations.rs:6–10`, `governance_actions.rs:4–7`, `http.rs:8978–9020`.

### Legacy routes still present

A second set of four **content-attestation** routes (`GET/POST /api/v1/attestations`, `POST /api/v1/attestations/{id}/revoke`, `GET /api/v1/attestations/{id}`) remain registered in `http.rs:8954–8972` and implemented in `api/attestations.rs:103–190`. These routes target the pre-consolidation `content_attestations` table (a different table from the unified `attestations` projection). The module doc at `api/attestations.rs:11–15` explicitly labels them "Legacy content-attestation routes (pre-consolidation)."

**These are a different surface from the 25+ bespoke-type routes that Stage E removed.** The 25+ removed routes were per-bespoke-type routes (e.g., `POST /api/v1/imagodei/attestations/humanness`, `POST /api/v1/mishpat/attestations/governance-reaction`). The four content-attestation routes predate the bespoke type era and serve a separate `content_attestations` table still used by the lamad pillar's content-quality flow.

### a2o feature file grep

```
rg "attestation|governance-action|/api/v1/attest|GET.*attest|POST.*attest"
  genesis/a2o/features/content/epr-content-addressing.feature
  genesis/a2o/features/federation/epr-cross-peer-resolution.feature
```

Results: The feature files contain no direct HTTP route assertions. The two attestation-narrative references (`epr-cross-peer-resolution.feature:74,103`) are Gherkin prose describing attestation visibility — not step definitions invoking routes. No step definitions in `genesis/a2o/steps/ui/epr-content.steps.ts` reference attestation routes by URL.

### @wip count discrepancy

The kickoff prompt states 6 `@wip` per feature file. Actual count as of HEAD:
- `epr-content-addressing.feature`: **9 @wip** (lines 27, 39, 52, 65, 79, 93, 106, 118, 130)
- `epr-cross-peer-resolution.feature`: **9 @wip** (lines 70, 86, 100, 116, 128, 142, 154, 167 — 8 counted, one comment at line 24 notes @wip lifted by Wave 0 audit)

Net: 17 `@wip` scenarios remain across both files (vs. the 12 stated in the kickoff). Wave 5 lift scope is larger than documented.

### Result

**0 stale route hits in a2o feature files.** The legacy content-attestation routes (`/api/v1/attestations` non-unified) are a pre-existing parallel surface, not a Stage E regression. They do not appear in EPR scenario step definitions. The Wave 5 prerequisite for route alignment does not apply to a2o — but **the @wip count is 17, not 12**: surface this as a scope adjustment for Wave 5.

---

## Audit Slice 3 — AttestationProjector fit for W2D

### AttestationProjector as-found

File: `elohim/elohim-storage/src/services/attestation_projector.rs`

`handle_content_signal` (lines 29–71) is an **accumulator** with two routing branches:
- `content_type.starts_with("attestation:")` → upsert into `attestations` + optional tally recompute
- `content_type.starts_with("governance-action:")` → upsert into `governance_actions` + initialise zero-count tally

The module doc (lines 1–11) explicitly scopes this module to "attestation + governance-action Content entries." The `resolve_manifest_ref` helper (lines 183–213) already maps `attestation:revocation-*` to "imagodei" — confirming that revocation-vote attestation children land in `attestations` correctly via this projector.

### W2D requirements vs. AttestationProjector grain

W2D's `key_revocations` projection serves `derive_compromise_at`, which:
1. Is state-machine-shaped (a revocation lifecycle: open → quorum → effective)
2. Requires a temporal `compromise_at` derivation from vote timestamps
3. Maps to a `governance-action:key-revocation` governance opener + `attestation:revocation-vote` children

The `attestation:revocation-vote` children **already land in `attestations` via AttestationProjector** (the `attestation:revocation-*` prefix is mapped to "imagodei" in `resolve_manifest_ref`). What W2D needs is a **state-machine controller** that reads the child rows from `attestations` plus the parent row from `governance_actions`, computes `compromise_at`, and writes that derived value to `key_revocations`. This is architecturally distinct from accumulation.

### M4 decision binding

The M4 brainstorm (D1) decided: **Sibling `RecoveryFlowProjector`** at `elohim/elohim-storage/src/services/recovery_flow_projector.rs`. EPR W2D's `key_revocations` table writer is co-located in `RecoveryFlowProjector`, not in `AttestationProjector`.

The call site routing:
- `attestation:*` | `governance-action:*` (non-recovery) → `AttestationProjector::handle_content_signal`
- `governance-action:key-revocation` | `key-revocation:*` | recovery-flow kinds → `RecoveryFlowProjector::handle_content_signal`

### Verdict

**Sibling confirmed from M4 — architectural sense confirmed from EPR audit side.**

The EPR audit independently arrives at the same verdict: `AttestationProjector` is a pure accumulator that must not be turned into a mixed accumulator+controller. The `key_revocations` projection requires compromise-window derivation logic (`derive_compromise_at` computing the retroactive sweep boundary) that does not compose as a branch in `handle_content_signal`'s match. The M4 D1 decision is architecturally sound from the EPR perspective. EPR W2D imports from `recovery_flow_projector.rs`, not from `attestation_projector.rs`, leaving `AttestationProjector` unmodified.

**No concerns from the EPR-side audit.** The seam is clean: `attestation:revocation-vote` children continue to land in `attestations` via `AttestationProjector` (correct); only `governance-action:key-revocation` opener signals route to `RecoveryFlowProjector` to drive the state machine and write `key_revocations`.

**`recovery_flow_projector.rs` does not yet exist** at HEAD — it is the primary deliverable of EPR Wave 1 (W2D) and Recovery M4 Stage 1/2. Confirmed: `find /projects/elohim/elohim/elohim-storage/src/services/ -name "recovery_flow_projector*"` returns no results.

---

## Audit Slice 4 — Phase 4 closure

### rg result for TODO(Phase 4 follow-up)

```
rg "TODO\(Phase 4 follow-up\)" elohim/ doorway/ steward/
```

**3 surviving markers** (none were removed by commit `dbd0947c1` — that commit is not in the current HEAD history at 7a57fb57f):

| File | Line | Content | Status |
|---|---|---|---|
| `elohim/elohim-storage/src/services/distribution_view.rs` | 98 | `projector_count: stubbed 0 (TODO Phase 4 follow-up — no projector table yet)` | **Doc-stale / code-landed** — the function body at lines 159–166 queries `projection_events::distinct_projectors_for_blob`, which is real. The `projection_events` table exists (migration at `migrations/2026-05-11-110000_projection_events`). Doc comment lags the implementation. |
| `elohim/elohim-storage/src/services/distribution_view.rs` | 99 | `diversity_hint: stubbed None (TODO Phase 4 follow-up — no geo/archetype index yet)` | **Doc-stale / code-landed** — the function body at lines 165–178 queries `peer_identity_bindings.device_archetype` and calls `peer_diversity::diversity_hint_from_archetype_strs`. Real implementation is present. |
| `elohim/elohim-storage/src/services/peer_topology_view.rs` | 49 | `resilience_cliffs stubbed vec![] — TODO Phase 4 follow-up.` | **Partially doc-stale** — The aggregate function (lines 185–189) calls `compute_resilience_cliffs(&mut conn, agent_cid)` which is a real `compute_resilience_cliffs` function defined at line 230+. However, the doc comment at line 49 of the OLD `aggregate_peer_topology_view` stub path still says `stubbed vec![]`. The _local_ resolver at lines 185–189 computes real values; the federation-aggregate path at line 71 still returns `resilience_cliffs: vec![]`. **Partially real, partially stub.** |

### Other Phase 4 deliverables (from master plan topology table)

| Deliverable | Code evidence | Status |
|---|---|---|
| `projector_acks` table + write path | No `projector_acks` table found in `migrations/` or `db/`. The `projection_events` table covers distinct-projector queries. | `projector_acks` as a separate table was **not landed** — `projection_events` serves the same query surface. Functionally equivalent but the naming from the master plan's topology table differs. |
| `projection_events` append-only log | `db/projection_events.rs` + migration `2026-05-11-110000_projection_events` both present | **Landed** |
| Device capacity totals minus committed | `reciprocity_view.rs:91` — `capacity_available_bytes` computed from `system_metrics` + `rea_commitments` | **Landed** |
| Display-name resolution via imagodei lookup | `reciprocity_view.rs:184–186` — calls `imagodei_lookup::resolve_display_name(pool, &counterparty)` | **Landed** |
| Online annotation from libp2p connected_peers | `reciprocity_view.rs:201` — calls `connectivity::any_online_in` | **Landed** |
| `geo`/`archetype` link tag + diversity composer | `distribution_view.rs:165–178` — queries `device_archetype` from `peer_identity_bindings` + `peer_diversity::diversity_hint_from_archetype_strs` | **Landed** |
| Sole-replica resilience analysis | Local path landed (`compute_resilience_cliffs` at `peer_topology_view.rs:230`); federation-aggregate path still returns `vec![]` stub | **Partially landed** |
| Manifest registry layer-1 | `main.rs:1142–1153` — calls `manifest_registry::load_pillar_manifest_layer1` with graceful-degrade | **Landed** |

### Commit `dbd0947c1` claim

The master plan's kickoff prompt says "Commit `dbd0947c1` claims to have removed stale TODO(Phase 4 follow-up) comments." This commit is **not present in the HEAD history** at `7a57fb57f`. The 3 surviving `"TODO Phase 4 follow-up"` markers in distribution_view.rs and peer_topology_view.rs are **doc-stale** (the code they describe is implemented), not code-stale. The commit claim is premature — it either was not pushed or refers to a future commit.

### Phase 4 closure verdict

**W1 Phase 4 is substantially landed**, with two caveats:
1. The three surviving `"TODO Phase 4 follow-up"` doc-comment markers in `distribution_view.rs` and `peer_topology_view.rs` need to be updated to reflect that the code is real. This is a documentation cleanup, not a code gap.
2. The federation-aggregate path in `peer_topology_view.rs:71` still returns `resilience_cliffs: vec![]` even though the local resolver computes real values. This is a Wave 1 tail: the federation path must be wired to call `compute_resilience_cliffs` rather than hardcode empty.

---

## Decisions surfaced for the operator

### D1 — sibling RecoveryFlowProjector (confirmed)
The M4 brainstorm decision stands from the EPR audit perspective. File `elohim/elohim-storage/src/services/recovery_flow_projector.rs` is the primary net-new deliverable for EPR Wave 1 (W2D). EPR W2D must import from this module, not from `attestation_projector.rs`. The call-site prefix router (location to be confirmed per M4 D1 sub-question) routes `governance-action:key-revocation` and `key-revocation:*` to the sibling.

### D2 — duality wins for RevocationAttestation (confirmed)
EPR D3 = duality. The existing `revocation-attestation.schema.json` contract stands. EPR W2B reads slim operational payload; no `contentEnvelope` field; no cross-sprint schema coordination required. The `key_revocations` table writer reads `actionHash` + progress-state fields. `attestation:revocation-vote` children land in `attestations` via `AttestationProjector` (no change).

### D3 — W2B KeyRotation arm has landed (new finding)
The W2B `integrity_notify_keyrotation` handler is **already wired** at `epr_atom_service.rs:340–384`. File `p2p/recovery_rotation.rs` exists. Tests `integrity_notify_keyrotation_acks_received_true` and `integrity_notify_keyrotation_dedup_returns_duplicate_reason` both present at lines 453 and 489. The W2B plan's checkbox list (`2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md`) shows all `[ ]` unchecked — this is plan-tracking debt. **W2B is landed; the plan needs its boxes ticked.** This changes the Wave 2 scope: W2B is done, not pending.

### D4 — GetDocument variant still stub
`p2p/epr_protocol.rs:46` still carries `/// Get document-tier content by ID (stub — not yet implemented)`. The handler at `epr_atom_service.rs:436` returns `EprResponse::NotFound`. This matches the master plan's D4 recommendation (defer to graph-native). No change to scope.

### D5 — @wip count is 17, not 12
Wave 5 scope is 17 `@wip` scenarios (9 in `epr-content-addressing.feature`, 8 in `epr-cross-peer-resolution.feature`), not 12 as stated in the kickoff. The extra 5 were likely added between the kickoff draft and this audit. Adjust Wave 5 effort estimate upward.

### D6 — W2A record_predecessor wiring is landed (new finding)
`p2p/mod.rs:5317–5372` contains the live `record_predecessor` call on Content-kind EPR Atom Announce. `api/epr.rs:189–191` comment confirms the wiring landed in `p2p/mod.rs`. W2A plan (`2026-05-11-epr-w2a-record-predecessor-plan.md`) shows all `[ ]` unchecked — plan-tracking debt. **W2A is landed; boxes need ticking.**

### D7 — epr_2b_batch_a_full_loop still #[ignore]'d (pending W2D + Jenkins gate)
`holochain/tests/sweettest/src/tests/epr_phase_2b_batch_a_e2e.rs:648` carries `#[ignore = "requires packed imagodei DNA artifact ... also requires Stage 2 derive_compromise_at upgrade"]`. Two conditions must be met to lift this: (1) Jenkins pack-then-test stage for `imagodei.dna` artifact, and (2) `derive_compromise_at` in `holochain_app_signal.rs`. Condition 2 is the W2D deliverable. This test remains the EPR sprint's final acceptance gate.

### D8 — doc-stale "TODO Phase 4 follow-up" comments need cleanup
The three surviving markers in `distribution_view.rs:98–99` and `peer_topology_view.rs:49` are doc-stale (the code they describe is real). These should be updated in the same commit that wires the federation-aggregate path's `resilience_cliffs` to use `compute_resilience_cliffs`. This is a small Wave 1 tail, not a blocker.

---

## Phase status updates

| Phase | Prior audit status | Post-consolidation status | Notes |
|---|---|---|---|
| P1 codec | ✅ LANDED | ✅ AUDIT-CONFIRMED post-consolidation | No attestation entry type dependency; codec is transport-only |
| P2A storage foundation | ✅ LANDED | ✅ AUDIT-CONFIRMED post-consolidation | No bespoke-type dependency; EprAtom/EprCoupling projections unchanged |
| P2C libp2p federation | ✅ LANDED | ✅ AUDIT-CONFIRMED post-consolidation | `is_integrity_kind()` bypass uses signal-kind strings; consolidation-neutral |
| P2B identity + projector + signal | ✅ AUDIT-CONFIRMED on `0f3ffa20d` | ✅ AUDIT-CONFIRMED post-consolidation | W2A (record_predecessor) landed in p2p/mod.rs; W2B (KeyRotation) landed in epr_atom_service.rs; only `epr_2b_batch_a_full_loop` `#[ignore]` remains (D7 above; unblocked by W2D `derive_compromise_at`) |
| P3 manifest resolver | ✅ AUDIT-CONFIRMED on `33438cdd8` | ✅ AUDIT-CONFIRMED post-consolidation | No attestation entry type dependency |
| P3.5 trust-compute gradient | ✅ AUDIT-CONFIRMED on `e122ec072` | ✅ AUDIT-CONFIRMED post-consolidation | T22 (record_predecessor) now confirmed landed; aunt-and-rage-bait still passes |
| LUG Light Up the Graph | ✅ AUDIT-CONFIRMED on `4ea4e1558` | ✅ AUDIT-CONFIRMED post-consolidation | T18 (= W2A) confirmed landed; production swarm adapters present |
| W1 Phase 4 projector controller | 🟡 AUDIT-CONFIRM PENDING | 🟡 SUBSTANTIALLY LANDED — needs follow-up: (1) doc-stale TODO comments in distribution_view.rs + peer_topology_view.rs need update; (2) federation-aggregate path for resilience_cliffs returns empty vec — wire to compute_resilience_cliffs; (3) no `projector_acks` table (projection_events serves equivalent query) | All core deliverables landed; two tail items remain |
| W2A record_predecessor | ⏳ PENDING | ✅ LANDED — plan-tracking debt only (boxes unchecked) | p2p/mod.rs:5317–5372 is live code |
| W2B IntegrityNotify KeyRotation | ⏳ PENDING | ✅ LANDED — plan-tracking debt only (boxes unchecked) | epr_atom_service.rs:340–384 + p2p/recovery_rotation.rs are live |
| W2C GetDocument | ⏳ decision-gated | ⏳ DEFERRED to graph-native sprint per D4 | epr_protocol.rs:46 stub remains; no change |
| W2D key_revocations + derive_compromise_at | ⏳ NEW | ⏳ PENDING — key_revocations table exists (migration `2026-04-24-010000_key_revocations`); `derive_compromise_at` not yet present; RecoveryFlowProjector not yet created | Unblocks epr_2b_batch_a_full_loop #[ignore] |
| W3 a2o @wip lift | ⏳ PENDING | ⏳ PENDING — 17 @wip scenarios (not 12); no stale route references | Wave 5 scope adjustment needed |
