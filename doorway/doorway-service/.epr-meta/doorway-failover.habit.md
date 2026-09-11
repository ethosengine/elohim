---
epr-habit-version: 1
id: doorway-failover
invariant: >
  The apex name serves a person the landing shell through the loss OR
  catch-up shed of either doorway: siblings are honestly classifiable
  (serving | shedding | dead), at least one always serves, the name rides
  through a sibling's shed, and whichever serves resolves the same
  declared head. Federation failover, not per-host luck.
status: red
active: false
checks:
  - "Packaged browser data use: just test mesh features/dataplane/doorway-runtime-endpoint.feature; the identical canonical package reads and renders the manifesto at both household doorway origins, with no substrate requests escaping to the compiled production host. This healthy-origin prerequisite does not certify sibling selection during an outage."
  - "Canonical fixture prerequisite: just test mesh features/dataplane/doorway-fixture-readiness.feature; both roots name assets/build stamps from the declared landing bundle and a non-primary holder answers the manifesto. Execute after the separate unclaimed-root app-delivery proof; never overwrite a canonical root to replay that proof."
  - "Actual public-name transition authority: A2O_RUN_WIP=1 just test mesh features/dataplane/doorway-apex-transition.feature. This born-red feature must have every scenario and step passed before graduation; WIP filtering, undefined steps, pending, skips, absence, or steady-state doorway-failover.feature green cannot discharge it. It separately requires induced shed, owner-only membership withdrawal/rejoin, sibling delivery through the unchanged public name, browser bootstrap and recovery. WAN ingress continuity is a distinct prerequisite, not implied by multi-A membership."
  - "Beacon shared-membership bridge: relay-addr-beacon/justfile gate with explicit native cargo-pool slot and empty RUSTFLAGS; production-cycle mock tests prove join2/leave3 hysteresis, WAN-independent withdrawal, exact-owner writes and retries. This foreign bridge measure does not certify public DNS propagation, WAN balancing or actual apex host selection."
  - "SDK package/adapter checks: just gate epr-app-package; current Angular and supplied non-browser adapter archives are validated before publication; runtime proof is separate."
  - "a2o @concern:doorway-failover (genesis/a2o/features/dataplane/doorway-failover.feature — @act:i, so its authority is the household lane: `just test mesh features/dataplane/doorway-failover.feature` against the built binaries, run-identified report under genesis/a2o/reports/. It is HELD on the edge Dataplane Validation stage by LAYERS.md design (Act II drops owned-substrate) — a fleet build number can never measure it; the fleet contributes only the deploy that carries the same commit.)"
  - "a2o @concern:doorway-failover (genesis/a2o/features/dataplane/served-shell-boots.feature — @act:ii, the browser-shell clause of the invariant: the page a doorway hands a visitor at `/` names only assets its declared browser head holds. Runs against the deployed fleet (edge Dataplane Validation, `[edge:validate-only]` too) and from any host: `cd genesis/a2o && E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host pnpm exec cucumber-js --name 'handed a page that can boot'`; RED 2026-09-04 naming main-EAKNZDUP.js -> 404.)"
  - "a2o @concern:doorway-failover (genesis/a2o/features/dataplane/served-shell-boots.feature — Act II dynamic clause: Chromium must observe a visible root with the browser-only bootstrap-success marker, no page errors or failed same-origin requests, and version.json equal to the immutable declared browser head. Both doorway origins are measured independently; redirecting to a sibling cannot certify the requested doorway.)"
  - "a2o @concern:doorway-failover (genesis/a2o/features/dataplane/epr-app-deliverability.feature — Act I: just test mesh features/dataplane/epr-app-deliverability.feature runs every station, including Chromium. Checks browser boot through both public mounts; server declaration and bytes on all peers; actual SSR and running-version adoption at both doorways; warmed rendering with storage unavailable; restart-before-storage recovery; and broken-bundle refusal/fallback. The serving receipt requires all stations passed for the current source, with no skipped or pending proof.)"
refs:
  - "genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md"
  - "genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md"
  - "2026-08-06: one layer below this invariant, doorway ZomeCaller grew multi-conductor failover for agent-agnostic DHT reads (reviewed ship, gates green — backlog conductor-source-chain-unwrap-panic-db-timeout.md), so a doorway's DHT routes no longer die with its pinned conductor; the apex-name born-red and its WS3 cure surface are unchanged"
  - "2026-08-07: live evidence toward the invariant — through the Wave-2 iroh transport flip (edge #1313, full 7-conductor restart) BOTH hostnames served the federation route with no visible outage window (measure 0->2 in minutes, vs the ~2.5-3h tx5 pre-fix churn the same night); attribution failover+fixed-binary+fresh-peer-stores, not decomposed; the a2o @concern:doorway-failover scenario remains the flip-to-green authority"
retire-when: >
  when the apex no longer depends on a doorway pair: a person's client resolves the
  declared head from the mesh directly. At that point "either doorway can die" is a
  statement about a component that no longer sits in the path.
---
DELTA 2026-09-11c (SERVING RECEIPT on current source; habit RED preserved by rule): epr-app-deliverability.feature
**5/5 scenarios, 102/102 steps** on a fresh household mesh (run 20260911T081627Z-62fb8fb6, doorway eff30a245 build,
storage fb4d10c7d build); serving-receipt validator EXIT=0; the pre-push gate accepted it and 105 commits landed on
origin/dev at 2c338124a (753b766a1..2c338124a, gates ALL CLEAR 1388 s). Two findings on the way: the deliverability
cleanup must cancel the owned root through its AUTHOR peer only (zome rule from 2cea494ee; glue 62fb8fb64), and a
mesh cold start keeps storage projection DBs (two receipt attempts lost; backlog
mesh-cold-start-leaves-storage-projection-dbs-2026-09-11). Fleet confirmation = the edge/app builds this push dispatches.

