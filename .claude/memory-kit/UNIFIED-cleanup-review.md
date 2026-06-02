# UNIFIED Cleanup Review — burndown + shifts, one gated pass

> **SAFETY:** This is a *review only*. Nothing on disk has been mutated. Every existing file under
> `genesis/docs/`, `.claude/archive/`, `.claude/memory/`, `.claude/shifts/` is untouched. The single write
> performed is *this file*. All A–D actions are operator-gated and execute LATER, on approval.
>
> **What's new vs the two input proposals:** this pass folds the **shifts refinement** in. Shifts are a
> budgeted working-memory sub-tier (~14-day live window) that must **fully decompose to zero residue**:
> a recurring anti-pattern → a watch-out pointer in the **canonical** surface *and* the **curated history**
> museum (not just a lone memory entry); an open issue a shift surfaced → **resolved into a backlog item**
> (promoted, not buried); the narration body → **git**. Recurrence across shifts is the signal that an
> anti-pattern has earned canonical placement — a one-off is just narration.

---

## EXECUTIVE SUMMARY (10 lines)

1. Three completed proposals (junk-drawer, pile dead-arch/dupes, shifts) merge into one gated A→B→C→D plan; risk rises monotonically and D never starts until A–C land.
2. **Museum grows 4 → ~16 curated history records** (4 junk-`2026-05-15` + 8 pile dead-arch; +1 shifts-recurrence record proposed below = ~17), all pure-additive, zero risk.
3. **Canonical seeds: 7** from the dupe clusters (4 new/amended architecture seeds, 3 merge-into-existing).
4. **~223 bodies retire to git** across all sources (~116 burndown pile+junk + ~107 shift artifacts), grouped + bulk-approvable, each git-recoverable at its live path.
5. **Pile:canonical ratio moves ~6.2× → ~1.5–2×** once the retired bodies leave and the ~19 new canonical artifacts land.
6. **Shifts frequency-rank:** ~12 anti-patterns recur in ≥3 shifts; the top recurring ones (orchestrator supersede/NOT_BUILT, baseline-rollback over-build, Dockerfile/manifest completeness, HUSKY=0-is-non-functional, sccache-poisons-rustc-output, `#[ignore]`-is-a-CI-no-op, cucumber empty-alternation, CPS method-size) get canonical watch-out pointers; most already have a memory entry, so the action is *ensure it's in the canonical surface*, not write a new entry.
7. **9 first-pass shift lessons (R1–R9) reconciled:** R1, R2, R4, R8 stay new memory entries (parameter-bearing, single-instance); R3/R7/R9 are augmentations; R5/R6 stay memory entries but also seed canonical watch-outs (dialog top-layer; Angular-19 SSR build-glue).
8. **Open issues deduped to ~24 backlog candidates**, destination `genesis/data/timeline/backlog/`, split operator-domain (RBAC/Harbor/cluster) vs code-domain (linters, measure-tightening, manifest-completeness gates).
9. **Guards carried forward verbatim:** 7 LEAVE-IN-PILE, 6 HELD-needs-CI, 1 CONFLICT (topology-resilience-qahal-synthesis → default LEAVE), 2 citation-guards (junk + shifts) — none are in the bulk-approve D set.
10. **Then:** re-point MemPalace ingestion at the cleaned surface and re-mine — it indexes curated history + seeds instead of junk-drawer residue and duplicate plan threads.

**File written:** `/projects/elohim/.claude/memory-kit/UNIFIED-cleanup-review.md`. Confidence HIGH across A–C; D removals are the only recoverability surface and every body is git-recoverable.

---

## 1. Shrink dashboard

| Surface | Before | After | Delta |
|---|---|---|---|
| **Museum** `…/history/` | 4 records (~13 KB) | **~17 records** (4 junk-`05-15` + 8 dead-arch + 1 shifts-recurrence + existing 4; ~50–60 KB) | +13, pure-additive |
| **Canonical seeds** (dupe clusters) | — | **7** (4 new/amended arch seeds + 3 merge-into-existing) | +4 files, +3 in-place |
| **Inline watch-out pointers** (canonical surfaces) | — | **~22** (12 burndown C-pointers + ~10 shifts-recurrence pointers) | additive edits, no content removed |
| **New memory entries** (shifts rescue) | — | **+5** (R1, R2, R4, R8, + R5/R6 stay) — R3/R7/R9 are augments | additive |
| **Pile** `genesis/docs/{plans,superpowers/*}` | ~6.2× canonical (junk-dominated) | **~1.5–2×** (genuinely-live work) | ~116 bodies retire |
| **Shift artifacts** `.claude/shifts/` (5 triaged slices) | ~108 files live | **~107 retire to git** (1 citation-guard kept) | working-tree copies removed; git preserves |
| **Total bodies → git** | — | **~223** (~116 burndown + ~107 shifts) | the only removals; all git-recoverable |
| **Pile:canonical headline ratio** | **~6.2×** | **~1.5–2×** | the MemPalace-quality signal |

