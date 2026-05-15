# EPR Foundation Completion (post-attestation) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close out the EPR foundation phases (P1 → P3.5 + Phase 4) by (a) creating the sibling `RecoveryFlowProjector` and upgrading `derive_compromise_at` from a Stage-1 stub to a real projection-lookup, (b) finishing the W1 federation-aggregate tail, (c) ticking the plan-tracking debt for W2A + W2B-KeyRotation (already landed in code), (d) adding the `RevocationAttestation` arm to the IntegrityNotify pipeline (and `AgentPeerBinding` iff Phase 12 caller-identity is live), and (e) lifting `@wip` on 17 EPR a2o scenarios. Leaves the substrate ready for the graph-native sprint.

**Architecture:** This is a coordinator plan that drives five narrow Rust/storage changes plus an a2o coverage lift, in three roughly-sequential bands. **Band A (Wave 1 substantive + tail)** lands the `RecoveryFlowProjector` sibling and the federation-aggregate `resilience_cliffs` wiring — this is the only band with new code. **Band B (Wave 2 narrowed)** adds the `RevocationAttestation` IntegrityNotify arm (mirroring the live `KeyRevocation` arm at `epr_atom_service.rs:291–339`) and, conditionally, the `AgentPeerBinding` arm if iroh master Phase 12 has landed (D5). **Band C (Waves 5–6)** lifts `@wip` on 9 + 8 = 17 EPR a2o scenarios (Opus-authored per `feedback_a2o_narrative_is_opus_work`) and kicks off the 1-week cross-stack soak. Plan-tracking debt (ticking the W2A and W2B-KeyRotation checkboxes that already landed in code) is folded into Band A as small parallel doc-edit tasks.

**Tech Stack:** Rust (elohim-storage), Diesel + SQLite (key_revocations + revocation_votes projection tables, both migrations already on dev), libp2p 0.54 (IntegrityNotify request-response codec), Holochain HDK 0.5 (DNA signal stream), MessagePack via rmp_serde, Gherkin / Cucumber (a2o features), Jenkins (cross-stack soak).

**Spec / parent master:** `genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md` (master) + `genesis/docs/plans/2026-05-15-epr-foundation-completion-post-attestation-kickoff-prompt.md` (kickoff) + `genesis/docs/plans/2026-05-15-epr-wave0-audit-results.md` (audit) + `genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md` (D1 = sibling RecoveryFlowProjector, D2 = duality binds EPR D3).

**Wave 0 (audit) is complete.** This plan is the post-audit substantively-narrowed scope. Decisions D1–D4 resolved; D5 is a runtime check before Band B.

---

## P2P Design Gate Output

| Entity | Category | Source of truth | Justification |
|---|---|---|---|
| `key_revocations` SQLite projection (existing migration `2026-04-24-010000_key_revocations`) | **C** — operational projection | DHT-notarized imagodei KeyRevocation entries; rebuildable via signal replay on `RecoveryV2Signal::KeyRevocationRequested` / `KeyRevocationEffective` | Migration shape already correct (12 columns, 3 indexes). This plan only moves the writer to a live signal-dispatch path and adds the projection-lookup join. No schema change. |
| `revocation_votes` SQLite projection (existing migration `2026-04-24-020000_revocation_votes`) | **C** — operational projection | DHT-notarized RevocationVote entries | Already wired by legacy code in `signals.rs::handle_recovery_v2_signal`; this plan re-uses the writer through the sibling projector. |
| `RecoveryFlowProjector::handle_content_signal` (new module) | helper function — no new entity | Pure dispatch + state-machine controller over the two existing projection tables | Sibling per M4 D1; co-located writer per audit Slice 3 + this kickoff prompt. Mirrors `AttestationProjector::handle_content_signal` module shape but for recovery-flow lifecycle (Open → Quorum → Effective). |
| `derive_compromise_at` upgrade in `holochain_app_signal.rs:289` | helper function | Pure read over `key_revocations` table | Function already exists as a Stage-1 stub that returns `effective_at_fallback` (`holochain_app_signal.rs:289–296`). Stage 2 replaces the body with a projection lookup. Same signature; no caller changes. |
| `RevocationAttestation` IntegrityNotify arm in `epr_atom_service.rs:handle_integrity_notify` | helper invocation | Pure function call site | Mirrors `KeyRevocation` arm at `epr_atom_service.rs:291`. No new wire contract — uses existing `revocation-attestation.schema.json` (D2 duality binding). |
| `AgentPeerBinding` IntegrityNotify arm (conditional on D5) | helper invocation | Pure function call site | Same shape; gated on Phase 12 caller identity landing on iroh master. |
| Federation-aggregate `resilience_cliffs` wiring at `peer_topology_view.rs:71` | callsite swap | Existing `compute_resilience_cliffs` at `peer_topology_view.rs:230` | Local resolver path already calls real implementation at `peer_topology_view.rs:185–189`. Federation-aggregate path returns `vec![]`. This is a one-line callsite swap. |
| `@wip` lift on 17 a2o scenarios | tag edit on existing scenarios | n/a | Step defs already in `genesis/a2o/steps/ui/epr-content.steps.ts`; audit confirmed zero legacy-route hits. Opus authors per master plan §"Dispatch shape recommendation". |

**Anti-pattern check:** ✓ No new DHT entry types (zero DNA capacity impact: lamad ~73/~100, mishpat 11/~100 unchanged). ✓ No new SQLite tables (`key_revocations` + `revocation_votes` migrations already on dev). ✓ No new HTTP routes. ✓ No new wire contracts. ✓ No CID-as-FK. ✓ Source-of-truth declarations preserved at module level. ✓ AttestationProjector left unmodified per M4 D1 (the sibling pattern keeps accumulator and controller cleanly separated).

---

## File Structure

### New files
| Path | Responsibility |
|------|----------------|
| `elohim/elohim-storage/src/services/recovery_flow_projector.rs` | Sibling to `attestation_projector.rs`. `handle_content_signal` routes `governance-action:key-revocation` opener + `key-revocation:*` Content entries into the existing `key_revocations` + `revocation_votes` projections. State-machine controller for recovery-flow lifecycle (Open → Quorum → Effective). Co-deliverable with Recovery M4 Stage 2 per M4 brainstorm D1. |

### Modified files (Rust storage)
| Path | What changes |
|------|--------------|
| `elohim/elohim-storage/src/services/mod.rs` | Add `pub mod recovery_flow_projector;` re-export. |
| `elohim/elohim-storage/src/reconcile/holochain_app_signal.rs:289–296` | Replace the Stage-1 stub body of `derive_compromise_at` with a projection-lookup that joins `key_revocations` + `revocation_votes` to compute the retroactive sweep window. Function signature unchanged. |
| `elohim/elohim-storage/src/reconcile/holochain_app_signal.rs` (consumer at `:177–202`) | Confirm `derive_compromise_at` is still called with `(db_pool, revocation_id, effective_at_dt)` — should require no caller change since the signature stays identical. |
| `elohim/elohim-storage/src/epr_atom_service.rs:419` (after the `KeyRotation` arm closes, before the `other_kind` catch-all) | Insert `"RevocationAttestation"` match arm reading the slim payload per `revocation-attestation.schema.json` (duality per D3). |
| `elohim/elohim-storage/src/epr_atom_service.rs:419` (conditional) | Insert `"AgentPeerBinding"` arm IFF Phase 12 caller identity is live (D5 runtime check). |
| `elohim/elohim-storage/src/services/peer_topology_view.rs:71` | Replace `resilience_cliffs: vec![]` with a call to `compute_resilience_cliffs(&mut conn, agent_cid)` (real implementation at `:230`). |
| `elohim/elohim-storage/src/services/peer_topology_view.rs:49` | Update the doc-stale `/// 6. resilience_cliffs stubbed vec![] — TODO Phase 4 follow-up.` comment to reflect the federation path now calls the real implementation. |
| `elohim/elohim-storage/src/services/distribution_view.rs:98` | Update doc-stale `/// - projector_count: stubbed 0 (TODO Phase 4 follow-up — no projector table yet)` — body at `:159–166` queries `projection_events::distinct_projectors_for_blob`, which is real. Replace the doc with the real description. |
| `elohim/elohim-storage/src/services/distribution_view.rs:99` | Update doc-stale `/// - diversity_hint: stubbed None (TODO Phase 4 follow-up — no geo/archetype index yet)` — body at `:165–178` queries `peer_identity_bindings.device_archetype` and calls `peer_diversity::diversity_hint_from_archetype_strs`. |

### Modified files (plan-tracking debt — Markdown only, no code)
| Path | What changes |
|------|--------------|
| `genesis/docs/superpowers/plans/2026-05-11-epr-w2a-record-predecessor-plan.md` | Tick every `[ ]` to `[x]`; live code lives at `p2p/mod.rs:5317–5372`; audit confirmed landed. |
| `genesis/docs/superpowers/plans/2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md` | Tick every `[ ]` to `[x]`; live code lives at `epr_atom_service.rs:340–419`; tests at `:453, :489`; audit confirmed landed. |

### Modified files (a2o coverage lift)
| Path | What changes |
|------|--------------|
| `genesis/a2o/features/content/epr-content-addressing.feature` (9 `@wip` at lines 27, 39, 52, 65, 79, 93, 106, 118, 130) | Strip `@wip` tag IFF the scenario's step defs in `genesis/a2o/steps/ui/epr-content.steps.ts` are real and the scenario reads true against the production substrate (Opus-authored review per `feedback_a2o_narrative_is_opus_work`). |
| `genesis/a2o/features/federation/epr-cross-peer-resolution.feature` (8 `@wip` at lines 70, 86, 100, 116, 128, 142, 154, 167) | Same Opus-authored review pattern. |

### Test files
| Path | Responsibility |
|------|----------------|
| `elohim/elohim-storage/src/services/recovery_flow_projector.rs::tests` | Inline `#[cfg(test)] mod tests` mirroring `attestation_projector.rs:216–331`. Three fixtures: open (governance-action:key-revocation opener), in-progress (vote arrival), effective (KeyRevocationEffective). |
| `elohim/elohim-storage/src/reconcile/holochain_app_signal.rs::tests` | Extend the existing test module with `derive_compromise_at_returns_earliest_approving_vote_timestamp` (Stage-2 projection-lookup behaviour) and `derive_compromise_at_falls_back_when_projection_missing` (graceful fallback). |
| `elohim/elohim-storage/src/epr_atom_service.rs::tests` | Add `integrity_notify_revocation_attestation_acks_received_true` + `integrity_notify_revocation_attestation_dedup_returns_duplicate_reason`, mirroring `integrity_notify_keyrotation_acks_received_true` / `_dedup_returns_duplicate_reason` at `:453, :489`. |
| `elohim/elohim-storage/src/epr_atom_service.rs::tests` (conditional) | Same shape for `integrity_notify_agent_peer_binding_*` IFF the AgentPeerBinding arm lands. |
| `elohim/elohim-storage/tests/recovery_flow_projector_e2e.rs` | E2E test: synthesise a `governance-action:key-revocation` opener signal, drive through the projector, assert a row appears in `key_revocations`. Then drive an `attestation:revocation-vote` child via `AttestationProjector` (unchanged path), assert vote count increments. Then drive a `KeyRevocationEffective` signal, assert `threshold_reached = 1` and `effective_at` set. |