DELTA 2026-09-11b (steady-state GREEN on the household pair; habit RED preserved by its own rule): run
20260911T041537Z-faca0d95 — doorway-failover.feature **10/10 scenarios, EXIT=0** on the mesh at faca0d95e (doorway
3c7ee8d89 build 04:03:51, both doorways restarted 04:11:38). The four reds of 02:41/03:24 were the Prologue refusing
to stage a bundle without version.json (cured: hc-mesh-prologue stamps it, 7a31996e0) — never doorway code.
Apex-transition (20260911T041555Z-faca0d95, A2O_RUN_WIP=1): 0 undefined, both scenarios fail at the named step
(no household-owned membership authority); scenario 2 SIGSTOPs the apex doorway and the sibling serves the same
declared head before the same-public-name step fails. Graduation still requires every apex-transition step passed +
WAN ingress (plan Task 21, held). Mesh preflight now REFUSES a pool binary older than its source commit (4710606dc).

DELTA 2026-09-11 (RED preserved; apex-transition is now a MEASURED red, not undefined): run 20260911T024151Z-b6c7947b
under A2O_RUN_WIP=1 — 2 scenarios, 16 steps, 0 undefined. Scenario 1 fails at the predicted named step: `no
household-owned membership authority: relay-addr-beacon reconcile_membership writes owner records through the
Cloudflare sink only … hc-mesh.sh stages no membership apparatus` (steps ea7c70026; scenario 2 really SIGSTOPs the
apex doorway and proves the sibling serves the same declared head before failing at the same-public-name step).
Steady-state doorway-failover.feature this run: 6/10 (20260911T024122Z) — the 4 reds (both local doorways
`shedding`, :8889/ 404, warmup.completed with servedBundleHeads []) coincide with the Prologue's staging legs failing
after a provisioning regression on the mesh (see hosted-human-lifecycle 2026-09-11b); cause under diagnosis, not
attributed to doorway code. Doorway /status wiring (1d8ca0200) and the dev_mode decouples (60fb28a39, f64d8c5bf)
are local-green, unrendered on the fleet.

DELTA 2026-09-08 (authorized current-source household receipt, RED preserved): sequential upgrade of three storage peers and both doorways completed with health/kernel-hash parity; run 20260909-codex-current-deliverability (executed 2026-09-08) passed all 5 deliverability scenarios / 102 steps, no skips or failures, 4m26s, zero findings. Both roots booted; server declarations and bytes propagated to all peers; rendering and next-version adoption passed at both doorways; warmed serving survived all-storage outage with successful cleanup; restart-before-peer recovery and broken-bundle refusal passed. The earlier Jessica server-pointer mismatch did not reproduce on current binaries. Receipt records dual/3 peers/processControl; networkStage and DNA hash remain unknown, with source inventory and actual executable hashes recorded separately. Evidence: genesis/a2o/reports/delivery-20260908/current-deliverability.log and runtime-upgrade-health-parity.json; sprint-report-household-20260909-codex-current-deliverability.json under genesis/a2o/reports/. No fleet deployment or apex routing-through-shed proof is claimed; whole habit remains RED.

DELTA 2026-09-08 (Codex delivery sweep, RED preserved): three native commitments independently reviewed and fulfilled: missing-root HTML no longer invents conductor failure (just gate doorway EXIT=0, 1209 passed/2 ignored; desktop/mobile template renders); local served-shell probes honor CI=false and require real Jenkins build context before installing (just gate genesis-a2o EXIT=0; 36 browser/bootstrap controls passed); storage fault-stop refuses mismatched/missing executable bytes, incompatible transport or failed capture before signal (just gate elohim-app EXIT=0, 4573 tests; 12 stop controls and 6 release-slot controls passed). The prior-runtime full deliverability run 20260908T2251Z-codex-deliverability measured 1/5 scenarios passed: Jessica held server metadata but had a null top-level server pointer, and replaced cargo-path bytes broke outage cleanup; all peers were subsequently restored without data reset. Current-source storage/doorway builds succeeded, but automatic approval review refused the controlled five-service upgrade before execution; explicit operator approval is pending for upgrade plus final fault run. Fleet root probe still fails its version.json stamp; no delivery graduation or fresh serving receipt is claimed. Reports and native briefs: genesis/a2o/reports/delivery-20260908/.

DELTA 2026-09-08 (root-route and host-capability regressions closed locally; fleet RED preserved): the Act I browser station now mounts its owned fixture at the real `/` address, refuses any existing root projection or active `project-epr` root commitment through a bounded exhaustive page walk, and restores every owned peer explicitly after the run. Household proof 20260908T192233Z-0e89181b passed the root station 1/1 (23 steps) with both roots absent after cleanup. This exposed `/version` prefix ownership swallowing the app's `/version.json`; doorway now reserves `/version` exactly, with a focused Rust regression green. A rebuilt learning-app browser showed Learning Paths with zero page errors and zero WASM requests after imported DataLoader services began honoring the consuming bundle's cache capability. The complete five-station receipt is invalidated by these source changes and must be re-earned from the committed tree; no fleet carrying them exists yet, so status stays RED.
DELTA 2026-09-08 (RED preserved; first hard fleet gate exposed runtime incompatibility): app #1698 failed SSR verification after publication at 3c2d57167; edge #1444 was still building, so this is evidence against the preceding runtime, not proof of the new convergence deployment. Live Loki and the same compiled bundle identify `ws` requiring `util.types.isUint8Array`, absent from the renderer shim. The local shim correction passes six compatibility tests and reaches actual bundle HTML. The declaration/adoption probe now distinguishes a missing canonical server pointer (90-second declaration bound) from renderer adoption (400 seconds); Delivery regressions cover no extra declaration request after the deadline, transient adoption within a wall-clock budget, and rejection of loading/serialized-state HTML when the real h1 is required. Runtime-source changes now invalidate the mesh receipt. Actual compiled Lamad now renders the full public /lamad/path/elohim-protocol route with its real h1 (3 completed fetches, 41660 HTML bytes), after fixing Angular pending-task tracking, zoneless view notifications, browser-only preference access, and SSR base-path parity; 56 focused frontend and 170 renderer tests pass. SDK packaging rejects base-path drift. Fleet verification remains outstanding.
DELTA 2026-09-08 (Act I proven; fleet RED preserved): household run 20260908T164209Z-e7401bfc passed all 5 deliverability scenarios and all 102 steps with zero findings (genesis/a2o/reports/sprint-report-household-20260908T164209Z-e7401bfc.json, SUT sha256:d9f87508c5ac4f04). It proves browser boot through both doors, canonical server identity and bytes across three peers, running-renderer N→N+1 adoption, local SSR cache service while peers are down, cold doorway recovery, and broken-bundle refusal/diagnostics. SDK packaging/adapter gate passed 19 tests; app lint/build and 4571 tests passed; receipt rejection tests passed 9. The shared staging retry remains bounded and deterministic broken verdicts remain terminal. Doorway/storage native gates are green as recorded below. The spec graduates to Active; fleet served-shell validation on a build carrying this change still owns the remaining RED.