The museum stays tiny even after a 4×+ growth; the pile stops being junk-dominated; shifts fully decompose (pattern→canon+history, issue→backlog, body→git) leaving **zero residue** in the ~14-day working-memory tier.

---

## 2. Safest-first execution plan (A → B → C → D), unified

Each phase is independently revertible; risk rises monotonically. **Do NOT start (D) until (A)–(C) have landed**, so no pointer ever dangles and no lesson is in-flight when a body leaves the tree.

### (A) ADD curated-history records — ZERO RISK, pure adds
New files under `genesis/docs/content/elohim-protocol/history/` + matching `INDEX.md` rows. Nothing existing is touched.
- **A.1 — 4 junk-`2026-05-15` records** (apply verbatim from `.claude/archive/2026-05-15/JUNK-DRAWER-TRIAGE-PROPOSAL.md`): rno-reference-implementation-positioning, deploy-is-not-a-graph-node, seed-row-shape-satisfies-view-sql-predicates, request-correlation-path-not-taken.
- **A.2 — 8 pile dead-arch records** (full text in burndown §A.2): epr-foundation-landed-by-waves, light-up-topology-operational-visibility-arc, archetype-primary-a2o-taxonomy-not-executed, doorway-dispatch-registry-fallback-and-vocabulary, rno-cross-wave-guidance-graduate-into-not-from, experience-story-discernment-gate, attestation-consolidation-phase2a-dedup, conductor-agent-info-substrate-gossip.
- **A.3 — 1 NEW shifts-recurrence record** (proposed this pass): `2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md` — the *curated* museum face of the high-frequency shift anti-patterns (§3 table), pointing back to the live memory entries + the root `Jenkinsfile`/orchestrator surfaces. This is what the shifts refinement requires: the recurring gotchas land in the **history museum**, not only scattered memory entries. (One record, not one-per-pattern — the museum is curated.)

### (B) CREATE / amend canonical seeds (dupes) + plant the recurring-anti-pattern watch-out pointers
*Additive; in-place spec trims are the only edits-to-existing, removing now-landed how-to whose truth is on disk (git-recoverable).*
- **B.1–B.7 canonical seeds** (burndown §B): Sweettest integration layer (NEW seed) · Doorway SSR runtime (NEW seed) · Peer OAuth portal (MERGE into `project_peer_native_account_canonical_surface`) · Doorway hub-edge / stewardship-chain (AMEND `2026-05-02-elohim-hub-boundaries-design`) · Capability-profile + element-contract (AMEND `cradle-to-grave-capability-gradient`) · App-manifest staged-intents (compact-in-place) · Conductor agent-info gossip (rendered as A.2.8 record + memory pointer).
- **B.8 — recurring-anti-pattern watch-out pointers** (this pass): plant the high-frequency shift anti-patterns into their canonical surfaces (§3 table → "canonical target" column). Most are *ensure-present* (the memory entry exists; confirm the canonical surface — root `Jenkinsfile` near the relevant stage, `genesis/orchestrator/README.md`, or root `CLAUDE.md` CI/CD section — carries the inline pointer). The shifts-rescue R5/R6 also plant canonical watch-outs here (dialog top-layer in the a2o/frontend conventions; Angular-19 SSR build-glue near the doorway-SSR seed B.2).

### (C) PLANT remaining inline pointers — small edits to live canonical docs, no content removed
- **C1–C12** from burndown §C (the 12-row table): junk-`05-14` precedent note, INDEX.md rows, graph-native + durability-topology + coherence-substrate + dispatch-contract + wave3-interop + experience-story + attestation + multi-doorway-registration + peer-native-account + capability-profile §5.1 pointers.
- **C13 (this pass)** — shifts-recurrence cross-links: the A.3 museum record's `cites:`/INDEX row + the memory-entry back-pointers for any augmented entries (R3 → `project_alpha_edge_deploy_debugging_landmarks`; R7 → `project_che_browser_feedback_loop`; R9 → `project_ci_storage_topology` *correction*).