### Out-of-scope (handed off to graph-native sprint per kickoff D4)
- `EprAtomRequest::GetDocument` variant (`p2p/epr_protocol.rs:46` stub remains; `EprService::handle_get_document` returns `NotFound` at `epr_atom_service.rs:436`).
- `experience-story` / `experience-moment` / `:story-point` EPR types.
- Full social-reach nervous system (provenance back-prop, quarantine, restitution).

---

## Sequencing (band order)

**REVISED 2026-05-15 — M4 coordination correction.** The Recovery M4 sprint is running in parallel (operator at `/projects/elohim` on `dev`; EPR in this worktree at `worktree-epr-foundation-completion`; merge target = dev). M4 owns the foundation:

- **M4 T6:** `key_revocations` migration — 15 columns including `derived_compromise_at`. EPR T5 + T6 consume this schema; do NOT redefine.
- **M4 T7:** Diesel models + CRUD for `recovery_flows` + `key_revocations`. EPR uses the M4 writers as-is.
- **M4 T8:** `RecoveryFlowProjector` skeleton (Open-state branch) at `elohim/elohim-storage/src/services/recovery_flow_projector.rs`. EPR T5's contribution is the `key-revocation:effective` arm + `key_revocations` writer added as branches **INSIDE M4's `handle_content_signal` dispatcher** — co-located (per M4 brainstorm D1), NOT in a sibling module.
- **M4 T9:** State-machine branches. Fully blocks EPR T5 until landed (~2h after T8 starts).
- **M4 T10:** Central dispatcher `elohim_content_dispatcher::dispatch` — where prefix-routing lives. EPR T7 (signal consumer) is **upstream** of this dispatcher; EPR T5 (writer arm) is **inside** the projector M4 builds. Keep that boundary.

**Revised execution order:**

- **Band A — debt cleanup + W1 tail** (Tasks 1–4): doc-only + federation-aggregate `resilience_cliffs`. ✅ all four landed.
- **Band B — independent of M4** (Tasks 7–10): pivot here while M4 T6–T9 land. T7 = `RevocationAttestation` IntegrityNotify arm (D2 slim payload, no envelope inlining — M4 T17 will fail-fast if any DNA emission tries to inline `contentEnvelope`). T8 = D5 Phase 12 check + conditional `AgentPeerBinding`. T9 + T10 = Opus-authored a2o `@wip` lift.
- **Band C — blocked on M4** (Tasks 5 + 6): resumes after M4 T6–T9 land (ETA ~4–5h from M4 kickoff). T5 becomes "add `key-revocation:effective` arm + `key_revocations` writer to M4's projector" (additive, not creation). T6 may reduce to "populate `derived_compromise_at` column" if M4 owns the computation.
- **Band D — sprint close** (Task 11): operator-driven cross-stack soak.

The `epr_2b_batch_a_full_loop` `#[ignore]` at `epr_phase_2b_batch_a_e2e.rs:648` lifts when both conditions clear: M4 T9 lands (compromise-window state machine) + Jenkins `imagodei.dna` pack-then-test stage (out of scope for this plan — operator coordinates with the holochain pipeline).

### Architectural notes (from operator + M4 coordination)

- **DNA role naming (saved in main checkout at `.claude/memory/project_elohim_dna_as_sdk_boundary.md`):** elohim DNA is the SDK contract; lamad is one implementation. EPR code calling into the elohim coordinator should target role `"elohim"` (SDK-correct) not `"lamad"` (implementation name). Production `happ.yaml` currently misdeclares this — backlog item `ce89d2e7e` (`genesis/data/timeline/backlog/cross-dna-role-name-and-contract-enforcement.md`). Don't fix it in this sprint; just stay consistent.
- **Devspace quirks for SweetTest local runs:** use `env -u RUSTFLAGS BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/clang/20/include" cargo test ...` for native builds (the `RUSTFLAGS=--cfg getrandom_backend="custom"` flag is WASM-only and breaks native).
- **SweetTests with `#[ignore = "requires packed DNAs from Jenkins pipeline"]`** only run in CI; do not attempt locally.
- **Disk pressure** is the dominant blocker — reclaim with `cargo-pool prune family <name>` before long Rust builds.

---

## Task 1: Plan-tracking debt — tick W2A boxes

**Files:**
- Modify: `genesis/docs/superpowers/plans/2026-05-11-epr-w2a-record-predecessor-plan.md`

Audit confirmed W2A landed at `elohim/elohim-storage/src/p2p/mod.rs:5317–5372`; comment at `api/epr.rs:189–191` confirms the wiring. The plan's `[ ]` checkboxes need to be ticked.

- [ ] **Step 1: Verify the W2A code is in place**

Run:
```
grep -n "record_predecessor" /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs
grep -n "record_predecessor" /projects/elohim/elohim/elohim-storage/src/api/epr.rs
grep -n "record_predecessor" /projects/elohim/elohim/elohim-storage/src/epr_atom_service.rs
```
Expected: live `record_predecessor` call inside `handle_epr_atom_request` (around line 5317–5372 in `p2p/mod.rs`); explanatory comment block in `api/epr.rs` around line 189–191; explanatory comment in `epr_atom_service.rs` around line 189. If any of these do not match the audit's findings, STOP and report BLOCKED — the audit may be stale.

- [ ] **Step 2: Tick every `[ ]` to `[x]` in the W2A plan**

In `genesis/docs/superpowers/plans/2026-05-11-epr-w2a-record-predecessor-plan.md`, walk every checkbox-bearing line. For each `- [ ] **Step N:` heading, replace the empty checkbox with a filled one: `- [x] **Step N:`. Do NOT modify any other content (code blocks, prose, file paths).

The plan has a single task ("Task T22.1") with 8 steps — all 8 are landed per the audit (W2A code is real, comments are updated). Tick all 8 to `[x]`.

- [ ] **Step 3: Append a "Status" line at the top of the plan, right after the goal**

After the `**Goal:**` line, insert a blank line and:
```markdown
**Status:** ✅ LANDED in dev. Live code at `elohim/elohim-storage/src/p2p/mod.rs:5317–5372`; comment block updated at `api/epr.rs:189–191`; confirmed by Wave 0 audit on 2026-05-15 (`genesis/docs/plans/2026-05-15-epr-wave0-audit-results.md` §D6). Checkboxes ticked as plan-tracking debt cleanup on the same date.
```

- [ ] **Step 4: Commit**

```bash
git add genesis/docs/superpowers/plans/2026-05-11-epr-w2a-record-predecessor-plan.md
git commit -m "$(cat <<'EOF'
docs(epr): tick W2A plan-tracking debt — record_predecessor landed

W2A (T18 LUG / T22 P3.5) record_predecessor wiring is live in
p2p/mod.rs:5317–5372 with comment updates at api/epr.rs:189–191
and epr_atom_service.rs:189. Wave 0 audit confirmed (2026-05-15
results §D6); the plan's checkboxes were never ticked. This commit
ticks all 8 steps in Task T22.1 and adds a Status line marking the
plan ✅ LANDED.

No code change.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Plan-tracking debt — tick W2B-KeyRotation boxes

**Files:**
- Modify: `genesis/docs/superpowers/plans/2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md`

Audit confirmed W2B-KeyRotation landed at `elohim/elohim-storage/src/epr_atom_service.rs:340–419` with `p2p/recovery_rotation.rs` present and acks/dedup tests at `epr_atom_service.rs:453, :489`. The plan's `[ ]` checkboxes need to be ticked.

- [ ] **Step 1: Verify the W2B-KeyRotation code is in place**

Run:
```
grep -n "\"KeyRotation\"" /projects/elohim/elohim/elohim-storage/src/epr_atom_service.rs
ls /projects/elohim/elohim/elohim-storage/src/p2p/recovery_rotation.rs
grep -n "integrity_notify_keyrotation_acks_received_true\|integrity_notify_keyrotation_dedup_returns_duplicate_reason" /projects/elohim/elohim/elohim-storage/src/epr_atom_service.rs
```
Expected: `"KeyRotation"` arm visible at `epr_atom_service.rs:340`; `recovery_rotation.rs` exists; both regression tests present at lines 453 and 489.

- [ ] **Step 2: Tick every `[ ]` to `[x]` in the W2B plan**

In `genesis/docs/superpowers/plans/2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md`, walk every checkbox-bearing line. Replace each `- [ ] **Step N:` with `- [x] **Step N:`. Do NOT modify code blocks or prose.

The plan has a single task ("Task W2B.1") with 10 steps. Tick all 10.

- [ ] **Step 3: Append a "Status" line at the top of the plan, right after the goal**

After the `**Goal:**` line, insert a blank line and:
```markdown
**Status:** ✅ LANDED in dev. Live code at `elohim/elohim-storage/src/epr_atom_service.rs:340–419` with `p2p/recovery_rotation.rs` present; regression tests at `epr_atom_service.rs:453, :489`. Confirmed by Wave 0 audit on 2026-05-15 (`genesis/docs/plans/2026-05-15-epr-wave0-audit-results.md` §D3). RevocationAttestation + AgentPeerBinding arms are the remaining W2B scope — covered by `2026-05-15-epr-foundation-completion.md` Band B.
```

- [ ] **Step 4: Commit**

```bash
git add genesis/docs/superpowers/plans/2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md
git commit -m "$(cat <<'EOF'
docs(epr): tick W2B-KeyRotation plan-tracking debt — handler landed

W2B KeyRotation arm of IntegrityNotify is live in
epr_atom_service.rs:340–419 with regression tests at :453 and :489;
recovery_rotation.rs module exists with round-trip coverage. Wave 0
audit confirmed (2026-05-15 results §D3); the plan's checkboxes
were never ticked. This commit ticks all 10 steps in Task W2B.1
and adds a Status line marking the KeyRotation arm ✅ LANDED.

RevocationAttestation + AgentPeerBinding arms remain as scope under
the foundation-completion plan Band B.

No code change.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: W1 federation-aggregate `resilience_cliffs` wiring

**Files:**
- Modify: `elohim/elohim-storage/src/services/peer_topology_view.rs:71` (federation-aggregate path)
- Modify: `elohim/elohim-storage/src/services/peer_topology_view.rs:49` (doc comment)

Wave 0 audit confirmed the local-resolver path computes real `resilience_cliffs` via `compute_resilience_cliffs(&mut conn, agent_cid)` at `peer_topology_view.rs:185–189` (real `fn` defined at `:230`). The federation-aggregate path at `:71` returns `resilience_cliffs: vec![]` as a stub. This task swaps the stub for the real call.

- [ ] **Step 1: Read the federation-aggregate path and confirm shape**

```
sed -n '40,85p' /projects/elohim/elohim/elohim-storage/src/services/peer_topology_view.rs
sed -n '180,200p' /projects/elohim/elohim/elohim-storage/src/services/peer_topology_view.rs
sed -n '225,275p' /projects/elohim/elohim/elohim-storage/src/services/peer_topology_view.rs
```

Confirm:
- The federation-aggregate construct at `:71` builds a `PeerTopologyView` (or analogous struct) field-by-field, with `resilience_cliffs: vec![]` literal.
- The local-resolver path at `:185–189` calls `compute_resilience_cliffs(&mut conn, agent_cid)` where `&mut conn` is a `SqliteConnection` and `agent_cid` is the subject the topology is being computed for.
- The federation-aggregate path has access to the same `conn` + `agent_cid` (or equivalents). If not, STOP and report BLOCKED — the federation path may need a separate threading change that is out of scope here.

