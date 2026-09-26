---
title: One head, delivered — the app release reaches the fleet by election, the pipeline chain stops waiting on itself, and the apex name is ready to outlive its doorway
id: one-head-delivered-sprint-plan
status: Draft
class: process-meta
process_subdomain: ci
domain: D8
sprint: one-head-delivered-2026-09-26
serves: [dataplane-convergence, push-delivers-within-budget, doorway-failover]
cites:
  - "native-delivery-sprint-plan | the 09-24 sprint whose open rows N6, A4 and B4 this plan drains on the fleet | sha256:3b4d266f8f3cfb91 | path: genesis/docs/superpowers/plans/2026-09-24-native-delivery-sprint-plan.md"
  - "serving-edge-failover-balance-stream-campaign-plan | the failover campaign; stories 2.2 and 2.3 are Lane F, 1.4 is its precondition | sha256:a63d475974424ace | path: genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md"
  - "evidence-ladder-push-left | evidence ladder §8 ranked moves 1, 2 and 5 are Lanes C3, C2 and D | sha256:c8bdfe8ba6c28790 | path: genesis/docs/superpowers/specs/2026-08-10-evidence-ladder-push-left-design.md"
  - "submodule-pin-attestation-gate-design | the attested pin gate Lane D1 moves the rakia pin through | sha256:79d9af05d5287df8 | path: genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md"
  - "peer-executed-stage-design | adam as a stage executor stays out of scope here; named as the next sprint | sha256:41d3536b4a97b0f8 | path: genesis/docs/superpowers/specs/2026-09-08-peer-executed-stage-design.md"
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md
  - doorway/doorway-service/.epr-meta/doorway-failover.habit.md
  - genesis/data/timeline/backlog/arch-workspace-discipline-backlog.md
  - genesis/data/timeline/backlog/fleet-standing-celldisabled-one-third-party-cell.md
informed-by: [genesis/orchestrator/README.md, /CLAUDE.md, genesis/docs/content/elohim-protocol/architecture/MAP.md]
derived_from: [native-delivery-sprint-plan, serving-edge-failover-balance-stream-campaign-plan]
---

# One head, delivered — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Opus implements; the controller rules and reviews.

**Goal:** the fleet serves ONE current app head on both doorways, delivered by election through one doorway; the orchestrator chain drops from ~300 to ≤180 minutes without waiting on measurements or on unrelated jobs; and the household proves the two owed failover checks plus story 2.2 so the anycast sprint (campaign 2.3) has only the operator's DNS act left.

**Architecture:** three lanes with disjoint write-sets. Lane D finishes native-delivery N6 on the fleet (rakia pin → root Jenkinsfile → channel follow → verify-class receipt → retire the per-host path). Lane C restructures dispatch and gates on existing rails (row 19 closure abort, edge dispatched beside DNA with two in-pipeline barriers, Dataplane Validation as a sibling validate-only run, roll gate settled-at-baseline). Lane F makes the failover harness honest (sibling symmetry), runs the owed checks, lands campaign story 2.2 on the household, and delivers the 2.3 manifests + runbook for the operator.

**Tech Stack:** Jenkins declarative Groovy (root, edge, DNA, orchestrator Jenkinsfiles — bash bodies in `scripts/ci/*.sh`, heredoc-free helpers), bash + `node --test` (`genesis/orchestrator/*.test.mjs`, `scripts/ci/*.test.sh`), a2o cucumber-js + Playwright (`genesis/a2o`), Rust doorway-service (story 2.2 only), rakia submodule pin.

**Spec:** this plan composes, never forks: `2026-09-24-native-delivery-sprint-plan.md` (Lane N6, A4, B4 are its open rows — this plan drains them), `2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md` (stories 2.2, 2.3), evidence-ladder spec §8 ranked moves 1/2/5.

## Position

