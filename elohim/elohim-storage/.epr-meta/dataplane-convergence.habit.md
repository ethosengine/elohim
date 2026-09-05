---
epr-habit-version: 1
id: dataplane-convergence
invariant: >
  Serving-critical content state converges peer-to-peer; no per-host
  imperative write is load-bearing. A peer that missed a deploy heals.
status: red
active: true
checks:
  - "a2o @concern:federation-deploy (genesis/a2o/features/dataplane/federation-deploy.feature — deploy uniformity, Act II; and genesis/a2o/features/dataplane/federation-version-convergence.feature — version convergence, Act I, runnable on the household mesh)"
  - "a2o @concern:content-sync, @concern:blob-replication, @concern:peer-mesh, @concern:epr-projection-fallback, @concern:reconcile-inventory (edge Dataplane Validation stage)"
  - "cargo test --test sync_libp2p_convergence (elohim/elohim-storage)"
  - "a2o @concern:transport-parity (genesis/a2o/features/dataplane/transport-comparison-matrix.feature; just mesh matrix) — identical fresh-document contract under libp2p, iroh, and dual; successful samples alone contribute timing percentiles"
  - ".claude/scripts/latency-scoreboard.py --local (and --deployed) — bounded convergence, not merely eventual: a CONJUNCTION of speed AND this habit's trust invariant, never speed alone. A fast stale head is worse than a slow correct one, so this check can only ever narrow the habit, never carry it."
  - "a2o @concern:trust-priced-sync (genesis/a2o/features/dataplane/trust-priced-sync.feature) — RED-FIRST 2026-09-01: a household peer prices its household edges `trusted` and no edge `public` on a pure-household mesh; the trust handshake is a stub that prices every edge `public`, so this check is born red and runnable on existing steps"
guard: >
  Regression risk = the fleet-safety invariants in sync/projector.rs
  (empty-never-projects, reconcile-offers/events-assert, broadcast-tier reach
  gate, amber set-to-set) — all pinned by lib tests.
  Cross-version note (2026-08-06): the CRDT plane is wire-compatible across
  the 0.5.12/0.10.0 span — save/load and save_after/load_incremental bytes
  cross-load in BOTH directions (empirically proven, both versions linked in
  one binary), so a mixed-version fleet during a rolling deploy converges.
  Upstream documents no backward-compat guarantee; that claim is ours and
  rests on the experiment, so re-run it before the next automerge bump.
  2026-08-07 iroh-lane lesson: a serving facade masked a fleet-wide
  storage-boot wedge + /db outage for ~14h (relay_url client skew wedged
  ensure_happ_installed BEFORE the HTTP bind; conductor websockets kept
  facade routes green) — cured fix-forward same-day, bootstrap publish +
  admin seam live-verified on the iroh fleet (all 7 publishing, measure
  0→8); probes that can 404-as-zero are part of the facade (seam-smoke now
  fail-closed). Incident: backlog/iroh-lane-bootstrap-publish-dark.
  2026-08-07 head-fork cure + honest-red localization: the 20h A/B declared-head
  fork (born when app #1657's declare fan-out hit B mid boot-loop; adam never
  saw the hash in 1.29M log lines) cured live — authorHeadOnce re-author +
  DHT heal converged both doorways on uhCkkDYpMVdqJ3ND; bootstrap store
  split-brain healed (both mongo, fail-loud guard c90d1b174); converged=0 is
  HONEST — pin is the content gap plateau (pending ~2.9k vs 120s leg budget,
  backlog/content-gap-limit-cycle-blocks-convergence), NOT MissLedger
  exhaustion (name-collision trap disproved twice, tests pin it in
  reconcile_rails.rs). Saga recorded state still 8/11 — measurement stays
  gated behind the plateau by design.
  2026-08-07 gate decision (integrator): fleet-quiesce-gate re-pointed from
  perfect (converged==1) to QUIESCED (caughtUp && blocked_by
  divergent_actionable==0 && unmeasured==0, sustained; fail-closed on absent
  series; 5-case fixture proof local) — measurement unblocks next deploy;
  converged stays honest telemetry; plateau drains under
  backlog/content-gap-limit-cycle-blocks-convergence (F-B fan-out + peer-probe
  source widening = next sprint objective).
  2026-08-08 edge #1320 evidence: lane cure PROVEN live (per-commit
  elohim-storage-iroh:1.0.0-dev-88a4d622 on all 7; blocked_by publishing
  from first sweep) — gate honestly refused on the ONE named term:
  divergent_actionable=1 sustained (matthew content actionable 1-3; rea 0,
  collectives 0). Embargo = ~3 rows. missing_deferred live ~146/h;
  refused_declared collapsed ~50x. Next: soak → re-probe → if persistent,
  Stage-2 adjudication on the sampled ids; validate-only fires measurement
  once actionable holds 0.
  2026-08-07 edge #1319 evidence: gate fail-closed EXPOSED a lane freeze —
  alpha storage pinned to the one-time elohim-storage-iroh:hc-elohim-0.6.3-iroh
  artifact (resolveStorageImage), so every storage dev-merge since the 08-05
  iroh flip was inert on alpha (blocked_by absent, actionable=None, honest
  DID-NOT-MEASURE). Cure = per-commit iroh-lane storage build + alpha repoint
  + 900s→2700s validate deadline; frozen tag kept as rollback.
  2026-08-07 cure committed (NOT yet build-proven): storage-iroh/Dockerfile +
  scripts/ci/push-storage-iroh.sh publish elohim-storage-iroh:${STORAGE_TAG}
  per commit; resolveStorageImage repointed; deadline flat 2700 on both
  dataplane paths. Awaiting an edge build number — the flip to measured stays
  red until a deploy shows blocked_by present on alpha.
retire-when: >
  never: convergence is the protocol's constitutive promise. A p2p substrate that stops
  converging is not a degraded version of this system, it is a different one — so this is
  watched permanently rather than until a milestone.