- [ ] **Step 2: Write a failing test for the federation-aggregate path**

Add to `elohim/elohim-storage/src/services/peer_topology_view.rs::tests` (or the matching test module — search for the existing `#[cfg(test)] mod tests` in the same file):

```rust
#[test]
fn federation_aggregate_resilience_cliffs_returns_real_values_not_empty() {
    use crate::db::test_helpers::fresh_pool_with_topology_fixture;
    let (pool, agent_cid) = fresh_pool_with_topology_fixture();
    let mut conn = pool.get().expect("conn");

    // Build the federation-aggregate view using the same path
    // exercised at line 71 of this file. Replace `aggregate_peer_topology_view`
    // with whatever the federation-aggregate entry-point function is named.
    let view = aggregate_peer_topology_view(&mut conn, &agent_cid)
        .expect("aggregate view");

    // The fixture has at least one sole-replica content item, so the cliffs
    // vec must be non-empty. If the fixture does not exercise resilience cliffs,
    // adapt to whichever fixture in test_helpers seeds quilt distribution data.
    assert!(
        !view.resilience_cliffs.is_empty(),
        "federation-aggregate resilience_cliffs should be computed, not stub"
    );
}
```

Adapt the import + entry-point function name to the actual symbols in the file. If `fresh_pool_with_topology_fixture` does not exist, find the closest fixture builder used in this file's existing `tests` module and reuse its pattern. The goal is: aggregate-path is called, returns non-empty cliffs.

- [ ] **Step 3: Run test → FAIL**

```
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --lib services::peer_topology_view::tests::federation_aggregate_resilience_cliffs_returns_real_values_not_empty 2>&1 | tail -30
```

Expected: FAIL — `assert!(!view.resilience_cliffs.is_empty())` panics because the federation-aggregate returns `vec![]`.

- [ ] **Step 4: Swap the stub for the real call at `:71`**

In `peer_topology_view.rs`, locate the literal `resilience_cliffs: vec![]` at approximately line 71. Replace it with a call mirroring the local-resolver pattern at `:185–189`:

```rust
resilience_cliffs: compute_resilience_cliffs(&mut conn, agent_cid).unwrap_or_default(),
```

Adapt to the federation-aggregate path's actual local variable names. If the federation path threads `conn` and `agent_cid` differently (e.g., as `&conn` or `&str`), match the signature of `compute_resilience_cliffs` at `:230` — that signature is the source of truth.

**STRICT FORBID:** do not change the signature of `compute_resilience_cliffs`, do not introduce a new helper function, do not change the field type of `resilience_cliffs` on `PeerTopologyView`. This is a one-line callsite swap.

- [ ] **Step 5: Update the doc-stale comment at `:49`**

Find the line:
```rust
/// 6. `resilience_cliffs` stubbed `vec![]` — TODO Phase 4 follow-up.
```

Replace with:
```rust
/// 6. `resilience_cliffs` is computed via `compute_resilience_cliffs` (`:230`)
///    on both the local-resolver path (`:185–189`) and the federation-aggregate
///    path (`:71`). Returns sole-replica resilience cliffs derived from the
///    quilt distribution projection.
```

- [ ] **Step 6: Run test → PASS**

Same command as Step 3. Expected: PASS.

- [ ] **Step 7: Run clippy + fmt**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo clippy --lib -- -D warnings 2>&1 | tail -20
cargo fmt --check
```

Both must pass.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/services/peer_topology_view.rs
git commit -m "$(cat <<'EOF'
fix(storage): W1 tail — wire federation-aggregate resilience_cliffs

Federation-aggregate path at peer_topology_view.rs:71 was returning
resilience_cliffs: vec![] while the local-resolver path at :185–189
called compute_resilience_cliffs (real implementation at :230).
Audit Slice 4 surfaced this as the substantive W1 tail.

Federation aggregator now mirrors the local-resolver call shape:
compute_resilience_cliffs(&mut conn, agent_cid).unwrap_or_default().
Same function, same defaults — no signature change.

Doc-stale comment at :49 updated to reflect the federation path now
computes real cliffs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: W1 doc-stale Phase 4 TODO cleanup in `distribution_view.rs`

**Files:**
- Modify: `elohim/elohim-storage/src/services/distribution_view.rs:98–99`

Wave 0 audit Slice 4 found two doc-stale TODO comments at `:98` (`projector_count`) and `:99` (`diversity_hint`). The code they describe is real:
- `projector_count` body at `:159–166` queries `projection_events::distinct_projectors_for_blob` (real table; migration `2026-05-11-110000_projection_events` exists).
- `diversity_hint` body at `:165–178` queries `peer_identity_bindings.device_archetype` and calls `peer_diversity::diversity_hint_from_archetype_strs` (real composer).

This task aligns the doc comments with the live code. No behavioural change.

- [ ] **Step 1: Read the doc-stale comments in context**

```
sed -n '90,110p' /projects/elohim/elohim/elohim-storage/src/services/distribution_view.rs
sed -n '155,180p' /projects/elohim/elohim/elohim-storage/src/services/distribution_view.rs
```

Confirm:
- Lines 98–99 are comment lines describing `projector_count` and `diversity_hint` as stubs.
- Lines 159–178 contain the real queries that populate those fields.

- [ ] **Step 2: Update the doc comment at `:98`**

Replace the existing comment with text that matches the real implementation. The existing comment is:
```rust
/// - `projector_count`: stubbed 0 (TODO Phase 4 follow-up — no projector table yet)
```

Replace with:
```rust
/// - `projector_count`: count of distinct doorway projectors that have acked
///   this blob, computed via `projection_events::distinct_projectors_for_blob`
///   (see body at `:159–166`).
```

- [ ] **Step 3: Update the doc comment at `:99`**

The existing comment is:
```rust
/// - `diversity_hint`: stubbed None (TODO Phase 4 follow-up — no geo/archetype index yet)
```

Replace with:
```rust
/// - `diversity_hint`: derived from `peer_identity_bindings.device_archetype`
///   via `peer_diversity::diversity_hint_from_archetype_strs` (see body at
///   `:165–178`). Returns `None` when no archetype-tagged peers are known.
```

- [ ] **Step 4: Run clippy + fmt + grep for remaining `TODO(Phase 4 follow-up)` markers**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo clippy --lib -- -D warnings 2>&1 | tail -10
cargo fmt --check
rg "TODO\(Phase 4 follow-up\)" /projects/elohim/elohim /projects/elohim/doorway /projects/elohim/steward
```

Expected:
- clippy + fmt clean
- `rg` returns zero hits across elohim/ doorway/ steward/ (Task 3 already updated `peer_topology_view.rs:49`; this task closes the remaining two).

If `rg` returns any hits, investigate — there may be additional doc-stale markers the audit missed.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/distribution_view.rs
git commit -m "$(cat <<'EOF'
docs(storage): W1 tail — align distribution_view doc comments with real code

distribution_view.rs:98–99 doc comments said projector_count and
diversity_hint were Phase 4 follow-up stubs, but the bodies at
:159–166 (projection_events::distinct_projectors_for_blob) and
:165–178 (peer_diversity::diversity_hint_from_archetype_strs) are
real implementations. Audit Slice 4 surfaced these as doc-stale.

Updated comments to describe the live queries.

No behaviour change. After this commit + Task 3, the
"TODO(Phase 4 follow-up)" marker is gone from the entire repo.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: W2D — create `RecoveryFlowProjector` (sibling) + wire central signal dispatcher

> **STATUS — 2026-05-15: ✅ LANDED (reframed under M4 coordination).** Commit `0104e5e48`.
>
> Task 5 was substantively reframed mid-sprint when the Recovery M4 sprint claimed ownership of the `RecoveryFlowProjector` itself (M4 commits `d89abd019`, `1d34a9153`, `c2b70133c`, `596c7b8c9`) — sibling-projector pattern co-located per M4 D1. The EPR-side contribution that landed instead is the **A.8 EPR-atom revocation sweep** (`signals.rs::sweep_dependent_caches_on_revocation`), which fills the Phase-2B extension point M4 explicitly left for the EPR worker at `signals.rs:1249` (stub with comment *"Phase 2B extension point: UPDATE epr_atoms SET verified_at = NULL WHERE signer_cid = ?revoked_key"*). The sweep is time-bounded by `compromise_at`, idempotent, called from both the legacy `RecoveryV2Signal::KeyRevocationEffective` path (signals.rs:1220) and the new T18 `DnaSignal::KeyRevocation` envelope path (signals.rs:1459). Migration `2026-04-25-000000_verified_at_on_epr_atoms` pre-staged the `(signer_cid, issued_at)` index for exactly this sweep. Four tests cover the four required semantics: clears matching, leaves pre-compromise alone, idempotent, zero-on-empty.
>
> The original task body below is retained for historical context. Steps 1–N are NOT to be executed — M4 owns that work and it has already landed on dev.

**Files:**
- Create: `elohim/elohim-storage/src/services/recovery_flow_projector.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (re-export)
- Create: `elohim/elohim-storage/tests/recovery_flow_projector_e2e.rs`
- Coordinate with: M4 Stage 2 (Recovery sprint owns the producer side of `governance-action:key-revocation` signal emission per `2026-05-15-recovery-m4-completion-shamir-optional-kickoff-prompt.md`).

This is the substantive piece of the sprint. Per M4 brainstorm D1 + EPR audit Slice 3: the `key_revocations` table writer lives in a sibling module to `AttestationProjector`, not as a fourth branch in `AttestationProjector::handle_content_signal`. The writer for the projection tables already exists (`db/key_revocations::upsert_key_revocation`, `set_key_revocation_effective`, `update_current_votes`; `db/revocation_votes::insert_revocation_vote`) — the legacy invocation lives at `signals.rs::handle_recovery_v2_signal:1040–1140`, on a now-dead path. This task moves the dispatch into a live signal-routing path through the new sibling projector.

- [ ] **Step 1: Read the templates + dead writer**

```
sed -n '1,80p' /projects/elohim/elohim/elohim-storage/src/services/attestation_projector.rs
sed -n '1000,1145p' /projects/elohim/elohim/elohim-storage/src/signals.rs
sed -n '1,80p' /projects/elohim/elohim/elohim-storage/src/db/key_revocations.rs
```

Confirm:
- `AttestationProjector::handle_content_signal` signature: `pub fn handle_content_signal(conn: &mut SqliteConnection, signal: &ElohimContentSignal) -> Result<(), StorageError>`. Mirror this exactly.
- `signals.rs::handle_recovery_v2_signal` at lines 1040–1140 contains the existing writer logic for `RecoveryV2Signal::KeyRevocationRequested`, `RevocationVoteSubmitted`, and `KeyRevocationEffective` — three blocks that this task ports into the sibling projector.
- `db::key_revocations::upsert_key_revocation`, `update_current_votes`, and `set_key_revocation_effective` are the writer functions to call. Their signatures must not change.

- [ ] **Step 2: Identify the central signal dispatch site**

Per M4 D1 sub-question, the central signal dispatcher's location is "to be confirmed." Grep:

```
grep -rn "attestation_projector::handle_content_signal" /projects/elohim/elohim/elohim-storage/src/
grep -rn "ElohimContentSignal" /projects/elohim/elohim/elohim-storage/src/
grep -rn "content_type.starts_with(" /projects/elohim/elohim/elohim-storage/src/
```

Expected outcomes:
- If a live caller of `attestation_projector::handle_content_signal` is found (outside tests): that file is the central dispatcher. Add the prefix-routing step there (route `governance-action:key-revocation`, `key-revocation:*` to `recovery_flow_projector::handle_content_signal`; everything else continues to `attestation_projector`).
- If no live caller is found: the dispatcher is itself unwired. STOP and report this as a discovery — the master plan's W1 Phase 4 acceptance assumed a live caller; this is an additional wiring task. Document it in the plan as a follow-up Step 2.5 before proceeding.

Document the discovered dispatcher path + line in a comment block in the new `recovery_flow_projector.rs` module so future readers can trace.

- [ ] **Step 3: Write a failing E2E test for the projector**

Create `elohim/elohim-storage/tests/recovery_flow_projector_e2e.rs`:

```rust
//! W2D — verify RecoveryFlowProjector projects governance-action:key-revocation
//! opener + key-revocation:* effective signals into key_revocations table.

