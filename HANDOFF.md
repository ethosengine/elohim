# HANDOFF — substrate-scope toggle is wired (the household-vs-shem-remote separation, generalized)

_Last updated: 2026-06-03 · Author: Claude Opus · Branch: `fix/e2e-cucumber-playwright-polish` · shem declared DOWN (`cluster-state.yaml shem: available: false`)_

## What landed this session (the separation, built as a general cybernetic reconciler)

The substrate-scope separation is now **fully wired as a bidirectional toggle over ANY dependency point** — `shem` is one instance. `cluster-state.yaml` is the sensor, `@requires:<cap>` the setpoint, the mover + a new runtime gate the actuators, the SessionStart `scope:` line the feedback. One `@requires:<cap>` vocabulary drives **two arms**:

- **Planning arm** (`scope-reconcile.py`): feature-level `@requires:<cap>` → whole `.feature` git-mv'd to `held/` (out of cucumber's glob AND agentic search). **13 features held.**
- **Runtime arm** (NEW): scenario-level `@requires:<cap>` → `Before` hook in `genesis/a2o/steps/common.steps.ts` returns `'skipped'`. **13 mixed features scenario-gated.** Closes the seam where a shem scenario that didn't name a remote persona ran-and-failed.
- **65 features stay live** — the focus form. Household-multi-node scenarios (matthew/jessica/james are 3 real nodes) deliberately NOT held (`shem ≠ multi-node`).

**Files:** new `genesis/a2o/src/framework/fixtures/substrate-scope.ts` (cap-generic primitive, 36 unit tests) + `humans.ts` delegates to it; `genesis/Jenkinsfile` (probe reconciles blind→cluster-state, seeder holds remote-only genesis peers, derive helper); `cucumber.mjs` testnet profile globs survive the held move; `deployments.json` (james → jessica's 3Gi profile, adam/matthew comments reconciled); new `genesis/manifests/humans/james-son.yaml`; docs (`a2o/CLAUDE.md`, scope-tree-reconciler spec §9); memories (`project_substrate_scope_runtime_arm`, `project_alpha_topology_bootstrap_pair` updated).

**Verified locally:** typecheck 0 · lint 0 · 107/107 a2o unit tests · cucumber dry-run parses clean (held excluded) · `scope-reconcile` off→on→off cycle coherent · gate `aligned ✅`. **Code-reviewed** (48-agent multi-angle pass): 10 findings, 8 fixed, 2 documented as guarded invariants.

## The three original defects — all resolved

1. **Probe fail-open** → FIXED. `probeRemotePoolStatus()` reconciles a blind kubectl probe to `cluster-state.yaml` instead of failing OPEN (CI twin of humans.ts). The three homes (cluster-state.yaml · `ELOHIM_REMOTE_COMPUTE_STATUS` · held/ tree) cannot disagree.
2. **adam mis-classified** → FIXED at CONSUMPTION (drift-free). `runContentSeedStage` holds remote-only genesis peers (adam) when shem is down; matthew (household) carries ingest. `adam.genesisPeer:true` stays correct-when-shem-is-up. Stale "re-armed 2026-05-18" comments reconciled.
3. **james under-provisioned** → FIXED in the repo. `deployments.json` james bumped 1536Mi→3Gi (the OOMKill value → jessica's profile) + new per-human manifest. **Needs operator deploy** (no kubectl from dev env; matthew/jessica/james render from the `consolidated` template which reads deployments.json resources).

## Next steps (ordered)

1. **Push to dev → trigger deploy+e2e** (orchestrator-indexed; `sprint/*` self-skips). This is the CI validation the local work could not do.
2. **Confirm in the build:** `substrate-status.json` shows `remoteComputeStatus:unavailable` reconciled (not blind `unknown`); shem scenarios **SKIPPED/held, not failed**; the remaining failures are the true test-layer surface (Class D device-setup, Class E console-strictness — recipes in the sprint-result). `reports/substrate-scope.json` lists `substrateSkippedScenarios`.
3. **Operator: deploy james** with the new resources so the household is solid (carries the run when shem is down).
4. **Toggle when shem returns:** `scope-reconcile.py --set shem=on --apply` moves the 11 shem features back live + the runtime gate stops skipping. ⚠ **Footgun:** `--set` WITHOUT `--apply` still writes the durable home (only the *move* is dry-run).

## Loose ends (low priority)

- **1 UNSURE feature:** `features/federation/cross-doorway-content.feature` needs a SECOND doorway (`E2E_DOORWAY_STAGING`); left live (not shem). Operator to confirm whether staging-doorway is household-deployable, then tag or leave.
- **Latent edge (documented, not a bug today):** deleting/renaming the `shem` block OUT of `cluster-state.yaml` (vs `--set` which keeps it declared-false) makes the Groovy arm conservative-unavailable while the generic TS arm fail-opens. Caught by scope-reconcile's VOCAB-drift warning. Keep `@requires:` caps declared.
- **Groovy `deriveRemoteComputeFromClusterState` is shem-specific** (YAGNI — generalize only when a 2nd cap is needed in CI).

## Constraints (unchanged)

- **No kubectl from this env** — operator owns cluster ops; agent stays code-level.
- **No Jenkins WRITE auth** — trigger builds only via git push to dev. **`qahal-m1` worktree is the operator's — never touch it.**
