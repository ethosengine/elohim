**CONFIRMED:** reviewed `origin/dev` at `0088755ec` against regenesis-1’s records. No files changed; no tests, builds, deployments, or live probes ran. “CLOSED” below means the source contains the cure, not that I independently reproduced its runtime result.

Evidence shorthand: **S** = [STATE.md](/projects/elohim/.worktrees/hc-k6v2/genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/STATE.md); **R** = [regression-classes.md](/projects/elohim/.worktrees/hc-k6v2/genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/regression-classes.md), numbered sections; **H** = [cycle-time handoff](/projects/elohim/.worktrees/hc-k6v2/genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/briefs/cycle-time-harvest-handoff.md). Dev paths below abbreviate `elohim/elohim-storage/src/` as **ST**, `genesis/orchestrator/` as **O**, `genesis/seeder/src/` as **SE**, and `doorway/doorway-service/src/` as **DW**.

Historical observations are **CONFIRMED by the cited records**. Pre-clear actions, sizes, and expected savings are **PLAUSIBLE recommendations**: S = small, M = bounded implementation, L = substantial work.

| ID | What happened; historical evidence | Recorded cost | Still open on dev? | Pre-clear action; size |
|---|---|---|---|---|
| F01 | Failed installs repeatedly minted keys; churned lair stores required replacement. WAL causation remained a hypothesis. S:254–259; R§5. | Two operator wipes; approximately 30 and 82 crash cycles | **CONFIRMED OPEN mechanism:** `ST/happ_manager.rs:934` still generates a key per fresh attempt; `ST/main.rs:1206` propagates install failure. | Reconcile indeterminate installs in-process; retain install intent/key; reproduce against forensic copies. **L** |
| F02 | Cold-space creation deadlocked under constrained workers. R§4. | Unrecorded | **CONFIRMED CLOSED:** conductor pin remains `993cf597a`; its `conductor/space.rs:822` contains the two-worker regression. | Make that regression a mandatory fork-port check; add quota-constrained install rehearsal. **S/M** |
| F03 | Broken old StatefulSet pods prevented replacement; operator deleted them. S:246,251; R§6. | 600-second timeout; operator intervention | **CONFIRMED OPEN:** `elohim/holochain/Jenkinsfile:2004–2046` applies and waits; `scripts/ci/rollout-watch.sh:26` excludes old revisions. | Add explicitly bounded replacement of an unready old-revision pod after successful apply. **M** |
| F04 | Reset erased the app release channel; another app build needed a steward tag. S:262,274; R§1. | Operator channel-create act | **CONFIRMED OPEN:** `scripts/ci/app-release-stage.sh:212` still requires `APP_RELEASE_CHANNEL_CREATE=1`. | Declare required channels and reconcile through the existing ceremony. **M** |
| F05 | Mongo accounts retained retired conductor keys; the first repair targeted incorrectly normalized identifiers. | Two repair rounds | **CONFIRMED OPEN residual:** backlog `doorway-orphaned-binding-after-regenesis.md:99–107` still owes live detection and targeted procedure; repair itself closed at #1757, lines 85–98. | Add orphan-binding probe and inventory-driven, identifier-aware repair plan; preserve unrelated users. **M** |
| F06 | Floating hApp tags could change during a partial roll—recorded risk, not observed failure. R§8. | Unrecorded | **CONFIRMED OPEN:** `O/manifests/humans/_edgenode-conductor.template.yaml:247` pulls `${HAPP_VERSION}`; the digest annotation does not pin that pull. | Resolve once per rollout and fetch every peer’s hApp by that digest. **M** |
| F07 | Licence-only edits made WASM permanently “stale,” even after rebuilding. S:280; R, smaller frictions. | Manual override | **CONFIRMED OPEN:** `app/elohim-app/scripts/hc-mesh.sh:2493,2563,2594` retains directory-mtime comparison. | Use a build-input fingerprint; cover metadata-only edits and dependency changes. **M** |
| F08 | Serving receipts cover the whole a2o tree. H:44–60. | 20–40-minute rerun | **CONFIRMED OPEN:** `genesis/a2o/scripts/lib/sut.ts:83` still keys `genesis/a2o`. | Compute feature/step transitive imports; retain all SUT dependencies and conservative fallback. **M** |
| F09 | Old rust-style method baseline prevented any successful CI dispatch. S:107–116. | Overnight hold; two rejected edit attempts | **CONFIRMED CLOSED:** `O/rust-style-ci.sh:39–56` advances to the method bootstrap. | Exercise the existing baseline fixture before changing the checker/toolchain. **S** |
| F10 | Rust debt paydown was discovered only at push. S:278. | 322 seconds | **PLAUSIBLE residual:** enforcement remains at `.husky/pre-push.bash:855`; current guidance already names early checking, so recurrence depends on execution. | Put rust-style before expensive gates in the crossing’s executable preflight. **S** |
| F11 | Brit failed after green hooks; worktree operations also failed. H:81–98; S:208. | 30–60-minute hook runs repeated | **PLAUSIBLE:** dev now pins Brit `4575ab2ee`; no push experiment ran here. | Qualify pinned Brit against a disposable remote/worktree; document the proven fallback. **M** |
| F12 | Notes refs failed the source-tree style gate, keeping receipts local. H:94–98; RESULT follow-ups. | Unrecorded | **CONFIRMED CLOSED:** `bd863680c`; `.husky/pre-push.bash:832` skips notes. | Add notes transport to landing rehearsal. **S** |
| F13 | Several gate projects lacked reuse enrollment. H:24–42. | 39 cold gate runs reported | **CONFIRMED CLOSED enrollment:** `genesis/agentic/pool-policy.json:89–94`; several JS keys still conservatively cover the whole tree. | Check reuse eligibility before the shift; expect compiler changes to invalidate closures. **S** |
| F14 | Storage captured absent conductor clients at boot and never adopted releases. S:274–300. | Operator restart and admission ruling | **CONFIRMED CLOSED:** `9faf54076`; lazy acquisition remains at `ST/main.rs:3684–3693`. | Add storage-before-conductor startup to preflight acceptance. **S** |
| F15 | Binding mint abandoned retries before conductors became available. S:263; R§7. | Retry stopped around 18 minutes; conductors arrived 45–70 minutes later | **CONFIRMED CLOSED:** `f99fb0f32`; `ST/p2p/binding_mint.rs:694–734` retries indefinitely with bounded backoff/wakeup. | Run the existing delayed-readiness regression during port qualification. **S** |
| F16 | Doorway readiness stayed green through conductor restart, blocking serving receipts. S:282–300. | Two failed receipts; morning ruling | **CONFIRMED CLOSED source gap:** `6bf78dd6b`; `DW/routes/health.rs:895,925` consults primary storage serving truth. Backlog text is stale. | Re-run station 2 on the candidate conductor; reconcile the backlog with evidence. **S** |
| F17 | Seeder treated transient deferred commitment writes as fatal. S:221–222. | Prologue rerun | **CONFIRMED CLOSED:** `35621055c`; `SE/doorway-client.ts:349,1357` implements and invokes bounded retries. | Include deferred-write tests in dependency qualification. **S** |
| F18 | Empty `blobCid` rejected a 100-record batch. R§2. | Unrecorded | **CONFIRMED OPEN type gap; instance fixed:** `SE/seed-sqlite.ts:473,506` still uses optional strings. | Introduce parsed address types at authoring boundaries; retain absence/invalid-value tests. **M** |
| F19 | Adam’s copied manifests missed nine environment variables. R§3. | Unrecorded | **CONFIRMED CLOSED instance:** `2381eb727`; `O/human-manifest-parity.test.mjs:21–72` compares both workload pairs. | Extend parity to relevant values/resources before changing runtime settings. **S** |
| F20 | Late-bind work nearly exhausted `main.rs`’s stated size allowance. S:278; R, smaller frictions. | Unrecorded | **CONFIRMED OPEN modularization:** current `ST/main.rs` has 7,030 physical lines. This is not a gate-failure claim. | Extract boot consumers before adding upgrade branches. **M** |
| F21 | Renamed upload files silently skipped downstream validation stages. R§9–10. | Two builds without those checks | **CONFIRMED CLOSED:** `genesis/Jenkinsfile:1321` consumes producer-owned exports; `genesis/a2o/scripts/substrate-verify.ts:788–807` declares and validates them. | Preserve the contract self-test during npm changes. **S** |
| F22 | Twelve worktrees were created from divergent local dev, then all rebases conflicted. `worktrees.md`, “Follow-up.” | Twelve aborted rebases | **PLAUSIBLE recurrence:** procedural, not a current source defect. | Resolve one integration base and record gitlinks before creating worktrees. **S** |
| F23 | HEAD reported missing apps while GET succeeded. S:301; R§15. | Unrecorded | **CONFIRMED CLOSED:** `DW/server/http.rs:121–161`, centralized `MethodClass`. | Retain listener-level HEAD/GET parity in readiness acceptance. **S** |
| F24 | Random base64 became an invalid test regex. R, smaller frictions. | Two recorded gate failures | **CONFIRMED CLOSED:** `genesis/agentic/compute/worker-start.test.mjs:588–592` uses literal containment. | Keep this test hermetic during toolchain qualification. **S** |