use elohim_storage::services::recovery_flow_projector;
use elohim_storage::signals::ElohimContentSignal;
use elohim_storage::db::test_helpers::fresh_pool;
// Adapt imports to the actual test_helpers API — verify before writing.

fn make_signal(content_type: &str, id: &str, body: serde_json::Value) -> ElohimContentSignal {
    ElohimContentSignal {
        id: id.to_string(),
        content_type: content_type.to_string(),
        body_json: serde_json::to_string(&body).expect("body json"),
        // Adapt to actual ElohimContentSignal field names (verify shape at signals.rs:1851).
        ..Default::default()
    }
}

#[test]
fn governance_action_key_revocation_opener_inserts_row() {
    let pool = fresh_pool();
    let mut conn = pool.get().expect("conn");

    let signal = make_signal(
        "governance-action:key-revocation",
        "u-revocation-1",
        serde_json::json!({
            "id": "u-revocation-1",
            "humanId": "human-matthew",
            "revokedKey": "base64-key-A",
            "reason": "compromise",
            "triggerType": "self",
            "initiatedBy": "agent-matthew",
            "requiredVotes": 3,
            "currentVotes": 1,
            "thresholdReached": false,
            "effectiveAt": null,
            "createdAt": "2026-05-15T12:00:00Z"
        }),
    );

    recovery_flow_projector::handle_content_signal(&mut conn, &signal)
        .expect("projector handles opener");

    // Assert row written to key_revocations.
    let row = elohim_storage::db::key_revocations::find_by_id(&mut conn, "u-revocation-1")
        .expect("find by id")
        .expect("row present");
    assert_eq!(row.human_id, "human-matthew");
    assert_eq!(row.required_votes, 3);
    assert_eq!(row.current_votes, 1);
    assert_eq!(row.threshold_reached, 0);
    assert!(row.effective_at.is_none());
}

#[test]
fn key_revocation_effective_signal_marks_row_effective() {
    let pool = fresh_pool();
    let mut conn = pool.get().expect("conn");

    // Pre-condition: opener row exists.
    recovery_flow_projector::handle_content_signal(
        &mut conn,
        &make_signal(
            "governance-action:key-revocation",
            "u-revocation-2",
            serde_json::json!({
                "id": "u-revocation-2",
                "humanId": "human-matthew",
                "revokedKey": "base64-key-B",
                "reason": "compromise",
                "triggerType": "self",
                "initiatedBy": "agent-matthew",
                "requiredVotes": 3,
                "currentVotes": 1,
                "thresholdReached": false,
                "effectiveAt": null,
                "createdAt": "2026-05-15T12:00:00Z"
            }),
        ),
    ).expect("opener");

    // Effective signal.
    recovery_flow_projector::handle_content_signal(
        &mut conn,
        &make_signal(
            "key-revocation:effective",
            "u-revocation-2-effective",
            serde_json::json!({
                "revocationId": "u-revocation-2",
                "humanId": "human-matthew",
                "revokedKey": "base64-key-B",
                "effectiveAt": "2026-05-15T13:00:00Z",
                "triggeringVoteId": null
            }),
        ),
    ).expect("effective");

    let row = elohim_storage::db::key_revocations::find_by_id(&mut conn, "u-revocation-2")
        .expect("find by id")
        .expect("row present");
    assert_eq!(row.effective_at.as_deref(), Some("2026-05-15T13:00:00Z"));
}
```

Adapt to actual `ElohimContentSignal` field names (read `signals.rs:1851` first) and actual `db::key_revocations::find_by_id` signature (read `db/key_revocations.rs`). If `find_by_id` does not exist, use the closest existing query (e.g., a Diesel `.select().first()` against `key_revocations` by `id`).

- [ ] **Step 4: Run test → FAIL**

```
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --test recovery_flow_projector_e2e 2>&1 | tail -30
```

Expected: FAIL — `recovery_flow_projector` module does not exist.

- [ ] **Step 5: Create `recovery_flow_projector.rs`**

```rust
// elohim/elohim-storage/src/services/recovery_flow_projector.rs
//! # Recovery Flow Projector
//!
//! Sibling to `attestation_projector` (per `2026-05-15-recovery-m4-brainstorm.md`
//! D1). Projects recovery-domain Content signals into the `key_revocations`
//! and `revocation_votes` projection tables.
//!
//! ## Why a sibling (not an extension of AttestationProjector)
//!
//! `AttestationProjector::handle_content_signal` is an accumulator — it routes
//! `attestation:*` and `governance-action:*` signals into the general-purpose
//! `attestations` and `governance_actions` tables. Recovery-flow projection is
//! state-machine-shaped (`Open → Quorum → Effective`); that lifecycle does not
//! compose cleanly as a fourth branch in the accumulator's match. Splitting
//! into a sibling keeps each projector's responsibility crisp.
//!
//! The seam: `attestation:revocation-vote` children continue to land in the
//! `attestations` accumulator table via `AttestationProjector` (no change). Only
//! the governance-action openers and the `key-revocation:effective` events are
//! routed here.
//!
//! ## Routed content_types
//!
//! - `governance-action:key-revocation` → upsert into `key_revocations` (opener)
//! - `key-revocation:effective`         → mark `key_revocations` row effective
//! - `attestation:revocation-vote`      → handled by `AttestationProjector`;
//!                                        this projector observes vote arrival
//!                                        via the recount path below
//!
//! ## Co-deliverable with M4 Stage 2
//!
//! The producer side (`governance-action:key-revocation` and
//! `key-revocation:effective` Content entries on the imagodei DHT, emitted from
//! the post-commit hook) is owned by `2026-05-15-recovery-m4-completion-shamir-
//! optional-kickoff-prompt.md` Stage 2.

use diesel::SqliteConnection;
use tracing::debug;

use crate::signals::ElohimContentSignal;
use crate::storage_error::StorageError;
use crate::db::{key_revocations, revocation_votes, models};

/// Project a recovery-flow Content signal into the projection tables.
///
/// Returns `Ok(())` for ignored content_types (consistent with
/// `AttestationProjector::handle_content_signal`'s "unknown content_type —
/// silently ignored" contract at line 70).
pub fn handle_content_signal(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    if signal.content_type == "governance-action:key-revocation" {
        // Opener: build UpsertKeyRevocationRow from the signal body and upsert.
        let row = build_key_revocation_opener_row(signal)?;
        key_revocations::upsert_key_revocation(conn, row)?;
        debug!(
            id = %signal.id,
            "key_revocations opener projected"
        );
    } else if signal.content_type == "key-revocation:effective" {
        // Effective: parse revocation_id + effective_at from body and mark row.
        let parsed = parse_key_revocation_effective(signal)?;
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        key_revocations::set_key_revocation_effective(
            conn,
            &parsed.revocation_id,
            &parsed.effective_at,
            &now,
        )?;
        debug!(
            revocation_id = %parsed.revocation_id,
            effective_at = %parsed.effective_at,
            "key_revocations marked effective"
        );
    }
    // All other content_types are not in this projector's grain; ignored.
    Ok(())
}

fn build_key_revocation_opener_row(
    signal: &ElohimContentSignal,
) -> Result<models::UpsertKeyRevocationRow, StorageError> {
    // Parse signal.body_json into the row shape. Mirror the field plumbing
    // already present at signals.rs:1040–1090 (legacy path) — same field names,
    // same types, same threshold_reached i32 encoding (0/1).
    //
    // STRICT: do not change the schema of UpsertKeyRevocationRow. If the
    // signal body is missing a required field, return StorageError::Deserialize
    // with a clear error message — do NOT default-fill.
    todo!("port body-json parsing from signals.rs:1040-1090; preserve i32 boolean encoding for threshold_reached")
}

struct ParsedKeyRevocationEffective {
    revocation_id: String,
    effective_at: String,
}

fn parse_key_revocation_effective(
    signal: &ElohimContentSignal,
) -> Result<ParsedKeyRevocationEffective, StorageError> {
    // Parse signal.body_json. Expected shape: { revocationId, humanId, revokedKey,
    // effectiveAt, triggeringVoteId? }. Mirror the field plumbing at
    // signals.rs:1128-1145 (legacy path).
    todo!("port body-json parsing from signals.rs:1128-1145")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_helpers::fresh_pool;

    // Mirror attestation_projector.rs:216-331 fixture style.
    #[test]
    fn handles_opener_signal() {
        let pool = fresh_pool();
        let mut conn = pool.get().expect("conn");
        // ... see e2e tests for the canonical fixture shape; this is a
        // unit-level smoke test against build_key_revocation_opener_row.
    }
}
```

Replace the `todo!()` stubs with the actual body-parsing logic, ported from `signals.rs:1040–1090` and `signals.rs:1128–1145`. The legacy code at those lines already does the right thing — port it directly, do not rewrite. The only behavioural difference is that the new projector consumes `ElohimContentSignal` (which carries `body_json: String`) instead of the typed `RecoveryV2Signal` variants — so the parse step is `serde_json::from_str::<KeyRevocationOpenerBody>(&signal.body_json)?`, where `KeyRevocationOpenerBody` is a private struct with `#[serde(rename_all = "camelCase")]` matching the body JSON shape.

- [ ] **Step 6: Re-export the module from `services/mod.rs`**

In `elohim/elohim-storage/src/services/mod.rs`, near the existing `pub mod attestation_projector;` line (97), add:

```rust
pub mod recovery_flow_projector;
```

- [ ] **Step 7: Wire the central signal dispatcher (from Step 2 discovery)**

At the dispatcher site discovered in Step 2, add prefix-routing BEFORE the existing call to `attestation_projector::handle_content_signal`:

```rust
let ct = &signal.content_type;
if ct == "governance-action:key-revocation"
    || ct == "key-revocation:effective"
    || ct.starts_with("key-revocation:")
{
    services::recovery_flow_projector::handle_content_signal(&mut conn, &signal)?;
} else {
    services::attestation_projector::handle_content_signal(&mut conn, &signal)?;
}
```

Note: `attestation:revocation-vote` does NOT match the recovery prefix and continues to flow to `attestation_projector` — this is correct per M4 D1 (vote-children land in the accumulator `attestations` table).

If the dispatcher does not have a live caller (Step 2 outcome 2), this step also wires the caller — record the wiring location in the commit message.

- [ ] **Step 8: Run E2E test + projector unit tests → PASS**