- **MAP-PATH:** domain **D8** (Web2 projection & doorway) for the serving-edge work; the delivery mechanism is D5/D7 (elected content, `runtime-upgrade-propagation`), already landed on the household. Pillar walk: doorway → `public_observer/epic.md` → D8 seeds. No MAP gap-ledger row closes here.
- **ROADMAP-PRIORITY:** the roadmap (regenerated 2026-09-09) predates this sprint's habits; the live ledger is the authority: `push-delivers-within-budget` delivery-series 1/10, p90 271.5 min, red; `dataplane-convergence` red on the federation-deploy check; `doorway-failover` red, not active. The WIP fence stays at its two actives; Lane F accrues evidence on `doorway-failover` without activating it until D4's fleet receipt lands.
- **Fleet reading that shaped the lanes (edge #1486 ← orchestrator #1912, app #1727–#1729, 2026-09-26):** the conductor pin `e0bfc6c7a` DID roll (`elohim-edgenode/main` #33, digest 6e491a36…), so the CellDisabled mechanism-(1) cure is on the fleet and adam settled in the roll gate. Four of six peers burned their whole 450–599 s roll-gate deadline on `no-movement(… divergentAnchor 55->55, sweeps 1->2)` — the settled-at-baseline shape C4 names. Dataplane Validation ended `FLEET-CHURNING: deadline 2700s exceeded — DID NOT MEASURE` after `ADVISORY-SAME-HEAD-DIFFERENT-BYTES` (A=9a0bae…, B=3bf228… moved 19:03Z that day). The three app builds refused in ~6 s each on `FLEET-NOT-READY https://elohim.host face=storage-refused` while `FLEET-READY https://alpha.elohim.host` — election needs ONE ready doorway, so D2 publishes through the ready one instead of refusing on the pair.
- **BLOCKED-BY-ENV:** none of the tasks below needs `shem`. `owned-substrate` is granted by the household lane itself. Campaign 2.3's DNS act is operator-owned by design (deliverable = manifests + runbook, not the apply).
- **Complementary work captured, not swallowed:** (a) the landing page links ten epic/concept slugs the household prologue does not seed (`/epr-head/<slug>` 404 on every household doorway) → backlog row, Lane F1 step 6; (b) backlog row 18 (edge BuildKit dep-layer PVC) stays operator-owned; (c) adam as measurement peer (four operator items) stays the next sprint per the 09-24 plan's "Next sprint captured".

## Global Constraints

- Local first: every task proves on the household mesh (`just mesh`, `just test mesh <feature>`) or in unit tests before the ONE Jenkins pass at the end of each lane; Jenkins confirms, never discovers.
- One push per batch, never mid-build; commit-only during a lane; the pre-push gate runs (`HUSKY=0` only when the batch already carries its receipt).
- Jenkinsfiles stay heredoc-free in helpers; bash bodies go in `scripts/ci/<name>.sh` with a `<name>.test.sh`; the root Jenkinsfile sits near the 64 KB CPS ceiling — every root change must keep the `Publish and Verify App Delivery` block thin (it calls helpers).
- Never two fleet stories in one deploy (campaign plan rule): Lane D's fleet receipt run and Lane C's first re-leveled run are separate pushes.
- `cargo nextest` is not installed; use `cargo test`. Rust work (Lane F3 only) runs `just gate doorway-service`.
- Habit deltas: every task ends with ONE line in the habit atom it serves, then `.claude/scripts/habits-project.py`. Status flips need evidence (build number, receipt path, run id).
- Blind-reader loop is mandatory for any NEW or edited `.feature` under `genesis/a2o` (its `.epr-meta` requires it); step-only changes do not trigger it.

## Review Focus

1. **A release the channel already carries** (rebuilt-but-identical dists): `publish-app-release.sh` must print `APP-RELEASE-CURRENT` and the stage must stay green with no adoption wait — pinned in D2 step 4.
2. **A peer that never adopts inside the bound** (adam CellDisabled window): `verify-app-adoption.sh` exit 3 must be UNSTABLE ("delivered, not yet proven"), never FAILURE, and never re-roll edge — pinned in D2 step 4 and D4.
3. **A failed leaf with no dependents** (row 19 shape, `elohim-eprfs`): the wave must still run edge/app/genesis and end FAILURE with `__global__` held — pinned in C1 step 1.
4. **DNA sweettest fails after edge already built against the candidate happ**: edge must refuse to deploy (no fleet roll on an unattested DNA) and say which DNA run refused it — pinned in C2 step 5.
5. **A sibling doorway that is honestly worse than the primary** (an asset only the primary serves): the symmetric assertion must still fail, naming the URL the primary served and the sibling did not — pinned in F1 step 2.

---

## Lane D — the app release reaches the fleet by election (serves dataplane-convergence; N6 fleet leg)

### Task D1: rakia pin carries the `app-bundle` release class through the attested gate

**Files:**
- Modify: gitlink `elohim/rakia` (d3329b2 → 720c132 or its fast-forward merge on rakia `main`)
- Test: `node genesis/orchestrator/gate-runner.mjs` attested path (`gate-attest.mjs`), `elohim/rakia/build-manifest.json` run.kind attested, check `test`

**Interfaces:**
- Produces: `elohim/rakia/schemas/v1/release-manifest.schema.json` enum carrying `app-bundle` at the pinned commit, so `genesis/a2o/scripts/epr-release-package.ts` runs without `ELOHIM_RAKIA_ROOT`.

- [ ] **Step 1: confirm 720c132 descends from the current pin and is not yet on the remote**

```bash
cd /projects/elohim/elohim/rakia && git fetch -q origin
git merge-base --is-ancestor d3329b2 720c132 && echo FF-OK || echo NOT-FF
git branch -r --contains 720c132   # expected: empty — the forge has never seen it
```
Expected: `FF-OK`, empty remote list. If NOT-FF, rebase `feat/app-bundle-class` onto `origin/main` first and re-run.

- [ ] **Step 2: push the branch and fast-forward rakia `main`** (a `--dry-run` to the same remote succeeded 2026-09-26; the forge must SEE the commit or the attested gate refuses)

```bash
cd /projects/elohim/elohim/rakia
git push origin 720c132:refs/heads/feat/app-bundle-class
git push origin 720c132:refs/heads/main   # fast-forward only; refuse otherwise
```
Expected: both refs updated. The rakia `ci` workflow (`.github/workflows/ci.yml`, check name `test`) starts on the push.

- [ ] **Step 3: wait for the `test` check on 720c132 to be green**

```bash
gh api repos/ethosengine/rakia/commits/720c132/check-runs --jq '.check_runs[] | "\(.name) \(.status) \(.conclusion)"'
```
Expected: `test completed success`. Red → stop the lane; the fix belongs in rakia, not here.

- [ ] **Step 4: move the pin in elohim and run the attested gate**

```bash
cd /projects/elohim && git -C elohim/rakia checkout -q 720c132
git add elohim/rakia
GATE_ORACLE=rakia node genesis/orchestrator/gate-runner.mjs --changed elohim/rakia 2>&1 | tail -5
```
Expected: `attested: passed` (or `attested: claimed` on an unreachable forge — acceptable, say so in the commit).

- [ ] **Step 5: drop the household override and prove the packager on the pin**

```bash
cd /projects/elohim/genesis/a2o && unset ELOHIM_RAKIA_ROOT
pnpm exec vitest run scripts/__tests__/epr-release-package.spec.ts 2>&1 | tail -5; echo "EXIT=$?"
```
Expected: PASS, EXIT=0, with the schema read from `elohim/rakia`.

- [ ] **Step 6: commit**

```bash
git commit -m "build(rakia): pin 720c132 — release-manifest carries the app-bundle class (native-delivery N6, attested)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

### Task D2: the App pipeline publishes one release and measures adoption

**Files:**
- Modify: `Jenkinsfile` (root) — `stageAndVerifyAllBundles` (~L600) and the stage at L1614–1641; add helper `publishReleaseAndVerifyAdoption(doorwayEprUrls, adminKey)`
- Create: `scripts/ci/app-release-stage.sh` (bash body: publish → adoption verify → junit lines), `scripts/ci/app-release-stage.test.sh`
- Test: `bash scripts/ci/app-release-stage.test.sh`, `bash scripts/ci/publish-app-release.test.sh`, `bash scripts/ci/verify-app-adoption.test.sh`, orchestrator `lint-jenkinsfiles`

**Interfaces:**
- Consumes: `scripts/ci/publish-app-release.sh <doorway-url> <manifest-out>` (exit 0 published/current, 2 refused); `scripts/ci/verify-app-adoption.sh <doorway-url> <release-cid> <peer>…` (0 adopted, 1 cannot-boot, 3 bound elapsed); env `STORAGE_API_KEY_ADMIN`, `APP_RELEASE_CHANNEL` (default `runtime:app-bundle:alpha:dev`), `APP_RELEASE_BUNDLES`.
- Produces: stage outcome keys `release|published`, `release|cid`, `adopt|<peer>`; a Jenkins param `APP_DELIVERY_LEGACY` (default `false`) that selects the retired per-host path for one release only.

- [ ] **Step 1: write the failing test for the stage body** (`scripts/ci/app-release-stage.test.sh`, same fixture style as `publish-app-release.test.sh`: stub `publish-app-release.sh` and `verify-app-adoption.sh` on PATH)

```bash
# case 1: publish prints APP-RELEASE-PUBLISHED, both peers adopt → exit 0, stdout has
#         APP-RELEASE-STAGE released=<cid> adopted=matthew,jessica
# case 2: publish prints APP-RELEASE-CURRENT → exit 0, no verify call (idempotent by content)
# case 3: verify exits 3 for jessica → exit 3, line APP-RELEASE-STAGE released=<cid> adopted=matthew pending=jessica
# case 4: verify exits 1 (cannot boot) → exit 1, line APP-RELEASE-STAGE refused=app_bundle_cannot_boot
# case 5: publish exits 2 → exit 2, line APP-RELEASE-STAGE refused=<publish's last line>
```
Run: `bash scripts/ci/app-release-stage.test.sh` → expected FAIL (script absent).

- [ ] **Step 2: write `scripts/ci/app-release-stage.sh`**

```bash
#!/bin/bash
# app-release-stage.sh — the App pipeline's ONE native delivery step: publish this
# build's bundles as one release through ONE doorway, then measure adoption per
# peer. Never a per-host write. Exit: 0 delivered+adopted · 1 refused (cannot boot)
# · 2 refused (publish) · 3 delivered, not yet proven (adoption bound elapsed).
# Usage: app-release-stage.sh <doorway-url> <manifest-out> <peer>[,<peer>...]
set -euo pipefail
DOORWAY="$1"; MANIFEST="$2"; PEERS="${3//,/ }"
out="$(bash "$(dirname "$0")/publish-app-release.sh" "$DOORWAY" "$MANIFEST")" || { echo "APP-RELEASE-STAGE refused=${out##*$'\n'}"; exit 2; }
echo "$out"
line="$(printf '%s\n' "$out" | grep -E '^APP-RELEASE-(PUBLISHED|CURRENT) ' | tail -n1)"
cid="${line##*release=}"
case "$line" in APP-RELEASE-CURRENT*) echo "APP-RELEASE-STAGE released=$cid adopted=current"; exit 0;; esac
set +e; vout="$(bash "$(dirname "$0")/verify-app-adoption.sh" "$DOORWAY" "$cid" $PEERS)"; rc=$?; set -e
echo "$vout"
adopted="$(printf '%s\n' "$vout" | sed -n 's/^APP-ADOPTED \([^ ]*\).*/\1/p' | paste -sd, -)"
pending="$(printf '%s\n' "$vout" | sed -n 's/^APP-NOT-ADOPTED \([^ ]*\).*/\1/p' | paste -sd, -)"
case "$rc" in
  0) echo "APP-RELEASE-STAGE released=$cid adopted=$adopted"; exit 0;;
  1) echo "APP-RELEASE-STAGE refused=app_bundle_cannot_boot"; exit 1;;
  3) echo "APP-RELEASE-STAGE released=$cid adopted=$adopted pending=$pending"; exit 3;;
  *) echo "APP-RELEASE-STAGE refused=verify-exit-$rc"; exit 2;;