DELTA 2026-09-08 (developer packaging continuation, RED preserved): the SDK now shares deterministic archive checks with CI staging and exposes `just dev package <app-directory>` with an Angular adapter plus an explicit local BYO adapter interface; SSR is optional web behavior, not an EPR-app-wide requirement. Adapter tests exercise a packaged/extracted non-browser executable and readiness separately from packaging. A real local Lamad build completed and produced checked browser/server archives via this command (logs genesis/a2o/reports/packaging/lamad.log); no runtime or publication claim follows from that package result. Doorway gate now passes 1206 library tests plus 2 binary tests, including preserving the measured missing-asset reason on honest503. The revised Act I story is READY after file-only review (5 scenarios/103 steps all bound); the final storage gate passed 3661 library tests plus integrations and doctests, including late-snapshot and metadata-erasure regressions (genesis/a2o/reports/deliverability-gates/storage.log). Its full mesh run remains pending, so no habit flip is claimed.

DELTA 2026-09-08 (continuation, RED preserved): independent review closed the observe-without-retry projection gap and broken-shell stocking gap; doorway gate passed 1205 library tests (2 ignored) plus 2 binary tests, and browser/SSR redirect negative controls passed 18/18. The first owned-mesh integration run (reports/sprint-report-household-20260908T151711Z-e7401bfc.json) measured all 5 scenarios and failed all 5, exposing fixture mount/base-URL/restart issues and server-pointer non-convergence; it is evidence of failure, never a serving receipt. SSR remains doorway-resident rendering/cache work, while executable server selection is being bound to the existing notarized Content metadata instead of unauthenticated sync gossip. A passing final household receipt and a fleet build carrying this change remain the delivery authorities.