```
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --test recovery_flow_projector_e2e 2>&1 | tail -20
cargo test --lib services::recovery_flow_projector 2>&1 | tail -20
cargo test --lib services::attestation_projector 2>&1 | tail -20
```

Expected: all three test groups PASS. The `attestation_projector` test group must STILL PASS (unmodified module).

- [ ] **Step 9: Schema-contract test + ts-rs codegen check**

If a new view type was introduced (this task should not introduce one — `KeyRevocationView` already exists at `views.rs:6874`), regenerate types:

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test export_bindings 2>&1 | tail -10
cd /projects/elohim
pnpm run schema:codegen:ts 2>&1 | tail -10
cargo test --test schema_contract 2>&1 | tail -10  # run inside elohim-storage dir
```

Expected: no diff (no new view types added).

- [ ] **Step 10: Run clippy + fmt**

```
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo clippy --lib --tests -- -D warnings 2>&1 | tail -20
cargo fmt --check
```

Both must pass.

- [ ] **Step 11: Commit**

```bash
git add elohim/elohim-storage/src/services/recovery_flow_projector.rs \
        elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/tests/recovery_flow_projector_e2e.rs \
        # plus the dispatcher file discovered in Step 2 (e.g., signals.rs or a reconcile module)
git commit -m "$(cat <<'EOF'
feat(storage): W2D — RecoveryFlowProjector sibling + dispatcher wiring

Per Recovery M4 brainstorm D1 + EPR Wave 0 audit Slice 3: create a
sibling RecoveryFlowProjector at services/recovery_flow_projector.rs
that projects governance-action:key-revocation openers and
key-revocation:effective signals into the existing key_revocations
projection table.

Why a sibling, not an extension of AttestationProjector:
AttestationProjector is an accumulator (sinks attestation:* and
governance-action:* into general-purpose tables). RecoveryFlow is
a state-machine controller (Open → Quorum → Effective lifecycle).
Splitting keeps each projector's responsibility crisp.

attestation:revocation-vote children continue to land in the
attestations accumulator table via AttestationProjector — unchanged.

Central signal dispatcher gains a prefix-routing step BEFORE the
attestation_projector call: governance-action:key-revocation and
key-revocation:* signals route to recovery_flow_projector;
everything else continues to attestation_projector.

The writer body-parsing logic is ported from the now-dead path at
signals.rs::handle_recovery_v2_signal:1040–1145. The db writer
functions (upsert_key_revocation, set_key_revocation_effective,
update_current_votes) are unchanged.

Co-deliverable with Recovery M4 Stage 2 (producer side: post-commit
hook emits governance-action:key-revocation and key-revocation:
effective Content entries on the imagodei DHT).

Unblocks epr_2b_batch_a_full_loop #[ignore] once Task 6 lands
(derive_compromise_at Stage 2 upgrade).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: W2D — upgrade `derive_compromise_at` from Stage-1 stub to projection lookup

> **STATUS — 2026-05-15: ✅ LANDED (reframed as legacy-path retirement).** Closing commit `e3f03e14c`.
>
> User directive 2026-05-15: *"we shouldn't be worried about backwards compatibility."* T6 was reframed mid-sprint from "upgrade the Stage-1 stub" into a pure consumer-side retirement of the legacy `RecoveryV2Signal::KeyRevocationEffective` path. The new T18 `DnaSignal::KeyRevocation` envelope carries `compromise_at` directly in metadata, making the derivation lookup unnecessary.
>
> **What landed on the consumer side (elohim-storage):**
> - `derive_compromise_at` function REMOVED entirely (was at `holochain_app_signal.rs:289-297`). Zero callers remain.
> - `translate_recovery_v2`'s legacy `KeyRevocationEffective` arm FOLDED into the "not consumed by reconcile controller" catch-all at `holochain_app_signal.rs:222-225`.
> - `handle_recovery_v2_signal`'s legacy arm at `signals.rs:1200-1206` REPLACED with explicit no-op (`{ .. } => Ok(())`) so producer-side emissions don't break match exhaustiveness while M4 deletes the variant.
> - Stage-1 fallback comment that T5 left at `signals.rs:1212-1213` REMOVED.
> - Module doc-comment at `holochain_app_signal.rs:27-35` rewritten to record the retirement.
> - Orphaned `KeyRevocationSignal` import REMOVED.
>
> Most of the deletion landed on dev independently and arrived in this branch via merge `d14b0e3ae`. EPR closing commit `e3f03e14c` cleaned up the orphaned import + finalized the file state.
>
> **What M4 still owes (producer side):** the `RecoveryV2Signal::KeyRevocationEffective` variant + its three emission sites in the imagodei coordinator zome (`create_self_revocation`, `submit_revocation_vote` threshold-reached branch, `submit_specialist_revocation`) — these emit to dead air on the EPR consumer side and M4 deletes them as part of T19+ tail work. EPR's no-op arm stays in place until M4 confirms producer-side deletion.
>
> **Canonical replacement:** `DnaSignal::KeyRevocation(KeyRevocationEnvelope)` consumed by `signals::handle_imagodei_dna_signal` at `signals.rs:1425`. See memory entry `project_t6_legacy_revocation_path_retired.md`.

**Files:**
- Modify: `elohim/elohim-storage/src/reconcile/holochain_app_signal.rs:289–296`

`derive_compromise_at` is the function that computes the retroactive sweep window for `KeyRevocationSignal::compromise_at`. Today it is a Stage-1 stub that always returns `effective_at_fallback`:

```rust
fn derive_compromise_at(
    _db_pool: Option<&Arc<DbPool>>,
    _revocation_id: &str,
    effective_at_fallback: DateTime<Utc>,
) -> DateTime<Utc> {
    // Stage 1: always use effective_at as conservative compromise_at.
    // See TODO(A.12) above for the upgrade path.
    effective_at_fallback
}
```

Stage 2 reads `revocation_votes` (joined to `key_revocations` for `revocation_id`) and returns the timestamp of the earliest approving vote (the moment the protocol can prove the key was compromised). This is the projection-lookup path mentioned in `recovery_flow_projector.rs`'s module doc.

- [ ] **Step 1: Read the function and its caller**

```
sed -n '180,210p' /projects/elohim/elohim/elohim-storage/src/reconcile/holochain_app_signal.rs
sed -n '280,300p' /projects/elohim/elohim/elohim-storage/src/reconcile/holochain_app_signal.rs
grep -n "fn count_approved_votes_for_revocation\|fn earliest_approving_vote" /projects/elohim/elohim/elohim-storage/src/db/revocation_votes.rs
```

Confirm:
- Function signature at `:289` is `fn derive_compromise_at(_db_pool: Option<&Arc<DbPool>>, _revocation_id: &str, effective_at_fallback: DateTime<Utc>) -> DateTime<Utc>`. Keep this exact signature (caller depends on it).
- Caller at `:189` passes `(db_pool, &revocation_id, effective_at_dt)`. No change needed there.
- `db::revocation_votes` may not have an `earliest_approving_vote` query helper yet. If not, add one in Step 3.

- [ ] **Step 2: Write failing tests**

Extend `holochain_app_signal.rs::tests` (the existing test module — search for `#[cfg(test)] mod tests` in the file):

```rust
#[test]
fn derive_compromise_at_returns_earliest_approving_vote_timestamp() {
    use crate::db::test_helpers::fresh_pool;
    use crate::db::models::{UpsertKeyRevocationRow, InsertRevocationVoteRow};
    let pool = fresh_pool();
    let pool_arc = Arc::new(pool);
    let mut conn = pool_arc.get().expect("conn");

    // Pre-seed: opener + 3 votes (2 approving, 1 rejecting).
    crate::db::key_revocations::upsert_key_revocation(&mut conn, UpsertKeyRevocationRow {
        dht_anchor_hash: "u-rev-1".into(),
        id: "u-rev-1".into(),
        human_id: "h1".into(),
        revoked_key: "k".into(),
        reason: "compromise".into(),
        trigger_type: "self".into(),
        initiated_by: "agent-1".into(),
        required_votes: 3,
        current_votes: 0,
        threshold_reached: 0,
        effective_at: None,
        created_at: "2026-05-15T10:00:00Z".into(),
        updated_at: "2026-05-15T10:00:00Z".into(),
    }).expect("opener");

    for (id, ts, approved) in &[
        ("vote-1", "2026-05-15T11:00:00Z", true),
        ("vote-2", "2026-05-15T10:30:00Z", true),   // earliest approving
        ("vote-3", "2026-05-15T10:15:00Z", false),  // earlier, but rejecting — must be ignored
    ] {
        crate::db::revocation_votes::insert_revocation_vote(&mut conn, InsertRevocationVoteRow {
            dht_anchor_hash: id.to_string(),
            id: id.to_string(),
            revocation_dht_anchor_hash: "u-rev-1".into(),
            revocation_id: "u-rev-1".into(),
            steward_id: format!("steward-{id}"),
            approved: if *approved { 1 } else { 0 },
            attestation: "{}".into(),
            voted_at: ts.to_string(),
        }).expect("vote");
    }

    let effective_at = chrono::DateTime::parse_from_rfc3339("2026-05-15T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let result = derive_compromise_at(Some(&pool_arc), "u-rev-1", effective_at);

    let expected = chrono::DateTime::parse_from_rfc3339("2026-05-15T10:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert_eq!(result, expected,
        "compromise_at must be the earliest *approving* vote timestamp, not the earliest vote overall");
}

#[test]
fn derive_compromise_at_falls_back_when_projection_missing() {
    let pool = crate::db::test_helpers::fresh_pool();
    let pool_arc = Arc::new(pool);

    let effective_at = chrono::DateTime::parse_from_rfc3339("2026-05-15T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    // No opener, no votes — projection is empty for this revocation_id.
    let result = derive_compromise_at(Some(&pool_arc), "u-rev-missing", effective_at);

    assert_eq!(result, effective_at,
        "absent projection rows must fall back to effective_at_fallback");
}

#[test]
fn derive_compromise_at_falls_back_when_no_approving_votes() {
    // Edge case: opener exists, all recorded votes are rejecting.
    // The function must still return effective_at_fallback rather than
    // the earliest rejecting vote.
    let pool = crate::db::test_helpers::fresh_pool();
    let pool_arc = Arc::new(pool);
    let mut conn = pool_arc.get().expect("conn");

    crate::db::key_revocations::upsert_key_revocation(&mut conn, UpsertKeyRevocationRow {
        dht_anchor_hash: "u-rev-2".into(), id: "u-rev-2".into(),
        human_id: "h".into(), revoked_key: "k".into(),
        reason: "compromise".into(), trigger_type: "self".into(),
        initiated_by: "agent".into(),
        required_votes: 3, current_votes: 0, threshold_reached: 0,
        effective_at: None,
        created_at: "2026-05-15T10:00:00Z".into(),
        updated_at: "2026-05-15T10:00:00Z".into(),
    }).expect("opener");

    crate::db::revocation_votes::insert_revocation_vote(&mut conn, InsertRevocationVoteRow {
        dht_anchor_hash: "vote-no".into(), id: "vote-no".into(),
        revocation_dht_anchor_hash: "u-rev-2".into(),
        revocation_id: "u-rev-2".into(),
        steward_id: "s".into(), approved: 0, attestation: "{}".into(),
        voted_at: "2026-05-15T10:30:00Z".into(),
    }).expect("vote");

    let effective_at = chrono::DateTime::parse_from_rfc3339("2026-05-15T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let result = derive_compromise_at(Some(&pool_arc), "u-rev-2", effective_at);
    assert_eq!(result, effective_at);
}
```

