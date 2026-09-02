---
title: "Dataplane pain-points sprint — measure without deploying, measure what the register names, one-command device peer, honest handshake"
id: dataplane-pain-points-sprint-plan
status: Draft
class: protocol-canonical
domain: D5
habits: [dataplane-convergence]
topic: [sprint, quiesce-gate, federation-deploy, act-ii, device-peer, trust-handshake, measurement, ci]
informed-by:
  - genesis/docs/superpowers/specs/2026-09-01-trust-priced-sync-edge-design.md (station 1 is wave 2 task T4)
  - genesis/docs/superpowers/specs/2026-08-30-workspace-stewarded-device-peer-design.md (the device-peer ladder T3/T5 serve)
  - genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md (rung 5 — leveraged, never touched by this sprint)
cites:
  - genesis/data/timeline/backlog/edge-quiesce-gate-timeout-aborts.md
  - genesis/data/timeline/backlog/dataplane-peer-fallback-and-blob-replication.md
  - genesis/data/timeline/backlog/sovereign-peer-network-read-no-authorities.md
  - genesis/data/timeline/backlog/task-runtime-upgrade-a2o-receipt.md
  - genesis/a2o/features/dataplane/federation-deploy.feature
  - genesis/a2o/LAYERS.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
---

# Dataplane pain-points sprint

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`
> (one fresh worker per task, reviewed between tasks). Steps use `- [ ]` syntax.
> Workers **commit only, path-limited, never push**; the integrator pushes one batch.

**Goal:** stop losing wall-clock to measurement-by-deploy, make the register's top red
actually measurable on the fleet, make the workspace device-peer one command, and land the
honest trust handshake — each with a receipt.

**Architecture:** four independent pain points, each cleared at its own seam: the edge
pipeline's quiesce leg (CI shell + one Groovy helper), the a2o act tag on the federation
feature (a2o), the device-peer launcher (hc-start.sh), and the storage trust handshake
(Rust, wave 2). Nothing here touches the rung-5 adoption controller, `hc-mesh.sh`, or the
mesh — those belong to the concurrent adoption ceremony and its push batch.

**Spec:** the trust-priced sync edge spec (station 1), the workspace device-peer design,
and the three cited backlog atoms. This plan argues from them; it adds no rung.

## Ground truth the sprint stands on (2026-09-02, live)

| fact | evidence |
|---|---|
| Both doorways serve `/` 200 with non-null `blobHash` for `elohim-host-landing` — federation-deploy **scenario 2 passes live** | probe 2026-09-02: elohim.host `sha256-f0f0e637…`, doorway-alpha `sha256-04ae4310…`, same `dhtAnchorHash uhCkkvfsT…` |
| The two doorways serve **different versions** (updatedAt 02:47Z vs 02:53Z on 08-31) — the divergence failure mode, scenario 4 `@wip` | same probe |
| Both doorways read `caughtUp:false converged:false`, divergentAnchor 2131 / 1011 | `/health` same probe |
| The fleet lane has **never measured** federation-deploy: 0 passed / 0 failed / 2 pending in every dataplane report since 08-22 — the feature is `@act:i`, and the fleet lane drops Act I | `sprint-report-dataplane.json` run `jenkins-elohim-edge-dev-1381`; LAYERS.md |
| Edge #1406/#1407/#1408 died ABORTED inside the quiesce leg after DEPLOY completed — the only `timeout` is the pipeline-global 120 min (`elohim/holochain/Jenkinsfile:1865`); the stage's `catchError` cannot intercept a global interrupt | backlog atom; Jenkinsfile `:2683-2687` |
| `elohim/holochain/Jenkinsfile` pipeline{} block is **64108 / 65000 bytes** | `genesis/orchestrator/scripts/check-jenkinsfile-method-size.sh` |
| `QUIESCE_MODE=label` / `churn_state` (claimed landed 2026-08-28 in the ratchet spec M2) **do not exist** in the tree | `grep -rn QUIESCE_MODE` empty |
| Bytes and seed authority are no longer federation-deploy's constraint: `API_KEY_SEED` is in both doorway manifests and applied on every deploy; `stageSpaBlobs` is byte-seed only, `authorHeadOnce` + `DECLARE_ONLY` fan-out carries the head | root `Jenkinsfile:223,300`; `genesis/orchestrator/manifests/doorway/alpha-b.yaml` |
| Coordinator hot-swap dispatch is already warn-only and has no in-flight-roll signal; connect-refused prints `INCOMPLETE` | `scripts/ci/fleet-coordswap-dispatch.sh:97-103` |
| Another session holds the storage crate, the mesh and the storage cargo slot for the pre-push adoption ceremony | `task-runtime-upgrade-a2o-receipt.md` §Local verification handoff; `ps` shows its `cargo test --lib release_adoption` |

## Global constraints

- **No `elohim-storage` Rust edits, no `hc-mesh.sh` edits, no mesh processes in wave 1.** Wave 2
  starts only when the ceremony's cargo run and storage restarts are done (operator says so, or
  `ps` shows no `cargo` under `elohim__elohim-storage` and the ceremony atom carries its transcript).
- Bash bodies live in `scripts/ci/*.sh`; the edge Jenkinsfile gains **no** new stage and **no**
  inline heredoc. Every edit to it is followed by
  `bash genesis/orchestrator/scripts/check-jenkinsfile-method-size.sh` and the pipeline{} byte
  count must go **down**, not up.
- A2o feature edits go through the blind-reader loop (`a2o-story` profile) until READY.
- Commit trailer on every commit:
  `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_0193dgMs2G8uWM7qzvfBQ9Wr`.
- Never `kubectl`. Cluster state is read from manifests and Jenkins; the repo is the surface.
- No new register, ledger, ranking script, or rung. The deliverable of each task is its receipt
  plus a one-line delta in `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`.
  **Workers do not edit the habit atom or `habits.yaml`** — three concurrent appenders on one
  file in a shared worktree is the conflict this rule prevents. Each worker puts its delta
  text in its commit message under a `HABIT-DELTA:` line; the integrator appends the three
  deltas once and re-projects.

---

## Wave 1 — no cargo, three disjoint write-sets, run in parallel

### Task T1: bound the quiesce leg; defer COORDSWAP honestly

**Tier:** Sonnet. **Reviewer:** Opus (CI is high-risk: a wrong edit can hide every deploy).

**Files:**
- Modify: `elohim/holochain/Jenkinsfile:2664-2688` (the `Dataplane Validation` `steps{}` body) and add one top-level `def runDataplaneValidation()` beside `runMeshQuiesceMeasure()` (`:311-330` is the precedent)
- Modify: `scripts/ci/fleet-coordswap-dispatch.sh:93-103`
- Modify: `scripts/ci/fleet-coordswap.sh:438-478` (`run_rolling_apply`)
- Modify: `genesis/data/timeline/backlog/edge-quiesce-gate-timeout-aborts.md` (status line)

**Interfaces:**
- Consumes: `isValidateOnly()` (`Jenkinsfile:1688-1690`, already computed from `params.VALIDATE_ONLY` or `[edge:validate-only]`).
- Produces: `runDataplaneValidation()` — no arguments, reads `env.WORKSPACE` and `isValidateOnly()`.

- [x] **Step 1: extract the stage body.** Replace the two `catchError` blocks inside `container('builder') { … }` at `:2670-2687` with one line: `runDataplaneValidation()`. Keep the `container('builder')` wrapper and the stage's `when`/`post` untouched.

- [x] **Step 2: write the helper** immediately after `runMeshQuiesceMeasure()`:

```groovy
/**
 * Dataplane Validation body. Lives as a top-level def for the same reason
 * runMeshQuiesceMeasure() does: the pipeline{} block sits at the JVM 64KB CPS
 * ceiling. Two behaviours ride here:
 *  - the quiesce leg carries ITS OWN timeout (edge #1406-#1408 rode the
 *    pipeline-global 120-min option to ABORTED after DEPLOY had completed —
 *    a healthy deploy read as ABORTED; the stage's catchError cannot catch a
 *    global interrupt, only a nested one);
 *  - ordinary deploy builds are warn-only (UNSTABLE, last measurement
 *    printed); [edge:validate-only] recording runs stay strict (FAILURE),
 *    because a validate-only run exists to record a verdict and a swallowed
 *    red there is a lie. Same shape as the T2 receipt leg in
 *    .husky/pre-push.bash (T2_RECEIPT=strict).
 */