DELTA 2026-09-08 (RED preserved; spec 2026-09-08-epr-app-deliverability-through-doorway D1+D2 landed LOCALLY in the doorway, unmeasured on mesh or fleet): the doorway's declared app head is now STORAGE-AUTHORITATIVE, not signal-fed — a BundleHeadsReconciler (render/bundle_heads.rs) reads GET /db/content/{slug} (2 s bounded) for every EPR-mounted bundle plus every configured SSR slug, on a BUNDLE_HEADS_TICK_SECS=30 tick AND on every content.created|updated event, and writes blobHash + serverBlobHash through to the in-memory slug index and the durable projected entry, evicting the warm shell when the browser head moves; storage unreachable keeps the last state and never fabricates or un-declares a head. That closes the 2026-09-08 mechanism directly: post_commit is cell-local and the content.updated bridge only cleared the slug index, whose fallback re-resolved out of the same stale projection, so both apex names served browser head sha256-6899… for 15 h after storage moved to sha256-e0e2f7… (page named main-AVSOD6V6.js -> 404; main-SQRMM2WZ.js 200 on the slug path). D2: AtHead now needs DELIVERABILITY as well as provenance — head_bound proves only WHERE bytes came from, and that shell was provably fetched by its declared head. plan_shell_serve consults a coherence oracle (render/coherence.rs: HEAD /apps/{head}/_capability reading storage's own X-Deliverability, falling back to GET+Range on the entry script the shell names, because /apps/{id}/{file} is GET-only and a HEAD probe 404s a file that GETs 200); a confirmation is memoised permanently (content-addressed head, so an unreachable peer can never un-prove it), a negative for 30 s. An unproven head serves the bytes in hand as `x-elohim-bundle: behind;<reason>` (missing-asset:<file> | head-unknown | storage-unreachable | stale-projection) with x-elohim-freshness: amber unchanged; `last-reconciled` and `slug-resolved` keep their exact meanings (additive — a2o reads them); nothing coherent at all answers a one-line converging 503 + Retry-After: 20, never a blank 200. The SSR adoption pass reads the reconciled server head in both PRE-materialize arms while both POST-materialize attestation reads stay live, so the TOCTOU guard is not weakened into a tautology. Gates on the branch: cargo fmt --check EXIT=0, clippy -D warnings EXIT=0, cargo test --lib 1200 passed / 0 failed / 2 ignored EXIT=0 — 26 new tests (9 warm_shell, 8 bundle_heads, 7 coherence, 2 http) and 4 seam-registry rows (judge_coherence, BehindReason, CoherenceVerdict, HeadMove). Status stays RED: the flip authority is the household mesh lane plus a fleet build carrying this commit with served-shell-boots / epr-app-deliverability green — nothing here has been measured on either. Spec D3 (serverBlobHash converging peer-to-peer) and D4 (the blocking boot-through-doorway test) are separately owned and not in this DELTA.
DELTA 2026-09-05 (RED preserved; slice 1 of the delivery verdict landed LOCALLY): the storage peer now judges every app head from its bytes inside the extraction walk (app_deliverability::judge_deliverability, memoised per hash) and says so on /apps surfaces (X-Deliverability / -Reason); stage-spa-blob.sh reads the verdict by content address and exit 2 (broken) is terminal; authorHeadOnce skips a BROKEN_HEAD bundle (no head minted) and one error() after Phase 2 fails the build; the gate applies to browser bundles only — server bundles are judged by the renderer in slice 2. Standalone storage proof (port 8095, isolated STORAGE_DIR): good bundle `X-Deliverability: boots` / gate EXIT=0; bundle missing main-A.js -> `broken`, gate `missing-asset:main-A.js` EXIT=2; counter lines: `elohim_app_deliverability_verdict_total{reason="missing-asset",verdict="broken"} 1`, `elohim_app_deliverability_verdict_total{reason="none",verdict="boots"} 1`. Mesh story run deferred to the fleet build (lease held). Storage lib 3417/3417, seam census: judge_deliverability registered, 4 contract tests. Flip to green still needs the fleet build carrying 44b5d94b5 + this slice with the scenario green there.
DELTA 2026-09-04b (RED preserved; cure landed LOCALLY, unmeasured on the fleet): the `/` shell path now addresses the shell by the declared head it will be stocked under (`/apps/{head}/{entry}`), marks archive docs `head_bound` so legacy/poisoned docs self-heal with one upgrade fetch, never classifies an unknown head AtHead (Behind + one atomic upgrade claim per 30 s, fresh slug-resolved bytes replace the hot shell and carry `x-elohim-bundle: slug-resolved`), serves the bytes in hand when an upgrade fetch fails instead of the upstream's 404/503, and evicts the warm entry on `/admin/cache/clear/{slug}` + the projection invalidation task. warm_shell 24/24, full doorway lib 1162/1162, fmt + clippy -D warnings clean; Codex adversarial review BLOCK -> SHIP after round 2. Flip to green needs served-shell-boots.feature green on a fleet build carrying this commit. The projection blindness it worked around is filed: genesis/data/timeline/backlog/2026-09-04-doorway-projection-never-carries-app-row-heads.md.
DELTA 2026-09-04 (GREEN -> RED on live evidence; the invariant's last clause failed on both names): from ~04:23Z both alpha.elohim.host and elohim.host answered 200 at `/` with a shell from the PREVIOUS bundle era (main-EAKNZDUP.js, app build #1691) while the row declared blobHash sha256-7725d4… (main-7QFGHX5X.js, build #1692, PATCHed 04:52Z) — entry script 404, blank page for every visitor. Mechanism (Opus + Codex RCA, converged): the doorway's warm-shell cache fetches the shell by SLUG and files it under the head its own projection declares; on the fleet that projection carries no head for app rows (DEV_MODE wiring drops the projection engine's signal sender at boot, and the storage `content.updated` event clears only the /apps slug index), so a shell stocked with an empty head classified AtHead and was never re-fetched; the SSR breaker had been OPEN since 04:33Z (build #1691's server bundle panics in the renderer), so every `/` rode the EPR-router warm arm. Evidence: Loki `EPR router: shell served from the warm-boot cache — no upstream fetch, head:""`, `HEAD /apps/elohim-host-landing/_capability` x-projection-ready:false, the new @regression scenario "A visitor asking for the site root is handed a page that can boot" (served-projected-head.feature) RED against the fleet at 14:40Z naming main-EAKNZDUP.js -> 404. Cure in flight: hash-bound shell fetch + head-bound archive marker + unknown-head never AtHead (rate-limited refresh) + admin eviction of the warm hot entry; re-green needs that scenario green on a fleet build carrying the cure. Separately named, not fixed here: apex API host doorway.elohim.host 503 (ingress backend in a prod namespace with zero pods).
DELTA 2026-09-01 (GREEN preserved; cross-seam serving honesty hardened): doorway `/health/serving` now performs one primary-scoped, 2-second-bounded read of storage's own `/health/serving`; storage 503 demotes doorway 503 without opening the general read breaker, preserves storage's `Retry-After: 20`, and renders typed `serving | refused | unreachable | not-configured | not-observed` evidence while `/health` remains 200 liveness. Focused doorway health suite: 35/35 green; concern census: 0 findings.
DELTA 2026-08-25b (RED -> GREEN on measured evidence; the flip authority
was mis-assigned): the fleet's first Dataplane Validation admitted by the
quiesce gate since the acts layering (edge #1381, validate-only on
286089679/47fb60f58, A-QUIESCED sustained 06:54:51Z) HELD 81/87
scenarios — `owned-substrate (act i baseline)` is unavailable on alpha by
LAYERS.md design, and doorway-failover.feature is @act:i. So "flip
authority is the fleet lane" (2026-08-23h) named an authority that
cannot exist for this check; the household lane is it. Evidence for
green: @concern:doorway-failover 10/10 (41 steps) on the household mesh
from the p2p+p2p-iroh binary built from 608a1ceff (run
2026-08-25 push-verify, report genesis/a2o/reports/push-verify/doorway-failover.json;
the same doorway + storage commit deployed to alpha by edge #1380
SUCCESS and confirmed live: irohNodeId, 6 peers, syncDocuments 5356),
after 10/10 on 2026-08-23h and 2026-08-23b. Both named reds stay cured
(primary-scoped health classification 530431088, guarded half-open
probe 99df9f72c). What the fleet DID measure: substrate seam-smoke OK on
both doorways and both relays; the one fleet red is inventory-convergence
(alpha-A p2p.caughtUp flap — matthew's heal-leg class), not a failover
property. Correction recorded: `epr flow note --kind correction` on the
feature. Re-red condition: any household run with a failing
@concern:doorway-failover scenario, or a fleet incident where the apex
name fails to ride a sibling's shed.
DELTA 2026-08-23h (both NAMED REDS cured; status stays red — flip
authority is the fleet lane): (a) health classification scoped to the
DECLARED PRIMARY (530431088) — ServingHealth::observe takes the primary
from the same declared order select_route ranks (one derivation,
config.rs declared_storage_peers); pool upstreams stay visible with an
additive role field; red-first proven by reverting the filter (2 tests
fail on the old any()). (b) periodic guarded half-open probe
(99df9f72c) — open pool circuits are trialed via the breaker's own
begin() RAII guard from the 30 s refresh loop (separate 2 s client, no
second admission path, trial-theft suite re-green; disable knob
DOORWAY_BREAKER_PROBE_DISABLED). 1086 doorway tests green. Independent
adversarial review: probe + scoping CLEAN (sequential probe N x 2 s
noted as follow-up); all five deployed doorway manifests declare a
primary so the None fallback never fires deployed. Local lane on the
cured binaries: @concern:doorway-failover 10/10 (run
20260823T212812Z-ad5d8f49, sut 9a94e137, transport dual). Confirm with
[build:edge] — this arc has real code to deploy, not measure-by-deploy.
DELTA 2026-08-23b (continuity session — selection-time blob failover + RS
ingest + roadmap; household lane closing run 20260823T150653Z-3a77a458):
189 passed / 25 failed / 4 pending / 28 skipped in 31m15s, from 186 / 25 /
4 / 29 this morning. @concern:doorway-failover 10/10 incl. the NEW story
"A blob rides through its primary's bad hour" (SIGSTOP the primary, trip
the breaker with a burst, Range-read past the pantry, bytes arrive from a
sibling; doorway_blob_target_failover_total 0->2 live). CURE: select_route
(server/http.rs:115) walks the declared priority order for /blob/ paths and
takes the first endpoint whose breaker would_shed()==false; all-shedding
falls back to the declared primary; non-blob routes keep matches.first()
(projections never float). 6 red-first unit tests; seam-registry row C7.
Gate green under rustc 1.98 after clearing result_large_err x3 +
items_after_test_module (toolchain drift, untouched files). TWO DEFECTS THE
PROOF SURFACED, both fixture-side and both cured: (1) the landing SSR
bundle (71,763,974 B, now carrying source maps) crossed RS_THRESHOLD and
PUT /blob panicked storage at http.rs:2617 (rs-4-7 manifest hand-sliced as
raw chunks; the RS band NEVER worked through PUT) — cured 8854f6de5:
shards come from create_shards, hash/count mismatch is a hard 500 with
nothing stored, reads reconstruct through parity (>=4 of 7), 4 red-first
integration tests; live: 201 on all three peers, byte-identical GET; a2o
">64 MiB artifact accepted whole, served whole" added to blob-durability.
(2) hc-mesh.sh since 123cea498 severed DOORWAY_ID/DOORWAY_HEALTH_PORT from
doorway A's launch (a comment block ended the continued assignment list)
— A booted with a random doorway_id, matched ZERO project-epr rows, / 503
and /lamad 404; the 14:30Z lane showed it as 13 cascading reds (38 failed);
cured 3a77a458b with a rail comment. Also: seed-forward budget now scales
with size (596a1c928); beacon shared lanes repeatable (Codex, 906d7b159);
MESH_TRANSPORT_BACKEND knob (Codex, f86dd32d5; three-mode boot proof
pending). NAMED RED, lines, not yet cured: (a) routes/health.rs:377-378
computes serving.shedding/degrading with any() over ALL upstreams, so an
open circuit on a POOL peer (8091, errorStreak 3 after a drill) demotes
doorway A to degraded/shedding while its primary is closed — the two
cross-doorway-content "alpha degraded" reds and a sibling-classification
lie (the invariant says siblings are honestly classifiable); (b) nothing
ever trials a pool peer's open circuit (warm-up skips open, select_route
skips open, projection fallback fires only on a primary miss) so it stays
open until the next boot — needs a periodic guarded half-open probe.
Still red, classified: identity join 3 (household-formation 1,
conductor-spin 2) · self-healing-flow-control 5 · delivery-diagnostics 3 ·
epr-cross-peer 2 · chaos-peer-churn 3 (custody precondition — backlog
seed-custody-coverage-for-drill-content) · web2 cache 1 · conductor-visibility 1
· content-sync 1 + peer-mesh 1 (caughtUp under load) · stewardship-allocation 1
· "same truth" 2 (ch10 stewardingCollectives alpha-A=1 vs elohim.host=2;
footprint convergence) — green this morning, red in both runs today;
not localized (the storage rows themselves read None on all peers).
Status stays RED: flip authority is the fleet lane
([build:edge] [edge:validate-only]). Roadmap:
genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md.
DELTA 2026-08-22 (household lane, run 20260822T170136Z-519d4f6b, sut
sha256:2c012553c2e79dfb, 3-peer mesh + two local doorways): @concern:doorway-failover
passed=9 failed=0 — the first MEASURED green for this concern since the cures
above landed. Status stays red on purpose: this habit names the edge Dataplane
Validation lane as its flip authority, and the fleet has not run since #1376
(11:08Z, 76/80 skips). Confirm with `[build:edge] [edge:validate-only]` — the
cluster is quiesced (matthew caughtUp, both doorways 200 on the canonical head),
and a bare [build:edge] would throw that away to measure it.

DELTA 2026-08-21c (freshness verdict + amber pantry landed host-green,
commits 5da4c3b0e + ccf0138a9): status stays red — the a2o scenario is
the flip authority.
DELTA 2026-08-21b (CORRECTION to this habit's own 2026-08-18 evidence,
and the amplifier of the 503 above): the entry below claims "Task 3.4
warm-boot shell cache LANDED desk-proven ... / is cache-first ... both
local doorways shed in 1-4ms (was 10-20s class)". The local proof was
real; the DEPLOYED claim was false, for the whole of that cure's life.
main.rs builds AppState through with_pool (343) or with_services (345),
and BOTH set warm_shell: WarmShellStore::inert() because no archive
exists at construction; the only constructor that ever built a live
store, with_projection, has ZERO production callers. The archive arrives
later in init_projection, which installed app_file_cache and never
rebuilt the store. An inert store's lookup_with_declared returns Cold
BEFORE it consults the hot map, so stock() writes are unreadable,
hydrate() returns 0 unconditionally (the boot log's "hydrated: 0" reads
as a cold archive, not a disabled one), decide_shell_serve(Cold, true) =
Fetch, and `/` paid a full EPR_DISPATCH_TIMEOUT_SECS fetch on EVERY
request plus a second one through the ProjectedEpr fallback. That is the
20.751s `/` measured on 2026-08-21, and those paired 10s failures are
what opened the breaker in the delta below. The suite had NINE tests for
this feature and every one built a store WITH an archive — nothing tested
the shape production actually used. Cured: init_projection now calls a
named bind_warm_shell_to_archive() (named, because the invariant is that
every archive-installing path must rebuild the store), with rails
stocking_an_inert_store_still_serves_nothing,
an_inert_store_hydrates_nothing, and
binding_the_archive_lights_the_warm_shell_and_the_invariant_holds.
`just gate doorway` exit 0. THE GENERAL LESSON, which is why this
correction is written here rather than only in the commit: a local proof
of a CACHE says nothing about whether production CONSTRUCTS that cache.
"desk-proven" is not "wired".
DELTA 2026-08-21 (STAYS RED — cause NAMED and four cures landed
host-green; the a2o @concern:doorway-failover scenario is still the flip
authority and has NOT run): the apex 503 was never the projector and
never adam. Four links, each measured. (1) STORAGE's /apps extraction
flight leaked a herd: a waiter whose post-wait cache re-check MISSED fell
through and extracted WITHOUT ever registering in in_flight, then created
a guard that broadcast finish_extraction for a flight it never held. So
when the first extractor's put_app failed, every waiter missed and all
became simultaneous extractors, and their concurrent put_app calls raced
evict_app's remove_dir_all against each other's directory writes ->
"Failed to cache extraction (non-fatal) … Directory not empty (os error
39)", logged TWICE 8ms apart, identifier=elohim-host-landing, 2026-08-20
20:11:27Z. put_app clears the index entry BEFORE writing and restores it
only on success, so each failure left the app permanently uncached and
every later request re-extracted the whole bundle — self-sustaining.
(2) The SSR shell fetch rides EPR_DISPATCH_TIMEOUT_SECS=10, SHORTER than
the proxy's STORAGE_PROXY_REQUEST_TIMEOUT_SECS=12, so the homepage render
trips the breaker first; three timeouts in ~25s = the threshold.
(3) The breaker is keyed by ENDPOINT, not route, so it then sheds EVERY
route on that peer — /db/content answered in 40-114ms throughout and was
shed as collateral. Two sites also recorded a Failure for an upstream
429/503, against ProxyOutcome::classify's explicit rule that answered
backpressure proves liveness. (4) UpstreamBreakers::is_open() is a GATE,
not a read — it advances Open->HalfOpen and CONSUMES the one half-open
trial — and two `/`-path planners called it with no BreakerTrial guard,
so no outcome was ever recorded and the circuit re-latched every
STALE_HALFOPEN_COOLDOWN_MULTIPLIER x cooldown = 120s. Fixed 2026-07-21
(f5e22baa2), REINTRODUCED 2026-08-18 (f0b908660, warm-boot shell cache),
whose in-tree comment asserted "is_open only READS the circuit" while
sitting directly on the bug it named.
LIVE PROOF of the shape, 2026-08-21 11:59Z: elohim.host / -> 503 in
20.751s carrying x-ssr-skipped:shell-fetch-failed and
x-elohim-hop-serve-ms:10004.681 — the 10s budget expiring to the
millisecond, twice sequentially — while its OWN
/db/content/elohim-host-landing -> 200 in 0.114s and its /health still
read healthy:true,status:"online". doorway-alpha in the same minute sat
circuit:closed/errorStreak:2 (the blind window) with /db -> 503 after
12.034s. BILATERAL, never an adam defect: breaker_open_total climbed
A 56->303 and B 47->295 over 13h, and doorway_upstream_backpressure_
honored_total reads 0 on BOTH — the opens are never-answered hops, not
storage backpressure.
CURES (host-green, NOT deployed): the extraction flight re-enters instead
of falling through and only its owner guards it
(MAX_EXTRACTION_COALESCE_ROUNDS); evict_app deletes the prefix
unconditionally so a failed put cannot leave an orphan the next put
inherits; both `/`-path planners use a new non-mutating would_shed() and
the raw gate is #[cfg(test)] so production cannot reach it a fourth time;
both classify violations defer to the one classifier; snapshot() stops
reporting a HalfOpen circuit as skipped:false; /health gains a `serving`
block and demotes on BOTH the shedding and the slow regime (errorStreak>0
— the blind window above is exactly where a shedding-only signal lies),
and /health/serving carries the status code /health cannot, because
/health on :8080 is simultaneously the startup, readiness AND liveness
probe and must never flip. Gates: `just gate doorway` exit 0,
`just gate elohim-storage` exit 0, elohim-cache-core 47/47, 6 new breaker
tests + 5 new health tests + 2 new cache tests, all red-first.
OPEN: fleet verification via [build:edge] [edge:validate-only], and
whether the endpoint-keyed breaker should be route-class aware — backlog
doorway-breaker-trial-theft-fleet-verification.md.
DELTA 2026-08-18c (batch closed on dev — pre-push review pass, 57688ae4a):
both cures below were hardened before landing, red-first — warm_shell keys
the stocking on the head declared when the fetch was DECIDED (a projection
advance mid-fetch can no longer relabel old-era bytes AtHead), and
custody_rotation's successor check is state-aware with an idempotent author
(a create-succeeded/activate-failed row now converges instead of stranding
the pledge into invisibility). Gates green; a2o scenarios remain the flip
authority, unchanged.
DELTA 2026-08-18 (ch07+failover session): Task 3.4 warm-boot shell cache
LANDED desk-proven (doorway gate green: 943 lib/bin tests, clippy, fmt) —
/ is cache-first (ServeWarm / UpgradeThenWarm-2s / Fetch / instant-Shed
decision matrix in render/warm_shell.rs, x-elohim-bundle:last-reconciled
marker, boot hydration from app_file_cache, mongo-less degrade = today's
path, 9 new tests incl x-ssr-fetches-free warm serve). Kills the live
defect measured this session: / stalled 10s (A, 200 shell-fetch-failed)
/ 20s (B, 503) on EVERY request through catch-up — the suite's
HeadersTimeoutError. Local-mesh proof: both local doorways shed in 1-4ms
(was 10-20s class) and the @concern suite runs 0.18s with the
classification scenario green. Also: E2E_DOORWAY_BETA/_B/_STORAGE_URL_B
now reach the cucumber env (reconcile-inventory's beta-leg red was env
plumbing); Task 3.2 apex-build client fallbacks configured
(environment.prod.ts). Task 3.1 apex multi-A attempted and REVERTED per
the WS3 revision — new hard evidence in the plan: relay-addr-beacon's
shared-record lane is single-slot (clap last-value-wins), so apex
multi-A needs a beacon change or sacrifices doorways.elohim.host;
operator menu item 2 now carries that constraint. Sibling saga ch07
(custody-witnessed): rotation cure landed (elohim-storage
services/custody_rotation.rs, gate green 2772 tests) and PROVEN on the
local 3-peer mesh end-to-end through public surfaces: stale pledge ->
rotation tick 150s -> notarized+ACTIVATED successor (origin:rotation,
dht-anchored) -> predecessor superseded -> self-held evidence ->
elohim_custody_class_count{class="stocked"} 0->1. Both cures reach alpha
on the next edge deploy; the a2o scenarios remain the flip authority.
Prior evidence below.

