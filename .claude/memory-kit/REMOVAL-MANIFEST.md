# REMOVAL MANIFEST — Guard-Filtered (D-phase, gated LAST)

> **NOTHING HAS BEEN REMOVED.** This file is a *review artifact* — a list only. Run the blocks below
> only after (A) curated-history records, (B) canonical seeds, and (C) inline pointers have landed.
> Source of truth: `burndown-proposal.md` §D.0–§D.15 + `UNIFIED-cleanup-review.md` §4.

## CRITICAL EXECUTION NOTE — two removal verbs

- **Groups 1–3 (junk drawers + pile bodies)** are **git-tracked** → use **`git rm` / `git rm -r`**.
- **Group 4 (shift artifacts)** lives under `.claude/shifts/`, which is **gitignored** (`.gitignore:10-12`,
  only `.gitkeep` is tracked). These files are **untracked working-tree copies** — `git rm` will error.
  Use plain **`rm` / `rm -r`**. Git history preserves nothing here except the historical commits that
  introduced any tracked ancestor; the working-tree copies are the only live form, so this is a true delete.
  (The burndown framed all of D as `git rm`; the shift sub-tier is the one exception — flagged here so the
  command actually runs.)

---

## (1) JUNK DRAWERS — whole dirs (git-tracked: 8 + 56 + 13 = 77 files)

```bash
# D.0 — after C1 (2026-05-14 precedent pointer) + the 4 A.1 records (2026-05-15) land
git rm -r .claude/archive/2026-05-14
git rm -r .claude/archive/2026-05-15
git rm -r .claude/archive/2026-06-01
```

## (2) PILE DEAD-ARCH BODIES (git-tracked: 25 paths)

```bash
# D.1 — EPR codec & storage foundation (after A.2.1 record). rno-guidance retires under D.6 (counted once there).
git rm genesis/docs/superpowers/plans/2026-04-21-elohim-epr-codec-crate-plan.md
git rm genesis/docs/superpowers/plans/2026-04-22-elohim-epr-storage-foundation-plan.md
git rm genesis/docs/superpowers/plans/2026-04-22-elohim-epr-storage-foundation-plan-BATCH-C-PIVOT.md
git rm genesis/docs/superpowers/plans/2026-04-23-epr-phase-2c-libp2p-federation-plan.md
git rm genesis/docs/superpowers/plans/2026-04-24-epr-phase-2c-batch-d-completion-addendum.md
git rm genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md
git rm genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md
git rm genesis/docs/superpowers/plans/2026-05-11-epr-w2a-record-predecessor-plan.md
git rm genesis/docs/superpowers/plans/2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md
git rm genesis/docs/superpowers/plans/2026-05-16-epr-foundation-closure.md
git rm genesis/docs/plans/2026-04-26-epr-phase-3-manifest-resolver-kickoff-prompt.md
git rm genesis/docs/plans/2026-05-15-epr-foundation-completion-post-attestation-kickoff-prompt.md
git rm genesis/docs/plans/2026-05-15-epr-w2b-resumption-handoff.md
git rm genesis/docs/plans/2026-05-16-epr-wip-disposition.md

# D.2 — Light Up the Topology / Graph (after A.2.2 record).
#   NOTE: 2026-05-19-topology-resilience-qahal-synthesis.md is the CONFLICT guard → LEFT OUT (default LEAVE).
git rm genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md
git rm genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md
git rm genesis/docs/plans/2026-05-20-light-up-the-topology.md
git rm genesis/docs/superpowers/plans/2026-05-07-topology-substrate-completion-m1-plan.md

# D.3 — Scenario archaeology & archetype map (after A.2.3 record).
git rm genesis/docs/plans/2026-05-22-scenario-archaeology-and-archetype-map.md
git rm genesis/docs/plans/2026-05-22-archaeology-decisions-digest.md

# D.4 — Doorway blob registry routing & vocabulary (after A.2.4 record).
git rm genesis/docs/superpowers/plans/2026-04-28-doorway-blob-registry-routing.md
git rm genesis/docs/superpowers/plans/2026-04-30-vocabulary-cleanup-sprint-kickoff.md

# D.6 — R&O lessons cross-wave guidance (after A.2.5 record + C7 repoint of the 3 qahal references).
git rm genesis/docs/plans/2026-04-21-rno-lessons-cross-wave-guidance.md

# D.7 — Experience-story discernment-gate stub (after A.2.6 record). rakia submodule untouched.
git rm genesis/docs/superpowers/plans/2026-04-18-experience-story-discernment-gate.md

# D.8 — Attestation-consolidation impl plan (after A.2.7 record). Design spec + recovery-m4 plan LEFT.
git rm genesis/docs/superpowers/plans/2026-05-11-attestation-consolidation-implementation-plan.md
```

