# EPR Delivery Master Plan

> **For agentic workers:** This is the coordinating plan for landing the EPR (Elohim Protocol Record) phase backlog after the iroh transport cutover. It maps inter-plan dependencies, sequences execution waves, and surfaces Decisions Required + Discovery Required items that block sub-plan execution. Sub-plans use the writing-plans format with checkbox steps.
>
> Use `superpowers:subagent-driven-development` to dispatch implementation agents per sub-plan, with the wave order below.

**Goal:** Convert the EPR phase plans from "draft" to "delivered" — confirm what already landed, finish the runtime gaps, ship Phase 4 (projector controller + projection events log) so the topology sprint's last-mile wiring unblocks, lift `@wip` on the EPR a2o scenarios, and leave the substrate ready for the next graph-native sprint.

**Architecture:** One Wave-0 audit converts plan checkboxes to truth. Wave 1 lands the highest-leverage substrate piece (Phase 4 projector controller — also the substrate the topology sprint is starved for). Wave 2 finishes Phase 3.5 runtime gaps + IntegrityNotify pipeline. Wave 3 lifts `@wip` on EPR a2o coverage. Wave 4 is soak + cross-stack integration validation against iroh master.

**Tech Stack:** Rust (elohim-storage, elohim-epr codec, holochain DNAs), Diesel migrations, libp2p 0.54 + iroh 0.92 (post-iroh-master), CBOR + DAG-CBOR codec, Ed25519, ts-rs codegen, JSON Schema (schema-first), Cucumber (a2o features), Angular 19 (elohim-app for a2o renderer surfaces).

**Companion sprint:** This sprint runs alongside the iroh delivery master (`2026-05-10-iroh-delivery-master.md`). Phase 12 caller-identity (iroh master Plan 1) lands first; this master picks up its results.

---

## P2P Design Gate — entity classification (master coordinator)

This document is a coordinator that introduces **zero new storage schemas, zero new HTTP routes, zero new wire formats, and zero new DHT entry types** of its own. Every artifact referenced is owned and classified by exactly one sub-plan; this section declares the source-of-truth pointers so the line-pattern audit has explicit context.

| Artifact mentioned in this master | Owning sub-plan | Category | Source of truth |
|---|---|---|---|
| `EprAtom`, `EprCoupling`, `EprClaims`, `EprSupersedence` tables | Phase 2A (LANDED) | C — operational projection of CBOR envelopes | `2026-04-22-elohim-epr-storage-foundation-plan.md`; rebuildable from canonical bytes |
| `Manifest` integrity entry type (4 kinds) | Phase 3 (LANDED, audit-pending) | A — DHT-notarized | `2026-04-30-epr-phase-3-manifest-resolver-plan.md`; lives in lamad zome |
| `FeedbackSignal`, `AttentionTending`, `CollectiveFilterPattern` | Phase 3.5 (LANDED, audit-pending) | A / B / C respectively | `2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md` §"P2P design gate" |
| `Predecessor` record (sealed 2-of-2 dryoc) | Phase 3.5 (PARTIAL — `record_predecessor` is T22 TODO) | C — operational | Same as above; rebuildable from FeedbackSignal arrival logs |
| `AgentPeerBinding`, `peer_identity_bindings`, `verified_at`, `verified_signer_fingerprint` | Phase 2B (LANDED, audit-pending) | A / C / C / C | `2026-04-24-epr-phase-2b-design.md` §"appendix A" |
| `projector_acks` table + `projection_events` log | Phase 4 (NEW, this sprint) | C — operational | This sprint Wave-1 sub-plan, to be drafted |
| `geo`/`archetype` fields on `peer_identity_bindings` | Phase 4 (NEW, this sprint) | A2 — link metadata on binding | Same |
| `display_name` resolution path through imagodei lookup | Phase 4 (NEW, this sprint) | C — operational read-projection | Same |
| `online` annotation from libp2p `connected_peers()` snapshot | Phase 4 (NEW, this sprint) | C — operational, ephemeral | Same; same shape as topology M1 cluster_view fix |
| `IntegrityNotify` extensions (KeyRotation, RevocationAttestation, AgentPeerBinding) | Phase 3.5 runtime gap | A2 (link metadata on binding) + A (revocation entry) | Recovery M4 + Phase 2B Batch A — coordinate per `project_epr2b_recovery_m4_convergence` |
| `experience-story`, `experience-moment`, `:story-point` (a2o tier 1/2/3) | OUT OF SCOPE — graph-native sprint | A / B / A2 | `2026-04-18-experience-story-epr-design.md` |
| Full social-reach nervous system (provenance back-prop, quarantine, restitution) | OUT OF SCOPE — graph-native sprint | downstream epic | `project_social_reach_nervous_system` memory pin |