def runDataplaneValidation() {
    catchError(buildResult: 'SUCCESS', stageResult: 'UNSTABLE') {
        sh "bash '${env.WORKSPACE}/scripts/ci/substrate-seam-smoke.sh' https://doorway-alpha.elohim.host https://elohim.host"
    }
    // 2700 s gate + cucumber + report must fit inside this bound; 55 min leaves
    // the pipeline-global 120 min for the deploy stages that follow.
    def strict = isValidateOnly()
    def body = {
        timeout(time: 55, unit: 'MINUTES') {
            withEnv(["E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host", "QUIESCE_DEADLINE_SECS=2700"]) {
                sh "bash '${env.WORKSPACE}/scripts/ci/run-dataplane-validation.sh'"
            }
        }
    }
    if (strict) {
        catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') { body() }
    } else {
        catchError(buildResult: 'SUCCESS', stageResult: 'UNSTABLE') { body() }
    }
}
```

- [x] **Step 3: size check.** Run `bash genesis/orchestrator/scripts/check-jenkinsfile-method-size.sh`. Expected: the `elohim/holochain/Jenkinsfile` pipeline{} byte count is **below 64108** (the body left the block). Record the number in the commit message.

- [x] **Step 4: Groovy sanity.** Run `git diff elohim/holochain/Jenkinsfile | grep -c '^[-+]' ` and eyeball braces; then `just gate elohim/holochain` (the manifest project owning the Jenkinsfile — if the gate names no step for Groovy, say so in the commit rather than inventing one).

- [x] **Step 5: COORDSWAP defer verdict.** In `scripts/ci/fleet-coordswap.sh` `run_rolling_apply`, when `call_sync` returns `LAST_HTTP_CODE = 000` (connect refused / no route), record the peer as `deferred` instead of `failed-pre-check` / `failed-apply` and continue to the next peer instead of `return 1`; at the end, `return 4` if any peer was deferred and none failed. In `fleet-coordswap-dispatch.sh`, add a branch:

```bash
elif [ "$rc" -eq 4 ]; then
  echo "COORDSWAP: DEFERRED — one or more peers refused the connection (an edge roll is likely in flight); nothing applied on those peers, re-run after the roll."