### (D) RETIRE bodies to git — the ONLY removals, gated LAST, grouped + bulk-approvable
Never start until A–C land. Approve in bulk per-cluster or veto specific files. Every body git-recoverable at its live path.
- **D.0 — Junk drawers** (3 dirs, ~77 files): `.claude/archive/2026-05-14/` (8), `2026-05-15/` (56), `2026-06-01/` (13).
- **D.1–D.8 — pile dead-arch bodies** (~25): EPR codec/storage, light-up-topology/graph, scenario-archaeology, doorway-blob-routing, R&O guidance, experience-story stub, attestation-consolidation impl-plan.
- **D.9–D.15 — pile dupe bodies** (~18): sweettest, doorway-SSR, peer-OAuth, hub-edge (+ **D.12 conditional** hold stewardship-chain plan), capability-profile, staged-intents plan, conductor-gossip.
- **D.16 — shift artifacts** (~107, grouped by the 6 date-slices in §4): all triaged shift dirs **except** the 1 citation-guard file. The 9 RESCUE-shift dirs clear *after* their lessons (R1–R9) land in A/B/C.

---

## 3. Shifts — refined disposition

### 3a. Recurring anti-patterns (frequency-ranked) → canonical target + history?

> Frequency = count of distinct shifts the pattern appears in, across the four extraction slices
> (Apr 17–30, May 1–10, May 11–20, May 21–27). Only ≥3-shift recurrences shown (the high-value capture);
> ≥2-shift ones noted in 3b. "In memory?" = a memory entry already exists → action is **ensure canonical
> placement**, not write a new entry. All recurring CI/orchestrator patterns ALSO land in the A.3 museum record.