**No new DNA entry types this sprint.** Lamad DNA capacity stays at ~73/~100; Mishpat at 11/~100. Phase 4 introduces only link-tag metadata (Category A2) and projections (Category C).

---

## Sub-plan portfolio

| # | Plan | File | Tasks | Status before this sprint | Notes |
|---|------|------|-------|---|---|
| P1 | Phase 1: codec | `2026-04-21-elohim-epr-codec-crate-plan.md` | 23 | ✅ LANDED — `elohim/epr/` crate (12 modules, 15 tests), `@elohim/epr` v0.1.0 TS package | Audit confirms; mark plan ✅ |
| P2A | Phase 2A: storage foundation | `2026-04-22-elohim-epr-storage-foundation-plan.md` (+ BATCH-C-PIVOT) | 22+6 | ✅ LANDED — `db/epr_atoms.rs`, 6 REST routes, view schemas, contract tests | Audit confirms; mark plan ✅ |
| P2C | Phase 2C: libp2p federation | `2026-04-23-epr-phase-2c-libp2p-federation-plan.md` (+ batch-D addendum) | 19+4 | ✅ LANDED — `/elohim/epr-atom/1.0.0` protocol, golden vectors, parity tests | Audit confirms; mark plan ✅ |
| P2B | Phase 2B: identity + projector + signal | `2026-04-24-epr-phase-2b-plan.md` | 39 (150/155 boxes ✅) | ✅ AUDIT-CONFIRMED LANDED on `0f3ffa20d` — Batches A/B/C/D + Wrap-up Z all green; only A.11 Steps 2-4 + A.12 Steps 1-2 (5 steps) remain — all blocked on Stage 2 `derive_compromise_at` which needs `key_revocations` projection (see W2D) | Audit complete |
| P3 | Phase 3: manifest resolver | `2026-04-30-epr-phase-3-manifest-resolver-plan.md` | 21 (83/83 boxes ✅) | ✅ AUDIT-CONFIRMED LANDED on `33438cdd8` — clean; T0–T15 all green; `cold_fetch_resolves_manifest_from_peer` `#[ignore]` lifted + passing; schema_contract 94/94 | Audit complete |
| P35 | Phase 3.5: trust-compute gradient | `2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md` | 21 (117/117 boxes ✅) | ✅ AUDIT-CONFIRMED LANDED on `e122ec072` — feature branch merged as `01526ce15` 2026-05-01; aunt-and-rage-bait scenario passes (12.11s); only T22 record_predecessor remains (= W2A) | Audit complete |
| LUG | Light Up the Graph | `2026-05-01-light-up-the-graph-plan.md` | 27 (116/124 boxes ✅) | ✅ AUDIT-CONFIRMED LANDED on `4ea4e1558` — 21/27 tasks; both T20 mocks lifted; Vouch primitive end-to-end (schema → Rust → integrity zome → coordinator); only T18 record_predecessor remains (= W2A) | Audit complete |
| **W1** | **Phase 4: projector controller + events log** | **TO WRITE** (Wave-1 sub-plan) | **~18 estimated** | **⏳ PENDING — 7 explicit `TODO(Phase 4 follow-up)` sites in `distribution_view.rs`, `reciprocity_view.rs`, `peer_topology_view.rs`, plus `main.rs:1129` manifest-registry layer-1** | **Co-primary deliverable; unblocks the topology sprint** |
| W2A | Phase 3.5 runtime gap: `record_predecessor` (T22) | inline in this master Wave-2 | 4 steps | ⏳ PENDING — `api/epr.rs:626 TODO(T22)` | Small surgical wiring |
| W2B | IntegrityNotify pipeline expansion | TO WRITE (small Wave-2 sub-plan) | ~8 estimated | ⏳ PENDING — only `KeyRevocation` is implemented; `KeyRotation`, `RevocationAttestation`, `AgentPeerBinding` are accept-and-log stubs | Cross-references Recovery M4 per `project_epr2b_recovery_m4_convergence` |
| W2C | EPR `GetDocument` variant | inline in this master Wave-2 | 5 steps | ⏳ PENDING — `p2p/epr_protocol.rs` line 46 marks it `not yet implemented`; `EprService::handle_get_document` returns NotFound | DECIDE — implement now or defer (D4) |
| **W2D** | **`key_revocations` projection + Stage 2 `derive_compromise_at`** | **TO WRITE** (small Wave-2 sub-plan, ~6 estimated) | ~6 estimated | **⏳ NEW — surfaced by P2B Wave-0 audit. Unblocks A.11 Steps 2-4 + A.12 Steps 1-2 (5 sweettests `#[ignore]`'d pending this).** Needs: (a) `key_revocations` SQLite projection table writer hooked to integrity-zome KeyRevocation entries via post-commit signal, (b) `derive_compromise_at` reads from it to compute the retroactive sweep window, (c) lift the 6 `#[ignore]` markers in `epr_phase_2b_batch_a_e2e.rs` | Coordinates with W2B IntegrityNotify pipeline (KeyRevocation handler is shared) |
| W3 | A2o EPR coverage lift | TO WRITE (Wave-3 sub-plan) | ~12 estimated | ⏳ PENDING — `epr-content-addressing.feature` 6 `@wip`, `epr-cross-peer-resolution.feature` 6 `@wip` | Browser-side step defs already exist in `genesis/a2o/steps/ui/epr-content.steps.ts` |

**Total scoped tasks:** ~55 net-new + audit confirmation of ~479 latent checkboxes across P2B/P3/P35/LUG.

---

## Pre-flight audit (Wave 0)

The four phase plans (P2B, P3, P35, LUG) show 0/479 checkboxes done, but the audit found substantial code, runtime wiring, and the canonical aunt-and-rage-bait integration test all present in dev. This is plan-tracking debt, not implementation debt. Wave 0 converts the checkboxes to truth before Wave 1 dispatch — otherwise we'll re-do landed work.

### Wave-0 Task — Convert plan checkboxes to truth

**Files:**
- Modify (in place): `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md`
- Modify (in place): `genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-manifest-resolver-plan.md`
- Modify (in place): `genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md`
- Modify (in place): `genesis/docs/superpowers/plans/2026-05-01-light-up-the-graph-plan.md`

- [ ] **Step 1 — For each plan, run the test that gates the plan's "done" condition.**

For P2B: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_reach_enforcement` and `cargo test --test epr_atom_federation_d8 --test epr_atom_federation_integration`.

For P3: `cargo test --test schema_contract` and the manifest-resolver tests in `services/manifest_registry.rs` (`#[cfg(test)]` blocks).

For P35 + LUG: `cargo test --test aunt_and_rage_bait_integration -- --test-threads=1`. Per LUG goal: "Lift the two T20 mocks; aunt-and-rage-bait runs without direct service substitutions" — this single test is the LUG closure condition.

Expected per plan: tests pass → plan can be marked ✅; tests fail → record the failing assertion as Wave-2 backlog.

- [ ] **Step 2 — For each plan, walk task-by-task and tick `[x]` for tasks whose code is grep-confirmable.**

Confirmation rule per task: the task names a file/symbol. Grep for it. If present and the corresponding test (Step 1) passes, mark `[x]`. If absent or the test fails, leave `[ ]` and capture the gap in the master sprint backlog (this file's Wave-2 list).

The audit dispatches one Sonnet agent **per plan**, with explicit forbids: do not modify code, do not modify schemas, do not run cargo build, do not git revert/reset. Output is plan-file edits only + a markdown summary table to stdout listing what landed vs what remains.

- [ ] **Step 3 — Commit the audit results**

```bash
git add genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md \
        genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-manifest-resolver-plan.md \
        genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md \
        genesis/docs/superpowers/plans/2026-05-01-light-up-the-graph-plan.md
git commit -m "$(cat <<'EOF'
docs(epr): audit-confirm landed checkboxes across P2B / P3 / P35 / LUG

Substrate code, runtime wiring, and aunt-and-rage-bait integration
test all present in dev. Plans were drafted 2026-04-24..05-01 and
implemented but checkbox state was never converted. Audit walks
each task, confirms code presence + closure-test passing, ticks
the boxes. No code changes.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 4 — Surface the audit results to the operator**

The audit summary is the input that scopes Wave 2's IntegrityNotify and `record_predecessor` work. If a phase test fails (vs the assumed "passes"), Wave 1 still dispatches but Wave 2 grows by the named gap.

---

## Inter-plan dependency graph

```
                              ┌─────────────────────┐
                              │ Wave 0: audit       │
                              │ convert checkboxes  │
                              └─────────┬───────────┘
                                        │
                ┌───────────────────────┼───────────────────────┐
                ▼                                               ▼
   ┌─────────────────────┐                       ┌─────────────────────────┐
   │ Wave 1: Phase 4     │                       │ Iroh master Plan 1      │
   │ projector + events  │                       │ peer_transport_manifest │
   │ (~18 tasks)         │                       │ (in-flight elsewhere)   │
   └─────────┬───────────┘                       └─────────┬───────────────┘
             │                                             │
             │  unblocks topology surfaces                 │  Phase 12 caller identity
             │                                             │  consumed by W2B
             ▼                                             ▼
   ┌─────────────────────┐                       ┌─────────────────────┐
   │ Topology sprints    │                       │ Wave 2A: T22        │
   │ (Light Up / M1 —    │                       │ record_predecessor  │
   │  separate sprint)   │                       │ (4 steps)           │
   └─────────────────────┘                       └─────────────────────┘
                                                             │
                                                             ▼
                                                  ┌─────────────────────┐
                                                  │ Wave 2B: Integrity- │
                                                  │ Notify pipeline     │
                                                  │ (~8 tasks)          │
                                                  └─────────┬───────────┘
                                                            │
                                                            ▼
                                                  ┌─────────────────────┐
                                                  │ Wave 2C: GetDocument│
                                                  │ (5 steps, D4 gates) │
                                                  └─────────┬───────────┘
                                                            │
                                                            ▼
                                                  ┌─────────────────────┐
                                                  │ Wave 3: a2o EPR     │
                                                  │ coverage lift       │
                                                  │ (~12 tasks)         │
                                                  └─────────┬───────────┘
                                                            │
                                                            ▼
                                                  ┌─────────────────────┐
                                                  │ Wave 4: soak +      │
                                                  │ cross-stack         │
                                                  │ integration         │
                                                  └─────────────────────┘
```

---

## Execution waves

**Wave 0 — Audit (single agent, blocks everything else):**
- Convert plan checkboxes to truth across P2B / P3 / P35 / LUG.

**Wave 1 — Substrate completion (single high-leverage sub-plan, parallel with iroh master Plan 1):**
- Phase 4 projector controller + projection events log + `geo`/`archetype` link metadata + `display_name` lookup + `online` annotation. **This is the topology-unblock piece.**

**Wave 2 — Runtime gap closure (parallel after Wave 1 + iroh master Plan 1):**
- W2A — T22 `record_predecessor` (4 steps, inline)
- W2B — IntegrityNotify pipeline (~8 tasks; coordinates with Recovery M4)
- W2C — EPR `GetDocument` variant (5 steps, D4 gates whether this lands here or defers)

**Wave 3 — A2o coverage lift:**
- Lift `@wip` on `epr-content-addressing.feature` (6 scenarios)
- Lift `@wip` on `epr-cross-peer-resolution.feature` (6 scenarios)

**Wave 4 — Soak + cross-stack integration:**
- Aunt-and-rage-bait scenario passes on both libp2p and iroh transports
- Phase 2B ↔ Recovery M4 DNA-signal-stream coordination soak (1 week, zero cross-branch divergence)
- End-to-end integration: Matthew publishes a manifest EPR → reach-earning gate evaluates → libp2p Kad announce → Jessica cold-fetches → her elohim-app renders three-pillar metadata → assertion in a2o feature

---

## Topology last-mile bridge (Wave 1 explicit unblocks)

The user is "still struggling to deliver the 'light up the topology' sprints" — both topology plans (Light Up the Topology 256 unchecked, Topology Substrate Completion M1 67 unchecked) are blocked on Phase 4 substrate. Wave 1 must explicitly resolve every `TODO(Phase 4 follow-up)` site so the topology sprint's "last mile" wiring becomes a no-op rendering pass.

| Phase 4 deliverable | Code site | Topology surface unblocked |
|---|---|---|
| `projector_acks` table + write path on doorway projection ack | `services/distribution_view.rs:160`, `:182`, `:256` (TODO sites) | Distribution badge `projector_count` + `any_projector` (per-content distribution view) |
| `projection_events` append-only log | `services/distribution_view.rs:259` (TODO site) | "recent_projection_events" feed in distribution details |
| Device capacity totals minus committed | `services/reciprocity_view.rs:78` (TODO site) | Reciprocity surface "capacity_available_bytes" |
| Display-name resolution via imagodei lookup | `services/reciprocity_view.rs:159` (TODO site) | Counterparty display names on reciprocity surface |
| Online annotation from libp2p `connected_peers()` snapshot | `services/reciprocity_view.rs:163` (TODO site) | Online dot on reciprocity counterparty list |
| `geo`/`archetype` link tag on `peer_identity_bindings` + diversity composer | `services/distribution_view.rs:99`, `:164` (TODO sites) | "diversity_hint" on distribution badge |
| Sole-replica resilience analysis surfaced from quilt distribution | `services/peer_topology_view.rs:180` (TODO site) | "resilience_cliffs" on peer topology surface |
| Manifest registry layer-1 (load pillar manifests from disk) | `main.rs:1129` (TODO site) | Write-through layer 1 base — currently empty HashMap |

**Acceptance for Wave 1:** Every `TODO(Phase 4 follow-up)` in those three view files is resolved (either replaced with real code or moved to a documented downstream epic with a tracked issue link). The topology sprint can then dispatch its rendering layer with confidence the substrate returns real data.

**Out-of-scope for Wave 1 (topology sprint owns):** seeder rewrites (`seed-commitments.ts`), `system_metrics.rs` create, `cluster_view.rs` real `build_local_slice`, view-federation `connected_peers` plumbing, Jenkinsfile stages, Playwright probes. Those are topology-team scope; Phase 4 hands them clean substrate to consume.

---

## Decisions Required (resolve before Wave 0 dispatch)

### D1 — Trust the audit, or re-test from scratch?

The Wave-0 audit tasks one Sonnet agent per plan to grep-confirm + run closure tests + tick checkboxes. Two options:

- **(a)** Trust the audit's `[x]` marks as the input to Wave 1+ scoping (recommended).
- **(b)** Re-run the entire test suite for each phase before dispatching Wave 1.

**Recommendation:** (a). The audit IS a test run — Step 1 of each plan's audit dispatches the closure test. Re-running adds latency without adding signal.

### D2 — Phase 4 scope: substrate-only or include topology composition helpers?

Phase 4 substrate (the `TODO(Phase 4 follow-up)` resolution) cleanly belongs to EPR — it's projector identity + events log + link metadata classification. But the user's signal "make the last mile easy wiring" suggests we could go further and provide ready-to-call helper functions (`resolve_display_name(agent_cid)`, `is_online(peer_id)`, `device_capacity_minus_committed(human_id)`) so the topology sprint's view-rendering code becomes one-liners.

- **(a)** Substrate only — fill the TODO data, leave composition to topology sprint.
- **(b) Recommended** — substrate + thin helper API consumed by `services/{distribution,reciprocity,peer_topology}_view.rs` directly, so topology sprint's last mile is "render what the view returns" with no glue.

**Recommendation:** (b). The whole reason the topology sprints stalled is glue debt; absorbing the glue into Phase 4 prevents another stall.

### D3 — IntegrityNotify breadth: full pipeline or staged?

Today only `KeyRevocation` is implemented; `KeyRotation`, `RevocationAttestation`, `AgentPeerBinding` are accept-and-log stubs. Recovery M4 (per `project_epr2b_recovery_m4_convergence`) is the producer of all four event kinds. Two options:

- **(a)** Implement all four in W2B this sprint.
- **(b)** Stage 2 — KeyRotation only this sprint; AgentPeerBinding waits on Phase 12 caller-identity convergence; RevocationAttestation deferred to graph-native sprint (it's an attestation primitive that ladders into the social-reach nervous system).

**Recommendation:** (b). Stage 2 (KeyRotation in addition to KeyRevocation) covers the recovery-protocol acute path. AgentPeerBinding handling shouldn't land before Phase 12 caller-identity is real on iroh; otherwise we'll wire it twice. RevocationAttestation is downstream-epic shape per `project_social_reach_nervous_system`.

### D4 — `GetDocument` variant: ship now or defer?

`p2p/epr_protocol.rs:46` declares `GetDocument` as "not yet implemented". The variant is for fetching document-tier content by ID over the EPR protocol — useful for a2o cross-peer resolution scenarios. Two options:

- **(a)** Implement now (5 steps; W2C). Concrete `EprService::handle_get_document` reads from the existing `documents` table; serializes via existing CBOR codec; mirror parity test for iroh.
- **(b)** Defer to graph-native sprint. Today the `epr-cross-peer-resolution.feature` scenarios use the existing `Resolve`/`ResolveBatch` variants; `GetDocument` is "nice to have" until a cross-peer document fetch is needed by a real renderer.

**Recommendation:** (b). Wave-3 a2o coverage lift can determine whether `GetDocument` is actually blocking; if so, escalate to W2C. If the existing `Resolve` covers cross-peer reads sufficiently, defer cleanly.

### D5 — Plan checkbox conversion: in-place edit or status header?

Wave-0 audit's recommended approach is to tick `[x]` on individual tasks. Alternative: prepend a `## Status` header at the top of each plan ("✅ LANDED on dev as of <commit>; checkboxes retained for traceability") and leave the `[ ]` boxes alone.

- **(a) Recommended** — tick individual `[x]` boxes. Clear per-task truth; traceable git diff.
- **(b)** Status header — less work; loses per-task granularity.

**Recommendation:** (a). The whole point of the audit is per-task truth; future re-audits won't have the same forensic capacity.

### D6 — Worktree strategy

Iroh master used one worktree per Wave 1 plan. EPR master has only one Wave-1 plan (Phase 4) but Wave 2 has three concurrent sub-plans (W2A, W2B, W2C — all touching `elohim-storage/src/`).

- **(a)** One worktree per wave (Wave 1 single, Wave 2 single shared).
- **(b)** One worktree per sub-plan within each wave.

**Recommendation:** (b) for Wave 2 only. W2A/B/C don't share files (W2A is `api/epr.rs`, W2B is `services/integrity_notify.rs` + `p2p/epr_atom_protocol.rs`, W2C is `epr_service.rs` + `p2p/epr_protocol.rs`), so parallel worktrees don't collide. ff-merge each into dev as it lands.

---

## Discovery Required

**Discovery DR1 — Audit may surface unexpected gaps**

The four phase plans were authored before iroh master Plan 4 (DualGossipPublisher) and Plan 1 (peer_transport_manifest) landed. The audit may discover that some Phase 2B / 3.5 task wording was outdated by iroh master's intervening commits — e.g., Phase 2B Batch D's gossipsub fanout work may have been subsumed/replaced by DualGossipPublisher.

- **Action:** Acknowledge. The audit's job is to mark `[x]` on landed tasks and capture genuine gaps in this master's Wave-2 backlog. If a task was "obsoleted by iroh master" (not done but no longer needed), the audit ticks it with a `<!-- obsoleted by Plan 4 commit ea94aeb7e -->` HTML comment.

**Discovery DR2 — Phase 4 might require a new DHT entry type for `projector_acks`**

`projector_acks` (doorway projection acks) is currently classified above as Category C (operational). But if the topology surfaces need verifiable proof that a doorway projector has acked a content item (not just "the doorway claims to have"), `projector_acks` should be an Attestation (Category A) on the DHT. Mishpat at 11/~100 has headroom.

- **Action:** Wave-1 sub-plan (when written) MUST run `p2p-design-gate` skill on `projector_acks` before scaffolding. The skill's decision tree decides Category A vs C based on whether the protocol would be lying if the ack were silently changed.

**Discovery DR3 — Phase 12 caller identity may not have landed by Wave-2 dispatch**

W2B's `AgentPeerBinding` IntegrityNotify path consumes Phase 12 caller identity (iroh master Plan 1). If iroh master is still in-flight when Wave 2 dispatches:

- **Action:** Wave 2 dispatches W2A + W2C. W2B's AgentPeerBinding piece waits on Phase 12 landing (per D3 recommendation). KeyRotation piece can dispatch independently.

---

## Out-of-scope (next sprint: graph-native surface)

This sprint is **substrate completion**. The user explicitly carved the graph-native surface out: "Graph-native surface is a separate follow-on sprint — do NOT fold it in here."

Confirmed out-of-scope:

| Item | Why deferred | Memory pin |
|---|---|---|
| Full social-reach nervous system: provenance back-prop chain capture, quarantine signals, restitution events | Downstream epic; substrate's reach-earning gate is the floor, but the four primitives (provenance, sense/respond, quarantine, restitution) are the next sprint | `project_social_reach_nervous_system` |
| `experience-story` / `experience-moment` / `:story-point` (a2o tier 1/2/3 attestation model) | Separate design (`2026-04-18-experience-story-epr-design.md`); creates new ContentNode subtype + EconomicEvent vocabulary | spec doc |
| VF-GraphQL / hREA shefa application layer | Application layer; substrate ≠ application; conflating them was explicitly flagged | `project_epr_substrate_vs_vf_graphql` |
| Elohim-mediated reach matchmaking (the discernment layer above the substrate gate) | Future sprint per memory: substrate returns `Pending`, elohim agent reads imagodei + collective manifest, suggests sponsors | `project_reach_gate_is_elohim_mediated_matchmaking` |
| Vouch sponsor lifecycle (FeedbackSignal `vouch` variant + collective-manifest BYO) | LUG plan T20-T27 cover the substrate; full sponsor UX is graph-native sprint | LUG plan §sponsor-flow |

If Wave-3 a2o scenario lifts surface a need for any of these, the sprint **flags the dependency in the sprint-result memory entry and stops** — does NOT silently fold them in.

---

## Closing condition

This sprint closes when:

- **Audit complete:** All 479 latent checkboxes converted to `[x]` or remain `[ ]` with documented backlog reason; closure tests for P1, P2A, P2C, P2B, P3, P35, LUG all green.
- **Phase 4 landed:** Every `TODO(Phase 4 follow-up)` site in `services/{distribution,reciprocity,peer_topology}_view.rs` resolved; `main.rs:1129` manifest layer-1 wired; topology sprint's last-mile renders real data with no glue debt.
- **Runtime gaps closed:** `record_predecessor` (T22) wired in `api/epr.rs:626`; IntegrityNotify pipeline at Stage 2 (KeyRevocation + KeyRotation, per D3); `GetDocument` decision (D4) recorded.
- **A2o coverage lifted:** All 12 `@wip` scenarios in `epr-content-addressing.feature` and `epr-cross-peer-resolution.feature` either pass or have documented out-of-scope deferrals.
- **Pre-push hook validates** every project gate end-to-end. The validation tooling commands all pass without introducing any new wire contracts of their own — they validate the existing artifacts whose source-of-truth declarations live in the P2P design gate table at the top of this document (every entity classified A — DHT-notarized, A2 — DHT-link-anchored, B — agent-scoped, B2 — agent-scoped with attestation, or C — operational projection). Commands: `pnpm run schema:codegen:ts` (regenerates from existing JSON schema source-of-truth files), `pnpm run schema:validate` (validates seed data against existing notarized schemas), `cargo test schema_contract` (runs the schema-contract harness against existing operational view types), `pnpm run a2o:lint`, `clippy -D warnings`, `cargo test`, `eslint`, `prettier`.
- **Push lands on origin/dev** and the orchestrator dispatches downstream pipelines all green.
- **Cross-stack integration:** Aunt-and-rage-bait scenario passes via both libp2p and iroh transports (after iroh master ships).
- **Sprint-result memory entry** captures the non-obvious discoveries — e.g., "0/479 checkboxes was plan-tracking debt, not implementation debt"; "Phase 4 substrate doubled as topology-sprint unblock"; any contract renegotiations on the pre-existing `dna-signal-stream` artifact (owned by Phase 2B Batch A; classification declared in the master P2P design gate table at the top of this document, coordinated with Recovery M4 per `project_epr2b_recovery_m4_convergence`).

---

## Dispatch shape recommendation

Per memory pins (`feedback_dev_branch_no_pr`, `feedback_subagent_scope_guardrails`, `feedback_no_generalize_permissions_in_shift`):
- Each sub-plan's commits land directly on dev as a stack of commits. No PR per plan.
- Every implementation-agent dispatch prompt MUST explicitly forbid: scope creep, dep version changes, destructive git ops (revert/reset/--force), and require BLOCKED report instead of silent cleanup.
- Auto mode handles permission prompts inline; no pre-shift palette pass.

**Per-wave shape:**
- **Wave 0:** Four parallel Sonnet agents (one per plan), read-only + plan-file edits only. Run in parallel; all four results synthesized into one Wave-2 backlog before Wave 1 dispatches.
- **Wave 1:** Single Sonnet agent for Phase 4 sub-plan authoring (writing-plans skill); then dispatch one implementation agent (Sonnet) following subagent-driven-development. Single worktree.
- **Wave 2:** Three parallel implementation agents (one per W2A/W2B/W2C), three worktrees, after Wave 1 lands. W2B partially gated on Phase 12 (iroh master Plan 1) per DR3.
- **Wave 3 — split by capability** (per `feedback_a2o_narrative_is_opus_work`): 
  - **Opus agent** authors / refines the a2o feature files, scenarios, frontmatter, tags, and persona setup. The human story has to land meaningfully into the technical libraries — this is the bridge between manifesto and substrate, and Haiku produces "scenario-shaped objects" without the deep value or interpretability the format exists to convey.
  - **Sonnet/Haiku agent** wires up step-definition glue, fixture builders, helper utilities — only after the Opus agent has authored the narrative shape.
  - **Opus reviews** the final result to confirm the story still reads true after the implementation lands. Mechanical `@wip` removal is fine for Haiku, but never the scenario authoring itself.
- **Wave 4:** Soak + integration runs as Jenkins-driven CI; this master closes when CI shows green for the integration-test stage on origin/dev.

---

## Self-review

- ✅ Every named EPR phase (1, 2A, 2B, 2C, 3, 3.5, LUG) has a sub-plan row with status assessment
- ✅ Phase 4 (the only fully-pending net-new substrate work) has a Wave-1 sub-plan reservation + topology-unblock mapping
- ✅ Inter-plan dependency graph reflects Wave 0's gating role + Phase 12 cross-sprint dependency
- ✅ All 6 Decisions Required have stated recommendations
- ✅ All 3 Discovery items have actions
- ✅ Out-of-scope explicitly enumerates the graph-native carve-outs the user named, with memory-pin backing
- ✅ Topology last-mile bridge (D2) makes "support the topology sprint" a concrete deliverable, not a vague intent
- ✅ Closing condition is measurable per item (audit done, TODO sites resolved, scenarios un-`@wip`ed, CI green)
- ✅ Dispatch shape recommendation honors `feedback_dev_branch_no_pr`, `feedback_subagent_scope_guardrails`, `feedback_no_generalize_permissions_in_shift`