esac
```
Run the test → expected PASS for all five cases.

- [ ] **Step 3: wire the helper into the root Jenkinsfile** — in `stageAndVerifyAllBundles` (the body behind the stage at L1614), branch on `params.APP_DELIVERY_LEGACY`. Readiness changes meaning on the native path: `fleetWriteReady` (L648–676) today refuses when ANY doorway is not ready (app #1727–#1729: `FLEET-NOT-READY https://elohim.host face=storage-refused` while alpha was READY). Election needs ONE writable doorway, so on the native path the helper keeps the READY doorways from `fleet-write-readiness.sh`'s per-URL lines and refuses only when none is ready; the publish goes through the first ready one:

```groovy
// Native path (native-delivery N6): one release, peers adopt. The retired per-host
// path stays behind APP_DELIVERY_LEGACY for exactly one release as rollback.
def publishReleaseAndVerifyAdoption(List doorwayEprUrls, String adminKey, Map outcomes) {
    def peers = 'matthew,jessica'   // the alpha serving pair behind doorway A / B
    def manifest = "${env.WORKSPACE}/app-release-manifest.json"
    def ready = readyDoorways(doorwayEprUrls)   // FLEET-READY lines of fleet-write-readiness.sh, in order
    if (ready.isEmpty()) { unstable('app release refused: no doorway is write-ready (deploy-intent.json archived for the re-dispatch)'); outcomes['release|delivered'] = 'not-ready'; return }
    def rc = 0
    withEnv(["STORAGE_API_KEY_ADMIN=${adminKey}", "APP_RELEASE_BUNDLES=${appReleaseBundles()}"]) {
        rc = sh(returnStatus: true, script: "bash '${env.WORKSPACE}/scripts/ci/app-release-stage.sh' '${ready[0]}' '${manifest}' '${peers}'")
    }
    archiveArtifacts artifacts: 'app-release-manifest.json*', allowEmptyArchive: true
    if (rc == 0) { outcomes['release|delivered'] = 'adopted'; return }
    if (rc == 3) { unstable('app release published; a peer has not adopted inside the bound (delivered, not yet proven)'); outcomes['release|delivered'] = 'pending'; return }
    if (rc == 1) { error('Deploy refused: the release cannot boot on a peer — fix the build, do not re-run') }
    unstable("app release refused (exit ${rc}) — see APP-RELEASE-STAGE line"); outcomes['release|delivered'] = 'refused'
}
```
`readyDoorways(urls)` runs `scripts/ci/fleet-write-readiness.sh` once (it already prints one `FLEET-READY <url>` / `FLEET-NOT-READY <url> face=…` line per doorway) and returns the READY urls; the legacy path keeps its all-or-nothing rule. `appReleaseBundles()` returns the same `<slug>:<kind>:<dist-dir>[:<mount>]` list `stageSpaBlobs` derives its bundles from (read `bundles` at ~L600 and format it; one source of truth). Add `booleanParam(name: 'APP_DELIVERY_LEGACY', defaultValue: false, description: 'Use the retired per-host stageSpaBlobs path (rollback only, one release)')` to `parameters {}`.