## (3) PILE DUPE BODIES (git-tracked: 16 paths)

```bash
# D.9 — Sweettest (after B.1 seed). Sprint landing-record LEFT for its own pass.
git rm genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md
git rm genesis/docs/superpowers/plans/2026-04-22-sweettest-integration-layer-plan.md
git rm genesis/docs/superpowers/specs/2026-05-24-sweettest-stage-efficiency-design.md

# D.10 — Doorway SSR (after B.2 seed). DEPLOY env-held does NOT block body retirement (code+tests landed).
git rm genesis/docs/superpowers/specs/2026-05-07-doorway-ssr-runtime-design.md
git rm genesis/docs/superpowers/plans/2026-05-07-doorway-ssr-runtime.md
git rm genesis/docs/superpowers/specs/2026-05-08-ssr-capability-design.md

# D.11 — Peer OAuth portal (after B.3 merge). session-bridge spec/plan LEFT (live).
git rm genesis/docs/superpowers/specs/2026-05-25-peer-oauth-portal-design.md
git rm genesis/docs/superpowers/notes/2026-05-25-peer-oauth-portal-substrate-audit.md
git rm genesis/docs/superpowers/plans/2026-05-25-peer-oauth-portal-plan.md

# D.12 — Doorway hub-edge (after B.4 amend). CONDITIONAL: retire hub-edge spec ONLY.
#   stewardship-chain plan (2026-05-19-doorway-stewardship-chain-design.md) is HELD → NOT removed.
git rm genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md

# D.13 — Capability profile + element contract (after B.5 amend).
git rm genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md
git rm genesis/docs/superpowers/plans/2026-05-20-capability-profile-element-contract-plan.md
git rm genesis/docs/superpowers/plans/2026-05-20-protocol-omni-component.md

# D.14 — App-manifest staged-intents (after B.6 in-place compaction). Retire the PLAN only;
#   design spec (2026-05-28-app-manifest-staged-intents-design.md) is COMPACTED-IN-PLACE, NOT removed.
git rm genesis/docs/superpowers/plans/2026-05-28-b-manifest-app-manifest-staged-intents.md

# D.15 — Conductor agent-info gossip (after A.2.8 record). Soak HELD does NOT block (code merged).
git rm genesis/docs/superpowers/specs/2026-05-28-conductor-agent-info-substrate-gossip-design.md
git rm genesis/docs/superpowers/plans/2026-05-28-conductor-agent-info-substrate-gossip.md
```

## (4) SHIFT ARTIFACTS — by date-slice (UNTRACKED/gitignored → use `rm`, NOT `git rm`)

> 129 files retire. In-window total (slices through May 24–27) = 133; minus 4 guard-KEEPs
> (3 federation-wiring-audit + 1 citation-guard `brainstorm-prompt-followup.md`) = **129**.
> KEPT infra (NOT shift artifacts, NOT removed): `.claude/shifts/.gitkeep`, `entities.json`, `mempalace.yaml`.
> KEPT (recent, INSIDE the ~14-day live window from 2026-06-02, NOT triaged by the review): every
> `.claude/shifts/2026-05-28*`, `2026-05-29*`, `2026-05-30*`, `2026-05-31*` file (12 files).

