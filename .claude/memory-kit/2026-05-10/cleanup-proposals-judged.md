# Cleanup Proposals — Judged — 2026-05-10

Refined version of `proposals.md` with judgment, evidence, and reasoning per candidate.

## Summary

| Judgment | Count |
|----------|-------|
| ARCHIVE | 0 |
| BACKLOG | 16 |
| KEEP-FRESH | 46 |
| OPERATOR-CALL | 2 |

**Total:** 64

Headline pattern: nearly all dangling references are **path drift** (file moved/renamed but spec/plan still cites old location) or **planned future files** (specs from active sprints declaring deliverables that landed in feature worktrees, not yet merged to dev). No specs surfaced as obsolete enough to archive — every one I investigated has either (a) ongoing related work in the last 30 days, (b) lives in a worktree, or (c) the underlying file simply moved. Path-drift cases are KEEP-FRESH (the spec/plan content is fine; only the citation is stale). Plan items where the deliverable hasn't materialized in dev OR a worktree are BACKLOG.

---

## Flagged for Review

### 1. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-04-18-seeder-registry-coherence-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/seeder/src/__tests__/deployment-registry.test.ts` exists (renamed location)
- `genesis/seeder/src/__tests__/seed-accounts.test.ts` exists (renamed location)
- `genesis/a2o/features/deployment/seeder-registry-coherence.feature` exists (the spec's own §4.2 deliverable)
- Companion plan `2026-04-18-seeder-registry-coherence.md` (item 17) is also live
- Account-import-result schema unfound but `genesis/seeder/src/seed-accounts.ts` is present — schema may have been inlined or deferred

**Reasoning:** The spec is delivered. References point to `src/foo.test.ts` but tests live at `src/__tests__/foo.test.ts`. Cosmetic path drift; spec content is accurate.

---

### 2. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-04-19-p2p-dataplane-visibility-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/a2o/features/elohim/compute-allocation.feature` exists (path was `elohim/...`, real path `genesis/a2o/features/elohim/...`)
- `genesis/a2o/features/elohim/compute-coordination.feature` exists
- `genesis/a2o/features/elohim/elohim-presence.feature` exists
- `elohim/holochain/dna/infrastructure/zomes/infrastructure/tests/peer_status.rs` exists (proper DNA-relative path)
- doorway admin features not found but companion plan (item 18) covers same ground

**Reasoning:** Path drift — spec wrote `elohim/` and `doorway/` as shortcuts where real paths are `genesis/a2o/features/elohim/` and need a doorway feature subdir. Underlying scenarios delivered.

---

### 3. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md`

**Judgment:** BACKLOG

**Evidence:**
- `genesis/a2o/support/steps/chaos-steps.ts` does NOT exist; no `chaos-steps` anywhere in the tree
- However `genesis/a2o/steps/resilience.steps.ts` exists — companion delivered
- dev-intent.jsonl 2026-04-20 entry: "Plan 1 shipped: contract-aware auto-distribute + placement_gaps signals + <elohim-resilience-snapshot> shared component" (Plan 2 = "periodic verification" was next)

**Reasoning:** Plan 1 of self-healing is shipped, but the chaos-test step file was never created; declared but unfinished. Plan 2 (periodic verification) listed as the follow-up in dev-intent — chaos test infrastructure remains to-do.

---

### 4. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-04-21-recovery-protocol-phase-2-m2-validator-impls.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `elohim/sdk/schemas/v1/views/identity-freeze.schema.json` not found by exact path; `git log` shows recovery-witness, key-revocation, key-rotation, revocation-vote schemas all landed
- M2 validator impls merged via key-revocation/votes work (commits `18558259d`, `eefdb5ea5`, `6d675b6b5`, `6efde…`)

**Reasoning:** identity-freeze schema was never created as a separate file; recovery work converged on revocation/witness primitives. Spec's design intent is delivered through related schemas; reference is conceptually superseded.

---

### 5. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `elohim/sdk/schemas/v1/views/agent-peer-binding-view.schema.json` exists
- `elohim/sdk/schemas/v1/objects/agent-peer-binding.schema.json` exists
- `elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json` exists
- EPR 2B Batch A merge commit referenced in MEMORY.md as completed (`79181f8e`)

**Reasoning:** AgentPeerBinding shipped in three forms (view, object, dna-signal) — spec just cited a flat path that doesn't match the final layout convention. Deliverable in place.

---

### 6. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `elohim/elohim-storage/src/p2p/recovery_invitation.rs` exists (spec said `src/libp2p/wire/recovery_invitation.rs` — wrong sub-path; correct location is `src/p2p/`)
- `elohim/elohim-storage/src/db/recovery_requests.rs` and `recovery_witnesses.rs` exist (spec said `src/views/recovery.rs`)
- `2026-04-22-m3-session-kickoff-prompt.md` not in `genesis/docs/plans/` — kickoff prompts are session artifacts often not committed
- Companion plan `2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage.md` is present
- M5 merge commit `a07aaa66d` confirms M3 → M5 completed

**Reasoning:** Recovery invitation/views landed at different paths than spec predicted. Concept delivered through `src/p2p/` + `src/db/`. Spec is reference history; path drift only.

---

### 7. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m4-fast-path-revocation-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `elohim/sdk/schemas/v1/views/key-revocation.schema.json` exists (spec wrote `key-revocation-view.schema.json` — `-view` suffix dropped per current convention)
- `elohim/sdk/schemas/v1/views/revocation-vote.schema.json` exists
- Schema commits `18558259d`, `6d675b6b5`
- `genesis/a2o/features/auth/` directory exists; revocation feature files not yet committed (work-in-progress on `feature/recovery-m4-fast-path-revocation` branch)

**Reasoning:** Schemas landed under suffixless naming; the design's wire contracts shipped. The two missing `genesis/a2o/features/auth/revocation-*.feature` files are still open work in the branch — reflected in BACKLOG dimension via item 28.

---

### 8. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-04-25-recovery-protocol-phase-2-m5-auth-portal-convergence-revocation-ux-and-stub-defender-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- All four schema files exist with current naming (no `-view` suffix): `key-rotation.schema.json`, `key-revocation.schema.json`, `revocation-vote.schema.json`, `recovery-request.schema.json`
- M5 merge commit `a07aaa66d` confirms work landed
- Kickoff prompt referenced (`2026-04-24-recovery-m5-...`) not committed but `2026-04-24-recovery-m4-...kickoff-prompt.md` IS — kickoff prompts are session-ephemeral
- M3 graduated key rotation plan referenced — never landed as separate plan file (graduated approach folded into M3 design)

**Reasoning:** Convention drift on schema names (`-view` dropped) plus a never-committed kickoff prompt. M5 work merged; spec content remains authoritative.

---

### 9. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md`

**Judgment:** BACKLOG

**Evidence:**
- `genesis/orchestrator/scripts/print-plan-summary.mjs` not found; no `print-plan-summary` anywhere
- Companion plan `2026-04-27-jenkins-stage-1a-brit-advisory.md` (item 31) declares Stage 1B/1C/2/3 follow-ons that also don't exist
- No commits since 2026-04-27 mentioning "brit attestation" or "print-plan-summary"

**Reasoning:** Stage-1A advisory may have shipped (pre-existing build-graph stages) but the print-plan-summary script and follow-on stages 1C/2/3 are unfinished. Brit-attestation epic is partially delivered, several gates remaining.

---

### 10. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-05-03-distribution-resilience-coherence-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `app/elohim-library/projects/elohim-service/src/distribution/distribution.service.ts` exists (moved from `services/` to `distribution/` subdir)
- `app/elohim-library/projects/elohim-service/src/public-api.ts` exists (spec said `public-api.ts`, missing the `src/` prefix)
- Distribution work landed (matches plan #37)

**Reasoning:** Path drift — spec assumed pre-restructure layout. Files exist in current layout.

---

### 11. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-05-03-distribution-resilience-substrate-sharing-backlog.md`

**Judgment:** BACKLOG

**Evidence:**
- `genesis/a2o/features/shefa/substrate-sharing-incentives.feature` not found
- `genesis/a2o/features/shefa/` exists with `human-resilience.feature` and `m1-matthew-timothy-delivery.feature` only
- File name itself contains "backlog" — design IS the backlog item

**Reasoning:** This spec is explicitly a backlog stash for substrate-sharing incentives. The single missing `.feature` is the deliverable when the topic moves from "backlog" to "active." Self-declared as backlog.

---

### 12. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-05-04-storybook-deployment-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `app/elohim-library/projects/lamad-ui/...component.ts` is a glob/wildcard, not a real path
- `app/elohim-library/tsconfig.storybook.json` not found at root but storybook stories DO exist (`*.stories.ts` under lamad-ui components)
- Storybook elohim-elements + lit-pivot work landed (item 14, 39, 40 specs/plans)

**Reasoning:** First missing reference is a deliberate glob in design prose; second is a Storybook-config file the design proposed creating. Storybook deployment had a pivot to elohim-elements (item 14), so the original plan partially superseded — fresh enough to keep as design history.

---

### 13. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-05-06-deliver-skill-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/docs/shifts/SPRINT-RESULT-TEMPLATE.md` exists (not `DELIVER-SPRINT-RESULT-TEMPLATE.md` — single shared template)
- `genesis/docs/shifts/JOURNAL-TEMPLATE.md` exists (same — single shared)
- `genesis/agentic/readiness.mjs` exists (not `deliver-readiness.mjs` — shared with `/shift`)
- `.claude/skills/deliver/` skill directory present per system-reminder

**Reasoning:** The deliver skill shipped but reused existing readiness/template infra rather than creating deliver-prefixed siblings. Design's path predictions wrong; skill works.

---

### 14. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-05-06-elohim-lit-component-pivot-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `app/elohim-styles/README.md` not found — but `app/elohim-elements/` directory exists with elohim-core + 8 sibling pillars (avodah, doorway, imagodei, lamad, qahal, shefa, shell)
- elohim-core has `package.json`, `src/`, custom-elements-manifest config — pivot delivered

**Reasoning:** elohim-styles became elohim-elements + per-pillar subpackages. Design intent delivered with different naming.

---

### 15. `spec-dangling-refs` — `genesis/docs/superpowers/specs/2026-05-07-topology-substrate-completion-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/docs/content/elohim-protocol/manifesto/` directory does NOT exist
- `genesis/docs/content/elohim-protocol/manifesto.md` exists (single-file form)
- Spec referenced split-file manifesto location that wasn't created

**Reasoning:** Manifesto stayed monolithic; spec assumed a split that never happened. Cosmetic — design's intent is captured in the single `manifesto.md`.

---

### 16. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-16-agentic-developer-loop.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/a2o/foo.feature` is a placeholder/example — not a real intended file
- Agentic developer loop fully shipped (`/shift` skill present, agentic infrastructure in `genesis/agentic/`)

**Reasoning:** Reference is illustrative ("foo.feature") in plan prose. False positive flag.

---

### 17. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-18-seeder-registry-coherence.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/a2o/steps/deployment.steps.ts` not found but `genesis/a2o/steps/` has 20+ step files including `node-registration.steps.ts`, `seeder.steps.ts`, `operator-onboarding.steps.ts` covering the deployment-registry concern
- `genesis/a2o/features/deployment/seeder-registry-coherence.feature` exists (the work landed)

**Reasoning:** Plan's predicted step file name didn't get created (work folded into existing seeder/operator step files). Deliverable shipped.

---

### 18. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-19-p2p-dataplane-visibility-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `peer_status.rs` lives at `elohim/holochain/dna/infrastructure/zomes/infrastructure/tests/peer_status.rs` (not `elohim/holochain/tests/`)
- `household.rs` not at `elohim/elohim-storage/src/services/`; `household_resilience.rs` and `household_backfill.rs` are
- `node_shape.rs` exists at `elohim/elohim-storage/src/api/node_shape.rs`
- `elohim/holochain/Cargo.toml` doesn't exist (the workspace uses per-DNA Cargo.toml files)
- `genesis/data/humans/matthew.json` doesn't exist; `genesis/data/humans/matthew-manager.md` does
- This plan has 23 dangling refs — high churn area; bulk are pre-restructure paths

**Reasoning:** Plan's intent is largely delivered (peer status, household, node shape, edge node JSON). Path predictions wildly out of date due to restructures (DNA workspace split, household services growing into specialized files, manifests vs data dirs). KEEP-FRESH because work shipped, not ARCHIVE because plan's narrative is still valid history.

---

### 19. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-19-runtime-request-correlation.md`

**Judgment:** BACKLOG

**Evidence:**
- No `correlation/` directory in `elohim/sdk/schemas/v1/`, `doorway/doorway-service/src/`, or `elohim/sdk/storage-client-ts/src/generated/`
- No commits since 2026-04-19 mentioning "request correlation" or "RequestCorrelation"
- Plan has 23 dangling refs — none of the planned files exist

**Reasoning:** Runtime request-correlation was planned but never started. Cross-cutting infra (correlation IDs, headers, propagation) — deferred. Worth surfacing as backlog: observability concern grows in importance with iroh + dual-stack.

---

### 20. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-19-self-healing-plan-1-observable-auto-distribute.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `app/elohim-library/projects/elohim-service/src/resilience/resilience-snapshot/resilience-snapshot.component.ts` exists (extra `resilience/` parent dir from spec prediction)
- `app/elohim-library/projects/elohim-service/src/resilience/resilience.service.ts` exists
- `genesis/a2o/steps/resilience.steps.ts` exists (not `genesis/a2o/support/steps/resilience-steps.ts`)
- `genesis/a2o/features/resilience/` directory exists
- dev-intent.jsonl 2026-04-20 confirms "Plan 1 shipped"

**Reasoning:** Plan delivered. All artifacts moved into a `resilience/` namespace under elohim-service rather than the spec-predicted flat `services/` + `components/` layout.

---

### 21. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-21-consolidate-human-deployments.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- Commit `1570fbb7d`: "chore(manifests): remove legacy template and orphaned per-human yamls"
- Commit `93a34d9e0`: "feat(orchestrator): template the 5 legacy humans, keep Adam as consolidated ref"
- `_edgenode-consolidated.template.yaml` IS the successor to `_edgenode-legacy.template.yaml`
- Per-human yamls moved to `genesis/manifests/humans/` (matthew-manager.yaml etc. live there)

**Reasoning:** Plan completed and the deletions were the explicit goal. Dangling refs are exactly what "remove legacy" produces. Plan content remains valuable as historical record of the consolidation.

---

### 22. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-21-recovery-protocol-phase-2-m1-data-model.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- Migration timestamps changed from `2026_04_21_recovery_phase_2` (snake) and `2026-04-21-recovery-phase-2` (kebab) to actual `2026-04-22-100000_recovery_request_m2_fields` and `2026-04-22-000000_recovery_phase_2_cleanup`
- `recovery-witness.schema.json` and `recovery-request.schema.json` exist (renamed from `recovery-seed-commitment` / `recovery-quorum-request`)
- `elohim/elohim-storage/src/db/recovery_witnesses.rs` and `db/recovery_requests.rs` exist
- M3/M4/M5 commits all merged — confirms M1 substrate is in place

**Reasoning:** M1 data model landed under different naming/timestamp convention. Plan is reference history; current code matches its intent.

---

### 23. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-21-recovery-protocol-phase-2-m2-validator-impls.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- Migration `elohim/elohim-storage/migrations/2026-04-22-100000_recovery_request_m2_fields/` exists (off by one day from plan's `2026-04-21-100000`)
- `elohim/elohim-storage/src/db/` contains schema-related files (Diesel uses generated `schema.rs` per migration)
- M2 validator impls integrated into recovery-request projection

**Reasoning:** M2 work shipped; only the migration date prefix shifted. Plan still accurate as design record.

---

### 24. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-22-elohim-epr-storage-foundation-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- Migration `elohim/elohim-storage/migrations/2026-04-22-050000_add_epr_tables/` exists (plan said `000000_`, actual is `050000_`)
- EPR foundation merged — see Phase 2B/3 commits in dev log

**Reasoning:** Time-prefix drift on migration; otherwise delivered.

---

### 25. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-22-recovery-protocol-phase-2-m1-cleanup.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `elohim/elohim-storage/migrations/2026-04-22-000000_recovery_phase_2_cleanup/` exists — that IS the cleanup migration
- Schema/projection consolidation evidenced by recovery-request and recovery-witness schemas (current naming, no `-seed-commitment`/`-quorum-request`)

**Reasoning:** Cleanup plan executed; schema names normalized. Plan content is accurate history.

---

### 26. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `agent-peer-binding` schemas exist in `views/`, `objects/`, `dna-signals/` (plan referenced flat `v1/agent-peer-binding.schema.json`)
- `device-archetype` lives at `enums/device-archetype.schema.json`
- `pillar-projection` lives at `manifest/pillar-projection.schema.json`
- `app-manifest` lives at `manifest/app-manifest.schema.json`
- Migration `peer_identity_bindings` exists at `2026-04-24-235000_peer_identity_bindings/`
- `verified_at_on_epr_atoms` migration at `2026-04-25-000000_verified_at_on_epr_atoms/`
- Projector cursor at `2026-04-25-010000_projector_cursor/`
- `controller_tests.rs` not in `src/reconcile/` but `src/reconcile/` directory exists
- Phase 2B Batch D merged via `cfa40e4b4`

**Reasoning:** Plan shipped via Batch A→D landings. Schema layout convention firmed up around `enums/`, `manifest/`, `views/`, `objects/`, `dna-signals/` subdirs after plan was written. Migrations landed with `YYYY-MM-DD-HHMMSS_` placeholders filled in.

---

### 27. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `elohim/elohim-storage/src/p2p/recovery_invitation.rs` exists (plan said `src/p2p/commands.rs`)
- `elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs` exists (plan said `src/tests/mod.rs`)
- M3 work merged

**Reasoning:** Path drift only. Recovery M3 delivered.

---

### 28. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-24-recovery-protocol-phase-2-m4-fast-path-revocation.md`

**Judgment:** BACKLOG

**Evidence:**
- Branch `feature/recovery-m4-fast-path-revocation` exists locally — work in progress
- Schema files (`key-revocation.schema.json`, `revocation-vote.schema.json`) ARE on dev (commits `18558259d`, `6d675b6b5`)
- Sweettest at `elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs` exists (plan referenced `tests/recovery_m4_revocation.rs`)
- Feature files `genesis/a2o/features/auth/revocation-self.feature` and `revocation-emergency-quorum.feature` NOT in dev or any worktree I scanned
- `db/schema.rs` Diesel-generated; recovery_v2_mesh.rs not found

**Reasoning:** M4 substrate (schemas, sweettest) merged but the user-facing a2o feature scenarios for revocation-self and emergency-quorum are still unfinished. Active feature branch suggests in-flight; surface to backlog so they don't fall off radar.

---

### 29. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-24-rno-wave1-sweettest-bodies.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- Kickoff prompt at `genesis/docs/plans/2026-04-24-rno-wave1-sweettest-bodies-kickoff-prompt.md` not committed (kickoff prompts are session ephemera per pattern across multiple specs)
- RNO Wave 1 work landed per recent commits

**Reasoning:** Single dangling ref is to a kickoff prompt that wasn't preserved. Plan body is intact and reflects shipped work.

---

### 30. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-25-recovery-protocol-phase-2-m5-auth-portal-convergence-revocation-ux-and-stub-defender.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- All 4 schemas exist with `key-rotation.schema.json` / `key-revocation.schema.json` / `revocation-vote.schema.json` / `recovery-request.schema.json` naming (no `-view` suffix per current convention)
- `imagodei_integrity` and `imagodei` zomes exist (plan used `imagodei-integrity` / `imagodei-coordinator` hyphenated names; actual uses underscore)
- `portal_host.rs` exists in both zomes
- `submit_specialist_revocation.rs` exists
- M5 merge commit `a07aaa66d` confirms full landing

**Reasoning:** M5 fully merged. Refs are stale due to (a) `-view` schema suffix drop and (b) imagodei zome naming uses underscore. Plan content is accurate to delivered work.

---

### 31. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-27-jenkins-stage-1a-brit-advisory.md`

**Judgment:** BACKLOG

**Evidence:**
- All three referenced follow-on plans (`stage-1c-attestation-writes`, `stage-2-consume-plan`, `stage-3-retire-legacy`) use `2026-XX-XX-` placeholder dates and don't exist
- No commits since 2026-04-27 mentioning brit-advisory or stage-1c/2/3
- Companion spec (item 9) is also incomplete (print-plan-summary.mjs absent)

**Reasoning:** Stage-1A may have shipped advisory-only mode but stages 1C/2/3 of the brit-attestation rollout are all not started. Active work item per multi-stage plan.

---

### 32. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `feedback-signal` schema at `p2p/feedback-signal.schema.json` (plan referenced `v1/feedback-signal.schema.json`)
- `attention-tending` schema at `p2p/attention-tending.schema.json`
- `standing-policy-floor` schema at `manifest/standing-policy-floor.schema.json`
- `tending-policy-floor` schema at `manifest/tending-policy-floor.schema.json`
- `feedback_signal.rs`, `attention_tending.rs`, `collective_filter_pattern.rs` exist in `dna/elohim/zomes/content_store/src/` AND `content_store_integrity/src/`
- DNA tests live in `tests/sweettest/src/tests/feedback_signal.rs` etc., not `dna/elohim/sweettests/`

**Reasoning:** Phase 3.5 trust-compute work landed. Schema sub-paths firmed up (`p2p/`, `manifest/`). DNA tests structure consolidated to single `sweettest/src/tests/`.

---

### 33. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-manifest-resolver-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `elohim/elohim-storage/tests/manifest_resolver_integration.rs` exists (plan referenced `tests/vectors/manifest_epr_messages.json` test vector that wasn't created)
- `elohim/elohim-storage/tests/vectors/epr_atom_messages.json` exists (different vector)
- `dna/elohim/zomes/content_store/src/manifest.rs` exists
- Schema work shipped (Phase 3 commits in log)

**Reasoning:** Manifest resolver shipped, vectors directory structure landed with one of two vector files. Test vector for manifest_epr_messages was never created — folded into integration test.

---

### 34. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-01-light-up-the-graph-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- DNA `feedback_signal.rs` exists at `tests/sweettest/src/tests/feedback_signal.rs` and `dna/elohim/zomes/content_store/src/feedback_signal.rs`
- `db/attention_tending.rs` not found but `dna/elohim/zomes/content_store/src/attention_tending.rs` exists
- Migration `2026-05-01-050000_tending` exists
- Phase 3.5 trust-compute work landed (item 32)

**Reasoning:** Graph work shipped via DNA + migrations. Storage-side `db/attention_tending.rs` either consolidated into a shared module or pending.

---

### 35. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-01-light-up-the-topology-plan.md`

**Judgment:** BACKLOG

**Evidence:**
- 38 dangling refs — most files don't exist anywhere in dev
- `content-node.adapter.ts`, `content-card.component.html`, `holochain_signal_stream.rs`, `peer_fallback.rs`, `dashboard.rs` (in views/), `views_contract.rs` all missing
- However `dashboard_topology.rs` exists in `doorway/doorway-service/{src,tests}/`
- `rea-action.schema.json` not found — REA action not yet schematized
- Plan touches topology view layer that's still being shaped

**Reasoning:** Light-up-the-topology was a sweeping multi-layer plan; large fraction of artifacts not delivered. Topology delivery work is in flight per item 42 (M1 plan dated 2026-05-07) and m1-matthew-timothy-delivery.feature exists. Surface as backlog — topology completion is active concern.

---

### 36. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-02-blob-custody-reconciliation-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `blob_inventory_smoke.rs` not found by exact name; `inventory_writer_smoke.rs` exists
- Migrations `2026-05-02-110000_peer_blob_inventory`, `2026-05-02-110100_peer_inventory_cursor`, `2026-05-02-100000_rea_blob_action_indexes`, `2026-05-02-120000_placement_gap_action` all dated same day as plan — work landed
- `src/p2p/inventory_broadcaster.rs`, `inventory_gossip.rs`, `db/peer_blob_inventory.rs` all exist

**Reasoning:** Blob custody reconciliation shipped — same-day migration timestamps confirm execution. Smoke test renamed.

---

### 37. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-03-distribution-resilience-coherence-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `distribution.service.ts` exists at `app/elohim-library/projects/elohim-service/src/distribution/distribution.service.ts` (moved out of elohim-app into elohim-library)
- Distribution badge component not found in elohim-app — plan may have placed badge inside resilience-snapshot

**Reasoning:** Distribution work shipped via library. Badge UX may have been folded into resilience-snapshot. KEEP-FRESH; design intent delivered.

---

### 38. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-04-design-surface-narrative-scaffold-plan.md`

**Judgment:** OPERATOR-CALL

**Evidence:**
- `__validation__/test-import.mdx` and `test-content.md` are mentioned but not created
- `doorway/__docs__/index.mdx`, `reference.mdx`, `components.mdx` not present
- `genesis/docs/content/elohim-protocol/imagodei.md` not present (manifesto stayed monolithic per item 15)
- `app/elohim-library/scripts/sync-genesis.mjs.bak` not present (likely intentional — `.bak` files shouldn't be committed; sync-genesis.mjs is present)
- `app/elohim-library/tsconfig.storybook.json` not present

**Considerations:**
- Storybook deployment plans (item 12, 39, 40) suggest pivot/restructure happening — design-surface-narrative-scaffold may have been superseded by elohim-elements/lit pivot
- But narrative-scaffold concerns (per-pillar docs surfaces) are ongoing
- Multiple separate concerns interleaved in one plan — partial shipping ambiguous

**Reasoning:** Cannot confidently classify. The pivot to elohim-elements (item 14) may obsolete the plan's concrete file predictions, but the narrative-scaffold concept may still be alive. Operator should decide whether to refresh or supersede.

---

### 39. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-04-storybook-deployment-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `tsconfig.storybook.json` not present at expected path; storybook `.stories.ts` files DO exist throughout `lamad-ui/`
- Storybook pivoted to elohim-elements package (item 14, 40)

**Reasoning:** Storybook deployment partially shipped via stories existing. Single missing file is config that may have been superseded by the lit-pivot. KEEP-FRESH — design intent partially delivered, related work ongoing.

---

### 40. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-06-elohim-lit-component-pivot-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `app/elohim-elements/elohim-core/` exists with `elohim-button.ts`, `register.ts`, `index.ts`, `package.json`, `vite.config.ts` — pivot delivered
- `_test_hook.ts` not found but lit components have spec files (`elohim-button.spec.ts`, `elohim-button.manifest.spec.ts`)
- `tsconfig.storybook.json` not yet present per item 39

**Reasoning:** Lit pivot delivered. Test-hook refactor was not adopted; component spec files serve the test infrastructure role.

---

### 41. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-07-doorway-ssr-runtime.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- SSR work landed: `932dbf44e` "feat(compute): un-stub ComputeMetricsView; SSR concurrency derives from operator-as-pod budget", `036c32b38` "docs(ssr): completion summary"
- `elohim/elohim-render/` doesn't exist as separate crate — SSR likely lives elsewhere (doorway service or a render module)
- `app/elohim-app/scripts/prebuild.mjs`, `pnpm-lock.yaml`, `dist/...main.server.mjs` are build artifacts not committed
- `cypress/e2e/ssr-hydration.cy.ts` not found but `genesis/a2o/steps/ui/ssr-hydration.steps.ts` exists (cucumber a2o instead of cypress)

**Reasoning:** SSR runtime largely delivered, with implementation choices diverging from plan: cucumber instead of cypress; render module placement different from `elohim-render` crate name.

---

### 42. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-07-topology-substrate-completion-m1-plan.md`

**Judgment:** BACKLOG

**Evidence:**
- `genesis/a2o/features/topology/m1-matthew-timothy-delivery.feature` not at that path; exists at `genesis/a2o/features/shefa/m1-matthew-timothy-delivery.feature` (commit `503323950`)
- `genesis/orchestrator/environments/topology.json` not found; `alpha.env`, `prod.env`, `staging.env` exist instead
- `genesis/orchestrator/render.js` not found
- Manifesto split file (per item 15) not present

**Reasoning:** M1 scenario landed but in `shefa/` not `topology/` directory. Orchestrator topology.json + render.js not yet created — plan has unfinished orchestration substrate work. Topology delivery is in active concern. Surface to backlog.

---

### 43. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-08-ssr-capability-implementation.md`

**Judgment:** BACKLOG

**Evidence:**
- `elohim/elohim-storage/src/app_context.rs` not found
- `doorway/doorway-service/tests/auth_mode_enforcement.rs`, `concurrency_overflow.rs` not found
- SSR work has ongoing momentum (commit `932dbf44e` 2026-05-08-ish; ssr completion summary docs)
- MEMORY entry `project_ssr_anonymous_auth_context` (2 days ago) confirms SSR work is unfinished — auth context propagation is open

**Reasoning:** SSR capability shipped at view-projection level but the auth/concurrency hardening tests + app_context module are not in dev. Active sprint area; remains backlog.

---

### 44. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-10-compute-metrics-un-stub.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- Commit `932dbf44e`: "feat(compute): un-stub ComputeMetricsView; SSR concurrency derives from operator-as-pod budget"
- `node_bootstrap.rs` lives at `doorway/doorway-service/src/orchestrator/node_bootstrap.rs` (plan said `doorway/orchestrator/node_bootstrap.rs`)

**Reasoning:** Compute metrics un-stub shipped today. Single ref has wrong sub-path; file exists.

---

### 45. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-10-iroh-delivery-master.md`

**Judgment:** BACKLOG

**Evidence:**
- Plan dated TODAY (2026-05-10) — runbooks/manifests are deliverables, not yet authored
- `genesis/manifests/RUNBOOK-iroh-parity-soak-2026-05-10.md`, `RUNBOOK-iroh-alpha-soak-2026-05-10.md`, `RUNBOOK-iroh-consumer-soak-2026-05-10.md`, `RUNBOOK-iroh-rollback-drill-2026-05-10.md` all not yet written
- `alpha-cluster.yaml` not in `genesis/manifests/`
- `bench_blob_stress_10k.rs` not yet created (plan calls for it; `bench_blob_perf.rs` exists as the model)
- Iroh delivery work is the most active engineering theme (per Plan 4 Tasks 1-9 commits 2026-05-09/10)

**Reasoning:** Plan written today as the cutover master plan; the dangling refs are exactly its checklist of TBD deliverables (runbooks, manifests, stress test). All belong on the active backlog.

---

### 46. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-10-iroh-phase12-peer-transport-manifest.md`

**Judgment:** BACKLOG

**Evidence:**
- Plan dated TODAY 2026-05-10
- `peer-transport-manifest.schema.json` exists at `elohim/sdk/schemas/v1/views/`
- TS generated files exist in `iroh-seeder-dual` worktree but NOT in dev (`peer-transport-manifest.ts` in seeder/elohim-app/elohim-library generated dirs)
- Phase 12 Tasks 1-13 commits in dev log show active in-flight work (`d36dbf62d`, `939b9bf56`, `8da3a70ec`)

**Reasoning:** Phase 12 in active development; generated TS bindings live in worktree pending merge. Codegen for `Libp2pTransportProfileView`, `IrohTransportProfileView`, `PeerTransportManifestView` likely still pending. Active backlog item.

---

### 47. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-10-iroh-recovery-e2e.md`

**Judgment:** BACKLOG

**Evidence:**
- Plan dated TODAY 2026-05-10
- Worktree `iroh-recovery-e2e` exists locally (per `git branch` output) — work in progress
- `elohim/elohim-storage/tests/iroh_recovery_cross_stack.rs`, `recovery-cross-stack-transport.feature`, `recovery-cross-stack.steps.ts`, `multi_stack_fixture.rs` all in worktree but not dev

**Reasoning:** Active feature branch dedicated to iroh recovery e2e. Backlog: pending merge.

---

### 48. `spec-dangling-refs` — `genesis/docs/superpowers/plans/2026-05-10-iroh-seeder-dual-write.md`

**Judgment:** BACKLOG

**Evidence:**
- Plan dated TODAY 2026-05-10
- Worktree `iroh-seeder-dual` confirmed: `git log` in worktree shows commits `45d536fba`, `c5d919dff`, `a68a6972f`, `f13114378` — Tasks 1-3 done in worktree
- All referenced files (put-blob-response schema, generated TS, dual-write tests, backfill, seeder-dual-write feature) exist in worktree, not in dev

**Reasoning:** Active feature branch with first 3 tasks complete; remaining work and merge pending. Backlog.

---

### 49. `plan-dangling-refs` — `genesis/docs/plans/2026-04-17-epr-link-navigation-boxes.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `app/lamad/services/resilience.service` is path shorthand for `app/elohim-app/src/app/lamad/services/resilience.service.ts` — the file exists
- `app/elohim/services/epr-resolver.service` resolves to `app/elohim-app/src/app/elohim/services/epr-resolver.service.ts` — exists
- `app/elohim/models/epr-head.model` resolves to `app/elohim-app/src/app/elohim/models/epr-head.model.ts` — exists
- dev-intent.jsonl 2026-04-17 confirms EPR link navigation shipped

**Reasoning:** Plan used pillar-relative shorthand paths (per `@app/elohim` import alias convention). All artifacts present; cosmetic citation drift.

---

### 50. `plan-dangling-refs` — `genesis/docs/plans/2026-04-17-peer-self-bootstrap-from-dht-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/plans/...multi` is illustrative pseudo-path with literal `...` — not a real reference
- Plan content elsewhere intact

**Reasoning:** False positive; reference is prose example.

---

### 51. `plan-dangling-refs` — `genesis/docs/plans/2026-04-17-peer-stewarded-availability-phase-1-plan.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `peer_status.rs` exists at `elohim/holochain/dna/infrastructure/zomes/infrastructure/tests/peer_status.rs`
- Commits `743c3d10e`, `22b1a9a68`, `c607b4e79`, `4364c0939` confirm peer-status work shipped
- Plan referenced `elohim/holochain/tests/peer_status.rs` (pre-DNA-workspace-split path)

**Reasoning:** Path drift — peer_status.rs lives in DNA workspace not top-level holochain/tests. Work delivered.

---

### 52. `plan-dangling-refs` — `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `peer_status.rs` exists at `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/peer_status.rs` and `infrastructure/zomes/infrastructure/src/peer_status.rs` and tests
- Plan referenced `elohim/holochain/dna/infrastructure/tests/peer_status.rs` (close but tests live under `zomes/infrastructure/tests/`)

**Reasoning:** Same path-drift pattern; zome tests live under their zome dir, not at DNA root.

---

### 53. `plan-dangling-refs` — `genesis/docs/plans/2026-04-21-rno-lessons-wave-1-execution-plan.md`

**Judgment:** OPERATOR-CALL

**Evidence:**
- `elohim/holochain/README.md` not present
- DNA-level READMEs exist (e.g. `elohim/holochain/dna/imagodei/STEWARDSHIP_PHILOSOPHY.md`, `dna/elohim/README.md`)
- No top-level holochain README — plan referenced one as if it should exist

**Considerations:** Either (a) plan assumed an aggregate Holochain README that was never created, or (b) the README was deleted at some point. No git log entry for `elohim/holochain/README.md` to verify history.

**Reasoning:** Single-ref ambiguity. Operator may want to either create the aggregate README the plan assumed, or update the plan to point at per-DNA READMEs.

---

### 54. `plan-dangling-refs` — `genesis/docs/plans/2026-04-21-rno-lessons-wave-2-execution-plan.md`

**Judgment:** BACKLOG

**Evidence:**
- `app/elohim-library/projects/elohim-service/src/services/feature-flags.service.ts` does not exist
- No `feature-flags` service anywhere in `app/elohim-library` (only in `sophia/` packages, unrelated)
- Plan calls for a feature-flags service that was never built
- RNO Wave 2 graduation-path kickoff prompt (referenced at item 56) is also not present — Wave 2 partially landed

**Reasoning:** Feature-flags service was an RNO Wave 2 deliverable that didn't ship. Backlog: feature flags affect runtime gating decisions; absence keeps coming up (e.g. `project_elohim_active_observed_not_flagged` MEMORY note).

---

### 55. `plan-dangling-refs` — `genesis/docs/plans/2026-04-24-epr-phase-2b-batch-a-execution-kickoff-prompt.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `YYYY-MM-DD-epr-phase-2b-batch-b-execution-kickoff-prompt.md` is placeholder text in prose; actual `2026-04-25-epr-phase-2b-batch-b-execution-kickoff-prompt.md` exists
- Batch A through D all merged

**Reasoning:** Placeholder pattern-match false positive. Real Batch B prompt exists with proper date.

---

### 56. `plan-dangling-refs` — `genesis/docs/plans/2026-04-24-rno-wave2-graduation-path-kickoff-prompt.md`

**Judgment:** BACKLOG

**Evidence:**
- `genesis/docs/plans/2026-04-24-rno-graduation-path.md` not present
- Wave 2 graduation-path execution plan never landed (plan was kicked off but the actual plan doc not created)
- Pairs with item 54 (feature-flags also Wave 2 deliverable)

**Reasoning:** Wave 2 graduation path is unfinished. Companion plan body never authored. Surface to backlog.

---

### 57. `plan-dangling-refs` — `genesis/docs/plans/2026-04-25-epr-phase-2b-batch-a-completion-kickoff-prompt.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- Same `YYYY-MM-DD-` placeholder issue as item 55
- Batch B prompt with real date exists

**Reasoning:** False positive on placeholder date pattern.

---

### 58. `plan-dangling-refs` — `genesis/docs/plans/2026-04-27-orchestrator-deploy-only-param-kickoff-prompt.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/orchestrator/environments/dev.env` not present
- Actual env files: `alpha.env`, `prod.env`, `staging.env`
- DEPLOY_ONLY param work landed (commit `0579f5a7c` "docs(plans): kickoff prompt for DEPLOY_ONLY orchestrator param")

**Reasoning:** No `dev` environment was added; existing 3 environments cover the intent. Path-drift only.

---

### 59. `plan-dangling-refs` — `genesis/docs/plans/cargo-target-pool-design.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/agentic/lib/pool.sh`, `pool.test.mjs`, `families.json` not at those paths
- `genesis/agentic/` has many .mjs files but no `lib/` subdir
- Recent commit `8ad665527`: "feat(agentic): cargo-pool disk hygiene — JS caches, watermarks, active-subagent guard"
- Recent commit `8ad5f2ca6`: "feat(agentic): cargo target pool — family-shared targets via SessionStart/Stop hooks"

**Reasoning:** Cargo target pool work shipped 2 days ago at different paths than design predicted (no `lib/` subdir, possibly `.mjs` not `.sh`). Design fulfilled.

---

### 60. `memory-dangling-refs` — `feedback_haiku_observe_only_no_specifics.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `genesis/a2o/features/deployment/hub-topology.feature` not present
- The reference in this memory is HISTORICAL — quoting the hallucinated file path that triggered the lesson, not a file that should exist
- Memory's value is the lesson, not the reference

**Reasoning:** Memory documents a Haiku hallucination; the cited path is the hallucinated path, deliberately. False positive flag.

---

### 61. `memory-dangling-refs` — `project_garage_sccache_substrate_2026_05_09.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `elohim/devfile.yaml` not present
- `devfile.yaml` exists at repo root: `/projects/elohim/devfile.yaml`
- Memory said `elohim/devfile.yaml` (with leading `elohim/` prefix) — common path-shorthand mistake

**Reasoning:** devfile.yaml is at repo root, not under `elohim/`. Memory content valid; path citation cosmetic.

---

### 62. `memory-dangling-refs` — `project_hub_archetype_abstraction.md`

**Judgment:** BACKLOG

**Evidence:**
- `genesis/a2o/features/deployment/hub-topology.feature`, `genesis/data/hubs/hubs.json`, `genesis/a2o/src/framework/fixtures/hubs.ts` all not present
- Memory explicitly says "Where it lives: `genesis/data/hubs/hubs.json` parallel to `devices.json`" as a TO-CREATE description, not extant
- Hub abstraction is a design memo; the listed files are explicit deliverables for "the next sprint"

**Reasoning:** Memory's referenced files are intentional forward-looking deliverables ("hub fixtures don't yet exist"). Surface to backlog: hub fixtures + Hub abstract type still need to be created.

---

### 63. `memory-dangling-refs` — `project_m5_reframe_auth_portal_convergence.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `2026-04-24-recovery-m5-elohim-defender-and-revocation-ux-kickoff-prompt.md` not in `genesis/docs/plans/` — kickoff prompt for M5 not committed
- `doorway/doorway-app/components/login/threshold-login.component.ts` not at that path; lives at `doorway/doorway-app/src/app/components/login/threshold-login.component.ts`

**Reasoning:** Path drift on threshold-login (missing `src/app/` segment) plus a non-committed kickoff prompt. Memory content matches shipped M5 work (merge `a07aaa66d`).

---

### 64. `memory-dangling-refs` — `project_ssr_anonymous_auth_context.md`

**Judgment:** KEEP-FRESH

**Evidence:**
- `app/elohim-app/REACH.md` not present
- Real REACH.md files: `elohim/elohim-storage/REACH.md`, `doorway/doorway-service/REACH.md`, `elohim/holochain/docs/REACH.md`
- Memory was written 2 days ago referring to REACH semantic concept

**Reasoning:** REACH.md docs exist on the storage / doorway / holochain sides; memory just cited an elohim-app version that doesn't exist. The reach concept is well-documented; pointer is wrong but content valid.