Adapt imports + `models::*` field names to the actual definitions if they have drifted from what was read in Step 1.

- [ ] **Step 3: Run tests → FAIL**

```
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --lib reconcile::holochain_app_signal::tests::derive_compromise_at 2>&1 | tail -30
```

Expected: FAIL — Stage-1 stub returns `effective_at_fallback` for all three tests; the first assert fails (expected `10:30`, got `12:00`).

- [ ] **Step 4: Add the query helper `earliest_approving_vote_at` to `db/revocation_votes.rs`**

If it does not already exist, add:

```rust
/// Return the timestamp of the earliest approving (`approved = 1`) vote for
/// the given revocation, or `None` if there are no approving votes (or no
/// votes at all).
pub fn earliest_approving_vote_at(
    conn: &mut SqliteConnection,
    revocation_id: &str,
) -> Result<Option<String>, StorageError> {
    use crate::schema::revocation_votes::dsl as rv;
    let voted_at: Option<String> = rv::revocation_votes
        .filter(rv::revocation_id.eq(revocation_id))
        .filter(rv::approved.eq(1))
        .select(rv::voted_at)
        .order_by(rv::voted_at.asc())
        .first(conn)
        .optional()?;
    Ok(voted_at)
}
```

Match the actual Diesel schema column names from `schema.rs` (run `grep -n "revocation_votes" /projects/elohim/elohim/elohim-storage/src/schema.rs` first to confirm). If the column for the vote timestamp is named differently (e.g., `cast_at` instead of `voted_at`), use the actual name.

- [ ] **Step 5: Replace the stub body of `derive_compromise_at`**

In `holochain_app_signal.rs:289–296`:

```rust
fn derive_compromise_at(
    db_pool: Option<&Arc<DbPool>>,
    revocation_id: &str,
    effective_at_fallback: DateTime<Utc>,
) -> DateTime<Utc> {
    // Stage 2 (W2D): read the projection to find the earliest approving vote
    // timestamp — the moment at which we can prove the key was compromised.
    // Fall back to effective_at if the projection has no approving votes
    // (defensive — the controller should not reach here without votes, but
    // the conservative fallback preserves correctness on cold-start replays).
    let Some(pool) = db_pool else {
        return effective_at_fallback;
    };
    let Ok(mut conn) = pool.get() else {
        return effective_at_fallback;
    };
    let earliest = match crate::db::revocation_votes::earliest_approving_vote_at(
        &mut conn,
        revocation_id,
    ) {
        Ok(Some(ts)) => ts,
        _ => return effective_at_fallback,
    };
    match chrono::DateTime::parse_from_rfc3339(&earliest) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(_) => effective_at_fallback,
    }
}
```

The signature is identical to Stage 1; the body is the only change. Underscores removed from `_db_pool` and `_revocation_id` since they are now used.

- [ ] **Step 6: Update the function's doc comment to reflect Stage 2**

Find the comment block immediately above `fn derive_compromise_at` (lines ~280–289) and update from Stage-1 stub language to:

```rust
/// Derive `compromise_at` for a `KeyRevocationEffective` signal.
///
/// Stage 2 (W2D) reads the `revocation_votes` projection and returns the
/// timestamp of the earliest *approving* vote for `revocation_id`. This is the
/// moment at which the protocol can prove the key was compromised — the
/// retroactive sweep window for `verified_at` invalidation extends back to
/// this point.
///
/// Falls back to `effective_at_fallback` when:
/// - `db_pool` is `None` (caller is in a non-db context — synthetic tests)
/// - DB pool acquisition fails (operational hazard; conservative fallback)
/// - The projection has no approving votes for `revocation_id` (edge case;
///   shouldn't happen via the controller path but is observable on cold-start
///   replays — conservative fallback preserves correctness)
```

Remove the prior `TODO(A.12)` reference in the comment if present.

- [ ] **Step 7: Run tests → PASS**

```
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --lib reconcile::holochain_app_signal::tests::derive_compromise_at 2>&1 | tail -20
cargo test --lib db::revocation_votes 2>&1 | tail -10
```

Expected: 3 new tests PASS; existing `revocation_votes` tests STILL PASS.

- [ ] **Step 8: Lift `#[ignore]` on `epr_2b_batch_a_full_loop`**

In `elohim/holochain/tests/sweettest/src/tests/epr_phase_2b_batch_a_e2e.rs` around line 645, the test has:

```rust
#[ignore = "requires packed imagodei DNA artifact — wire into Jenkins pack-then-test stage; \
            also requires Stage 2 derive_compromise_at upgrade for verified_at sweep assertion \
            (TODO(A.12) in holochain_app_signal.rs::derive_compromise_at)"]
```

The Stage 2 upgrade is now landed (this task), but the Jenkins pack-then-test condition is **not** in scope for this plan. **Do not lift the `#[ignore]` until both conditions are met.**

If the operator confirms the Jenkins stage is also in place before this commit, lift the `#[ignore]` by deleting the entire `#[ignore = "..."]` attribute line. Otherwise, narrow the ignore message to only the Jenkins condition:

```rust
#[ignore = "requires packed imagodei DNA artifact — wire into Jenkins pack-then-test stage"]
```

In either case, capture the decision in the commit message.

- [ ] **Step 9: Run clippy + fmt**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo clippy --lib --tests -- -D warnings 2>&1 | tail -20
cargo fmt --check
```

- [ ] **Step 10: Commit**

```bash
git add elohim/elohim-storage/src/reconcile/holochain_app_signal.rs \
        elohim/elohim-storage/src/db/revocation_votes.rs \
        elohim/holochain/tests/sweettest/src/tests/epr_phase_2b_batch_a_e2e.rs
git commit -m "$(cat <<'EOF'
feat(storage): W2D Stage 2 — derive_compromise_at reads projection

Replace the Stage-1 stub at holochain_app_signal.rs:289 (which
always returned effective_at_fallback) with a projection-lookup
that finds the earliest approving vote in revocation_votes and
uses its timestamp as compromise_at — the moment we can prove
the key was compromised.

Falls back to effective_at when:
- db_pool is None (non-db test contexts)
- DB pool acquisition fails (operational hazard)
- No approving votes for revocation_id (cold-start edge)

Adds db::revocation_votes::earliest_approving_vote_at as the
single query helper. Same signature on derive_compromise_at,
caller at :189 unchanged.

[Lifted | Narrowed] #[ignore] on epr_2b_batch_a_full_loop —
[Stage 2 + Jenkins pack-then-test both ready / Jenkins pack
condition still outstanding, see narrowed ignore message].

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

(Pick the appropriate `[bracket]` variant in the commit message based on the Step 8 decision.)

---

## Task 7: W2 — `RevocationAttestation` IntegrityNotify arm

**Files:**
- Modify: `elohim/elohim-storage/src/epr_atom_service.rs` (insert new match arm in `handle_integrity_notify` after the `KeyRotation` arm, before `other_kind`)
- Possibly Modify: `elohim/elohim-storage/src/p2p/recovery_revocation.rs` if a new wire-shape struct is required (it should not be — duality means we use the existing `revocation-attestation.schema.json` shape directly)
- Modify: `elohim/elohim-storage/src/epr_atom_service.rs::tests` (add two regression tests)

Per D3 (duality): the signal payload follows `elohim/sdk/schemas/v1/dna-signals/revocation-attestation.schema.json` — slim operational payload with `actionHash`, `revocationId`, `stewardId`, `approved`, `attestationKind`, `currentVotes`, `requiredVotes`, `thresholdReached`, `attestedAt`, `emittedAt`. No new wire contract. Mirror the `KeyRotation` arm at `:340–419` and the `KeyRevocation` arm at `:291–339`.

- [ ] **Step 1: Read the schema + sibling arms**

```
cat /projects/elohim/elohim/sdk/schemas/v1/dna-signals/revocation-attestation.schema.json
sed -n '280,420p' /projects/elohim/elohim/elohim-storage/src/epr_atom_service.rs
sed -n '1,60p' /projects/elohim/elohim/elohim-storage/src/p2p/recovery_revocation.rs
sed -n '1,60p' /projects/elohim/elohim/elohim-storage/src/p2p/recovery_rotation.rs
```

Confirm:
- The slim payload field names + types match the schema (cross-reference at `revocation-attestation.schema.json`).
- The two sibling arms share the same shape: decode message → dedup → log info → return `IntegrityAck { received: true }`. On decode failure: log warn + return `IntegrityAck { received: false, reason }`.
- A wire struct similar to `RecoveryRevocationMessage` / `RecoveryRotationMessage` is needed. Decision: add a new struct in a new file `p2p/revocation_attestation_message.rs` so this stays parallel to the two existing wire types.

- [ ] **Step 2: Create the wire struct**

Create `elohim/elohim-storage/src/p2p/revocation_attestation_message.rs`:

```rust
//! # RevocationAttestationMessage wire type
//!
//! **Source of truth:** Holochain DHT — `KeyRevocation` and `RevocationVote`
//! integrity entries (imagodei zome). This struct is a **Category C
//! operational projection** of those entries, serialized over the IntegrityNotify
//! libp2p direct-notify path. Per EPR D3 / M4 D2 (duality binding), the wire
//! shape mirrors the published `elohim/sdk/schemas/v1/dna-signals/revocation-
//! attestation.schema.json` contract exactly — no new schema, no envelope
//! inlining, no DHT round-trip per vote.
//!
//! **Lifetime:** transient. No persistence at this layer — canonical writes
//! happen via the local conductor's signal stream through the
//! `AttestationProjector` (for `attestation:revocation-vote` children landing
//! in `attestations`) and `RecoveryFlowProjector` (for `governance-action:
//! key-revocation` openers landing in `key_revocations`).
//!
//! **Producer:** Recovery M4 Stage 3 (post-commit hook).
//! **Consumer:** `epr_atom_service::handle_integrity_notify` "RevocationAttestation" arm.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RevocationAttestationMessage {
    /// Holochain ActionHash (base64) of the originating DHT entry.
    /// For 'request' kind: the KeyRevocation entry action hash.
    /// For 'vote' kind: the RevocationVote entry action hash.
    pub action_hash: String,
    /// Stable revocation flow id (M4 KeyRevocationRequested.id /
    /// RevocationVoteSubmitted.revocation_id).
    pub revocation_id: String,
    /// Agent CID or pubkey of the steward submitting the attestation.
    pub steward_id: String,
    /// Whether this attestation approves the revocation.
    pub approved: bool,
    /// 'request' or 'vote' per schema.
    pub attestation_kind: String,
    pub current_votes: u32,
    pub required_votes: u32,
    pub threshold_reached: bool,
    /// ISO-8601 timestamp when the attestation was recorded in the DNA.
    pub attested_at: String,
    /// ISO-8601 timestamp when the signal was emitted from post-commit.
    pub emitted_at: String,
}

impl RevocationAttestationMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revocation_attestation_message_roundtrips_msgpack() {
        let original = RevocationAttestationMessage {
            action_hash: "uhCkk...".into(),
            revocation_id: "u-revocation-1".into(),
            steward_id: "agent-cid-1".into(),
            approved: true,
            attestation_kind: "vote".into(),
            current_votes: 2,
            required_votes: 3,
            threshold_reached: false,
            attested_at: "2026-05-15T11:00:00Z".into(),
            emitted_at: "2026-05-15T11:00:01Z".into(),
        };