```bash
# --- SLICE 1: Apr 17–27 (13 files) | RESCUE R1 (04-24) R2 (04-25) must land first
rm .claude/shifts/2026-04-20T02-30-elohim-edge-docker-green.journal.md
rm .claude/shifts/2026-04-20T02-30-elohim-edge-docker-green.objective.json
rm .claude/shifts/2026-04-20T02-30-elohim-edge-docker-green.sprint-result.md
rm .claude/shifts/2026-04-23T19-10-epr-2c-push-lands-green.objective.json
rm .claude/shifts/2026-04-23T19-10-epr-2c-push-lands-green.readiness-report.md
rm .claude/shifts/2026-04-24T15-20-orchestrator-pipeline-unstable.journal.md
rm .claude/shifts/2026-04-24T15-20-orchestrator-pipeline-unstable.objective.json
rm .claude/shifts/2026-04-24T15-20-orchestrator-pipeline-unstable.readiness-report.md
rm .claude/shifts/2026-04-25T00-43-clear-dna-integration-holochain.objective.json
rm .claude/shifts/2026-04-26T03-21-dna-pass-orchestrator-finishes.objective.json
rm .claude/shifts/2026-04-27T03-56-all-pipelines-green-or-unstable.journal.md
rm .claude/shifts/2026-04-27T03-56-all-pipelines-green-or-unstable.objective.json
rm .claude/shifts/2026-04-27T03-56-all-pipelines-green-or-unstable.sprint-result.md

# --- SLICE 2: Apr 28–May 03 (17 files) | RESCUE R3 (04-28) R4 (04-30) must land first
rm .claude/shifts/2026-04-28T03-13-orchestrator-clean-deploy.journal.md
rm .claude/shifts/2026-04-28T03-13-orchestrator-clean-deploy.objective.json
rm .claude/shifts/2026-04-28T03-13-orchestrator-clean-deploy.sprint-result.md
rm .claude/shifts/2026-04-28T13-41-genesis-seeder-unstable.journal.md
rm .claude/shifts/2026-04-28T13-41-genesis-seeder-unstable.objective.json
rm .claude/shifts/2026-04-28T13-41-genesis-seeder-unstable.sprint-result.md
rm .claude/shifts/2026-04-29T00-15-alpha-blob-deploy-as-expected.journal.md
rm .claude/shifts/2026-04-29T00-15-alpha-blob-deploy-as-expected.objective.json
rm .claude/shifts/2026-04-29T00-15-alpha-blob-deploy-as-expected.sprint-result.md
rm .claude/shifts/2026-04-30T22-30-orchestrator-781-recover.journal.md
rm .claude/shifts/2026-04-30T22-30-orchestrator-781-recover.objective.json
rm .claude/shifts/2026-04-30T22-30-orchestrator-781-recover.sprint-result.md
rm .claude/shifts/2026-05-01T19-49-clear-dna-integration-bootstrap-steward.journal.md
rm .claude/shifts/2026-05-01T19-49-clear-dna-integration-bootstrap-steward.objective.json
rm .claude/shifts/2026-05-03T18-19-orchestrator-805-pipelines-unstable.journal.md
rm .claude/shifts/2026-05-03T18-19-orchestrator-805-pipelines-unstable.objective.json
rm .claude/shifts/2026-05-03T18-19-orchestrator-805-pipelines-unstable.sprint-result.md

# --- SLICE 3: May 04–10 (51 files: 28 flat + 23 in subdirs) | RESCUE R5/R6/R7 (05-07) land first
#     CITATION-GUARD KEPT: doorway-ssr-deliver-2026-05-07T23-37/brainstorm-prompt-followup.md (the other 5 clear)
rm .claude/shifts/2026-05-04T22-51-alpha-pipelines-green-no-shem.journal.md
rm .claude/shifts/2026-05-04T22-51-alpha-pipelines-green-no-shem.objective.json
rm .claude/shifts/2026-05-04T22-51-alpha-pipelines-green-no-shem.sprint-result.md
rm .claude/shifts/2026-05-05T17-20-drive-genesis-e2e-verification-quality.journal.md
rm .claude/shifts/2026-05-05T17-20-drive-genesis-e2e-verification-quality.objective.json
rm .claude/shifts/2026-05-05T20-30-verify-and-finish-genesis-e2e-verification-quality.journal.md
rm .claude/shifts/2026-05-05T20-30-verify-and-finish-genesis-e2e-verification-quality.objective.json
rm .claude/shifts/2026-05-05T20-30-verify-and-finish-genesis-e2e-verification-quality.sprint-result.md
rm .claude/shifts/2026-05-06T02-44-rca-genesis-browser-failure-classes.handoff-brief.md
rm .claude/shifts/2026-05-06T02-44-rca-genesis-browser-failure-classes.journal.md
rm .claude/shifts/2026-05-06T02-44-rca-genesis-browser-failure-classes.objective.json
rm .claude/shifts/2026-05-06T02-44-rca-genesis-browser-failure-classes.sprint-result.md
rm .claude/shifts/2026-05-07-feedback-dialogue-panel.sprint-result.md
rm .claude/shifts/2026-05-07T00-47-storybook-stage-green.journal.md
rm .claude/shifts/2026-05-07T00-47-storybook-stage-green.objective.json
rm .claude/shifts/2026-05-07T00-47-storybook-stage-green.sprint-result.md
rm .claude/shifts/2026-05-07T14-15-topology-substrate-completion-m1-handoff.md
rm .claude/shifts/2026-05-09T16-30-orchestrator-clean-cascade.journal.md
rm .claude/shifts/2026-05-09T16-30-orchestrator-clean-cascade.objective.json
rm .claude/shifts/2026-05-09T16-30-orchestrator-clean-cascade.sprint-result.md
rm -r .claude/shifts/alpha-ingress-static-asset-cluster-2026-05-06T19-15
rm -r .claude/shifts/doorway-conductor-stale-mapping-2026-05-06T17-40
rm -r .claude/shifts/geospatial-cybersyn-deliver-2026-05-07T02-06
rm -r .claude/shifts/light-up-the-topology-deliver-2026-05-06T04-57
rm -r .claude/shifts/light-up-the-topology-deliver-2026-05-07T04-20
rm -r .claude/shifts/light-up-the-topology-deliver-cont-2026-05-06T08-30
# doorway-ssr-deliver-2026-05-07T23-37: remove the 5 CLEAR files, KEEP brainstorm-prompt-followup.md
rm .claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/feature-promise.json
rm .claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/iter0-verdict.md
rm .claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/iter12-verdict.md
rm .claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/journal.md
rm .claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/sprint-result.md
# (KEEP: .claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/brainstorm-prompt-followup.md — CITATION-GUARD)

# --- SLICE 4: May 11–17 (21 files) | RESCUE R8 (05-16) lands first
rm .claude/shifts/2026-05-11T02-24-fix-sccache-unbound-on-elohim-holochain.journal.md
rm .claude/shifts/2026-05-11T02-24-fix-sccache-unbound-on-elohim-holochain.objective.json
rm .claude/shifts/2026-05-14T23-37-clean-delivery-memory-substrate.journal.md
rm .claude/shifts/2026-05-14T23-37-clean-delivery-memory-substrate.objective.json
rm .claude/shifts/2026-05-14T23-37-clean-delivery-memory-substrate.sprint-result.md
rm .claude/shifts/2026-05-15-recovery-m4-deliver-pending.sprint-result.md
rm .claude/shifts/2026-05-15T03-43-fix-attestation-cid-decode.journal.md
rm .claude/shifts/2026-05-15T03-43-fix-attestation-cid-decode.objective.json
rm .claude/shifts/2026-05-15T23-53-recovery-m4-orchestrator-sail-through.journal.md
rm .claude/shifts/2026-05-15T23-53-recovery-m4-orchestrator-sail-through.objective.json
rm .claude/shifts/2026-05-15T23-53-recovery-m4-orchestrator-sail-through.readiness-report.md
rm .claude/shifts/2026-05-16T05-00-three-pipelines-green.journal.md
rm .claude/shifts/2026-05-16T05-00-three-pipelines-green.objective.json
rm .claude/shifts/2026-05-17T03-29-land-graph-native-push.journal.md
rm .claude/shifts/2026-05-17T03-29-land-graph-native-push.objective.json
rm .claude/shifts/2026-05-17T15-57-rca-orchestrator-963-graph-failure.journal.md
rm .claude/shifts/2026-05-17T15-57-rca-orchestrator-963-graph-failure.objective.json
rm .claude/shifts/2026-05-17T15-57-rca-orchestrator-963-graph-failure.sprint-result.md
rm .claude/shifts/2026-05-17T20-47-genesis-cucumber-parse-recovery.journal.md
rm .claude/shifts/2026-05-17T20-47-genesis-cucumber-parse-recovery.objective.json
rm .claude/shifts/2026-05-17T20-47-genesis-cucumber-parse-recovery.sprint-result.md

# --- SLICE 5: May 18–23 (20 files) | all CLEAR
rm .claude/shifts/2026-05-18T15-30-ci-propagation-of-plan-3a-and-resilience.journal.md
rm .claude/shifts/2026-05-18T15-30-ci-propagation-of-plan-3a-and-resilience.objective.json
rm .claude/shifts/2026-05-18T15-30-ci-propagation-of-plan-3a-and-resilience.sprint-result.md
rm .claude/shifts/2026-05-21T00-30-pipelines-unstable-or-better.journal.md
rm .claude/shifts/2026-05-21T00-30-pipelines-unstable-or-better.objective.json
rm .claude/shifts/2026-05-22T02-40Z-lift-elohim-edge-storage-dockerfile.journal.md
rm .claude/shifts/2026-05-22T02-40Z-lift-elohim-edge-storage-dockerfile.objective.json
rm .claude/shifts/2026-05-22T10-45-orchestrator-dev-unstable-or-better.journal.md
rm .claude/shifts/2026-05-22T10-45-orchestrator-dev-unstable-or-better.objective.json
rm .claude/shifts/2026-05-22T10-45-orchestrator-dev-unstable-or-better.readiness-report.md
rm .claude/shifts/2026-05-22T10-45-orchestrator-dev-unstable-or-better.sprint-result.md
rm .claude/shifts/2026-05-22T18-48-validate-ci-cd-gap-close-push.journal.md
rm .claude/shifts/2026-05-22T18-48-validate-ci-cd-gap-close-push.objective.json
rm .claude/shifts/2026-05-22T18-48-validate-ci-cd-gap-close-push.sprint-result.md
rm .claude/shifts/2026-05-23T05-25-alpha-landing-page-dual-doorway.journal.md
rm .claude/shifts/2026-05-23T05-25-alpha-landing-page-dual-doorway.objective.json
rm .claude/shifts/2026-05-23T19-00-orchestrator-and-genesis-unstable-or-better.journal.md
rm .claude/shifts/2026-05-23T19-00-orchestrator-and-genesis-unstable-or-better.objective.json
rm -r .claude/shifts/hosted-steward-portal-deliver-2026-05-22T01-42

# --- SLICE 6: May 24–27 (7 files) | RESCUE R9 (05-27) lands first
#     GUARD-KEEP: the 3 federation-wiring-audit files (2026-05-27T18-50-*) — NOT removed (see exclusions)
rm .claude/shifts/2026-05-24T03-30-sweettest-efficiency-w1-w2-w3.journal.md
rm .claude/shifts/2026-05-24T03-30-sweettest-efficiency-w1-w2-w3.objective.json
rm .claude/shifts/2026-05-26T08-30-deliver-epr-app-iter0.sprint-result.md
rm .claude/shifts/2026-05-26T08-35-shift-epr-app-delivery.journal.md
rm .claude/shifts/2026-05-26T08-35-shift-epr-app-delivery.sprint-result.md
rm .claude/shifts/2026-05-27T00-14-first-clean-post-migration-dev-build.journal.md
rm .claude/shifts/2026-05-27T00-14-first-clean-post-migration-dev-build.objective.json
# (KEEP: 2026-05-27T18-50-federation-wiring-audit.{journal.md,objective.json,readiness-report.md} — GUARD)
```