| # | Anti-pattern | Freq | Representative shifts | In memory? | → Canonical target (watch-out pointer) | Also history? |
|---|---|---|---|---|---|---|
| 1 | **Orchestrator NOT_BUILT/superseded read as regression** (abortPrevious preempts in-flight child; FAILURE-count measure reads NOT_BUILT/ABORTED/UNSTABLE all as 0 — lossy) | **8** | 04-30, 05-03, 05-11, 05-14, 05-16, 05-21, 05-24, 05-30 | partial (`feedback_orchestrator_abort_baseline_rollback`) | `genesis/orchestrator/README.md` (measure-semantics note) + root `genesis/orchestrator/Jenkinsfile` near supersede/measure; the lossy-measure note in CLAUDE.md CI/CD | **YES** (A.3) |
| 2 | **Baseline-rollback over-build** (FAILURE/ABORT invalidates per-pipeline baseline → reverts to global → full cascade; `lastSuccessful()` pins ancient green) | **6** | 04-28, 05-03, 05-05, 05-16, 05-22, 05-24 | YES (`feedback_orchestrator_abort_baseline_rollback`) | ensure-present in that entry + `genesis/orchestrator/README.md` baseline section | YES (A.3) |
| 3 | **Dockerfile / build-manifest completeness** (new Cargo target OR new path-dep crate breaks Docker context but passes host pre-push; manifest under-covers source inputs) | **6** | 04-20, 04-27, 05-03, 05-17, 05-18, 05-21/05-22 | YES (`feedback_dockerfile_target_completeness`, `feedback_orchestrator_build_manifest_required`) | ensure-present; root `CLAUDE.md` Dockerfile gotcha section watch-out (Docker-only failure mode) | YES (A.3) |
| 4 | **HUSKY=0 is NON-FUNCTIONAL; `--no-verify` is the real bypass** (`core.hooksPath=.husky` bypasses the wrapper that honors HUSKY=0) | **4** | 05-06, 05-11, 05-16, 05-18 | partial (`feedback_husky_bypass_for_ci_only_changes` covers *when*, not the mechanical gotcha) | **AUGMENT** `feedback_husky_bypass_for_ci_only_changes` with the mechanical `core.hooksPath` note + correct the root `CLAUDE.md` "Bypass with HUSKY=0" line (it is misleading) | YES (A.3) |
| 5 | **sccache/S3 cache poisons rustc output** (NoSuchKey/null-byte interleaves into diagnostics → spurious "unclosed delimiter"; RUSTC_WRAPPER="" bypasses, key persists) | **4** | 05-09, 05-11, 05-17, 05-18 | YES (`feedback_sccache_cache_corruption_recovery`, `feedback_sccache_spawn_enoent_rca`) | ensure-present; add the **no-runbook open issue** to backlog (heal = SCCACHE_RECACHE=1 / repave) | YES (A.3) |
| 6 | **`#[ignore]` is a CI no-op** (DNA sweettest runs `--run-ignored all`; quarantine-by-ignore costs a full ~75-min cycle) | **2** | 05-11, 05-16 | NO (only `feedback_cargo_nextest_installed` notes local skip) | **NEW entry R8** `feedback_sweettest_ignore_is_ci_noop` + watch-out in `elohim/holochain/dna/Jenkinsfile` near `--run-ignored all` | YES (A.3) |
| 7 | **Cucumber/Gherkin parse aborts the whole E2E run** (unescaped `/` → empty-alternation; bare continuation line → AST reject; empty cucumber-report → UNSTABLE w/ blank body) | **4** | 04-28, 05-05, 05-17, (05-06 adjacent) | partial (no dedicated entry; `feedback_schema_data_enum_drift_cascade` is adjacent) | watch-out in `genesis/a2o/` framework CLAUDE/conventions + read-the-E2E-log-FIRST note; backlog: pre-push gherkin linter | YES (A.3) |
| 8 | **CPS method-size limit** (11000-byte/64KB hard cap; inline handlers/pre-flight blocks blow it; extract a helper above `pipeline {}`) | **4** | 04-28, 05-17, 05-24, 05-26 | YES (root `CLAUDE.md` "Jenkinsfile Size Limit" gotcha; `jenkinsfile-cps-scope.test.mjs` exists) | ensure-present; the CPS-scope-loss-across-stages variant (env-bridge, not method-size) is **NOT** captured → backlog: CPS-scope static lint | YES (A.3) |
| 9 | **Webhook double-fire** (one dev push → 2 builds, first superseded; explicit `triggers{githubPush()}` + Multibranch implicit both fire) | **4** | 04-30, 05-03, 05-05, 05-22 (timer-collision variant) | NO | watch-out in `genesis/orchestrator/README.md` triggers section; backlog: remove explicit trigger (rollback-ready shift) | YES (A.3) |
| 10 | **Cascade-halt / cascade-hidden test surface** (driving long-red pipeline green unmasks buried failures; track ratio not raw count) | **5** | 05-03, 05-05, 05-16, 05-21, (multiple) | YES (`feedback_cascade_halt_masks_failures`, `feedback_cascade_hidden_test_surface`) | ensure-present | YES (A.3) |
| 11 | **Doorway single-target fan-out violation** (warm-stream/STORAGE_URLS iteration overwrites correct row; last-writer-wins) | **3** | 04-29, 05-06, 05-23 | YES (`project_doorway_single_target_no_fanout`) | ensure-present | (covered by existing memory + A.2.x) |
| 12 | **DHT readback envelope bug** (`Entry::try_into()→SerializedBytes` serializes the `Entry::App` variant tag → "missing field" on readback; use `to_app_option::<T>()`) | **3** | 04-24, 04-25, 05-01, (05-15 CID-format sibling) | NO (distinct from `feedback_serde_json_value_breaks_zome_boundary`) | **NEW entry R2** `feedback_dht_readback_use_to_app_option` + watch-out in the DNA coordinator CLAUDE/conventions | (memory entry suffices) |