```

  keeping `exit 0` (warn-only by policy is unchanged).

- [x] **Step 6: test the defer path without a fleet.** `bash scripts/ci/fleet-coordswap.sh --happ /dev/null --peers 'x=http://127.0.0.1:9' --apply --timeout 5 --json; echo EXIT=$?` — expected: the JSON report shows `"status":"deferred"` for peer `x` and `EXIT=4`. Then the dispatch wrapper with the same peer prints the `COORDSWAP: DEFERRED` line and exits 0.

- [x] **Step 7: atom + delta.** Set the backlog atom's `status:` to `"in-tree"` and append one dated line naming the two edits. Put this in the commit message: `HABIT-DELTA: quiesce leg bounded (55 min, warn-only on deploy, strict on validate-only) + COORDSWAP deferred verdict — fleet-unproven until the next edge build shows UNSTABLE-not-ABORTED`.

- [x] **Step 8: commit** (path-limited): the Jenkinsfile, the two scripts, the atom.

**Evidence that closes it (integrator, after push):** the next edge build's log shows the stage-scoped `Timeout has been exceeded` if any, final result UNSTABLE not ABORTED, and `Deploy Edge Node - Staging` runs after it; a `[edge:validate-only]` run propagates FAILURE.

### Task T2: make the register's top red measurable on the fleet

**Tier:** Sonnet (a2o), then blind-reader loop. **Reviewer:** Opus for the act-tag decision only.

**Files:**
- Modify: `genesis/a2o/features/dataplane/federation-deploy.feature` (tags + the "Live state observed" block)
- Modify: `genesis/data/timeline/backlog/dataplane-peer-fallback-and-blob-replication.md` (status + a 2026-09-02 supersession note)
- Modify: `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md` (delta), `genesis/manifests/habits.yaml` (projection)

**Interfaces:**
- Consumes: LAYERS.md act baselines (Act II = `alpha-cluster-6peer dht-anchored-content … deploy-churn …`, drops `owned-substrate`); the gate warning `MULTIPLE ACT TAGS`.
- Produces: a federation-deploy feature the fleet lane executes.

- [ ] **Step 1: decide the act per LAYERS.md, not per habit.** federation-deploy asserts that TWO doorways on the alpha fleet agree — Act II ("adam's household federates; doorway B becomes adam's") by definition. Change the feature-level tag line from `@act:i` to `@act:ii`. Every scenario in the file is a two-doorway fleet claim, so no per-scenario act tags are added (two act tags on one scenario is an authoring error).

- [ ] **Step 2: update the "Live state observed" block** with the 2026-09-02 probe (both doorways `/` → 200; blobHash `sha256-f0f0e637…` on elohim.host vs `sha256-04ae4310…` on alpha-A; same dhtAnchor) and state plainly: scenario 2's two conditions hold live; the remaining red is version divergence (the final scenario). Do not remove the 2026-06-29 baseline — it is the history.

- [ ] **Step 3: run the act gate locally.** `cd genesis/a2o && node ./scripts/gherkin-prepush-lint.mjs` (expect `parsed N feature files`, exit 0) and `pnpm scan:coverage` (expect no `MULTIPLE ACT TAGS` warning for this file).

- [ ] **Step 4: blind-reader loop.** Dispatch a fresh `blind-reader` with only the feature path and profile `a2o-story`; revise; repeat until READY.

- [ ] **Step 5: supersede D5's framing.** In the D5 atom, set `status:` to `"superseded-in-code"` and append a 2026-09-02 note: bytes are seeded once per deploy, `authorHeadOnce` + `DECLARE_ONLY` carry the head, `API_KEY_SEED` is deployed; the pointer-propagation gap it names is closed, the open item is head *election* under restart arc churn (cite `sovereign-peer-network-read-no-authorities.md`).

- [ ] **Step 6: habit delta (commit-message line only).** `HABIT-DELTA: 2026-09-02 live probe — federation-deploy scenario 2 conditions hold on both doorways; the two serve different blobHashes (divergence, scenario 4 `@wip`); feature re-acted to `@act:ii` so the fleet lane measures it for the first time. NO status flip: caughtUp false on both, divergence open.`

- [ ] **Step 7: commit** the feature and the D5 atom.

**Evidence that closes it:** the next Dataplane Validation report shows `federation-deploy` with `passed: 2` (scenarios 1–2) and the divergence scenario counted, not `pending: 2`.

### Task T3: the device peer is one command

**Tier:** Sonnet. **Reviewer:** Opus reads the diff for anything that starts a process.

**Files:**
- Modify: `app/elohim-app/scripts/hc-start.sh` (the join-alpha path: `:172-218` refusal + env block; `:453-480` storage launch)
- Modify: `justfile:202-214` (`dev conductor` recipe help)
- Create: `genesis/a2o/scripts/device-peer-receipt.ts` (precedent: `genesis/a2o/scripts/late-joiner-receipt.ts`)

**Interfaces:**
- Consumes: `ELOHIM_RELEASE_CHANNELS` (read by `elohim-storage` runtime-config, format `channelId` or `channelId=observe|apply|canary`); `GET /admin/adoption` on the storage peer; `GET /p2p/status`.
- Produces: `CONDUCTOR_RELEASE_CHANNELS` env → exported as `ELOHIM_RELEASE_CHANNELS` for the storage process hc-start launches; `just dev conductor alpha` prints its preflight checklist; the receipt script prints one line per ladder station.

- [ ] **Step 1: preflight prints, then refuses.** Before the fork-pair refusal at `hc-start.sh:198-217`, print a checklist (one line each, `✓`/`✗`): fork `holochain`+`hc` pair found (path), `CONDUCTOR_APP_PORT` (4485 default, note the household 4445/4455/4465 range), `CONDUCTOR_ARC_FACTOR`, hApp source (`fetch-deployed-dna.sh` vs `FORCE_LOCAL_HAPP`), `DOORWAY_AUTH` posture (and whether `mongod` resolves), `CONDUCTOR_RELEASE_CHANNELS` (unset = not following). The existing refusal text stays; it now follows the checklist.

- [ ] **Step 2: channel passthrough.** In the storage launch block (`:453-480`), if `CONDUCTOR_RELEASE_CHANNELS` is set, export `ELOHIM_RELEASE_CHANNELS="$CONDUCTOR_RELEASE_CHANNELS"` for the storage process only, and print `following: <value>`; otherwise print `following: (none — set CONDUCTOR_RELEASE_CHANNELS=<channel>=observe to ride a release channel)`.

- [ ] **Step 3: justfile help.** In the `dev conductor` recipe, add the one-line usage `just dev conductor alpha [CONDUCTOR_RELEASE_CHANNELS=<channel>=observe]` to the existing comment/echo. No new recipe.

- [ ] **Step 4: receipt script.** `device-peer-receipt.ts` takes `--storage <url>` (default `http://127.0.0.1:8090`) and `--doorway <url>` (default `https://doorway-alpha.elohim.host`) and prints, one per line with a `PASS`/`FAIL`/`SKIP` and the measured value: station 1 joined (`/p2p/status` lists ≥1 remote peer, elapsed since process start), following (`/admin/adoption` lists the channel or `SKIP not following`), station 2 pulled (`SKIP` — gated on the backlog gaps, printed honestly), station 3 recognised (`SKIP` — `bind_identity` unshipped). Exit 0 if every non-SKIP line is PASS. Write it so it never starts a process.

