# Backlog Refresh — 2026-05-10

Items judged BACKLOG by `proposals-judged.md`. Sorted by recency of related activity (most-recently-touched adjacent area first). These represent unfinished work that's still relevant — surfaced so the operator can either pull to current sprint, file as long-term backlog, or reframe and continue.

---

## 1. `genesis/docs/superpowers/plans/2026-05-10-iroh-delivery-master.md`

**What's unfinished:** The iroh cutover master plan (gates 6, 7, 8, 9, 11) calls for four runbooks (`RUNBOOK-iroh-parity-soak`, `RUNBOOK-iroh-alpha-soak`, `RUNBOOK-iroh-consumer-soak`, `RUNBOOK-iroh-rollback-drill`), the `alpha-cluster.yaml` topology manifest, and a 10k-iteration `bench_blob_stress_10k.rs` benchmark. None exist yet.

**Why it still matters:** Iroh delivery is the most active engineering theme in the repo. Plan 4 Tasks 1-9 just landed (commits `0283e5adf` through `4af3226cf`), Plan 2 (blob backend) Tasks 1-7 also recent. The cutover gates are the path from "phases done in worktrees" to "iroh canonical in production" — without runbooks the operator can't run the soak.

**Suggested next step:** Bring to current sprint. The runbooks are 1-2 hour authoring tasks per item; the stress test is half a day; alpha-cluster.yaml depends on operator-side cluster topology decisions.

---

## 2. `genesis/docs/superpowers/plans/2026-05-10-iroh-phase12-peer-transport-manifest.md`

**What's unfinished:** Phase 12 generates TypeScript bindings for `Libp2pTransportProfileView`, `IrohTransportProfileView`, `PeerTransportManifestView` and distributes them to seeder/elohim-app/elohim-library generated dirs. Schema file landed in dev (`elohim/sdk/schemas/v1/views/peer-transport-manifest.schema.json`); generated TS files exist in `iroh-seeder-dual` worktree only.

**Why it still matters:** Phase 12 Tasks 1-13 commits show active in-flight cutover prep (`d36dbf62d`, `939b9bf56`, `8da3a70ec` 2026-05-09/10). Without these TS bindings, frontend/seeder can't introspect peer transport capability.

**Suggested next step:** Bring to current sprint — finish codegen run (`pnpm run schema:codegen:ts`) and merge from worktree.

---

## 3. `genesis/docs/superpowers/plans/2026-05-10-iroh-recovery-e2e.md`

**What's unfinished:** Cross-stack iroh recovery e2e (`iroh_recovery_cross_stack.rs`, `recovery-cross-stack-transport.feature`, `recovery-cross-stack.steps.ts`, `multi_stack_fixture.rs`) — all in `iroh-recovery-e2e` worktree, not yet on dev.

**Why it still matters:** Recovery via iroh transport is gate #4 of the iroh cutover. Worktree exists locally per `git branch` listing — work in progress, awaiting validation.

**Suggested next step:** Bring to current sprint — finish e2e validation in worktree, merge to dev.

---

## 4. `genesis/docs/superpowers/plans/2026-05-10-iroh-seeder-dual-write.md`

**What's unfinished:** Seeder publishes blob with both libp2p sha256 + iroh blake3 addresses; backfill tool migrates existing libp2p-only blobs. Worktree `iroh-seeder-dual` has Tasks 1-3 done (`a68a6972f` PutBlobResponse schema, `c5d919dff` view, `45d536fba` record_self_custody for dual-write). Remaining tasks 4-10 + a2o feature (`seeder-dual-write.feature`) outstanding.

**Why it still matters:** Without dual-write, fresh seeded content is only addressable via libp2p, blocking iroh cutover for anything new. Active in-flight work.

**Suggested next step:** Bring to current sprint — continue Plan 3 in the worktree.

---

## 5. `genesis/docs/superpowers/plans/2026-05-08-ssr-capability-implementation.md`

**What's unfinished:** `app_context.rs` (carries auth/render context), `auth_mode_enforcement.rs` and `concurrency_overflow.rs` doorway tests not on dev. SSR landed at view-projection level (`932dbf44e` un-stub ComputeMetricsView) but the auth-context propagation through V8 boundary and concurrency hardening tests are open.