RED WRITTEN 2026-07-31 (doorway-federation-failover sprint session,
live-probed in-session): elohim.host / -> 503 catching-up shed (adam
post-deploy arc-convergence window) while doorway-alpha / -> 200. The
pair floor and honest-classification scenarios hold; the apex-name
scenario is the born-red — the name is pinned to doorway-B, so B's
hours-long catch-up sheds the apex while a healthy sibling holds the
identical converged content. Cure surface named by the sprint plan:
apex multi-A + client fallback (WS3), warm-boot shell cache
(x-ssr-fetches:0 invariant), and the operator ceiling on adam's
hosted-agent provisioning.

2026-09-03 (shift land-rung5-batch) — evidence AGAINST green, status NOT flipped here (the
declared check did not run: Dataplane Validation on edge #1422 lists doorway-failover as "RAN AND
SKIPPED (apparatus)"). Both alpha doorways failed on every container restart from the first
post-split storage roll (2026-08-31) until edge #1422 (2026-09-03 03:44Z): the conductor split moved
the admin/app socat bridge into `<prefix>-conductor-0`, the doorway manifests kept
`CONDUCTOR_ADMIN_URL` on the storage Service, and every doorway start died minting its app auth
token (probe kill after one 130 s connect timeout). doorway-B (`elohim.host`) had no Ready pod;
doorway-A survived on a single pre-split pod. The register saw nothing because no lane restarts a
doorway and reads the rollout. Fix 2d356dbc2; incident atom
genesis/data/timeline/backlog/conductor-split-left-doorway-admin-url-on-the-storage-service.md.
Bind this habit's check to a rollout read (`kube_deployment_status_replicas_unavailable == 0` for
both doorway Deployments after each edge roll) before calling it green again.
DELTA 2026-09-06 03:4xZ (fleet look, RED preserved; two findings for the served-shell clause). Both apex names
(`elohim.host`, `doorway-alpha.elohim.host`) serve `/` 200 with an intact landing, no pageErrors — but every
render 404s `/version.json` (the footer's build stamp, generated by CI into the browser dir and staged with the
whole bundle by `scripts/ci/stage-spa-blob.sh`) and `/wasm/elohim-cache-core/elohim_cache_core.js` (optional:
alpha's environment disables preferWasm, the resolver falls back to TS). The wasm miss is tolerated by design; the
`version.json` miss is not — a shell whose own build stamp is absent from the bundle it names is the
2026-09-04 stale-shell shape one notch down (assets 200, stamp 404). The doorway routes neither path as a
service path (EPR dispatch proxies both to the bundle), so the gap is in what the staged bundle HOLDS, not in
routing: the app job's `Jenkinsfile` tolerates a missing wasm artifact and does not require `version.json`
before publishing the browser bundle. Cure candidate (not landed): require the stamp before publish, and make
`served-shell-boots.feature` assert the served `/version.json` matches the declared head's stamp — that turns
"stale but 200" into a runner-observable red. Second finding: the B doorway's storage peer (adam) is wedged
(`reanchorDeadRemaining 9`, `stuckSweeps 55`) with `dhtAnchorState unverified` on the landing row where A says
`live` — filed under dataplane-convergence (primary), noted here because "whichever serves resolves the same
declared head" currently holds only for the blob hash, not for the anchor state or the SSR bundle hash.

DELTA 2026-09-09 (RED preserved): native doorway-continuity chain records full-feature household recovery 10/10 (run 20260909-codex-failover-prepared), strengthened canonical fixture 2/2 with 22 steps (20260909-codex-fixture-ready-strengthened), and independently reviewed beacon membership gate 47/47 plus narrow seeder gate 601 passed/9 skipped; actual apex transition run 20260909-codex-apex-frontier remains NOT MEASURED (2 undefined scenarios, 14 undefined/2 skipped steps, EXIT 1), so neither apex failover nor WAN balancing is claimed. Kubernetes remains a bootstrap test bench for native EPR-governed selection/recovery.

DELTA 2026-09-09 (runtime endpoint station GREEN; whole habit RED preserved): real production package browser sha256-12013a92cc5560abf6dccff195aa7566ba8258034fdccad255634021377978dc serves both household doorways; runtime-endpoint-delivery run 20260909-codex-runtime-endpoint-delivery passes 2 scenarios/14 steps, final fixture run 20260909-codex-runtime-endpoint-fixture-delivery passes 2/22, app gate 4581 tests and a2o gates pass. Browser manifesto reads and startup health stay at each serving origin; desktop/mobile have zero network failures or uncaught page errors, with a handled local cache 404 still visible. Landing references outside the narrow fixture and config.json also return local404. Five services healthy; no sibling/WAN failover or fleet delivery claimed. Native commitment plans__2026-09-09-doorway-continuity-proof-chain#5 carries the report and review.

DELTA 2026-09-09 (RED preserved): current-source household receipt renewal exposed a false Iroh-capability refusal: `strings | grep -q` can SIGPIPE under inherited pipefail. The launcher now drains the same exact marker check; peer-transport regression passes 11 checks, including a large trailing string payload, absent marker and unreadable binary. Household full-feature renewal and deployed delivery remain pending.

DELTA 2026-09-09 (RED preserved): the preserved-household restart then selected its own `awk` processes from command-line text and stopped before launch. Restart selection now reuses executable plus household config/cwd ownership checks. Process-selection regression passes 8 checks (owned, foreign household, decoy and empty selection), shutdown regression passes 12 and peer-transport regression passes 11; full-feature household renewal and fleet delivery remain pending. Namespace mismatch before shared writes remains a separate open station.

DELTA 2026-09-09 (RED preserved): isolated resume4 proved a cancellation convergence defect: own-conductor reads restore the original commitment anchor and shared projection omits lifecycle fields, so peers report zero anchor divergence with different active/cancelled state. Continuity chain station 7 names the atomic repair and multi-peer proof; no revocation-convergence claim is made. The existing serving suite uses per-peer canonical fixture cleanup and retains its full unclaimed-root and five-scenario gates.

DELTA 2026-09-09 (anonymous browser sibling station GREEN; whole habit RED preserved): run 20260909-codex-sibling-reader-delivery passes 1 scenario/11 steps against the real packaged app: owned doorway A SIGSTOP, discovered B serves actual manifesto metadata and byte-identical blob without credentials in one running app document, then A serves the landing after recovery and the read retry interval; teardown restores health. Final app gate 4596 tests/225 files, a2o gate, Gherkin204 and projection1851 checks pass; healthy endpoint regression2/14 and fixture2/22 pass, five services200, desktop/mobile look0 HTTP/network/page errors. Browser package sha256-5663662a0f1e5cdc3acbbae8e913d127d5e8b1d1d7703531ef079cd7f8fd95da; native continuity commitment#6 and its report preserve source/run evidence and earlier red attempts. This proves anonymous running-browser content continuity, not cold bootstrap, session migration, WAN/apex routing or fleet delivery.

DELTA 2026-09-09 (RED preserved): fresh edge1449 deployed f1191564 and sustained quiescence, but alpha's immutable declared main script was200 while its root asset was404 through a stale slug mapping. GET assets now reuse the proven warm-shell head, preserving held-old and unbound fallback; independent review GO, focused11/serverHTTP152 and doorway gate1209+2 pass. Current-revision household renewal and fresh public delivery remain pending; HEAD and cross-request hot-swap binding are not claimed.

DELTA 2026-09-09 (asset station GREEN; whole habit RED preserved): edge #1450 SUCCESS deployed 9b55e82f to both public doorways; mandatory shell/SSR and unchanged browser scenarios pass (3 scenarios / 24 steps), with declared/root asset bytes and version matching. Fresh private full5 resume10 passes 5 scenarios / 102 steps with cleanup and incumbent/source guards at EXIT=0. Native station 8 records DONE_WITH_CONCERNS: all eight inspected light/dark captures render complete pages but pnpm look exits 1 for real federation discovery timeouts and third-party aborts; newer app publication, cross-request hot-swap binding and actual apex/WAN transition remain unproved.

DELTA 2026-09-09 (delivery gate repair; RED preserved): the documentation-only public-proof push exposed gate-runner --names emitting human status prose as project IDs. The CLI now emits an empty machine list for an empty selection; human output, manifest selection, executed commands and exit codes are unchanged. The real subprocess regression failed before the fix and passes afterward; just gate orchestrator EXIT=0, including 148 suite tests. No serving receipt, fixture, assertion or shift measurement was weakened.

DELTA 2026-09-11 (fleet confirm of the sprint batch; RED preserved): edge #1452 SUCCESS deployed f7cfa7e6 (the doorway-federation batch 753b766a1..2c338124a plus fixes 86f2b32e8/3c690ec54) to all 7 storage + 7 conductor statefulsets and both doorways; `fleet-quiesce` PASS sustained 362 s (A/B caughtUp, A quiesced actionable=0, converged, both doorways 200 on elohim-host-landing). Dataplane Validation 112 scenarios: 7 passed / 3 failed / 3 pending (baseline edge #1450: 6 / 4 / 3) — `[FAIL] doorway-failover: passed=1 failed=2 pending=20`, identical to #1450: both reds are `served-shell-boots.feature:76/77` "opens the page … in a browser" failing on `browserType.launch: Executable doesn't exist at /root/.cache/ms-playwright/chromium_headless_shell-1217` — the validation container has no chromium (env, pre-existing; cure = the `playwright install --with-deps chromium` line verify-served-shell.sh already runs, added to run-dataplane-validation.sh this shift, unproven until the next edge run). Steady-state clauses are confirmed on the fleet only to the extent the plain-cucumber stations reach; the browser stations stay unmeasured on CI; apex-transition + WAN ingress unchanged. Serving receipt on the household mesh remains the current-source proof (run 20260911T081627Z-62fb8fb6). Fleet context: adam's conductor was CPU-pegged (4/4 cores) for ~24 h before the roll and the apex doorway shed writes (app #1704/#1705 red on env) — scale-risk row 8.
DELTA 2026-09-11 (browser-station apparatus landed; household re-measure; RED preserved): the chromium install for the edge Dataplane Validation stage (scripts/ci/run-dataplane-validation.sh) reached origin/dev as 82610e30f tagged `[edge:validate-only]`, so its edge run measures the served-shell-boots browser clause on the live fleet WITHOUT a build or deploy — the fleet build number is appended when that run is terminal. Household lane 20260911T223432Z-466536cb on storage sut sha256:371d348555e6d37f: doorway-failover.feature 4/10, epr-app-deliverability 3/5, fixture-readiness 0/2, runtime-endpoint 0/2, sibling-reader 0/1. Causes read from the report, not the count: (a) the two process-control chapters were REFUSED by the recovery guard (`storage jessica recovery executable is missing or differs from the running bytes` — the pool binary was rebuilt after the mesh launched), apparatus; (b) `/db/content/elohim-host-landing` and `manifesto` 404 on both local doorways and `/` 503 — the prologue fixture is not present on this mesh (fixture-readiness is the declared prerequisite and it red first), so the pair-floor, declared-head and sibling-reader reds are prerequisite failures, not failover verdicts; (c) `warmup.completed=true but servedBundleHeads is []` on alpha-A reproduces the 2026-08-21 false-green class and IS a real red. Apex-transition remains the un-discharged authority.
DELTA 2026-09-11 (fleet validate-only measure; RED preserved): edge #1454 (Jenkinsfile from 617d85866, `[edge:validate-only]` — build/deploy skipped, Dataplane Validation only, no fleet roll) — fleet-quiesce PASS sustained 362 s (A/B caughtUp, A quiesced actionable=0, A-converged flipped 0→1 inside the window); the chromium install landed (Chrome for Testing 147 / headless shell v1217 downloaded before cucumber), and the served-shell browser stations no longer red on `Executable doesn't exist`: doorway-failover passed=3 failed=0 (was 7 passed / 3 failed at #1452, two of the three the chromium class), federation-deploy 2/0, reconcile-inventory 3/0; 99 of 112 scenarios HELD @act:i by LAYERS design. The build result is FAILURE only because validate-only is strict and blob-durability's `coverageShortfall is ABSENT` scenario (pre-existing, #1452 and #1454) is the one measured red on the fleet. The apex-transition born-red is unchanged, so the habit stays RED.