- [ ] **Step 5: verify without running the stack.** `bash -n app/elohim-app/scripts/hc-start.sh`; `shellcheck app/elohim-app/scripts/hc-start.sh` if available (report, do not chase pre-existing warnings); `cd genesis/a2o && pnpm exec tsx scripts/device-peer-receipt.ts --storage http://127.0.0.1:9 --doorway http://127.0.0.1:9` → every line `FAIL`/`SKIP` with a connection-refused reason, exit 1.

- [ ] **Step 6: commit.** `HABIT-DELTA: device-peer preflight checklist + release-channel passthrough + receipt script (unmeasured against alpha until the operator runs `just dev conductor alpha`)`. Commit the three files.

**Evidence that closes it:** the operator runs `just dev conductor alpha CONDUCTOR_RELEASE_CHANNELS=<the ceremony's channel>=observe`, then `pnpm exec tsx scripts/device-peer-receipt.ts` prints station 1 PASS with its elapsed seconds and `following: <channel>`.

---

## Wave 2 — needs the storage crate and cargo slot (after the adoption ceremony)

### Task T4: station 1 — the honest handshake

**Tier:** `rust-architect` (Opus). **Reviewer:** `code-reviewer` + one fresh Explore agent confirming zero remaining `reach_ceiling: "public"` literals.

