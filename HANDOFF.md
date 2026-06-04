# HANDOFF — peer-native OAuth portal finished + doorway→steward portal-handoff BUILT end-to-end

_Last updated: 2026-06-04 · Author: Claude Opus · Branch: `shift/a2o-greenup` · Session mode: implementing (diff is the work product) · **8 commits ready, committed-NOT-pushed — integrator owns the dev-merge/push**_

## Goal

Finish the peer-native captive OAuth portal (verified already LANDED — merge `a224c3e79`, phases A–F; the four deferred items are settled fast-follows, do NOT reopen) and **build the unbuilt doorway→steward portal-handoff**: a graduated steward authenticating at a doorway is handed to their own portal host (doorway = relying party, steward portal = identity provider), with fall-through to hosted auth when the portal is unreachable.

## Current Progress — the 8 portal-handoff commits (all on `shift/a2o-greenup`, interleaved with the concurrent session's quilt-policy commits)

| Commit | Surface | What |
|---|---|---|
| `6ada3ad55` | doorway-service | GAP-1: OAuth `/auth/token` response carries `portalHostUrl` (mirrors the built login-path probe; additive, RFC-6749 code+state untouched) |
| `1c70578cf` | elohim-app | GAP-3: "Manage from your steward →" on `/account/security` (`data-testid=portal-host-redirect`), bound to canonical `AccountView.isSteward` + `portalHosts[].lastReachableAt`; fall-through when unreachable |
| `49626d995` | a2o | GAP-5a: step defs for `steward-login-portal-handoff.feature` — all 4 scenarios bind (0 undefined); scenario 4 held at runtime by `@requires:shem` |
| `48bca05f3` | a2o | Stale `.claude/memory/*` cites → MemPalace pointers (memories graduated 2026-06-02) |
| `3b51e94f1` | doorway-service | GAP-2a: **convergence decision** — handoff REUSES existing `GET /auth/session-token` (mint) + `GET /auth/exchange-session` (redeem); routing-contract tests guard both as auth-owned (37c822d1c shadow-bug class) |
| `0e9ef8357` | elohim-storage | GAP-2b: `POST /session/exchange` — TOFU issuer allowlist (origin-normalized vs `local_sessions` history; 403 unknown), back-channel redeem (401 invalid / 502 unreachable), seeds LocalSession stamped with the VALIDATED issuer URL, httpOnly `elohim_session` cookie; `/auth/me` prefers cookie-named session (Tauri single-active behavior unchanged); NOT in `build_manifest()` |
| `f3f881242` | imagodei-portal | GAP-4: steward-login step — consumes `?session_token&doorway_url` post-consent, exchanges at own storage, strips params via `history.replaceState` (OAuth params survive into consent), authority RE-DISCOVERED from `/auth/me` (no `peer-conductor` literal in prod code) |
| `6011d00d4` | doorway-app + elohim-app | GAP-2c: both redirect flows mint a single-use code — **the doorway JWT never rides a URL**; mint failure falls through (login never blocked / hosted view stays) |

## Key decisions (don't re-litigate)

1. **p2p-design-gate ratified redemption-callback** (OAuth authorization-code semantics) for the session_token. Token = Operational (Category C), no DHT entry, issuer-side truth. The gap-map's "storage verifies doorway-signed JWT" sketch is **impossible**: doorway JWTs are HS256 (`jwt.rs:235,289`) — sharing the secret would let any steward forge anyone's tokens.
2. **No new doorway endpoints.** Mid-sprint discovery: doorway already had the exact primitive (Session Transfer Store, `auth_routes.rs:44-128`, 60s-TTL single-use, already consumed by HandoffService/account-guard). A parallel `POST /auth/handoff(/redeem)` prototype (599 lines, Mongo-backed) was deliberately reverted — one store, one vocabulary.
3. **Client-driven redirect, no doorway 302** (matches the dissolved design verbatim); a2o browser steps assert the resulting navigation URL.
4. **Conductor stays authority**: exchange only SEEDS a LocalSession; `trustMode` is discovered from `/auth/me`, never configured.

## Verification (all run this session, all green)

- doorway-service: 533/533 lib tests · clippy `-D warnings` clean · fmt
- elohim-storage: 1337/1337 lib+bins · 208/208 schema_contract · fmt · ts-rs export byte-stable
- elohim-app: 4508/4508 full Vitest · touched-files lint clean
- doorway-app: 15/15 · lint clean
- imagodei-portal: 29/29 · `tsc --noEmit` · `ng build`
- a2o: `cucumber-js --dry-run --tags '@auth-portal-convergence'` → 10 scenarios, 62 steps, **0 undefined**

## What Worked / What Didn't

- **Worked:** fixing the wire contract up-front let 4 surfaces build in parallel; injectable redeem seam in storage made the endpoint-swap (handoff→exchange-session) a 1-function change; the account-m5 step helpers cloned cleanly.
- **Didn't:** two background-agent generations died to harness restarts; one agent was wrongly presumed dead from transcript mtime (**clock skew between container clocks** — `date` said 06:15 while vitest stamped 12:13) and its late writes raced mine on `threshold-login.component.spec.ts`. If you see `Promise.resolve()`→`settle()` style mystery edits, check for a live concurrent writer before assuming linter.

## Next Steps (ordered)

1. **Integrator: dev-merge + push** (orchestrator-indexed; `shift/*` itself is NOT — CI runs on the dev merge).
2. **CI watch:** doorway + storage + app pipelines; then the a2o browser runs — scenarios 1–3 of `steward-login-portal-handoff.feature` are household-testable NOW; scenario 4 stays held until `scope-reconcile.py --set shem=on`.
3. **Live-stack smoke** (per `recovery-m5-doorway-handoff-to-steward.feature`): steward login at doorway → redirected with `session_token`+`doorway_url` → portal exchanges → lands authenticated `/account/security`, `trustMode: peer-conductor`.
4. **Known infra gap (operator):** `app/imagodei-portal` has no eslint config / `lint` script / `build-manifest.json` / pre-push detection entry — strict tsc is the stand-in gate. Needs standing up to match other Angular surfaces.
5. **Pre-existing, untouched (out of scope, reported):** 4 `-D warnings` clippy findings from rust-1.95.0 in storage files not mine (`infrastructure.rs:1813`, `rea_commitment_service.rs:319`, `replication_prioritizer.rs:159`, `controller.rs:1824`); ~665 pre-existing elohim-app repo-wide lint errors (my files clean); a2o repo-wide lint debt (~68 problems outside my steps file).
6. **Fast-follows already settled as NOT-gaps** (canonical-surface drawer): recovery-launcher primitives, hosted→native migration UI, dynamic OAuth-client registration, cross-doorway PortalHost lookup. Don't reopen as gaps.

---

# PRIOR HANDOFF (2026-06-03) — substrate-scope toggle is wired (still-live operator items: deploy james, push-to-dev validation)

_Branch: `fix/e2e-cucumber-playwright-polish` · shem declared DOWN (`cluster-state.yaml shem: available: false`)_

## What landed that session (the separation, built as a general cybernetic reconciler)

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
