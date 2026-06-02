# Shifts-Cleanup Proposal

> Generated from five date-slice triages (Apr 17 – May 27 2026) of `.claude/shifts/`.
> READ-ONLY triage. Nothing has been moved, edited, or deleted. Every removal and
> every memory write below is **operator-gated** and happens only on bulk approval.

## Summary

- **Shifts triaged: ~44 distinct shifts** across 5 slices (6 + 6 + 15 + 9 + 5 + 5; the corpus `.claude/shifts/` currently holds 121 top-level entries — the remainder fall outside these five date slices).
- **Files in the triaged slices: ~108** (13 + 17 + 51 + 21 + 10).
- **Disposition split:** **9 RESCUE-LESSON shifts**, **31 CLEAR-TO-GIT shifts**.
- **Lessons to rescue: 9** (7 new memory entries + 2 augmentations of existing entries). All are pure additions — **zero risk**.
- **Citation guard: 1 file** must NOT be cleared (cited by two live memory entries).
- **Files cleared to git on approval:** ~107 (all triaged files except the 1 citation-protected file). Git preserves the full record; only the live working-tree copies are removed.

---

## Lessons to rescue (ready to apply)

Each lesson is a pure addition. Seven create a new memory entry; two augment an existing entry.

### R1 — `feedback_sweettest_native_build_env.md` (NEW, type: feedback)
*Source: shift `2026-04-24T15-20-orchestrator-pipeline-unstable`, journal L22–28.*

> **DNA Integration (sweettest) is a NATIVE build and needs the native-build env, not the WASM env.** Three stacked failures, each revealing the next: (1) `datachannel-sys` panics `is cmake not installed?` — the Nix devShell must provide `cmake, pkg-config, clang, libclang.lib, openssl, zlib, libsodium` + `LIBCLANG_PATH` (fixed in `flake.nix` `b2c471f5`). (2) After cmake, the link fails `undefined reference to __getrandom_v03_custom` — the WASM `RUSTFLAGS=--cfg getrandom_backend="custom"` leaked into the native sweettest compile; the Jenkinsfile DNA Integration stage must **clear `RUSTFLAGS`** (`c6eb632a`). (3) After it links, the cold sweettest compile alone exceeds a 30-min stage budget — bump DNA Integration stage timeout to ~90min and pipeline to ~150min (`e29e2e6a`). Generalizes the CLAUDE.md "RUSTFLAGS Override Required" gotcha to the CI sweettest stage; pairs with `project_sweettest_cost_anatomy.md`.
> Cites: `elohim/holochain/dna/elohim/flake.nix`, `elohim/holochain/Jenkinsfile`.

### R2 — `feedback_dht_readback_use_to_app_option.md` (NEW, type: feedback)
*Source: shift `2026-04-25T00-43-clear-dna-integration-holochain`, objective description; fix landed `d68fe834`.*

> **Read DHT entries back with `record.entry().to_app_option::<T>()`, never `Entry::try_into() -> SerializedBytes`.** The `node_registry_coordinator` deserialize helpers used an `Entry::try_into()` SerializedBytes round-trip that serializes the `Entry::App(...)` *variant tag* into the bytes instead of unwrapping the inner app entry. On readback this fails with `Deserialize error: missing field 'node_id'` even though fixture and integrity structs have identical (23-field) shape — the shape is fine; the envelope is wrong. Sibling DNAs (mishpat, imagodei) used the correct `to_app_option` pattern. Distinct failure mode from `feedback_serde_json_value_breaks_zome_boundary.md` (that one is `serde_json::Value` at the zome *call* boundary; this is the *Entry variant envelope* on DHT readback).
> Cites: `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs`.

### R3 — SQLite multi-writer root cause (AUGMENT `project_alpha_edge_deploy_debugging_landmarks.md`, fold into its SQLite bullet as the upstream cause; or NEW feedback entry if operator prefers)
*Source: shift `2026-04-28T13-41-genesis-seeder-unstable`.*