**Routing principle applied:** patterns 1–10 are CI/orchestrator/build recurrences → they earn the **A.3 curated museum record** (the shifts refinement's "land in the HISTORY museum" requirement) *and* an inline canonical watch-out. Patterns 11–12 are substrate-domain and already own (or get) a dedicated memory entry. There is **no `genesis/orchestrator/CLAUDE.md`** — the canonical orchestrator doc is `genesis/orchestrator/README.md`; orchestrator watch-outs route there + to the root `Jenkinsfile`/`genesis/orchestrator/Jenkinsfile` inline + the root `CLAUDE.md` "CI/CD" section.

### 3b. The 9 first-pass lessons (R1–R9) reconciled

| R | Lesson | Disposition | Canonical watch-out also? |
|---|---|---|---|
| **R1** | Sweettest is a NATIVE build (cmake/clang/LIBCLANG_PATH; clear WASM RUSTFLAGS; ~90/150-min budget) | **NEW memory** `feedback_sweettest_native_build_env` (parameter-bearing, single-instance) | Yes — fold into **B.1 sweettest seed** watch-outs (generalizes the CLAUDE.md RUSTFLAGS gotcha to the CI sweettest stage) |
| **R2** | DHT readback `to_app_option`, not `Entry::try_into` | **NEW memory** `feedback_dht_readback_use_to_app_option` | Pattern #12 above; watch-out in DNA coordinator conventions |
| **R3** | SQLite multi-writer contention is the seeder's failure mode (WAL + busy_timeout-before-WAL) | **AUGMENT** `project_alpha_edge_deploy_debugging_landmarks` (fold into SQLite bullet) — operator may prefer standalone | No (memory augment) |
| **R4** | Jenkins 3-layer checkout + `elohim-holochain`→DNA-Jenkinsfile job-alias trap | **NEW memory** `feedback_jenkins_checkout_layers_and_job_alias` | Yes — the job-alias note also belongs in root `CLAUDE.md` pipeline table (per the 04-30 open issue) |
| **R5** | Native `<dialog>`+`showModal()` top-layer modal fix | **NEW memory** (frontend) `feedback_native_dialog_top_layer_modal` | Yes — **canonical watch-out** in `genesis/a2o/` framework conventions (the migration gotchas: `:modal` not z-index; synthetic Escape no-op; backdrop via `event.target`) |
| **R6** | Angular-19-on-doorway SSR build-glue cluster (fetch-shim, index.csr.html, pnpm filter, shamefully-hoist) | **NEW memory** `feedback_angular19_ssr_build_glue` | Yes — **canonical watch-out** folded into **B.2 doorway-SSR seed** (build-glue subsection) |
| **R7** | Che-headless cannot render WebGL (ANGLE/EGL init fails) | **AUGMENT** `project_che_browser_feedback_loop` (1–2 sentences; entry is about L1 render primitive, not the WebGL init failure) | No (memory augment) |
| **R8** | CI runs sweettests `--run-ignored all`; `#[ignore]` is a no-op silencer | **NEW memory** `feedback_sweettest_ignore_is_ci_noop` | Pattern #6 above; watch-out in DNA Jenkinsfile |
| **R9** | hostpath PVC needs deterministic node pinning — **corrects a factually-stale entry** | **AUGMENT (CORRECT)** `project_ci_storage_topology` (it currently says the OPPOSITE: "openebs-jiva… no nodeSelector/affinity tricks needed") — fold as dated 2026-05-27 update | No — but this is the **highest-leverage augment** (single source-of-truth correction; the stale entry actively misleads) |

**Net new memory entries: 5** (R1, R2, R4, R5/R6, R8 — R6 distinct from R5). **3 augmentations** (R3, R7, R9). Operator-judgment forks: R3 (augment vs standalone), R9 (correct-in-place vs standalone `feedback_hostpath_pvc_needs_node_pinning`) — see §6.

### 3c. Open issues → backlog list

> Deduped across all four slices. Destination: `genesis/data/timeline/backlog/` (suggested file per cluster).
> Split: **OPERATOR-DOMAIN** (cluster/RBAC/infra — agent cannot fix; surface for operator) vs **CODE-DOMAIN**
> (repo-level fix the next shift/PR can land). The shifts refinement requires these be **resolved into backlog
> items, not buried** — promotion is the act that lets the narration body clear to git with zero residue.

**OPERATOR-DOMAIN (infra/cluster/RBAC — recurring, deduped):**
| Backlog item | Area | Recurrence | Suggested backlog dest |
|---|---|---|---|
| jenkins-deployer/ee-jenkins SA RBAC drift (PVC/Deployment/Service/scale Forbidden; masked by `+ true`); commit `rbac/jenkins-deployer-{ns}.yaml` so drift is reviewable from code | recovery/k8s | 04-27, 05-04, 05-26 | `backlog/ci-rbac-jenkins-deployer.md` |
| Harbor registry single-point-of-failure (ImagePullBackOff halts all CI; no self-heal); multi-replica or cached-image fail-over | CI/infra | 04-28, 05-30 (handoff) | `backlog/harbor-registry-spof.md` |
| Genesis cross-namespace NetworkPolicy blocks Verify-Target-Health storage check (jenkins ns → elohim-alpha ns); route via doorway proxy / NP exception / co-locate pod | k8s networking | 04-28 | `backlog/genesis-cross-ns-netpol.md` |
| Jenkins git-fetch intermittently broken (Maximum checkout retry; SIGTERM at ~78%); shallow/gitcache reference repo; kubelet eviction on node-type:edge | CI/infra | 04-30, 05-06, 05-07 | `backlog/jenkins-checkout-reliability.md` |
| Edge 1-hour wall-timeout on cold-start statefulset rollout (~43% flake, 3/7 builds); bump timeout / pre-pull / parallelize the 3 rollouts | CI | 05-16 | `backlog/edge-rollout-walltimeout.md` |
| elohim-epr multibranch job not provisioned (404); create item → `elohim/epr/Jenkinsfile` (soft-skip NOT_PROVISIONED already lands) | CI | 05-17, 05-22 | `backlog/elohim-epr-job-provision.md` |
| Genesis baseline UNSTABLE on dev (CellDisabled seed 503s; commit-hash drift; missing conductor-readiness.json) — conductor deploy/init regression | CI | 05-18 | `backlog/genesis-dev-conductor-regression.md` |
| Cluster pressure (intel-nuc 135% CPU; jessica edgenode OOM-flap; nodeAffinity rebalance) | recovery | 05-05 | `backlog/cluster-pressure-rebalance.md` |
| EPR/blob byte-replication across alpha peers not implemented (project-epr commitments + SPA blobHash invisible cross-peer); pick fan-out-seed vs substrate P2P-replication | storage/SSR | 04-29, 05-06, 05-23, 05-26 | `backlog/epr-blob-replication-direction.md` |
| Dependabot 170 vulns (109 high) on default branch — untriaged | cargo | 05-17 (multi) | `backlog/dependabot-triage.md` |

**CODE-DOMAIN (repo-level fixes a shift/PR can land):**
| Backlog item | Area | Recurrence | Suggested backlog dest |
|---|---|---|---|
| Tighten orchestrator measure: require lastBuild commit==HEAD AND non-NOT_BUILT result (NOT_BUILT/superseded falsely passes) | CI/orchestrator | 04-30, 05-14 | `backlog/orchestrator-measure-tightening.md` |
| Baseline advances only on confirmed-downstream-success (phantom-success on FAILURE during dispatch; lossy FAILURE-count grep) | CI/orchestrator | 05-16, 05-24 | `backlog/orchestrator-baseline-state-machine.md` |
| `build-manifest ⊆ orchestrator-strategy changePatterns` drift test (data/** missing was one of many) | CI/orchestrator | 05-04, 05-22 | `backlog/manifest-strategy-drift-test.md` |
| Dockerfile target-completeness lint ([[bin]]/[[bench]]/[[example]] + path-deps vs placeholder/COPY) at PR time | cargo | 05-17 | `backlog/dockerfile-completeness-lint.md` |
| Pre-push gherkin/cucumber grammar linter (empty-alternation, bare-continuation) before AST-abort drops whole E2E | genesis/a2o | 05-05, 05-17 | `backlog/gherkin-prepush-lint.md` |
| Pre-push hook bypasses cargo-target-pool (no CARGO_TARGET_DIR per-crate → ENOSPC); source cargo-pool key per-crate | cargo | 05-18 | `backlog/prepush-cargo-target-pool.md` |
| sccache+garage reliability OR retry-without-sccache fallback in pre-push (NoSuchKey poisons rustc output); SCCACHE_RECACHE runbook | cargo | 05-17, 05-18 | `backlog/sccache-garage-harden.md` |
| HUSKY=0 hook fix (top-of-hook early-exit) + correct CLAUDE.md doc-vs-behavior drift | cargo | 05-11, 05-18 | (fold into R4/§3a #4) |
| Strict console-error After-hook needs per-scenario `@allow-doorway-flake` allowlist (env flake masks passing assertions) | genesis/a2o | 05-07 | `backlog/a2o-console-error-allowlist.md` |
| ci-observer (Haiku) schema enforcement: no specific test names in `primary_failure.evidence`; estimatedDuration misuse on cascade builds | CI/tooling | 05-11, 05-16 | `backlog/ci-observer-schema-tighten.md` |
| Remove explicit `triggers{githubPush()}` (double-fire); reschedule cron off the late-EDT/PDT webhook window | CI/orchestrator | 05-05, 05-22 | `backlog/orchestrator-trigger-dedup.md` |
| SSR staging.yaml + prod.yaml carry 256Mi (will OOM on SSR roll); apply memory bump + startupProbe | SSR/doorway | 05-07 | `backlog/ssr-staging-prod-pod-floor.md` |
| Pipeline-level Jenkinsfiles ignore orchestrator step-level rebuild set (holochain rebuilds all stages) | CI/orchestrator | 05-05 | `backlog/jenkinsfile-step-rebuild-set.md` |
| heredoc-aware shellcheck pass in lint-jenkinsfiles-fast.sh (`//`-in-heredoc; CPS-scope static lint) | CI/orchestrator | 05-22, 05-24 | `backlog/jenkinsfile-heredoc-lint.md` |

---

## 4. Gated removals (the only D — guards carried forward verbatim)

**Total bodies → git: ~223** = ~116 burndown (junk + dead-arch + dupe) + ~107 shift artifacts.

### 4a. Burndown bodies (~116) — see burndown §D.0–§D.15 for the exact file lists
- **D.0** junk drawers (~77): `2026-05-14/` (8) after C1; `2026-05-15/` (56) after A.1+pointers; `2026-06-01/` (13), no records needed.
- **D.1–D.8** dead-arch (~25) after the matching A.2 record lands.
- **D.9–D.15** dupe (~18) after the matching B seed lands.

### 4b. Shift artifacts (~107) — grouped, bulk-approvable by slice
| Slice | Shifts | Files | Note |
|---|---|---|---|
| Apr 17–27 | 6 | 13 | RESCUE R1 (04-24), R2 (04-25) land first |
| Apr 28–May 3 | 6 | 17 | RESCUE R3 (04-28), R4 (04-30) land first |
| May 4–10 | 15 | 51 | RESCUE R5 (05-07), R6 (05-07 ssr), R7 (05-07 geo); **KEEP citation-guard** (below) |
| May 11–17 | 9 | 21 | RESCUE R8 (05-16) lands first |
| May 18–23 | 8 | 20 | all CLEAR |
| May 24–27 | 5 | 10 | RESCUE R9 (05-27) lands first |

### 4c. GUARDS — carried forward verbatim; NONE in the bulk-approve set

**Citation-guards (hard "do NOT remove"):**
1. **`.claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/brainstorm-prompt-followup.md`** — cited by two live memory entries (`project_ssr_anonymous_auth_context.md`, `project_ssr_is_compute_capability_claim.md`). Stays in the live tree; the other 5 files in that dir clear normally.
2. **junk-`2026-05-14` two enumerated rubrics** (ungrudging-service design rules; six stewardship principles) — git-only after clearing; the **C1 pointer** is the only thing telling a future author they exist. (Operator may promote into a durable `elohim-protocol/` ethics doc now — §6.)

**7 LEAVE-IN-PILE (still-live, do not touch):**
- `superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md` + `plans/2026-05-16-…` (live successor substrate).
- `superpowers/plans/2026-05-29-light-up-the-topology-sprint-kickoff.md` (active in-flight sprint).
- `plans/2026-05-22-value-scanner-content-audit.md` (active standing index).
- `superpowers/specs/2026-05-28-session-bridge-design.md` + `plans/2026-05-28-session-bridge-implementation.md` (net-new crate, NOT landed).
- `plans/2026-05-18-app-manifest-modularization.md` (FALSE DUPE — landed `$ref`-refactor).
- the live design/architecture specs being amended/pointed-into (experience-story EPR, attestation-consolidation, hub-boundaries, cradle-to-grave gradient, wave3 VF/hREA).
- `superpowers/sprints/2026-05-24-sweettest-stage-efficiency-w1-w2-w3-w5.md` (landing-record; own pass).

**6 HELD — needs-CI / verify before any seed/record claims "verified-stable":**
- Doorway SSR alpha pod — deploy BLOCKED on Harbor registry storage EIO (`cf53a76c2`); code+tests landed, do NOT assert in-cluster green. (B.2/D.10.)
- Conductor agent-info gossip — `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP` default false; 24h soak outstanding. (A.2.8/D.15.)
- Stewardship-chain wiring — substrate landed; task-#16 NOT in DNA. **HOLD the plan body (D.12 conditional).** (B.4.)
- Experience-story gate — structural-verified only; no CI evidence. (A.2.6.)
- Staged-intents vocabulary — on-disk + validates but UNEXERCISED end-to-end. Assert only "substrate landed". (B.6.)
- Topology a2o @wip — env-blocked browser-tier scenarios flagged HELD, not green. (A.2.2.)

**1 CONFLICT (operator judgment, default LEAVE):**
- `plans/2026-05-19-topology-resilience-qahal-synthesis.md` — body-to-git in topology dead-arch (D.2) AND still-live-leave-in-pile in hub-edge dupe (B.4); recently touched (mtime 2026-06-02). **Default: LEAVE in pile**; operator may retire under D.2 if confirmed fully superseded by realized GraphQL Viewer resolvers.

**1 federation-wiring-audit recovery hold (shifts §Flags):**
- `2026-05-27T18-50-federation-wiring-audit` — its named deliverable rode an **unmerged docs branch** (`claude/federation-wiring-audit-docs`), NOT in the sprints dir. Phase-1 routing landed via `91f300663`. If the operator still wants the audit doc, recover from the unmerged branch **before** the shift narration clears — otherwise only git history of the shift file remains.

---

## 5. Then: MemPalace

After A–D land, the live surface is clean: the pile is ~116 bodies lighter, the museum holds ~17 curated records, 7 canonical surfaces carry the dupe-cluster truth, the ~14-day shift tier has fully decomposed (recurring anti-patterns → canon + the A.3 museum record; open issues → backlog; bodies → git; **zero residue**), and every retired body's lesson is reachable via a planted pointer. **Re-point MemPalace ingestion at this cleaned surface and run a fresh re-mine** — it will now index curated history + canonical seeds + the recurring-anti-pattern museum record instead of decomposed junk-drawer bodies, duplicate plan threads, and one-off shift narration. The pile:canonical ratio dropping toward ~1.5–2× is exactly the signal that MemPalace is mining a curated corpus, not a junk pile. Re-mine should specifically pick up: the A.3 CI/orchestrator-anti-patterns museum record (so the drawer for "CI gotchas" fills with the frequency-ranked canon, not 40 scattered journal entries), the 7 dupe seeds, and the 8 dead-arch records.

---

## 6. What needs YOUR judgment

1. **R9 — correct the stale entry in place, or stand up a new one?** `project_ci_storage_topology.md` currently asserts the OPPOSITE of reality ("openebs-jiva… no nodeSelector/affinity tricks needed; don't reach for hostpath"). Recommendation: **fold the 2026-05-27 hostpath-pinning update in** (single source-of-truth; the stale text actively misleads). Alternative: standalone `feedback_hostpath_pvc_needs_node_pinning.md` + leave the canonical entry wrong. *Strong recommend: correct in place.*

2. **R3 — augment vs standalone.** Fold SQLite-multi-writer-contention into `project_alpha_edge_deploy_debugging_landmarks.md` (recommended; keeps the SQLite debugging knowledge in one landmark) or land as a new `feedback_seeder_sqlite_multiwriter.md`. *Lean: augment.*

3. **HUSKY=0 line in root `CLAUDE.md`.** The pnpm-workspace gotcha says "Bypass with `HUSKY=0 git push`" — confirmed non-functional (`core.hooksPath=.husky`). Recommendation: **correct it to `git push --no-verify`** as part of B.8 (it's a load-bearing doc-vs-behavior drift that has cost real shift time 4×). Touching root CLAUDE.md is a slightly heavier edit than a memory entry — your call on whether that lands now or as a follow-up backlog item.

4. **D.12 conditional — stewardship-chain plan body.** Retire the hub-edge spec under B.4, but the stewardship-chain plan's §8 task-#16 wiring is HELD-still-live. Recommendation: **retire hub-edge spec; HOLD the stewardship-chain plan** until the wiring lands or moves to a live successor. Confirm.

5. **CONFLICT: `2026-05-19-topology-resilience-qahal-synthesis.md`** (default LEAVE). Confirm it's fully superseded by the realized GraphQL Viewer resolvers before retiring under D.2, or keep the conservative LEAVE.

6. **federation-wiring-audit doc on the unmerged branch.** Do you want `genesis/docs/superpowers/sprints/2026-05-27-federation-wiring-audit.md` recovered from `claude/federation-wiring-audit-docs` before the `2026-05-27T18-50` shift narration clears? If not, it lives only in shift-file git history.

7. **Belt-and-suspenders ethics doc (optional).** The two junk-`2026-05-14` rubrics (ungrudging-service design rules; six stewardship principles) are git+pointer-only after clearing. If you want them durable, promote into an `elohim-protocol/` ethics doc NOW rather than relying on the C1 pointer. *Default: pointer is sufficient; promote only if you want them first-class.*