**Why it still matters:** MEMORY entry `project_ssr_anonymous_auth_context` (2 days ago) explicitly flags the auth-propagation gap as unsolved. Higher-reach content currently renders empty for authenticated users via SSR.

**Suggested next step:** Long-term backlog (next sprint) — depends on framework-agnostic auth propagation design (per memory: signed delegation token, OAuth on-behalf-of style).

---

## 6. `genesis/docs/superpowers/plans/2026-05-07-topology-substrate-completion-m1-plan.md`

**What's unfinished:** M1 a2o feature exists at `genesis/a2o/features/shefa/m1-matthew-timothy-delivery.feature` (commit `503323950`) but plan referenced `topology/` subdir. Orchestrator topology assets missing: `genesis/orchestrator/environments/topology.json` not present (only `alpha.env`, `prod.env`, `staging.env`); `genesis/orchestrator/render.js` not created.

**Why it still matters:** Topology delivery (matthew↔timothy household scenarios) is the canonical multi-node test. Plan 4 iroh dual-publish work is recent (2026-05-09); cluster topology orchestration becomes more important as iroh stack matures.

**Suggested next step:** Bring to current sprint — relocate feature to topology/ if convention requires, decide whether topology.json + render.js are still wanted or if alpha-cluster.yaml (item 1 in this list) supersedes them.

---

## 7. `genesis/docs/superpowers/specs/2026-05-03-distribution-resilience-substrate-sharing-backlog.md`

**What's unfinished:** `genesis/a2o/features/shefa/substrate-sharing-incentives.feature` not present. Spec is explicitly tagged "backlog" — design memo for shefa's substrate-sharing incentive mechanics.

**Why it still matters:** Shefa pillar (economy) needs incentive scenarios for stewards to share substrate (compute, storage, bandwidth). Pairs with `project_dpin_contracts_are_policy` and `project_household_horizontal_scaling` memories. Distribution resilience work (item 37 in proposals) just landed; incentives are the next layer.

**Suggested next step:** Long-term backlog — promote when shefa work resumes.

---

## 8. `genesis/docs/superpowers/plans/2026-05-01-light-up-the-topology-plan.md`

**What's unfinished:** 38 dangling refs across content adapters, lamad components, dashboard views, REA action schema, peer-fallback, doorway dashboard schemas. Most files don't exist anywhere.

**Why it still matters:** Topology completion (per item 6 in this list, item 42 in proposals) is in active iteration. Light-up was a sweeping plan that overlaps with topology-substrate-completion-m1; portions converged into M1, others not picked up.

**Suggested next step:** Reframe and continue — use the M1 plan + iroh-delivery-master as the active completion path; revise this plan to mark which artifacts went to other plans vs which remain TBD.

---

## 9. `genesis/docs/superpowers/plans/2026-04-24-recovery-protocol-phase-2-m4-fast-path-revocation.md`

**What's unfinished:** Schemas merged (`key-revocation.schema.json`, `revocation-vote.schema.json` via commits `18558259d`, `6d675b6b5`); sweettest exists. Missing: a2o feature files `genesis/a2o/features/auth/revocation-self.feature` and `revocation-emergency-quorum.feature`. `feature/recovery-m4-fast-path-revocation` branch exists locally.

**Why it still matters:** Revocation UX is M4's user-visible deliverable. Without a2o scenarios, the user-experience contract isn't captured (per repo policy: "Story-First Default").

**Suggested next step:** Long-term backlog — revocation UX feature scenarios should land before M5+ work consolidates.

---

## 10. `/projects/.claude-config/projects/-projects-elohim/memory/project_hub_archetype_abstraction.md`

**What's unfinished:** Memory describes Hub abstraction (HouseholdHub vs CollectiveHub as separate implementations), names canonical hub archetypes, declares "Where it lives: `genesis/data/hubs/hubs.json`" + `HubArchetype` types in `genesis/a2o/src/framework/fixtures/hubs.ts` + `hub-topology.feature` constitutional scenarios. None of these created.