- [ ] **Step 4: pin Review Focus 1 and 2 in the bash test** (cases 2 and 3 above already do; add an assertion that case 3's exit is 3, not 0 or 1) and run every touched test

```bash
bash scripts/ci/app-release-stage.test.sh; echo "EXIT=$?"
bash scripts/ci/publish-app-release.test.sh; echo "EXIT=$?"
bash scripts/ci/verify-app-adoption.test.sh; echo "EXIT=$?"
just gate genesis/orchestrator   # lint-jenkinsfiles covers the root Jenkinsfile
```
Expected: EXIT=0 each; gate green.

- [ ] **Step 5: household rehearsal of the exact CI body** (both doorways up, `just mesh status` green)

```bash
cd /projects/elohim && STORAGE_API_KEY_ADMIN="$(just mesh admin-key 2>/dev/null || cat genesis/local-dev/household-dowell/admin-key)" \
APP_RELEASE_BUNDLES="elohim-host-landing:browser:app/elohim-app/dist/elohim-app/browser:/ ..." \
bash scripts/ci/app-release-stage.sh http://localhost:8888 /tmp/claude-0/app-release.json matthew,jessica,james; echo "EXIT=$?"
```
Expected: `APP-RELEASE-STAGE released=<cid> adopted=matthew,jessica,james`, EXIT=0 (or `adopted=current` on a repeat). If the channel is absent the script says so and names `release-ceremony.ts channel create` — run the prologue's leg that creates it first.

- [ ] **Step 6: commit**

```bash
git add Jenkinsfile scripts/ci/app-release-stage.sh scripts/ci/app-release-stage.test.sh
git commit -m "feat(ci): the App pipeline publishes one release and measures adoption — native delivery replaces per-host stageSpaBlobs (N6 fleet leg)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

### Task D3: the alpha serving pair follows the app-bundle channel at `apply`; the channel exists on the fleet

**Files:**
- Modify: `genesis/orchestrator/data/deployments.json` (matthew, jessica `runtimeConfig.ELOHIM_RELEASE_CHANNELS`: add `runtime:app-bundle:alpha:dev=apply`; james `=canary` if james serves; adam stays `observe`)
- Test: `node --test genesis/orchestrator/runtime-config-render.test.mjs`
- Runbook line: `genesis/a2o/scripts/release-ceremony.ts channel create` against `https://doorway-alpha.elohim.host` (steward act, run once, recorded)

- [ ] **Step 1: check whether alpha's storage already answers the adoption surface** (the storage binary carrying `ArtifactClass::AppBundle` must be deployed — edge #1486 deployed the 2c9dc80a9 batch)

```bash
curl -s --max-time 15 'https://doorway-alpha.elohim.host/db/p2p/adoption?peer=matthew' | head -c 600; echo
curl -s --max-time 15 'https://elohim.host/db/p2p/adoption?peer=jessica' | head -c 600; echo
```
Expected: JSON with `channels[]` naming `runtime:coordinators:elohim:workspace`; if the endpoint 404s, the storage on alpha predates N3/N4 and this task waits for the next edge deploy (say so in the DELTA; do not fake the receipt).

- [ ] **Step 2: write the failing render test** — in `runtime-config-render.test.mjs` add: matthew and jessica render `ELOHIM_RELEASE_CHANNELS` containing `runtime:app-bundle:alpha:dev=apply`, adam does not. Run → FAIL.

- [ ] **Step 3: edit deployments.json** (matthew + jessica entries, keep the workspace channel first; update `$appBundleChannelComment` to say WHY the flip is now allowed: household 6/6 twice 2026-09-26, storage binary deployed by edge #1486). Run the test → PASS.

- [ ] **Step 4: create the channel on the fleet once (steward act; record the output path)**

```bash
cd /projects/elohim/genesis/a2o && STORAGE_API_KEY_ADMIN=<alpha admin key> pnpm exec tsx scripts/release-ceremony.ts channel create runtime:app-bundle:alpha:dev --transport doorway --doorway https://doorway-alpha.elohim.host 2>&1 | tee ../a2o/reports/recovery/serving-edge-20260926/app-bundle-channel-create.log
```
Expected: `channel created` (or `already exists`). Refused → stop and report; the channel is the one precondition `publish-app-release.sh` cannot create.

- [ ] **Step 5: commit**

```bash
git add genesis/orchestrator/data/deployments.json genesis/orchestrator/runtime-config-render.test.mjs
git commit -m "deploy(alpha): the serving pair applies the app-bundle channel — the household proof (6/6 twice) and the deployed storage class allow it

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

### Task D4: the fleet receipt — one verify-class run, both alpha peers adopt, no per-host PUT

**Files:**
- Modify: `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md` (DELTA), `doorway/doorway-service/.epr-meta/doorway-failover.habit.md` (DELTA: fleet same-head reading), `genesis/manifests/habits.yaml` via projection
- Evidence: app build console (`APP-RELEASE-STAGE released=… adopted=matthew,jessica`), `verify-app-adoption.sh` lines, both doorways' `/apps/elohim-host-landing/version.json` commit equal, `GET /api/v1/federation/coherence` digests

- [ ] **Step 1: push Lane D as ONE batch (D1–D3), no tag** — the orchestrator selects edge (deployments.json changed → runtime-config reload) then app.

```bash
git push origin dev 2>&1 | tail -20
```
Expected: pre-push gates green; `T2 receipt` leg satisfied by the household sprint reports of 2026-09-26.

- [ ] **Step 2: watch the app build's delivery stage** (ci-observer, then ci-investigator if the line is missing)

Evidence to quote: `APP-RELEASE-PUBLISHED channel=runtime:app-bundle:alpha:dev release=<cid>`, `APP-ADOPTED matthew …`, `APP-ADOPTED jessica …`, stage result. Grep the console for `stage-spa-blob.sh` — expected ZERO occurrences.

- [ ] **Step 3: read the fleet**

```bash
for h in doorway-alpha.elohim.host elohim.host; do curl -s https://$h/apps/elohim-host-landing/version.json | grep commit; curl -s https://$h/ | grep -o 'src="main-[^"]*"'; done
```
Expected: same `commit` and same `main-*.js` on both. This is the doorway-failover same-declared-head clause read on the fleet.

- [ ] **Step 4: DELTA lines, re-project, commit**

```
DELTA 2026-09-2x: N6 fleet leg — app #<n> APP-RELEASE-STAGE released=<cid> adopted=matthew,jessica; zero per-host PUT; both doorways serve commit <sha> main-<hash>.js. federation-deploy check: measured on the fleet by election. <status decision with evidence>
```
```bash
python3 .claude/scripts/habits-project.py && git add -A elohim/elohim-storage/.epr-meta doorway/doorway-service/.epr-meta genesis/manifests/habits.yaml && git commit -m "habit(dataplane-convergence, doorway-failover): DELTA — the app release reached both alpha peers by election (N6 fleet leg)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

### Task D5: retire the per-host path (A4 + N6 retirements) — only after D4's receipt

**Files:**
- Modify: `Jenkinsfile` (root): delete `stageSpaBlobs` (L223–~320), `authorHeadOnce` (L322–~440), `APP_DELIVERY_LEGACY`; `app/elohim-app/build-manifest.json`: drop `dependsOn: elohim-edge`; `app/elohim-app/scripts/hc-mesh.sh` prologue leg 4b (`stage-spa-blob.sh` per host); delete `scripts/ci/stage-spa-blob.sh` + `.test.sh`
- Test: `just gate genesis/orchestrator`, `node --test genesis/orchestrator/pipeline-budget.test.mjs`, `just mesh prologue` (must stage nothing per host), `bash scripts/ci/tests/*.sh`

- [ ] **Step 1: grep every reader of the deleted symbols**: `grep -rn "stageSpaBlobs\|authorHeadOnce\|stage-spa-blob\|DECLARE_ONLY" --include=Jenkinsfile --include=*.sh --include=*.mjs --include=*.ts --include=*.md . | grep -v node_modules | grep -v "^./genesis/docs"` — each hit is either deleted or rewritten to the release path.
- [ ] **Step 2: remove, then run the gates and `just mesh prologue`**; expected: prologue log has no `stage-spa-blob`, gates green, `pipeline-budget.test.mjs` PASS (app no longer in edge's chain).
- [ ] **Step 3: commit** `chore(ci): retire per-host app staging — stageSpaBlobs, authorHeadOnce, prologue leg 4b and the app→edge dependsOn (native-delivery A4/N6)`.

---

## Lane C — the chain stops waiting on itself (serves push-delivers-within-budget)

### Task C1: a failed leaf aborts only its dependents (backlog row 19)

**Files:**
- Modify: `genesis/orchestrator/graph-walker.mjs` (export `dependentsClosure(pipelines, failedNames)`; CLI `--dependents a,b`), `genesis/orchestrator/Jenkinsfile` fail-fast block (L2624–2636)
- Create: `genesis/orchestrator/dependents-closure.test.mjs`
- Modify: `genesis/data/timeline/backlog/arch-workspace-discipline-backlog.md` row 19 (status)

**Interfaces:**
- Produces: `dependentsClosure(pipelineMeta: {name: {dependsOn: string[]}}, failed: string[]) → string[]` (transitive consumers of any failed pipeline, sorted, excluding the failed ones themselves).

- [ ] **Step 1: failing test** (pin Review Focus 3)

```js
import { test } from 'node:test'; import assert from 'node:assert/strict';
import { dependentsClosure } from './graph-walker.mjs';
const meta = { 'elohim-eprfs': {dependsOn: []}, 'elohim-holochain': {dependsOn: []}, 'elohim-edge': {dependsOn: ['elohim-holochain']}, 'elohim': {dependsOn: ['elohim-edge']}, 'elohim-genesis': {dependsOn: ['elohim']} };
test('a leaf with no consumers dooms nothing', () => assert.deepEqual(dependentsClosure(meta, ['elohim-eprfs']), []));
test('a producer dooms its transitive consumers only', () => assert.deepEqual(dependentsClosure(meta, ['elohim-holochain']), ['elohim', 'elohim-edge', 'elohim-genesis']));
```
Run: `node --test genesis/orchestrator/dependents-closure.test.mjs` → FAIL (not exported).

- [ ] **Step 2: implement in graph-walker.mjs** (pure; reuse the `dependsOn` map the walker already builds) and a CLI branch `node graph-walker.mjs --dependents <csv>` printing one name per line from the manifests it already loads. Run test → PASS.

- [ ] **Step 3: Groovy — replace the level abort with a doomed set**

```groovy
// Fail forward (backlog row 19): a failed pipeline dooms ONLY its transitive
// consumers; unrelated levels keep running. The run still ends FAILURE and
// __global__ is held by the post checkpoint exactly as before.
def levelFailed = level.findAll { !results[it]?.success }
if (levelFailed) {
    def doomed = sh(returnStdout: true, script: "node '${env.WORKSPACE}/genesis/orchestrator/graph-walker.mjs' --dependents '${levelFailed.join(',')}'").trim().split('\n').findAll { it }
    doomed.each { name -> results[name] = [success: false, result: 'SKIPPED-BY-UPSTREAM-FAILURE', upstream: levelFailed.join(',')]; recordPipelineResult(name, results[name], pipelineBaselines) }
    failedPipelines.addAll(levelFailed)   // a list declared before the level loop
    echo "❌ ${levelFailed.join(', ')} failed — dooming ${doomed ?: 'nothing'}; unrelated pipelines continue"
}
```
Later levels skip names present in `results` with `SKIPPED-BY-UPSTREAM-FAILURE`; after the loop, `if (failedPipelines) error "Build(s) failed: ${failedPipelines.join(', ')}"` keeps the FAILURE verdict and the existing checkpoint/actual-graph writes. Keep `buildActualGraph` fed with the skipped results so the delivery series reads them as not-delivered.

- [ ] **Step 4: gates** `just gate genesis/orchestrator` (lint-jenkinsfiles + node tests) → green. Update row 19's status cell to `landed <sha> — first fleet reading owed`.
- [ ] **Step 5: commit** `fix(orchestrator): a failed leaf dooms only its dependents — unrelated pipelines keep the wave (backlog row 19)`.

### Task C2: edge starts beside the DNA pipeline; the happ candidate and the DNA verdict are barriers inside edge

**Files:**
- Modify: `elohim/holochain/dna/Jenkinsfile` `Build DNA` stage (after L699 stash): push `elohim-happ:<version>` immediately (candidate; the floating `dev-latest` still moves only at `Push to Harbor` L1016 after sweettest)
- Modify: `genesis/orchestrator/Jenkinsfile`: `needsDetachedDependencyBarrier` (L889) — for `elohim-holochain` → `elohim-edge`, start DNA with `waitForStart` and pass `DNA_RUN_ID` + `HAPP_CANDIDATE_TAG` to edge in the SAME level instead of awaiting the DNA job; `build-graph.groovy` level assignment for that pair
- Modify: `elohim/holochain/Jenkinsfile`: params `DNA_RUN_ID`, `HAPP_CANDIDATE_TAG`; `fetchHappFromHarbor` (L106) prefers the candidate tag (poll with a bounded deadline via a new `scripts/ci/await-happ-candidate.sh`); `Deploy Edge Node - Alpha` opens with `awaitDnaVerdict()` (`waitForBuild(runId: params.DNA_RUN_ID, propagate: false)`; refuse deploy unless SUCCESS)
- Create: `scripts/ci/await-happ-candidate.sh` + `.test.sh`
- Test: `node --test genesis/orchestrator/pipeline-budget.test.mjs` (chain sum shrinks: edge no longer serial after DNA), `bash scripts/ci/await-happ-candidate.test.sh`, `just gate genesis/orchestrator`

- [ ] **Step 1: failing test for the awaiter** — stub `oras manifest fetch` on PATH: (a) tag present → exit 0 within one poll; (b) tag absent for the whole deadline → exit 3 printing `HAPP-CANDIDATE-ABSENT tag=<t> after <s>s`; (c) deadline honoured (a 2 s deadline ends in <5 s).
- [ ] **Step 2: write `await-happ-candidate.sh <tag> <deadline-secs>`** (poll every 15 s; `oras manifest fetch harbor.ethosengine.com/ethosengine/elohim-happ:<tag> >/dev/null`). Test → PASS.
- [ ] **Step 3: DNA pipeline** — after the stash in `Build DNA`, a `withCredentials(harbor-robot-registry)` block calling `bash scripts/ci/push-happ-candidate.sh <version>` (the versioned `oras push` copied from L1051–1054 with the roll-key annotation; NOT the floating tag). Add the script + a 3-line test that the floating tag is never named in it.
- [ ] **Step 4: orchestrator** — in the level loop, when `elohim-holochain` and `elohim-edge` are both selected: start DNA with `build(job:…, waitForStart:true, propagate:false)`, capture `externalizableId`, and add `stringParam(name:'DNA_RUN_ID', value:id)` + `stringParam(name:'HAPP_CANDIDATE_TAG', value: dnaVersionFor(commit))` to edge's params; edge moves to the same level as DNA (`build-graph.groovy` `topoSortAndLevel` treats this edge as satisfied-by-barrier — a `barrier: true` attribute on that dependsOn entry in `elohim/holochain/build-manifest.json`). The DNA job is still awaited before `elohim` (app) exactly as today.
- [ ] **Step 5: edge** — `awaitDnaVerdict()` at the top of `Deploy Edge Node - Alpha` (pin Review Focus 4):

```groovy
def awaitDnaVerdict() {
    if (!params.DNA_RUN_ID) { return true }   // standalone/manual edge run: today's behaviour
    def dna = waitForBuild(runId: params.DNA_RUN_ID, propagate: false, propagateAbort: false)
    if (dna.result != 'SUCCESS') { unstable("deploy refused: DNA run ${params.DNA_RUN_ID} ended ${dna.result} — a fleet never rolls on an unattested DNA"); return false }
    return true
}
```
`Build hApp Installer` calls `await-happ-candidate.sh ${params.HAPP_CANDIDATE_TAG} 2400` when the param is set, then `fetchHappFromHarbor(dest, params.HAPP_CANDIDATE_TAG)`; exit 3 → `unstable` and the stage falls back to `dev-latest` with a loud line (never silent).
- [ ] **Step 6: budgets** — `pipeline-budget.test.mjs` must PASS with the new chain (`max(DNA, edge) → app → genesis`); update the orchestrator `options { timeout }` and the comment pointer in the same commit. Gates green.
- [ ] **Step 7: commit** `ci(orchestrator,edge,dna): edge builds beside the DNA pipeline — the happ candidate and the DNA verdict are barriers inside edge, not a job-level wait (−49 min)`.

### Task C3: Dataplane Validation runs beside app and genesis as a validate-only sibling

**Files:**
- Modify: `elohim/holochain/Jenkinsfile` `Dataplane Validation` when-block (L2982–3001): add `expression { isValidateOnly() || params.RUN_CLASS != 'deploy' }` so a deploy-class run SKIPS it; the Deploy stage's `post { success }` records `DATAPLANE_VALIDATION=owed-by-sibling`
- Modify: `genesis/orchestrator/Jenkinsfile`: after a successful edge deploy in the level loop, `build(job:'elohim-edge/<branch>', wait:false, propagate:false, parameters:[VALIDATE_ONLY=true, RUN_CLASS=verify, FORCE_BUILD=false])` — fire-and-forget; app/genesis proceed
- Modify: `genesis/orchestrator/validate-only-pipeline.test.mjs` (a deploy-class edge run declares no validation; the sibling dispatch carries VALIDATE_ONLY)
- Test: `node --test genesis/orchestrator/validate-only-pipeline.test.mjs`, `just gate genesis/orchestrator`, `bash scripts/ci/run-dataplane-validation.test.sh`

- [ ] **Step 1: failing test** in `validate-only-pipeline.test.mjs`: the edge Jenkinsfile's Dataplane Validation `when` names `RUN_CLASS != 'deploy'`; the orchestrator source contains one `VALIDATE_ONLY` sibling dispatch after edge success. Run → FAIL.
- [ ] **Step 2: implement both edits.** The sibling run is strict (existing L387–391 rule: validate-only reds are FAILURE) and archives the same reports; the deploy run's summary line says `Dataplane Validation: owed by sibling run` so nobody reads its absence as green.
- [ ] **Step 3: tests + gates green; commit** `ci(edge,orchestrator): Dataplane Validation runs as a validate-only sibling beside app and genesis — the measurement leaves the delivery path (−45 min)`.

### Task C4: the roll gate releases a peer that is back at its pre-roll baseline

**Files:**
- Modify: `scripts/ci/peer-roll-gate.sh` (new settle branch), `elohim/holochain/Jenkinsfile` `deployHumansInParallel` (~L837–910: read `/p2p/status` BEFORE restarting each peer; write `PRE_DIVERGENT_<peer>=<n>` and `PRE_HEALED_<peer>=<n>` into `stateFile`)
- Create: `scripts/ci/peer-roll-gate.test.sh` (stub storage via a python `http.server` serving a scripted sequence of `/p2p/status` bodies; `ROLL_PROM_URL` unset → degraded mode)
- Test: `bash scripts/ci/peer-roll-gate.test.sh`, `bash scripts/ci/sequenced-roll.test.sh`

**Design ruling (controller, 2026-09-26):** a peer is *settled-at-baseline* when, on ≥2 sweeps strictly newer than the post-restart baseline, `divergentAnchor ≤ PRE_DIVERGENT` and `healedTotal ≥ base_healed`. It is printed as `settled-at-baseline(divergentAnchor pre=<p> now=<n>, sweeps +<k>)`, never as `converged`. A standing divergence the peer carried INTO the roll is not the roll's debt; the sibling validate-only run (C3) still measures it.

- [ ] **Step 1: failing test** — sequences: (a) converged:true on a newer sweep → exit 0 `converged` (today's behaviour kept); (b) divergentAnchor 41 → 41 → 41 across three sweeps with PRE_DIVERGENT=41 → exit 0 `settled-at-baseline`; (c) same but PRE_DIVERGENT=30 → keeps waiting, exit 3 at deadline; (d) no PRE_DIVERGENT in the state file → today's behaviour (no new branch). Run → FAIL.
- [ ] **Step 2: implement** in the poll loop next to the `no-movement(` branch (L349); read `PRE_DIVERGENT_${PEER}` from `$STATE_FILE`. Test → PASS.
- [ ] **Step 3: edge Jenkinsfile** — before `kubectl rollout restart statefulset/${resourcePrefix}` (L1818 path) inside the sequenced roll, one `curl --max-time 20 <storage>/p2p/status` parsed by a 10-line `scripts/ci/pre-roll-reading.sh <peer> <storage-url> <state-file>` (appends the two lines; absent/unreachable → writes nothing and says so). Add its 2-case test.
- [ ] **Step 4: gates + commit** `ci(edge): the roll gate releases a peer that is back at its pre-roll baseline — settled-at-baseline is named, never converged (−30..45 min)`.

### Task C5: the Lane C receipt — one push, one orchestrator run, the series read

- [ ] **Step 1: push Lane C as ONE batch (C1–C4) with no tag** and read the orchestrator run: levels (DNA ∥ edge), edge's `awaitDnaVerdict` line, the sibling validate-only run id, per-peer `settled-at-baseline(` lines, app/genesis start times relative to edge deploy end.
- [ ] **Step 2:** `node genesis/orchestrator/delivery-series.mjs --window 10` → quote p90 and exit; `--stages` for the per-stage table.
- [ ] **Step 3: DELTA** in `push-delivers-within-budget.habit.md` with the run number, wall clock, and which of the four moves fired; re-project; commit.

---

## Lane F — the apex name is ready to outlive its doorway (serves doorway-failover; campaign 2.2, 2.3 prep)

### Task F1: the sibling is held to the primary's own reading, not to a corpus it never had

**Files:**
- Modify: `genesis/a2o/steps/dataplane/apex-transition.steps.ts` — `Given 'a new visitor reaches the declared landing page through one owned public name'` (L1300–1338) also takes a `visitInBrowser` of the primary and stores `state.primaryBrowser`; `Then 'that visitor completes browser bootstrap with the same declared build stamp'` (L1417–1433) asserts symmetry
- Create: backlog row `genesis/data/timeline/backlog/household-landing-links-unseeded-epics.md` (D8, campaign 1.x; the ten `/epr-head/<slug>` 404s, same on A/B/C)
- Test: `cd genesis/a2o && pnpm exec vitest run steps/dataplane/__tests__/apex-transition-symmetry.spec.ts`; `just test mesh features/dataplane/doorway-apex-transition.feature`

- [ ] **Step 1: failing unit test** for a pure helper `siblingWorseThanPrimary(primary: HttpError[], sibling: HttpError[]): HttpError[]` (compare by URL *path* + status, origins differ): (a) identical lists → `[]`; (b) sibling has `/epr-head/x` 404 the primary also has → `[]`; (c) sibling 404s `/main-abc.js` the primary served → `[{status:404, url:'/main-abc.js'}]` (pin Review Focus 5). Run → FAIL.
- [ ] **Step 2: implement the helper** (export from the steps file or a sibling `apex-transition.compare.ts`), then change the sibling step:

```ts
const worse = siblingWorseThanPrimary(state.primaryBrowser?.httpErrors ?? [], browser.httpErrors);
assert.deepEqual(worse, [], `sibling browser HTTP errors the primary did not have (${worse.map(e => `${e.status} ${e.url}`).join(', ')})`);
```
and in the Given step, after the raw visit: `state.primaryBrowser = await visitInBrowser(\`${visit.origin}/\`)` (same declared head, before the shed). Unit test → PASS.
- [ ] **Step 3: household run** `just test mesh features/dataplane/doorway-apex-transition.feature` (scoped, verify-class) → expected: "The apex name survives its doorway's shed" PASSES; the other three scenarios unchanged. Quote the sprint-report path.
- [ ] **Step 4: backlog row** for the unseeded epic links (one line each: slugs, evidence `apex-transition-run18.log`, fix shape = add them to the prologue's seed set or drop the links from the household landing content).
- [ ] **Step 5: DELTA** in `doorway-failover.habit.md`: the scenario passes; the corpus gap is a named backlog row; NO status flip (owed checks remain). Re-project.
- [ ] **Step 6: commit** `test(a2o): the sibling is held to the primary's own reading — apex-transition symmetry; landing's unseeded epic links captured as a backlog row`.

### Task F2: the owed checks run — channel isolation and runtime endpoint

**Files:**
- Evidence: `just test mesh features/dataplane/epr-app-channel-isolation.feature`, `just test mesh features/dataplane/doorway-runtime-endpoint.feature` (verify-class, `BERTH_TTL=1800`)
- Modify: `doorway/doorway-service/.epr-meta/doorway-failover.habit.md` (DELTA per feature, with report paths); fixes only if bounded (step-level or a doorway route bug ≤ 1 file) — otherwise the failing step is the frontier, named

- [ ] **Step 1: run both** (mesh green first: `just mesh status`; `just mesh prologue` if the roster is stale). Quote scenario tallies and the first failing step of each.
- [ ] **Step 2: bounded fix or named frontier.** A fix goes through its tree's `just gate` and a re-run; a frontier goes into the DELTA verbatim.
- [ ] **Step 3: DELTA + re-project + commit** `habit(doorway-failover): DELTA — channel isolation <n/m>, runtime endpoint <n/m> on the household`.

### Task F3: campaign story 2.2 — a doorway answers for a name it is a member of (household)

**Files:**
- Create: `genesis/a2o/features/dataplane/name-routing.feature` (`@concern:served-under-standing @act:i @requires:multi-node`), steps in `genesis/a2o/steps/dataplane/name-routing.steps.ts`
- Modify: `doorway/doorway-service/src/routes/coherence.rs` + `services/federation.rs` (~L1120–1160, the sibling probe already exists) — a request whose `Host` is a public name this doorway is a member of but not the current holder is served under this doorway's contract (same declared head, `x-elohim-served-by: <doorwayId>`), or relayed to the holder with the same header; a name this doorway is NOT a member of stays 421
- Test: `cargo test -p doorway-service name_routing` (unit: member/non-member/holder), `just gate doorway-service`, blind-reader loop on the feature, `just test mesh features/dataplane/name-routing.feature`

**P2P design gate (answered):** no entity is created or stored. The membership document is already notarized (campaign 1.x, `doorway-apex-transition.feature`); this is a projection-tier routing rule over it (D8, Track 4). No DHT entry type, no coordinator function, no storage column.

**Scenarios (write these, then the blind-reader loop shapes the wording):**
1. *A member doorway answers the sibling's name with the same head* — visitor asks doorway B for `alpha.elohim.local`'s landing; 200, `app-root`, same declared build stamp as A, header `x-elohim-served-by: apex-elohim-host`.
2. *A non-member doorway refuses the name* — gamma (`MESH_DOORWAY_GAMMA=1`) asked for a name it is not a member of → 421 with the membership document's names in the body.
3. *The holder's own answer carries the header too* — so an outside client can always see who served.

- [ ] **Step 1: write the feature; run the blind-reader loop** (`genesis/a2o/.epr-meta` requires it; cap 3 rounds, record c/i/p).
- [ ] **Step 2: Rust failing unit tests** for the routing decision (`served_under_standing(host, membership, self_id) → Serve | Relay(holder) | Refuse`), then the implementation in the request path where `Host` is already resolved (find `fn resolve_public_name` / the 421 site with `grep -n "421\|MisdirectedRequest" doorway/doorway-service/src`).
- [ ] **Step 3: `just gate doorway-service`** (fmt, clippy `-D warnings`, `cargo test --lib --bins`); rebuild the binary in the pool slot and `just mesh doorway-restart a` / `b` / `c`.
- [ ] **Step 4: household run** → 3/3; DELTA in `doorway-failover.habit.md` (and a `served-under-standing` atom line if that habit exists in `doorway/doorway-service/.epr-meta/`); re-project.
- [ ] **Step 5: commit** `feat(doorway): a doorway answers for a name it is a member of, same head, x-elohim-served-by — campaign story 2.2 on the household`.

### Task F4: campaign 2.3 deliverables — the manifests and the runbook the operator applies

**Files:**
- Modify: `genesis/orchestrator/manifests/infra/alpha-coturn-shem.yaml` and `alpha-coturn-operations.yaml` (beacon `--shared-record elohim.host=<both legs>` posture, per campaign 2.1's landed `--shared-record NAME=OWNER`), the doorway ingress manifests under `genesis/orchestrator/manifests/doorway/` (host-conflict check: no ingress claims the apex host once both doorways stand behind it)
- Create: `genesis/docs/content/elohim-protocol/architecture/2026-09-2x-apex-multi-a-failover-runbook.md` — the DNS multi-A change, the beacon flip order, the withdraw → re-admit proof to read in the beacon log, the rollback (single-A), and the outside-client check (`curl --resolve`)
- Test: `just gate genesis/orchestrator` (manifest lint), `bash scripts/ci/dead-config-lint.sh`; a `python3 -c` YAML load of each touched manifest

- [ ] **Step 1:** read campaign plan lines 205–229 and the 2.1 landing (`04abd05ef`) to copy the exact flag grammar; write the manifests; lint green.
- [ ] **Step 2:** write the runbook (≤120 lines, every command copy-pasteable, each step with its expected log line).
- [ ] **Step 3:** cites: `epr flow cites seal <runbook>`; commit `docs(serving-edge): apex multi-A failover — manifests and runbook for the operator's DNS act (campaign 2.3)`. The apply is the operator's; the plan ends with the story 2.3 proof still owed, named.

---

## Execution order and parallelism

| Wave | Tasks (parallel, disjoint write-sets) | Serial after |
|---|---|---|
| 1 | D1 (rakia + gitlink) · C1 (orchestrator Jenkinsfile + graph-walker) · C4 (peer-roll-gate.sh + edge roll section) · F1 (a2o steps) | — |
| 2 | D2 (root Jenkinsfile + scripts/ci) · C2 (DNA + orchestrator + edge happ/deploy) · F2 (mesh runs) | C1, C4 |
| 3 | D3 (deployments.json) · C3 (edge validation when-block + orchestrator sibling dispatch) · F3 (doorway-service + feature) | C2 |
| 4 | D4 push + receipt (Lane D batch) | D1–D3 |
| 5 | C5 push + receipt (Lane C batch, separate deploy) | C1–C4, D4 |
| 6 | D5 retirements · F4 manifests + runbook | D4 |

Two orchestrator Jenkinsfile writers never run at once (C1 → C2 → C3 serial). The root Jenkinsfile has one writer (D2, then D5). The edge Jenkinsfile: C4 (roll section) then C2 (happ/deploy) then C3 (validation when-block).

## Frontier this plan leaves named

- adam's conductor window (CellDisabled after restart) — the pinned fork carries the mechanism-(1) cure; whether it rolled in edge #1486 is read in D4/C5 and recorded, not assumed.
- Campaign 2.3's proof (a real shed behind one name with an outside client seeing no outage) — operator DNS act.
- Adam as measurement peer — next sprint per the 09-24 plan.