---

## COUNT

| Group | Verb | Paths (commands) | Files removed |
|---|---|---|---|
| (1) Junk drawers | `git rm -r` | 3 | 77 |
| (2) Pile dead-arch | `git rm` | 25 | 25 |
| (3) Pile dupe | `git rm` | 16 | 16 |
| (4) Shift artifacts | `rm` / `rm -r` | 56 commands (50 file + 6 dir) | 129 |
| **TOTAL** | | **100 commands** | **247 files** |

- Group-4 slice breakdown (files): 13 + 17 + 51 + 21 + 20 + 7 = **129**.
- Pile total (2)+(3) = **41 bodies** retired (matches burndown ~25 dead-arch + ~16 dupe).
- Grand total bodies → git/working-tree: **247** (77 junk + 41 pile + 129 shift). The UNIFIED §4 "~223"
  estimate was low; the on-disk count is 247 (its 107 shift estimate vs. the real 129 in-window-minus-guards
  is the bulk of the delta).

---

## GUARD CHECK — every guarded path verified ABSENT from the removal list above

- [x] **Citation-guard KEPT** — `.claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/brainstorm-prompt-followup.md`
      is NOT in any `rm` line; the other 5 files in that dir ARE removed individually (no `rm -r` on that dir).
- [x] **7 LEAVE-IN-PILE specs/plans** — none appear: `2026-05-16-graph-native-projection-substrate-design.md`
      (the `…-plan` variant does not exist on disk), `2026-05-29-light-up-the-topology-sprint-kickoff.md`,
      `2026-05-22-value-scanner-content-audit.md`, `2026-05-28-session-bridge-design.md` +
      `2026-05-28-session-bridge-implementation.md`, `2026-05-18-app-manifest-modularization.md`, the amended
      live design specs (experience-story EPR, attestation-consolidation, hub-boundaries, cradle-to-grave,
      wave3 VF/hREA), and `2026-05-24-sweettest-stage-efficiency-w1-w2-w3-w5.md` (sprints/ landing-record).
- [x] **HELD body that HOLDS** — `plans/2026-05-19-doorway-stewardship-chain-design.md` (D.12 conditional HOLD)
      is NOT removed; only the hub-edge spec retires under D.12.
- [x] **CONFLICT LEFT** — `plans/2026-05-19-topology-resilience-qahal-synthesis.md` is NOT in the D.2 block.
- [x] **federation-wiring shift KEPT** — all 3 `.claude/shifts/2026-05-27T18-50-federation-wiring-audit*`
      files are NOT in any `rm` line (explicitly excluded from Slice 6).
- [x] **Compacted-in-place, NOT removed** — `2026-05-28-app-manifest-staged-intents-design.md` (D.14 retires
      only the `-b-manifest-` plan).
- [x] **Infra KEPT** — `.claude/shifts/.gitkeep`, `entities.json`, `mempalace.yaml` are NOT removed.
- [x] **Live working-memory KEPT** — every `.claude/shifts/2026-05-28*` … `2026-05-31*` file (inside the
      ~14-day window; not triaged by the review's slices) is NOT removed.

**Confirmation: every guarded path above was checked and is excluded from the removal list.**