Reserve these **activation steps** for the crossing; their code and tests can be prepared beforehand:

- **CONFIRMED requirement:** publish and reconcile the new effective five-DNA hash set, then retire/recreate the intended cells and keys. The inventory distinguishes packed hashes from installed hashes and identifies state requiring preservation: `alpha-state-inventory.md`, “Recovery limits.”
- **CONFIRMED requirement:** apply targeted account/key rebinding when those keys actually retire—F05’s corrected procedure, not a blanket Mongo wipe.
- **CONFIRMED candidate requiring an integrity move:** stronger AgentPeerBinding validation. Current `imagodei_integrity/src/agent_peer_binding.rs:155–189` checks structure, not cryptographic authorship. `arch-dataplane-borrows-backlog.md`, row 25, explicitly separates coordinator-only record retrieval from hash-moving integrity enforcement. **PLAUSIBLE recommendation:** absorb the agreed integrity subset; do not automatically expand into membrane work.

Assuming the supplied regenesis-2 scope, these interactions are **PLAUSIBLE**:

- **Holochain/fork:** F01–F03 and F14–F16 are the critical qualification chain. Preserve the single-flight fix while porting capability grants; test install, reconnect, signal delivery, and error classification against `CellNotRunning`.
- **DNA reinstall:** F04–F07 prevent a healthy new conductor from leaving apps, accounts, or peer bundles inconsistent.
- **Rust 1.100:** F07–F13 and F20 interact with build-directory assumptions, invalidated gate caches, new lints, and constrained source files. F09’s baseline cure already exists.
- **Security/npm/Angular:** F17–F18 and F21 need contract coverage. The previous crossing also exposed an ESM/CommonJS loader failure: `briefs/morning-2026-11-06.md:37`.

My **PLAUSIBLE top eight**, ranked by likely wall-clock or attention saved:

1. **F01** — prevent crash churn and forensic recovery.
2. **F03** — remove repeated rollout timeouts and operator pod deletion.
3. **F05** — avoid another multi-round account repair.
4. **F04** — restore app channels without remembered manual steps.
5. **F08** — reduce avoidable 20–40-minute receipt reruns.
6. **F06** — prevent mixed-artifact rollout diagnosis.
7. **F07** — eliminate unrecoverable freshness refusals.
8. **F10** — catch cheap style failures before expensive gates.

{"items":24,"confirmed_open":9,"top":["F01","F03","F05","F04","F08","F06","F07","F10"]}