> **SQLite multi-writer contention is the seeder's failure mode, not seeder bugs.** elohim-storage's SQLite pool ran rollback-journal mode + 5s busy_timeout. Under sustained load, many background writers — heartbeat, InfrastructureSignal subscriber, reconcile controller, import-handler drain — compete for the file-level write lock alongside HTTP handlers and the bulk seeder. The seeder's 100-row batch transaction deterministically loses to a concurrent writer around batch ~26, surfacing as ~100 SQLITE_BUSY ("database is locked") errors per run. Fix: WAL + synchronous=NORMAL + busy_timeout=30000 (readers stop blocking writers; writer-contention budget 6×). Validated: genesis #956/#957 reported 0 seeder errors and the defensive retry layer never fired — confirming root cause was storage-side. Corollary (the related landmark): set `busy_timeout` BEFORE `journal_mode=WAL` in `on_acquire`, or contended r2d2 warm-up hits immediate SQLITE_BUSY → pool-init crashloop.
> Files: `elohim/elohim-storage/src/db/mod.rs`.

### R4 — Jenkins 3-layer checkout + `elohim-holochain`→DNA-Jenkinsfile job-alias trap (NEW, type: feedback)
*Source: shift `2026-04-30T22-30-orchestrator-781-recover`.*

> **Jenkins checkout on this 316MB / 65k-object repo needs three layers, and `elohim-holochain` loads the DNA Jenkinsfile.** Repeated SCM failures (#1179/#783/#785) all died at ~10:15 elapsed with `git-remote-https died of signal 15` — the git-plugin's 10-min per-attempt default firing on the implicit `Declarative: Checkout SCM` (which uses job-level SCM config, NOT the Jenkinsfile CloneOption). Fix is three layers: (1) `skipDefaultCheckout(true)` in `options{}` bypasses the implicit checkout; (2) `CloneOption shallow:true depth:200 timeout:30` (~10× size reduction, 3× budget); (3) `honorRefspec:true` + explicit `+refs/heads/${BRANCH}:refs/remotes/origin/${BRANCH}` fetches one branch, not all heads. **Job-alias trap:** the multibranch job named `elohim-holochain` is configured to load `elohim/holochain/dna/Jenkinsfile` (DNA pipeline), NOT `elohim/holochain/Jenkinsfile` (edge) — verify via the console's `Obtained …/Jenkinsfile from <sha>` line. depth:200 is conservative; depth:50 usually suffices unless a pipeline has been red for days.
> Cites: `genesis/orchestrator/Jenkinsfile`, `elohim/holochain/dna/Jenkinsfile`.

### R5 — Native `<dialog>` top-layer modal fix (NEW, type: feedback / frontend-Angular)
*Source: shift `2026-05-07-feedback-dialogue-panel`.*

> **Native `<dialog>` + `showModal()` is the fix for "modal slides behind / off-page" — `position: fixed` is NOT viewport-relative under a transformed ancestor.** Any ancestor with `transform`, `filter`, `perspective`, `will-change`, or `contain: paint` becomes the containing block for `position: fixed` descendants, breaking viewport centering (lesson-view's `overflow:hidden` + path-navigator transforms caused it). The robust fix is the native `<dialog>` element with `dialog.showModal()`, which renders into the browser **top layer** — above all stacking contexts, unaffected by ancestor transforms/overflow, with native `::backdrop` dimming and UA auto-centering. Migration gotchas: native `<dialog>` has no traditional z-index (`getComputedStyle().zIndex` → `"auto"` → NaN — assert the `:modal` pseudo-class instead); synthetic `KeyboardEvent('Escape')` does NOT trigger the UA Escape handler (test via the `(close)` event / `dialog.close()`); backdrop-click is detected via `event.target === dialogEl`.

### R6 — Angular-19-on-doorway SSR build-glue cluster (NEW, type: feedback)
*Source: shift `doorway-ssr-deliver-2026-05-07T23-37`.*

> **Angular-19-on-doorway SSR build-glue gotchas (the 13-fix unblock cluster).** (1) The render unblock was wiring the **`fetch` shim into the V8 isolate** — `deno_core`'s `JsRuntime::with_shims()` does NOT include `fetch`; `with_full_shims(fetcher)` does. The Angular bundle awaits HTTP during bootstrap (ConfigService/AuthService); with no `fetch` global those hang forever past any timeout (the `elohim-render/src/angular.rs:99-102` "Task 14+" TODO was the actual blocker). (2) **Angular 19's application builder with SSR emits `index.csr.html`, not `index.html`** — nginx-ingress base images carrying their own `index.html` silently serve the base Welcome page; requires `rm -rf` of the nginx html dir before COPY. (3) **pnpm `--filter "elohim-app..."` does NOT walk tsconfig-path-aliased workspaces** — `@elohim/service` is referenced via `tsconfig.json` paths (not `package.json` deps), so its peerDeps weren't installed; needs an explicit `--filter "@elohim/service..."`. (4) **`shamefully-hoist=true` is required** for Angular pnpm monorepos; the docker build context doesn't COPY repo-root `.npmrc` (carries Nexus auth), so a stripped inline write is needed.

### R7 — Che-headless cannot render WebGL (AUGMENT `project_che_browser_feedback_loop.md`, 1–2 sentence addition)
*Source: shift `geospatial-cybersyn-deliver-2026-05-07T02-06` (findings live only in its journal — no sprint-result; without rescue the constraint is lost).*

> **WebGL features (MapLibre/Three.js/WebGL charts) cannot be visually verified headless in Eclipse Che** — ANGLE/EGL init fails (`xcb_connect failed, error 1`; `EGL_NOT_INITIALIZED`) because the Nix devcontainer has no X display; SwiftShader fallback also fails (GL stack still inits via xcb); no `xvfb`, and `apt-get`/`sudo` are blocked. Visual proof for any WebGL surface must go off-Che (operator's own browser at `localhost:4200`, alpha, or a CI browser image with WebGL). Non-WebGL DOM/layout defects ARE still catchable headless (this shift caught + fixed a `height:100%`→`100dvh` collapse that stunted the map canvas to a 300px strip).

### R8 — CI runs sweettests with `--run-ignored all`; `#[ignore]` is a no-op silencer (NEW, type: feedback)
*Source: shift `2026-05-16T05-00-three-pipelines-green`, iter 2.*

> **CI runs sweettests with `--run-ignored all` — `#[ignore]` is a no-op as a CI silencer.** The DNA sweettest stage invokes `cargo nextest run --release --run-ignored all` (`elohim/holochain/dna/Jenkinsfile:~632`). Every sweettest carries `#[ignore]` so local runs skip them; CI deliberately overrides to run them all. Consequence: adding `#[ignore]` to quarantine a broken sweettest does nothing in CI — the test still runs and still fails. (Cost a full ~75-min holochain build cycle when `#[ignore]` on `proposal_round_trips_across_agents` was a no-op and the test had to be deleted instead.) To remove a sweettest from the CI run you must delete it or change the Jenkinsfile invocation — not annotate it. Complements `feedback_cargo_nextest_installed.md` (which notes the local default-skip behavior but not the CI override).
> Cites: `elohim/holochain/dna/Jenkinsfile`.

### R9 — hostpath PVC needs deterministic node pinning (AUGMENT `project_ci_storage_topology.md` — that entry is **factually stale**: it still documents openebs-jiva as live CI cache storage)
*Source: shift `2026-05-27T00-14-first-clean-post-migration-dev-build`. Append as a dated update.*

> **## Update 2026-05-27 — migrated openebs-jiva → openebs-hostpath; hostpath needs deterministic node pinning**
> The Holochain/edge CI cache PVCs were cut over from `openebs-jiva-csi-default` (replicated, network-attached, schedule-anywhere) to `openebs-hostpath` (node-local). Hostpath is `volumeBindingMode: WaitForFirstConsumer`, so on a MULTI-NODE pool the PVC binds to whichever node the first consumer pod lands on — a scheduler lottery. When that node is resource-contended, the PVC pins there and subsequent pods can't schedule: pods 1–5 time out ~1000s each on `volume node affinity conflict` (Phase B; Phase A reads `0/7 nodes... didn't find available persistent volumes to bind`), and the build only proceeds when cluster pressure shifts (observed ~1h thrash before pod 6 scheduled, elohim-edge #1010). Anti-pattern AP-009 (pod-scheduling thrash).
> **Fix:** make hostpath binding deterministic — add a `kubernetes.io/hostname` nodeAffinity to each pipeline's Jenkinsfile pod spec. Operator assignment: elohim-edge → `thinkc-p1s`; elohim-holochain (DNA) + steward → `intel-nuc`. Tradeoff: if a PVC had previously bound to a DIFFERENT node, the pin triggers an immediate FailedScheduling and the operator must delete+reapply that PVC. Dead `nix-store` volume declarations are intentionally left in the Jenkinsfiles as breadcrumbs for future sccache work. (Co-symptom on #1067: the Angular alpha bundle exceeded its 4MB error cap at 7.05MB → bumped to 8MB warn / 9MB error as a pragmatic unblock; not a storage issue.)
> **Note:** this also corrects the entry, which currently states the OPPOSITE ("openebs-jiva… no nodeSelector/affinity tricks needed"). If the operator prefers not to touch the canonical entry, the lesson stands alone as a new `feedback_hostpath_pvc_needs_node_pinning.md` — but folding it in keeps the CI-storage source-of-truth in one place.

---

## Clear to git (gated removal)

These shift artifacts carry no un-captured lesson (each cross-checked against `.claude/memory/`; already-harvested, superseded, or pure narration). Git preserves the full record; only live working-tree copies are removed. **Operator approves in bulk.** The files belonging to the 9 RESCUE shifts above also clear once their lessons are applied.

**EXCEPTION — citation guard (do NOT remove):**
`/projects/elohim/.claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/brainstorm-prompt-followup.md`
— cited by two live memory entries (`project_ssr_anonymous_auth_context.md`, `project_ssr_is_compute_capability_claim.md`). Clearing it creates dead citations. The other 5 files in that directory clear normally.

### Slice Apr 17 – Apr 27 (6 shifts, 13 files)
- `2026-04-20T02-30-elohim-edge-docker-green` (3) — CLEAR
- `2026-04-23T19-10-epr-2c-push-lands-green` (2) — CLEAR (aborted at readiness)
- `2026-04-24T15-20-orchestrator-pipeline-unstable` (3) — **RESCUE R1**, then clear
- `2026-04-25T00-43-clear-dna-integration-holochain` (1) — **RESCUE R2**, then clear
- `2026-04-26T03-21-dna-pass-orchestrator-finishes` (1) — CLEAR
- `2026-04-27T03-56-all-pipelines-green-or-unstable` (3) — CLEAR

### Slice Apr 28 – May 3 (6 shifts, 17 files)
- `2026-04-28T03-13-orchestrator-clean-deploy` (3) — CLEAR
- `2026-04-28T13-41-genesis-seeder-unstable` (3) — **RESCUE R3**, then clear
- `2026-04-29T00-15-alpha-blob-deploy-as-expected` (3) — CLEAR
- `2026-04-30T22-30-orchestrator-781-recover` (3) — **RESCUE R4**, then clear
- `2026-05-01T19-49-clear-dna-integration-bootstrap-steward` (2) — CLEAR
- `2026-05-03T18-19-orchestrator-805-pipelines-unstable` (3) — CLEAR

### Slice May 4 – May 10 (15 shifts, 51 files)
- `2026-05-04T22-51-alpha-pipelines-green-no-shem` (3) — CLEAR
- `2026-05-05T17-20-drive-genesis-e2e-verification-quality` (2) — CLEAR (superseded)
- `2026-05-05T20-30-verify-and-finish-genesis-e2e-verification-quality` (3) — CLEAR
- `2026-05-06T02-44-rca-genesis-browser-failure-classes` (4) — CLEAR
- `2026-05-07-feedback-dialogue-panel` (1) — **RESCUE R5**, then clear
- `2026-05-07T00-47-storybook-stage-green` (3) — CLEAR
- `2026-05-07T14-15-topology-substrate-completion-m1-handoff` (1) — CLEAR
- `2026-05-09T16-30-orchestrator-clean-cascade` (3) — CLEAR
- `alpha-ingress-static-asset-cluster-2026-05-06T19-15` (2) — CLEAR
- `doorway-conductor-stale-mapping-2026-05-06T17-40` (3) — CLEAR
- `doorway-ssr-deliver-2026-05-07T23-37` (6) — **RESCUE R6**; clear 5 files, **KEEP `brainstorm-prompt-followup.md`** (citation guard)
- `geospatial-cybersyn-deliver-2026-05-07T02-06` (5, incl 2 PNG) — **RESCUE R7**, then clear
- `light-up-the-topology-deliver-2026-05-06T04-57` (3) — CLEAR
- `light-up-the-topology-deliver-cont-2026-05-06T08-30` (3) — CLEAR
- `light-up-the-topology-deliver-2026-05-07T04-20` (9, incl 5 PNG) — CLEAR

### Slice May 11 – May 17 (9 shifts, 21 files)
- `2026-05-11T02-24-fix-sccache-unbound-on-elohim-holochain` (2) — CLEAR
- `2026-05-14T23-37-clean-delivery-memory-substrate` (3) — CLEAR
- `2026-05-15-recovery-m4-deliver-pending` (1) — CLEAR
- `2026-05-15T03-43-fix-attestation-cid-decode` (2) — CLEAR
- `2026-05-15T23-53-recovery-m4-orchestrator-sail-through` (3) — CLEAR
- `2026-05-16T05-00-three-pipelines-green` (2) — **RESCUE R8**, then clear
- `2026-05-17T03-29-land-graph-native-push` (2) — CLEAR
- `2026-05-17T15-57-rca-orchestrator-963-graph-failure` (3) — CLEAR
- `2026-05-17T20-47-genesis-cucumber-parse-recovery` (3) — CLEAR

### Slice May 18 – May 23 (8 shifts, 20 files)
- `2026-05-18T15-30-ci-propagation-of-plan-3a-and-resilience` (3) — CLEAR
- `2026-05-21T00-30-pipelines-unstable-or-better` (2) — CLEAR
- `hosted-steward-portal-deliver-2026-05-22T01-42` (2) — CLEAR
- `2026-05-22T02-40Z-lift-elohim-edge-storage-dockerfile` (2) — CLEAR
- `2026-05-22T10-45-orchestrator-dev-unstable-or-better` (4) — CLEAR
- `2026-05-22T18-48-validate-ci-cd-gap-close-push` (3) — CLEAR
- `2026-05-23T05-25-alpha-landing-page-dual-doorway` (2) — CLEAR
- `2026-05-23T19-00-orchestrator-and-genesis-unstable-or-better` (2) — CLEAR

### Slice May 24 – May 27 (5 shifts, 10 files)
- `2026-05-24T03-30-sweettest-efficiency-w1-w2-w3` (2) — CLEAR
- `2026-05-26T08-30-deliver-epr-app-iter0` (1) — CLEAR
- `2026-05-26T08-35-shift-epr-app-delivery` (2) — CLEAR
- `2026-05-27T00-14-first-clean-post-migration-dev-build` (2) — **RESCUE R9**, then clear
- `2026-05-27T18-50-federation-wiring-audit` (3) — CLEAR

---

## Flags

Items that genuinely want operator judgment or touch still-active work:

1. **R3, R7, R9 are AUGMENTATIONS of existing entries, not new files.** R9 in particular *corrects a factually-stale entry* (`project_ci_storage_topology.md` still describes the superseded openebs-jiva topology and explicitly says the opposite of the current hostpath-pinning requirement). Folding R9 in is the higher-leverage move (single source of truth), but the operator may prefer a standalone `feedback_hostpath_pvc_needs_node_pinning.md` to avoid editing the canonical entry. R3 may either fold into `project_alpha_edge_deploy_debugging_landmarks.md` or land as a new feedback entry — operator's call.

2. **Citation guard is the one hard "do not remove."** `doorway-ssr-deliver-2026-05-07T23-37/brainstorm-prompt-followup.md` is load-bearing for two live memory entries. It stays in the live tree even though the rest of its shift clears. (If the operator wants it gone too, the two citing entries must first be re-pointed — but recommendation is to keep it.)

3. **Still-active / forward-referenced work** (clearing the shift narration is safe — the work itself lives in code/commits/memory, not the shift file — but noting for awareness):
   - **EPR projection / serving chain** (shifts `2026-05-26T08-30-deliver-epr-app-iter0`, `2026-05-26T08-35-shift-epr-app-delivery`): the diagnosis thread that landed weeks later. The durable lessons are already in the *later* `project_epr_projection_serving_chain.md` + `project_alpha_edge_deploy_debugging_landmarks.md`. Safe to clear the early diagnosis shifts.
   - **federation-wiring-audit** (`2026-05-27T18-50`): its named deliverable `genesis/docs/superpowers/sprints/2026-05-27-federation-wiring-audit.md` rode an **unmerged docs branch** (`claude/federation-wiring-audit-docs`) and is NOT present in the sprints dir. Phase 1 routing landed via the operator's convergent commit `91f300663`. If the operator still wants that audit doc, it must be recovered from the unmerged branch *before* the shift narration is cleared — otherwise the only remaining trace is git history of the shift file.
   - **recovery-m4** thread (`2026-05-15*` shifts): state lives in `project_epr2b_recovery_m4_convergence` — shift files are scaffolding/observe-only, safe to clear.

4. **No genuinely ambiguous rescue/clear calls remain.** Every CLEAR was content-cross-checked against memory (not just filename match) and confirmed already-captured or low-value narration; every RESCUE carries a quantified, parameter-bearing constraint confirmed absent from `.claude/memory/`.