**Why it still matters:** Hub abstraction is foundational for cluster topology + household-as-hub scaling math. Topology delivery work (items 6, 8 in this list) and EPR Phase 3.5 trust-compute gradient (item 32 in proposals) both presume hub-archetype fixtures.

**Suggested next step:** Long-term backlog — needs its own design sprint. Memory is 8 days old; concept ratification still pending per memory's own "to be ratified in follow-up brainstorm" notes.

---

## 11. `genesis/docs/superpowers/plans/2026-04-21-rno-lessons-wave-2-execution-plan.md`

**What's unfinished:** Wave 2 expected an `app/elohim-library/projects/elohim-service/src/services/feature-flags.service.ts`. Never built. Pairs with item 12 (graduation-path plan also unfinished).

**Why it still matters:** MEMORY entry `project_elohim_active_observed_not_flagged` explicitly references the absence of feature flags in the runtime. Multiple downstream concerns (mode gating, gradual rollout) want feature flag substrate.

**Suggested next step:** Long-term backlog — small but recurring blocker. Decide between explicit feature-flag service vs continuing the "observed not flagged" pattern.

---

## 12. `genesis/docs/plans/2026-04-24-rno-wave2-graduation-path-kickoff-prompt.md`

**What's unfinished:** The kickoff prompt references a `2026-04-24-rno-graduation-path.md` plan body that was never authored. RNO Wave 2 is partially landed but the graduation-path plan body is missing.

**Why it still matters:** Graduation path (wave 2 #9) connects RNO lessons to a learner's progression — incomplete documentation leaves the design implicit.

**Suggested next step:** Long-term backlog — write the plan body or fold the graduation-path concept into a successor plan.

---

## 13. `genesis/docs/superpowers/plans/2026-04-19-runtime-request-correlation.md`

**What's unfinished:** 23 dangling refs spanning `correlation/types.rs`, `correlation/mod.rs`, `correlation/context.rs`, headers schema (`request-correlation.schema.json`, `admin-logs-response.schema.json`), generated TS bindings, and inbound-correlation tests. None of the planned files exist; no commits since 2026-04-19 mention request correlation.

**Why it still matters:** Cross-cutting observability (correlation IDs, trace propagation) becomes more important with iroh + dual-stack transport. Without correlation, debugging cross-stack issues (P2P → doorway → admin logs) is much harder.

**Suggested next step:** Long-term backlog — defer until iroh cutover completes; revisit as part of observability sprint. Could pair well with the iroh consumer-soak runbook (item 1 in this list) which will need correlation IDs to debug per-archetype failures.

---

## 14. `genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md` + `genesis/docs/superpowers/plans/2026-04-27-jenkins-stage-1a-brit-advisory.md`

**What's unfinished:** Stage 1A advisory may have shipped (no specific commits found post-2026-04-27 confirming). Stages 1C (attestation writes), 2 (consume plan), 3 (retire legacy) all use `2026-XX-XX-` placeholder dates and have no plan bodies. `print-plan-summary.mjs` orchestrator script not created.

**Why it still matters:** Brit-attestation epic — Jenkins as observability + attestation producer — is a multi-stage rollout. Stage 1A scaffolds; stages 1C-3 are the actual value. Without them, build outputs aren't notarized into the attestation graph.

**Suggested next step:** Long-term backlog — needs scoping pass before pickup. Coupled to brit infrastructure (`elohim/brit/` exists).

---

## 15. `genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md`

**What's unfinished:** Plan 1 shipped (per dev-intent.jsonl 2026-04-20). Plan 2 (periodic verification) was the noted next step. The chaos-test step file `genesis/a2o/support/steps/chaos-steps.ts` was never created (the spec referenced it for chaos scenarios).

**Why it still matters:** Self-healing dataplane needs chaos testing (kill nodes, partition network, verify auto-distribute). Without chaos steps, the contract isn't enforced under failure modes.

**Suggested next step:** Long-term backlog — small but valuable testing infra; couples well with iroh consumer-soak (item 1) which exercises real-world failure modes.