**Spec:** `2026-09-01-trust-priced-sync-edge-design.md` §5 station 1 and §7 gaps 1–4.

**Files:** `elohim/elohim-storage/src/p2p/mod.rs` (handshake sender `:5416-5423`, receiver arms `:6675-6684`, `:6706-6715`), `src/trust_service.rs:47-53`, `src/trust_verification.rs:93-230`, `src/p2p_iroh/auth_backends.rs:190-191`, the imagodei/qahal coordinator zome (add `get_relationship_by_action` beside `get_membership_by_action` at `qahal_coordinator.rs:596` — coordinator-only, DNA-hash-neutral), `src/p2p/trust_cache.rs` tests.

- [ ] Write the failing test first: a `VerifiedTrustContext` built from a handshake with empty vectors has `reach_ceiling == "unverified"` (today: `"public"`), and one built from a verified household membership has `"trusted"`.
- [ ] Sender fills `membership_cids` / `relationship_cids` / `stewardship_cids` from the node's own rows and presents the steward agent key, not the libp2p peer id.
- [ ] Receiver (both libp2p arms and `TrustService::handle`) calls `verify_trust_context` through `hc_registry.imagodei_client()`; `calculate_reach_ceiling` gains the commitment input (`rea_commitments` active `replicates-*` between the two agents → `trusted`).
- [ ] One derivation function, two callers, one contract test asserting equality.
- [ ] `just gate elohim-storage` green (`EXIT=` echoed on its own line, never judged from piped output). Then `just test mesh '@concern:trust-priced-sync'` on the household mesh: scenario 1 green is the receipt.
- [ ] Habit delta + commit.