        let bytes = original.to_bytes().expect("encode");
        let decoded = RevocationAttestationMessage::from_bytes(&bytes).expect("decode");
        assert_eq!(decoded, original);
    }
}
```

Add `pub mod revocation_attestation_message;` to `elohim/elohim-storage/src/p2p/mod.rs` near the existing `pub mod recovery_revocation;` and `pub mod recovery_rotation;` lines.

- [ ] **Step 3: Run the round-trip test → PASS**

```
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --lib p2p::revocation_attestation_message 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 4: Add failing tests for the IntegrityNotify arm**

In `epr_atom_service.rs::tests` (near the existing `integrity_notify_keyrotation_acks_received_true` + `_dedup_returns_duplicate_reason` at lines 453 + 489), add:

```rust
#[test]
fn integrity_notify_revocation_attestation_acks_received_true() {
    let service = EprAtomService::new_for_test();  // match existing helper
    let msg = crate::p2p::revocation_attestation_message::RevocationAttestationMessage {
        action_hash: "uhCkk1".into(),
        revocation_id: "u-rev-int-1".into(),
        steward_id: "agent-cid".into(),
        approved: true,
        attestation_kind: "vote".into(),
        current_votes: 2,
        required_votes: 3,
        threshold_reached: false,
        attested_at: "2026-05-15T11:00:00Z".into(),
        emitted_at: "2026-05-15T11:00:01Z".into(),
    };
    let bytes = msg.to_bytes().expect("encode");

    let response = service.handle(
        "test-peer",
        CallerIdentity::Anonymous,
        EprAtomRequest::IntegrityNotify {
            kind: "RevocationAttestation".to_string(),
            payload_bytes: bytes,
        },
    );

    match response {
        EprAtomResponse::IntegrityAck { received: true, reason: None } => {}
        other => panic!("expected IntegrityAck {{ received: true }}, got {:?}", other),
    }
}

#[test]
fn integrity_notify_revocation_attestation_dedup_returns_duplicate_reason() {
    let service = EprAtomService::new_for_test();
    let msg = crate::p2p::revocation_attestation_message::RevocationAttestationMessage {
        action_hash: "uhCkk2".into(),
        revocation_id: "u-rev-int-dedup".into(),
        steward_id: "agent-cid".into(),
        approved: true,
        attestation_kind: "vote".into(),
        current_votes: 2,
        required_votes: 3,
        threshold_reached: false,
        attested_at: "2026-05-15T11:00:00Z".into(),
        emitted_at: "2026-05-15T11:00:01Z".into(),
    };
    let bytes = msg.to_bytes().expect("encode");

    // First delivery: received: true, reason: None
    let _ = service.handle(
        "test-peer", CallerIdentity::Anonymous,
        EprAtomRequest::IntegrityNotify {
            kind: "RevocationAttestation".into(),
            payload_bytes: bytes.clone(),
        });

    // Second delivery: dedup'd, received: true, reason: Some("duplicate")
    let response = service.handle(
        "test-peer", CallerIdentity::Anonymous,
        EprAtomRequest::IntegrityNotify {
            kind: "RevocationAttestation".into(),
            payload_bytes: bytes,
        });

    match response {
        EprAtomResponse::IntegrityAck { received: true, reason: Some(r) } if r == "duplicate" => {}
        other => panic!("expected dedup'd IntegrityAck, got {:?}", other),
    }
}
```

- [ ] **Step 5: Run tests → FAIL**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --lib epr_atom_service 2>&1 | tail -20
```

Expected: the two new tests FAIL — the `RevocationAttestation` arm falls into `other_kind` and returns `received: false`.

- [ ] **Step 6: Insert the `RevocationAttestation` match arm**

In `epr_atom_service.rs::handle_integrity_notify`, after the `"KeyRotation"` arm closes (around line 419) and BEFORE the `other_kind` catch-all:

```rust
"RevocationAttestation" => {
    match crate::p2p::revocation_attestation_message::RevocationAttestationMessage::from_bytes(
        &payload_bytes,
    ) {
        Ok(msg) => {
            // Dedup on synthetic key. Same attestation arriving via direct-
            // notify + signal stream will not double-process after the first
            // delivery. The dedup key includes the action_hash so two
            // attestations on the same revocation (a 'request' and a 'vote',
            // or two different stewards' votes) do not collide.
            let dedup_key = format!(
                "RevocationAttestation:{}:{}",
                msg.revocation_id, msg.action_hash
            );
            if !self.dedup.insert(&dedup_key) {
                debug!(
                    target: "elohim_storage::dedup",
                    from = %peer_label,
                    revocation_id = %msg.revocation_id,
                    action_hash = %msg.action_hash,
                    "duplicate RevocationAttestation direct-notify — dropped"
                );
                return EprAtomResponse::IntegrityAck {
                    received: true,
                    reason: Some("duplicate".to_string()),
                };
            }
            info!(
                target: "elohim_storage::recovery",
                from = %peer_label,
                revocation_id = %msg.revocation_id,
                action_hash = %msg.action_hash,
                attestation_kind = %msg.attestation_kind,
                steward_id = %msg.steward_id,
                threshold_reached = msg.threshold_reached,
                "W2: Received RevocationAttestation via direct-notify"
            );
            // Per D3 duality: the canonical write to the projection happens
            // via the AttestationProjector (for attestation:revocation-vote
            // children landing in `attestations`) and via the
            // RecoveryFlowProjector (for governance-action:key-revocation
            // openers / key-revocation:effective signals landing in
            // `key_revocations`). Direct-notify is delivery-optimistic — it
            // does not write to projections here, to avoid divergence with
            // the canonical signal-stream-driven path.
            EprAtomResponse::IntegrityAck {
                received: true,
                reason: None,
            }
        }
        Err(e) => {
            warn!(
                target: "elohim_storage::recovery",
                from = %peer_label,
                error = %e,
                "W2: Failed to decode RevocationAttestationMessage from direct-notify"
            );
            EprAtomResponse::IntegrityAck {
                received: false,
                reason: Some(format!("decode failed: {e}")),
            }
        }
    }
}
```

**STRICT FORBID:** do not change `EprAtomResponse::IntegrityAck` shape, do not modify `handle_integrity_notify` signature, do not touch other match arms.

- [ ] **Step 7: Run tests → PASS**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --lib epr_atom_service 2>&1 | tail -20
cargo test --lib p2p::revocation_attestation_message 2>&1 | tail -10
```

Expected: both new tests PASS; existing `integrity_notify_*` tests (KeyRevocation, KeyRotation, unhandled_kind) STILL PASS.

- [ ] **Step 8: Run clippy + fmt**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo clippy --lib --tests -- -D warnings 2>&1 | tail -10
cargo fmt --check
```

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-storage/src/p2p/revocation_attestation_message.rs \
        elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/src/epr_atom_service.rs
git commit -m "$(cat <<'EOF'
feat(storage): W2 — IntegrityNotify RevocationAttestation arm

Per D3 (duality binding M4 D2): handle the RevocationAttestation
DNA signal via the existing slim revocation-attestation.schema.json
contract — no new schema, no contentEnvelope inlining.

Adds RevocationAttestationMessage wire type at
p2p/revocation_attestation_message.rs (mirrors recovery_rotation.rs
shape, camelCase struct-as-map MessagePack via to_vec_named).

epr_atom_service::handle_integrity_notify now matches
"RevocationAttestation" explicitly: decode → dedup on
RevocationAttestation:<revocation_id>:<action_hash> →
log at info → return IntegrityAck { received: true }.
Dedup key includes action_hash so a request and a vote (and
multiple stewards' votes) on the same revocation do not collide.

Direct-notify is delivery-optimistic. The canonical projection
writes happen via:
- AttestationProjector for attestation:revocation-vote children
  (lands in `attestations` accumulator table)
- RecoveryFlowProjector for governance-action:key-revocation
  openers + key-revocation:effective (lands in `key_revocations`)

AgentPeerBinding arm follows in Task 8 IFF Phase 12 caller identity
is live on iroh master (D5 gate).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: W2 — D5 Phase 12 gate check + conditional `AgentPeerBinding` arm

**Files (conditional on D5 outcome):**
- Modify: `elohim/elohim-storage/src/epr_atom_service.rs` (new match arm)
- Create: `elohim/elohim-storage/src/p2p/agent_peer_binding_message.rs`

D5 from the kickoff: the `AgentPeerBinding` IntegrityNotify arm waits on iroh master Phase 12 caller-identity landing. If Phase 12 is not live when this task runs, it is a Wave-2 follow-up, not a sprint blocker.

- [ ] **Step 1: Check Phase 12 status**

Phase 12 (iroh peer-transport manifest) is owned by `genesis/docs/superpowers/plans/2026-05-10-iroh-phase12-peer-transport-manifest.md`. Check its closure state:

```
grep -c "^- \[x\]" /projects/elohim/genesis/docs/superpowers/plans/2026-05-10-iroh-phase12-peer-transport-manifest.md
grep -c "^- \[ \]" /projects/elohim/genesis/docs/superpowers/plans/2026-05-10-iroh-phase12-peer-transport-manifest.md
```

Then verify that the caller-identity API surface is present on dev:

```
grep -rn "caller_identity\|CallerIdentity::Peer\|verify_peer_signature" /projects/elohim/elohim/elohim-storage/src/p2p/ | head -10
```

Decision matrix:
- **Phase 12 plan is ✅ LANDED on dev AND caller-identity API surface is present** → proceed to Step 2 (implement the AgentPeerBinding arm).
- **Phase 12 plan is incomplete OR caller-identity API surface is absent** → STOP. Record the gate state in a follow-up memory entry (`feedback_w2_agent_peer_binding_deferred.md`) noting the date checked + the remaining gate. The arm is deferred to a follow-up Wave-2 mini-plan; this sprint closes without it.

- [ ] **Step 2 (conditional): Implement `AgentPeerBindingMessage` wire type + IntegrityNotify arm**

If Step 1 said proceed, mirror Task 7 exactly with substitutions:
- Wire-type file: `p2p/agent_peer_binding_message.rs` (struct `AgentPeerBindingMessage` with fields per the `agent-peer-binding.schema.json` contract at `elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json` — read the schema first).
- Match arm in `epr_atom_service.rs`: `"AgentPeerBinding" => { ... }`, decode → dedup on synthetic key `AgentPeerBinding:<binding_action_hash>` → log info → return `IntegrityAck { received: true }`.
- Two tests: `integrity_notify_agent_peer_binding_acks_received_true` + `_dedup_returns_duplicate_reason`.

Follow the same TDD + clippy + commit shape as Task 7.

- [ ] **Step 3 (conditional): Run the full integrity-notify test set**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --lib epr_atom_service 2>&1 | tail -30
```

Expected: all four `integrity_notify_*` arm-handler test pairs PASS (KeyRevocation, KeyRotation, RevocationAttestation, AgentPeerBinding), plus the unhandled-kind regression.

- [ ] **Step 4 (conditional): Commit**

