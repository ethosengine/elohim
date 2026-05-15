# EPR Foundation Completion (post-attestation) — Phase Closeout for Graph-Native Readiness: Kickoff Prompt

**Date:** 2026-05-15
**Status:** Wave 0 audit complete; D1–D4 resolved; ready for plan-writing
**Precondition:** Attestation Consolidation Sprint A→F merged on `dev` at `34fcf1070`; CID-decode fix at `a01e274e3` (verified by orchestrator dev #950).
**Wave 0 audit:** `genesis/docs/plans/2026-05-15-epr-wave0-audit-results.md` (audited at HEAD `7a57fb57f`)
**Cross-sprint binding:** Recovery M4 brainstorm at `genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md` — M4 D1 = sibling `RecoveryFlowProjector`; M4 D2 = duality (binds EPR D3).

> **Audit headline (sprint scope shrinks):** W2A (`record_predecessor`) and W2B (KeyRotation IntegrityNotify arm) are **already landed in code** — sub-plan checkbox state is plan-tracking debt only. Effective remaining substantive work: **W2D** (`key_revocations` projection + `derive_compromise_at`) is the single substantive piece, plus a small W1 federation-aggregate `resilience_cliffs` tail and the @wip lift (revised to **17 scenarios, not 12**).

---

## Framing

This sprint **closes out the EPR foundation phases (P1 → P3.5 + Phase 4) with an attestation-aware re-audit, finishes the remaining Wave-2 runtime gaps, lifts `@wip` on the EPR a2o scenarios, and leaves the substrate ready for the graph-native sprint** (experience-story / experience-moment / social-reach nervous system).

The companion master plan that scoped this work — `genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md` — was written **before** the attestation consolidation merged. The plans of record showed the foundation as ~ready; the consolidation changed the shape of three things downstream from EPR (the projector signal handler pattern, the `RevocationAttestation` primitive, and the way IntegrityNotify pipeline consumes attestation envelopes). This sprint **revisits the master plan with that lens**, confirms what stayed valid, marks what shifted, and ships the small surgical pieces that remain.

The graph-native sprint that comes after — `experience-story` / `experience-moment` / `:story-point` EPR types and the full social-reach nervous system — is out of scope here. This sprint is the **substrate handoff**: when it closes, the foundation under graph-native work is unambiguous and free of TODO markers.

---

## P2P Design Gate — entity classification (mandatory)

This sprint introduces **zero new DHT entry types** and aims for **zero new SQLite tables**. The `key_revocations` projection (W2D below) is the one possible exception — and its source-of-truth is declared inline as the EPR W2D sub-plan to be drafted in Wave 1 (see master plan §"Sub-plan portfolio" row W2D where the table is already pre-classified).

| Entity / surface | Category | Source of truth | Sprint impact |
|---|---|---|---|
| `EprAtom`, `EprCoupling`, `EprClaims`, `EprSupersedence` projections | **C** — operational | `2026-04-22-elohim-epr-storage-foundation-plan.md` (LANDED Phase 2A) | Read-only audit; no change |
| `Manifest` entry type (4 kinds) | **A** — DHT-notarized | `2026-04-30-epr-phase-3-manifest-resolver-plan.md` (LANDED Phase 3, lamad zome) | Read-only audit; no change |
| `FeedbackSignal`, `AttentionTending`, `CollectiveFilterPattern` | **A** / **B** / **C** | `2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md` §P2P design gate (LANDED Phase 3.5) | Read-only audit; no change |
| `Predecessor` record (sealed 2-of-2 dryoc) | **C** — operational | Same as P3.5; rebuildable from FeedbackSignal arrival logs | W2A finishes `record_predecessor` wiring; no new entity |
| `AgentPeerBinding`, `peer_identity_bindings` + `verified_at` + `verified_signer_fingerprint` | **A** / **C** | `2026-04-24-epr-phase-2b-design.md` §appendix A (LANDED Phase 2B) | Read-only audit; no change |
| `projector_acks` table + `projection_events` log | **C** — operational | EPR delivery master §"Sub-plan portfolio" W1 (Phase 4); already drafted | Audit-confirm; T1–T13 + cleanup commits visible on dev |
| `geo` / `archetype` fields on `peer_identity_bindings` | **A2** — link metadata on binding | Same as W1 (Phase 4) | Audit-confirm |
| `key_revocations` SQLite projection | **C** — operational (rebuildable from `key-revocation:*` Content entries on the elohim DHT) | Migration `2026-04-24-010000_key_revocations` exists; **writer co-located in `RecoveryFlowProjector`** at `elohim/elohim-storage/src/services/recovery_flow_projector.rs` (per M4 D1; file does not yet exist — primary deliverable of EPR Wave 1 + M4 Stage 2). Post-commit signal hooks via prefix-routing dispatcher: `governance-action:key-revocation` | `key-revocation:*` → `RecoveryFlowProjector`. | **The one net-new projection this sprint** |
| `IntegrityNotify` payloads for KeyRotation / RevocationAttestation / AgentPeerBinding | **wire-protocol** — already declared at `elohim/sdk/schemas/v1/dna-signals/*.schema.json` (all 4 sub-contracts exist on dev) | Companion: Recovery M4 prompt §Stage 3 owns the producer side | W2B handles consumer wiring; no new contract |
| `GetDocument` EPR variant | **wire-protocol** — extension to existing `/elohim/epr-atom/1.0.0` request-response codec (D4 decides whether to land this sprint or defer) | `elohim/elohim-storage/src/p2p/epr_protocol.rs:46` currently marks it `not yet implemented` | Decision required (D4 below) |
| `experience-story`, `experience-moment`, `:story-point` (a2o tier 1/2/3) | **A** / **B** / **A2** — graph-native sprint scope | `2026-04-18-experience-story-epr-design.md` | **OUT OF SCOPE — graph-native sprint** |
| Full social-reach nervous system (provenance back-prop, quarantine, restitution) | downstream epic | `project_social_reach_nervous_system` memory pin | **OUT OF SCOPE — graph-native sprint** |

**DNA capacity impact:** zero. Lamad ~73/~100, Mishpat 11/~100, Imagodei 28, Infrastructure 6 — all unchanged by this sprint.
**Net new tables:** one (`key_revocations`, pre-classified in master plan W2D).
**Net new routes:** zero.

---

## Context (self-contained)

### What the consolidation changed that EPR plans should re-read

- **22 attestation subtypes + 7 governance-action kinds** are now emitted from JSON Schema + pillar manifests via codegen (Stage A of consolidation). The legacy bespoke entry types are gone (Stage C — 22+ types removed). The EPR plans were written assuming the bespoke types; that assumption is no longer valid.
- **8 unified HTTP routes** replaced 25+ legacy attestation routes (Stage E). The EPR a2o scenarios may reference the old routes; the @wip lift in Wave 3 must use the new ones.
- **2 unified projection tables + 1 derived tally** replaced 22 legacy attestation projection tables (Stage D). The AttestationProjector signal handler is the canonical pattern for new projection wiring — the W2D `key_revocations` writer should mirror it.
- **F1/F5/F7/F8 validator floors are live** (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs`). The parent-entry-lookup bug fixed today by `a01e274e3` was in floor5 + floor7: validator now uses `EntryHash::try_from(parent_cid)` mirroring the coordinator's existing `EntryHash::to_string()` round-trip — the parent reference is anchored to the parent's EntryHash (already a content address on the Holochain DHT), no foreign-key indirection introduced. The bug surfaced because the dispatch graph started running the consolidated attestation_coordinator suite for the first time on dev — see `AP-pre-existing-bug-exposed-by-unrelated-changeset` from the prior shift's sprint result.
- **Stage G partial — humanness bridge + Shamir scaffold**. Other recovery primitives (RecoveryRequest, KeyRevocation, IdentityFreeze) are NOT yet on the consolidated `Content` pattern. The companion Recovery M4 sprint owns finishing that.

### EPR phase audit state (per master plan)

| Phase | Plan checkbox state | Audit status (post-consolidation) |
|---|---|---|
| P1 codec | `elohim/epr/` crate complete | ✅ AUDIT-CONFIRMED post-consolidation |
| P2A storage foundation | `db/epr_atoms.rs` + 6 REST routes + contract tests | ✅ AUDIT-CONFIRMED post-consolidation |
| P2C libp2p federation | `/elohim/epr-atom/1.0.0` + golden vectors + parity tests | ✅ AUDIT-CONFIRMED post-consolidation; `is_integrity_kind()` bypass uses signal-kind strings (consolidation-neutral) |
| P2B identity + projector + signal | 150/155 boxes ✅ | ✅ AUDIT-CONFIRMED post-consolidation; W2A + W2B both landed in code (see W2 table below); only `epr_2b_batch_a_full_loop` `#[ignore]` remains, blocked on W2D `derive_compromise_at` |
| P3 manifest resolver | 83/83 boxes ✅ | ✅ AUDIT-CONFIRMED post-consolidation |
| P3.5 trust-compute gradient | 117/117 boxes ✅ | ✅ AUDIT-CONFIRMED post-consolidation; T22 (record_predecessor) confirmed landed at `p2p/mod.rs:5317–5372` |
| LUG Light Up the Graph | 116/124 boxes ✅ | ✅ AUDIT-CONFIRMED post-consolidation; T18 (= W2A) confirmed landed |
| W1 Phase 4 projector controller | T1–T13 + cleanup commits visible on dev | 🟡 **SUBSTANTIALLY LANDED** — all core deliverables present; two tail items: (1) 3 doc-stale `TODO(Phase 4 follow-up)` comments in `distribution_view.rs:98–99` + `peer_topology_view.rs:49` (code is real; comments lag); (2) federation-aggregate path at `peer_topology_view.rs:71` returns `resilience_cliffs: vec![]` while local path at `:230` is real — wire to `compute_resilience_cliffs`. Note: `dbd0947c1` referenced in earlier draft is not in HEAD history — claim was premature. |

### Documentation drift (audit finding)

The master plan references three sub-plan files that **do not exist** as files: `2026-04-24-epr-phase-2b-plan.md`, `2026-04-30-epr-phase-3-manifest-resolver-plan.md`, `2026-05-01-light-up-the-graph-plan.md`. The kickoff prompts at `genesis/docs/plans/2026-04-24-epr-phase-2b-brainstorm-kickoff-prompt.md` (+ batch-b/c/d) plus the LUG design doc at `genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md` serve as the substantive record. Surface as a docs-cleanup item (rename or backfill plan files) — not a sprint blocker.

### Wave-2 items (still pending — this sprint's primary scope)

| ID | Item | Source pointer | Status (post-audit) |
|---|---|---|---|
| W2A | `record_predecessor` runtime wiring (P3.5 T22 / LUG T18) | `p2p/mod.rs:5317–5372` (live code); `api/epr.rs:189–191` comment confirms wiring landed | ✅ **LANDED** — plan-tracking debt only (`2026-05-11-epr-w2a-record-predecessor-plan.md` boxes need ticking) |
| W2B | IntegrityNotify pipeline — KeyRotation arm | `epr_atom_service.rs:340–384` + `p2p/recovery_rotation.rs`; tests `integrity_notify_keyrotation_acks_received_true` + `integrity_notify_keyrotation_dedup_returns_duplicate_reason` at lines 453, 489 | ✅ **LANDED** — plan-tracking debt only (`2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md` boxes need ticking) |
| W2B-bis | IntegrityNotify pipeline — RevocationAttestation + AgentPeerBinding arms | Producer side comes from M4 §Stage 3; AgentPeerBinding gated on Phase 12 caller-identity (D5) | ⏳ remaining Wave 2 work; RevocationAttestation reads slim duality payload (M4 D2 binding); AgentPeerBinding deferred per D5 if Phase 12 not landed |
| W2C | EPR `GetDocument` request-response variant | `p2p/epr_protocol.rs:46` still stub; `EprService::handle_get_document` returns NotFound at `epr_atom_service.rs:436` | ⏳ **DEFERRED** to graph-native sprint (D4 below) |
| W2D | `key_revocations` projection + `derive_compromise_at` | Migration `2026-04-24-010000_key_revocations` exists; `derive_compromise_at` not yet present in `holochain_app_signal.rs`; `RecoveryFlowProjector` not yet created | ⏳ **The substantive piece** — unblocks `epr_2b_batch_a_full_loop` `#[ignore]` at `holochain/tests/sweettest/src/tests/epr_phase_2b_batch_a_e2e.rs:648` |
| W1-tail | Federation-aggregate `resilience_cliffs` wiring + 3 doc-stale TODO cleanups | `peer_topology_view.rs:71` (federation path stubs `vec![]`); `compute_resilience_cliffs` at `:230` is real; doc comments at `distribution_view.rs:98–99` + `peer_topology_view.rs:49` lag the implementation | ⏳ small surgical Wave 1 tail |

### Wave-3 — a2o @wip lift (revised count from audit)

- `genesis/a2o/features/content/epr-content-addressing.feature` — **9 `@wip`** (lines 27, 39, 52, 65, 79, 93, 106, 118, 130)
- `genesis/a2o/features/federation/epr-cross-peer-resolution.feature` — **8 `@wip`** (lines 70, 86, 100, 116, 128, 142, 154, 167; one earlier @wip already lifted by Wave 0 audit per comment at line 24)
- **Total: 17 `@wip` scenarios** (revised up from 12 in earlier draft)
- Browser-side step defs already exist in `genesis/a2o/steps/ui/epr-content.steps.ts`
- **No stale-route prerequisite**: audit confirmed the feature files contain no direct HTTP route assertions and no step definitions reference attestation routes by URL. The two attestation-narrative references at `epr-cross-peer-resolution.feature:74,103` are Gherkin prose, not invocations.

---

## Sprint scope (waves)

### Wave 0 — Attestation-aware re-audit ✅ COMPLETE

Audit results: `genesis/docs/plans/2026-05-15-epr-wave0-audit-results.md` (audited at HEAD `7a57fb57f`).

Headline findings:
- **0 orphaned tasks** from bespoke attestation entry-type removal. Plan references to `RevocationAttestation` are uniformly to the DNA signal kind (which still exists), not to a removed entry type. The `is_integrity_kind()` bypass is consolidation-neutral.
- **0 stale-route hits** in EPR a2o feature files. Legacy `/api/v1/attestations` (non-unified) routes at `http.rs:8954–8972` are a pre-existing parallel surface (content-attestation table, lamad pillar), not Stage E regressions.
- **Sibling `RecoveryFlowProjector` confirmed** from EPR-side audit (independently arrived at the same verdict as M4 D1).
- **W1 Phase 4 substantially landed**, with two tail items now folded into Wave 1 (federation-aggregate `resilience_cliffs` + 3 doc-stale TODO cleanups).
- **W2A and W2B (KeyRotation) already landed** in code — sub-plan checkbox state is plan-tracking debt only.
- Per-phase status moved from "audit-confirmed pre-consolidation" to "audit-confirmed post-consolidation" (see table above).

### Wave 1 — W2D `key_revocations` projection + W1 federation-aggregate tail

The substantive piece of this sprint. Two strands:

**1a — W2D core**
- [ ] Migration `2026-04-24-010000_key_revocations` already exists; confirm shape matches the W2D requirements; add a follow-up migration if needed.
- [ ] **Create `elohim/elohim-storage/src/services/recovery_flow_projector.rs`** (does not yet exist; co-deliverable with M4 Stage 2). Co-locate the `key_revocations` table writer here (per M4 D1 + EPR audit Slice 3). Project `governance-action:key-revocation` opener + `key-revocation:*` Content entries.
- [ ] Wire the central signal dispatcher's prefix-routing step (location TBD per M4 D1.1 sub-question — likely HTTP/WebSocket signal handler or service orchestrator).
- [ ] `derive_compromise_at` in `holochain_app_signal.rs` reads from `key_revocations` to compute the retroactive sweep window for P2B Stage 2.
- [ ] Lift the `#[ignore]` on `epr_2b_batch_a_full_loop` at `holochain/tests/sweettest/src/tests/epr_phase_2b_batch_a_e2e.rs:648` once both ignore-conditions are met (also requires the Jenkins pack-then-test stage for `imagodei.dna`).
- [ ] schema_contract test pass; ts-rs regen if a view type was added.
- [ ] Tick the boxes on `2026-05-11-epr-w2d-key-revocations-projection-plan.md` once landed.

**1b — W1 federation-aggregate tail**
- [ ] Wire the federation-aggregate path at `peer_topology_view.rs:71` to call `compute_resilience_cliffs` (real implementation present at `:230`).
- [ ] Update the 3 doc-stale `TODO(Phase 4 follow-up)` comments in `distribution_view.rs:98–99` and `peer_topology_view.rs:49` to reflect that the code they describe is real.
- [ ] Confirm `projector_acks` table absence is intentional (audit found `projection_events` table serves the equivalent query surface — functionally equivalent, naming differs from master plan's topology table).

**Plan-tracking debt (clean-up parallel to Wave 1)**
- [ ] Tick the boxes on `2026-05-11-epr-w2a-record-predecessor-plan.md` (audit confirmed W2A landed at `p2p/mod.rs:5317–5372`).
- [ ] Tick the boxes on `2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md` (audit confirmed W2B KeyRotation arm landed at `epr_atom_service.rs:340–384`).

**Acceptance:** P2B `epr_2b_batch_a_full_loop` ticks green; W1 federation-aggregate `resilience_cliffs` returns real values; plan checkboxes match code reality.

### Wave 2 — IntegrityNotify pipeline tail (narrowed; KeyRotation already done)

Audit confirmed the **KeyRotation arm is already wired** at `epr_atom_service.rs:340–384` (with `p2p/recovery_rotation.rs` and acks/dedup tests present). Wave 2 narrows to two remaining handlers:

- **RevocationAttestation handler** — light up per **duality** (M4 D2 binding; EPR D3 below). The handler reads the slim operational payload (`actionHash`, `currentVotes`, `requiredVotes`, `thresholdReached`, `attestedAt`) directly from the existing `revocation-attestation.schema.json` contract. When envelope fields are needed (e.g., threshold configuration for governance validation), read from the local `governance_actions` projection table — already populated by the earlier `governance-action:key-revocation` signal. No new schema; no `contentEnvelope` field; no DHT round-trip per vote.
- **AgentPeerBinding handler** — light up **only after** Phase 12 caller-identity is live on iroh (master plan §Inter-plan dependency graph; D5 below). If Phase 12 is still in flight when this sprint runs, this is a Wave-2 follow-up, not a blocker.

**Acceptance:** Cross-stack integration test (M4 producer + 2B-A IntegrityNotify consumer) round-trips a `RevocationAttestation` signal end-to-end with structurally-valid slim payload. KeyRotation round-trip is already covered by the existing tests at `epr_atom_service.rs:453, 489`.

### Wave 3 — W2A `record_predecessor` ✅ ALREADY LANDED

Audit confirmed: `record_predecessor` is wired at `p2p/mod.rs:5317–5372` (live code, not stub). `aunt_and_rage_bait_integration` continues to pass. Plan checkboxes on `2026-05-11-epr-w2a-record-predecessor-plan.md` need ticking — covered by the plan-tracking debt cleanup in Wave 1 above.

### Wave 4 — W2C `GetDocument` variant ⏳ DEFERRED to graph-native sprint

Per D4 (resolved below): defer. `p2p/epr_protocol.rs:46` stub stands; `EprService::handle_get_document` returns NotFound at `epr_atom_service.rs:436`. Graph-native sprint is the consumer that exercises document-grain EPRs — it owns this implementation.

### Wave 5 — A2o @wip lift (Wave-3 from master plan) — revised count

- [ ] Lift `@wip` on `epr-content-addressing.feature` (**9** scenarios at lines 27, 39, 52, 65, 79, 93, 106, 118, 130).
- [ ] Lift `@wip` on `epr-cross-peer-resolution.feature` (**8** scenarios at lines 70, 86, 100, 116, 128, 142, 154, 167).
- [ ] Confirm browser-side step defs in `genesis/a2o/steps/ui/epr-content.steps.ts` hit the consolidated 8-route surface — audit found no legacy-route invocations to fix.
- [ ] Run genesis pipeline browser stage; confirm 17 scenarios pass.

### Wave 6 — Cross-stack soak (master plan Wave 4)

- [ ] 1-week Phase 2B ↔ Recovery M4 DNA-signal-stream coordination soak with zero cross-branch divergence.
- [ ] orchestrator dev SUCCESS or UNSTABLE-not-regressed for the full week.

---

## Decisions Resolved (2026-05-15)

D1–D4 settled by Wave 0 audit (`genesis/docs/plans/2026-05-15-epr-wave0-audit-results.md`) and M4 brainstorm (`genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md`). D5 remains a runtime check.

1. **D1 — Audit-only (Wave 0 done).** No full re-test needed. The slice re-audit produced 0 orphaned tasks, 0 stale-route hits in EPR a2o files, and confirmed all phase statuses post-consolidation. Sprint scope can rely on the audit results.

2. **D2 — New sibling `key_revocations` table.** Per audit Slice 3 + M4 D1: the writer is co-located in **`RecoveryFlowProjector`** (`elohim/elohim-storage/src/services/recovery_flow_projector.rs`, to be created), not in `AttestationProjector`. Compromise-window derivation is state-machine-shaped and does not compose into the accumulator's branches. The `key_revocations` migration already exists at `migrations/2026-04-24-010000_key_revocations`.

3. **D3 — Duality (binds M4 D2).** Existing `revocation-attestation.schema.json` contract stands. IntegrityNotify reads slim operational payload directly; envelope fields read from local `governance_actions` projection table when needed. No `contentEnvelope` field; no schema changes; no per-vote DHT fetches. Single source of truth resolved by reading the right projection at the right grain, not by inlining envelopes into every signal.

4. **D4 — Defer `GetDocument` to graph-native.** Stub at `epr_protocol.rs:46` stands. Graph-native sprint exercises document-grain EPRs and is the natural consumer/owner of this implementation.

5. **D5 — Phase 12 caller-identity dependency (runtime check, not a brainstorm decision).** AgentPeerBinding handler waits on iroh Phase 12. KeyRotation arm is already landed; RevocationAttestation arm proceeds independently per D3. **Confirm Phase 12 status before Wave 2 dispatch** — if not landed, treat AgentPeerBinding as a Wave-2 follow-up, not a sprint blocker.

---

## Out of scope (handed off to graph-native sprint)

- `experience-story`, `experience-moment`, `:story-point` EPR types (tier 1/2/3 a2o) — `2026-04-18-experience-story-epr-design.md`
- Full social-reach nervous system: provenance back-prop, quarantine, restitution — `project_social_reach_nervous_system` memory
- Standing-aware code paths transitioning from `Standing::Unknown` returns to live gradient signal flow (the Phase 3.5 architectural commitment) — that's downstream gradient-surfacing work
- Per-edge reach/discovery coupling refinements — `project_reach_earned_at_authoring`, `project_first_class_graph_pattern` memories

---

## Acceptance for the sprint as a whole

1. Master plan `2026-05-11-epr-delivery-master.md` has all sub-plan rows showing ✅ AUDIT-CONFIRMED post-consolidation (not just pre-consolidation).
2. orchestrator dev returns SUCCESS or UNSTABLE-not-regressed for 2 consecutive runs with at least one fresh trigger from this sprint's push.
3. elohim-holochain dev passes the full sweettest suite including `epr_phase_2b_batch_a_e2e`'s 5 newly-unblocked tests.
4. `@wip` removed from 12 EPR a2o scenarios; genesis pipeline browser stage shows them green.
5. Cross-stack integration test (M4 producer + EPR W2B consumer) round-trips KeyRotation + RevocationAttestation signals.
6. The graph-native sprint kickoff prompt (future) can reference this sprint's closure and proceed without "we should finish foundation first" being a real blocker.

---

## How to start

Wave 0 audit done; D1–D4 resolved; D5 is a pre-Wave-2 runtime check. Next step: dispatch via `superpowers:writing-plans` to convert the **substantively narrowed** scope into the master-plan sub-plan format (checkbox-per-task). The effective remaining work:

- **Wave 1 (substantive):** W2D `key_revocations` projection + `derive_compromise_at` + co-create `RecoveryFlowProjector` with M4 Stage 2.
- **Wave 1 (tail):** federation-aggregate `resilience_cliffs` wiring + 3 doc-stale TODO cleanups.
- **Wave 1 (debt cleanup):** tick W2A and W2B-KeyRotation plan checkboxes (code already landed).
- **Wave 2 (narrowed):** RevocationAttestation IntegrityNotify arm; AgentPeerBinding arm if Phase 12 caller-identity is live (D5 check).
- **Wave 5:** lift 17 `@wip` scenarios.
- **Wave 6:** 1-week cross-stack soak.

W2C `GetDocument` deferred to graph-native (D4). KeyRotation IntegrityNotify and W2A `record_predecessor` already landed.

This sprint is **the last foundation sprint before graph-native**. Done well, the next prompt the operator writes is the experience-story EPR kickoff and the social-reach nervous-system design.