### Task T5: `--sync-coordinators-once` for the T3 workspace peer

**Tier:** `rust-architect`. **Files:** `elohim/elohim-storage/src/main.rs` (flag), `src/happ_manager.rs` (`sync_coordinators` reuse against `HOLOCHAIN_ADMIN_URL`), `app/elohim-app/scripts/hc-start.sh` (call it after the conductor is up on the join-alpha path). Backlog: `sovereign-peer-network-read-no-authorities.md:70-87`. Receipt: the atom's own `delegation-live-check.ts` no longer fails with `Fn grant_head_delegation doesn't exist` after a coordinator bump, without re-keying W.

---

## Operator decisions this sprint surfaces (not tasks)

1. **Flip `ELOHIM_OBEY_CARRIED_ELECTION` on alpha** (both doorway/storage manifests under `genesis/orchestrator/manifests/`) and record one `[edge:validate-only]` measure. It is the dormant cure for the divergence the probe shows; it moves heads, so it is yours. Evidence it worked: `election_obeyed_total{path="carried"} > 0` and both doorways return the same `blobHash` for `elohim-host-landing`.
2. **The storage-arc reset on conductor restart** (`storageArc: null` on every alpha agent-info; arcs never re-promote before the next roll) is the deepest constraint under both federation-deploy's divergence and the sovereign-peer reads. It is a conductor-fork / kitsune2 concern, captured in `sovereign-peer-network-read-no-authorities.md`, and it is exactly what the rung-5 no-big-bang-roll work reduces the frequency of. Not absorbed here.
3. **The ratchet spec's M2 delta claims `QUIESCE_MODE=label` landed; it did not.** One flow-note correction on that spec, then the claim is honest.

## Self-review

- Spec coverage: T1 ↔ quiesce atom (both halves); T2 ↔ the register's top red + D5 staleness; T3 ↔ device-peer candidates 1 (partial: checklist, not auto-fetch — the fork-pair source is a decision) and 3; T5 ↔ candidate 2; T4 ↔ trust spec station 1. Candidate 1's auto-fetch is deliberately not scheduled: the fork pair's fetch source (Harbor image vs cargo build) is an operator decision.
- Placeholders: none; every step names its command and expected output.
- Disjointness: T1 writes `elohim/holochain/Jenkinsfile` + `scripts/ci/fleet-coordswap*`; T2 writes `features/dataplane/federation-deploy.feature` + the D5 atom; T3 writes `hc-start.sh` + `justfile` + a new a2o script. The habit atom is written by the integrator alone, from the three `HABIT-DELTA:` commit lines.