```bash
git add elohim/elohim-storage/src/p2p/agent_peer_binding_message.rs \
        elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/src/epr_atom_service.rs
git commit -m "$(cat <<'EOF'
feat(storage): W2 — IntegrityNotify AgentPeerBinding arm

D5 Phase 12 caller-identity gate is now live on iroh master
(verified via [...]). AgentPeerBinding direct-notify arm wires up
following the RevocationAttestation pattern: decode → dedup on
AgentPeerBinding:<binding_action_hash> → log info → IntegrityAck
{ received: true }.

Wire shape mirrors the agent-peer-binding.schema.json contract
exactly — same camelCase struct-as-map MessagePack discipline as
the three sibling wire types (recovery_revocation,
recovery_rotation, revocation_attestation_message).

Canonical projection writes for AgentPeerBinding remain on the
DHT signal-stream path through the reconcile controller. Direct-
notify is delivery-optimistic.

This commit closes Wave 2 of the EPR foundation-completion sprint.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Wave 5 — Opus-authored `@wip` lift on `epr-content-addressing.feature`

**Files:**
- Modify: `genesis/a2o/features/content/epr-content-addressing.feature`

Per `feedback_a2o_narrative_is_opus_work` + master plan dispatch shape: `@wip` removal is NOT mechanical. Each of the 9 scenarios must be read end-to-end against the production substrate; if a scenario reads false (the substrate cannot make it pass against real code), keep `@wip` and capture the substrate gap. If it reads true, lift `@wip` and ensure the scenario's persona setup + Given/When/Then phrasing tells the human story coherently.

**This task requires Opus judgment, not pattern matching.** Do not use Haiku for scenario-author work.

- [ ] **Step 1: Identify all 9 scenarios**

```
sed -n '20,140p' /projects/elohim/genesis/a2o/features/content/epr-content-addressing.feature
```

The `@wip` lines are at 27, 39, 52, 65, 79, 93, 106, 118, 130. Each one annotates a scenario starting on the next line. Capture the scenario titles + intent.

- [ ] **Step 2: For each scenario, walk the substrate**

For each of the 9 scenarios, ask:
1. **Does the step-def in `genesis/a2o/steps/ui/epr-content.steps.ts` exist and target real code?** (Audit confirmed yes; verify by grep on the scenario's step phrases.)
2. **Does the scenario's persona setup match how that persona actually appears on dev today?** (e.g., if it says "Matthew authors a manifest EPR" — does the matthew-as-author flow exist in seed data + UI?)
3. **Is the scenario's narrative the right grain for an a2o feature?** Per `feedback_a2o_is_human_experience_not_dev_bugs`: a2o = human experience, not internal dev mechanics. If a scenario is testing a serialization detail, route it to a unit test and DELETE the scenario (do not just lift `@wip`).

Capture decisions in a markdown table in the commit body:

| Line | Scenario title | Decision | Rationale |
|---|---|---|---|
| 27 | … | lift / keep @wip / delete + route to unit | … |

- [ ] **Step 3: Apply decisions per scenario**

For each "lift" decision: delete the `@wip` line (preserving `@browser-only` if present on the same line — the audit found 6 of the 9 also have `@browser-only`).

For "keep @wip" decisions: leave the line; add a comment above it explaining the still-missing substrate piece, of the form `# @wip retained: <one-line gap>`.

For "delete + route" decisions: delete the entire scenario block (Scenario / Given / When / Then) and add a `# Migrated to <unit-test-path>` line in its place.

- [ ] **Step 4: Run the elohim-app browser stage locally to confirm at least one lifted scenario passes**

The full browser stage is Jenkins-driven, but a smoke run validates the lift:

```
cd /projects/elohim/app/elohim-app
pnpm run hc:start:seed &  # bring up holochain + doorway + storage with seed
# wait until services are healthy
pnpm run cypress:run --spec "**/epr-content-addressing.feature"  # run just this feature
```

Expected: scenarios with `@wip` lifted pass; scenarios still tagged `@wip` are skipped per Cucumber's default tag filter.

If any lifted scenario fails, restore its `@wip` tag and document the gap in a `# @wip retained:` comment.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/content/epr-content-addressing.feature
git commit -m "$(cat <<'EOF'
feat(a2o): W5 — lift @wip on epr-content-addressing.feature (9 scenarios reviewed)

Opus-authored review of each @wip-tagged scenario per
feedback_a2o_narrative_is_opus_work. For each scenario: walked
the step-defs in epr-content.steps.ts, verified persona setup
exists on dev, and judged whether the scenario tells a human-
experience story (a2o grain) vs. a dev-mechanics story (unit-test
grain).

Decisions per scenario:
[paste table from Step 2]

Smoke run via cypress:run on the lifted scenarios passes locally;
full Jenkins browser stage will validate end-to-end.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Wave 5 — Opus-authored `@wip` lift on `epr-cross-peer-resolution.feature`

**Files:**
- Modify: `genesis/a2o/features/federation/epr-cross-peer-resolution.feature`

Same shape as Task 9; 8 scenarios at lines 70, 86, 100, 116, 128, 142, 154, 167.

- [ ] **Step 1: Identify all 8 scenarios**

```
sed -n '65,180p' /projects/elohim/genesis/a2o/features/federation/epr-cross-peer-resolution.feature
```

Note the comment at line 24 explaining one earlier `@wip` was lifted by Wave 0 audit — do not re-lift, just confirm presence.

- [ ] **Step 2: For each scenario, walk the substrate**

Same questions as Task 9, with federation-specific emphasis:
- Cross-peer resolution scenarios may require both libp2p AND iroh substrates to be live. If a scenario assumes iroh-only behaviour that is still in master, flag it and keep `@wip`.
- The attestation-narrative references at lines 74 + 103 are Gherkin prose (not step-defs) — the audit confirmed they do not invoke routes. These should not block a `@wip` lift on their containing scenarios.

- [ ] **Step 3: Apply decisions + commit**

Same shape as Task 9 Step 3 + Step 5. Use a single combined commit for this file.

- [ ] **Step 4: Run the federation pipeline smoke locally**

```
cd /projects/elohim/app/elohim-app
pnpm run cypress:run --spec "**/epr-cross-peer-resolution.feature"
```

Expected: scenarios with `@wip` lifted pass; scenarios still tagged `@wip` are skipped.

---

## Task 11: Wave 6 — kick off 1-week cross-stack soak

**Files:**
- No code change. This task is an operator-driven Jenkins observation, not a checkbox-per-step implementation. Captured here so the sprint closure tracking is explicit.

Per the kickoff §"Acceptance for the sprint as a whole" point 5 + master plan Wave 4:

- [ ] **Step 1: Confirm cross-stack integration test exists**

```
find /projects/elohim/elohim/holochain -name "*integrity_notify*" -o -name "*recovery*round_trip*" 2>/dev/null
find /projects/elohim/elohim/elohim-storage/tests -name "*revocation*round_trip*" -o -name "*m4*round_trip*" 2>/dev/null
```

If no cross-stack integration test exists yet (an M4 producer → IntegrityNotify consumer round-trip for `RevocationAttestation`), STOP and surface to the operator. This is a Recovery M4 sprint deliverable, not an EPR foundation-completion deliverable — but the soak depends on it.

- [ ] **Step 2: Trigger orchestrator dev build with a marker commit**

After Tasks 1–10 have landed on dev, push a small no-op commit to trigger a fresh orchestrator run:

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(epr): mark start of cross-stack soak (foundation-completion sprint close)

Tasks 1–10 of 2026-05-15-epr-foundation-completion.md landed.
Starting 1-week observation window per master plan Wave 4. SUCCESS
or UNSTABLE-not-regressed for 2 consecutive runs with at least one
fresh trigger from this push closes the sprint.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
git push origin dev
```

- [ ] **Step 3: Watch orchestrator dev for 1 week**

The acceptance criterion: orchestrator dev returns SUCCESS or UNSTABLE-not-regressed for **2 consecutive runs with at least one fresh trigger from this sprint's push** + the cross-stack integration test (M4 producer + W2 RevocationAttestation consumer) round-trips at least once green.

Use Jenkins MCP (per `feedback_shift_measure_jenkins`) to observe — do NOT poll the orchestrator from the dev environment.

- [ ] **Step 4: At soak close, write the sprint-result memory entry**

When the soak closes green, capture in `.claude/memory/`:

- `project_epr_foundation_completion_landed_<date>.md` — sprint-result; what landed, what surprised, what shifted in the master plan.
- Update `MEMORY.md` index with the new entry.

Capture the non-obvious discoveries — e.g., the W1/W2A/W2B-KeyRotation "plan-tracking debt vs implementation debt" pattern, the W2D `signals.rs::handle_recovery_v2_signal` dead-path salvage, any contract renegotiations on `revocation-attestation.schema.json` (per D2 duality, should be zero).

If the soak does NOT close green: capture the regression shape, dispatch a remediation sub-plan, and re-soak.

---

## Self-Review

**Spec coverage:**
- Kickoff Wave 1 (substantive) — W2D `RecoveryFlowProjector` + `derive_compromise_at` + sibling co-create: **Tasks 5 + 6**.
- Kickoff Wave 1 (tail) — federation-aggregate `resilience_cliffs` + doc-stale TODO cleanup: **Tasks 3 + 4**.
- Kickoff Wave 1 (debt cleanup) — tick W2A + W2B-KeyRotation: **Tasks 1 + 2**.
- Kickoff Wave 2 (narrowed) — RevocationAttestation arm + conditional AgentPeerBinding: **Tasks 7 + 8**.
- Kickoff Wave 5 (a2o @wip lift, 17 scenarios): **Tasks 9 + 10** (9 + 8 scenarios respectively).
- Kickoff Wave 6 (1-week soak): **Task 11**.
- Kickoff Wave 3 + 4 (already landed / deferred): not new tasks, marked in the wave-status table in the kickoff (no task needed).
- D1 (audit-only, Wave 0 done): no task — pre-condition.
- D2 (sibling RecoveryFlowProjector): Task 5 implements.
- D3 (duality binds M4 D2): Task 7 implements per slim payload.
- D4 (defer GetDocument): out-of-scope, noted in File Structure.
- D5 (Phase 12 runtime check): Task 8 Step 1.

**Placeholder scan:** No "TBD", "TODO later", or "fill in details". Tasks 5 and 6 use `todo!()` macros INSIDE example code blocks as documented stubs for body-parsing logic that is to be ported verbatim from `signals.rs:1040–1145` — the porting instruction is explicit, not a placeholder.

**Type consistency:**
- `handle_content_signal(conn: &mut SqliteConnection, signal: &ElohimContentSignal) -> Result<(), StorageError>` — used consistently in Task 5 (sibling projector matches `AttestationProjector`).
- `derive_compromise_at(db_pool: Option<&Arc<DbPool>>, revocation_id: &str, effective_at_fallback: DateTime<Utc>) -> DateTime<Utc>` — signature preserved in Task 6.
- `EprAtomResponse::IntegrityAck { received: bool, reason: Option<String> }` — Task 7 and Task 8 both produce this shape; matches existing arms.
- `RecoveryRotationMessage` / `RecoveryRevocationMessage` / `RevocationAttestationMessage` / `AgentPeerBindingMessage` — all named consistently, all use camelCase struct-as-map MessagePack via `rmp_serde::to_vec_named` + `from_slice`. Task 7's wire type names exactly mirror the existing pattern.
- `compute_resilience_cliffs(&mut conn, agent_cid)` — Task 3 swap matches the local-resolver call shape at `peer_topology_view.rs:185–189`.

No type drift detected.

---

## Plan complete.