---
DELTA 2026-09-02 (dataplane pain-points sprint, wave 1; NO status flip): LIVE PROBE
2026-09-02 — federation-deploy scenario 2 conditions HOLD on both doorways (GET / 200;
blobHash non-null: elohim.host sha256-f0f0e637…, doorway-alpha sha256-04ae4310…, same
dhtAnchor uhCkkvfsT…) — but the two serve DIFFERENT versions (divergence, scenario 4
@wip) and both read caughtUp=false converged=false (divergentAnchor 2131 / 1011). The
fleet lane had never measured this concern (0/0/2 pending since 08-22: the feature was
@act:i and the fleet lane drops Act I) — re-acted to @act:ii (e6dd94c4a) so the next
Dataplane Validation measures it; D5 atom superseded-in-code (bytes + seed authority +
declare fan-out all landed). Quiesce leg bounded (5fb684ef6): runDataplaneValidation()
top-level def, own 55-min timeout, warn-only on deploy, strict on [edge:validate-only];
COORDSWAP records connect-refused peers as deferred (rc 4) and prints DEFERRED —
fleet-unproven until the next edge build shows UNSTABLE-not-ABORTED. Device peer
(50f43d706): join-alpha preflight checklist, CONDUCTOR_RELEASE_CHANNELS →
ELOHIM_RELEASE_CHANNELS scoped to the storage launch, device-peer-receipt.ts (stations
2-3 SKIP honestly) — unmeasured against alpha until the operator runs it. Plan:
2026-09-02-dataplane-pain-points-sprint-plan.md; wave 2 (station 1 handshake,
--sync-coordinators-once) waits on the storage slot. Stays red.
---
DELTA 2026-09-01 (trust-priced sync edge design; NO status flip): the design gate
for ratchet move M3 landed as spec 2026-09-01-trust-priced-sync-edge-design.md
(refines trust-as-efficiency-signal; composes the trust/ pricer seam, never a second
pricer). Grounded: the trust handshake is a stub end to end (sender presents empty
credential vectors, both receiver arms and the iroh TrustService hard-code
reach_ceiling=public; verify_trust_context has zero callers), so every edge on every
mesh is asserted verified-public — a C4 violation, now a named red: new check
@concern:trust-priced-sync scenario 1 fails on existing steps (trusted absent, public
~48 on the household mesh). Every catch-up knob is global or failure-keyed (window 32,
timeout 30 s, backoff 60 s→15 min, round-robin providers). Design: one derived
EdgeClass per edge (the existing reach-ceiling vocabulary + commitments), one pure
EdgeBudget predicate keying window/patience/backoff/admission/reverify with liveness
and authority floors (trust widens, never gates; no budget moves a head). First slice
named, not coded: the honest handshake (station 1). Stays red.
---
DELTA 2026-09-01 (iroh observed-version receipt; NO status flip): `/p2p/status` now projects the live peer book as nullable `irohPeers`, and `version-matrix.ts --observed` read a green three-peer dual household matrix: every observer reported both remote NodeIds as `elohim-storage/0.1.0`, with no divergent column. Fleet confirmation remains the integrator's post-roll watch, so this habit stays red.
---
DELTA 2026-09-01 (iroh observed-version fleet-partial; NO status flip): edge #1410 completed UNSTABLE after deploying storage build `0abe0b344` (6/7 storage peers Ready; 0/2 doorways Ready), yet the two public storage projections (matthew and adam) each reported all six remote alpha NodeIds as `elohim-storage/0.1.0+0abe0b3` — 12 populated cells, agreement on the five independently cross-observed third-party NodeIds, and each observer seeing the other at the same build. All storage Services are cluster-internal; the five peers without public doorway projections cannot be queried as observers from this workspace, so the required seven-observer 42-cell matrix is still unmeasured and this habit stays red.
---
DELTA 2026-09-01 (late-joiner fixture regime, NO status flip): an additive
`hc-mesh.sh join-peer` verb plus exact-identity receipt now stage a fourth peer
after the ordinary three-peer mesh is warm. Three clean-teardown runs passed:
all unchanged incumbents learned the signed joiner NodeId in 2.1–20.7 s while
their recurring board-watch counters advanced. This closes the household
fixture residue only; organic alpha confirmation remains owed.
---
DELTA 2026-09-01 (iroh-quinn GSO crash cure, NO status flip): vendored the
published iroh-quinn-proto 0.13.0 and backported Quinn #2167 / 434c3586's
one-line tail-loss-probe bound; `just test-iroh` and `just gate
elohim-storage` are green. Fleet-shaped ~891-exchange burst/disconnect T2
receipt remains owed, so the task and this habit stay red/wip.
---
DELTA 2026-08-29c (FLEET CONFIRM of the 2026-08-29 batch; NO status flip —
the habit's own fleet leg is still red on federation-deploy). Push
38eb75677..71f310ce6 (37 commits) → orchestrator #1742: edge #1389 built,
pushed and deployed elohim-storage-iroh:1.0.0-dev-71f310ce to 7/7 alpha
peers + 2/2 doorways Ready (dual). Dataplane Validation read A-caughtUp=True
AND B-caughtUp=True for its whole 45-min gate — the first run both storages
read caught-up — but A-quiesced never held (actionable never drained to
tol 2) so the gate DID-NOT-MEASURE (post-deploy churn). No
elohim_transport_route_total / acquisition_dispatch_total series yet on
the fleet: nothing was routed in the window (genesis #1518's upload, the
first routed shard push, failed upstream on the doorway rollout collision;
retrigger d0b3efe30 pending). Local 3.4k-corpus delta: per-op iroh wins
(bulk ~7x, small ~25x); recovery/quiesce wall-clock cadence-bound. Portal
lane: Act II twins for auth-discovery + doorway-portal-login landed in the
genesis E2E legs (proven 4/4 + 2/2 on the mesh). Ledger rows filed:
content-id-with-spaces-unreachable-by-id, transport-route-metrics-
pretouch-zero, elohim-app-gate-lint-debt-blocks-push,
doorway-schema-contract-test-runs-nowhere, seed-relationship-type-step.
DELTA 2026-08-29b (M4 transport self-awareness; NO status flip). Landed
spec 2026-08-24 §3.1: p2p::transport_paths (PathObservation ring keyed by
the cross-plane peer label; select_path pure predicate, registry row, C3/C4
contract tests), elohim_transport_route_total{transport,op_class,reason} +
elohim_transport_path_rtt_ms, /p2p/status.transportPaths. Small ops race
both planes and sample them; Bulk selects on evidence — pull leg
(plan_acquisition_targets_selected) and the write-time custody push
(push_shard_routed, new iroh leg + one fallback). Measured, dual household
mesh: jessica->matthew Small iroh 15 ms vs libp2p 816 ms (degraded); probe
content's shards pushed transport=iroh to both holders. Warm recovery
on 62/59/61 s, off 61/60/58 s, on-v2 61/57/60 s: inside noise on a LAN
(spec-predicted); ELOHIM_TRANSPORT_SELECTION=off is the rollback. Gap
filed: a warm-restarted peer drains its pulls before its iroh book warms.
DELTA 2026-08-29a (transport-parity check; NO status flip). Pure-iroh
PARITY measured: homo-iroh warm jessica<-matthew 61 / 60 / 72 s and cold
113 s, all PASS P0-P4 (cut reconcile-peers-1, durable rows 01:16-01:21Z);
homo-libp2p warm 58 s the same hour. Cause of the last red: the
projection-reconcile discovery arm + anchor heal from the own conductor
were P2PHandle-gated, so pure iroh had no heal leg at all (P1 red 469 s on
a row libp2p healed in 58 s; the 2026-08-28g 258 s pass stood on P1 luck).
Cure: p2p::reconcile_peers::ReconcilePeers (libp2p + IrohReconcilePeers
over the peer book + view-fed ALPN); every arm takes &dyn ReconcilePeers.
Reverse heal is idempotent and names from/to (elohim_storage::sync_heal).
Residual filed: content-doc-blobhash-representation-drift (plane-neutral).
Focused tests 65/0, clippy green; full gate in flight at write time.
DELTA 2026-08-28g (transport-parity check; NO status flip). Pure-iroh
parity landed (b9c9ad477): pull core, heal-on-read without a libp2p
handle, shard responder alias fallback, composite manifest pivot. homo-iroh
warm jessica<-matthew: PASS P0-P4 in 258 s (durable record 22:54:08Z),
from P0-only 901 s FAIL the same evening. Lane P rung P3 locked for the
warm shape; cold + N=3 outstanding. Full storage lib suite 3166/0.
DELTA 2026-08-28f (transport-parity check; NO status flip). Row 13 landed:
the pull leg dispatches across libp2p + the iroh book (iroh preferred for a
dual peer), GetContent + quilt-draw blob pull over the iroh shard ALPN. Dual
household mesh, warm jessica<-matthew: 58 s PASS P0-P4,
acquisition_dispatch{iroh}=3 {libp2p}=8, iroh_blob_fetches{ok}=3 — first bulk
bytes on iroh. homo-iroh stays red past P0 (no P2PNode -> no projection loop;
backlog iroh-only-content-projection-loop-gap). Mesh now runs dual by default.
DELTA 2026-08-28e (M0 pawls, NO status flip). Lane R rung R3: the recovery
timeline is durable (genesis/a2o/reports/recovery/, /tmp symlink; first record
warm jessica<-matthew 62 s libp2p). Lane D rung D2: pre-push T2-receipt leg
(warn-only) fires on dataplane paths without a fresh household report. Lane P
rung P5: sovereign-peer-join sc1 GREEN x4 on the fork iroh pair (a stock tx5
join is listed-but-unconnected; hc-start.sh now refuses it); sc5 RED in both
arc modes because every live alpha agent-info advertises storageArc: null —
backlog sovereign-peer-network-read-no-authorities, a conductor-fork/kitsune2
ceiling. Mesh hygiene found: workdir elohim.happ 12 days behind the storage
binary -> head-record WasmError Deserialize (MESH_HAPP_PATH + probe label).
DELTA 2026-08-28d (ACTIVE; NO status flip). Swapped into the WIP fence in
place of doorway-failover (green, run-identified 2026-08-25b). Campaign
spec ratchet-to-delivery-dataplane-sdk-lanes lays five ratchet lanes
(R/P/F/D/S, 48 rungs, 9 locked / 39 open) with this habit's fleet red as
R7 and its local reproduction as R4; first move M0 (pawls: mesh restart
arms, mesh seed profile, T2 receipt banner, durable recovery timeline,
sovereign-peer sc1+5), then M1 names the plateau (syncVerdicts, R5).
DELTA 2026-08-28c (measurement hygiene cure fleet-PROVEN; NO status flip): genesis #1515 and #1516 both measured 2 failed / 274 (from 6) once the pipeline stopped bouncing doorway-alpha before its own E2E — ad90163c7 replaced the pod-delete with a two-tick wait + `x-epr-router: dispatched` proof (the router has self-healed every 30 s since 379668123); #1516's log: "EprRouter populated without a restart … zero pod churn". The 6-vs-2 swing on identical code (#1514 vs #1513/#1515) was fleet churn: the shem edgenode containers exit 139 several times an hour (backlog runtime-shem-edgenode-container-exit-139-chronic) and docs-only pushes had been dispatching seed storms (genesis manifest `genesis/data/**` narrowed, 4f4785e03). The habit's own fleet red is unchanged and honest: inventory-convergence `alpha-A p2p.caughtUp` false on every run. Stays red.

---
DELTA 2026-08-28b (FLEET CONFIRM of the overnight batch; NO status flip): edge #1388 (orchestrator #1736, from the 10:53Z push of 6e4fa4389) deployed `elohim-storage-iroh:1.0.0-dev-6e4fa438` to all 7 pods and doorway 6e4fa438 to both doorways — the `peer_class` label, the relationship-vocabulary fix and the admin-socket closure are all live. First fleet read of the sync edge by trust class: `elohim_sync_request_outcomes_total` {public,ok} 1,959 · {public,timeout} 38 (1.9 %) · {unverified,ok} 558 · {unverified,timeout} 13 (2.3 %) — no edge above `public`, exactly as the handshake-stub finding predicts; the trust-gradient question is now askable on the fleet and today answers "no trusted edges exist". Quiesce gate on #1388: no_measure — 45 polls / 44.6 min A-not-quiesced (actionable 1, count 14) after the rolling restart, so no Dataplane report; seam-smoke all OK. genesis #1512 (the shift's measure) UNSTABLE at 6 failed: the #1511 ECONNREFUSED / catching-up set cleared and what remains is three doorway-alpha-unavailability fingerprints (/health 503 <html>, DoorwaySessionError, shem-by-design) — genesis restarts doorway-alpha itself (`seedProjectionsStage`, 13:12Z) about two minutes before its E2E stage, so the pipeline measures the doorway it just bounced. Stays red.

---
DELTA 2026-08-28a (overnight shift shakeout-landing-perf-trust-hybrid; NO status flip — the fleet leg stayed unmeasured because a Jenkins controller/cluster outage ~05:00–09:34Z aborted every in-flight build: orchestrator #1735, edge #1387 (died in the doorway image compile, Push:Skip, nothing deployed), holochain #1405): the W4 local-first measure ran on the household mesh (dual, stock holochain 0.6.0, mesh binaries at 0fdbbd285 then 6e4fa4389): full corpus seed 39 s → A=3,452 rows; cross-peer propagation first moved at ~45 s (one sync round), B ≈7 rows/s, C ≈4.4 rows/s; B plateau 3,441 @ ~9 min, C 3,449 @ ~12 min, both flat >4 min (B: divergent{content} 216 / gaps 127 / actionable 21; C: 3 / 33 / 0; reach_scoped 0 on both — genuine divergence, not scoping); A's sustained quiesce NOT reached in 20 min (sweep 21 re-found actionable 65; 65 heads oscillate) — verdict FAIL(deadline), the same converged=0 plateau class this habit carries on the fleet, now reproduced locally with a 3,452-row corpus. Rate to plateau ≈280 rows/min vs the 2026-08-17 definitive 2,190 rows/min; io_baseline 96.6 MB/s then 545 MB/s across the two runs — the environment moved 5× inside one hour, so no regression claim is made (a contended measure is a discarded measure). Two instruments born: the sync edge is now observable by trust class (`elohim_sync_request_outcomes_total{peer_class}`, 0fdbbd285) and its first read says every household edge prices `public` — the ambient trust handshake is a stub (empty CID lists, libp2p peer id as agent key, verifier has zero callers), so the gradient has no live input yet (backlog sync-edge-susan-timeouts-per-edge-observability); and the relationship table is per-host — 8,920 relationships on A after the vocabulary fix, 0 on B and C after content converged (rides no sync plane; captured). Household lane on the fixed binaries: content-sync 4/4, saga 11/11. Local seed had been dropping its entire relationship graph on `relationship_type 'EXTENDS'` (manifest/storage/authored vocabularies had drifted) — cured 6e4fa4389 with a manifest-⊆-accepted pin test. Sovereign-peer spike: a workspace conductor joined alpha's DHT (no partition, listed live by doorway-alpha within ~4 min) and its authored node was never served by the fleet (P1 gap — storage discovers ids only from other storages: backlog p1-dht-authored-content-not-projected). Stays red.

---
DELTA 2026-08-27c (FLEET CONFIRM of 2026-08-27 + 2026-08-27b; NO status flip — the habit's own fleet leg is still red on caughtUp): 13ed1721e landed on dev (push 971857934..9a2072771, 21:05Z) and edge #1386 (SUCCESS, 89 min, image 1.0.0-dev-9a207277) rolled all 7 statefulsets + both doorways ~23:26Z. Live probe 2026-08-28T00:12Z: anonymous WebSocket upgrade to /hc/admin → 401 "Authentication required. Use POST /hc/connect." on BOTH doorway-alpha and elohim.host — the admin plane named OPEN in the 2026-08-27 delta is closed on the fleet. The quiesce gate ADMITTED a measure on this build (A-QUIESCED; ledger row: 27 polls, time_to_verdict 1602 s, best_window 363 s, 0 resets, blocking A-not-quiesced 20 of 27) — the first admitted measure since #1383; #1384/#1385 were both no_measure. Dataplane Validation: 87 scenarios — 3 passed, 83 HELD @act:i (LAYERS design), 1 FAILED = inventory-convergence "seed-facing doorway peer catches its projection up under sustained gossip": alpha-A p2p.caughtUp=false ~30 min after the rolling restart (doorway /health at 00:11Z: caughtUp=false, converged=false, divergentAnchor A=855 B=457; Prometheus 23:56Z: matthew divergent_actionable 16→0 with converged_blocked_by{term=pending}=1). Same class as #1381's red: restart churn + hours of catch-up, gating an amber measure on a green fact. Two CI facts beside it: the pre-deploy MESH-QUIESCE stage is SKIPPED-NO-CONDUCTOR (the ci-builder ships no hc/holochain, so the local-mesh quiesce measure never runs in CI), and elohim-holochain #1403 failed in 85 s on a dead sccache Garage key (backlog sccache-garage-harden rec. 3) — invisible to the orchestrator because the DNA job is fire-and-forget. Stays red.

DELTA 2026-08-27b (NO status flip — local cure + live-local proof in BOTH postures, unmeasured on the fleet): the conductor admin plane named as OPEN in 2026-08-27 is now CLOSED, and the coupling that blocked it turned out to be on the CLIENT side, not the doorway's. Three independent gates were each keyed on `dev_mode` (`"true"` on all five deployed manifests): the /hc/admin route arm, the WS permission ladder, and the proxy message filter — so an anonymous internet caller reached an UNFILTERED Holochain admin interface (install_app / uninstall_app / revoke_agent_key). A SECOND credential-free arm in the ladder did not need dev_mode at all: `|| !api_validator.is_configured()` returned Ok(Public) to a caller presenting nothing whenever no API keys were configured — exactly the workspace/mesh shape — so closing only the dev_mode arm would have left the ladder open through it; they closed together. All three collapse to ONE predicate, native_local_first_operator: kernel-observed loopback peer AND pre-coordination stage AND no declared JWT_SECRET. That third conjunct is the one a deployment cannot fake — every deployed manifest populates JWT_SECRET from a secretKeyRef, and all five sit at the fail-closed Bootstrap stage, so STAGE ALONE WOULD NOT HAVE SAVED THEM. Proxy filtering is now unconditional (an unparseable frame is never forwarded: an operation we cannot name is one we cannot authorize). The onboarding coupling was resolved by deleting `!isCheEnvironment() &&` from useChaperone: every environment holding a session token now takes POST /hc/connect, whose handler requires only a valid JWT (no permission level) and provisions server-side via auto_provision — so the browser never needs Admin and never opens an admin socket. hc-start.sh now resolves a posture from what the box HAS (DOORWAY_AUTH, default auto): secure (mongod present → per-workspace JWT_SECRET + API_KEY_ADMIN persisted, HAPP_BUNDLE_PATH, no --dev-mode) or keyless (native local-first). The per-workspace secret is load-bearing: a keyless doorway signs with the publicly-known dev placeholder (JwtValidator::new_dev), so enabling the chaperone without one would have been security theatre. Two adjacent fail-fasts stopped keying on dev_mode in the same pass, because the secure posture is the first non-dev_mode doorway that legitimately declares neither: MongoDB (whose error text claimed "a MongoDB was declared" when none was) and NATS are now fatal iff DECLARED. PROVEN LIVE on the real binary, both postures, loopback+anonymous: keyless /hc/admin → 101 Switching Protocols (conductor operator), secure /hc/admin → 401 "Use POST /hc/connect.", secure + valid X-API-Key → 101, secure + wrong key → 401, /health 200 on both, POST /hc/connect without a token → 401 on both. Verified: 1128/1128 doorway tests (6 new), clippy -D warnings clean, fmt clean, seam census registry errors 0, decision point registered as websocket::extract_permission (kind: verdict-fn). Both Angular apps build (elohim-app incl. SSR, lamad) and 252 touched-spec tests pass. STILL OPEN and named rather than half-fixed: handle_app_upgrade (/hc/app/{port} and legacy /app/{port}) has NO doorway-side permission check at all — the only gate is the numeric port range — relying entirely on the conductor's own app-interface authentication. Flip still needs a fleet build number.

DELTA 2026-08-27 (NO status flip — local cure + live-local proof, unmeasured on the fleet): the auth arc's remaining DEV_MODE derivations closed, and one of them was a bigger hole than the seed gate it followed. `extract_http_permission` promoted EVERY anonymous caller on the open web to Authenticated (DEV_MODE is "true" on all five deployed manifests); it now derives from `network_stage < Coordinated && peer_is_loopback`, mirroring seed authority (1). The feared blast radius was MEASURED, not assumed: the crate holds exactly ONE Authenticated gate (the elohim-agent invocation proxy, whose own comment already says it should refuse anonymous traffic) — content/blob/apps/cache are registry StorageProxy paths with no permission gate, so browsing never consults this ladder. PROVEN LIVE on the real binary: anonymous POST /api/v1/elohim/invoke from a non-loopback address → 401, from 127.0.0.1 → 502 (gate passed, sidecar absent), /health 502 on both (still public). Separately, a MongoDB OUTAGE was an authentication downgrade: four auth paths branch on `dev_mode && mongo.is_none()` and auth_routes.rs:1626 accepted ANY credentials and minted Admin, while main.rs continued past a failed connection under dev_mode — so a Mongo blip turned the fleet into "any password logs in as Admin". A DECLARED-but-unreachable MongoDB is now fatal (mirroring the fail-loud bootstrap-store precedent beside it; declaration counts env var OR --mongodb-uri flag), and that branch's ceiling dropped to Authenticated. PROVEN LIVE: EXIT_CODE=1 both routes. NOT closed, and named rather than half-fixed: the conductor admin plane is the most severe finding of the arc — `if dev_mode { passthrough }` in proxy/{admin,pool,nats}.rs skips filter_message entirely, the WS ladder answers Ok(Public) instead of Err, /hc/admin's "disabled in production" arm is dead on the fleet, and the ingress is a catch-all `path: /`, so an anonymous internet client reaches install_app/uninstall_app/revoke_agent_key. It is NOT closed because the deployed app's ANONYMOUS visitors use that same socket to self-provision (connectViaAdminWs runs whenever no doorwayToken exists), so closing it alone breaks onboarding on a live site — it is coupled to migrating anonymous visitors onto the chaperone, which is the operator's sequencing. Likewise fixture_only_gate stayed open on purpose: a2o calls both its routes against the DEPLOYED doorway (agency.steps.ts:208, account-m5.steps.ts:248), so a loopback conjunct would pass unit tests and break E2E; it wants its own narrow fixture credential on the API_KEY_SEED pattern. Verified: `just gate doorway` green, clippy -D warnings clean, fmt clean, 1122/1122 lib tests (6 new), decision point registered (census 31 pts · 31 cited · 0 uncited). Flip still needs a fleet build number.

DELTA 2026-08-25e (NO status flip — local cure, awaiting edge deploy): `pnpm look` showed elohim.host's SSR shell naming main-M2EFD2IP.js while that mutable-slug asset request returned 404; the exact browser archive sha256-255fdcfa… was locally present, valid, and served the same asset by `/apps/{hash}/…` with HTTP 200. Doorway had resolved the slug to that exact hash for its cache key, then discarded the resolution on a MISS and asked storage through the moving slug, which had advanced to a different release. The miss leader and coalesced-failure paths now remain pinned to `/apps/{resolved-hash}/{file}`. Local proof: app-route tests 25/25; owning doorway gate format + clippy + 1119 runnable tests green (2 Mongo-only ignored). Fleet proof remains an edge deploy followed by a fresh `pnpm look https://elohim.host/` with no missing main chunk; the DHT trust state may still be amber while locally verified bytes serve, so this cure adds no wait-for-green gate.

DELTA 2026-08-25d (NO status flip — cure landed in-tree, unmeasured on
the fleet): the habit's own first check could not have gone green for a
reason upstream of convergence. From app #1672 every `seed elohim.host`
leg answered 403, so the landing bytes never reached doorway-B at all —
blobHash propagation was never the binding constraint on the apex. Cause
was a conflation, not a credential: require_seed_authority asked "is this
caller MY admin?" while ONE deploy pipeline drives several doorways whose
admin identities are deliberately distinct. It was masked for months by
alpha-b.yaml's DEV_MODE:"true", set expressly to bypass this gate and
carrying its own TODO ("Remove when doorway-B federation auth hardens");
62b658784 correctly closed that bypass and left no seed path. Cured by
deriving seed authority from the DECLARED stage — the doorway already
resolved ELOHIM_NETWORK_STAKES into AppState::network_stage sharing
seam_contracts::freshness::NetworkStage with storage's trust/stage.rs,
with ZERO auth consumers until now — plus a fleet seed authority
(API_KEY_SEED) scoped to the seed/admin-cache routes and never to the
permission ladder. Behaviour-preserving on the fleet (every manifest sets
DEV_MODE:"true" and none declares a stage, so Bootstrap < Coordinated
holds exactly where dev_mode held) and self-retiring at Coordinated, where
LAYERS.md puts Act II. Local proof only: cargo clippy -D warnings clean,
1116/1116 doorway lib tests, 7 new gate tests incl.
fleet_seed_key_never_grants_general_admin and
coordinated_stage_retires_both_pre_coordination_affordances. NOT MEASURED
on the fleet — needs an edge deploy (image + manifests land together; both
half-states are safe) then an App run whose four `seed elohim.host` legs go
green. Two findings filed, not fixed: the DECLARE_ONLY fan-out
(Jenkinsfile:342-357) is NOT seed-gated, so through #1672-#1673 doorway-B
accepted a canonical head for bytes whose PUT it had just refused — a
declare outrunning its bytes, which is this habit's shape exactly; and the
fleet's api-key-admin/jwt-secret fixtures are committed in plaintext and
applied verbatim to internet-facing hosts.

DELTA 2026-08-25b (FLEET CONFIRM of the 2026-08-23/24 arc; NO status
flip): 608a1ceff landed on dev as 286089679 (+47fb60f58 genesis CI fix);
edge #1380 SUCCESS deployed it to alpha; live: irohNodeId on doorway-A,
connectedPeers 6, syncDocuments 5356 (the count fix — this field read 0
on every prior build), elohim_iroh_sync_rounds_total 159 / gossip
received 9,149 / manifests 4,243 fleet-sum within 30 min of restart,
increase(sync_rounds[30m]) 13–28 on every one of the 7 pods (iroh driver
live everywhere; sync_changes_applied 0 = converged corpus, as the
2026-08-24 lens predicts). matthew's actionable remainder healed
13 -> 3 -> 2 -> 0 across sweeps 1–23 on this build (the predecessor's
restart stalled at 2 heals/20 min) and the validate-only recording run
edge #1381 was the first Dataplane Validation ADMITTED by the quiesce
gate (A-QUIESCED sustained 06:54:51Z) — but it measured almost nothing:
81/87 scenarios HELD because every content-sync/transport-parity/
heal-on-read/failover scenario is @act:i and the fleet lane drops
owned-substrate by LAYERS.md design. The fleet red that IS this habit's:
inventory-convergence "alpha-A p2p.caughtUp true" FAILED (caughtUp flaps
false while the remainder oscillates 0–10) — matthew conductor
saturation class, filed. The act-i legs of this habit (household matrix
libp2p/iroh/dual 3/3 each, content-sync 4/4, heal-on-read 2/2) are green
on the household lane; the act-ii/fleet leg is red on caughtUp; the
latency-scoreboard --deployed conjunction is unmeasured. Stays red.
CI note: the doorway loopback seed gate exposed a latent genesis
Jenkinsfile bug (readFile of a container-side /tmp token) — fixed
47fb60f58, genesis #1504 Upload Blob-Backed Content HTTP 200 with the
bearer.
DELTA 2026-08-25a (localdev push-verification of the iroh/dual plane; NO
status flip — flip authority is the fleet lane): the full tree re-verified
on the household mesh from the p2p+p2p-iroh binary (storage gate green
incl. doctests, doorway 1109/0, a2o lint + gherkin + unit 272/272). Matrix
run 20260825T014418Z-04235e21: libp2p 3/3 · iroh 3/3 · dual 3/3 (p50
56.6 / 58.6 / 56.7 s on the same-host contract; mixed per-peer is still
explicit absence). Lanes on the same binary: transport-dual-plane 1/1,
content-sync 4/4, heal-on-read 2/2, doorway-failover 10/10; warm recovery
jessica<-matthew (2 survivors, dual/dual) RECOVERED in 58 s with rows=11
after the survivor snapshot learned to page /db/content by the
stats-bounded offset (a single 500-row page hid every blob-bearing base
fixture behind a full import and read a healthy survivor as vacuous).
Two environment reds, neither transport: stage-landing-server-A OOMed on
a 1.1 GB zip (dist/server had accumulated 4.5 GB of dev-build chunks and
maps; a production ng build → 15 MB cured it) and the via-A stewardship
post-flight was the known read-after-write false red (all 3 rows 200 on
A, B and direct storage minutes later). The 13-row local recovery series
died with a container restart ($MESH_DIR is /tmp) — the recovery JSONL
needs the same in-repo home the a2o reports got. Fleet confirm still
owed: [build:edge].
DELTA 2026-08-24b (recovery measurement reader; NO status flip):
`genesis/scripts/recovery-timeline.py` now renders the raw local recovery
JSONL as a per-run series, scenario × shape aggregates, and per-scenario
before/after reliability · median · spread · failing-leg deltas; null
conductor receipts remain absent rather than becoming 0.0, and missing
scenario labels remain visible as `<unlabeled>`. Focused reader tests 6/6;
the live 13-row series rendered 13/13. Status stays red: this improves the
measurement surface but does not change dataplane convergence behavior.
DELTA 2026-08-24a (overnight iroh build-out; iroh 3/3 for the first time;
NO status flip — flip authority is the fleet lane): the iroh plane went
from responder-only to full parity. Landed: iroh eager-announce doorbell
(f21a71d9e — change-by-hash payload, landing check, pull fallback);
pure-iroh bootstrap via doorway manifest board (4cf96041e + aef9dc203 —
POST/GET /p2p/manifests, signed, bounded; storage announces + seeds its
book from it); T2 heal-on-read races the iroh plane (8346143f4 —
elohim_iroh_blob_fetches_total{ok} live, shard-ALPN not blake3, verify
parity proven). Matrix run 20260824T043035Z-a638f895: libp2p 3/3 · iroh
3/3 · dual 3/3 — pure iroh FORMED A MESH (52 s via the manifest board,
first ever). Local lanes on the built binaries: doorway-failover 10/10,
dual-plane 1/1, heal-on-read 2/2 (+ iroh blob counter moved), content-sync
4/4; full household lane 197/18/4 = ZERO genuine novel reds (one flake,
"disable the projection cache", failed 2x on 08-22 pre-dating this work);
saga 11/11. Adversarial review found a CRITICAL (manifest board Sybil-
floodable) + HIGH (client fetch OOM): board gated behind
DOORWAY_MANIFEST_BOARD_ENABLED (default OFF — dormant on the fleet which
runs dual, live in localdev), client fetch bounded (1 MiB + 64 entries);
Sybil-resistance design + 2 MEDIUMs captured to backlog
2026-08-24-manifest-board-sybil-resistance.md. Fleet confirm of BOTH
batches still owed (matthew conductor-saturation heal-leg loop blocks the
quiesce gate — backlog 2026-08-24-matthew-conductor-saturation...; the
batch-2 redeploy's pod restart is the pragmatic cure).
DELTA 2026-08-23g (doorbell + matrix green x2 of 3; NO status flip —
iroh mode still red on the T0' bootstrap boundary): root cause of the
2026-08-23e all-red matrix was NOT transport — the eager AnnounceChange
doorbell was inert (change_data always None, receiver acked without
pulling, sender had no ChangeAck arm; propagation was pull-only on the
60 s round). Cured 7d2db0a62 + d4b54537a: announce carries THE announced
change by hash (bounded 64 KiB), receiver verifies the change actually
landed (Automerge queues dep-missing changes while apply_changes returns
Ok — silent-loss trap) and falls back to a one-shot pull; 60 s round
stays the reconciliation backstop. Measured live on the 3-peer dual
mesh: fresh-write cross-peer convergence 24 s -> <=4 s. Matrix run
20260823T211809Z-ad5d8f49 (readiness-gated, split budgets, surfaced
poll errors, 9def5623e): libp2p 3/3, dual 3/3 (was 0/1 each), iroh 0/3
honest red — pure iroh forms no mesh (manifests bootstrap over libp2p;
Lane T0' owns it; remesh log iroh formed=no at 60 s deadline). Pure-iroh
/sync/v1 503 cured (29ed60c9c — with_sync_manager was libp2p-gated).
Adversarial review caught TWO dual-mode races on the shared
Arc<SyncManager> (back-fill x2 aed3bf4b2, projection listener x2
ad5d8f496 — the listener race predates this arc and shipped in every
dual deploy since Wave-2 E3); both guarded pure-iroh-only. Localdev
mesh default flipped to dual (b21bb6975, fleet parity). Full household
lane on the cured binaries: 197/18/4/31 vs 189/25/4/28 baseline — ZERO
novel reds, 7 cured (incl. the 30 s fresh-write story and both ch10
same-truth reds). Saga 11/11. Fleet confirm still owed: [build:edge].
DELTA 2026-08-23f (local T4; NO status change and no fleet claim): inventory snapshots
received over iroh and libp2p now enqueue identical work into the bounded shared scorer
queue; pure iroh records and logs the absent byte consumer. Parity 2/2, full `just test-iroh`
IROH_EXIT=0 (3,092 library tests + all iroh integration binaries), feature clippy and build
green, and `just gate elohim-storage` GATE_EXIT=0. This is the lower-layer inventory preflight
beside matrix run 20260823T185926Z-b93bcb9d; it does not cure or counterclaim that run's
three live CRDT-sync failures.
DELTA 2026-08-23e (household transport matrix; GREEN -> RED): the first
comparable live run (20260823T185926Z-b93bcb9d, 3 same-host peers, one
sample per backend, networkStage unknown) failed all three 30-second
contracts. libp2p and dual projected Matthew's new Automerge document but
Jessica and James still served no heads; pure iroh accepted the content
write but never produced Matthew's local sync document. The ValueFlows
Process view reports failed witnesses and withholds p50/p95/p99 because
failed-fast duration is not performance. Mixed per-peer launch, LAN/NAT/
relay, and heterogeneous devices remain explicitly NOT MEASURED. This
flips the habit because a runnable transport-parity check now falsifies
serving-critical convergence; it does not erase the stronger historical
evidence below.
DELTA 2026-08-23d (household lane, dual; NO status change — the habit's iroh leg
existed only on paper until tonight): the iroh plane had no peer discovery, no
sync initiator and no production request callers, so every elohim_iroh_* counter
read zero on alpha and on the mesh — "dual boots" was libp2p beside an idle
listener. Cured 972748a6d (signed elohim/transport/manifest gossip → IrohPeerBook →
per-topic join_peers) + cbc8edda5 (iroh sync-round driver). Mesh proof on the
18:02 binary: peers_known=2 on all three peers within 33 s, 18 neighbor-up,
39 Automerge changes applied OVER IROH in the first populated round, 0 failures;
a2o transport-dual-plane.feature 7/7 (the counters, not the listener, are the
claim). Regression guard held: content-sync 4/4, heal-on-read 2/2,
doorway-failover 10/10. Still open for this habit's iroh leg: pure-iroh bootstrap
(a node with no libp2p peer never receives a manifest) and blob/custody over iroh
(roadmap Lane T0′/T2/T3). Fleet evidence still required before any status flip.
DELTA 2026-08-22 (household lane; NO status change — three reds classified, one
cure landed unverified): (1) content-sync "authored node converges on a second
peer within 30 s" FAILED inside the full-lane run (run 20260822T170136Z: 18
failures, chaos drills concurrent) and PASSED 4/4 scoped at rest minutes later
(reports/cucumber-mesh-content-sync.json) — a load-sensitive window, not a
defect; read the next full-lane red as contention first. (2) identity-coherence
"lone fossil agentPubKey for human-james-son" root-caused: james was re-keyed at
RUNTIME by chaos-rekey (…7LDZ -> …5gpm at 17:15Z) and no household membership
under the new key exists on the DHT (participants of collective:uhCkkoQ… read 0
on every peer; jessica carries no james row at all), so matthew's row (…kAYh,
older than both) and the custody providers faithfully follow DEAD membership
truth — no supersede pass, boot or periodic, can converge to a key membership
never names. Cure landed for the narrower class (elohim-storage main.rs: the
membership-truth key-supersede pass re-runs every MEMBERSHIP_RECONCILE_SECS=300
instead of boot-only, so a member who DOES re-join under a new key no longer
needs its household-mates restarted), UNVERIFIED here because its input is
empty; the wider gap — a re-keyed peer must re-affirm household membership
under its new key, a DHT Membership supersession (p2p-design-gate) — is named,
not built. (3) conductor-validation-spin "undisturbed household is quiet" is an
HONEST red: the shared-log conductor is emitting "0 fetched of 1 missing
dependencies" every ~5s hours after the last drill — sys-validation spinning on
a dependency no peer can supply, the post-re-key shape, on a mesh re-keyed three
times today; and the re-key scenario's 12-minute ETIMEDOUT now carries the chaos
script's clock-stamped transcript (conductor-spin.steps.ts) so the next red
names the phase that stalled. The register read 0/11 saga chapters all morning
while a 10/11 household report sat on disk: saga-status.py read the fleet's
single mutable slot only; it now shares habits-status.py's run-identified
discovery, newest `generatedAt` across lanes.

DELTA 2026-08-20b (latency instrument born; NO status change — the
instrument is not the outcome): doorway had exactly ONE histogram and it
timed session LIFETIME, so lane C had no request-duration series at all
and its SLO was anchored to a multiplication rather than an observation.
doorway_hop_duration_ms now records four Chain-R hops (serve, proxy,
proxy_blob, resolve) at a tier the PEER derives from its own capacity —
a 1-2 core peer declines, unknown capacity resolves DOWN. Measured the
same day, and the reason this is worth a check: five stage0b runs came in
cleanly BIMODAL at 7 vs 9 poll ticks (a 1.0s step between identical runs)
and a median would have reported 4.5s and hidden it — so the scoreboard
reports modality, never a bare p50. Still UNMEASURED on this habit: no
deployed number exists yet (needs one edge deploy carrying the instrument
plus the PodMonitor selector fix that finally scrapes doorway B, which had
NO target at all). Not a flip; a rung.

DELTA 2026-08-20 (the 2026-08-19 cure below is FLEET-CONFIRMED on edge
#1370, and it did NOT produce convergence — both halves matter): reach_scoped
landed 1849 on matthew / 1715 on jessica / 124 on adam / 1 on the four
healthy pods, selecting exactly the nodes with depressed local_total, and
the anchor-gap class went to ZERO (matthew gaps == divergent == 991;
exhausted 318 -> 0). But healed is still 0 and caughtUp still false: what
remains is genuine divergence (the SPIN class), which has no discharge path
today. The queue is honest for the first time; convergence is a separate,
now-unmasked blocker. Habit stays GREEN on its own checks — which is itself
the finding: those checks passed through the whole 7-day freeze.
DELTA 2026-08-19 (residual gap-plateau root-caused and cured; desk-proven,
fleet-UNCONFIRMED — status deliberately NOT flipped): the content arm's
classify_content_gap answered "is it anchored?" against the
distribution-safe-FILTERED map while answering "is it present?"
reach-agnostically, so a present+anchored row held at a scoped reach read as
AnchorGap forever — unhealable by construction (heal stamps anchors, never
widens reach) and uncountable in local_total (distribution-safe only). Live
shape: matthew gaps{content}=2585 against reanchorPending=1 (ONE genuinely
NULL-anchor row), local_total frozen 2445->2466 over 7 days,
healed{content}=0 fleet-wide over 24h, and a permanent gap floor that kept
divergent_actionable from settling (the quiesce gate's DID-NOT-MEASURE runs
#1367-#1369). Cure: ContentGap::ReachScoped + reach-agnostic
anchored_content_ids_any_reach + elohim_projection_reconcile_reach_scoped,
red-first pinned by two tests (classifier + the three SQL predicates). This
habit's own checks stayed GREEN throughout a 7-day fleet non-convergence —
that coverage gap is the honest finding beside the cure. Flip evidence is
the new gauge: reach_scoped ~1500-2000 with gaps collapsing toward divergent
CONFIRMS; reach_scoped ~0 with gaps ~2585 FALSIFIES. Supersedes the H3
"~2000 unanchored rows" reading in
backlog/content-gap-limit-cycle-blocks-convergence.md (RCA #2 there).

DELTA 2026-08-17b (W4.1 definitive local quiesce measure, uncontended,
io_baseline 528MB/s): seed-to-sustained-PASS 94s for 3,432 rows =
2,190 rows/min — 57x the 38 rows/min baseline, 6.4x past the >=344
target; quiesce leg 82s incl. 41s sustain. Q11 prediction (200.34s,
recorded pre-run) settled +113% over with the residual NAMED:
bootstrap-vs-repair count conflation (adopt_declare 516 actual vs 6,878
modeled — Q7 ceremony compression working as designed; head_batch 24 vs
6,878) + uncalibrated fallback costs (put_record 0.61ms actual vs 50ms).
Scope honest per the plateau ruling: probe-A converged+sustained, not
fleet convergence. DELTA 2026-08-17 (chunked-blob local-read unification, desk-proven): the
>16MB serving split — /blob 404ing composites /apps served, and an
oversized notary-declared head never winning declared_head_served_blob's
raw file probe (18MB landing bundle class) — is cured by one
read_local_blob seam over both on-disk shapes (manifest-shards |
whole-composite), with /blob's shard-gap arm falling through to peer
heal. RCA falsified the memory-resident theory: bytes were durable,
agreement wasn't. Pinned by tests/chunked_blob_local_read_unification.rs
(RED->GREEN both legs); 2744 lib tests green. Surfaced separate open
hole: >64MB RS-band ingest hashes erasure-coded shards wrong
(warn-only) — backlog addendum in
chunked-blob-over-16mb-not-durable-mesh-repro.md.       DELTA 2026-08-16 (shift dataplane-facade-first-consumer): the genesis
substrate-validation suite — the mesh/propagation/custody-convergence/
delivery probes that keep this habit honest — now runs on the typed
.dataplane() facade (genesis/a2o/scripts/substrate-verify.ts consuming
@elohim/storage-client; 616-line bash+curl+jq retired from the pipeline,
kept in-tree one cycle as revert lever). Verified green ×2 across fresh
triggers (genesis #1477 + #1478: 5/7 subcommands failed=0, projection+
federation failing only on unconverged-fleet env state, byte-for-byte
the bash baseline). Every artifact stamps runner:"facade" — the PROPOSED
dataplane-sdk habit's probe is GREEN and mintable; register is at the
12-cap, so the mint (which habit yields a slot) is an operator decision
surfaced in the shift sprint result. Found+fixed in-flight: event CIDs
depended on the git binary's %aI date spelling (2.47 +00:00 vs 2.52 Z) —
normalize_git_timestamp now pins canonical UTC-Z (afa001d0b, eprfs #22).
edge #1135 2026-07-02: four standing-red concerns flipped green; elohim.host
200 with blobHash converged from matthew (zero manual PATCH); 1838 lib tests.
HELD 2026-08-06 across the automerge 0.5.12 → 0.10.0 bump (host-verified,
not yet CI-measured): sync_libp2p_convergence 3/3 including
doc_authored_on_a_converges_to_b, sync_integration 5/5 including
test_concurrent_same_field_edits UNEDITED, all four guard invariants below
green, `just gate` exit 0 (3199 passed / 0 failed). The bump needed ZERO
source edits in this crate — 0.10 changed only get_changes' return from
Vec<&Change> to Vec<Change>, and our sole call site is `.len()`.
HELD 2026-08-10: /account/import no longer writes viewer-relative package
assignments into peer-global content.reach; red-first handler regression
and the full elohim-storage unit/integration inventory are green.
2026-08-29 ratchet slice (local evidence): ids with spaces reachable by id (leaf percent-decode) +
recovery harness quotes ids (P1 measures replication, not URL grammar); route vocabulary pre-touched
at zero (absence ≠ 'nothing routed'); first acquisition drain holds ≤10 s for the iroh book
(`first_drain_total{outcome}`). Lib tests green on default + p2p-iroh; fleet/mesh reads pending.
MEASURED 2026-08-29 (household mesh, dual, full corpus): warm recovery jessica PASS 439 s with P1 honest
(space-ids reachable by id on the recovered peer; absent set drained to 0); route vocabulary at 0 on boot then
best_rtt iroh 27:3 on the pull leg; first-drain hold expired at 10 s while the book warmed at +28 s (30 s
manifest cadence) — shape 3 is the next cut. Record: reports/recovery/recovery-timeline.jsonl.
- 2026-08-29 (shape 3, household mesh dual, `d4fc29f5b`+retry): the doorway bootstrap seeds the iroh book at T0 — simultaneous warm restart of all three peers reads `{boot,seeded} 1` → first drain `book_warm` at 139/343/6 ms (this morning: `held 2 → expired 1` on all three, gossip +28 s); warm recovery jessica pull-leg dispatch iroh 274 / libp2p 29 (was 225 / 73). Cold-start first-booter `{boot,empty}` closed by a 2 s × 5 bounded retry inside the 10 s hold.
- 2026-08-29 (half-record chain, household mesh dual, `89715cae0`+`e4eb19ec6`): warm recovery on the shape-3 binary went NOT-RECOVERED 915 s / P1 134 — half records served by a half survivor, never re-read against docs that held the hash. Level-triggered half-row sweep + amber NULL-fill landed; all three peers converge to full 3414 / half 1 / stuck 0. Final warm recovery 3375/3376 — the one row is a cross-survivor blob_hash disagreement (CID vs sha256 on an html5-app bundle), a new node, not this cut. Open siblings filed: inventory-refresh-pages-dropped-as-gaps; blob_hash writers that skip ContentUpdated (docs stale until cold-start back-fill).
- 2026-08-29 (inventory reorder + content-touch, `4230637f4`, cold dual mesh): a neighbour's inventory view 184 → 2892 of 3513 hashes within one refresh, gap warnings 3189/h → 0, overflow 0; bus-less DHT-signal writers now announce touches (sent 27/76/31). Fleet read without a deploy: the board is gated off on alpha (shape 3 designed-inert there); alpha's pull leg fetched 0 in 24 h vs 3.2k fetch_error — filed `alpha-pull-leg-fetch-error-storm`.
- 2026-08-29 (inventory pacing + restart re-base + declare-touch, household mesh dual): a 77-page refresh now lands 0..76 contiguous (was 0..62 — a 64-slot command channel, not reordering); a neighbour view holds 3535/3535; a publisher restart no longer strands receivers (3535→46 stuck for ~seq/78 refreshes before; now re-based within ≤2 refreshes); the cold-start drift report names `headActionHash` as the stale-doc class (318/327/558 → 54/22/86 after the touch sink; HTTP declare handlers now announce). Mesh restart guard fixed (fixture rows with sha256 anchors read as a DNA mismatch).
- 2026-08-29 (sync-state contract, `3af9cef7d`+`f4d819a30`): one vocabulary for every stream — epoch before position, position monotone per epoch, caught-up is `position >= declared` and `None` while unknown — spec'd with a station map for inventory / docs / pull; station 1 landed (inventory sequences carry the publisher's boot epoch; a restart is strictly ahead on the wire, no heuristic). Full gate green on the tree before it (3052 passed); station 1's tests + clippy green.
- 2026-08-29 (fleet deploy, edge #1390, `bdd5f9ef2`, alpha): the day batch landed on all 7 peers + both doorways (deploy junit 7/7 Ready; #1388 had 2 not-Ready). First fleet read of the new counters at +10 min: inventory `rebased 0` with all seven publishers restarted together (station-1 boot epoch on the wire), `flushed 879` / `overflow 0` / `failed 68`; published `sent 542 deferred 0`; `first_drain` expired 5/7 inside the 10 s hold, book_warm 2/7, yet `iroh_peers_known=6` everywhere minutes later; bootstrap board 0 (gated); half-row sweep scanned 0; content touches 0 (no fleet writes since deploy). Pull leg: `outcomes_total` has NO series — matthew `pull 2/2 caughtUp=true`, 0 dispatches (every want already local; 135 pins retired), so the 3.2k/24 h `fetch_error` burn is gone by absence, and the holds-it vs rejects-it split stays unobserved. Dataplane Validation 3 pass / 1 fail / 84 gated: `inventory-convergence` red on storage-A `p2p.caughtUp=false divergentAnchor=1007` at +30 min (inside restart churn and `projection_reconcile`'s ≈1 h dormancy — re-read pending). `federation-deploy` (this habit's check) stayed gated → **status stays red**.
- 2026-08-30 (post-deploy watch, edge #1390 `bdd5f9ef2`, storage-A, ~+2 h after the 23:05Z restart): `p2p.caughtUp` never flipped in 100 min of polling; `replication` and `pull` caught up, the red is the projection-reconcile leg alone — `pending` 1045 (23:20Z) → 764 (00:24Z) → see pending=1001 healed=3 failed=1 sweeps=23 exhausted=0 divergentAnchor=1047 peersAsked=5 pull.caughtUp=True replication.caughtUp=True at 01:14Z; `healedTotal` 3, `failed` 3, `exhausted` 0, `divergentAnchor` cycling 817/1007/1047/1111 by peer set. Not draining: `pending` is a live comparison that re-grows with fresh publishes (764 → 1001) while `healedTotal` stays at 3 over 23 sweeps — the leg is sweeping without healing, as on every prior alpha deploy (R7 false for a day after #1381–#1388); restart churn on this fleet is not ≈20 min for the projection leg. Whether that is pace or a fills-never-moves stall needs a read past +6 h; the `inventory-convergence` scenario will red every deploy until then. Status stays red.
- 2026-08-30 04:11Z — device-peer probe from the Che workspace (fork pair c9a6c4439, agent W): conductor joined alpha (bootstrap 03:41:14Z, diagnostics ≤5 min, sovereign-peer-join sc.1 PASS); workspace storage on iroh attached to relay.alpha in 3 s (iroh 0.92 ↔ fork relay OK); W bulk-created → PATCH-anchored (uhCkkFPFhi…, 04:05:51Z) → DECLARED head (trust notarized, 04:10:55Z) a content node with no doorway and no Jenkins. Fleet doorways 404 the id for 25 min: storage plane is namespace-isolated (pod IPs timeout, fleet iroh relay-less, board gated) so no fleet storage can learn it — p1-dht-authored-content-not-projected re-measured on the correct pair. Enablers staged (storage ELOHIM_IROH_RELAY_URL + board on doorway-alpha), uncommitted pending operator OK.
- 2026-08-30 06:05Z — two deploys later (a17316af pin c9a6c4439; 9d2842a63 storage copies /bin/holochain): the fleet's conductor is NOT the CI-built one — household pods log the pre-fix `access.rs:192` line (1.04 M/10 min), fleet agents sign iroh addresses a tx5-only CI build cannot, so alpha runs the holo-host base's conductor and every "fork fix on the fleet" reading in this ledger since the holo-host base needs `holochain --build-info` before it is trusted (backlog conductor-pin-ships-base-binary). Storage plane is namespace-isolated for outside peers (relay-less iroh, board gated, pod-IP libp2p) — enablers staged, uncommitted. Status stays red; the honest measure of this habit tonight is the device-peer probe: declared on the DHT by W, unservable by the fleet.
- 2026-08-30 17:36Z — FIRST native content sync from a device peer, no doorway seed, no Jenkins in the content path: workspace storage (iroh 0.92 via relay.alpha; agent W) authored/anchored/declared `gate-reading-manifesto-20260830T131032Z` at 13:14Z; plane opened by three env enablers (storage ELOHIM_IROH_RELAY_URL 7234b6ff0, doorway-alpha board on, storage ELOHIM_DOORWAY_URL 08654c016); first storage-A sync contact 16:40:17Z (board 4 entries); doorway-alpha served it 17:04:35Z (notarized, anchor = W's uhCkkDoIATk…, bytes 200) and elohim.host 17:35:49Z (published/amber, bytes 200). t_first-contact→served-A ≈24 min (inside storage-A's post-deploy churn), A→B ≈31 min. Workspace: irohPeersKnown 7, replication completed 4498 caughtUp, 2,588 inventory pages applied. The pull leg works for an outside peer once it is reachable; the remaining reds are identity (W ≠ matthew) and the reach-change join.
- 2026-08-31 01:45Z — fresh convergence read, no deploy fired: both doorways hold W2's manifesto BYTES (blobHash sha256-2454b80…, stamped 23:37Z on both) but serve DIFFERENT heads (A uhCkkk8Gu…, B uhCkkC9oC…) — blob/iroh plane converged, head plane frozen. Measured cause chain: all 32 agent-infos in BOTH doorways' peer stores advertise `storageArc: None` (~2 h after the roll, 0/7 conductors are an authority for anything — kitsune2 resets current arc to Empty on every restart and promotes to FULL only after one clean gossip round; adam-firstman ARC-CONVERGENCE comment documents "every zome get leaves the box and dies on request_timeout_s" in this state), so canonical head records are unfetchable: `canonical_answers_total{tier="earned"}`=0 on every pod, `election_obeyed_total{path="carried"}`=0 fleet-wide (the carried-record adoption path exists and is never exercised), and `divergent_refused`≈`divergent` (adam 2603/2619, james 1715/1715, matthew 981/991) — the reconciler sweeps and correctly refuses root-author fallback forever (I3). `projection_reconcile_converged`=0 at EVERY hourly sample for 3 days across ~13 full-fleet simultaneous rolls (instance-IP churn; kube restart counters 0–6, so recreation not crashes): the fleet re-enters the all-arcs-Empty state faster than promotion completes. Net: the only channel that moves a served head is an explicit per-host declare (deploy authorHeadOnce / seed / W2's declare), and those hit A and B asymmetrically (B byte-seed 403 since 08-25) → A≠B indefinitely. Compounding: the manifesto's root-author key uhCAkSK6u9… is absent from both peer stores (retired by a prior re-key), so root-author fallback for genesis ids is permanently stale even after arc convergence. Status stays red; cure surface = (1) staggered/less-frequent rolls or arc persistence across restart, (2) drive head adoption over the already-converged storage plane via the built-but-unused carried-record election path, (3) doorway-B seed 403, (4) genesis root-author lineage.
- 2026-08-31 02:05Z — carry-the-election PROVEN on the household mesh (libp2p, 3 peers, stock conductor, fresh happ): two peers authored + DECLARED divergent heads for one id (the fleet's frozen class); an EARNED canonical declared on matthew; matthew's conductor served the winning declaration LINK's signed Record (`get_canonical_election_evidence`, 499 B); jessica's conductor VERIFIED it in wasm (`verify_carried_election`) and answered the earned election with matthew's head as winner; a 1-byte-tampered record was REFUSED in wasm. Sweep leg: the heal leg's `Refreshed` echo (declared row, conductor echoes it back) was the un-admitted refusal site — the flag-gated DECLARED-DIVERGENCE ADMISSION now feeds it to the adopt pre-flight; on this run contest minted a canonical candidate USING the carried record and the DHT election moved jessica's row → BOTH peers serve the SAME elected head (~2 min, `carried-election-mesh-proof.ts` exit 0). Storage lib 3059/0 ×2, clippy clean. Fleet residue: the peer_carried supply arm (for gossip-dead conductors) is proven cross-conductor but not yet sweep-exercised (mesh gossip works — the mesh cannot fake the fleet's arc-Empty regime); fleet enactment = coordinator hot-swap + edge deploy + ELOHIM_OBEY_CARRIED_ELECTION flip, pending operator. Build trap filed in-session: rustc 1.98 (container, 2026-08-18) made rust-lld refuse undefined wasm host imports — zome cdylibs need `-C link-arg=--import-undefined`; `just build` in the DNA workspace is red without it (hc-rna + every zome).
- 2026-08-31 18:55Z — carried-election FLEET enactment shift closed at budget (shift 2026-08-31T02-40, 8 iterations, 5 fleet rolls). LANDED live on all 7 pods: the flag (ELOHIM_OBEY_CARRIED_ELECTION=true, boot-warn confirmed 7/7), the iroh/libp2p frame-cap drift cure (iroh reader 256KiB→1MiB, sender clamp under the deployed 256KiB floor — W2's inventory starvation "frame too large: 536378 > 262144" cured, zero recurrences), and the dual-peer-source cure (reconcile stream was libp2p-IF-PRESENT-ELSE-iroh: a dual fleet NEVER polls iroh-only peers by construction — CompositeReconcilePeers unions the planes; the reason no board/book/reboot could make the fleet ask W2). Supplier W2 re-stood on :8093 (fork conductor hot-swapped via admin update_coordinators — no re-key; serves the manifesto's earned election evidence, winner uhCkk1kms…, 499B signed link record). NOT YET OBSERVED at close: first fleet content-inventory poll of W2 → hint → obeyed{path="peer_carried"} → both doorways serving uhCkk1kms…. Watch: `elohim_content_election_obeyed_total{path="peer_carried"}` + `genesis/a2o/scripts/dataplane-convergence-measure.ts` (probe file reports/carried-election-fleet-probe.json). W2 left RUNNING as the standing supplier. New backlog: late-joiner-peer-discovery-boot-only-board · mesh-fixture-fidelity-regimes · upgrade-propagation-p2p-design-arc (operator course-set: the north star). Status stays red pending the observed obey.
- 2026-09-01 16:01Z — household `peer_carried` sweep receipt STOPPED at the fixture seam: conductor-UP negative control converged with `peer_carried_delta=0`; stopping/pausing the adopter conductor makes the local election answer unreachable/blocked and returns before peer supply, while the successful supply branch still requires that same conductor for `verify_carried_election` + `validate_carried_head_record`. Missing station recorded in `task-peer-carried-sweep-mesh-receipt`: fixture observed-ABSENT without removing the wasm verifier. No receipt and no status flip; fleet watch remains authoritative.
- 2026-09-01 16:20Z — household GSO burst/disconnect receipt STOPPED at the fixture seam (NO status flip): `gso-burst-receipt.ts` preserved the mixed `matthew=dual` / `jessica=iroh` posture and counted only decoded head-record replies, but a settled-mesh storage restart produced 12 successful exchanges in 360 s versus the required >=200. The fail-closed `no-burst` leg withheld SIGKILL and the receipt. Missing station: fixture-owned stale/full-corpus requester staging between mesh-harness -> burst-regime; parent GSO atom stays wip and this habit stays red.
- 2026-09-02 00:43Z — household election-convergence numbers from the adoption ceremony (NO status flip: the a2o concern here is federation-deploy on the fleet): a fresh release channel's staged head reached 3/3 conductors ≤19 s after publish with doorway A (bootstrap/signal) up; the same publish took 20 min 18 s for the iroh peer while doorway A was DOWN (island; conductor log `NoPeersForLocation`) — doorway liveness is a precondition of every election measure, now in the ceremony preflight. Earned-head convergence: 75 s (promote) and 31 s (revert), 3/3.

DELTA 2026-09-04 00:3xZ (holochain 0.7 cutover, F5): the alpha fleet re-genesised on holochain 0.7.0 — 7 storage + 7
conductor StatefulSets rolled by edge #1428 onto wiped volumes; both doorways `caughtUp:true converged:true peerCount:6`;
conductor peer stores 35 agents / 5 spaces homed across BOTH relays (`relay.alpha.elohim.host` + `relay.elohim.host`).
Status unchanged (this habit's check is federation-deploy, Act II; the fleet has no content yet — re-seed follows). The
line change itself was a wipe, not propagation: see runtime-upgrade-propagation's 2026-09-03 scope DELTA.

- 2026-09-05 06:00Z — VERSION-CONVERGENCE SCENARIO MEASURED GREEN on the household mesh, first time: 14/14 steps,
  `cucumber-js` exit 0, 1m21s (receipt `genesis/a2o/reports/dataplane/2026-09-05/carried-election-organic-receipt-2026-09-05T05-58-30-956Z.json`,
  run report `federation-deploy-final-055657Z.json`). Two peers holding divergent declared heads for one page CONVERGED
  organically in 40s with the operator flag on: `ledgerAtWhen == ledgerAtThen == 5` and `declarationCallsDuringCure: 0`
  — the fixture's five staging writes all land BEFORE the wait and it makes none during it, so the cure is the peer's
  own sweep, not the test. Both doorways then served the same head, and each doorway's served head matched its own
  declared head for the REAL landing EPR (the pain-points T7 assertion, exercised green). The move is discriminated from
  recency by construction: the laggard gave up a head it authored at 05:57:50 for one authored at 05:57:13 — 37s
  BACKWARDS, which only earned-beats-staging explains.
  FIVE measured reds got there, each a substrate fact the plan had not anticipated, and each is now written into the
  fixture rather than worked around:
    (1) `@requires:household-nodes` GATES NOTHING (`available: true` in both cluster-state files) while
        run-dataplane-validation.sh selects `@dataplane and not @wip` — un-@wip'ing had admitted a WRITING scenario to
        the live fleet lane. Now `@requires:owned-substrate`.
    (2) The feature's `@act:ii` held every scenario in it on the household lane (`acts[0]` takes the FEATURE tag and a
        second act tag is an authoring error), so the Act I scenario was SPLIT into
        `features/dataplane/federation-version-convergence.feature` (@act:i). A file carries exactly one act.
    (3) `POST /db/content/{id}/head` gates on the CANONICAL head's author: 403 on seeded content. Unnecessary anyway —
        the conductor-routed PATCH already declares via `upsert_with_anchor(.., Declare)`.
    (4) `declare_earned_canonical_head` is restricted IN WASM to the root author / its delegated device / the progenitor
        steward. NO fixture can ever stage an earned declaration on a page it did not author, so the vehicle is a page
        the run authors; the landing EPR is read, never staged. (Its body was verified byte-intact on both peers after
        every run.)
    (5) A page id is UNIQUE in the DHT — the second peer to plant a root is refused ("already exists. Use
        update_content"), sequentially AND concurrently, because local household gossip beats the race. The 2026-08-31
        proof staged two roots and passed only by winning that race; staging is now ONE root plus two divergent
        revisions, which is deterministic.
  NOT PROVEN HERE, and it cannot be: `elohim_content_election_obeyed_total{path="peer_carried"}` stayed structurally
  ABSENT while the row converged — `obey_probe_total` accounted for all 3 probes as `no_election` (2) / `resolve_error`
  (1). On a healthy mesh the laggard's own conductor resolves the election, so the peer-carried supply arm is never
  needed and cannot be exercised; the move came through the contest path. This confirms rather than contradicts the
  2026-08-31 note — the mesh cannot fake the fleet's arc-Empty regime, and peer_carried evidence stays a FLEET matter.
  The task brief's "the obeyed{path=peer_carried} counter moved" assertion was therefore replaced by the recency
  discriminator above; which arm ran is recorded verbatim in every receipt as observation.
  Status stays RED: this is one of six checks, and the fleet-side peer_carried observation the 2026-08-31 entry is
  waiting on is still unobserved.
